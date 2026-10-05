mod acquisition;
mod campaign;
pub use campaign::{CampaignPersistenceResult, CampaignReceipt, CampaignRequest};
mod challenges;
pub use challenges::{ChallengePersistenceResult, ChallengeReceipt, ChallengeRequest};
mod database_config;

pub use acquisition::{AcquisitionPersistenceError, PersistedAcquisitionState};

pub use database_config::{database_url_from_environment, DatabaseConfigurationError};

use postgres::{Client, GenericClient, NoTls, Row, Transaction};
use revenant_inventory::RELAY_CORE_FRAGMENT;
use revenant_modules::{
    canonicalize_loadout, module_definition, validate_operation_id, ActivityPhase,
    CombinationOutcome, CombinationRequest, LoadoutOutcome, LoadoutRequest, ModuleId,
    ModuleMutationError, ModuleState, OperationDisposition, OperationReceipt,
    MAX_OPERATION_RECORDS_PER_KIND,
};
use revenant_operations::{
    resolve_event, route_definition, OperationDisposition as RouteOperationDisposition,
    OperationReceipt as RouteOperationReceipt, RouteEffect, RouteError, RouteEventId, RouteId,
    RouteObjectiveId, RouteOperationState, RouteOperationSummary, RouteReward, RouteSeed,
    RouteSelectionOutcome, RouteSelectionRequest, TerminalOutcome, CATALOG_REVISION,
    RESOLVER_REVISION,
};
use revenant_progression::{ExperienceReward, Progression as DomainProgression};
use revenant_replay::{
    decode_module_payload, decode_route_payload, encode_module_combined,
    encode_module_loadout_changed, encode_module_state_snapshot, encode_route_operation_failed,
    encode_route_operation_succeeded, encode_route_selected, module_state_evidence, reconstruct,
    DecodedModuleReplayPayload, DecodedRouteReplayPayload, ModuleCombinedPayloadV1,
    ModuleLoadoutChangedPayloadV1, ModuleStateSnapshotPayloadV1, ReconstructedCooperationState,
    ReconstructedSession, ReplayError, ReplayEvent as DomainReplayEvent, ReplayEventKind,
    ReplayProtocolGeneration, RouteSelectedPayloadV1, RouteTerminalPayloadV1,
    MODULE_REPLAY_SCHEMA_VERSION,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

mod cooperation;
pub use cooperation::{
    CooperationOperationInput, CooperationParticipant, CooperationPersistenceError,
    CooperationReceipt,
};

const SCHEMA: &str = concat!(
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_replay_events.sql"),
    include_str!("../migrations/0003_inventory_rewards.sql"),
    include_str!("../migrations/0004_progression_rewards.sql"),
    include_str!("../migrations/0005_equipment_loadouts.sql"),
    include_str!("../migrations/0006_module_progression.sql"),
    include_str!("../migrations/0007_route_operations.sql"),
    include_str!("../migrations/0008_cooperation_operations.sql"),
    include_str!("../migrations/0009_acquisition_commissions.sql")
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCharacter {
    pub id: String,
    pub display_name: String,
    pub class_name: String,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEntry {
    pub item_id: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progression {
    pub level: i32,
    pub experience: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedModuleState {
    pub fragments: u32,
    pub owned_modules: Vec<ModuleId>,
    pub loadout: Vec<ModuleId>,
    pub revision: u64,
    pub combination_operations: usize,
    pub loadout_operations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteParticipant<'a> {
    pub account_id: &'a str,
    pub character_id: &'a str,
    pub actor_id: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct RouteOperationSelection<'a> {
    pub session_id: &'a str,
    pub activity_id: &'a str,
    pub requester_account_id: &'a str,
    pub operation_id: &'a str,
    pub route_id: RouteId,
    pub candidate_seed: RouteSeed,
    pub participants: &'a [RouteParticipant<'a>],
}

#[derive(Debug)]
pub enum RoutePersistenceError {
    Database(postgres::Error),
    Domain(RouteError),
    Serialization(serde_json::Error),
    Replay(ReplayError),
    InvalidPersistedState(String),
}

impl fmt::Display for RoutePersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => error.fmt(formatter),
            Self::Domain(error) => error.fmt(formatter),
            Self::Serialization(error) => error.fmt(formatter),
            Self::Replay(error) => error.fmt(formatter),
            Self::InvalidPersistedState(message) => {
                write!(formatter, "invalid persisted route operation: {message}")
            }
        }
    }
}

impl std::error::Error for RoutePersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Domain(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::Replay(error) => Some(error),
            Self::InvalidPersistedState(_) => None,
        }
    }
}

impl From<postgres::Error> for RoutePersistenceError {
    fn from(error: postgres::Error) -> Self {
        Self::Database(error)
    }
}

impl From<RouteError> for RoutePersistenceError {
    fn from(error: RouteError) -> Self {
        Self::Domain(error)
    }
}

impl From<serde_json::Error> for RoutePersistenceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

impl From<ReplayError> for RoutePersistenceError {
    fn from(error: ReplayError) -> Self {
        Self::Replay(error)
    }
}

#[derive(Debug)]
pub enum ModulePersistenceError {
    Database(postgres::Error),
    Domain(ModuleMutationError),
    Serialization(serde_json::Error),
    Replay(ReplayError),
    CharacterNotFound,
    InvalidPersistedState(String),
}

impl fmt::Display for ModulePersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => error.fmt(formatter),
            Self::Domain(error) => error.fmt(formatter),
            Self::Serialization(error) => error.fmt(formatter),
            Self::Replay(error) => error.fmt(formatter),
            Self::CharacterNotFound => formatter.write_str("module character was not found"),
            Self::InvalidPersistedState(message) => {
                write!(formatter, "invalid persisted module state: {message}")
            }
        }
    }
}

impl std::error::Error for ModulePersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Domain(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::Replay(error) => Some(error),
            Self::CharacterNotFound | Self::InvalidPersistedState(_) => None,
        }
    }
}

impl From<postgres::Error> for ModulePersistenceError {
    fn from(error: postgres::Error) -> Self {
        Self::Database(error)
    }
}

impl From<ModuleMutationError> for ModulePersistenceError {
    fn from(error: ModuleMutationError) -> Self {
        Self::Domain(error)
    }
}

impl From<serde_json::Error> for ModulePersistenceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

impl From<ReplayError> for ModulePersistenceError {
    fn from(error: ReplayError) -> Self {
        Self::Replay(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredCombinationRequest {
    module_id: ModuleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredLoadoutRequest {
    expected_revision: u64,
    modules: Vec<ModuleId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredRouteParticipant {
    account: String,
    character: String,
    actor: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredRouteOperation {
    activity_id: String,
    selection: RouteSelectionOutcome,
    participants: Vec<StoredRouteParticipant>,
    terminal: Option<RouteOperationSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionRewards {
    pub item_quantity: i32,
    pub experience_granted: i64,
    pub previous_level: i32,
    pub level: i32,
    pub experience: i64,
}

/// Inputs required to grant one character's idempotent activity reward.
pub struct ActivityCompletion<'a> {
    /// Session that owns the completion and reward grant.
    pub session_id: &'a str,
    /// Account that owns the rewarded character.
    pub account_id: &'a str,
    /// Character receiving inventory, progression, and history updates.
    pub character_id: &'a str,
    /// Completed activity identifier.
    pub activity_id: &'a str,
    /// Inventory item granted by the activity.
    pub item_id: &'a str,
    /// Positive quantity granted by the activity definition.
    pub item_quantity: i32,
    /// Validated experience granted by the activity definition.
    pub experience_reward: ExperienceReward,
}

/// One participant included in an atomic multiplayer completion.
pub struct ActivityParticipantCompletion<'a> {
    /// Participant-specific reward and history inputs.
    pub completion: ActivityCompletion<'a>,
    /// Authoritative actor recorded on participant reward replay events.
    pub actor_id: i64,
}

/// Replay identity attached to one atomic player-join module snapshot.
#[derive(Debug, Clone, Copy)]
pub struct ModuleJoinReplayContext<'a> {
    pub catalog_revision: Option<&'a str>,
    pub session_id: &'a str,
    pub account_id: &'a str,
    pub activity_id: &'a str,
    pub actor_id: i64,
    pub player_joined_payload: &'a str,
    pub protocol_generation: ReplayProtocolGeneration,
}

/// Replay identity attached to one accepted module mutation.
#[derive(Debug, Clone, Copy)]
pub struct ModuleMutationReplayContext<'a> {
    pub catalog_revision: Option<&'a str>,
    pub session_id: &'a str,
    pub account_id: &'a str,
    pub activity_id: &'a str,
    pub actor_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReplayEvent<'a> {
    pub event_type: &'a str,
    pub session_id: &'a str,
    pub account_id: &'a str,
    pub activity_id: Option<&'a str>,
    pub actor_id: Option<i64>,
    pub payload: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedReplayEvent {
    pub id: i64,
    pub event_type: String,
    pub timestamp: String,
    pub session_id: String,
    pub account_id: String,
    pub activity_id: Option<String>,
    pub actor_id: Option<i64>,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedReplaySession {
    pub session_id: String,
    pub activity_id: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub event_count: i64,
    pub participant_count: i64,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AuthoritativeSessionSummary {
    pub session_id: String,
    pub activity_id: Option<String>,
    pub first_joined_at: String,
    pub activity_started_at: Option<String>,
    pub activity_ended_at: String,
    pub join_to_start_ms: Option<i64>,
    pub activity_duration_ms: Option<i64>,
    pub participant_count: i64,
    pub completed: bool,
    pub enemy_spawn_count: i64,
    pub enemy_defeat_count: i64,
    pub boss_spawned: bool,
    pub equipment_change_count: i64,
    pub loot_grant_count: i64,
    pub progression_grant_count: i64,
    pub module_replay_legacy: bool,
    pub module_participant_count: i64,
    pub module_snapshot_count: i64,
    pub module_combination_count: i64,
    pub module_loadout_change_count: i64,
    pub acquisition_claim_count: i64,
    pub route_replay_legacy: bool,
    pub route_id: Option<RouteId>,
    pub route_event_id: Option<RouteEventId>,
    pub route_terminal_outcome: Option<TerminalOutcome>,
    pub route_elapsed_ms: Option<u64>,
    pub route_transition_count: i64,
    pub route_reward_participant_count: i64,
    pub cooperation_replay_legacy: bool,
    pub cooperation_replay_state: ReconstructedCooperationState,
    pub cooperation_last_phase: Option<revenant_cooperation::Phase>,
    pub cooperation_terminal_outcome: Option<revenant_cooperation::TerminalOutcome>,
    pub cooperation_terminal_elapsed_ms: Option<u64>,
    pub cooperation_participant_count: i64,
    pub cooperation_contribution_count: i64,
    pub cooperation_revive_count: i64,
    pub cooperation_reward_participant_count: i64,
    pub event_count: i64,
}

#[derive(Debug)]
pub enum SessionSummaryError {
    Database(postgres::Error),
    InvalidEvidence(&'static str),
}

impl fmt::Display for SessionSummaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => error.fmt(formatter),
            Self::InvalidEvidence(message) => {
                write!(formatter, "invalid replay evidence: {message}")
            }
        }
    }
}

impl std::error::Error for SessionSummaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::InvalidEvidence(_) => None,
        }
    }
}

impl From<postgres::Error> for SessionSummaryError {
    fn from(error: postgres::Error) -> Self {
        Self::Database(error)
    }
}

pub struct Persistence {
    client: Client,
}

impl Persistence {
    /// Connects to `PostgreSQL` and applies the idempotent Revenant schema.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the connection or migration fails.
    pub fn connect(database_url: &str) -> Result<Self, postgres::Error> {
        let mut client = Client::connect(database_url, NoTls)?;
        let mut transaction = client.transaction()?;
        transaction.query_one("SELECT pg_advisory_xact_lock(824_180_018)", &[])?;
        transaction.batch_execute(SCHEMA)?;
        transaction.commit()?;
        Ok(Self { client })
    }

    /// Connects to a schema already initialized by the process startup gate.
    ///
    /// Runtime connection replacement uses this path so recovery never reapplies
    /// DDL or competes for migration locks with active gameplay transactions.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the connection fails.
    pub fn connect_existing(database_url: &str) -> Result<Self, postgres::Error> {
        Ok(Self {
            client: Client::connect(database_url, NoTls)?,
        })
    }

