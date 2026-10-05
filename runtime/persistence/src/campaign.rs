//! Campaign saves use the existing immutable replay journal and reward ledgers.
//! The character's module lock serializes campaign/crafting/commission writes.
//! Every read verifies the complete campaign transition chain from legacy-empty
//! state; each first clear has the natural (character, chapter) identity in it.
use std::{collections::BTreeSet, error::Error, io};

use revenant_campaign::{CampaignState, Command, Transition};
use revenant_replay::{
    decode_campaign_payload, decode_campaign_resume, encode_campaign_payload,
    encode_campaign_resume, reconstruct, verify_campaign_milestone, CampaignReplayPayload,
    CampaignResumePayload, CampaignStoryProof, ReplayEvent, ReplayEventKind,
    ReplayProtocolGeneration,
};

use super::{
    apply_completion_reward, ensure_and_lock_module_state, ensure_character_account,
    insert_replay_event, module_mutation_replay_event, validate_module_replay_identity,
    ActivityCompletion, CompletionRewards, ExperienceReward, GenericClient,
    ModuleMutationReplayContext, Persistence, Transaction, RELAY_CORE_FRAGMENT,
};

pub type CampaignPersistenceResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub struct CampaignRequest<'a> {
    pub operation_id: &'a str,
    pub expected_revision: u64,
    pub command: Command,
    /// Confirm references prior world evidence. Story takes None: its spatial
    /// evidence and checkpoint are inserted in the same transaction.
    pub proof_event_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignReceipt {
    pub replayed: bool,
    pub transition: Transition,
    /// Current save, which may have advanced since a retried operation.
    pub state: CampaignState,
    /// Present only when this transaction applies a first-clear reward.
    pub rewards: Option<CompletionRewards>,
}

impl Persistence {
    /// Admits a new session at the character's last verified checkpoint, before
    /// world events. A retry returns the original admission, never a reward or
    /// a revision advance. Callers stage the encounter from this returned state.
    ///
    /// # Errors
    /// Rejects stale saves, foreign chapters/participants, late admission and
    /// corrupt evidence. The snapshot is committed under the character lock.
    pub fn resume_campaign_with_replay(
        &mut self,
        character_id: &str,
        expected_revision: u64,
        context: ModuleMutationReplayContext<'_>,
    ) -> CampaignPersistenceResult<CampaignState> {
        let mut tx = self.client.transaction()?;
        validate_module_replay_identity(context.session_id, context.activity_id, context.actor_id)?;
        ensure_character_account(&mut tx, character_id, context.account_id)?;
        ensure_and_lock_module_state(&mut tx, character_id)?;
        let (state, journal) = load_journal(&mut tx, context.account_id, character_id)?;
        let events = load_events(&mut tx, context.session_id, i64::MAX)?;
        validate_admission(&events, character_id, context)?;
        if let Some(event) = events
            .iter()
            .find(|e| e.kind == ReplayEventKind::CampaignResumed)
        {
            let existing = decode_campaign_resume(&event.payload)?;
            if existing.character_id != character_id || existing.state.revision != expected_revision
            {
                return Err(invalid(
                    "campaign admission retry conflicts with original save",
                ));
            }
            return Ok(existing.state);
        }
        if state.revision != expected_revision
            || state
                .active
                .as_ref()
                .is_none_or(|run| run.chapter.as_str() != context.activity_id)
        {
            return Err(invalid(
                "campaign resume uses a stale save or different chapter",
            ));
        }
        let checkpoint_event_id = journal
            .last()
            .ok_or_else(|| invalid("campaign has no checkpoint to resume"))?
            .0
            .id;
        let encoded = encode_campaign_resume(&CampaignResumePayload {
            schema_version: 1,
            character_id: character_id.to_owned(),
            checkpoint_event_id,
            state: state.clone(),
        })?;
        let id = insert_replay_event(
            &mut tx,
            &module_mutation_replay_event(context, "campaign_resumed", &encoded),
        )?;
        reconstruct(&load_events(&mut tx, context.session_id, id)?)?;
        tx.commit()?;
        Ok(state)
    }

