use super::{
    ensure_and_lock_module_state, ensure_character_account, insert_replay_event,
    load_persisted_module_state, module_mutation_replay_event,
    validate_module_mutation_replay_context, GenericClient, ModuleMutationReplayContext,
    ModulePersistenceError, Persistence, RELAY_CORE_FRAGMENT,
};
use revenant_modules::acquisition::{
    AcquisitionError, AcquisitionState, ArcId, Grant, Milestone, REVISION,
};
use revenant_replay::{
    acquisition_milestones, decode_acquisition_claimed, encode_acquisition_claimed, reconstruct,
    AcquisitionClaimedPayloadV1, AcquisitionMilestoneProof, ReplayError, ReplayEvent,
    ReplayEventKind,
};
use std::fmt;

#[derive(Debug)]
pub enum AcquisitionPersistenceError {
    Database(postgres::Error),
    Domain(AcquisitionError),
    Module(ModulePersistenceError),
    Replay(ReplayError),
    InvalidEvidence(&'static str),
}

impl fmt::Display for AcquisitionPersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(e) => e.fmt(f),
            Self::Domain(e) => e.fmt(f),
            Self::Module(e) => e.fmt(f),
            Self::Replay(e) => e.fmt(f),
            Self::InvalidEvidence(message) => write!(f, "invalid acquisition evidence: {message}"),
        }
    }
}

impl std::error::Error for AcquisitionPersistenceError {}

