use postgres::{GenericClient, Transaction};
use revenant_cooperation::{
    CombatProfile, CooperationError, CooperationState, LifeState, MutationDisposition, Phase, Role,
    Target, TerminalOutcome, CATALOG_REVISION as COOPERATION_CATALOG_REVISION,
    MAX_REVIVE_DISTANCE_SQUARED, OPERATION_DURATION_MS, PARTICIPANT_COUNT, PING_TTL_MS,
    REVIVE_CHANNEL_MS, REVIVE_HEALTH, REVIVE_WINDOW_MS, SUCCESS_REWARD,
};
use revenant_inventory::{weapon_profile, RELAY_CORE_FRAGMENT};
use revenant_modules::{
    resolve_build, BaseCombatProfile, ModuleId, CATALOG_REVISION as MODULE_CATALOG_REVISION,
};
use revenant_progression::ExperienceReward;
use revenant_replay::{
    decode_module_payload, encode_cooperation_failed, encode_cooperation_pinged,
    encode_cooperation_started, encode_cooperation_succeeded, encode_player_downed,
    encode_player_revived, reconstruct, CooperationPingedPayloadV1, CooperationReplayPrefix,
    CooperationStartedPayloadV1, CooperationTerminalPayloadV1, DecodedModuleReplayPayload,
    PlayerDownedPayloadV1, PlayerRevivedPayloadV1, ReconstructedSession,
    ReplayCooperationContributionEvidence, ReplayCooperationGrantEvidence, ReplayCooperationHazard,
    ReplayCooperationParticipantEvidence, ReplayCooperationRewardEvidence,
    ReplayCooperationTargetEvidence, ReplayCooperationTerminalParticipantEvidence,
    ReplayCooperationTimingEvidence, ReplayError, ReplayEvent, ReplayEventKind,
    ReplayProtocolGeneration, COOPERATION_REPLAY_SCHEMA_VERSION,
};
use serde::Deserialize;
use std::fmt::{self, Display, Formatter};

use super::{
    apply_completion_reward, insert_replay_event, ActivityCompletion, NewReplayEvent, Persistence,
};

const ACTIVITY_ID: &str = "relay_awakening";

#[derive(Debug, Clone, Copy)]
pub struct CooperationParticipant<'a> {
    pub account_id: &'a str,
    pub character_id: &'a str,
    pub actor_id: u64,
    pub weapon_item_id: &'a str,
    pub modules: &'a [ModuleId],
    pub profile: CombatProfile,
    pub current_health: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct CooperationOperationInput<'a> {
    pub session_id: &'a str,
    pub activity_id: &'a str,
    pub requester_account_id: &'a str,
    pub operation_id: &'a str,
    pub participants: &'a [CooperationParticipant<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperationReceipt {
    pub disposition: MutationDisposition,
    pub state: CooperationState,
}

#[derive(Debug)]
pub enum CooperationPersistenceError {
    Database(postgres::Error),
    Domain(CooperationError),
    Replay(ReplayError),
    Serialization(serde_json::Error),
    InvalidPersistedState(String),
}

impl Display for CooperationPersistenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => error.fmt(formatter),
            Self::Domain(error) => error.fmt(formatter),
            Self::Replay(error) => error.fmt(formatter),
            Self::Serialization(error) => error.fmt(formatter),
            Self::InvalidPersistedState(message) => {
                write!(
                    formatter,
                    "invalid persisted cooperation operation: {message}"
                )
            }
        }
    }
}

impl std::error::Error for CooperationPersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Domain(error) => Some(error),
            Self::Replay(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::InvalidPersistedState(_) => None,
        }
    }
}

impl From<postgres::Error> for CooperationPersistenceError {
    fn from(error: postgres::Error) -> Self {
        Self::Database(error)
    }
}

impl From<CooperationError> for CooperationPersistenceError {
    fn from(error: CooperationError) -> Self {
        Self::Domain(error)
    }
}

impl From<ReplayError> for CooperationPersistenceError {
    fn from(error: ReplayError) -> Self {
        Self::Replay(error)
    }
}

impl From<serde_json::Error> for CooperationPersistenceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
struct StoredParent {
    session_id: String,
    activity_id: String,
    start_operation_id: String,
    catalog_revision: String,
    anchor_account_id: String,
    participant_count: i16,
    phase: String,
    operation_duration_ms: i64,
    ping_ttl_ms: i64,
    revive_window_ms: i64,
    revive_channel_ms: i64,
    revive_health: i32,
    max_revive_distance_squared: i64,
    reward_item_id: String,
    reward_quantity: i32,
    reward_experience: i64,
    anchor_arrived: bool,
    anchor_elapsed_ms: Option<i64>,
    pinged: bool,
    ping_operation_id: Option<String>,
    ping_target: Option<String>,
    ping_elapsed_ms: Option<i64>,
    runner_arrived: bool,
    runner_elapsed_ms: Option<i64>,
    downed_elapsed_ms: Option<i64>,
    revive_operation_id: Option<String>,
    revive_target: Option<String>,
    revive_started_elapsed_ms: Option<i64>,
    revived: bool,
    revive_completed_elapsed_ms: Option<i64>,
    revive_count: i16,
    warden_completed: bool,
    warden_elapsed_ms: Option<i64>,
    terminal_outcome: Option<String>,
    terminal_elapsed_ms: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct StoredParticipant {
    participant_index: i16,
    role: String,
    account_id: String,
    character_id: String,
    actor_id: i64,
    weapon_item_id: String,
    module_loadout: Vec<ModuleId>,
    damage: i32,
    range: i32,
    cooldown_ms: i64,
    max_health: i32,
    admitted_health: i32,
    current_health: i32,
    life_state: String,
}

#[derive(Debug, Clone)]
struct StoredOperation {
    parent: StoredParent,
    participants: [StoredParticipant; PARTICIPANT_COUNT],
    state: CooperationState,
}

impl Persistence {
    /// Persists exactly two ordered cooperation participants and one pure start.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid authority, identity, build/profile/health,
    /// conflicting retry, corrupt stored evidence, or database failure.
    pub fn start_cooperation_operation(
        &mut self,
        input: &CooperationOperationInput<'_>,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.start_cooperation_operation_internal(input, false)
    }

    /// Persists the cooperation start and its canonical replay atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for missing/mismatched M26 admission snapshots,
    /// conflicting replay mode or evidence, invalid input, or database failure.
    pub fn start_cooperation_operation_with_replay(
        &mut self,
        input: &CooperationOperationInput<'_>,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.start_cooperation_operation_internal(input, true)
    }

    fn start_cooperation_operation_internal(
        &mut self,
        input: &CooperationOperationInput<'_>,
        append_replay: bool,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        validate_storage_id(input.session_id, "session")?;
        let supplied = validate_start_input(input)?;
        let profiles = supplied.map(|participant| participant.profile);
        let health = supplied.map(|participant| participant.current_health);
        let state = CooperationState::start_with_health(input.operation_id, profiles, health)?;
        let mut transaction = self.client.transaction()?;
        lock_session(&mut transaction, input.session_id)?;
        if let Some(stored) = load_operation(&mut transaction, input.session_id, true)? {
            validate_start_retry(&stored, input, &supplied)?;
            validate_cooperation_replay_mode(&mut transaction, input.session_id, append_replay)?;
            if append_replay {
                let reconstructed =
                    reconstruct_cooperation_stream(&mut transaction, input.session_id)?;
                validate_started_replay_matches_storage(&reconstructed, &stored)?;
                validate_exact_cooperation_event(
                    &mut transaction,
                    &stored,
                    ReplayEventKind::CooperationStarted,
                    Role::Anchor,
                    &encode_cooperation_started(
                        reconstructed
                            .cooperation
                            .started
                            .as_ref()
                            .ok_or_else(|| invalid_state("cooperation start replay is missing"))?,
                    )?,
                )?;
            }
            transaction.commit()?;
            return Ok(CooperationReceipt {
                disposition: MutationDisposition::Replayed,
                state: stored.state,
            });
        }
        validate_participant_identity(&mut transaction, &supplied)?;
        let start_replay = append_replay
            .then(|| build_started_replay(&mut transaction, input, &supplied))
            .transpose()?;
        validate_new_cooperation_replay_mode(&mut transaction, input.session_id)?;
        insert_start(&mut transaction, input, &supplied)?;
        if let Some((payload, encoded)) = start_replay {
            insert_cooperation_event(
                &mut transaction,
                input.session_id,
                input.activity_id,
                supplied[0].account_id,
                supplied[0].actor_id,
                ReplayEventKind::CooperationStarted,
                &encoded,
            )?;
            let reconstructed = reconstruct_cooperation_stream(&mut transaction, input.session_id)?;
            if reconstructed.cooperation.started.as_ref() != Some(&payload) {
                return Err(invalid_state(
                    "committed cooperation start differs from replay reconstruction",
                ));
            }
        }
        let stored = load_operation(&mut transaction, input.session_id, true)?
            .ok_or_else(|| invalid_state("cooperation start disappeared before commit"))?;
        if stored.state != state {
            return Err(invalid_state("stored start differs from pure domain"));
        }
        transaction.commit()?;
        Ok(CooperationReceipt {
            disposition: MutationDisposition::Applied,
            state,
        })
    }

    /// Persists the fixed anchor arrival.
    ///
    /// # Errors
    ///
    /// Returns an error for missing/corrupt state, conflicting elapsed input,
    /// domain rejection, or database failure.
    pub fn arrive_cooperation_anchor(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.mutate_cooperation(session_id, elapsed_ms, None, |transaction, stored, _| {
            if let Some(existing) = stored.parent.anchor_elapsed_ms {
                stored.state.observe_deadlines(elapsed_ms)?;
                require_same_elapsed(existing, elapsed_ms)?;
                return Ok(MutationDisposition::Replayed);
            }
            let disposition =
                stored
                    .state
                    .arrive_anchor(Role::Anchor, Target::RelayAnchor, elapsed_ms)?;
            let elapsed = elapsed_i64(elapsed_ms)?;
            require_one(transaction.execute(
                "UPDATE cooperation_operations SET phase = 'awaiting_ping', \
                 anchor_arrived = TRUE, anchor_elapsed_ms = $2 \
                 WHERE session_id = $1 AND terminal_outcome IS NULL",
                &[&session_id, &elapsed],
            )?)?;
            Ok(disposition)
        })
    }

    /// Persists the one fixed contextual ping.
    ///
    /// # Errors
    ///
    /// Returns an error for authority/order/deadline/identifier rejection,
    /// conflicting retry, corrupt state, or database failure.
    pub fn ping_cooperation_operation(
        &mut self,
        session_id: &str,
        operation_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.ping_cooperation_operation_internal(session_id, operation_id, elapsed_ms, false)
    }