    /// Loads verified chapter progress without creating rows for legacy saves.
    ///
    /// # Errors
    /// Rejects foreign identity, a corrupt journal/reward proof or database errors.
    pub fn campaign_state_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> CampaignPersistenceResult<CampaignState> {
        let mut tx = self
            .client
            .build_transaction()
            .isolation_level(postgres::IsolationLevel::RepeatableRead)
            .start()?;
        ensure_character_account(&mut tx, character_id, account_id)?;
        let (state, _) = load_journal(&mut tx, account_id, character_id)?;
        tx.commit()?;
        Ok(state)
    }

    /// Commits an accepted server milestone, chapter state, terminal mission
    /// evidence and first-clear reward in one transaction. Operation identities
    /// are resolved under the same character lock before evaluating a new command.
    ///
    /// # Errors
    /// Rejects stale/foreign/unproved progress, operation conflicts, invalid saves
    /// and database failures. No checkpoint or reward survives a rejected write.
    #[allow(clippy::too_many_lines)] // Keep lock, retry, spatial proof and checkpoint in one transaction.
    pub fn apply_campaign_command(
        &mut self,
        character_id: &str,
        request: &CampaignRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
    ) -> CampaignPersistenceResult<CampaignReceipt> {
        let mut tx = self.client.transaction()?;
        validate_module_replay_identity(context.session_id, context.activity_id, context.actor_id)?;
        ensure_character_account(&mut tx, character_id, context.account_id)?;
        ensure_and_lock_module_state(&mut tx, character_id)?;
        let (state, journal) = load_journal(&mut tx, context.account_id, character_id)?;
        let story_command = matches!(request.command, Command::Story { .. });
        if story_command && request.proof_event_id.is_some() {
            return Err(invalid(
                "story evidence must be written with its checkpoint",
            ));
        }
        if let Some((event, payload)) = journal
            .iter()
            .find(|(_, p)| p.operation_id == request.operation_id)
        {
            if payload.transition.command != request.command
                || payload.transition.before.revision != request.expected_revision
                || (!story_command && payload.proof_event_id != request.proof_event_id)
                || event.session_id != context.session_id
                || event.activity_id.as_deref() != Some(context.activity_id)
                || event.actor_id != u64::try_from(context.actor_id).ok()
            {
                return Err(invalid(
                    "campaign operation identity was reused with different inputs",
                ));
            }
            return Ok(CampaignReceipt {
                replayed: true,
                transition: payload.transition.clone(),
                state,
                rewards: None,
            });
        }
        let transition = state.propose(request.expected_revision, request.command.clone())?;
        if let Command::Enter { run_id, .. } = &request.command {
            if journal.iter().any(|(_, p)| matches!(&p.transition.command, Command::Enter { run_id: previous, .. } if previous == run_id)) {
                return Err(invalid("campaign run identity was already used"));
            }
        }
        let chapter = match &request.command {
            Command::Enter { chapter, .. } => *chapter,
            Command::Confirm { .. } | Command::Story { .. } => {
                state
                    .active
                    .as_ref()
                    .ok_or_else(|| invalid("campaign run missing"))?
                    .chapter
            }
        };
        if context.activity_id != chapter.as_str() {
            return Err(invalid("campaign session uses a different chapter"));
        }
        let events = load_events(&mut tx, context.session_id, i64::MAX)?;
        validate_admission(&events, character_id, context)?;
        if events
            .iter()
            .any(|e| e.kind == ReplayEventKind::ActivityCompleted)
        {
            return Err(invalid(
                "campaign mission was already completed outside this transaction",
            ));
        }
        if let Command::Confirm { milestone, .. } = request.command {
            verify_campaign_milestone(
                &events,
                context.account_id,
                u64::try_from(context.actor_id)?,
                milestone,
                request
                    .proof_event_id
                    .ok_or_else(|| invalid("campaign proof missing"))?,
            )?;
        }
        let proof_event_id = if story_command {
            let proof =
                CampaignStoryProof::new(character_id, &state, &request.command)?.encode()?;
            Some(insert_replay_event(
                &mut tx,
                &module_mutation_replay_event(context, "campaign_story", &proof),
            )?)
        } else {
            request.proof_event_id
        };
        let payload = CampaignReplayPayload {
            schema_version: 1,
            character_id: character_id.to_owned(),
            operation_id: request.operation_id.to_owned(),
            transition: transition.clone(),
            proof_event_id,
        };
        let encoded = encode_campaign_payload(&payload)?;
        let terminal = state.active.is_some() && transition.after.active.is_none();
        let rewards = if terminal {
            complete_chapter(&mut tx, character_id, &transition, context)?
        } else {
            None
        };
        let event_id = insert_replay_event(
            &mut tx,
            &module_mutation_replay_event(context, "campaign_checkpoint", &encoded),
        )?;
        reconstruct(&load_events(&mut tx, context.session_id, event_id)?)?;
        tx.commit()?;
        Ok(CampaignReceipt {
            replayed: false,
            state: transition.after.clone(),
            transition,
            rewards,
        })
    }
}