    /// Creates the local account and its initial operator when they do not exist.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the transaction fails.
    pub fn ensure_local_account(
        &mut self,
        account_id: &str,
        username: &str,
    ) -> Result<(), postgres::Error> {
        let character_id = format!("{account_id}:operator");
        let mut transaction = self.client.transaction()?;
        transaction.execute(
            "INSERT INTO accounts (id, username) VALUES ($1, $2) \
             ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username",
            &[&account_id, &username],
        )?;
        transaction.execute(
            "INSERT INTO characters (id, account_id, display_name, class_name, level) \
             VALUES ($1, $2, $3, 'Operator', 1) ON CONFLICT (id) DO NOTHING",
            &[&character_id, &account_id, &username],
        )?;
        transaction.execute(
            "INSERT INTO progression (character_id, level, experience) VALUES ($1, 1, 0) \
             ON CONFLICT (character_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, 'pulse_rifle', 1) \
             ON CONFLICT (character_id, item_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, 'arc_sidearm', 1) \
             ON CONFLICT (character_id, item_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.execute(
            "INSERT INTO equipment_loadouts (character_id, weapon_item_id) \
             VALUES ($1, 'pulse_rifle') ON CONFLICT (character_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.commit()
    }

    /// Provisions the negotiated content weapon once without replacing saved equipment.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn ensure_content_equipment(&mut self, character_id: &str) -> Result<(), postgres::Error> {
        self.client.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, 'coil_lance', 1) \
             ON CONFLICT (character_id, item_id) DO NOTHING",
            &[&character_id],
        )?;
        Ok(())
    }

    /// Provisions the two fixed M35 weapons once, preserving saved equipment.
    ///
    /// # Errors
    /// Returns the database error without granting a partial arsenal.
    pub fn ensure_arsenal_equipment(&mut self, character_id: &str) -> Result<(), postgres::Error> {
        self.client.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) \
             SELECT $1, item_id, 1 FROM unnest(ARRAY['scatter_caster', 'rail_driver']) AS item_id \
             ON CONFLICT (character_id, item_id) DO NOTHING",
            &[&character_id],
        )?;
        Ok(())
    }

    /// Loads characters owned by an account in stable identifier order.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn characters_for(
        &mut self,
        account_id: &str,
    ) -> Result<Vec<PersistedCharacter>, postgres::Error> {
        self.client
            .query(
                "SELECT id, display_name, class_name, level FROM characters \
                 WHERE account_id = $1 ORDER BY id",
                &[&account_id],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| PersistedCharacter {
                        id: row.get(0),
                        display_name: row.get(1),
                        class_name: row.get(2),
                        level: row.get(3),
                    })
                    .collect()
            })
    }

    /// Loads the persisted inventory for a character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn inventory_for(
        &mut self,
        character_id: &str,
    ) -> Result<Vec<InventoryEntry>, postgres::Error> {
        self.client
            .query(
                "SELECT item_id, quantity FROM inventory WHERE character_id = $1 ORDER BY item_id",
                &[&character_id],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| InventoryEntry {
                        item_id: row.get(0),
                        quantity: row.get(1),
                    })
                    .collect()
            })
    }

    /// Loads progression for a character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn progression_for(
        &mut self,
        character_id: &str,
    ) -> Result<Option<Progression>, postgres::Error> {
        self.client
            .query_opt(
                "SELECT level, experience FROM progression WHERE character_id = $1",
                &[&character_id],
            )
            .map(|row| {
                row.map(|row| Progression {
                    level: row.get(0),
                    experience: row.get(1),
                })
            })
    }

    /// Loads the selected weapon for a character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn equipped_weapon_for(
        &mut self,
        character_id: &str,
    ) -> Result<Option<String>, postgres::Error> {
        self.client
            .query_opt(
                "SELECT weapon_item_id FROM equipment_loadouts WHERE character_id = $1",
                &[&character_id],
            )
            .map(|row| row.map(|row| row.get(0)))
    }

    /// Persists the authoritative weapon selection.
    ///
    /// The caller validates ownership and equipability before crossing this boundary.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the update fails.
    pub fn equip_weapon(
        &mut self,
        character_id: &str,
        item_id: &str,
    ) -> Result<bool, postgres::Error> {
        self.client
            .execute(
                "UPDATE equipment_loadouts SET weapon_item_id = $2 WHERE character_id = $1",
                &[&character_id, &item_id],
            )
            .map(|updated| updated == 1)
    }

    /// Persists a weapon selection and its replay evidence atomically.
    ///
    /// The replay event is inserted only when the loadout row exists and is
    /// updated. A database error rolls both writes back.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when either write or the commit fails.
    pub fn equip_weapon_with_replay(
        &mut self,
        character_id: &str,
        item_id: &str,
        replay_event: &NewReplayEvent<'_>,
    ) -> Result<bool, postgres::Error> {
        let mut transaction = self.client.transaction()?;
        let updated = transaction.execute(
            "UPDATE equipment_loadouts SET weapon_item_id = $2 WHERE character_id = $1",
            &[&character_id, &item_id],
        )? == 1;
        if updated {
            insert_replay_event(&mut transaction, replay_event)?;
        }
        transaction.commit()?;
        Ok(updated)
    }

    /// Loads legacy-compatible character module state without creating rows.
    ///
    /// A character with no M26 rows resolves to zero fragments/modules/loadout
    /// revision only for module state; its existing generic fragment inventory
    /// quantity is still returned exactly.
    ///
    /// # Errors
    ///
    /// Returns an error when the character is missing, a query fails, or the
    /// persisted rows violate catalog, ownership, slot, quantity, or revision rules.
    pub fn module_state_for(
        &mut self,
        character_id: &str,
    ) -> Result<PersistedModuleState, ModulePersistenceError> {
        load_persisted_module_state(&mut self.client, character_id)
    }

    /// Atomically appends a player join and its complete module-state snapshot.
    ///
    /// The snapshot loads legacy-compatible state without materializing module
    /// rows. Frozen V1 records persisted state but applies no module effects.
    ///
    /// # Errors
    ///
    /// Returns an error when account/character identity disagrees, state or
    /// arithmetic is invalid, replay serialization fails, or either insert fails.
    pub fn append_player_joined_with_module_snapshot(
        &mut self,
        character_id: &str,
        context: ModuleJoinReplayContext<'_>,
    ) -> Result<(i64, i64, PersistedModuleState), ModulePersistenceError> {
        let mut transaction = self.client.transaction()?;
        validate_module_replay_identity(context.session_id, context.activity_id, context.actor_id)?;
        ensure_character_account(&mut transaction, character_id, context.account_id)?;
        let duplicate: bool = transaction
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM replay_events \
                 WHERE event_type = 'module_state_snapshot' AND session_id = $1 \
                   AND account_id = $2 AND actor_id = $3)",
                &[&context.session_id, &context.account_id, &context.actor_id],
            )?
            .get(0);
        if duplicate {
            return Err(ModulePersistenceError::InvalidPersistedState(
                "participant module snapshot is already persisted".to_owned(),
            ));
        }
        let persisted = load_persisted_module_state(&mut transaction, character_id)?;
        let state = module_replay_state(
            &persisted,
            context.protocol_generation,
            context.catalog_revision,
        )?;
        let snapshot_payload = encode_module_state_snapshot(&ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: character_id.to_owned(),
            protocol_generation: context.protocol_generation,
            state,
        })?;
        let joined_kind = ReplayEventKind::PlayerJoined.to_string();
        let joined_id = insert_replay_event(
            &mut transaction,
            &NewReplayEvent {
                event_type: &joined_kind,
                session_id: context.session_id,
                account_id: context.account_id,
                activity_id: Some(context.activity_id),
                actor_id: Some(context.actor_id),
                payload: context.player_joined_payload,
            },
        )?;
        let snapshot_kind = ReplayEventKind::ModuleStateSnapshot.to_string();
        let snapshot_id = insert_replay_event(
            &mut transaction,
            &NewReplayEvent {
                event_type: &snapshot_kind,
                session_id: context.session_id,
                account_id: context.account_id,
                activity_id: Some(context.activity_id),
                actor_id: Some(context.actor_id),
                payload: &snapshot_payload,
            },
        )?;
        transaction.commit()?;
        Ok((joined_id, snapshot_id, persisted))
    }

    /// Atomically consumes fragments, grants one fixed module, and records the
    /// accepted idempotency outcome.
    ///
    /// An accepted retry returns its stored result without another inventory or
    /// ledger write. Rejected requests roll back the lazily created module-state
    /// row and reserve no operation identifier.
    ///
    /// # Errors
    ///
    /// Returns a domain error for invalid lifecycle/preconditions/identity, a
    /// serialization error for an invalid stored outcome, or a database error.
    pub fn combine_module(
        &mut self,
        character_id: &str,
        phase: ActivityPhase,
        operation_id: &str,
        module_id: ModuleId,
    ) -> Result<OperationReceipt<CombinationOutcome>, ModulePersistenceError> {
        self.combine_module_internal(character_id, phase, operation_id, module_id, None)
    }

    /// Atomically consumes/grants, reserves the operation, and appends replay.
    ///
    /// # Errors
    ///
    /// Returns a domain, persisted-state, replay, or database error. Any error
    /// rolls back inventory, module state, operation identity, and replay.
    pub fn combine_module_with_replay(
        &mut self,
        character_id: &str,
        phase: ActivityPhase,
        operation_id: &str,
        module_id: ModuleId,
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<OperationReceipt<CombinationOutcome>, ModulePersistenceError> {
        self.combine_module_internal(character_id, phase, operation_id, module_id, Some(replay))
    }

    fn combine_module_internal(
        &mut self,
        character_id: &str,
        phase: ActivityPhase,
        operation_id: &str,
        module_id: ModuleId,
        replay: Option<ModuleMutationReplayContext<'_>>,
    ) -> Result<OperationReceipt<CombinationOutcome>, ModulePersistenceError> {
        validate_operation_id(operation_id).map_err(ModulePersistenceError::Domain)?;
        let request = StoredCombinationRequest { module_id };
        let mut transaction = self.client.transaction()?;
        ensure_and_lock_module_state(&mut transaction, character_id)?;
        if let Some((stored_request, outcome)) =
            load_stored_operation::<StoredCombinationRequest, CombinationOutcome, _>(
                &mut transaction,
                character_id,
                "combine",
                operation_id,
            )?
        {
            if stored_request != request {
                return Err(ModuleMutationError::IdempotencyConflict.into());
            }
            return Ok(OperationReceipt {
                disposition: OperationDisposition::Replayed,
                outcome,
            });
        }

        let persisted = load_persisted_module_state(&mut transaction, character_id)?;
        let mut state = module_state_from_persisted(&persisted)?;
        let receipt = state.combine(
            phase,
            CombinationRequest {
                operation_id,
                module_id,
            },
        )?;
        if persisted.combination_operations >= MAX_OPERATION_RECORDS_PER_KIND {
            return Err(ModuleMutationError::OperationLimitReached.into());
        }
        if let Some(context) = replay {
            validate_module_mutation_replay_context(&mut transaction, character_id, context)?;
        }
        let recipe = module_definition(module_id).recipe_fragments;
        let recipe_i32 = i32::try_from(recipe).map_err(|_| {
            ModulePersistenceError::InvalidPersistedState(
                "module recipe does not fit PostgreSQL inventory quantity".to_owned(),
            )
        })?;
        let resulting_quantity = transaction
            .query_opt(
                "UPDATE inventory SET quantity = quantity - $3 \
                 WHERE character_id = $1 AND item_id = $2 AND quantity >= $3 \
                 RETURNING quantity",
                &[&character_id, &RELAY_CORE_FRAGMENT, &recipe_i32],
            )?
            .ok_or_else(|| {
                ModulePersistenceError::InvalidPersistedState(
                    "locked fragment quantity no longer satisfies accepted recipe".to_owned(),
                )
            })?
            .get::<_, i32>(0);
        let resulting_quantity = u32::try_from(resulting_quantity).map_err(|_| {
            ModulePersistenceError::InvalidPersistedState(
                "resulting fragment quantity is negative".to_owned(),
            )
        })?;
        if resulting_quantity != receipt.outcome.resulting_fragments {
            return Err(ModulePersistenceError::InvalidPersistedState(
                "domain and PostgreSQL fragment arithmetic disagree".to_owned(),
            ));
        }
        transaction.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, $2, 1)",
            &[&character_id, &module_id.as_str()],
        )?;
        insert_stored_operation(
            &mut transaction,
            character_id,
            "combine",
            operation_id,
            &request,
            &receipt.outcome,
        )?;
        if let Some(context) = replay {
            insert_module_combined_replay(
                &mut transaction,
                character_id,
                operation_id,
                module_id,
                &persisted,
                &receipt.outcome,
                context,
            )?;
        }
        transaction.commit()?;
        Ok(receipt)
    }

    /// Atomically replaces all canonical module slots, advances revision once,
    /// and records the accepted idempotency outcome.
    ///
    /// # Errors
    ///
    /// Returns a domain error for invalid lifecycle/loadout/revision/identity, a
    /// serialization error for an invalid stored outcome, or a database error.
    pub fn set_module_loadout(
        &mut self,
        character_id: &str,
        phase: ActivityPhase,
        operation_id: &str,
        expected_revision: u64,
        requested_modules: &[ModuleId],
    ) -> Result<OperationReceipt<LoadoutOutcome>, ModulePersistenceError> {
        self.set_module_loadout_internal(
            character_id,
            phase,
            operation_id,
            expected_revision,
            requested_modules,
            None,
        )
    }

    /// Atomically replaces slots, advances revision, reserves identity, and appends replay.
    ///
    /// # Errors
    ///
    /// Returns a domain, persisted-state, replay, or database error. Any error
    /// rolls back slots, revision, operation identity, and replay.
    pub fn set_module_loadout_with_replay(
        &mut self,
        character_id: &str,
        phase: ActivityPhase,
        operation_id: &str,
        expected_revision: u64,
        requested_modules: &[ModuleId],
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<OperationReceipt<LoadoutOutcome>, ModulePersistenceError> {
        self.set_module_loadout_internal(
            character_id,
            phase,
            operation_id,
            expected_revision,
            requested_modules,
            Some(replay),
        )
    }

    fn set_module_loadout_internal(
        &mut self,
        character_id: &str,
        phase: ActivityPhase,
        operation_id: &str,
        expected_revision: u64,
        requested_modules: &[ModuleId],
        replay: Option<ModuleMutationReplayContext<'_>>,
    ) -> Result<OperationReceipt<LoadoutOutcome>, ModulePersistenceError> {
        validate_operation_id(operation_id).map_err(ModulePersistenceError::Domain)?;
        let modules = canonicalize_loadout(requested_modules)
            .map_err(ModuleMutationError::from)
            .map_err(ModulePersistenceError::Domain)?;
        let request = StoredLoadoutRequest {
            expected_revision,
            modules: modules.clone(),
        };
        let mut transaction = self.client.transaction()?;
        ensure_and_lock_module_state(&mut transaction, character_id)?;
        if let Some((stored_request, outcome)) =
            load_stored_operation::<StoredLoadoutRequest, LoadoutOutcome, _>(
                &mut transaction,
                character_id,
                "loadout",
                operation_id,
            )?
        {
            if stored_request != request {
                return Err(ModuleMutationError::IdempotencyConflict.into());
            }
            return Ok(OperationReceipt {
                disposition: OperationDisposition::Replayed,
                outcome,
            });
        }

        let persisted = load_persisted_module_state(&mut transaction, character_id)?;
        let mut state = module_state_from_persisted(&persisted)?;
        let receipt = state.set_loadout(
            phase,
            LoadoutRequest {
                operation_id,
                expected_revision,
                modules: &modules,
            },
        )?;
        if persisted.loadout_operations >= MAX_OPERATION_RECORDS_PER_KIND {
            return Err(ModuleMutationError::OperationLimitReached.into());
        }
        if let Some(context) = replay {
            validate_module_mutation_replay_context(&mut transaction, character_id, context)?;
        }
        transaction.execute(
            "DELETE FROM module_loadout_slots WHERE character_id = $1",
            &[&character_id],
        )?;
        for (slot_index, module_id) in receipt.outcome.resulting_modules.iter().enumerate() {
            let slot_index = i16::try_from(slot_index).map_err(|_| {
                ModulePersistenceError::InvalidPersistedState(
                    "module slot index does not fit PostgreSQL SMALLINT".to_owned(),
                )
            })?;
            transaction.execute(
                "INSERT INTO module_loadout_slots (character_id, slot_index, module_item_id) \
                 VALUES ($1, $2, $3)",
                &[&character_id, &slot_index, &module_id.as_str()],
            )?;
        }
        let resulting_revision =
            i64::try_from(receipt.outcome.resulting_revision).map_err(|_| {
                ModulePersistenceError::InvalidPersistedState(
                    "module revision does not fit PostgreSQL BIGINT".to_owned(),
                )
            })?;
        let expected_revision = i64::try_from(receipt.outcome.previous_revision).map_err(|_| {
            ModulePersistenceError::InvalidPersistedState(
                "previous module revision does not fit PostgreSQL BIGINT".to_owned(),
            )
        })?;
        if transaction.execute(
            "UPDATE module_states SET revision = $2 WHERE character_id = $1 AND revision = $3",
            &[&character_id, &resulting_revision, &expected_revision],
        )? != 1
        {
            return Err(ModulePersistenceError::InvalidPersistedState(
                "locked module revision changed during accepted loadout".to_owned(),
            ));
        }
        insert_stored_operation(
            &mut transaction,
            character_id,
            "loadout",
            operation_id,
            &request,
            &receipt.outcome,
        )?;
        if let Some(context) = replay {
            insert_module_loadout_replay(
                &mut transaction,
                character_id,
                operation_id,
                &persisted,
                &receipt.outcome,
                context,
            )?;
        }
        transaction.commit()?;
        Ok(receipt)
    }

    /// Atomically persists one server-resolved route choice and its participants.
    ///
    /// The session advisory lock makes concurrent same-input requests converge on
    /// one stored seed/event. A retry ignores its new candidate seed and returns
    /// the durable selection. Any conflicting request writes nothing.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid domain/input/catalog identity, mismatched
    /// account/character ownership, invalid stored evidence, or database failure.
    pub fn select_route_operation(
        &mut self,
        input: &RouteOperationSelection<'_>,
    ) -> Result<RouteOperationReceipt<RouteSelectionOutcome>, RoutePersistenceError> {
        self.select_route_operation_internal(input, None)
    }

    /// Atomically persists one route choice and its complete selection replay.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied evidence disagrees with the resolved
    /// selection, persisted admission snapshot/equipment, or any database write.
    pub fn select_route_operation_with_replay(
        &mut self,
        input: &RouteOperationSelection<'_>,
        replay: &RouteSelectedPayloadV1,
    ) -> Result<RouteOperationReceipt<RouteSelectionOutcome>, RoutePersistenceError> {
        self.select_route_operation_internal(input, Some(replay))
    }

    fn select_route_operation_internal(
        &mut self,
        input: &RouteOperationSelection<'_>,
        replay: Option<&RouteSelectedPayloadV1>,
    ) -> Result<RouteOperationReceipt<RouteSelectionOutcome>, RoutePersistenceError> {
        validate_route_storage_id(input.session_id, "session")?;
        validate_activity_id(input.activity_id)?;
        let mut transaction = self.client.transaction()?;
        transaction.query_one(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 27))",
            &[&input.session_id],
        )?;
        if let Some(stored) = load_route_operation(&mut transaction, input.session_id, true)? {
            let matches = stored.activity_id == input.activity_id
                && stored.selection.operation_id == input.operation_id
                && stored.selection.leader_id == input.requester_account_id
                && stored.selection.route_id == input.route_id
                && route_participants_match(&stored.participants, input.participants);
            if !matches {
                return Err(RouteError::IdempotencyConflict.into());
            }
            validate_route_replay_mode(
                &mut transaction,
                input.session_id,
                replay.is_some(),
                "route_selected",
            )?;
            if replay.is_some() {
                load_route_selection_replay(&mut transaction, input.session_id, &stored)?;
            }
            let outcome = stored.selection;
            transaction.commit()?;
            return Ok(RouteOperationReceipt {
                disposition: RouteOperationDisposition::Replayed,
                outcome,
            });
        }

        let selection = resolve_route_selection(input)?;
        let participants = validate_route_participants(&mut transaction, input.participants)?;
        insert_route_selection(
            &mut transaction,
            input.session_id,
            input.activity_id,
            &selection,
            &participants,
        )?;
        if let Some(replay) = replay {
            let payload = validate_route_selection_replay(
                &mut transaction,
                input,
                &selection,
                &participants,
                replay,
            )?;
            let event_type = ReplayEventKind::RouteSelected.to_string();
            let actor_id = i64::try_from(participants[0].actor)
                .map_err(|_| invalid_route_state("leader actor identifier exceeds BIGINT"))?;
            insert_replay_event(
                &mut transaction,
                &NewReplayEvent {
                    event_type: &event_type,
                    session_id: input.session_id,
                    account_id: &participants[0].account,
                    activity_id: Some(input.activity_id),
                    actor_id: Some(actor_id),
                    payload: &payload,
                },
            )?;
        }
        transaction.commit()?;
        Ok(RouteOperationReceipt {
            disposition: RouteOperationDisposition::Applied,
            outcome: selection,
        })
    }

    /// Atomically records route success and applies every participant reward.
    ///
    /// # Errors
    ///
    /// Returns an error for missing/corrupt selection, deadline violation,
    /// terminal conflict, pre-existing partial grants, or database failure.
    pub fn complete_route_operation(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<RouteOperationReceipt<RouteOperationSummary>, RoutePersistenceError> {
        self.finish_route_operation(session_id, TerminalOutcome::Succeeded, elapsed_ms, None)
    }

    /// Atomically records route success, generic completion/rewards, and the
    /// validated terminal route replay as one transaction.
    ///
    /// # Errors
    ///
    /// Returns an error for contradictory replay evidence or any persistence
    /// failure; no terminal or reward write then becomes visible.
    pub fn complete_route_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteOperationReceipt<RouteOperationSummary>, RoutePersistenceError> {
        self.finish_route_operation(
            session_id,
            TerminalOutcome::Succeeded,
            elapsed_ms,
            Some(replay),
        )
    }

    /// Atomically records a route timeout without history or rewards.
    ///
    /// # Errors
    ///
    /// Returns an error for missing/corrupt selection, premature timeout,
    /// terminal conflict, or database failure.
    pub fn fail_route_operation_timeout(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> Result<RouteOperationReceipt<RouteOperationSummary>, RoutePersistenceError> {
        self.finish_route_operation(session_id, TerminalOutcome::FailedTimeout, elapsed_ms, None)
    }

    /// Atomically records route timeout and its terminal replay without any
    /// generic completion, history, or reward.
    ///
    /// # Errors
    ///
    /// Returns an error for premature timeout, contradictory replay evidence,
    /// or database failure.
    pub fn fail_route_operation_timeout_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteOperationReceipt<RouteOperationSummary>, RoutePersistenceError> {
        self.finish_route_operation(
            session_id,
            TerminalOutcome::FailedTimeout,
            elapsed_ms,
            Some(replay),
        )
    }

    fn finish_route_operation(
        &mut self,
        session_id: &str,
        outcome: TerminalOutcome,
        elapsed_ms: u64,
        replay: Option<&RouteTerminalPayloadV1>,
    ) -> Result<RouteOperationReceipt<RouteOperationSummary>, RoutePersistenceError> {
        validate_route_storage_id(session_id, "session")?;
        let mut transaction = self.client.transaction()?;
        let stored = load_route_operation(&mut transaction, session_id, true)?
            .ok_or_else(|| invalid_route_state("route selection was not found"))?;
        if let Some(summary) = stored.terminal.as_ref() {
            if summary.outcome != outcome || summary.elapsed_ms != elapsed_ms {
                return Err(RouteError::TerminalConflict.into());
            }
            let replay_payload = prepare_route_terminal_replay(
                &mut transaction,
                session_id,
                &stored,
                outcome,
                summary,
                replay,
            )?;
            validate_route_terminal_replay_retry(
                &mut transaction,
                session_id,
                &stored,
                outcome,
                replay_payload.as_deref(),
            )?;
            transaction.commit()?;
            return Ok(RouteOperationReceipt {
                disposition: RouteOperationDisposition::Replayed,
                outcome: summary.clone(),
            });
        }
        validate_terminal_deadline(outcome, elapsed_ms, stored.selection.duration_budget_ms)?;
        let reward = (outcome == TerminalOutcome::Succeeded).then_some(stored.selection.reward);
        let summary = route_terminal_summary(&stored, outcome, elapsed_ms);
        let terminal_payload = prepare_route_terminal_replay(
            &mut transaction,
            session_id,
            &stored,
            outcome,
            &summary,
            replay,
        )?;
        if let Some(reward) = reward {
            apply_route_rewards(
                &mut transaction,
                session_id,
                &stored.activity_id,
                &stored.participants,
                reward,
                replay.is_some(),
            )?;
        }
        persist_route_terminal(
            &mut transaction,
            session_id,
            &stored,
            outcome,
            elapsed_ms,
            terminal_payload.as_deref(),
        )?;
        transaction.commit()?;
        Ok(RouteOperationReceipt {
            disposition: RouteOperationDisposition::Applied,
            outcome: summary,
        })
    }

    /// Records a completed activity for operational history.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the insert fails.
    pub fn record_activity_completion(
        &mut self,
        account_id: &str,
        character_id: &str,
        activity_id: &str,
    ) -> Result<(), postgres::Error> {
        self.client.execute(
            "INSERT INTO activity_history (account_id, character_id, activity_id) VALUES ($1, $2, $3)",
            &[&account_id, &character_id, &activity_id],
        )?;
        Ok(())
    }

    /// Atomically records completion and grants idempotent inventory and progression rewards.
    ///
    /// Returns the resulting authoritative state, or `None` when this session was
    /// already applied to the character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the transaction fails.
    pub fn complete_activity_with_rewards(
        &mut self,
        completion: &ActivityCompletion<'_>,
    ) -> Result<Option<CompletionRewards>, postgres::Error> {
        let mut transaction = self.client.transaction()?;
        let rewards = apply_completion_reward(&mut transaction, completion)?;
        transaction.commit()?;
        Ok(rewards)
    }

    /// Atomically records one activity completion, every participant reward,
    /// and the corresponding existing replay vocabulary.
    ///
    /// Repeating the same participant tuple remains idempotent. The completion
    /// event is inserted at most once for the session, and reward replay events
    /// are inserted only with a newly applied reward grant.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when any write fails. The transaction then
    /// exposes none of the completion, reward, history, or replay writes.
    pub fn complete_activity_with_rewards_and_replay(
        &mut self,
        activity_completed: &NewReplayEvent<'_>,
        participants: &[ActivityParticipantCompletion<'_>],
    ) -> Result<Vec<Option<CompletionRewards>>, postgres::Error> {
        let mut transaction = self.client.transaction()?;
        transaction.execute(
            "INSERT INTO replay_events \
             (event_type, session_id, account_id, activity_id, actor_id, payload) \
             SELECT $1, $2, $3, $4, $5, $6 \
             WHERE NOT EXISTS ( \
                 SELECT 1 FROM replay_events \
                 WHERE session_id = $2 AND event_type = $1 \
             )",
            &[
                &activity_completed.event_type,
                &activity_completed.session_id,
                &activity_completed.account_id,
                &activity_completed.activity_id,
                &activity_completed.actor_id,
                &activity_completed.payload,
            ],
        )?;

        let mut results = Vec::with_capacity(participants.len());
        for participant in participants {
            let rewards = apply_completion_reward(&mut transaction, &participant.completion)?;
            if let Some(rewards) = &rewards {
                let loot_payload = format!(
                    "loot granted: {} x{} (total {})",
                    participant.completion.item_id,
                    participant.completion.item_quantity,
                    rewards.item_quantity
                );
                let progression_payload = format!(
                    "progression granted: +{} XP (total {}, level {} -> {})",
                    rewards.experience_granted,
                    rewards.experience,
                    rewards.previous_level,
                    rewards.level
                );
                insert_replay_event(
                    &mut transaction,
                    &NewReplayEvent {
                        event_type: "loot_granted",
                        session_id: participant.completion.session_id,
                        account_id: participant.completion.account_id,
                        activity_id: Some(participant.completion.activity_id),
                        actor_id: Some(participant.actor_id),
                        payload: &loot_payload,
                    },
                )?;
                insert_replay_event(
                    &mut transaction,
                    &NewReplayEvent {
                        event_type: "progression_granted",
                        session_id: participant.completion.session_id,
                        account_id: participant.completion.account_id,
                        activity_id: Some(participant.completion.activity_id),
                        actor_id: Some(participant.actor_id),
                        payload: &progression_payload,
                    },
                )?;
            }
            results.push(rewards);
        }
        transaction.commit()?;
        Ok(results)
    }

    /// Counts recorded completions for smoke and diagnostic tooling.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn activity_completion_count(
        &mut self,
        account_id: &str,
        activity_id: &str,
    ) -> Result<i64, postgres::Error> {
        self.client
            .query_one(
                "SELECT COUNT(*) FROM activity_history WHERE account_id = $1 AND activity_id = $2",
                &[&account_id, &activity_id],
            )
            .map(|row| row.get(0))
    }

    /// Appends one immutable event to a replay session.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the insert fails.
    pub fn append_replay_event(
        &mut self,
        event: &NewReplayEvent<'_>,
    ) -> Result<i64, postgres::Error> {
        insert_replay_event(&mut self.client, event)
    }

    /// Loads one session in its authoritative append order.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn replay_events(
        &mut self,
        session_id: &str,
    ) -> Result<Vec<PersistedReplayEvent>, postgres::Error> {
        self.client
            .query(
                "SELECT id, event_type, occurred_at::TEXT, session_id, account_id, activity_id, actor_id, payload \
                 FROM replay_events WHERE session_id = $1 ORDER BY id",
                &[&session_id],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| PersistedReplayEvent {
                        id: row.get(0),
                        event_type: row.get(1),
                        timestamp: row.get(2),
                        session_id: row.get(3),
                        account_id: row.get(4),
                        activity_id: row.get(5),
                        actor_id: row.get(6),
                        payload: row.get(7),
                    })
                    .collect()
            })
    }

    /// Lists the most recent replay sessions for inspection.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn replay_sessions(
        &mut self,
        limit: i64,
    ) -> Result<Vec<PersistedReplaySession>, postgres::Error> {
        self.client
            .query(
                "SELECT session_id, MAX(activity_id), MIN(occurred_at)::TEXT, \
                        MAX(occurred_at)::TEXT, COUNT(*), COUNT(DISTINCT account_id), \
                        BOOL_OR(event_type = 'activity_completed') \
                 FROM replay_events GROUP BY session_id \
                 ORDER BY MAX(id) DESC LIMIT $1",
                &[&limit.clamp(1, 100)],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| PersistedReplaySession {
                        session_id: row.get(0),
                        activity_id: row.get(1),
                        started_at: row.get(2),
                        ended_at: row.get(3),
                        event_count: row.get(4),
                        participant_count: row.get(5),
                        completed: row.get(6),
                    })
                    .collect()
            })
    }

    /// Derives the bounded authoritative playtest summary for one replay session.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or persisted timestamps contradict replay order.
    #[allow(clippy::too_many_lines)]
    pub fn authoritative_session_summary(
        &mut self,
        session_id: &str,
    ) -> Result<Option<AuthoritativeSessionSummary>, SessionSummaryError> {
        let row = self.client.query_opt(
            "WITH summary AS ( \
                 SELECT session_id, MAX(activity_id) AS activity_id, \
                        MIN(occurred_at) FILTER (WHERE event_type = 'player_joined') AS first_joined_at, \
                        MIN(occurred_at) FILTER (WHERE event_type = 'activity_started') AS activity_started_at, \
                        MIN(occurred_at) FILTER (WHERE event_type = 'activity_completed') AS completed_at, \
                        MAX(occurred_at) AS latest_at, COUNT(DISTINCT account_id) AS participant_count, \
                        COUNT(*) FILTER (WHERE event_type = 'enemy_spawned') AS enemy_spawn_count, \
                        COUNT(*) FILTER (WHERE event_type = 'enemy_died') AS enemy_defeat_count, \
                        BOOL_OR(event_type = 'boss_spawned') AS boss_spawned, \
                        COUNT(*) FILTER (WHERE event_type = 'equipment_changed') AS equipment_change_count, \
                        COUNT(*) FILTER (WHERE event_type = 'loot_granted') AS loot_grant_count, \
                        COUNT(*) FILTER (WHERE event_type = 'progression_granted') AS progression_grant_count, \
                        COUNT(*) AS event_count, \
                        COUNT(*) FILTER (WHERE event_type NOT IN ( \
                            'player_joined', 'activity_started', 'enemy_spawned', 'enemy_died', \
                            'boss_spawned', 'activity_completed', 'loot_granted', \
                            'progression_granted', 'equipment_changed', 'field_activity', \
                            'module_state_snapshot', 'module_combined', \
                            'module_loadout_changed', 'acquisition_claimed', 'campaign_checkpoint', 'campaign_resumed', \
                            'campaign_objective', 'campaign_story', 'challenge_checkpoint', 'challenge_objective', 'route_selected', \
                            'route_operation_succeeded', 'route_operation_failed', \
                            'cooperation_started', 'cooperation_pinged', 'player_downed', \
                            'player_revived', 'cooperation_succeeded', 'cooperation_failed' \
                        )) AS unknown_event_count \
                 FROM replay_events WHERE session_id = $1 GROUP BY session_id \
             ) \
             SELECT session_id, activity_id, first_joined_at::TEXT, activity_started_at::TEXT, \
                    COALESCE(completed_at, latest_at)::TEXT, \
                    CASE WHEN first_joined_at IS NULL OR activity_started_at IS NULL THEN NULL \
                         ELSE (EXTRACT(EPOCH FROM (activity_started_at - first_joined_at)) * 1000)::BIGINT END, \
                    CASE WHEN activity_started_at IS NULL OR completed_at IS NULL THEN NULL \
                         ELSE (EXTRACT(EPOCH FROM (completed_at - activity_started_at)) * 1000)::BIGINT END, \
                    participant_count, completed_at IS NOT NULL, enemy_spawn_count, enemy_defeat_count, \
                    boss_spawned, equipment_change_count, loot_grant_count, progression_grant_count, event_count, \
                    unknown_event_count \
             FROM summary",
            &[&session_id],
        )?;
        let Some(row) = row else {
            return Ok(None);
        };
        let first_joined_at =
            row.get::<_, Option<String>>(2)
                .ok_or(SessionSummaryError::InvalidEvidence(
                    "session has no player_joined event",
                ))?;
        let join_to_start_ms: Option<i64> = row.get(5);
        let activity_duration_ms: Option<i64> = row.get(6);
        if join_to_start_ms.is_some_and(|elapsed| elapsed < 0) {
            return Err(SessionSummaryError::InvalidEvidence(
                "activity_started precedes player_joined",
            ));
        }
        if activity_duration_ms.is_some_and(|elapsed| elapsed < 0) {
            return Err(SessionSummaryError::InvalidEvidence(
                "activity_completed precedes activity_started",
            ));
        }
        if row.get::<_, i64>(16) > 0 {
            return Err(SessionSummaryError::InvalidEvidence(
                "session contains an unknown replay event kind",
            ));
        }
        let reconstructed = reconstruct_persisted_events(self.replay_events(session_id)?)?;
        let route_selection = reconstructed.route.selection.as_ref();
        let route_terminal = reconstructed.route.terminal.as_ref();
        let cooperation_started = reconstructed.cooperation.started.as_ref();
        let cooperation_terminal = reconstructed.cooperation.terminal.as_ref();
        Ok(Some(AuthoritativeSessionSummary {
            session_id: row.get(0),
            activity_id: row.get(1),
            first_joined_at,
            activity_started_at: row.get(3),
            activity_ended_at: row.get(4),
            join_to_start_ms,
            activity_duration_ms,
            participant_count: row.get(7),
            completed: row.get(8),
            enemy_spawn_count: count_as_i64(reconstructed.spawned_enemies)?,
            enemy_defeat_count: row.get(10),
            boss_spawned: row.get(11),
            equipment_change_count: row.get(12),
            loot_grant_count: row.get(13),
            progression_grant_count: row.get(14),
            module_replay_legacy: reconstructed.module_replay_legacy,
            module_participant_count: count_as_i64(reconstructed.module_participants.len())?,
            module_snapshot_count: count_as_i64(reconstructed.module_snapshots)?,
            module_combination_count: count_as_i64(reconstructed.module_combinations)?,
            module_loadout_change_count: count_as_i64(reconstructed.module_loadout_changes)?,
            acquisition_claim_count: count_as_i64(reconstructed.acquisition_claims.len())?,
            route_replay_legacy: route_selection.is_none(),
            route_id: route_selection.map(|selection| selection.route_id),
            route_event_id: route_selection.map(|selection| selection.event_id),
            route_terminal_outcome: route_terminal.map(|terminal| terminal.summary.outcome),
            route_elapsed_ms: route_terminal.map(|terminal| terminal.summary.elapsed_ms),
            route_transition_count: count_as_i64(
                route_terminal.map_or(0, |terminal| terminal.transitions.len()),
            )?,
            route_reward_participant_count: count_as_i64(
                route_terminal.map_or(0, |terminal| terminal.grants.len()),
            )?,
            cooperation_replay_legacy: cooperation_started.is_none(),
            cooperation_replay_state: reconstructed.cooperation.state,
            cooperation_last_phase: reconstructed.cooperation.last_phase(),
            cooperation_terminal_outcome: cooperation_terminal.map(|terminal| terminal.outcome),
            cooperation_terminal_elapsed_ms: cooperation_terminal
                .map(|terminal| terminal.terminal_elapsed_ms),
            cooperation_participant_count: count_as_i64(
                cooperation_started.map_or(0, |started| started.participants.len()),
            )?,
            cooperation_contribution_count: count_as_i64(
                reconstructed.cooperation.contribution_count(),
            )?,
            cooperation_revive_count: i64::from(cooperation_terminal.map_or_else(
                || u8::from(reconstructed.cooperation.revived.is_some()),
                |terminal| terminal.contributions.revive_count,
            )),
            cooperation_reward_participant_count: count_as_i64(
                cooperation_terminal.map_or(0, |terminal| terminal.grants.len()),
            )?,
            event_count: row.get(15),
        }))
    }

    /// Finds the most recently completed replay session for an account.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn latest_completed_session_id(
        &mut self,
        account_id: &str,
    ) -> Result<Option<String>, postgres::Error> {
        self.client
            .query_opt(
                "SELECT completed.session_id FROM replay_events AS completed \
                 WHERE completed.event_type = 'activity_completed' \
                   AND EXISTS (SELECT 1 FROM replay_events AS participant \
                               WHERE participant.session_id = completed.session_id \
                                 AND participant.account_id = $1) \
                 ORDER BY completed.id DESC LIMIT 1",
                &[&account_id],
            )
            .map(|row| row.map(|row| row.get(0)))
    }
}

