//! Challenge transitions share the character's existing module lock. The
//! immutable journal is the save; there are no challenge rewards or migrations.
use std::{collections::BTreeSet, error::Error, io};

use revenant_challenges::{
    ChallengeState, Command, Equipment, Transition, ELITE_REVISION, EXPANDED_REVISION,
    GAUNTLET_REVISION, MODIFIER_REVISION, PRISM_REVISION, REVISION, SIGNAL_REVISION,
    SURVIVAL_REVISION,
};
use revenant_replay::{
    decode_challenge_payload, encode_challenge_payload, reconstruct, ChallengeReplayPayload,
    ChallengeVisitProof, EliteEvidence, PrismEvidence, ReplayEvent, ReplayEventKind,
    SupportEvidence,
};

use super::{
    campaign::{from_row, load_events},
    ensure_and_lock_module_state, ensure_character_account, insert_replay_event,
    load_persisted_module_state, module_mutation_replay_event, validate_module_replay_identity,
    GenericClient, ModuleMutationReplayContext, Persistence,
};

mod combat;
mod gauntlet;
mod recovery;
mod route;
mod signal;
mod survival;
use combat::Proof as CombatProof;

pub type ChallengePersistenceResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
type Journal = Vec<(ReplayEvent, ChallengeReplayPayload)>;