fn validate_admission(
    events: &[ReplayEvent],
    character_id: &str,
    context: ModuleMutationReplayContext<'_>,
) -> CampaignPersistenceResult<()> {
    let reconstructed = reconstruct(events)?;
    if reconstructed.module_participants.len() != 1
        || !reconstructed.module_participants.iter().any(|p| {
            p.character_id == character_id
                && i64::try_from(p.actor_id).ok() == Some(context.actor_id)
                && p.protocol_generation == ReplayProtocolGeneration::V2
        })
        || !events.iter().any(|e| {
            e.kind == ReplayEventKind::ModuleStateSnapshot
                && e.actor_id == u64::try_from(context.actor_id).ok()
                && e.account_id == context.account_id
                && e.activity_id.as_deref() == Some(context.activity_id)
        })
    {
        return Err(invalid(
            "campaign admission does not match this solo V2 character",
        ));
    }
    Ok(())
}

fn complete_chapter(
    tx: &mut Transaction<'_>,
    character_id: &str,
    transition: &Transition,
    context: ModuleMutationReplayContext<'_>,
) -> CampaignPersistenceResult<Option<CompletionRewards>> {
    insert_replay_event(
        tx,
        &module_mutation_replay_event(context, "activity_completed", "activity completed"),
    )?;
    let Some(grant) = transition.first_clear else {
        return Ok(None);
    };
    let rewards = apply_completion_reward(
        tx,
        &ActivityCompletion {
            session_id: context.session_id,
            account_id: context.account_id,
            character_id,
            activity_id: context.activity_id,
            item_id: RELAY_CORE_FRAGMENT,
            item_quantity: i32::try_from(grant.fragments)?,
            experience_reward: ExperienceReward::validated(grant.experience)?,
        },
    )?
    .ok_or_else(|| invalid("campaign first-clear reward identity already exists"))?;
    let loot = format!(
        "loot granted: {RELAY_CORE_FRAGMENT} x{} (total {})",
        grant.fragments, rewards.item_quantity
    );
    let progression = format!(
        "progression granted: +{} XP (total {}, level {} -> {})",
        rewards.experience_granted, rewards.experience, rewards.previous_level, rewards.level
    );
    insert_replay_event(
        tx,
        &module_mutation_replay_event(context, "loot_granted", &loot),
    )?;
    insert_replay_event(
        tx,
        &module_mutation_replay_event(context, "progression_granted", &progression),
    )?;
    Ok(Some(rewards))
}

type Journal = Vec<(ReplayEvent, CampaignReplayPayload)>;