fn reconstruct_persisted_events(
    events: Vec<PersistedReplayEvent>,
) -> Result<ReconstructedSession, SessionSummaryError> {
    let events = events
        .into_iter()
        .map(|event| {
            let kind = event.event_type.parse::<ReplayEventKind>().map_err(|_| {
                SessionSummaryError::InvalidEvidence(
                    "session contains an unknown replay event kind",
                )
            })?;
            let actor_id = event.actor_id.map(u64::try_from).transpose().map_err(|_| {
                SessionSummaryError::InvalidEvidence(
                    "session contains a negative replay actor identifier",
                )
            })?;
            Ok(DomainReplayEvent {
                id: event.id,
                kind,
                timestamp: event.timestamp,
                session_id: event.session_id,
                account_id: event.account_id,
                activity_id: event.activity_id,
                actor_id,
                payload: event.payload,
            })
        })
        .collect::<Result<Vec<_>, SessionSummaryError>>()?;
    reconstruct(&events)
        .map_err(|_| SessionSummaryError::InvalidEvidence("session replay reconstruction failed"))
}

fn count_as_i64(count: usize) -> Result<i64, SessionSummaryError> {
    i64::try_from(count).map_err(|_| {
        SessionSummaryError::InvalidEvidence("session replay count exceeds signed bounds")
    })
}