    /// Persists the accepted ping and canonical replay in one transaction.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-replay operation, replay mismatch, domain
    /// rejection, corrupt persisted state, or database failure.
    pub fn ping_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        operation_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.ping_cooperation_operation_internal(session_id, operation_id, elapsed_ms, true)
    }

    fn ping_cooperation_operation_internal(
        &mut self,
        session_id: &str,
        operation_id: &str,
        elapsed_ms: u64,
        append_replay: bool,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.mutate_cooperation(
            session_id,
            elapsed_ms,
            Some(append_replay),
            |transaction, stored, replay_mode| {
                let disposition = stored.state.ping(
                    Role::Anchor,
                    Target::RelayConsole,
                    operation_id,
                    elapsed_ms,
                )?;
                if disposition == MutationDisposition::Applied {
                    let elapsed = elapsed_i64(elapsed_ms)?;
                    require_one(transaction.execute(
                        "UPDATE cooperation_operations SET phase = 'awaiting_runner', \
                     pinged = TRUE, ping_operation_id = $2, ping_target = 'relay_console', \
                     ping_elapsed_ms = $3 WHERE session_id = $1 AND terminal_outcome IS NULL",
                        &[&session_id, &operation_id, &elapsed],
                    )?)?;
                }
                if replay_mode {
                    persist_or_validate_ping_replay(
                        transaction,
                        stored,
                        operation_id,
                        elapsed_ms,
                        disposition,
                    )?;
                }
                Ok(disposition)
            },
        )
    }

    /// Persists runner console contribution and exact scripted downing.
    ///
    /// # Errors
    ///
    /// Returns an error for order/deadline/life rejection, conflicting elapsed
    /// retry, corrupt state, or database failure.
    pub fn down_cooperation_runner(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.down_cooperation_runner_internal(session_id, elapsed_ms, false)
    }

    /// Persists scripted runner downing and its canonical replay atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-replay operation, replay mismatch, domain
    /// rejection, corrupt persisted state, or database failure.
    pub fn down_cooperation_runner_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.down_cooperation_runner_internal(session_id, elapsed_ms, true)
    }

    fn down_cooperation_runner_internal(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        append_replay: bool,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.mutate_cooperation(
            session_id,
            elapsed_ms,
            Some(append_replay),
            |transaction, stored, replay_mode| {
                if let Some(existing) = stored.parent.runner_elapsed_ms {
                    stored.state.observe_deadlines(elapsed_ms)?;
                    require_same_elapsed(existing, elapsed_ms)?;
                    if replay_mode {
                        persist_or_validate_down_replay(
                            transaction,
                            stored,
                            elapsed_ms,
                            MutationDisposition::Replayed,
                        )?;
                    }
                    return Ok(MutationDisposition::Replayed);
                }
                let disposition =
                    stored
                        .state
                        .arrive_runner(Role::Runner, Target::RelayConsole, elapsed_ms)?;
                let elapsed = elapsed_i64(elapsed_ms)?;
                require_one(transaction.execute(
                    "UPDATE cooperation_operations SET phase = 'runner_downed', \
                 runner_arrived = TRUE, runner_elapsed_ms = $2, downed_elapsed_ms = $2 \
                 WHERE session_id = $1 AND terminal_outcome IS NULL",
                    &[&session_id, &elapsed],
                )?)?;
                require_one(transaction.execute(
                    "UPDATE cooperation_operation_participants SET current_health = 0, \
                 life_state = 'downed' WHERE session_id = $1 AND participant_index = 1",
                    &[&session_id],
                )?)?;
                if replay_mode {
                    persist_or_validate_down_replay(transaction, stored, elapsed_ms, disposition)?;
                }
                Ok(disposition)
            },
        )
    }

    /// Persists one authoritative revive-channel start.
    ///
    /// # Errors
    ///
    /// Returns an error for authority/range/deadline/identifier/order rejection,
    /// conflicting retry, corrupt state, or database failure.
    pub fn start_cooperation_revive(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.mutate_cooperation(session_id, elapsed_ms, None, |transaction, stored, _| {
            let disposition = stored.state.start_revive(
                Role::Anchor,
                Role::Runner,
                distance_squared,
                operation_id,
                elapsed_ms,
            )?;
            if disposition == MutationDisposition::Replayed {
                return Ok(disposition);
            }
            let elapsed = elapsed_i64(elapsed_ms)?;
            require_one(transaction.execute(
                "UPDATE cooperation_operations SET phase = 'revive_channel', \
                 revive_operation_id = $2, revive_target = 'runner', \
                 revive_started_elapsed_ms = $3 \
                 WHERE session_id = $1 AND terminal_outcome IS NULL",
                &[&session_id, &operation_id, &elapsed],
            )?)?;
            Ok(disposition)
        })
    }

    /// Observes, cancels, or completes the active revive channel atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for authority/identifier/time/deadline rejection,
    /// corrupt state, or database failure.
    pub fn observe_cooperation_revive(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.observe_cooperation_revive_internal(
            session_id,
            operation_id,
            distance_squared,
            elapsed_ms,
            None,
        )
    }

    /// Observes a replay-mode revive and atomically appends `player_revived`
    /// only when the durable channel completes.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-replay operation, replay mismatch, domain
    /// rejection, corrupt persisted state, or database failure.
    pub fn observe_cooperation_revive_with_replay(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.observe_cooperation_revive_internal(
            session_id,
            operation_id,
            distance_squared,
            elapsed_ms,
            Some(true),
        )
    }

    fn observe_cooperation_revive_internal(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
        replay_expectation: Option<bool>,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.mutate_cooperation(
            session_id,
            elapsed_ms,
            replay_expectation,
            |transaction, stored, replay_mode| {
                let disposition = stored.state.observe_revive(
                    Role::Anchor,
                    Role::Runner,
                    distance_squared,
                    operation_id,
                    elapsed_ms,
                )?;
                match disposition {
                    MutationDisposition::Pending | MutationDisposition::Replayed => {}
                    MutationDisposition::Cancelled => {
                        require_one(transaction.execute(
                            "UPDATE cooperation_operations SET phase = 'runner_downed', \
                         revive_operation_id = NULL, revive_target = NULL, \
                         revive_started_elapsed_ms = NULL WHERE session_id = $1 \
                         AND terminal_outcome IS NULL",
                            &[&session_id],
                        )?)?;
                    }
                    MutationDisposition::Applied => {
                        let elapsed = elapsed_i64(elapsed_ms)?;
                        require_one(transaction.execute(
                            "UPDATE cooperation_operations SET phase = 'encounter_active', \
                         revived = TRUE, revive_completed_elapsed_ms = $2, revive_count = 1 \
                         WHERE session_id = $1 AND terminal_outcome IS NULL",
                            &[&session_id, &elapsed],
                        )?)?;
                        require_one(transaction.execute(
                            "UPDATE cooperation_operation_participants SET current_health = 50, \
                         life_state = 'active' WHERE session_id = $1 AND participant_index = 1",
                            &[&session_id],
                        )?)?;
                        if replay_mode {
                            persist_or_validate_revive_replay(
                                transaction,
                                stored,
                                operation_id,
                                elapsed_ms,
                                disposition,
                            )?;
                        }
                    }
                }
                if replay_mode && disposition == MutationDisposition::Replayed {
                    persist_or_validate_revive_replay(
                        transaction,
                        stored,
                        operation_id,
                        elapsed_ms,
                        disposition,
                    )?;
                }
                Ok(disposition)
            },
        )
    }

    /// Persists cooperation success and both equal participant rewards atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for deadline/lifecycle/retry conflict, partial prior
    /// grants, corrupt state, or any database failure.
    pub fn complete_cooperation_operation(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.complete_cooperation_operation_internal(session_id, elapsed_ms, false)
    }

    /// Persists cooperation success, generic rewards, and the canonical
    /// terminal cooperation replay as one transaction.
    ///
    /// # Errors
    ///
    /// Returns an error for replay mismatch, lifecycle/deadline rejection,
    /// partial prior rewards, corrupt persisted state, or database failure.
    pub fn complete_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.complete_cooperation_operation_internal(session_id, elapsed_ms, true)
    }

    fn complete_cooperation_operation_internal(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        append_replay: bool,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        validate_storage_id(session_id, "session")?;
        let mut transaction = self.client.transaction()?;
        lock_session(&mut transaction, session_id)?;
        let mut stored = require_operation(&mut transaction, session_id)?;
        validate_existing_cooperation_replay(
            &mut transaction,
            session_id,
            &stored,
            Some(append_replay),
        )?;
        if let Some(outcome) = stored.state.terminal() {
            validate_terminal_retry(&stored.parent, TerminalOutcome::Succeeded, elapsed_ms)?;
            if outcome != TerminalOutcome::Succeeded {
                return Err(invalid_state("cooperation terminal outcome conflicts"));
            }
            if append_replay {
                persist_or_validate_terminal_replay(
                    &mut transaction,
                    &stored,
                    TerminalOutcome::Succeeded,
                    None,
                    elapsed_ms,
                    MutationDisposition::Replayed,
                )?;
            }
            transaction.commit()?;
            return Ok(CooperationReceipt {
                disposition: MutationDisposition::Replayed,
                state: stored.state,
            });
        }
        let last_phase = stored.state.phase();
        stored.state.complete_warden(elapsed_ms)?;
        apply_success_rewards(
            &mut transaction,
            session_id,
            &stored.participants,
            append_replay,
        )?;
        let elapsed = elapsed_i64(elapsed_ms)?;
        require_one(transaction.execute(
            "UPDATE cooperation_operations SET warden_completed = TRUE, \
             warden_elapsed_ms = $2, terminal_outcome = 'succeeded', \
             terminal_elapsed_ms = $2, terminal_at = NOW() WHERE session_id = $1 \
             AND terminal_outcome IS NULL",
            &[&session_id, &elapsed],
        )?)?;
        if append_replay {
            persist_or_validate_terminal_replay(
                &mut transaction,
                &stored,
                TerminalOutcome::Succeeded,
                None,
                elapsed_ms,
                MutationDisposition::Applied,
            )?;
            let terminal = stored.state.terminal();
            if terminal != Some(TerminalOutcome::Succeeded) || last_phase != Phase::EncounterActive
            {
                return Err(invalid_state("cooperation success replay phase drifted"));
            }
            reconstruct_cooperation_stream(&mut transaction, session_id)?;
        }
        transaction.commit()?;
        Ok(CooperationReceipt {
            disposition: MutationDisposition::Applied,
            state: stored.state,
        })
    }

    /// Observes and persists the first applicable phase-specific/overall timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if no deadline has expired, on terminal conflict, corrupt
    /// state, or database failure.
    pub fn timeout_cooperation_operation(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.timeout_cooperation_operation_internal(session_id, elapsed_ms, false)
    }

    /// Persists the first applicable timeout and canonical failure replay.
    ///
    /// # Errors
    ///
    /// Returns an error if no deadline expired, replay mode/evidence conflicts,
    /// persisted state is corrupt, or a database write fails.
    pub fn timeout_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.timeout_cooperation_operation_internal(session_id, elapsed_ms, true)
    }

    fn timeout_cooperation_operation_internal(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        append_replay: bool,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.finish_cooperation_failure(
            session_id,
            elapsed_ms,
            None,
            append_replay,
            is_timeout_outcome,
            |state| match state.observe_deadlines(elapsed_ms) {
                Err(CooperationError::DeadlineExpired(_)) => Ok(()),
                Ok(_) => Err(invalid_state("cooperation deadline has not expired")),
                Err(error) => Err(error.into()),
            },
        )
    }

    /// Persists one post-revive participant defeat without any reward.
    ///
    /// # Errors
    ///
    /// Returns an error for deadline/phase/terminal conflict, corrupt state, or
    /// database failure.
    pub fn defeat_cooperation_participant(
        &mut self,
        session_id: &str,
        role: Role,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.defeat_cooperation_participant_internal(session_id, role, elapsed_ms, false)
    }

    /// Persists participant defeat and the canonical terminal replay atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for replay mismatch, domain rejection, corrupt state,
    /// or database failure.
    pub fn defeat_cooperation_participant_with_replay(
        &mut self,
        session_id: &str,
        role: Role,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.defeat_cooperation_participant_internal(session_id, role, elapsed_ms, true)
    }

    fn defeat_cooperation_participant_internal(
        &mut self,
        session_id: &str,
        role: Role,
        elapsed_ms: u64,
        append_replay: bool,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.finish_cooperation_failure(
            session_id,
            elapsed_ms,
            Some(role),
            append_replay,
            |outcome| outcome == TerminalOutcome::FailedParticipantDefeated,
            |state| {
                state.defeat_participant(role, elapsed_ms)?;
                Ok(())
            },
        )
    }

    /// Persists deterministic abandonment for either admitted role without reward.
    ///
    /// # Errors
    ///
    /// Returns an error for deadline/terminal conflict, corrupt state, or
    /// database failure.
    pub fn abandon_cooperation_operation(
        &mut self,
        session_id: &str,
        role: Role,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.abandon_cooperation_operation_internal(session_id, role, elapsed_ms, false)
    }

    /// Persists abandonment and its exact disconnecting-role replay atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for replay mismatch, deadline/terminal conflict,
    /// corrupt state, or database failure.
    pub fn abandon_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        role: Role,
        elapsed_ms: u64,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.abandon_cooperation_operation_internal(session_id, role, elapsed_ms, true)
    }

    fn abandon_cooperation_operation_internal(
        &mut self,
        session_id: &str,
        role: Role,
        elapsed_ms: u64,
        append_replay: bool,
    ) -> Result<CooperationReceipt, CooperationPersistenceError> {
        self.finish_cooperation_failure(
            session_id,
            elapsed_ms,
            Some(role),
            append_replay,
            |outcome| outcome == TerminalOutcome::AbandonedDisconnect,
            |state| {
                state.disconnect(role, elapsed_ms)?;
                Ok(())
            },
        )
    }

    fn mutate_cooperation<F>(
        &mut self,
        session_id: &str,
        observed_elapsed_ms: u64,
        replay_expectation: Option<bool>,
        mutation: F,
    ) -> Result<CooperationReceipt, CooperationPersistenceError>
    where
        F: FnOnce(
            &mut Transaction<'_>,
            &mut StoredOperation,
            bool,
        ) -> Result<MutationDisposition, CooperationPersistenceError>,
    {
        validate_storage_id(session_id, "session")?;
        let mut transaction = self.client.transaction()?;
        lock_session(&mut transaction, session_id)?;
        let mut stored = require_operation(&mut transaction, session_id)?;
        let append_replay = validate_existing_cooperation_replay(
            &mut transaction,
            session_id,
            &stored,
            replay_expectation,
        )?;
        let disposition = match mutation(&mut transaction, &mut stored, append_replay) {
            Ok(disposition) => disposition,
            Err(CooperationPersistenceError::Domain(CooperationError::DeadlineExpired(
                outcome,
            ))) => {
                persist_failure_terminal(
                    &mut transaction,
                    session_id,
                    outcome,
                    observed_elapsed_ms,
                )?;
                if append_replay {
                    persist_or_validate_terminal_replay(
                        &mut transaction,
                        &stored,
                        outcome,
                        None,
                        observed_elapsed_ms,
                        MutationDisposition::Applied,
                    )?;
                }
                MutationDisposition::Applied
            }
            Err(error) => return Err(error),
        };
        if append_replay {
            reconstruct_cooperation_stream(&mut transaction, session_id)?;
        }
        transaction.commit()?;
        Ok(CooperationReceipt {
            disposition,
            state: stored.state,
        })
    }

    fn finish_cooperation_failure<F, R>(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        subject_role: Option<Role>,
        append_replay: bool,
        retry_matches: R,
        mutation: F,
    ) -> Result<CooperationReceipt, CooperationPersistenceError>
    where
        F: FnOnce(&mut CooperationState) -> Result<(), CooperationPersistenceError>,
        R: FnOnce(TerminalOutcome) -> bool,
    {
        validate_storage_id(session_id, "session")?;
        let mut transaction = self.client.transaction()?;
        lock_session(&mut transaction, session_id)?;
        let mut stored = require_operation(&mut transaction, session_id)?;
        validate_existing_cooperation_replay(
            &mut transaction,
            session_id,
            &stored,
            Some(append_replay),
        )?;
        if let Some(outcome) = stored.state.terminal() {
            if !retry_matches(outcome) {
                return Err(invalid_state("cooperation terminal outcome conflicts"));
            }
            validate_terminal_retry(&stored.parent, outcome, elapsed_ms)?;
            if append_replay {
                persist_or_validate_terminal_replay(
                    &mut transaction,
                    &stored,
                    outcome,
                    subject_role,
                    elapsed_ms,
                    MutationDisposition::Replayed,
                )?;
            }
            transaction.commit()?;
            return Ok(CooperationReceipt {
                disposition: MutationDisposition::Replayed,
                state: stored.state,
            });
        }
        let last_phase = stored.state.phase();
        match mutation(&mut stored.state) {
            Ok(())
            | Err(CooperationPersistenceError::Domain(CooperationError::DeadlineExpired(_))) => {}
            Err(error) => return Err(error),
        }
        let outcome = stored
            .state
            .terminal()
            .ok_or_else(|| invalid_state("failure mutation did not produce a terminal"))?;
        if outcome == TerminalOutcome::Succeeded {
            return Err(invalid_state("failure mutation produced success"));
        }
        if outcome == TerminalOutcome::FailedParticipantDefeated {
            let role = subject_role
                .ok_or_else(|| invalid_state("defeat terminal has no participant role"))?;
            let index = i16::try_from(role.index())
                .map_err(|_| invalid_state("participant index exceeds SMALLINT"))?;
            require_one(transaction.execute(
                "UPDATE cooperation_operation_participants SET current_health = 0, \
                 life_state = 'defeated' WHERE session_id = $1 AND participant_index = $2",
                &[&session_id, &index],
            )?)?;
        }
        persist_failure_terminal(&mut transaction, session_id, outcome, elapsed_ms)?;
        if append_replay {
            persist_or_validate_terminal_replay(
                &mut transaction,
                &stored,
                outcome,
                subject_role,
                elapsed_ms,
                MutationDisposition::Applied,
            )?;
            if last_phase.is_terminal() {
                return Err(invalid_state(
                    "cooperation failure replay phase is terminal",
                ));
            }
            reconstruct_cooperation_stream(&mut transaction, session_id)?;
        }
        transaction.commit()?;
        Ok(CooperationReceipt {
            disposition: MutationDisposition::Applied,
            state: stored.state,
        })
    }
}