impl From<postgres::Error> for AcquisitionPersistenceError {
    fn from(e: postgres::Error) -> Self {
        Self::Database(e)
    }
}
impl From<AcquisitionError> for AcquisitionPersistenceError {
    fn from(e: AcquisitionError) -> Self {
        Self::Domain(e)
    }
}
impl From<ModulePersistenceError> for AcquisitionPersistenceError {
    fn from(e: ModulePersistenceError) -> Self {
        Self::Module(e)
    }
}
impl From<ReplayError> for AcquisitionPersistenceError {
    fn from(e: ReplayError) -> Self {
        Self::Replay(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersistedAcquisitionState {
    pub proofs: Vec<AcquisitionMilestoneProof>,
    pub claimed: Vec<ArcId>,
}

impl PersistedAcquisitionState {
    /// Rebuilds the fixed policy from verified durable records.
    ///
    /// # Errors
    /// Rejects a claim without all required milestones.
    pub fn domain(&self) -> Result<AcquisitionState, AcquisitionError> {
        AcquisitionState::new(
            &self.proofs.iter().map(|p| p.milestone).collect::<Vec<_>>(),
            &self.claimed,
        )
    }
}

impl Persistence {
    /// Reads and verifies commission proofs without materializing legacy rows.
    ///
    /// # Errors
    /// Rejects foreign identity, invalid evidence or database failures.
    pub fn acquisition_state_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> Result<PersistedAcquisitionState, AcquisitionPersistenceError> {
        let mut tx = self
            .client
            .build_transaction()
            .isolation_level(postgres::IsolationLevel::RepeatableRead)
            .start()?;
        ensure_character_account(&mut tx, character_id, account_id)?;
        let state = load_state(&mut tx, account_id, character_id)?;
        tx.commit()?;
        Ok(state)
    }

    /// Imports eligible historical completions, including missions played before
    /// commissions existed. No progress or balance is accepted from the client.
    ///
    /// # Errors
    /// Rejects invalid ownership, corrupt replay or database failures atomically.
    pub fn refresh_acquisition_progress(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> Result<PersistedAcquisitionState, AcquisitionPersistenceError> {
        let mut tx = self.client.transaction()?;
        ensure_character_account(&mut tx, character_id, account_id)?;
        ensure_and_lock_module_state(&mut tx, character_id)?;
        let sessions = tx.query(
            "SELECT DISTINCT grant_row.session_id FROM inventory_reward_grants AS grant_row \
             WHERE grant_row.character_id = $1 AND grant_row.item_id = $2 \
             ORDER BY grant_row.session_id",
            &[&character_id, &RELAY_CORE_FRAGMENT],
        )?;
        for row in sessions {
            record_session(&mut tx, account_id, character_id, &row.get::<_, String>(0))?;
        }
        let state = load_state(&mut tx, account_id, character_id)?;
        tx.commit()?;
        Ok(state)
    }

    /// Records the four finite milestones from one server-owned completion.
    ///
    /// # Errors
    /// Rejects invalid evidence; a failed write leaves no partial progress.
    pub fn record_acquisition_progress(
        &mut self,
        account_id: &str,
        character_id: &str,
        session_id: &str,
    ) -> Result<PersistedAcquisitionState, AcquisitionPersistenceError> {
        let mut tx = self.client.transaction()?;
        ensure_character_account(&mut tx, character_id, account_id)?;
        ensure_and_lock_module_state(&mut tx, character_id)?;
        record_session(&mut tx, account_id, character_id, session_id)?;
        let state = load_state(&mut tx, account_id, character_id)?;
        tx.commit()?;
        Ok(state)
    }

    /// Commits the natural character/arc claim identity, inventory and typed
    /// replay together. A retry returns zero and the current balance, even after
    /// crafting or reconnecting; it never restores the original grant balance.
    ///
    /// # Errors
    /// Rejects incomplete requirements, invalid session ownership, overflow,
    /// inconsistent evidence or any database failure without consuming the claim.
    pub fn claim_acquisition_with_replay(
        &mut self,
        character_id: &str,
        arc_id: ArcId,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<Grant, AcquisitionPersistenceError> {
        let mut tx = self.client.transaction()?;
        ensure_character_account(&mut tx, character_id, context.account_id)?;
        ensure_and_lock_module_state(&mut tx, character_id)?;
        validate_module_mutation_replay_context(&mut tx, character_id, context)?;
        // Shares the module lock with crafting and the inventory row lock with
        // normal mission rewards. Read the balance only after both are held.
        tx.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, $2, 0) \
             ON CONFLICT (character_id, item_id) DO NOTHING",
            &[&character_id, &RELAY_CORE_FRAGMENT],
        )?;
        tx.query_one(
            "SELECT quantity FROM inventory WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
            &[&character_id, &RELAY_CORE_FRAGMENT],
        )?;
        let persisted = load_persisted_module_state(&mut tx, character_id)?;
        let state = load_state(&mut tx, context.account_id, character_id)?;
        let grant = state.domain()?.claim(arc_id, persisted.fragments)?;
        let events = load_events(&mut tx, context.session_id, i64::MAX)?;
        validate_claim_context(&events, character_id, context, persisted.fragments)?;
        if grant.replayed {
            tx.commit()?;
            return Ok(grant);
        }
        let proofs = state
            .proofs
            .into_iter()
            .filter(|p| arc_id.requirements().contains(&p.milestone))
            .collect();
        let payload = encode_acquisition_claimed(&AcquisitionClaimedPayloadV1 {
            schema_version: 1,
            revision: REVISION.to_owned(),
            character_id: character_id.to_owned(),
            arc_id,
            proofs,
            previous_fragments: persisted.fragments,
            granted_fragments: grant.granted_fragments,
            resulting_fragments: grant.resulting_fragments,
        })?;
        let quantity = i32::try_from(grant.resulting_fragments)
            .map_err(|_| AcquisitionError::FragmentOverflow)?;
        tx.execute(
            "UPDATE inventory SET quantity = $3 WHERE character_id = $1 AND item_id = $2",
            &[&character_id, &RELAY_CORE_FRAGMENT, &quantity],
        )?;
        let replay_id = insert_replay_event(
            &mut tx,
            &module_mutation_replay_event(context, "acquisition_claimed", &payload),
        )?;
        // Validate the appended event before exposing any of the transaction.
        reconstruct(&load_events(&mut tx, context.session_id, replay_id)?)?;
        let granted = i32::try_from(grant.granted_fragments)
            .map_err(|_| AcquisitionError::FragmentOverflow)?;
        tx.execute(
            "INSERT INTO acquisition_claims (character_id, arc_id, replay_event_id, quantity) \
             VALUES ($1, $2, $3, $4)",
            &[&character_id, &arc_id.as_str(), &replay_id, &granted],
        )?;
        tx.commit()?;
        Ok(grant)
    }
}

fn validate_claim_context(
    events: &[ReplayEvent],
    character_id: &str,
    context: ModuleMutationReplayContext<'_>,
    fragments: u32,
) -> Result<(), AcquisitionPersistenceError> {
    let state = reconstruct(events)?;
    let participant = state
        .module_participants
        .iter()
        .find(|p| p.character_id == character_id)
        .ok_or(AcquisitionPersistenceError::InvalidEvidence(
            "claim character has no snapshot",
        ))?;
    if !state.completed
        || participant.protocol_generation != revenant_replay::ReplayProtocolGeneration::V2
        || i64::try_from(participant.actor_id).ok() != Some(context.actor_id)
        || participant.state.fragments != fragments
        || !events.iter().any(|e| {
            e.kind == ReplayEventKind::ModuleStateSnapshot
                && e.actor_id == Some(participant.actor_id)
                && e.account_id == context.account_id
                && e.activity_id.as_deref() == Some(context.activity_id)
        })
    {
        return Err(AcquisitionPersistenceError::InvalidEvidence(
            "claim session or inventory disagrees",
        ));
    }
    Ok(())
}

fn record_session(
    client: &mut impl GenericClient,
    account_id: &str,
    character_id: &str,
    session_id: &str,
) -> Result<(), AcquisitionPersistenceError> {
    let events = load_events(client, session_id, i64::MAX)?;
    if events.is_empty() {
        return Ok(());
    }
    let milestones = verified_milestones(client, account_id, character_id, &events)?;
    let end_id = events.last().expect("nonempty proof").id;
    for milestone in milestones {
        client.execute(
            "INSERT INTO acquisition_milestones (character_id, milestone, proof_session_id, proof_event_id) \
             VALUES ($1, $2, $3, $4) ON CONFLICT (character_id, milestone) DO NOTHING",
            &[&character_id, &milestone.as_str(), &session_id, &end_id],
        )?;
    }
    Ok(())
}

fn verified_milestones(
    client: &mut impl GenericClient,
    account_id: &str,
    character_id: &str,
    events: &[ReplayEvent],
) -> Result<Vec<Milestone>, AcquisitionPersistenceError> {
    let milestones = acquisition_milestones(events, character_id)?;
    if milestones.is_empty() {
        return Ok(milestones);
    }
    let state = reconstruct(events)?;
    let actor_id = state
        .module_participants
        .iter()
        .find(|p| p.character_id == character_id)
        .ok_or(AcquisitionPersistenceError::InvalidEvidence(
            "missing proof participant",
        ))?
        .actor_id;
    let wrong_owner = events
        .iter()
        .any(|e| e.actor_id == Some(actor_id) && e.account_id != account_id);
    let has_grant: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM inventory_reward_grants \
         WHERE session_id = $1 AND character_id = $2 AND item_id = $3)",
            &[&state.session_id, &character_id, &RELAY_CORE_FRAGMENT],
        )?
        .get(0);
    if wrong_owner || !has_grant {
        return Err(AcquisitionPersistenceError::InvalidEvidence(
            "proof owner or mission reward missing",
        ));
    }
    Ok(milestones)
}

fn load_state(
    client: &mut impl GenericClient,
    account_id: &str,
    character_id: &str,
) -> Result<PersistedAcquisitionState, AcquisitionPersistenceError> {
    let mut state = PersistedAcquisitionState::default();
    for row in client.query(
        "SELECT milestone, proof_session_id, proof_event_id FROM acquisition_milestones \
         WHERE character_id = $1 ORDER BY milestone",
        &[&character_id],
    )? {
        let name: String = row.get(0);
        let milestone = Milestone::ALL
            .into_iter()
            .find(|m| m.as_str() == name)
            .ok_or(AcquisitionPersistenceError::InvalidEvidence(
                "unknown milestone",
            ))?;
        let proof = AcquisitionMilestoneProof {
            milestone,
            session_id: row.get(1),
            through_event_id: row.get(2),
        };
        let events = load_events(client, &proof.session_id, proof.through_event_id)?;
        if events.last().map(|e| e.id) != Some(proof.through_event_id)
            || !verified_milestones(client, account_id, character_id, &events)?.contains(&milestone)
        {
            return Err(AcquisitionPersistenceError::InvalidEvidence(
                "stored milestone proof does not qualify",
            ));
        }
        state.proofs.push(proof);
    }
    for row in client.query(
        "SELECT c.arc_id, c.quantity, e.id, e.event_type, e.session_id, e.account_id, e.payload \
         FROM acquisition_claims c JOIN replay_events e ON e.id = c.replay_event_id \
         WHERE c.character_id = $1 ORDER BY c.arc_id",
        &[&character_id],
    )? {
        let arc: String = row.get(0);
        let payload = decode_acquisition_claimed(&row.get::<_, String>(6))?;
        if payload.character_id != character_id
            || payload.arc_id.as_str() != arc
            || i32::try_from(payload.granted_fragments).ok() != Some(row.get(1))
            || row.get::<_, String>(3) != "acquisition_claimed"
            || row.get::<_, String>(5) != account_id
            || payload.proofs.iter().any(|p| !state.proofs.contains(p))
        {
            return Err(AcquisitionPersistenceError::InvalidEvidence(
                "claim ledger and replay disagree",
            ));
        }
        reconstruct(&load_events(client, &row.get::<_, String>(4), row.get(2))?)?;
        state.claimed.push(payload.arc_id);
    }
    state.domain()?;
    Ok(state)
}

fn load_events(
    client: &mut impl GenericClient,
    session_id: &str,
    through_event_id: i64,
) -> Result<Vec<ReplayEvent>, AcquisitionPersistenceError> {
    client.query(
        "SELECT id, event_type, occurred_at::TEXT, session_id, account_id, activity_id, actor_id, payload \
         FROM replay_events WHERE session_id = $1 AND id <= $2 ORDER BY id",
        &[&session_id, &through_event_id],
    )?.into_iter().map(|row| Ok(ReplayEvent {
        id: row.get(0),
        kind: row.get::<_, String>(1).parse().map_err(|_| AcquisitionPersistenceError::InvalidEvidence("unknown event kind"))?,
        timestamp: row.get(2), session_id: row.get(3), account_id: row.get(4), activity_id: row.get(5),
        actor_id: row.get::<_, Option<i64>>(6).map(u64::try_from).transpose()
            .map_err(|_| AcquisitionPersistenceError::InvalidEvidence("negative actor identifier"))?,
        payload: row.get(7),
    })).collect()
}