fn module_replay_state(
    persisted: &PersistedModuleState,
    protocol_generation: ReplayProtocolGeneration,
    catalog_revision: Option<&str>,
) -> Result<revenant_replay::ReplayModuleStateEvidence, ModulePersistenceError> {
    if let Some(revision) = catalog_revision {
        // A negotiated replay records the same ownership projection as its
        // client. Newer unequipped modules remain in durable inventory.
        let owned = persisted
            .owned_modules
            .iter()
            .copied()
            .filter(|id| match revision {
                revenant_modules::CATALOG_REVISION => ModuleId::LEGACY.contains(id),
                revenant_modules::CONTENT_CATALOG_REVISION
                | revenant_modules::ARSENAL_CATALOG_REVISION => ModuleId::CONTENT.contains(id),
                _ => true,
            })
            .collect::<Vec<_>>();
        return Ok(revenant_replay::module_state_evidence_for_catalog(
            revision,
            persisted.fragments,
            &owned,
            &persisted.loadout,
            persisted.revision,
            protocol_generation,
        )?);
    }
    Ok(module_state_evidence(
        persisted.fragments,
        &persisted.owned_modules,
        &persisted.loadout,
        persisted.revision,
        protocol_generation,
    )?)
}

fn insert_module_combined_replay(
    client: &mut impl GenericClient,
    character_id: &str,
    operation_id: &str,
    module_id: ModuleId,
    persisted: &PersistedModuleState,
    outcome: &CombinationOutcome,
    context: ModuleMutationReplayContext<'_>,
) -> Result<(), ModulePersistenceError> {
    let before = module_replay_state(
        persisted,
        ReplayProtocolGeneration::V2,
        context.catalog_revision,
    )?;
    let mut resulting_owned = before.owned_modules.clone();
    resulting_owned.push(module_id);
    let after = revenant_replay::module_state_evidence_for_catalog(
        &before.catalog_revision,
        outcome.resulting_fragments,
        &resulting_owned,
        &persisted.loadout,
        persisted.revision,
        ReplayProtocolGeneration::V2,
    )?;
    let payload = encode_module_combined(&ModuleCombinedPayloadV1 {
        schema_version: MODULE_REPLAY_SCHEMA_VERSION,
        character_id: character_id.to_owned(),
        operation_id: operation_id.to_owned(),
        module_id,
        recipe_fragments: outcome.recipe_fragments,
        before,
        after,
    })?;
    let event_type = ReplayEventKind::ModuleCombined.to_string();
    insert_replay_event(
        client,
        &module_mutation_replay_event(context, &event_type, &payload),
    )?;
    Ok(())
}