fn build_started_replay(
    client: &mut impl GenericClient,
    input: &CooperationOperationInput<'_>,
    participants: &[CooperationParticipant<'_>; PARTICIPANT_COUNT],
) -> Result<(CooperationStartedPayloadV1, String), CooperationPersistenceError> {
    let rows = client.query(
        "SELECT account_id, activity_id, actor_id, payload FROM replay_events \
         WHERE session_id = $1 AND event_type = 'module_state_snapshot' ORDER BY id",
        &[&input.session_id],
    )?;
    if rows.len() != PARTICIPANT_COUNT {
        return Err(invalid_state(
            "cooperation replay requires exactly two admission module snapshots",
        ));
    }
    let mut evidence = Vec::with_capacity(PARTICIPANT_COUNT);
    for (index, (row, participant)) in rows.iter().zip(participants).enumerate() {
        let actor_id = row
            .get::<_, Option<i64>>(2)
            .ok_or_else(|| invalid_state("cooperation snapshot has no actor"))?;
        let actor_id = u64::try_from(actor_id)
            .map_err(|_| invalid_state("cooperation snapshot actor is negative"))?;
        let payload_text = row.get::<_, String>(3);
        let Some(DecodedModuleReplayPayload::ModuleStateSnapshot(snapshot)) =
            decode_module_payload(ReplayEventKind::ModuleStateSnapshot, &payload_text)?
        else {
            return Err(invalid_state("cooperation snapshot decoded incorrectly"));
        };
        if row.get::<_, String>(0) != participant.account_id
            || row.get::<_, Option<String>>(1).as_deref() != Some(input.activity_id)
            || actor_id != participant.actor_id
            || snapshot.protocol_generation != ReplayProtocolGeneration::V2
            || snapshot.character_id != participant.character_id
            || snapshot.state.persisted_loadout != participant.modules
            || snapshot.state.applied_loadout != participant.modules
        {
            return Err(invalid_state(
                "cooperation admission disagrees with its M26 snapshot",
            ));
        }
        let role = Role::ALL[index];
        let participant_evidence = ReplayCooperationParticipantEvidence {
            participant_id: participant.account_id.to_owned(),
            character_id: participant.character_id.to_owned(),
            actor_id: participant.actor_id,
            role,
            weapon_item_id: participant.weapon_item_id.to_owned(),
            module_state: snapshot.state,
            admitted_health: participant.current_health,
        };
        if replay_profile(&participant_evidence)? != participant.profile {
            return Err(invalid_state(
                "cooperation profile disagrees with immutable replay evidence",
            ));
        }
        evidence.push(participant_evidence);
    }
    let payload = CooperationStartedPayloadV1 {
        schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
        activity_id: input.activity_id.to_owned(),
        catalog_revision: COOPERATION_CATALOG_REVISION.to_owned(),
        start_operation_id: input.operation_id.to_owned(),
        participants: evidence,
        anchor_target: ReplayCooperationTargetEvidence {
            target: Target::RelayAnchor,
            position: [3, 0, 3],
        },
        runner_target: ReplayCooperationTargetEvidence {
            target: Target::RelayConsole,
            position: [4, 0, 3],
        },
        timing: ReplayCooperationTimingEvidence {
            operation_duration_ms: OPERATION_DURATION_MS,
            ping_ttl_ms: PING_TTL_MS,
            revive_window_ms: REVIVE_WINDOW_MS,
            revive_channel_ms: REVIVE_CHANNEL_MS,
            revive_health: REVIVE_HEALTH,
            maximum_distance_squared: MAX_REVIVE_DISTANCE_SQUARED,
        },
        reward: ReplayCooperationRewardEvidence {
            item_id: RELAY_CORE_FRAGMENT.to_owned(),
            item_quantity: SUCCESS_REWARD.fragments,
            experience: SUCCESS_REWARD.experience,
        },
    };
    let encoded = encode_cooperation_started(&payload)?;
    Ok((payload, encoded))
}

fn replay_profile(
    participant: &ReplayCooperationParticipantEvidence,
) -> Result<CombatProfile, CooperationPersistenceError> {
    let mut matching = participant
        .module_state
        .weapons
        .iter()
        .filter(|weapon| weapon.item_id == participant.weapon_item_id);
    let weapon = matching
        .next()
        .ok_or_else(|| invalid_state("cooperation replay weapon profile is missing"))?;
    if matching.next().is_some() {
        return Err(invalid_state(
            "cooperation replay weapon profile is duplicated",
        ));
    }
    Ok(CombatProfile {
        damage: weapon.effective.damage,
        range: weapon.effective.range,
        cooldown_ms: weapon.effective.cooldown_ms,
        max_health: weapon.effective.max_health,
    })
}

fn validate_started_replay_matches_storage(
    reconstructed: &ReconstructedSession,
    stored: &StoredOperation,
) -> Result<(), CooperationPersistenceError> {
    let started = reconstructed
        .cooperation
        .started
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation replay start is missing"))?;
    if started.activity_id != stored.parent.activity_id
        || started.start_operation_id != stored.parent.start_operation_id
        || started.catalog_revision != stored.parent.catalog_revision
        || started.participants.len() != PARTICIPANT_COUNT
    {
        return Err(invalid_state(
            "cooperation replay start disagrees with durable parent",
        ));
    }
    for (evidence, participant) in started.participants.iter().zip(&stored.participants) {
        if evidence.participant_id != participant.account_id
            || evidence.character_id != participant.character_id
            || i64::try_from(evidence.actor_id).ok() != Some(participant.actor_id)
            || role_text(evidence.role) != participant.role
            || evidence.weapon_item_id != participant.weapon_item_id
            || evidence.module_state.persisted_loadout != participant.module_loadout
            || evidence.module_state.applied_loadout != participant.module_loadout
            || i32::try_from(evidence.admitted_health).ok() != Some(participant.admitted_health)
            || replay_profile(evidence)?
                != (CombatProfile {
                    damage: u32::try_from(participant.damage)
                        .map_err(|_| invalid_state("stored replay damage is negative"))?,
                    range: participant.range,
                    cooldown_ms: stored_elapsed(participant.cooldown_ms, "replay cooldown")?,
                    max_health: stored_health(participant.max_health, "replay maximum health")?,
                })
        {
            return Err(invalid_state(
                "cooperation replay participant disagrees with durable admission",
            ));
        }
    }
    Ok(())
}

fn validate_new_cooperation_replay_mode(
    client: &mut impl GenericClient,
    session_id: &str,
) -> Result<(), CooperationPersistenceError> {
    let (cooperation_count, route_count) = replay_family_counts(client, session_id)?;
    if cooperation_count != 0 || route_count != 0 {
        return Err(invalid_state(
            "cooperation start conflicts with existing route/cooperation replay",
        ));
    }
    Ok(())
}

fn validate_cooperation_replay_mode(
    client: &mut impl GenericClient,
    session_id: &str,
    replay_expected: bool,
) -> Result<bool, CooperationPersistenceError> {
    let (cooperation_count, route_count) = replay_family_counts(client, session_id)?;
    let start_count = client
        .query_one(
            "SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
             AND event_type = 'cooperation_started'",
            &[&session_id],
        )?
        .get::<_, i64>(0);
    if route_count != 0
        || (replay_expected && (start_count != 1 || cooperation_count < 1))
        || (!replay_expected && cooperation_count != 0)
    {
        return Err(invalid_state(
            "cooperation replay mode is missing, partial, mixed, or conflicting",
        ));
    }
    Ok(replay_expected)
}

fn validate_existing_cooperation_replay(
    client: &mut impl GenericClient,
    session_id: &str,
    stored: &StoredOperation,
    expectation: Option<bool>,
) -> Result<bool, CooperationPersistenceError> {
    let (cooperation_count, route_count) = replay_family_counts(client, session_id)?;
    let start_count = client
        .query_one(
            "SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
             AND event_type = 'cooperation_started'",
            &[&session_id],
        )?
        .get::<_, i64>(0);
    let replay_mode = match (start_count, cooperation_count) {
        (0, 0) => false,
        (1, count) if count >= 1 => true,
        _ => {
            return Err(invalid_state(
                "cooperation replay start/cardinality is partial or duplicated",
            ))
        }
    };
    if route_count != 0 || expectation.is_some_and(|expected| expected != replay_mode) {
        return Err(invalid_state(
            "cooperation replay mode conflicts with the requested mutation",
        ));
    }
    if replay_mode {
        let reconstructed = reconstruct_cooperation_stream(client, session_id)?;
        validate_reconstructed_cooperation_matches_storage(&reconstructed, stored)?;
    }
    Ok(replay_mode)
}

fn replay_family_counts(
    client: &mut impl GenericClient,
    session_id: &str,
) -> Result<(i64, i64), CooperationPersistenceError> {
    let row = client.query_one(
        "SELECT COUNT(*) FILTER (WHERE event_type IN (\
             'cooperation_started', 'cooperation_pinged', 'player_downed', \
             'player_revived', 'cooperation_succeeded', 'cooperation_failed')), \
                COUNT(*) FILTER (WHERE event_type IN (\
             'route_selected', 'route_operation_succeeded', 'route_operation_failed')) \
         FROM replay_events WHERE session_id = $1",
        &[&session_id],
    )?;
    Ok((row.get(0), row.get(1)))
}

fn reconstruct_cooperation_stream(
    client: &mut impl GenericClient,
    session_id: &str,
) -> Result<ReconstructedSession, CooperationPersistenceError> {
    reconstruct_cooperation_stream_internal(client, session_id, false)
}

fn reconstruct_cooperation_prefix(
    client: &mut impl GenericClient,
    session_id: &str,
) -> Result<ReconstructedSession, CooperationPersistenceError> {
    reconstruct_cooperation_stream_internal(client, session_id, true)
}

fn reconstruct_cooperation_stream_internal(
    client: &mut impl GenericClient,
    session_id: &str,
    omit_terminal_generics: bool,
) -> Result<ReconstructedSession, CooperationPersistenceError> {
    let rows = client.query(
        "SELECT id, event_type, occurred_at::TEXT, session_id, account_id, \
                activity_id, actor_id, payload FROM replay_events \
         WHERE session_id = $1 ORDER BY id",
        &[&session_id],
    )?;
    let cooperation_started_id = rows
        .iter()
        .find(|row| row.get::<_, String>(1) == "cooperation_started")
        .map(|row| row.get::<_, i64>(0));
    let events = rows
        .into_iter()
        .filter(|row| {
            if !omit_terminal_generics {
                return true;
            }
            let kind = row.get::<_, String>(1);
            let id = row.get::<_, i64>(0);
            !cooperation_started_id.is_some_and(|start_id| {
                id > start_id
                    && matches!(
                        kind.as_str(),
                        "activity_completed" | "loot_granted" | "progression_granted"
                    )
            })
        })
        .map(|row| {
            let event_type = row.get::<_, String>(1);
            let kind = event_type
                .parse::<ReplayEventKind>()
                .map_err(|_| invalid_state("cooperation stream has an unknown replay kind"))?;
            let actor_id = row
                .get::<_, Option<i64>>(6)
                .map(u64::try_from)
                .transpose()
                .map_err(|_| invalid_state("cooperation replay actor is negative"))?;
            Ok(ReplayEvent {
                id: row.get(0),
                kind,
                timestamp: row.get(2),
                session_id: row.get(3),
                account_id: row.get(4),
                activity_id: row.get(5),
                actor_id,
                payload: row.get(7),
            })
        })
        .collect::<Result<Vec<_>, CooperationPersistenceError>>()?;
    reconstruct(&events).map_err(Into::into)
}

fn validate_reconstructed_cooperation_matches_storage(
    reconstructed: &ReconstructedSession,
    stored: &StoredOperation,
) -> Result<(), CooperationPersistenceError> {
    validate_started_replay_matches_storage(reconstructed, stored)?;
    let started = reconstructed
        .cooperation
        .started
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation replay start is missing"))?;
    let expected_ping = stored
        .parent
        .pinged
        .then(|| ping_payload_from_storage(stored, started))
        .transpose()?;
    if reconstructed.cooperation.pinged != expected_ping {
        return Err(invalid_state(
            "cooperation ping replay disagrees with durable state",
        ));
    }
    let expected_down = stored
        .parent
        .runner_arrived
        .then(|| down_payload_from_storage(stored, started))
        .transpose()?;
    if reconstructed.cooperation.downed != expected_down {
        return Err(invalid_state(
            "cooperation downing replay disagrees with durable state",
        ));
    }
    let expected_revive = stored
        .parent
        .revived
        .then(|| revive_payload_from_storage(stored, started))
        .transpose()?;
    if reconstructed.cooperation.revived != expected_revive {
        return Err(invalid_state(
            "cooperation revive replay disagrees with durable state",
        ));
    }
    match stored.state.terminal() {
        None if reconstructed.cooperation.terminal.is_none() => Ok(()),
        Some(outcome) => {
            let subject = match outcome {
                TerminalOutcome::FailedParticipantDefeated => stored
                    .state
                    .participants()
                    .iter()
                    .find(|participant| participant.life == LifeState::Defeated)
                    .map(|participant| participant.role),
                TerminalOutcome::AbandonedDisconnect => reconstructed
                    .cooperation
                    .terminal
                    .as_ref()
                    .and_then(|terminal| terminal.subject_role),
                _ => None,
            };
            let expected = terminal_payload_from_storage(
                stored,
                started,
                outcome,
                subject,
                stored_elapsed(
                    stored
                        .parent
                        .terminal_elapsed_ms
                        .ok_or_else(|| invalid_state("durable terminal elapsed is missing"))?,
                    "terminal elapsed",
                )?,
            )?;
            if reconstructed.cooperation.terminal.as_ref() != Some(&expected) {
                return Err(invalid_state(
                    "cooperation terminal replay disagrees with durable state",
                ));
            }
            Ok(())
        }
        None => Err(invalid_state(
            "active cooperation has unexpected terminal replay",
        )),
    }
}

fn persist_or_validate_ping_replay(
    client: &mut impl GenericClient,
    stored: &StoredOperation,
    operation_id: &str,
    elapsed_ms: u64,
    disposition: MutationDisposition,
) -> Result<(), CooperationPersistenceError> {
    let reconstructed = reconstruct_cooperation_stream(client, stored_session(stored))?;
    let started = reconstructed
        .cooperation
        .started
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation start replay is missing"))?;
    let payload = if disposition == MutationDisposition::Replayed {
        ping_payload_from_storage(stored, started)?
    } else {
        let anchor_elapsed_ms = stored_elapsed(
            stored
                .parent
                .anchor_elapsed_ms
                .ok_or_else(|| invalid_state("ping replay has no anchor arrival"))?,
            "anchor elapsed",
        )?;
        CooperationPingedPayloadV1 {
            schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
            catalog_revision: stored.parent.catalog_revision.clone(),
            start_operation_id: stored.parent.start_operation_id.clone(),
            anchor_elapsed_ms,
            ping_operation_id: operation_id.to_owned(),
            source_participant_id: stored.participants[0].account_id.clone(),
            source_actor_id: stored_actor(&stored.participants[0])?,
            target: Target::RelayConsole,
            ping_elapsed_ms: elapsed_ms,
            expires_elapsed_ms: elapsed_ms
                .checked_add(PING_TTL_MS)
                .ok_or_else(|| invalid_state("ping replay expiry overflowed"))?,
        }
    };
    let encoded = encode_cooperation_pinged(&payload, started)?;
    persist_or_validate_cooperation_event(
        client,
        stored,
        ReplayEventKind::CooperationPinged,
        Role::Anchor,
        &encoded,
        disposition,
    )
}

fn ping_payload_from_storage(
    stored: &StoredOperation,
    started: &CooperationStartedPayloadV1,
) -> Result<CooperationPingedPayloadV1, CooperationPersistenceError> {
    let elapsed_ms = stored_elapsed(
        stored
            .parent
            .ping_elapsed_ms
            .ok_or_else(|| invalid_state("durable ping elapsed is missing"))?,
        "ping elapsed",
    )?;
    let payload = CooperationPingedPayloadV1 {
        schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
        catalog_revision: stored.parent.catalog_revision.clone(),
        start_operation_id: stored.parent.start_operation_id.clone(),
        anchor_elapsed_ms: stored_elapsed(
            stored
                .parent
                .anchor_elapsed_ms
                .ok_or_else(|| invalid_state("durable anchor elapsed is missing"))?,
            "anchor elapsed",
        )?,
        ping_operation_id: stored
            .parent
            .ping_operation_id
            .clone()
            .ok_or_else(|| invalid_state("durable ping operation is missing"))?,
        source_participant_id: stored.participants[0].account_id.clone(),
        source_actor_id: stored_actor(&stored.participants[0])?,
        target: Target::RelayConsole,
        ping_elapsed_ms: elapsed_ms,
        expires_elapsed_ms: elapsed_ms
            .checked_add(PING_TTL_MS)
            .ok_or_else(|| invalid_state("durable ping expiry overflowed"))?,
    };
    encode_cooperation_pinged(&payload, started)?;
    Ok(payload)
}

fn persist_or_validate_down_replay(
    client: &mut impl GenericClient,
    stored: &StoredOperation,
    elapsed_ms: u64,
    disposition: MutationDisposition,
) -> Result<(), CooperationPersistenceError> {
    let reconstructed = reconstruct_cooperation_stream(client, stored_session(stored))?;
    let started = reconstructed
        .cooperation
        .started
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation start replay is missing"))?;
    let pinged = reconstructed
        .cooperation
        .pinged
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation ping replay is missing"))?;
    let payload = down_payload(stored, elapsed_ms)?;
    let encoded = encode_player_downed(&payload, started, pinged)?;
    persist_or_validate_cooperation_event(
        client,
        stored,
        ReplayEventKind::PlayerDowned,
        Role::Runner,
        &encoded,
        disposition,
    )
}

fn down_payload(
    stored: &StoredOperation,
    elapsed_ms: u64,
) -> Result<PlayerDownedPayloadV1, CooperationPersistenceError> {
    let admitted = stored_health(
        stored.participants[Role::Runner.index()].admitted_health,
        "runner admitted health",
    )?;
    Ok(PlayerDownedPayloadV1 {
        schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
        catalog_revision: stored.parent.catalog_revision.clone(),
        start_operation_id: stored.parent.start_operation_id.clone(),
        participant_id: stored.participants[1].account_id.clone(),
        actor_id: stored_actor(&stored.participants[1])?,
        role: Role::Runner,
        target: Target::RelayConsole,
        runner_elapsed_ms: elapsed_ms,
        hazard: ReplayCooperationHazard::RelayFeedback,
        health_before: admitted,
        damage: admitted,
        health_after: 0,
        life_before: LifeState::Active,
        life_after: LifeState::Downed,
    })
}

fn down_payload_from_storage(
    stored: &StoredOperation,
    started: &CooperationStartedPayloadV1,
) -> Result<PlayerDownedPayloadV1, CooperationPersistenceError> {
    let payload = down_payload(
        stored,
        stored_elapsed(
            stored
                .parent
                .runner_elapsed_ms
                .ok_or_else(|| invalid_state("durable runner elapsed is missing"))?,
            "runner elapsed",
        )?,
    )?;
    let ping = ping_payload_from_storage(stored, started)?;
    encode_player_downed(&payload, started, &ping)?;
    Ok(payload)
}

fn persist_or_validate_revive_replay(
    client: &mut impl GenericClient,
    stored: &StoredOperation,
    operation_id: &str,
    elapsed_ms: u64,
    disposition: MutationDisposition,
) -> Result<(), CooperationPersistenceError> {
    let reconstructed = reconstruct_cooperation_stream(client, stored_session(stored))?;
    let started = reconstructed
        .cooperation
        .started
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation start replay is missing"))?;
    let downed = reconstructed
        .cooperation
        .downed
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation downing replay is missing"))?;
    let payload = revive_payload(stored, operation_id, elapsed_ms)?;
    let encoded = encode_player_revived(&payload, started, downed)?;
    persist_or_validate_cooperation_event(
        client,
        stored,
        ReplayEventKind::PlayerRevived,
        Role::Runner,
        &encoded,
        disposition,
    )
}

fn revive_payload(
    stored: &StoredOperation,
    operation_id: &str,
    elapsed_ms: u64,
) -> Result<PlayerRevivedPayloadV1, CooperationPersistenceError> {
    Ok(PlayerRevivedPayloadV1 {
        schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
        catalog_revision: stored.parent.catalog_revision.clone(),
        start_operation_id: stored.parent.start_operation_id.clone(),
        revive_operation_id: operation_id.to_owned(),
        source_participant_id: stored.participants[0].account_id.clone(),
        source_actor_id: stored_actor(&stored.participants[0])?,
        target_participant_id: stored.participants[1].account_id.clone(),
        target_actor_id: stored_actor(&stored.participants[1])?,
        target_role: Role::Runner,
        revive_started_elapsed_ms: stored_elapsed(
            stored
                .parent
                .revive_started_elapsed_ms
                .ok_or_else(|| invalid_state("durable revive start is missing"))?,
            "revive start",
        )?,
        revive_completed_elapsed_ms: elapsed_ms,
        channel_duration_ms: REVIVE_CHANNEL_MS,
        maximum_distance_squared: MAX_REVIVE_DISTANCE_SQUARED,
        health_before: 0,
        health_after: REVIVE_HEALTH,
        life_before: LifeState::Downed,
        life_after: LifeState::Active,
        revive_count: 1,
    })
}

fn revive_payload_from_storage(
    stored: &StoredOperation,
    started: &CooperationStartedPayloadV1,
) -> Result<PlayerRevivedPayloadV1, CooperationPersistenceError> {
    let operation_id = stored
        .parent
        .revive_operation_id
        .as_deref()
        .ok_or_else(|| invalid_state("durable revive operation is missing"))?;
    let payload = revive_payload(
        stored,
        operation_id,
        stored_elapsed(
            stored
                .parent
                .revive_completed_elapsed_ms
                .ok_or_else(|| invalid_state("durable revive completion is missing"))?,
            "revive completion",
        )?,
    )?;
    let downed = down_payload_from_storage(stored, started)?;
    encode_player_revived(&payload, started, &downed)?;
    Ok(payload)
}

fn persist_or_validate_terminal_replay(
    client: &mut impl GenericClient,
    stored: &StoredOperation,
    outcome: TerminalOutcome,
    subject_role: Option<Role>,
    elapsed_ms: u64,
    disposition: MutationDisposition,
) -> Result<(), CooperationPersistenceError> {
    let session_id = stored_session(stored);
    let reconstructed = if disposition == MutationDisposition::Applied {
        reconstruct_cooperation_prefix(client, session_id)?
    } else {
        reconstruct_cooperation_stream(client, session_id)?
    };
    let started = reconstructed
        .cooperation
        .started
        .as_ref()
        .ok_or_else(|| invalid_state("cooperation start replay is missing"))?;
    let payload =
        terminal_payload_from_storage(stored, started, outcome, subject_role, elapsed_ms)?;
    let prefix = CooperationReplayPrefix {
        pinged: reconstructed.cooperation.pinged.as_ref(),
        downed: reconstructed.cooperation.downed.as_ref(),
        revived: reconstructed.cooperation.revived.as_ref(),
    };
    let (kind, encoded) = if outcome == TerminalOutcome::Succeeded {
        (
            ReplayEventKind::CooperationSucceeded,
            encode_cooperation_succeeded(&payload, started, prefix)?,
        )
    } else {
        (
            ReplayEventKind::CooperationFailed,
            encode_cooperation_failed(&payload, started, prefix)?,
        )
    };
    persist_or_validate_cooperation_event(client, stored, kind, Role::Anchor, &encoded, disposition)
}

fn terminal_payload_from_storage(
    stored: &StoredOperation,
    started: &CooperationStartedPayloadV1,
    outcome: TerminalOutcome,
    subject_role: Option<Role>,
    elapsed_ms: u64,
) -> Result<CooperationTerminalPayloadV1, CooperationPersistenceError> {
    let contribution = stored.state.contributions();
    let parent = &stored.parent;
    let participants = stored
        .participants
        .iter()
        .zip(stored.state.participants())
        .map(|(identity, state)| {
            Ok(ReplayCooperationTerminalParticipantEvidence {
                participant_id: identity.account_id.clone(),
                character_id: identity.character_id.clone(),
                actor_id: stored_actor(identity)?,
                role: state.role,
                current_health: state.current_health,
                life: state.life,
            })
        })
        .collect::<Result<Vec<_>, CooperationPersistenceError>>()?;
    let grants = (outcome == TerminalOutcome::Succeeded)
        .then(|| {
            stored
                .participants
                .iter()
                .map(|participant| {
                    Ok(ReplayCooperationGrantEvidence {
                        participant_id: participant.account_id.clone(),
                        character_id: participant.character_id.clone(),
                        actor_id: stored_actor(participant)?,
                        item_id: RELAY_CORE_FRAGMENT.to_owned(),
                        item_quantity: SUCCESS_REWARD.fragments,
                        experience: SUCCESS_REWARD.experience,
                    })
                })
                .collect::<Result<Vec<_>, CooperationPersistenceError>>()
        })
        .transpose()?
        .unwrap_or_default();
    let payload = CooperationTerminalPayloadV1 {
        schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
        catalog_revision: parent.catalog_revision.clone(),
        start_operation_id: parent.start_operation_id.clone(),
        last_nonterminal_phase: parse_phase(&parent.phase)?,
        contributions: ReplayCooperationContributionEvidence {
            anchor_arrived: contribution.anchor_arrived,
            anchor_elapsed_ms: optional_elapsed(parent.anchor_elapsed_ms, "anchor elapsed")?,
            pinged: contribution.pinged,
            ping_operation_id: parent.ping_operation_id.clone(),
            ping_elapsed_ms: optional_elapsed(parent.ping_elapsed_ms, "ping elapsed")?,
            runner_arrived: contribution.runner_arrived,
            runner_elapsed_ms: optional_elapsed(parent.runner_elapsed_ms, "runner elapsed")?,
            downed_elapsed_ms: optional_elapsed(parent.downed_elapsed_ms, "downed elapsed")?,
            revive_operation_id: parent.revive_operation_id.clone(),
            revive_started_elapsed_ms: optional_elapsed(
                parent.revive_started_elapsed_ms,
                "revive start",
            )?,
            revived: contribution.revived,
            revive_completed_elapsed_ms: optional_elapsed(
                parent.revive_completed_elapsed_ms,
                "revive completion",
            )?,
            revive_count: stored.state.revive_count(),
            warden_completed: contribution.warden_completed,
            warden_elapsed_ms: (outcome == TerminalOutcome::Succeeded).then_some(elapsed_ms),
        },
        participants,
        outcome,
        subject_role,
        terminal_elapsed_ms: elapsed_ms,
        grants,
    };
    if started.start_operation_id != payload.start_operation_id {
        return Err(invalid_state(
            "terminal replay start identity disagrees with admission",
        ));
    }
    Ok(payload)
}

fn persist_or_validate_cooperation_event(
    client: &mut impl GenericClient,
    stored: &StoredOperation,
    kind: ReplayEventKind,
    role: Role,
    payload: &str,
    disposition: MutationDisposition,
) -> Result<(), CooperationPersistenceError> {
    match disposition {
        MutationDisposition::Applied => insert_cooperation_event(
            client,
            stored_session(stored),
            &stored.parent.activity_id,
            &stored.participants[role.index()].account_id,
            stored_actor(&stored.participants[role.index()])?,
            kind,
            payload,
        ),
        MutationDisposition::Replayed => {
            validate_exact_cooperation_event(client, stored, kind, role, payload)
        }
        MutationDisposition::Pending | MutationDisposition::Cancelled => Err(invalid_state(
            "pending/cancelled cooperation cannot persist replay evidence",
        )),
    }
}

fn insert_cooperation_event(
    client: &mut impl GenericClient,
    session_id: &str,
    activity_id: &str,
    account_id: &str,
    actor_id: u64,
    kind: ReplayEventKind,
    payload: &str,
) -> Result<(), CooperationPersistenceError> {
    let actor_id = i64::try_from(actor_id)
        .map_err(|_| invalid_state("cooperation replay actor exceeds BIGINT"))?;
    let event_type = kind.to_string();
    insert_replay_event(
        client,
        &NewReplayEvent {
            event_type: &event_type,
            session_id,
            account_id,
            activity_id: Some(activity_id),
            actor_id: Some(actor_id),
            payload,
        },
    )?;
    Ok(())
}

fn validate_exact_cooperation_event(
    client: &mut impl GenericClient,
    stored: &StoredOperation,
    kind: ReplayEventKind,
    role: Role,
    payload: &str,
) -> Result<(), CooperationPersistenceError> {
    let event_type = kind.to_string();
    let rows = client.query(
        "SELECT account_id, activity_id, actor_id, payload FROM replay_events \
         WHERE session_id = $1 AND event_type = $2 ORDER BY id",
        &[&stored_session(stored), &event_type],
    )?;
    if rows.len() != 1
        || rows[0].get::<_, String>(0) != stored.participants[role.index()].account_id
        || rows[0].get::<_, Option<String>>(1).as_deref()
            != Some(stored.parent.activity_id.as_str())
        || rows[0].get::<_, Option<i64>>(2) != Some(stored.participants[role.index()].actor_id)
        || rows[0].get::<_, String>(3) != payload
    {
        return Err(invalid_state(
            "cooperation replay retry disagrees with its exact durable event",
        ));
    }
    Ok(())
}

fn stored_session(stored: &StoredOperation) -> &str {
    &stored.parent.session_id
}

fn stored_actor(participant: &StoredParticipant) -> Result<u64, CooperationPersistenceError> {
    u64::try_from(participant.actor_id)
        .map_err(|_| invalid_state("stored cooperation actor is negative"))
}

fn optional_elapsed(
    value: Option<i64>,
    field: &str,
) -> Result<Option<u64>, CooperationPersistenceError> {
    value.map(|value| stored_elapsed(value, field)).transpose()
}

fn validate_start_input<'a>(
    input: &CooperationOperationInput<'a>,
) -> Result<[CooperationParticipant<'a>; PARTICIPANT_COUNT], CooperationPersistenceError> {
    if input.activity_id != ACTIVITY_ID
        || input.participants.len() != PARTICIPANT_COUNT
        || input.requester_account_id != input.participants[0].account_id
    {
        return Err(invalid_state(
            "cooperation start requires exact activity, leader, and two participants",
        ));
    }
    let supplied: [CooperationParticipant<'_>; PARTICIPANT_COUNT] =
        input
            .participants
            .try_into()
            .map_err(|_| invalid_state("cooperation requires exactly two participants"))?;
    if supplied[0].account_id == supplied[1].account_id
        || supplied[0].character_id == supplied[1].character_id
        || supplied[0].actor_id == supplied[1].actor_id
    {
        return Err(invalid_state("cooperation participant identities repeat"));
    }
    for participant in supplied {
        validate_storage_id(participant.account_id, "account")?;
        validate_storage_id(participant.character_id, "character")?;
        validate_profile(participant)?;
    }
    Ok(supplied)
}

fn validate_profile(
    participant: CooperationParticipant<'_>,
) -> Result<(), CooperationPersistenceError> {
    let weapon = weapon_profile(participant.weapon_item_id)
        .ok_or_else(|| invalid_state("cooperation weapon is unknown"))?;
    let resolved = resolve_build(
        MODULE_CATALOG_REVISION,
        BaseCombatProfile {
            damage: weapon.damage,
            range: weapon.range,
            cooldown_ms: weapon.cooldown_ms,
            max_health: 100,
        },
        participant.modules,
    )
    .map_err(|error| invalid_state(error.to_string()))?;
    let expected = CombatProfile {
        damage: resolved.profile.damage,
        range: resolved.profile.range,
        cooldown_ms: resolved.profile.cooldown_ms,
        max_health: resolved.profile.max_health,
    };
    if participant.profile != expected {
        return Err(invalid_state(
            "cooperation profile does not match weapon/module admission",
        ));
    }
    CooperationState::start_with_health(
        "profile-check",
        [expected, expected],
        [participant.current_health, participant.current_health],
    )?;
    Ok(())
}

fn validate_participant_identity(
    transaction: &mut Transaction<'_>,
    participants: &[CooperationParticipant<'_>; PARTICIPANT_COUNT],
) -> Result<(), CooperationPersistenceError> {
    for participant in participants {
        let owner = transaction
            .query_opt(
                "SELECT account_id FROM characters WHERE id = $1 FOR UPDATE",
                &[&participant.character_id],
            )?
            .ok_or_else(|| invalid_state("cooperation character was not found"))?
            .get::<_, String>(0);
        if owner != participant.account_id {
            return Err(invalid_state(
                "cooperation character does not belong to account",
            ));
        }
    }
    Ok(())
}

fn insert_start(
    transaction: &mut Transaction<'_>,
    input: &CooperationOperationInput<'_>,
    participants: &[CooperationParticipant<'_>; PARTICIPANT_COUNT],
) -> Result<(), CooperationPersistenceError> {
    transaction.execute(
        "INSERT INTO cooperation_operations (session_id, activity_id, \
         start_operation_id, catalog_revision, anchor_account_id, participant_count, \
         phase, operation_duration_ms, ping_ttl_ms, revive_window_ms, \
         revive_channel_ms, revive_health, max_revive_distance_squared, \
         reward_item_id, reward_quantity, reward_experience) VALUES (\
         $1, $2, $3, $4, $5, 2, 'awaiting_anchor', $6, $7, $8, $9, $10, $11, \
         $12, $13, $14)",
        &[
            &input.session_id,
            &input.activity_id,
            &input.operation_id,
            &COOPERATION_CATALOG_REVISION,
            &input.requester_account_id,
            &elapsed_i64(OPERATION_DURATION_MS)?,
            &elapsed_i64(PING_TTL_MS)?,
            &elapsed_i64(REVIVE_WINDOW_MS)?,
            &elapsed_i64(REVIVE_CHANNEL_MS)?,
            &i32::try_from(REVIVE_HEALTH)
                .map_err(|_| invalid_state("revive health exceeds INTEGER"))?,
            &MAX_REVIVE_DISTANCE_SQUARED,
            &RELAY_CORE_FRAGMENT,
            &i32::try_from(SUCCESS_REWARD.fragments)
                .map_err(|_| invalid_state("reward quantity exceeds INTEGER"))?,
            &i64::try_from(SUCCESS_REWARD.experience)
                .map_err(|_| invalid_state("reward experience exceeds BIGINT"))?,
        ],
    )?;
    for (index, participant) in participants.iter().enumerate() {
        let role = Role::ALL[index];
        let participant_index = i16::try_from(index)
            .map_err(|_| invalid_state("participant index exceeds SMALLINT"))?;
        let actor = i64::try_from(participant.actor_id)
            .map_err(|_| invalid_state("participant actor exceeds BIGINT"))?;
        let modules = serde_json::to_string(participant.modules)?;
        let damage = i32::try_from(participant.profile.damage)
            .map_err(|_| invalid_state("profile damage exceeds INTEGER"))?;
        let cooldown = elapsed_i64(participant.profile.cooldown_ms)?;
        let max_health = i32::try_from(participant.profile.max_health)
            .map_err(|_| invalid_state("maximum health exceeds INTEGER"))?;
        let health = i32::try_from(participant.current_health)
            .map_err(|_| invalid_state("current health exceeds INTEGER"))?;
        transaction.execute(
            "INSERT INTO cooperation_operation_participants (session_id, \
             participant_index, role, account_id, character_id, actor_id, \
             weapon_item_id, module_loadout, damage, range, cooldown_ms, max_health, \
             admitted_health, current_health, life_state) VALUES (\
             $1, $2, $3, $4, $5, $6, $7, $8::TEXT::JSONB, $9, $10, $11, $12, \
             $13, $13, 'active')",
            &[
                &input.session_id,
                &participant_index,
                &role_text(role),
                &participant.account_id,
                &participant.character_id,
                &actor,
                &participant.weapon_item_id,
                &modules,
                &damage,
                &participant.profile.range,
                &cooldown,
                &max_health,
                &health,
            ],
        )?;
    }
    Ok(())
}

fn load_operation(
    client: &mut impl GenericClient,
    session_id: &str,
    for_update: bool,
) -> Result<Option<StoredOperation>, CooperationPersistenceError> {
    let query = if for_update {
        "SELECT row_to_json(c)::TEXT FROM cooperation_operations c \
         WHERE session_id = $1 FOR UPDATE"
    } else {
        "SELECT row_to_json(c)::TEXT FROM cooperation_operations c WHERE session_id = $1"
    };
    let Some(row) = client.query_opt(query, &[&session_id])? else {
        return Ok(None);
    };
    let parent = serde_json::from_str::<StoredParent>(&row.get::<_, String>(0))?;
    let participant_query = if for_update {
        "SELECT row_to_json(p)::TEXT FROM cooperation_operation_participants p \
         WHERE session_id = $1 ORDER BY participant_index FOR UPDATE"
    } else {
        "SELECT row_to_json(p)::TEXT FROM cooperation_operation_participants p \
         WHERE session_id = $1 ORDER BY participant_index"
    };
    let participants = client
        .query(participant_query, &[&session_id])?
        .into_iter()
        .map(|row| serde_json::from_str::<StoredParticipant>(&row.get::<_, String>(0)))
        .collect::<Result<Vec<_>, _>>()?;
    let participants: [StoredParticipant; PARTICIPANT_COUNT] = participants
        .try_into()
        .map_err(|_| invalid_state("stored cooperation requires exactly two participants"))?;
    let state = reconstruct_state(client, &parent, &participants)?;
    Ok(Some(StoredOperation {
        parent,
        participants,
        state,
    }))
}

fn require_operation(
    client: &mut impl GenericClient,
    session_id: &str,
) -> Result<StoredOperation, CooperationPersistenceError> {
    load_operation(client, session_id, true)?
        .ok_or_else(|| invalid_state("cooperation operation was not found"))
}

#[allow(clippy::too_many_lines)]
fn reconstruct_state(
    client: &mut impl GenericClient,
    parent: &StoredParent,
    participants: &[StoredParticipant; PARTICIPANT_COUNT],
) -> Result<CooperationState, CooperationPersistenceError> {
    validate_parent_constants(parent)?;
    let profiles = [
        validate_stored_participant(client, parent, &participants[0], Role::Anchor)?,
        validate_stored_participant(client, parent, &participants[1], Role::Runner)?,
    ];
    let admitted_health = [
        stored_health(participants[0].admitted_health, "anchor admitted health")?,
        stored_health(participants[1].admitted_health, "runner admitted health")?,
    ];
    let mut state =
        CooperationState::start_with_health(&parent.start_operation_id, profiles, admitted_health)?;
    if let Some(elapsed) = parent.anchor_elapsed_ms {
        state.arrive_anchor(
            Role::Anchor,
            Target::RelayAnchor,
            stored_elapsed(elapsed, "anchor elapsed")?,
        )?;
    }
    if let (Some(operation_id), Some(elapsed)) = (&parent.ping_operation_id, parent.ping_elapsed_ms)
    {
        state.ping(
            Role::Anchor,
            Target::RelayConsole,
            operation_id,
            stored_elapsed(elapsed, "ping elapsed")?,
        )?;
    }
    if let Some(elapsed) = parent.runner_elapsed_ms {
        state.arrive_runner(
            Role::Runner,
            Target::RelayConsole,
            stored_elapsed(elapsed, "runner elapsed")?,
        )?;
    }
    if let (Some(operation_id), Some(started)) = (
        &parent.revive_operation_id,
        parent.revive_started_elapsed_ms,
    ) {
        state.start_revive(
            Role::Anchor,
            Role::Runner,
            MAX_REVIVE_DISTANCE_SQUARED,
            operation_id,
            stored_elapsed(started, "revive start")?,
        )?;
        if let Some(completed) = parent.revive_completed_elapsed_ms {
            state.observe_revive(
                Role::Anchor,
                Role::Runner,
                MAX_REVIVE_DISTANCE_SQUARED,
                operation_id,
                stored_elapsed(completed, "revive completion")?,
            )?;
        }
    }
    let expected_phase = parse_phase(&parent.phase)?;
    if state.phase() != expected_phase {
        return Err(invalid_state(
            "stored phase differs from reconstructed phase",
        ));
    }
    if let (Some(outcome), Some(elapsed)) = (&parent.terminal_outcome, parent.terminal_elapsed_ms) {
        let outcome = parse_terminal(outcome)?;
        let elapsed = stored_elapsed(elapsed, "terminal elapsed")?;
        match outcome {
            TerminalOutcome::Succeeded => {
                state.complete_warden(elapsed)?;
            }
            TerminalOutcome::FailedPingTimeout
            | TerminalOutcome::FailedReviveTimeout
            | TerminalOutcome::FailedOperationTimeout => {
                if !matches!(
                    state.observe_deadlines(elapsed),
                    Err(CooperationError::DeadlineExpired(candidate)) if candidate == outcome
                ) {
                    return Err(invalid_state("stored timeout does not match pure deadline"));
                }
            }
            TerminalOutcome::FailedParticipantDefeated => {
                let defeated = participants
                    .iter()
                    .filter(|participant| participant.life_state == "defeated")
                    .collect::<Vec<_>>();
                if defeated.len() != 1 {
                    return Err(invalid_state("participant defeat cardinality is invalid"));
                }
                state.defeat_participant(parse_role(&defeated[0].role)?, elapsed)?;
            }
            TerminalOutcome::AbandonedDisconnect => {
                state.disconnect(Role::Anchor, elapsed)?;
            }
        }
    } else if parent.terminal_outcome.is_some() || parent.terminal_elapsed_ms.is_some() {
        return Err(invalid_state("stored terminal evidence is partial"));
    }
    validate_state_parity(parent, participants, &state)?;
    Ok(state)
}

fn validate_parent_constants(parent: &StoredParent) -> Result<(), CooperationPersistenceError> {
    if parent.activity_id != ACTIVITY_ID
        || parent.catalog_revision != COOPERATION_CATALOG_REVISION
        || parent.participant_count != 2
        || parent.operation_duration_ms != elapsed_i64(OPERATION_DURATION_MS)?
        || parent.ping_ttl_ms != elapsed_i64(PING_TTL_MS)?
        || parent.revive_window_ms != elapsed_i64(REVIVE_WINDOW_MS)?
        || parent.revive_channel_ms != elapsed_i64(REVIVE_CHANNEL_MS)?
        || parent.revive_health != i32::try_from(REVIVE_HEALTH).unwrap_or(i32::MAX)
        || parent.max_revive_distance_squared != MAX_REVIVE_DISTANCE_SQUARED
        || parent.reward_item_id != RELAY_CORE_FRAGMENT
        || parent.reward_quantity != i32::try_from(SUCCESS_REWARD.fragments).unwrap_or(i32::MAX)
        || parent.reward_experience != i64::try_from(SUCCESS_REWARD.experience).unwrap_or(i64::MAX)
        || parent.anchor_account_id.is_empty()
    {
        return Err(invalid_state("stored cooperation constants are invalid"));
    }
    Ok(())
}

fn validate_stored_participant(
    client: &mut impl GenericClient,
    parent: &StoredParent,
    participant: &StoredParticipant,
    expected_role: Role,
) -> Result<CombatProfile, CooperationPersistenceError> {
    if participant.participant_index
        != i16::try_from(expected_role.index())
            .map_err(|_| invalid_state("role index exceeds SMALLINT"))?
        || parse_role(&participant.role)? != expected_role
        || (expected_role == Role::Anchor && participant.account_id != parent.anchor_account_id)
        || participant.actor_id < 0
    {
        return Err(invalid_state(
            "stored participant identity/order is invalid",
        ));
    }
    let owner = client
        .query_opt(
            "SELECT account_id FROM characters WHERE id = $1",
            &[&participant.character_id],
        )?
        .ok_or_else(|| invalid_state("stored cooperation character was not found"))?
        .get::<_, String>(0);
    if owner != participant.account_id {
        return Err(invalid_state(
            "stored cooperation character ownership is invalid",
        ));
    }
    let weapon = weapon_profile(&participant.weapon_item_id)
        .ok_or_else(|| invalid_state("stored cooperation weapon is invalid"))?;
    let resolved = resolve_build(
        MODULE_CATALOG_REVISION,
        BaseCombatProfile {
            damage: weapon.damage,
            range: weapon.range,
            cooldown_ms: weapon.cooldown_ms,
            max_health: 100,
        },
        &participant.module_loadout,
    )
    .map_err(|error| invalid_state(error.to_string()))?;
    let profile = CombatProfile {
        damage: u32::try_from(participant.damage)
            .map_err(|_| invalid_state("stored profile damage is negative"))?,
        range: participant.range,
        cooldown_ms: stored_elapsed(participant.cooldown_ms, "profile cooldown")?,
        max_health: stored_health(participant.max_health, "maximum health")?,
    };
    if profile.damage != resolved.profile.damage
        || profile.range != resolved.profile.range
        || profile.cooldown_ms != resolved.profile.cooldown_ms
        || profile.max_health != resolved.profile.max_health
    {
        return Err(invalid_state(
            "stored profile differs from canonical weapon/module resolution",
        ));
    }
    Ok(profile)
}

fn validate_state_parity(
    parent: &StoredParent,
    participants: &[StoredParticipant; PARTICIPANT_COUNT],
    state: &CooperationState,
) -> Result<(), CooperationPersistenceError> {
    let contribution = state.contributions();
    if contribution.anchor_arrived != parent.anchor_arrived
        || contribution.pinged != parent.pinged
        || contribution.runner_arrived != parent.runner_arrived
        || contribution.revived != parent.revived
        || contribution.warden_completed != parent.warden_completed
        || i16::from(state.revive_count()) != parent.revive_count
    {
        return Err(invalid_state(
            "stored contribution evidence differs from domain",
        ));
    }
    for (stored, actual) in participants.iter().zip(state.participants()) {
        if stored_health(stored.current_health, "current health")? != actual.current_health
            || parse_life(&stored.life_state)? != actual.life
        {
            return Err(invalid_state("stored participant life differs from domain"));
        }
    }
    if parent.ping_target.as_deref() != parent.pinged.then_some("relay_console")
        || parent.revive_target.as_deref() != parent.revive_operation_id.as_ref().map(|_| "runner")
        || parent.runner_elapsed_ms != parent.downed_elapsed_ms
        || parent.warden_elapsed_ms.is_some() != parent.warden_completed
    {
        return Err(invalid_state("stored target/downing evidence is invalid"));
    }
    Ok(())
}

fn validate_start_retry(
    stored: &StoredOperation,
    input: &CooperationOperationInput<'_>,
    supplied: &[CooperationParticipant<'_>; PARTICIPANT_COUNT],
) -> Result<(), CooperationPersistenceError> {
    let profiles = supplied.map(|participant| participant.profile);
    let health = supplied.map(|participant| participant.current_health);
    stored
        .state
        .retry_start(input.operation_id, profiles, health)?;
    if stored.parent.activity_id != input.activity_id
        || stored.parent.anchor_account_id != input.requester_account_id
        || stored
            .participants
            .iter()
            .zip(supplied)
            .any(|(left, right)| {
                left.account_id != right.account_id
                    || left.character_id != right.character_id
                    || u64::try_from(left.actor_id).ok() != Some(right.actor_id)
                    || left.weapon_item_id != right.weapon_item_id
                    || left.module_loadout != right.modules
            })
    {
        return Err(invalid_state("cooperation start retry conflicts"));
    }
    Ok(())
}

fn apply_success_rewards(
    transaction: &mut Transaction<'_>,
    session_id: &str,
    participants: &[StoredParticipant; PARTICIPANT_COUNT],
    append_replay: bool,
) -> Result<(), CooperationPersistenceError> {
    let experience = ExperienceReward::validated(SUCCESS_REWARD.experience)
        .map_err(|error| invalid_state(error.to_string()))?;
    if append_replay {
        let anchor = &participants[Role::Anchor.index()];
        insert_cooperation_event(
            transaction,
            session_id,
            ACTIVITY_ID,
            &anchor.account_id,
            stored_actor(anchor)?,
            ReplayEventKind::ActivityCompleted,
            "activity completed",
        )?;
    }
    for participant in participants {
        let applied = apply_completion_reward(
            transaction,
            &ActivityCompletion {
                session_id,
                account_id: &participant.account_id,
                character_id: &participant.character_id,
                activity_id: ACTIVITY_ID,
                item_id: RELAY_CORE_FRAGMENT,
                item_quantity: i32::try_from(SUCCESS_REWARD.fragments)
                    .map_err(|_| invalid_state("reward quantity exceeds INTEGER"))?,
                experience_reward: experience,
            },
        )?;
        if applied.is_none() {
            return Err(invalid_state(
                "cooperation reward exists without terminal evidence",
            ));
        }
        if append_replay {
            let applied = applied.expect("validated applied reward");
            let loot_payload = format!(
                "loot granted: {RELAY_CORE_FRAGMENT} x{} (total {})",
                SUCCESS_REWARD.fragments, applied.item_quantity
            );
            let progression_payload = format!(
                "progression granted: +{} XP (total {}, level {} -> {})",
                applied.experience_granted,
                applied.experience,
                applied.previous_level,
                applied.level
            );
            let actor_id = stored_actor(participant)?;
            insert_cooperation_event(
                transaction,
                session_id,
                ACTIVITY_ID,
                &participant.account_id,
                actor_id,
                ReplayEventKind::LootGranted,
                &loot_payload,
            )?;
            insert_cooperation_event(
                transaction,
                session_id,
                ACTIVITY_ID,
                &participant.account_id,
                actor_id,
                ReplayEventKind::ProgressionGranted,
                &progression_payload,
            )?;
        }
    }
    Ok(())
}

fn persist_failure_terminal(
    transaction: &mut Transaction<'_>,
    session_id: &str,
    outcome: TerminalOutcome,
    elapsed_ms: u64,
) -> Result<(), CooperationPersistenceError> {
    if outcome == TerminalOutcome::Succeeded {
        return Err(invalid_state("failure persistence received success"));
    }
    let elapsed = elapsed_i64(elapsed_ms)?;
    require_one(transaction.execute(
        "UPDATE cooperation_operations SET terminal_outcome = $2, \
         terminal_elapsed_ms = $3, terminal_at = NOW() WHERE session_id = $1 \
         AND terminal_outcome IS NULL",
        &[&session_id, &terminal_text(outcome), &elapsed],
    )?)
}

fn lock_session(
    transaction: &mut Transaction<'_>,
    session_id: &str,
) -> Result<(), CooperationPersistenceError> {
    transaction.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 28))",
        &[&session_id],
    )?;
    Ok(())
}

fn validate_terminal_retry(
    parent: &StoredParent,
    expected: TerminalOutcome,
    elapsed_ms: u64,
) -> Result<(), CooperationPersistenceError> {
    if parent.terminal_outcome.as_deref() != Some(terminal_text(expected))
        || parent.terminal_elapsed_ms != Some(elapsed_i64(elapsed_ms)?)
    {
        return Err(invalid_state("cooperation terminal retry conflicts"));
    }
    Ok(())
}

fn require_same_elapsed(stored: i64, supplied: u64) -> Result<(), CooperationPersistenceError> {
    if stored == elapsed_i64(supplied)? {
        Ok(())
    } else {
        Err(invalid_state(
            "cooperation transition elapsed value conflicts",
        ))
    }
}

fn require_one(rows: u64) -> Result<(), CooperationPersistenceError> {
    if rows == 1 {
        Ok(())
    } else {
        Err(invalid_state(
            "cooperation mutation affected an unexpected row count",
        ))
    }
}

fn validate_storage_id(value: &str, context: &str) -> Result<(), CooperationPersistenceError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
    {
        return Err(invalid_state(format!(
            "invalid cooperation {context} identifier"
        )));
    }
    Ok(())
}