fn load_journal(
    client: &mut impl GenericClient,
    account_id: &str,
    character_id: &str,
) -> CampaignPersistenceResult<(CampaignState, Journal)> {
    let rows = client.query(
        "SELECT id, event_type, occurred_at::TEXT, session_id, account_id, activity_id, actor_id, payload \
         FROM replay_events WHERE account_id = $1 AND event_type IN ('campaign_checkpoint', 'campaign_resumed') \
         AND payload::jsonb->>'character_id' = $2 ORDER BY id", &[&account_id, &character_id],
    )?;
    let mut state = CampaignState::default();
    let mut journal: Journal = Vec::new();
    let mut operations = BTreeSet::new();
    let mut runs = BTreeSet::new();
    for row in rows {
        let event = from_row(&row)?;
        if event.kind == ReplayEventKind::CampaignResumed {
            let payload = decode_campaign_resume(&event.payload)?;
            if payload.state != state
                || journal
                    .last()
                    .is_none_or(|(checkpoint, _)| checkpoint.id != payload.checkpoint_event_id)
            {
                return Err(invalid(
                    "campaign admission does not reference the current durable checkpoint",
                ));
            }
            let session = reconstruct(&load_events(client, &event.session_id, event.id)?)?;
            if session.campaign.len() != 1 || session.campaign[0].state != state {
                return Err(invalid("campaign admission and session replay disagree"));
            }
            continue;
        }
        let payload = decode_campaign_payload(&event.payload)?;
        if payload.transition.before != state || !operations.insert(payload.operation_id.clone()) {
            return Err(invalid(
                "campaign journal is discontinuous or duplicates an operation",
            ));
        }
        if let Command::Enter { run_id, .. } = &payload.transition.command {
            if !runs.insert(run_id.clone()) {
                return Err(invalid("campaign journal reuses a run identity"));
            }
        }
        let events = load_events(client, &event.session_id, event.id)?;
        let reconstructed = reconstruct(&events)?;
        if reconstructed.campaign.len() != 1
            || reconstructed.campaign[0].state != payload.transition.after
        {
            return Err(invalid("campaign journal and session replay disagree"));
        }
        if payload.transition.before.active.is_some() && payload.transition.after.active.is_none() {
            if !reconstructed.completed {
                return Err(invalid("chapter clear lacks mission completion"));
            }
            let items = client.query(
                "SELECT item_id, quantity FROM inventory_reward_grants WHERE session_id = $1 AND character_id = $2",
                &[&event.session_id, &character_id],
            )?;
            let experience = client.query(
                "SELECT experience FROM progression_reward_grants WHERE session_id = $1 AND character_id = $2",
                &[&event.session_id, &character_id],
            )?;
            match payload.transition.first_clear {
                Some(grant)
                    if items.len() == 1
                        && experience.len() == 1
                        && items[0].get::<_, String>(0) == RELAY_CORE_FRAGMENT
                        && i32::try_from(grant.fragments).ok() == Some(items[0].get(1))
                        && i64::try_from(grant.experience).ok() == Some(experience[0].get(0))
                        && reconstructed.loot_grants == 1
                        && reconstructed.progression_grants == 1 => {}
                None if items.is_empty()
                    && experience.is_empty()
                    && reconstructed.loot_grants == 0
                    && reconstructed.progression_grants == 0 => {}
                _ => {
                    return Err(invalid(
                        "campaign first-clear ledger differs from durable mission rewards",
                    ))
                }
            }
        }
        state = payload.transition.after.clone();
        journal.push((event, payload));
    }
    Ok((state, journal))
}

pub(super) fn load_events(
    client: &mut impl GenericClient,
    session_id: &str,
    through: i64,
) -> CampaignPersistenceResult<Vec<ReplayEvent>> {
    client.query("SELECT id, event_type, occurred_at::TEXT, session_id, account_id, activity_id, actor_id, payload \
        FROM replay_events WHERE session_id = $1 AND id <= $2 ORDER BY id", &[&session_id, &through])?
        .iter().map(from_row).collect()
}

pub(super) fn from_row(row: &postgres::Row) -> CampaignPersistenceResult<ReplayEvent> {
    Ok(ReplayEvent {
        id: row.get(0),
        kind: row.get::<_, String>(1).parse()?,
        timestamp: row.get(2),
        session_id: row.get(3),
        account_id: row.get(4),
        activity_id: row.get(5),
        actor_id: row
            .get::<_, Option<i64>>(6)
            .map(u64::try_from)
            .transpose()?,
        payload: row.get(7),
    })
}

fn invalid(message: &'static str) -> Box<dyn Error + Send + Sync> {
    io::Error::other(message).into()
}