fn insert_module_loadout_replay(
    client: &mut impl GenericClient,
    character_id: &str,
    operation_id: &str,
    persisted: &PersistedModuleState,
    outcome: &LoadoutOutcome,
    context: ModuleMutationReplayContext<'_>,
) -> Result<(), ModulePersistenceError> {
    let before = module_replay_state(
        persisted,
        ReplayProtocolGeneration::V2,
        context.catalog_revision,
    )?;
    let after = revenant_replay::module_state_evidence_for_catalog(
        &before.catalog_revision,
        persisted.fragments,
        &before.owned_modules,
        &outcome.resulting_modules,
        outcome.resulting_revision,
        ReplayProtocolGeneration::V2,
    )?;
    let payload = encode_module_loadout_changed(&ModuleLoadoutChangedPayloadV1 {
        schema_version: MODULE_REPLAY_SCHEMA_VERSION,
        character_id: character_id.to_owned(),
        operation_id: operation_id.to_owned(),
        before,
        after,
    })?;
    let event_type = ReplayEventKind::ModuleLoadoutChanged.to_string();
    insert_replay_event(
        client,
        &module_mutation_replay_event(context, &event_type, &payload),
    )?;
    Ok(())
}

fn ensure_character_account(
    client: &mut impl GenericClient,
    character_id: &str,
    account_id: &str,
) -> Result<(), ModulePersistenceError> {
    let matches: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM characters WHERE id = $1 AND account_id = $2)",
            &[&character_id, &account_id],
        )?
        .get(0);
    if !matches {
        return Err(ModulePersistenceError::InvalidPersistedState(
            "module replay account does not own the character".to_owned(),
        ));
    }
    Ok(())
}

fn validate_module_mutation_replay_context(
    client: &mut impl GenericClient,
    character_id: &str,
    context: ModuleMutationReplayContext<'_>,
) -> Result<(), ModulePersistenceError> {
    validate_module_replay_identity(context.session_id, context.activity_id, context.actor_id)?;
    ensure_character_account(client, character_id, context.account_id)?;
    let ready: bool = client
        .query_one(
            "SELECT EXISTS ( \
                 SELECT 1 FROM replay_events AS snapshot \
                 WHERE snapshot.event_type = 'module_state_snapshot' \
                   AND snapshot.session_id = $1 AND snapshot.account_id = $2 \
                   AND snapshot.activity_id = $3 AND snapshot.actor_id = $4 \
                   AND EXISTS ( \
                       SELECT 1 FROM replay_events AS completed \
                       WHERE completed.session_id = snapshot.session_id \
                         AND completed.event_type = 'activity_completed' \
                         AND completed.id > snapshot.id \
                   ) \
             )",
            &[
                &context.session_id,
                &context.account_id,
                &context.activity_id,
                &context.actor_id,
            ],
        )?
        .get(0);
    if !ready {
        return Err(ModulePersistenceError::InvalidPersistedState(
            "module mutation replay has no matching completed participant snapshot".to_owned(),
        ));
    }
    Ok(())
}

fn validate_module_replay_identity(
    session_id: &str,
    activity_id: &str,
    actor_id: i64,
) -> Result<(), ModulePersistenceError> {
    let valid_session = !session_id.is_empty()
        && session_id.len() <= 128
        && session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    let valid_activity = !activity_id.is_empty()
        && activity_id.len() <= 64
        && activity_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    if !valid_session || !valid_activity || actor_id <= 0 {
        return Err(ModulePersistenceError::InvalidPersistedState(
            "module replay session, activity, or actor identity is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn module_mutation_replay_event<'a>(
    context: ModuleMutationReplayContext<'a>,
    event_type: &'a str,
    payload: &'a str,
) -> NewReplayEvent<'a> {
    NewReplayEvent {
        event_type,
        session_id: context.session_id,
        account_id: context.account_id,
        activity_id: Some(context.activity_id),
        actor_id: Some(context.actor_id),
        payload,
    }
}

fn ensure_and_lock_module_state(
    client: &mut impl GenericClient,
    character_id: &str,
) -> Result<(), ModulePersistenceError> {
    client.execute(
        "INSERT INTO module_states (character_id, revision) \
         SELECT $1, 0 WHERE EXISTS (SELECT 1 FROM characters WHERE id = $1) \
         ON CONFLICT (character_id) DO NOTHING",
        &[&character_id],
    )?;
    if client
        .query_opt(
            "SELECT revision FROM module_states WHERE character_id = $1 FOR UPDATE",
            &[&character_id],
        )?
        .is_none()
    {
        return Err(ModulePersistenceError::CharacterNotFound);
    }
    Ok(())
}

fn load_persisted_module_state(
    client: &mut impl GenericClient,
    character_id: &str,
) -> Result<PersistedModuleState, ModulePersistenceError> {
    let character_exists: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM characters WHERE id = $1)",
            &[&character_id],
        )?
        .get(0);
    if !character_exists {
        return Err(ModulePersistenceError::CharacterNotFound);
    }
    let (fragments, owned_modules) = load_module_inventory(client, character_id)?;
    let (revision, loadout) = load_module_loadout(client, character_id)?;
    let (combination_operations, loadout_operations) =
        load_module_operation_counts(client, character_id)?;
    let state = PersistedModuleState {
        fragments,
        owned_modules,
        loadout,
        revision,
        combination_operations,
        loadout_operations,
    };
    module_state_from_persisted(&state)?;
    Ok(state)
}