fn elapsed_i64(value: u64) -> Result<i64, CooperationPersistenceError> {
    i64::try_from(value).map_err(|_| invalid_state("elapsed value exceeds BIGINT"))
}

fn stored_elapsed(value: i64, field: &str) -> Result<u64, CooperationPersistenceError> {
    u64::try_from(value).map_err(|_| invalid_state(format!("stored {field} is negative")))
}

fn stored_health(value: i32, field: &str) -> Result<u32, CooperationPersistenceError> {
    u32::try_from(value).map_err(|_| invalid_state(format!("stored {field} is negative")))
}

fn parse_phase(value: &str) -> Result<Phase, CooperationPersistenceError> {
    match value {
        "awaiting_anchor" => Ok(Phase::AwaitingAnchor),
        "awaiting_ping" => Ok(Phase::AwaitingPing),
        "awaiting_runner" => Ok(Phase::AwaitingRunner),
        "runner_downed" => Ok(Phase::RunnerDowned),
        "revive_channel" => Ok(Phase::ReviveChannel),
        "encounter_active" => Ok(Phase::EncounterActive),
        _ => Err(invalid_state("stored cooperation phase is unknown")),
    }
}

fn parse_role(value: &str) -> Result<Role, CooperationPersistenceError> {
    match value {
        "anchor" => Ok(Role::Anchor),
        "runner" => Ok(Role::Runner),
        _ => Err(invalid_state("stored cooperation role is unknown")),
    }
}

