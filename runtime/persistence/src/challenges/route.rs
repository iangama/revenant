//! A modified Meridian move is one transaction with its linked station checkpoint.
//! (attempt, sequence) provides retry identity even for steps without a checkpoint.
use revenant_challenges::ContractId;
use revenant_replay::RouteEvidence;

use super::{
    encode_challenge_payload, ensure_and_lock_module_state, ensure_character_account,
    insert_replay_event, invalid, load_events, load_journal, module_mutation_replay_event,
    policy_for, reconstruct, validate_module_replay_identity, ChallengePersistenceResult,
    ChallengeReceipt, ChallengeReplayPayload, ChallengeState, GenericClient,
    ModuleMutationReplayContext, Persistence, ReplayEventKind, Transition,
};

impl Persistence {
    /// Stores a position/time step with its linked station checkpoint.
    /// Replay proves the admitted route, clock and objective order before commit.
    /// Callers publish movement and completion only after success.
    ///
    /// # Errors
    /// Rejects stale/foreign attempts, missing or reordered steps, changed retry
    /// evidence and unproved movement, time or objectives. Database failures roll back both rows.
    pub fn apply_challenge_route_step(
        &mut self,
        character_id: &str,
        evidence: &RouteEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> ChallengePersistenceResult<ChallengeReceipt> {
        validate_module_replay_identity(context.session_id, context.activity_id, context.actor_id)?;
        if context.catalog_revision != Some(revenant_modules::BUILD_CATALOG_REVISION)
            || context.activity_id != ContractId::MeridianCircuit.activity_id()
        {
            return Err(invalid(
                "route step requires its admitted build and activity",
            ));
        }
        let encoded = evidence.encode()?;
        let mut tx = self.client.transaction()?;
        ensure_character_account(&mut tx, character_id, context.account_id)?;
        ensure_and_lock_module_state(&mut tx, character_id)?;
        let (state, journal) = load_journal(&mut tx, context.account_id, character_id)?;
        let events = load_events(&mut tx, context.session_id, i64::MAX)?;
        let current = reconstruct(&events)?;
        if current
            .challenge
            .as_ref()
            .is_none_or(|challenge| challenge.character_id != character_id)
            || events
                .iter()
                .find(|e| e.kind == ReplayEventKind::ChallengeCheckpoint)
                .is_none_or(|e| e.actor_id != u64::try_from(context.actor_id).ok())
        {
            return Err(invalid(
                "route evidence belongs to another admitted participant",
            ));
        }
        if let Some(existing) = events.iter().find(|event| {
            RouteEvidence::decode(&event.payload).is_ok_and(|proof| {
                proof.run_id == evidence.run_id && proof.sequence == evidence.sequence
            })
        }) {
            if existing.payload != encoded
                || existing.account_id != context.account_id
                || existing.actor_id != Some(evidence.actor_id)
                || existing.activity_id.as_deref() != Some(context.activity_id)
                || existing.kind != evidence.kind()
            {
                return Err(invalid(
                    "route step retry changed its evidence or participant",
                ));
            }
            let transition = journal
                .iter()
                .find(|(_, entry)| entry.proof_event_id == Some(existing.id))
                .map_or_else(
                    || Transition {
                        state: state.clone(),
                        first_completion: None,
                    },
                    |(_, entry)| Transition {
                        state: entry.after.clone(),
                        first_completion: entry.first_completion,
                    },
                );
            return Ok(ChallengeReceipt {
                replayed: true,
                transition,
                state,
            });
        }
        if state.active.as_ref().is_none_or(|run| {
            run.run_id != evidence.run_id || run.rules.contract != ContractId::MeridianCircuit
        }) || current.challenge.is_none_or(|challenge| {
            challenge.character_id != character_id || challenge.state != state
        }) {
            return Err(invalid("route step belongs to another active attempt"));
        }
        let operation = format!("route-{}-{}", evidence.run_id, evidence.sequence);
        if evidence.command().is_some()
            && journal
                .iter()
                .any(|(_, entry)| entry.operation_id == operation)
        {
            return Err(invalid(
                "route checkpoint operation is already used by another event",
            ));
        }
        let proof = insert_step(&mut tx, evidence, context, &encoded)?;
        let transition =
            commit_checkpoint(&mut tx, character_id, &state, evidence, proof, context)?;
        let replay = reconstruct(&load_events(&mut tx, context.session_id, i64::MAX)?)?;
        if replay.challenge.is_none_or(|challenge| {
            challenge.state != transition.state || challenge.character_id != character_id
        }) {
            return Err(invalid("route step disagrees with accepted world replay"));
        }
        tx.commit()?;
        Ok(ChallengeReceipt {
            replayed: false,
            state: transition.state.clone(),
            transition,
        })
    }
}

fn commit_checkpoint(
    tx: &mut impl GenericClient,
    character: &str,
    state: &ChallengeState,
    evidence: &RouteEvidence,
    proof: i64,
    context: ModuleMutationReplayContext<'_>,
) -> ChallengePersistenceResult<Transition> {
    let rules = &state
        .active
        .as_ref()
        .ok_or_else(|| invalid("route run missing"))?
        .rules;
    let Some(command) = evidence.command_for(rules) else {
        return Ok(Transition {
            state: state.clone(),
            first_completion: None,
        });
    };
    let policy = policy_for(state, &command);
    let transition = revenant_challenges::apply(policy, state, state.revision, &command)?;
    let payload = ChallengeReplayPayload {
        schema_version: 1,
        policy_revision: policy.into(),
        character_id: character.into(),
        operation_id: format!("route-{}-{}", evidence.run_id, evidence.sequence),
        before: state.clone(),
        command,
        after: transition.state.clone(),
        first_completion: transition.first_completion,
        proof_event_id: Some(proof),
    };
    insert_replay_event(
        tx,
        &module_mutation_replay_event(
            context,
            "challenge_checkpoint",
            &encode_challenge_payload(&payload)?,
        ),
    )?;
    Ok(transition)
}

fn insert_step(
    tx: &mut impl GenericClient,
    evidence: &RouteEvidence,
    context: ModuleMutationReplayContext<'_>,
    encoded: &str,
) -> ChallengePersistenceResult<i64> {
    let kind = match evidence.kind() {
        ReplayEventKind::EnemySpawned => "enemy_spawned",
        ReplayEventKind::BossSpawned => "boss_spawned",
        ReplayEventKind::EnemyDied => "enemy_died",
        _ => "field_activity",
    };
    let mut event = module_mutation_replay_event(context, kind, encoded);
    event.actor_id = Some(i64::try_from(evidence.actor_id)?);
    Ok(insert_replay_event(tx, &event)?)
}