fn load_module_inventory(
    client: &mut impl GenericClient,
    character_id: &str,
) -> Result<(u32, Vec<ModuleId>), ModulePersistenceError> {
    let fragments = client
        .query_opt(
            "SELECT quantity FROM inventory WHERE character_id = $1 AND item_id = $2",
            &[&character_id, &RELAY_CORE_FRAGMENT],
        )?
        .map_or(Ok(0), |row| {
            u32::try_from(row.get::<_, i32>(0)).map_err(|_| {
                ModulePersistenceError::InvalidPersistedState(
                    "fragment quantity is negative".to_owned(),
                )
            })
        })?;
    let mut owned_modules = client
        .query(
            "SELECT item_id, quantity FROM inventory \
             WHERE character_id = $1 AND item_id IN ( \
                 'module_force_matrix', 'module_tempo_regulator', \
                 'module_reach_lattice', 'module_ward_capacitor', \
                 'module_focus_lens', 'module_cycle_bypass', \
                 'module_breach_shunt', 'module_standoff_optic', \
                 'module_skirmish_drive', 'module_ablative_shell' \
             ) ORDER BY item_id",
            &[&character_id],
        )?
        .into_iter()
        .map(|row| {
            let item_id: String = row.get(0);
            let quantity: i32 = row.get(1);
            if quantity != 1 {
                return Err(ModulePersistenceError::InvalidPersistedState(format!(
                    "owned module {item_id} has quantity {quantity}, expected one"
                )));
            }
            ModuleId::from_str(&item_id)
                .map_err(|error| ModulePersistenceError::InvalidPersistedState(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    owned_modules.sort_unstable();
    Ok((fragments, owned_modules))
}

fn load_module_loadout(
    client: &mut impl GenericClient,
    character_id: &str,
) -> Result<(u64, Vec<ModuleId>), ModulePersistenceError> {
    let revision = client
        .query_opt(
            "SELECT revision FROM module_states WHERE character_id = $1",
            &[&character_id],
        )?
        .map_or(Ok(0), |row| {
            u64::try_from(row.get::<_, i64>(0)).map_err(|_| {
                ModulePersistenceError::InvalidPersistedState(
                    "module revision is negative".to_owned(),
                )
            })
        })?;
    let slot_rows = client.query(
        "SELECT slot_index, module_item_id FROM module_loadout_slots \
         WHERE character_id = $1 ORDER BY slot_index",
        &[&character_id],
    )?;
    let mut loadout = Vec::with_capacity(slot_rows.len());
    for (expected_index, row) in slot_rows.into_iter().enumerate() {
        let slot_index: i16 = row.get(0);
        if usize::try_from(slot_index).ok() != Some(expected_index) {
            return Err(ModulePersistenceError::InvalidPersistedState(
                "module slots are not contiguous from zero".to_owned(),
            ));
        }
        let item_id: String = row.get(1);
        loadout.push(
            ModuleId::from_str(&item_id).map_err(|error| {
                ModulePersistenceError::InvalidPersistedState(error.to_string())
            })?,
        );
    }
    let canonical = canonicalize_loadout(&loadout)
        .map_err(|error| ModulePersistenceError::InvalidPersistedState(error.to_string()))?;
    if canonical != loadout {
        return Err(ModulePersistenceError::InvalidPersistedState(
            "module slots are not in canonical family order".to_owned(),
        ));
    }
    Ok((revision, loadout))
}

fn load_module_operation_counts(
    client: &mut impl GenericClient,
    character_id: &str,
) -> Result<(usize, usize), ModulePersistenceError> {
    let operation_counts = client.query_one(
        "SELECT COUNT(*) FILTER (WHERE operation_kind = 'combine'), \
                COUNT(*) FILTER (WHERE operation_kind = 'loadout') \
         FROM module_operations WHERE character_id = $1",
        &[&character_id],
    )?;
    let combination_operations =
        usize::try_from(operation_counts.get::<_, i64>(0)).map_err(|_| {
            ModulePersistenceError::InvalidPersistedState(
                "combination operation count is invalid".to_owned(),
            )
        })?;
    let loadout_operations = usize::try_from(operation_counts.get::<_, i64>(1)).map_err(|_| {
        ModulePersistenceError::InvalidPersistedState(
            "loadout operation count is invalid".to_owned(),
        )
    })?;
    Ok((combination_operations, loadout_operations))
}

fn module_state_from_persisted(
    persisted: &PersistedModuleState,
) -> Result<ModuleState, ModulePersistenceError> {
    ModuleState::new(
        persisted.fragments,
        &persisted.owned_modules,
        &persisted.loadout,
        persisted.revision,
    )
    .map_err(|error| ModulePersistenceError::InvalidPersistedState(error.to_string()))
}

fn load_stored_operation<Request, Outcome, ClientType>(
    client: &mut ClientType,
    character_id: &str,
    operation_kind: &str,
    operation_id: &str,
) -> Result<Option<(Request, Outcome)>, ModulePersistenceError>
where
    Request: DeserializeOwned,
    Outcome: DeserializeOwned,
    ClientType: GenericClient,
{
    client
        .query_opt(
            "SELECT request_payload::TEXT, result_payload::TEXT FROM module_operations \
             WHERE character_id = $1 AND operation_kind = $2 AND operation_id = $3",
            &[&character_id, &operation_kind, &operation_id],
        )?
        .map(|row| {
            let request = serde_json::from_str(&row.get::<_, String>(0))?;
            let outcome = serde_json::from_str(&row.get::<_, String>(1))?;
            Ok((request, outcome))
        })
        .transpose()
}

fn insert_stored_operation<Request, Outcome>(
    client: &mut impl GenericClient,
    character_id: &str,
    operation_kind: &str,
    operation_id: &str,
    request: &Request,
    outcome: &Outcome,
) -> Result<(), ModulePersistenceError>
where
    Request: Serialize,
    Outcome: Serialize,
{
    let request_payload = serde_json::to_string(request)?;
    let result_payload = serde_json::to_string(outcome)?;
    if client.execute(
        "INSERT INTO module_operations \
         (character_id, operation_kind, operation_id, request_payload, result_payload) \
         VALUES ($1, $2, $3, $4::TEXT::JSONB, $5::TEXT::JSONB)",
        &[
            &character_id,
            &operation_kind,
            &operation_id,
            &request_payload,
            &result_payload,
        ],
    )? != 1
    {
        return Err(ModulePersistenceError::InvalidPersistedState(
            "accepted module operation was not inserted".to_owned(),
        ));
    }
    Ok(())
}

fn insert_replay_event(
    client: &mut impl GenericClient,
    event: &NewReplayEvent<'_>,
) -> Result<i64, postgres::Error> {
    client
        .query_one(
            "INSERT INTO replay_events \
             (event_type, session_id, account_id, activity_id, actor_id, payload) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            &[
                &event.event_type,
                &event.session_id,
                &event.account_id,
                &event.activity_id,
                &event.actor_id,
                &event.payload,
            ],
        )
        .map(|row| row.get(0))
}

fn validate_route_replay_mode(
    client: &mut impl GenericClient,
    session_id: &str,
    replay_expected: bool,
    event_type: &str,
) -> Result<(), RoutePersistenceError> {
    let count = client
        .query_one(
            "SELECT COUNT(*) FROM replay_events WHERE session_id = $1 AND event_type = $2",
            &[&session_id, &event_type],
        )?
        .get::<_, i64>(0);
    let expected = i64::from(replay_expected);
    if count != expected {
        return Err(invalid_route_state(format!(
            "route replay event {event_type} count is {count}, expected {expected}"
        )));
    }
    Ok(())
}

fn validate_route_selection_replay(
    client: &mut impl GenericClient,
    input: &RouteOperationSelection<'_>,
    selection: &RouteSelectionOutcome,
    participants: &[StoredRouteParticipant],
    replay: &RouteSelectedPayloadV1,
) -> Result<String, RoutePersistenceError> {
    validate_route_replay_mode(client, input.session_id, false, "route_selected")?;
    validate_selection_replay_matches_storage(replay, input.activity_id, selection, participants)?;
    for (evidence, participant) in replay.participants.iter().zip(participants) {
        let rows = client.query(
            "SELECT payload FROM replay_events WHERE event_type = 'module_state_snapshot' \
             AND session_id = $1 AND account_id = $2 AND activity_id = $3 AND actor_id = $4",
            &[
                &input.session_id,
                &participant.account,
                &input.activity_id,
                &i64::try_from(participant.actor)
                    .map_err(|_| invalid_route_state("participant actor exceeds BIGINT"))?,
            ],
        )?;
        if rows.len() != 1 {
            return Err(invalid_route_state(
                "route participant lacks exactly one immutable M26 snapshot",
            ));
        }
        let snapshot_payload = rows[0].get::<_, String>(0);
        let Some(DecodedModuleReplayPayload::ModuleStateSnapshot(snapshot)) =
            decode_module_payload(ReplayEventKind::ModuleStateSnapshot, &snapshot_payload)?
        else {
            return Err(invalid_route_state(
                "module snapshot decoded as another event",
            ));
        };
        if snapshot.protocol_generation != ReplayProtocolGeneration::V2
            || snapshot.character_id != participant.character
            || snapshot.state != evidence.module_state
        {
            return Err(invalid_route_state(
                "route replay participant disagrees with its M26 admission snapshot",
            ));
        }
        let weapon = client
            .query_opt(
                "SELECT weapon_item_id FROM equipment_loadouts WHERE character_id = $1 FOR SHARE",
                &[&participant.character],
            )?
            .ok_or_else(|| invalid_route_state("route participant has no equipped weapon"))?
            .get::<_, String>(0);
        if weapon != evidence.weapon_item_id {
            return Err(invalid_route_state(
                "route replay weapon disagrees with immutable admission equipment",
            ));
        }
    }
    encode_route_selected(replay).map_err(Into::into)
}

fn validate_selection_replay_matches_storage(
    replay: &RouteSelectedPayloadV1,
    activity_id: &str,
    selection: &RouteSelectionOutcome,
    participants: &[StoredRouteParticipant],
) -> Result<(), RoutePersistenceError> {
    if replay.activity_id != activity_id
        || replay.operation_id != selection.operation_id
        || replay.leader_id != selection.leader_id
        || replay.route_id != selection.route_id
        || replay.seed != selection.seed
        || replay.event_id != selection.event_id
        || replay.effect != selection.effect
        || replay.duration_budget_ms != selection.duration_budget_ms
        || replay.reward != selection.reward
        || replay
            .objectives
            .iter()
            .map(|objective| objective.objective_id)
            .ne(selection.objective_path.iter().copied())
        || replay.participants.len() != participants.len()
        || replay
            .participants
            .iter()
            .zip(participants)
            .any(|(evidence, participant)| {
                evidence.participant_id != participant.account
                    || evidence.character_id != participant.character
                    || evidence.actor_id != participant.actor
            })
    {
        return Err(invalid_route_state(
            "route selection replay disagrees with durable selection",
        ));
    }
    Ok(())
}

fn load_route_selection_replay(
    client: &mut impl GenericClient,
    session_id: &str,
    stored: &StoredRouteOperation,
) -> Result<RouteSelectedPayloadV1, RoutePersistenceError> {
    let rows = client.query(
        "SELECT account_id, activity_id, actor_id, payload FROM replay_events \
         WHERE session_id = $1 AND event_type = 'route_selected' ORDER BY id",
        &[&session_id],
    )?;
    if rows.len() != 1 {
        return Err(invalid_route_state(
            "routed replay requires exactly one selection event",
        ));
    }
    let payload_text = rows[0].get::<_, String>(3);
    let Some(DecodedRouteReplayPayload::RouteSelected(payload)) =
        decode_route_payload(ReplayEventKind::RouteSelected, &payload_text)?
    else {
        return Err(invalid_route_state(
            "route selection decoded as another event",
        ));
    };
    validate_selection_replay_matches_storage(
        &payload,
        &stored.activity_id,
        &stored.selection,
        &stored.participants,
    )?;
    let leader = &stored.participants[0];
    let actor_id = rows[0]
        .get::<_, Option<i64>>(2)
        .and_then(|actor| u64::try_from(actor).ok());
    if rows[0].get::<_, String>(0) != leader.account
        || rows[0].get::<_, Option<String>>(1).as_deref() != Some(stored.activity_id.as_str())
        || actor_id != Some(leader.actor)
    {
        return Err(invalid_route_state(
            "route selection row identity disagrees with durable leader",
        ));
    }
    Ok(payload)
}

const fn terminal_replay_event_type(outcome: TerminalOutcome) -> &'static str {
    match outcome {
        TerminalOutcome::Succeeded => "route_operation_succeeded",
        TerminalOutcome::FailedTimeout => "route_operation_failed",
    }
}

fn route_terminal_summary(
    stored: &StoredRouteOperation,
    outcome: TerminalOutcome,
    elapsed_ms: u64,
) -> RouteOperationSummary {
    RouteOperationSummary {
        catalog_revision: CATALOG_REVISION.to_owned(),
        resolver_revision: RESOLVER_REVISION.to_owned(),
        operation_id: stored.selection.operation_id.clone(),
        leader_id: stored.selection.leader_id.clone(),
        participant_ids: stored
            .participants
            .iter()
            .map(|participant| participant.account.clone())
            .collect(),
        route_id: stored.selection.route_id,
        seed: stored.selection.seed,
        event_id: stored.selection.event_id,
        effect: stored.selection.effect,
        objective_path: stored.selection.objective_path.clone(),
        duration_budget_ms: stored.selection.duration_budget_ms,
        elapsed_ms,
        outcome,
        reward: (outcome == TerminalOutcome::Succeeded).then_some(stored.selection.reward),
    }
}

fn prepare_route_terminal_replay(
    transaction: &mut Transaction<'_>,
    session_id: &str,
    stored: &StoredRouteOperation,
    outcome: TerminalOutcome,
    summary: &RouteOperationSummary,
    replay: Option<&RouteTerminalPayloadV1>,
) -> Result<Option<String>, RoutePersistenceError> {
    let Some(replay) = replay else {
        validate_route_replay_mode(transaction, session_id, false, "route_selected")?;
        return Ok(None);
    };
    let selection_replay = load_route_selection_replay(transaction, session_id, stored)?;
    if replay.summary != *summary {
        return Err(invalid_route_state(
            "terminal replay summary disagrees with durable terminal result",
        ));
    }
    if outcome == TerminalOutcome::FailedTimeout {
        for event_type in ["activity_completed", "loot_granted", "progression_granted"] {
            validate_route_replay_mode(transaction, session_id, false, event_type)?;
        }
    }
    let encoded = match outcome {
        TerminalOutcome::Succeeded => encode_route_operation_succeeded(replay, &selection_replay)?,
        TerminalOutcome::FailedTimeout => encode_route_operation_failed(replay, &selection_replay)?,
    };
    Ok(Some(encoded))
}

fn validate_route_terminal_replay_retry(
    transaction: &mut Transaction<'_>,
    session_id: &str,
    stored: &StoredRouteOperation,
    outcome: TerminalOutcome,
    expected_payload: Option<&str>,
) -> Result<(), RoutePersistenceError> {
    let event_type = terminal_replay_event_type(outcome);
    let rows = transaction.query(
        "SELECT account_id, activity_id, actor_id, payload FROM replay_events \
         WHERE session_id = $1 AND event_type = $2",
        &[&session_id, &event_type],
    )?;
    let Some(expected_payload) = expected_payload else {
        if rows.is_empty() {
            return Ok(());
        }
        return Err(invalid_route_state(
            "non-replay route terminal has replay evidence",
        ));
    };
    if rows.len() != 1 {
        return Err(invalid_route_state(
            "route terminal retry lacks exactly one replay event",
        ));
    }
    let leader = &stored.participants[0];
    let actor_id = rows[0]
        .get::<_, Option<i64>>(2)
        .and_then(|actor| u64::try_from(actor).ok());
    if rows[0].get::<_, String>(0) != leader.account
        || rows[0].get::<_, Option<String>>(1).as_deref() != Some(stored.activity_id.as_str())
        || actor_id != Some(leader.actor)
        || rows[0].get::<_, String>(3) != expected_payload
    {
        return Err(invalid_route_state(
            "route terminal retry disagrees with persisted replay evidence",
        ));
    }
    Ok(())
}

fn persist_route_terminal(
    transaction: &mut Transaction<'_>,
    session_id: &str,
    stored: &StoredRouteOperation,
    outcome: TerminalOutcome,
    elapsed_ms: u64,
    replay_payload: Option<&str>,
) -> Result<(), RoutePersistenceError> {
    let elapsed = i64::try_from(elapsed_ms)
        .map_err(|_| invalid_route_state("elapsed duration exceeds BIGINT"))?;
    let terminal = terminal_outcome_text(outcome);
    if transaction.execute(
        "UPDATE route_operations SET terminal_outcome = $2, elapsed_ms = $3, \
         terminal_at = NOW() WHERE session_id = $1 AND terminal_outcome IS NULL",
        &[&session_id, &terminal, &elapsed],
    )? != 1
    {
        return Err(invalid_route_state(
            "route terminal row changed while locked",
        ));
    }
    let Some(payload) = replay_payload else {
        return Ok(());
    };
    let actor_id = i64::try_from(stored.participants[0].actor)
        .map_err(|_| invalid_route_state("leader actor identifier exceeds BIGINT"))?;
    insert_replay_event(
        transaction,
        &NewReplayEvent {
            event_type: terminal_replay_event_type(outcome),
            session_id,
            account_id: &stored.participants[0].account,
            activity_id: Some(&stored.activity_id),
            actor_id: Some(actor_id),
            payload,
        },
    )?;
    Ok(())
}

fn resolve_route_selection(
    input: &RouteOperationSelection<'_>,
) -> Result<RouteSelectionOutcome, RoutePersistenceError> {
    let participant_ids = input
        .participants
        .iter()
        .map(|participant| participant.account_id)
        .collect::<Vec<_>>();
    let mut state = RouteOperationState::new(&participant_ids)?;
    for participant_id in &participant_ids {
        state.mark_capable(participant_id)?;
    }
    state.open_choice()?;
    state
        .select(
            input.requester_account_id,
            RouteSelectionRequest {
                operation_id: input.operation_id,
                route_id: input.route_id,
            },
            input.candidate_seed,
        )
        .map(|receipt| receipt.outcome)
        .map_err(Into::into)
}

fn validate_route_storage_id(value: &str, context: &str) -> Result<(), RoutePersistenceError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
    {
        return Err(invalid_route_state(format!(
            "{context} identifier must contain 1-64 bounded ASCII characters"
        )));
    }
    Ok(())
}

fn validate_activity_id(value: &str) -> Result<(), RoutePersistenceError> {
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(invalid_route_state(
            "activity identifier must match [a-z0-9_]{1,32}",
        ));
    }
    Ok(())
}