fn parse_life(value: &str) -> Result<LifeState, CooperationPersistenceError> {
    match value {
        "active" => Ok(LifeState::Active),
        "downed" => Ok(LifeState::Downed),
        "defeated" => Ok(LifeState::Defeated),
        _ => Err(invalid_state("stored cooperation life state is unknown")),
    }
}

fn parse_terminal(value: &str) -> Result<TerminalOutcome, CooperationPersistenceError> {
    match value {
        "succeeded" => Ok(TerminalOutcome::Succeeded),
        "failed_ping_timeout" => Ok(TerminalOutcome::FailedPingTimeout),
        "failed_revive_timeout" => Ok(TerminalOutcome::FailedReviveTimeout),
        "failed_operation_timeout" => Ok(TerminalOutcome::FailedOperationTimeout),
        "failed_participant_defeated" => Ok(TerminalOutcome::FailedParticipantDefeated),
        "abandoned_disconnect" => Ok(TerminalOutcome::AbandonedDisconnect),
        _ => Err(invalid_state("stored cooperation terminal is unknown")),
    }
}

const fn role_text(role: Role) -> &'static str {
    match role {
        Role::Anchor => "anchor",
        Role::Runner => "runner",
    }
}

const fn terminal_text(outcome: TerminalOutcome) -> &'static str {
    match outcome {
        TerminalOutcome::Succeeded => "succeeded",
        TerminalOutcome::FailedPingTimeout => "failed_ping_timeout",
        TerminalOutcome::FailedReviveTimeout => "failed_revive_timeout",
        TerminalOutcome::FailedOperationTimeout => "failed_operation_timeout",
        TerminalOutcome::FailedParticipantDefeated => "failed_participant_defeated",
        TerminalOutcome::AbandonedDisconnect => "abandoned_disconnect",
    }
}

const fn is_timeout_outcome(outcome: TerminalOutcome) -> bool {
    matches!(
        outcome,
        TerminalOutcome::FailedPingTimeout
            | TerminalOutcome::FailedReviveTimeout
            | TerminalOutcome::FailedOperationTimeout
    )
}

fn invalid_state(message: impl Into<String>) -> CooperationPersistenceError {
    CooperationPersistenceError::InvalidPersistedState(message.into())
}