/// Server-owned inputs. Gateway adapters must derive equipment and position
/// from the admitted character/actor, never from client completion claims.
pub struct ChallengeRequest<'a> {
    pub operation_id: &'a str,
    pub expected_revision: u64,
    pub command: Command,
    /// Only combat confirms and defeat cite an existing authoritative event.
    pub proof_event_id: Option<i64>,
    /// Exploration visits record the server actor's position atomically.
    pub position: Option<[i32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeReceipt {
    pub replayed: bool,
    pub transition: Transition,
    pub state: ChallengeState,
}

impl Persistence {
    /// Reads the character's mastery-v1 archive from its immutable, verified
    /// challenge history. No new grant/counter is written; existing qualifying
    /// attempts remain durable proof and repeated reads are idempotent.
    ///
    /// # Errors
    /// Rejects a foreign account, branching journal or invalid session evidence.
    pub fn challenge_archive_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> ChallengePersistenceResult<(ChallengeState, revenant_challenges::mastery::Archive)> {
        let mut tx = self
            .client
            .build_transaction()
            .isolation_level(postgres::IsolationLevel::RepeatableRead)
            .start()?;
        ensure_character_account(&mut tx, character_id, account_id)?;
        let (state, journal) = load_journal(&mut tx, account_id, character_id)?;
        let mut archive = revenant_challenges::mastery::Archive::default();
        for (event, entry) in journal {
            if entry.after.active.is_some() {
                continue;
            }
            let events = load_events(&mut tx, &event.session_id, event.id)?;
            let attempt = revenant_replay::challenge_mastery_attempt(&events)?
                .ok_or_else(|| invalid("mastery terminal lacks its verified attempt"))?;
            archive.observe(attempt);
        }
        if let Some(result) = &state.last_result {
            // Retry records the old interrupted attempt in its admission row;
            // it has no terminal event in the old session and earns no goal.
            if result.outcome != revenant_challenges::Outcome::Completed {
                archive.observe(revenant_challenges::mastery::assess(
                    result,
                    &revenant_challenges::mastery::Facts::default(),
                ));
            }
        }
        tx.commit()?;
        Ok((state, archive))
    }

    /// Reads the verified character journal, treating legacy saves as empty.
    ///
    /// # Errors
    /// Rejects foreign accounts, discontinuous history or invalid world proofs.
    pub fn challenge_state_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> ChallengePersistenceResult<ChallengeState> {
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

    /// Resolves operation retries before revision checks under the character
    /// lock, then commits a verified transition and any spatial proof together.
    /// No inventory, XP, campaign or completion reward is changed.
    ///
    /// # Errors
    /// Rejects stale/foreign requests, operation conflicts, unaccepted equipment,
    /// invalid world evidence and database failures without partial progress.
    pub fn apply_challenge_command(
        &mut self,
        character_id: &str,
        request: &ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
    ) -> ChallengePersistenceResult<ChallengeReceipt> {
        self.apply_challenge_with_combat(character_id, request, context, None)
    }

    /// Commits a server's fatal combat event and its challenge transition in one
    /// transaction. A transport retry compares the original evidence before
    /// checking revisions and cannot append another fatal hit.
    ///
    /// # Errors
    /// Rejects nonfatal/foreign evidence, inconsistent transitions and database
    /// failures, rolling back both the combat row and the checkpoint.
    pub fn apply_challenge_combat_command(
        &mut self,
        character_id: &str,
        request: &ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        evidence: &SupportEvidence,
    ) -> ChallengePersistenceResult<ChallengeReceipt> {
        self.apply_challenge_with_combat(
            character_id,
            request,
            context,
            Some(CombatProof::Support(evidence)),
        )
    }

    /// Commits an elite fatal attack or slam together with the challenge checkpoint.
    ///
    /// # Errors
    /// Rejects changed retry evidence, foreign actors, unproved kills or defeat;
    /// both the world event and checkpoint roll back on failure.
    pub fn apply_challenge_elite_command(
        &mut self,
        character_id: &str,
        request: &ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        evidence: &EliteEvidence,
    ) -> ChallengePersistenceResult<ChallengeReceipt> {
        self.apply_challenge_with_combat(
            character_id,
            request,
            context,
            Some(CombatProof::Elite(evidence)),
        )
    }

    /// Commits a Prism fatal attack or pattern and its challenge checkpoint.
    ///
    /// # Errors
    /// Rejects foreign targets, changed retries and unproved terminals atomically.
    pub fn apply_challenge_prism_command(
        &mut self,
        character_id: &str,
        request: &ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        boss_id: u64,
        evidence: &PrismEvidence,
    ) -> ChallengePersistenceResult<ChallengeReceipt> {
        self.apply_challenge_with_combat(
            character_id,
            request,
            context,
            Some(CombatProof::Prism { boss_id, evidence }),
        )
    }

    fn apply_challenge_with_combat(
        &mut self,
        character_id: &str,
        request: &ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        combat: Option<CombatProof<'_>>,
    ) -> ChallengePersistenceResult<ChallengeReceipt> {
        let mut tx = self.client.transaction()?;
        validate_module_replay_identity(context.session_id, context.activity_id, context.actor_id)?;
        if context.catalog_revision != Some(revenant_modules::BUILD_CATALOG_REVISION) {
            return Err(invalid("challenge admission requires the build catalog"));
        }
        ensure_character_account(&mut tx, character_id, context.account_id)?;
        ensure_and_lock_module_state(&mut tx, character_id)?;
        let (state, journal) = load_journal(&mut tx, context.account_id, character_id)?;
        validate_request_shape(request, combat)?;
        if let Some((event, entry)) = journal
            .iter()
            .find(|(_, e)| e.operation_id == request.operation_id)
        {
            validate_retry(&mut tx, event, entry, request, context)?;
            if let Some(evidence) = combat {
                validate_combat_retry(&mut tx, entry, context, evidence)?;
            }
            return Ok(ChallengeReceipt {
                replayed: true,
                transition: Transition {
                    state: entry.after.clone(),
                    first_completion: entry.first_completion,
                },
                state,
            });
        }
        let policy = policy_for(&state, &request.command);
        let transition = revenant_challenges::apply(
            policy,
            &state,
            request.expected_revision,
            &request.command,
        )?;
        let events = load_events(&mut tx, context.session_id, i64::MAX)?;
        let proof_event_id = resolve_proof(
            &mut tx,
            &events,
            character_id,
            &state,
            request,
            context,
            combat,
        )?;
        let payload = ChallengeReplayPayload {
            schema_version: 1,
            policy_revision: policy.to_owned(),
            character_id: character_id.to_owned(),
            operation_id: request.operation_id.to_owned(),
            before: state,
            command: request.command.clone(),
            after: transition.state.clone(),
            first_completion: transition.first_completion,
            proof_event_id,
        };
        let encoded = encode_challenge_payload(&payload)?;
        let id = insert_replay_event(
            &mut tx,
            &module_mutation_replay_event(context, "challenge_checkpoint", &encoded),
        )?;
        let replay = reconstruct(&load_events(&mut tx, context.session_id, id)?)?;
        if replay
            .challenge
            .as_ref()
            .is_none_or(|r| r.character_id != character_id || r.state != transition.state)
        {
            return Err(invalid(
                "challenge transition disagrees with session replay",
            ));
        }
        tx.commit()?;
        Ok(ChallengeReceipt {
            replayed: false,
            state: transition.state.clone(),
            transition,
        })
    }
}

fn resolve_proof(
    client: &mut impl GenericClient,
    events: &[ReplayEvent],
    character_id: &str,
    state: &ChallengeState,
    request: &ChallengeRequest<'_>,
    context: ModuleMutationReplayContext<'_>,
    combat: Option<CombatProof<'_>>,
) -> ChallengePersistenceResult<Option<i64>> {
    let proof = match &request.command {
        Command::Start { equipment, .. }
        | Command::StartModified { equipment, .. }
        | Command::Retry { equipment, .. } => {
            validate_equipment(client, character_id, equipment)?;
            Some(
                events
                    .iter()
                    .find(|e| e.kind == ReplayEventKind::ModuleStateSnapshot)
                    .ok_or_else(|| invalid("challenge has no module admission"))?
                    .id,
            )
        }
        _ => {
            if let Some(position) = request.position {
                let encoded = ChallengeVisitProof::for_policy(
                    policy_for(state, &request.command),
                    character_id,
                    state,
                    &request.command,
                    position,
                )?
                .encode()?;
                Some(insert_replay_event(
                    client,
                    &module_mutation_replay_event(context, "challenge_objective", &encoded),
                )?)
            } else if let Some(evidence) = combat {
                let encoded = evidence.encode()?;
                let (kind, actor) = evidence.identity(context.actor_id)?;
                let mut event = module_mutation_replay_event(context, kind, &encoded);
                event.actor_id = Some(actor);
                Some(insert_replay_event(client, &event)?)
            } else {
                request.proof_event_id
            }
        }
    };
    Ok(proof)
}

fn validate_request_shape(
    request: &ChallengeRequest<'_>,
    combat: Option<CombatProof<'_>>,
) -> ChallengePersistenceResult<()> {
    let visit = matches!(request.command, Command::Confirm { objective, .. } if objective.position().is_some());
    let existing_proof = matches!(request.command,
        Command::Confirm { objective, .. } if objective.position().is_none())
        || matches!(
            request.command,
            Command::End {
                reason: revenant_challenges::EndReason::Defeated,
                ..
            }
        );
    if visit != request.position.is_some()
        || existing_proof != (request.proof_event_id.is_some() || combat.is_some())
        || (request.proof_event_id.is_some() && combat.is_some())
    {
        return Err(invalid(
            "challenge request has incompatible world proof inputs",
        ));
    }
    Ok(())
}

fn validate_combat_retry(
    client: &mut impl GenericClient,
    entry: &ChallengeReplayPayload,
    context: ModuleMutationReplayContext<'_>,
    evidence: CombatProof<'_>,
) -> ChallengePersistenceResult<()> {
    let (kind, actor) = evidence.identity(context.actor_id)?;
    let row = client.query_one(
        "SELECT event_type, actor_id, payload FROM replay_events WHERE id = $1",
        &[&entry.proof_event_id],
    )?;
    if row.get::<_, String>(0) != kind
        || row.get::<_, Option<i64>>(1) != Some(actor)
        || row.get::<_, String>(2) != evidence.encode()?
    {
        return Err(invalid("challenge retry changed its fatal combat evidence"));
    }
    Ok(())
}

fn validate_equipment(
    client: &mut impl GenericClient,
    character: &str,
    equipment: &Equipment,
) -> ChallengePersistenceResult<()> {
    let modules = load_persisted_module_state(client, character)?;
    let weapon = client
        .query_opt(
            "SELECT weapon_item_id FROM equipment_loadouts WHERE character_id = $1 FOR SHARE",
            &[&character],
        )?
        .ok_or_else(|| invalid("challenge character has no equipped weapon"))?
        .get::<_, String>(0);
    let owned: bool = client.query_one(
        "SELECT EXISTS (SELECT 1 FROM inventory WHERE character_id = $1 AND item_id = $2 AND quantity > 0)",
        &[&character, &weapon])?.get(0);
    if !owned
        || equipment.weapon_id != weapon
        || equipment.modules != modules.loadout
        || equipment.loadout_revision != modules.revision
    {
        return Err(invalid(
            "challenge equipment differs from accepted character equipment",
        ));
    }
    Ok(())
}

fn validate_retry(
    client: &mut impl GenericClient,
    event: &ReplayEvent,
    entry: &ChallengeReplayPayload,
    request: &ChallengeRequest<'_>,
    context: ModuleMutationReplayContext<'_>,
) -> ChallengePersistenceResult<()> {
    if event.session_id != context.session_id
        || event.account_id != context.account_id
        || event.activity_id.as_deref() != Some(context.activity_id)
        || event.actor_id != u64::try_from(context.actor_id).ok()
        || entry.command != request.command
        || entry.before.revision != request.expected_revision
        || (request.proof_event_id.is_some() && request.proof_event_id != entry.proof_event_id)
    {
        return Err(invalid(
            "challenge operation was reused with different inputs",
        ));
    }
    if let Some(position) = request.position {
        let expected = ChallengeVisitProof::for_policy(
            &entry.policy_revision,
            &entry.character_id,
            &entry.before,
            &entry.command,
            position,
        )?;
        let proof = client
            .query_one(
                "SELECT payload FROM replay_events WHERE id = $1",
                &[&entry.proof_event_id],
            )?
            .get::<_, String>(0);
        if serde_json::from_str::<ChallengeVisitProof>(&proof)? != expected {
            return Err(invalid("challenge operation retry changed its position"));
        }
    }
    Ok(())
}

fn load_journal(
    client: &mut impl GenericClient,
    account: &str,
    character: &str,
) -> ChallengePersistenceResult<(ChallengeState, Journal)> {
    let rows = client.query(
        "SELECT id, event_type, occurred_at::TEXT, session_id, account_id, activity_id, actor_id, payload \
         FROM replay_events WHERE event_type = 'challenge_checkpoint' \
         AND payload::jsonb->>'character_id' = $1 ORDER BY id", &[&character])?;
    let mut state = ChallengeState::default();
    let mut journal = Vec::new();
    let mut operations = BTreeSet::new();
    for row in rows {
        let event = from_row(&row)?;
        let entry = decode_challenge_payload(&event.payload)?;
        if event.account_id != account
            || entry.character_id != character
            || entry.before != state
            || !operations.insert(entry.operation_id.clone())
        {
            return Err(invalid(
                "challenge journal changes owner, branches or reuses an operation",
            ));
        }
        let session = reconstruct(&load_events(client, &event.session_id, event.id)?)?;
        if session
            .challenge
            .is_none_or(|r| r.character_id != character || r.state != entry.after)
        {
            return Err(invalid("challenge journal and world replay disagree"));
        }
        state = entry.after.clone();
        journal.push((event, entry));
    }
    Ok((state, journal))
}

fn invalid(message: &'static str) -> Box<dyn Error + Send + Sync> {
    io::Error::other(message).into()
}

fn policy_for(state: &ChallengeState, command: &Command) -> &'static str {
    [
        REVISION,
        EXPANDED_REVISION,
        ELITE_REVISION,
        PRISM_REVISION,
        SIGNAL_REVISION,
        SURVIVAL_REVISION,
        GAUNTLET_REVISION,
        MODIFIER_REVISION,
    ]
    .into_iter()
    .find(|policy| {
        state.supports_policy(policy)
            && (!matches!(command, Command::StartModified { .. }) || *policy == MODIFIER_REVISION)
            && !matches!(command, Command::Start { contract, .. } if !contract.available_in(policy))
    })
    // The transition validator rejects invalid states; the latest policy
    // is the only possible fallback when none of the frozen policies fit.
    .unwrap_or(MODIFIER_REVISION)
}