fn validate_route_participants(
    client: &mut impl GenericClient,
    participants: &[RouteParticipant<'_>],
) -> Result<Vec<StoredRouteParticipant>, RoutePersistenceError> {
    let mut accounts = BTreeSet::new();
    let mut characters = BTreeSet::new();
    let mut actors = BTreeSet::new();
    let mut stored = Vec::with_capacity(participants.len());
    for participant in participants {
        let actor_id = i64::try_from(participant.actor_id)
            .map_err(|_| invalid_route_state("participant actor identifier exceeds BIGINT"))?;
        if !accounts.insert(participant.account_id)
            || !characters.insert(participant.character_id)
            || !actors.insert(actor_id)
        {
            return Err(invalid_route_state(
                "route participants must have unique account, character, and actor identities",
            ));
        }
        let account_id = client
            .query_opt(
                "SELECT account_id FROM characters WHERE id = $1 FOR SHARE",
                &[&participant.character_id],
            )?
            .ok_or_else(|| invalid_route_state("route participant character was not found"))?
            .get::<_, String>(0);
        if account_id != participant.account_id {
            return Err(invalid_route_state(
                "route participant character does not belong to its account",
            ));
        }
        stored.push(StoredRouteParticipant {
            account: account_id,
            character: participant.character_id.to_owned(),
            actor: participant.actor_id,
        });
    }
    Ok(stored)
}

fn route_participants_match(
    stored: &[StoredRouteParticipant],
    supplied: &[RouteParticipant<'_>],
) -> bool {
    stored.len() == supplied.len()
        && stored.iter().zip(supplied).all(|(stored, supplied)| {
            stored.account == supplied.account_id
                && stored.character == supplied.character_id
                && stored.actor == supplied.actor_id
        })
}

fn insert_route_selection(
    transaction: &mut Transaction<'_>,
    session_id: &str,
    activity_id: &str,
    selection: &RouteSelectionOutcome,
    participants: &[StoredRouteParticipant],
) -> Result<(), RoutePersistenceError> {
    let participant_count = i16::try_from(participants.len())
        .map_err(|_| invalid_route_state("participant count exceeds SMALLINT"))?;
    let seed = i64::try_from(selection.seed.value())
        .map_err(|_| invalid_route_state("route seed exceeds BIGINT"))?;
    let health = i32::try_from(selection.effect.warden_health_basis_points)
        .map_err(|_| invalid_route_state("Warden health effect exceeds INTEGER"))?;
    let counter = i32::try_from(selection.effect.warden_counter_damage)
        .map_err(|_| invalid_route_state("Warden counter effect exceeds INTEGER"))?;
    let objective_path = serde_json::to_string(&selection.objective_path)?;
    let duration = i64::try_from(selection.duration_budget_ms)
        .map_err(|_| invalid_route_state("duration budget exceeds BIGINT"))?;
    let reward_quantity = i32::try_from(selection.reward.fragments)
        .map_err(|_| invalid_route_state("fragment reward exceeds INTEGER"))?;
    let experience = i64::try_from(selection.reward.experience)
        .map_err(|_| invalid_route_state("experience reward exceeds BIGINT"))?;
    transaction.execute(
        "INSERT INTO route_operations (\
             session_id, activity_id, operation_id, catalog_revision, resolver_revision, \
             leader_account_id, participant_count, route_id, seed, event_id, \
             warden_health_basis_points, warden_counter_damage, objective_path, \
             duration_budget_ms, reward_item_id, reward_quantity, experience\
         ) VALUES (\
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, \
             $13::TEXT::JSONB, $14, $15, $16, $17\
         )",
        &[
            &session_id,
            &activity_id,
            &selection.operation_id,
            &CATALOG_REVISION,
            &RESOLVER_REVISION,
            &selection.leader_id,
            &participant_count,
            &selection.route_id.as_str(),
            &seed,
            &selection.event_id.to_string(),
            &health,
            &counter,
            &objective_path,
            &duration,
            &RELAY_CORE_FRAGMENT,
            &reward_quantity,
            &experience,
        ],
    )?;
    for (index, participant) in participants.iter().enumerate() {
        let index = i16::try_from(index)
            .map_err(|_| invalid_route_state("participant index exceeds SMALLINT"))?;
        let actor_id = i64::try_from(participant.actor)
            .map_err(|_| invalid_route_state("participant actor identifier exceeds BIGINT"))?;
        transaction.execute(
            "INSERT INTO route_operation_participants \
             (session_id, participant_index, account_id, character_id, actor_id) \
             VALUES ($1, $2, $3, $4, $5)",
            &[
                &session_id,
                &index,
                &participant.account,
                &participant.character,
                &actor_id,
            ],
        )?;
    }
    Ok(())
}

fn load_route_operation(
    client: &mut impl GenericClient,
    session_id: &str,
    for_update: bool,
) -> Result<Option<StoredRouteOperation>, RoutePersistenceError> {
    let query = if for_update {
        "SELECT activity_id, operation_id, catalog_revision, resolver_revision, \
                leader_account_id, participant_count, route_id, seed, event_id, \
                warden_health_basis_points, warden_counter_damage, objective_path::TEXT, \
                duration_budget_ms, reward_item_id, reward_quantity, experience, \
                terminal_outcome, elapsed_ms \
         FROM route_operations WHERE session_id = $1 FOR UPDATE"
    } else {
        "SELECT activity_id, operation_id, catalog_revision, resolver_revision, \
                leader_account_id, participant_count, route_id, seed, event_id, \
                warden_health_basis_points, warden_counter_damage, objective_path::TEXT, \
                duration_budget_ms, reward_item_id, reward_quantity, experience, \
                terminal_outcome, elapsed_ms \
         FROM route_operations WHERE session_id = $1"
    };
    let Some(row) = client.query_opt(query, &[&session_id])? else {
        return Ok(None);
    };
    let leader_id = row.get::<_, String>(4);
    let participants = load_route_participants(client, session_id, row.get(5), &leader_id)?;
    let selection = parse_stored_route_selection(&row)?;
    let terminal = load_terminal_summary(row.get(16), row.get(17), &selection, &participants)?;
    Ok(Some(StoredRouteOperation {
        activity_id: row.get(0),
        selection,
        participants,
        terminal,
    }))
}

fn load_route_participants(
    client: &mut impl GenericClient,
    session_id: &str,
    participant_count: i16,
    leader_id: &str,
) -> Result<Vec<StoredRouteParticipant>, RoutePersistenceError> {
    let participant_rows = client.query(
        "SELECT account_id, character_id, actor_id FROM route_operation_participants \
         WHERE session_id = $1 ORDER BY participant_index",
        &[&session_id],
    )?;
    let participants = participant_rows
        .into_iter()
        .map(|row| {
            let actor_id = row.get::<_, i64>(2);
            Ok(StoredRouteParticipant {
                account: row.get(0),
                character: row.get(1),
                actor: u64::try_from(actor_id)
                    .map_err(|_| invalid_route_state("stored actor identifier is negative"))?,
            })
        })
        .collect::<Result<Vec<_>, RoutePersistenceError>>()?;
    let participant_count = usize::try_from(participant_count)
        .map_err(|_| invalid_route_state("stored participant count is negative"))?;
    if participant_count != participants.len() || participants.is_empty() {
        return Err(invalid_route_state(
            "stored participant rows do not match participant count",
        ));
    }
    if participants[0].account != leader_id {
        return Err(invalid_route_state(
            "stored leader is not the first admitted participant",
        ));
    }
    Ok(participants)
}

fn parse_stored_route_selection(row: &Row) -> Result<RouteSelectionOutcome, RoutePersistenceError> {
    let catalog_revision = row.get::<_, String>(2);
    let resolver_revision = row.get::<_, String>(3);
    if catalog_revision != CATALOG_REVISION || resolver_revision != RESOLVER_REVISION {
        return Err(invalid_route_state("stored route revision is unknown"));
    }
    let leader_id = row.get::<_, String>(4);
    let route_id = row
        .get::<_, String>(6)
        .parse::<RouteId>()
        .map_err(RoutePersistenceError::Domain)?;
    let seed_value = row.get::<_, i64>(7);
    let seed = RouteSeed::new(
        u64::try_from(seed_value).map_err(|_| invalid_route_state("stored seed is negative"))?,
    )?;
    let event_id = parse_route_event_id(&row.get::<_, String>(8))?;
    let health = u32::try_from(row.get::<_, i32>(9))
        .map_err(|_| invalid_route_state("stored Warden health effect is negative"))?;
    let counter = u32::try_from(row.get::<_, i32>(10))
        .map_err(|_| invalid_route_state("stored Warden counter effect is negative"))?;
    let effect = RouteEffect {
        warden_health_basis_points: health,
        warden_counter_damage: counter,
    };
    let objective_path = serde_json::from_str::<Vec<RouteObjectiveId>>(&row.get::<_, String>(11))?;
    let duration_budget_ms = u64::try_from(row.get::<_, i64>(12))
        .map_err(|_| invalid_route_state("stored duration is negative"))?;
    if row.get::<_, String>(13) != RELAY_CORE_FRAGMENT {
        return Err(invalid_route_state("stored reward item is unknown"));
    }
    let fragments = u32::try_from(row.get::<_, i32>(14))
        .map_err(|_| invalid_route_state("stored fragment reward is negative"))?;
    let experience = u64::try_from(row.get::<_, i64>(15))
        .map_err(|_| invalid_route_state("stored experience reward is negative"))?;
    let reward = RouteReward {
        fragments,
        experience,
    };
    let definition = route_definition(route_id);
    let resolved = resolve_event(route_id, seed);
    if event_id != resolved.event_id
        || effect != resolved.effect
        || reward != definition.reward
        || objective_path != definition.objective_path
        || duration_budget_ms != definition.duration_ms
    {
        return Err(invalid_route_state(
            "stored selection does not match the frozen route catalog",
        ));
    }
    Ok(RouteSelectionOutcome {
        operation_id: row.get(1),
        leader_id,
        route_id,
        seed,
        event_id,
        effect,
        reward,
        objective_path,
        duration_budget_ms,
    })
}

fn load_terminal_summary(
    terminal: Option<String>,
    elapsed: Option<i64>,
    selection: &RouteSelectionOutcome,
    participants: &[StoredRouteParticipant],
) -> Result<Option<RouteOperationSummary>, RoutePersistenceError> {
    let (terminal, elapsed) = match (terminal, elapsed) {
        (None, None) => return Ok(None),
        (Some(terminal), Some(elapsed)) => (terminal, elapsed),
        _ => return Err(invalid_route_state("stored terminal fields are partial")),
    };
    let outcome = parse_terminal_outcome(&terminal)?;
    let elapsed_ms = u64::try_from(elapsed)
        .map_err(|_| invalid_route_state("stored elapsed duration is negative"))?;
    validate_terminal_deadline(outcome, elapsed_ms, selection.duration_budget_ms)?;
    Ok(Some(RouteOperationSummary {
        catalog_revision: CATALOG_REVISION.to_owned(),
        resolver_revision: RESOLVER_REVISION.to_owned(),
        operation_id: selection.operation_id.clone(),
        leader_id: selection.leader_id.clone(),
        participant_ids: participants
            .iter()
            .map(|participant| participant.account.clone())
            .collect(),
        route_id: selection.route_id,
        seed: selection.seed,
        event_id: selection.event_id,
        effect: selection.effect,
        objective_path: selection.objective_path.clone(),
        duration_budget_ms: selection.duration_budget_ms,
        elapsed_ms,
        outcome,
        reward: (outcome == TerminalOutcome::Succeeded).then_some(selection.reward),
    }))
}

fn parse_route_event_id(value: &str) -> Result<RouteEventId, RoutePersistenceError> {
    RouteEventId::ALL
        .into_iter()
        .find(|event_id| event_id.to_string() == value)
        .ok_or_else(|| invalid_route_state("stored route event is unknown"))
}

fn parse_terminal_outcome(value: &str) -> Result<TerminalOutcome, RoutePersistenceError> {
    match value {
        "succeeded" => Ok(TerminalOutcome::Succeeded),
        "failed_timeout" => Ok(TerminalOutcome::FailedTimeout),
        _ => Err(invalid_route_state("stored terminal outcome is unknown")),
    }
}

const fn terminal_outcome_text(outcome: TerminalOutcome) -> &'static str {
    match outcome {
        TerminalOutcome::Succeeded => "succeeded",
        TerminalOutcome::FailedTimeout => "failed_timeout",
    }
}

fn validate_terminal_deadline(
    outcome: TerminalOutcome,
    elapsed_ms: u64,
    budget_ms: u64,
) -> Result<(), RoutePersistenceError> {
    match outcome {
        TerminalOutcome::Succeeded if elapsed_ms > budget_ms => Err(RouteError::DeadlineExceeded {
            elapsed_ms,
            budget_ms,
        }
        .into()),
        TerminalOutcome::FailedTimeout if elapsed_ms <= budget_ms => {
            Err(RouteError::DeadlineNotReached {
                elapsed_ms,
                budget_ms,
            }
            .into())
        }
        _ => Ok(()),
    }
}

fn apply_route_rewards(
    transaction: &mut Transaction<'_>,
    session_id: &str,
    activity_id: &str,
    participants: &[StoredRouteParticipant],
    reward: RouteReward,
    append_replay: bool,
) -> Result<(), RoutePersistenceError> {
    let item_quantity = i32::try_from(reward.fragments)
        .map_err(|_| invalid_route_state("route reward quantity exceeds INTEGER"))?;
    let experience_reward = ExperienceReward::validated(reward.experience)
        .map_err(|error| invalid_route_state(error.to_string()))?;
    if append_replay {
        for event_type in ["activity_completed", "loot_granted", "progression_granted"] {
            validate_route_replay_mode(transaction, session_id, false, event_type)?;
        }
        let leader = participants
            .first()
            .ok_or_else(|| invalid_route_state("route reward has no leader"))?;
        let actor_id = i64::try_from(leader.actor)
            .map_err(|_| invalid_route_state("leader actor identifier exceeds BIGINT"))?;
        insert_replay_event(
            transaction,
            &NewReplayEvent {
                event_type: "activity_completed",
                session_id,
                account_id: &leader.account,
                activity_id: Some(activity_id),
                actor_id: Some(actor_id),
                payload: "activity completed",
            },
        )?;
    }
    for participant in participants {
        let applied = apply_completion_reward(
            transaction,
            &ActivityCompletion {
                session_id,
                account_id: &participant.account,
                character_id: &participant.character,
                activity_id,
                item_id: RELAY_CORE_FRAGMENT,
                item_quantity,
                experience_reward,
            },
        )?;
        let applied = applied.ok_or_else(|| {
            invalid_route_state("route reward grant already exists without a terminal summary")
        })?;
        if append_replay {
            let actor_id = i64::try_from(participant.actor)
                .map_err(|_| invalid_route_state("participant actor identifier exceeds BIGINT"))?;
            let loot_payload = format!(
                "loot granted: {RELAY_CORE_FRAGMENT} x{} (total {})",
                reward.fragments, applied.item_quantity
            );
            let progression_payload = format!(
                "progression granted: +{} XP (total {}, level {} -> {})",
                applied.experience_granted,
                applied.experience,
                applied.previous_level,
                applied.level
            );
            insert_replay_event(
                transaction,
                &NewReplayEvent {
                    event_type: "loot_granted",
                    session_id,
                    account_id: &participant.account,
                    activity_id: Some(activity_id),
                    actor_id: Some(actor_id),
                    payload: &loot_payload,
                },
            )?;
            insert_replay_event(
                transaction,
                &NewReplayEvent {
                    event_type: "progression_granted",
                    session_id,
                    account_id: &participant.account,
                    activity_id: Some(activity_id),
                    actor_id: Some(actor_id),
                    payload: &progression_payload,
                },
            )?;
        }
    }
    Ok(())
}

fn invalid_route_state(message: impl Into<String>) -> RoutePersistenceError {
    RoutePersistenceError::InvalidPersistedState(message.into())
}

fn apply_completion_reward(
    transaction: &mut Transaction<'_>,
    completion: &ActivityCompletion<'_>,
) -> Result<Option<CompletionRewards>, postgres::Error> {
    let inserted = transaction.query_opt(
        "INSERT INTO inventory_reward_grants (session_id, character_id, item_id, quantity) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING quantity",
        &[
            &completion.session_id,
            &completion.character_id,
            &completion.item_id,
            &completion.item_quantity,
        ],
    )?;
    if inserted.is_none() {
        return Ok(None);
    }
    let resulting_quantity: i32 = transaction
        .query_one(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, $2, $3) \
             ON CONFLICT (character_id, item_id) DO UPDATE \
             SET quantity = inventory.quantity + EXCLUDED.quantity RETURNING quantity",
            &[
                &completion.character_id,
                &completion.item_id,
                &completion.item_quantity,
            ],
        )?
        .get(0);
    let progression_row = transaction.query_one(
        "SELECT level, experience FROM progression WHERE character_id = $1 FOR UPDATE",
        &[&completion.character_id],
    )?;
    let previous_level: i32 = progression_row.get(0);
    let previous_experience: i64 = progression_row.get(1);
    let current = DomainProgression::from_experience(previous_experience.unsigned_abs());
    let resulting = current.grant(completion.experience_reward);
    let experience = i64::try_from(resulting.experience).unwrap_or(i64::MAX);
    let level = i32::try_from(resulting.level).unwrap_or(i32::MAX);
    let experience_granted =
        i64::try_from(completion.experience_reward.experience()).unwrap_or(i64::MAX);
    transaction.execute(
        "INSERT INTO progression_reward_grants (session_id, character_id, experience) \
         VALUES ($1, $2, $3)",
        &[
            &completion.session_id,
            &completion.character_id,
            &experience_granted,
        ],
    )?;
    transaction.execute(
        "UPDATE progression SET level = $2, experience = $3 WHERE character_id = $1",
        &[&completion.character_id, &level, &experience],
    )?;
    transaction.execute(
        "UPDATE characters SET level = $2 WHERE id = $1",
        &[&completion.character_id, &level],
    )?;
    transaction.execute(
        "INSERT INTO activity_history (account_id, character_id, activity_id) VALUES ($1, $2, $3)",
        &[
            &completion.account_id,
            &completion.character_id,
            &completion.activity_id,
        ],
    )?;
    Ok(Some(CompletionRewards {
        item_quantity: resulting_quantity,
        experience_granted,
        previous_level,
        level,
        experience,
    }))
}

#[cfg(test)]
mod tests {
    use super::SCHEMA;

    #[test]
    fn schema_covers_persisted_state_and_replay() {
        for table in [
            "accounts",
            "characters",
            "inventory",
            "progression",
            "activity_history",
            "replay_events",
            "inventory_reward_grants",
            "progression_reward_grants",
            "equipment_loadouts",
            "module_states",
            "module_loadout_slots",
            "module_operations",
            "route_operations",
            "route_operation_participants",
            "cooperation_operations",
            "cooperation_operation_participants",
        ] {
            assert!(SCHEMA.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")));
        }
    }
}
