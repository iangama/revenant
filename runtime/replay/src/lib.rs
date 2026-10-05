use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use revenant_inventory::{ARC_SIDEARM_PROFILE, COIL_LANCE_PROFILE, PULSE_RIFLE_PROFILE};
use revenant_modules::{
    module_definition, resolve_build, validate_operation_id, AggregateModifiers, BaseCombatProfile,
    EffectiveCombatProfile, ModuleDefinition, ModuleId, ModuleState, ARSENAL_CATALOG_REVISION,
    CATALOG_REVISION, CONTENT_CATALOG_REVISION, CONTENT_MODULE_CATALOG, MAX_LOADOUT_REVISION,
    MODULE_CATALOG,
};
use revenant_modules::{BUILD_CATALOG_REVISION, BUILD_MODULE_CATALOG};
use serde::{Deserialize, Serialize};

mod acquisition;
mod bulwark;
mod campaign;
mod campaign_story;
mod challenge_authority;
mod challenge_elite;
mod challenge_gauntlet;
mod challenge_mastery;
pub use challenge_mastery::challenge_mastery_attempt;
mod challenge_route;
pub use challenge_route::RouteEvidence;
mod challenge_prism;
mod challenge_recovery;
mod challenge_signal;
mod challenge_survival;
pub use challenge_gauntlet::{GauntletAction, GauntletEffect, GauntletEvidence};
pub use challenge_recovery::RecoveryEvidence;
pub use challenge_signal::SignalEvidence;
pub use challenge_survival::SurvivalEvidence;
mod challenges;
pub use acquisition::{
    acquisition_milestones, decode_acquisition_claimed, encode_acquisition_claimed,
    AcquisitionClaimedPayloadV1, AcquisitionMilestoneProof,
};
pub use campaign::{
    decode_campaign_payload, decode_campaign_resume, encode_campaign_payload,
    encode_campaign_resume, verify_campaign_milestone, CampaignReplayPayload,
    CampaignResumePayload, ReconstructedCampaign,
};
pub use campaign_story::CampaignStoryProof;
pub use challenge_authority::{ChallengeVisitProof, ReconstructedChallenge};
pub use challenges::{
    decode_challenge_payload, encode_challenge_payload, reconstruct_challenge_journal,
    ChallengeReplayPayload,
};
mod cooperation;
mod elite;
mod lancer;
mod prism;
mod route;
mod support;

pub use bulwark::{encode_bulwark_evidence, BulwarkEvidence};
pub use elite::{encode_elite_evidence, EliteEvidence, EliteStep, EliteTransition};
pub use lancer::{encode_lancer_evidence, LancerEvidence};
pub use prism::{
    encode_prism_evidence, PrismEvidence, PrismPatternEvidence, PrismStep, PrismTransition,
};
pub use support::{encode_support_evidence, SupportEvidence, SupportTransition};

pub use cooperation::{
    decode_cooperation_payload, encode_cooperation_failed, encode_cooperation_pinged,
    encode_cooperation_started, encode_cooperation_succeeded, encode_player_downed,
    encode_player_revived, CooperationPingedPayloadV1, CooperationReplayPrefix,
    CooperationStartedPayloadV1, CooperationTerminalPayloadV1, DecodedCooperationReplayPayload,
    PlayerDownedPayloadV1, PlayerRevivedPayloadV1, ReconstructedCooperationReplay,
    ReconstructedCooperationState, ReplayCooperationContributionEvidence,
    ReplayCooperationGrantEvidence, ReplayCooperationHazard, ReplayCooperationParticipantEvidence,
    ReplayCooperationRewardEvidence, ReplayCooperationTargetEvidence,
    ReplayCooperationTerminalParticipantEvidence, ReplayCooperationTimingEvidence,
    COOPERATION_REPLAY_SCHEMA_VERSION, MAX_COOPERATION_REPLAY_PAYLOAD_BYTES,
};

pub use route::{
    decode_route_payload, encode_route_operation_failed, encode_route_operation_succeeded,
    encode_route_selected, DecodedRouteReplayPayload, ReconstructedRouteReplay,
    ReconstructedRouteState, ReplayObjectiveTransition, ReplayRouteEncounterEvidence,
    ReplayRouteGrantEvidence, ReplayRouteObjectiveEvidence, ReplayRouteObjectiveKind,
    ReplayRouteObjectiveState, ReplayRouteParticipantEvidence, RouteSelectedPayloadV1,
    RouteTerminalPayloadV1, AUTHORING_REVISION, MAX_ROUTE_REPLAY_PAYLOAD_BYTES,
    ROUTE_REPLAY_SCHEMA_VERSION,
};

pub const MODULE_REPLAY_SCHEMA_VERSION: u8 = 1;
pub const MAX_MODULE_REPLAY_PAYLOAD_BYTES: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayEventKind {
    PlayerJoined,
    ActivityStarted,
    EnemySpawned,
    EnemyDied,
    BossSpawned,
    ActivityCompleted,
    LootGranted,
    ProgressionGranted,
    EquipmentChanged,
    FieldActivity,
    ModuleStateSnapshot,
    ModuleCombined,
    ModuleLoadoutChanged,
    AcquisitionClaimed,
    CampaignCheckpoint,
    CampaignResumed,
    CampaignObjective,
    CampaignStory,
    ChallengeCheckpoint,
    ChallengeObjective,
    RouteSelected,
    RouteOperationSucceeded,
    RouteOperationFailed,
    CooperationStarted,
    CooperationPinged,
    PlayerDowned,
    PlayerRevived,
    CooperationSucceeded,
    CooperationFailed,
}

impl ReplayEventKind {
    #[must_use]
    pub const fn is_module_event(self) -> bool {
        matches!(
            self,
            Self::ModuleStateSnapshot | Self::ModuleCombined | Self::ModuleLoadoutChanged
        )
    }

    #[must_use]
    pub const fn is_route_event(self) -> bool {
        matches!(
            self,
            Self::RouteSelected | Self::RouteOperationSucceeded | Self::RouteOperationFailed
        )
    }

    #[must_use]
    pub const fn is_cooperation_event(self) -> bool {
        matches!(
            self,
            Self::CooperationStarted
                | Self::CooperationPinged
                | Self::PlayerDowned
                | Self::PlayerRevived
                | Self::CooperationSucceeded
                | Self::CooperationFailed
        )
    }
}

impl Display for ReplayEventKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PlayerJoined => "player_joined",
            Self::ActivityStarted => "activity_started",
            Self::EnemySpawned => "enemy_spawned",
            Self::EnemyDied => "enemy_died",
            Self::BossSpawned => "boss_spawned",
            Self::ActivityCompleted => "activity_completed",
            Self::LootGranted => "loot_granted",
            Self::ProgressionGranted => "progression_granted",
            Self::EquipmentChanged => "equipment_changed",
            Self::FieldActivity => "field_activity",
            Self::ModuleStateSnapshot => "module_state_snapshot",
            Self::ModuleCombined => "module_combined",
            Self::ModuleLoadoutChanged => "module_loadout_changed",
            Self::AcquisitionClaimed => "acquisition_claimed",
            Self::CampaignCheckpoint => "campaign_checkpoint",
            Self::CampaignResumed => "campaign_resumed",
            Self::CampaignObjective => "campaign_objective",
            Self::CampaignStory => "campaign_story",
            Self::ChallengeCheckpoint => "challenge_checkpoint",
            Self::ChallengeObjective => "challenge_objective",
            Self::RouteSelected => "route_selected",
            Self::RouteOperationSucceeded => "route_operation_succeeded",
            Self::RouteOperationFailed => "route_operation_failed",
            Self::CooperationStarted => "cooperation_started",
            Self::CooperationPinged => "cooperation_pinged",
            Self::PlayerDowned => "player_downed",
            Self::PlayerRevived => "player_revived",
            Self::CooperationSucceeded => "cooperation_succeeded",
            Self::CooperationFailed => "cooperation_failed",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReplayEventKind(pub String);

impl Display for UnknownReplayEventKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown replay event kind: {}", self.0)
    }
}

impl Error for UnknownReplayEventKind {}

impl FromStr for ReplayEventKind {
    type Err = UnknownReplayEventKind;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "player_joined" => Ok(Self::PlayerJoined),
            "activity_started" => Ok(Self::ActivityStarted),
            "enemy_spawned" => Ok(Self::EnemySpawned),
            "enemy_died" => Ok(Self::EnemyDied),
            "boss_spawned" => Ok(Self::BossSpawned),
            "activity_completed" => Ok(Self::ActivityCompleted),
            "loot_granted" => Ok(Self::LootGranted),
            "progression_granted" => Ok(Self::ProgressionGranted),
            "equipment_changed" => Ok(Self::EquipmentChanged),
            "field_activity" => Ok(Self::FieldActivity),
            "module_state_snapshot" => Ok(Self::ModuleStateSnapshot),
            "module_combined" => Ok(Self::ModuleCombined),
            "module_loadout_changed" => Ok(Self::ModuleLoadoutChanged),
            "acquisition_claimed" => Ok(Self::AcquisitionClaimed),
            "campaign_checkpoint" => Ok(Self::CampaignCheckpoint),
            "campaign_resumed" => Ok(Self::CampaignResumed),
            "campaign_objective" => Ok(Self::CampaignObjective),
            "campaign_story" => Ok(Self::CampaignStory),
            "challenge_checkpoint" => Ok(Self::ChallengeCheckpoint),
            "challenge_objective" => Ok(Self::ChallengeObjective),
            "route_selected" => Ok(Self::RouteSelected),
            "route_operation_succeeded" => Ok(Self::RouteOperationSucceeded),
            "route_operation_failed" => Ok(Self::RouteOperationFailed),
            "cooperation_started" => Ok(Self::CooperationStarted),
            "cooperation_pinged" => Ok(Self::CooperationPinged),
            "player_downed" => Ok(Self::PlayerDowned),
            "player_revived" => Ok(Self::PlayerRevived),
            "cooperation_succeeded" => Ok(Self::CooperationSucceeded),
            "cooperation_failed" => Ok(Self::CooperationFailed),
            _ => Err(UnknownReplayEventKind(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEvent {
    pub id: i64,
    pub kind: ReplayEventKind,
    pub timestamp: String,
    pub session_id: String,
    pub account_id: String,
    pub activity_id: Option<String>,
    pub actor_id: Option<u64>,
    pub payload: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayProtocolGeneration {
    V1,
    V2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayWeaponEvidence {
    pub item_id: String,
    pub base: BaseCombatProfile,
    pub effective: EffectiveCombatProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayModuleStateEvidence {
    pub catalog_revision: String,
    pub catalog: Vec<ModuleDefinition>,
    pub fragments: u32,
    pub owned_modules: Vec<ModuleId>,
    pub loadout_revision: u64,
    pub persisted_loadout: Vec<ModuleId>,
    pub applied_loadout: Vec<ModuleId>,
    pub module_effects_active: bool,
    pub modifiers: AggregateModifiers,
    pub weapons: Vec<ReplayWeaponEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleStateSnapshotPayloadV1 {
    pub schema_version: u8,
    pub character_id: String,
    pub protocol_generation: ReplayProtocolGeneration,
    pub state: ReplayModuleStateEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleCombinedPayloadV1 {
    pub schema_version: u8,
    pub character_id: String,
    pub operation_id: String,
    pub module_id: ModuleId,
    pub recipe_fragments: u32,
    pub before: ReplayModuleStateEvidence,
    pub after: ReplayModuleStateEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleLoadoutChangedPayloadV1 {
    pub schema_version: u8,
    pub character_id: String,
    pub operation_id: String,
    pub before: ReplayModuleStateEvidence,
    pub after: ReplayModuleStateEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event_type", content = "payload", rename_all = "snake_case")]
pub enum DecodedModuleReplayPayload {
    ModuleStateSnapshot(ModuleStateSnapshotPayloadV1),
    ModuleCombined(ModuleCombinedPayloadV1),
    ModuleLoadoutChanged(ModuleLoadoutChangedPayloadV1),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedModuleParticipant {
    pub character_id: String,
    pub actor_id: u64,
    pub protocol_generation: ReplayProtocolGeneration,
    pub state: ReplayModuleStateEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedSession {
    pub session_id: String,
    pub activity_id: Option<String>,
    pub player_actor_id: Option<u64>,
    pub spawned_enemies: usize,
    pub defeated_enemies: usize,
    pub boss_spawned: bool,
    pub completed: bool,
    pub loot_grants: usize,
    pub progression_grants: usize,
    pub equipment_changes: usize,
    pub module_replay_legacy: bool,
    pub module_snapshots: usize,
    pub module_combinations: usize,
    pub module_loadout_changes: usize,
    pub module_participants: Vec<ReconstructedModuleParticipant>,
    pub acquisition_claims: Vec<AcquisitionClaimedPayloadV1>,
    pub campaign: Vec<ReconstructedCampaign>,
    pub challenge: Option<ReconstructedChallenge>,
    pub route: ReconstructedRouteReplay,
    pub cooperation: ReconstructedCooperationReplay,
    pub field_objectives: BTreeMap<String, String>,
    pub timeline: Vec<String>,
}

/// Builds complete deterministic evidence for one persisted module state.
///
/// # Errors
///
/// Returns an error when ownership/loadout state or checked combat arithmetic
/// violates the fixed M26 contract.
pub fn module_state_evidence(
    fragments: u32,
    owned_modules: &[ModuleId],
    persisted_loadout: &[ModuleId],
    loadout_revision: u64,
    protocol_generation: ReplayProtocolGeneration,
) -> Result<ReplayModuleStateEvidence, ReplayError> {
    module_state_evidence_for_catalog(
        if owned_modules
            .iter()
            .any(|id| !ModuleId::CONTENT.contains(id))
        {
            BUILD_CATALOG_REVISION
        } else if protocol_generation == ReplayProtocolGeneration::V2
            || owned_modules
                .iter()
                .any(|id| !ModuleId::LEGACY.contains(id))
        {
            CONTENT_CATALOG_REVISION
        } else {
            CATALOG_REVISION
        },
        fragments,
        owned_modules,
        persisted_loadout,
        loadout_revision,
        protocol_generation,
    )
}

/// Resolves the catalog recorded by an event, retaining historical arithmetic.
///
/// # Errors
///
/// Returns an error for unsupported revisions or invalid module evidence.
pub fn module_state_evidence_for_catalog(
    catalog_revision: &str,
    fragments: u32,
    owned_modules: &[ModuleId],
    persisted_loadout: &[ModuleId],
    loadout_revision: u64,
    protocol_generation: ReplayProtocolGeneration,
) -> Result<ReplayModuleStateEvidence, ReplayError> {
    if loadout_revision > MAX_LOADOUT_REVISION {
        return Err(invalid_module(
            "loadout revision exceeds the signed persistence bound",
        ));
    }
    if catalog_revision == CATALOG_REVISION
        && owned_modules
            .iter()
            .any(|id| !ModuleId::LEGACY.contains(id))
    {
        return Err(invalid_module("legacy ownership contains a content module"));
    }
    if catalog_revision != BUILD_CATALOG_REVISION
        && owned_modules
            .iter()
            .any(|id| !ModuleId::CONTENT.contains(id))
    {
        return Err(invalid_module("ownership requires the build catalog"));
    }
    let domain = ModuleState::new(
        fragments,
        owned_modules,
        persisted_loadout,
        loadout_revision,
    )
    .map_err(|error| invalid_module(error.to_string()))?;
    let owned_modules = domain.owned_modules();
    let persisted_loadout = domain.loadout().to_vec();
    let module_effects_active = protocol_generation == ReplayProtocolGeneration::V2;
    let applied_loadout = if module_effects_active {
        persisted_loadout.clone()
    } else {
        Vec::new()
    };
    let mut base_profiles = vec![PULSE_RIFLE_PROFILE, ARC_SIDEARM_PROFILE];
    if [
        CONTENT_CATALOG_REVISION,
        ARSENAL_CATALOG_REVISION,
        BUILD_CATALOG_REVISION,
    ]
    .contains(&catalog_revision)
    {
        base_profiles.push(COIL_LANCE_PROFILE);
    }
    if [ARSENAL_CATALOG_REVISION, BUILD_CATALOG_REVISION].contains(&catalog_revision) {
        base_profiles.extend([
            revenant_inventory::SCATTER_CASTER_PROFILE,
            revenant_inventory::RAIL_DRIVER_PROFILE,
        ]);
    }
    let mut modifiers = None;
    let mut weapons = Vec::with_capacity(base_profiles.len());
    for weapon in base_profiles {
        let item_id = weapon.item_id;
        let base = BaseCombatProfile {
            damage: weapon.damage,
            range: weapon.range,
            cooldown_ms: weapon.cooldown_ms,
            max_health: 100,
        };
        let resolved = resolve_build(catalog_revision, base, &applied_loadout)
            .map_err(|error| invalid_module(error.to_string()))?;
        if modifiers
            .replace(resolved.modifiers)
            .is_some_and(|prior| prior != resolved.modifiers)
        {
            return Err(invalid_module(
                "weapon builds resolved different aggregate modifiers",
            ));
        }
        weapons.push(ReplayWeaponEvidence {
            item_id: item_id.to_owned(),
            base,
            effective: resolved.profile,
        });
    }
    Ok(ReplayModuleStateEvidence {
        catalog_revision: catalog_revision.to_owned(),
        catalog: if catalog_revision == BUILD_CATALOG_REVISION {
            BUILD_MODULE_CATALOG.to_vec()
        } else if [CONTENT_CATALOG_REVISION, ARSENAL_CATALOG_REVISION].contains(&catalog_revision) {
            CONTENT_MODULE_CATALOG.to_vec()
        } else {
            MODULE_CATALOG.to_vec()
        },
        fragments,
        owned_modules,
        loadout_revision,
        persisted_loadout,
        applied_loadout,
        module_effects_active,
        modifiers: modifiers.unwrap_or_default(),
        weapons,
    })
}

/// Serializes one bounded module snapshot payload.
///
/// # Errors
///
/// Returns an error for JSON serialization failure or a payload over 8,192 bytes.
pub fn encode_module_state_snapshot(
    payload: &ModuleStateSnapshotPayloadV1,
) -> Result<String, ReplayError> {
    encode_module_payload(payload)
}

/// Serializes one bounded module-combination payload.
///
/// # Errors
///
/// Returns an error for JSON serialization failure or a payload over 8,192 bytes.
pub fn encode_module_combined(payload: &ModuleCombinedPayloadV1) -> Result<String, ReplayError> {
    encode_module_payload(payload)
}

/// Serializes one bounded module-loadout payload.
///
/// # Errors
///
/// Returns an error for JSON serialization failure or a payload over 8,192 bytes.
pub fn encode_module_loadout_changed(
    payload: &ModuleLoadoutChangedPayloadV1,
) -> Result<String, ReplayError> {
    encode_module_payload(payload)
}

/// Decodes one structured module event for reconstruction or read-only display.
///
/// # Errors
///
/// Returns an error for an oversized, malformed, wrong-version, or invalid
/// module payload. Non-module kinds return `Ok(None)`.
pub fn decode_module_payload(
    kind: ReplayEventKind,
    payload: &str,
) -> Result<Option<DecodedModuleReplayPayload>, ReplayError> {
    if !kind.is_module_event() {
        return Ok(None);
    }
    validate_payload_size(payload)?;
    let decoded = match kind {
        ReplayEventKind::ModuleStateSnapshot => {
            let value: ModuleStateSnapshotPayloadV1 = decode_json(payload)?;
            validate_snapshot_payload(&value)?;
            DecodedModuleReplayPayload::ModuleStateSnapshot(value)
        }
        ReplayEventKind::ModuleCombined => {
            let value: ModuleCombinedPayloadV1 = decode_json(payload)?;
            validate_combination_payload(&value)?;
            DecodedModuleReplayPayload::ModuleCombined(value)
        }
        ReplayEventKind::ModuleLoadoutChanged => {
            let value: ModuleLoadoutChangedPayloadV1 = decode_json(payload)?;
            validate_loadout_payload(&value)?;
            DecodedModuleReplayPayload::ModuleLoadoutChanged(value)
        }
        _ => return Ok(None),
    };
    Ok(Some(decoded))
}

fn validate_field_activity_events(
    events: &[ReplayEvent],
) -> Result<BTreeMap<String, String>, ReplayError> {
    let mut phase = 0;
    let resumed = campaign::resume_snapshot(events)?;
    let mut owner = resumed
        .as_ref()
        .map(|(event, _)| (event.account_id.as_str(), event.actor_id));
    let mut meridian = resumed
        .as_ref()
        .map(|(_, payload)| campaign::restored_meridian(payload))
        .transpose()?
        .flatten();
    let mut field_objectives = BTreeMap::new();
    for event in events.iter().filter(|event| {
        event.kind == ReplayEventKind::FieldActivity
            && !event.payload.starts_with(lancer::PREFIX)
            && !event.payload.starts_with(bulwark::PREFIX)
            && !event.payload.starts_with(support::PREFIX)
            && !event.payload.starts_with(challenge_recovery::PREFIX)
            && !event.payload.starts_with(challenge_signal::PREFIX)
            && !event.payload.starts_with(challenge_survival::PREFIX)
            && !event.payload.starts_with(challenge_gauntlet::PREFIX)
            && !event.payload.starts_with(challenge_route::PREFIX)
            && !event.payload.starts_with(elite::PREFIX)
            && !event.payload.starts_with(prism::PREFIX)
    }) {
        let identity = (event.account_id.as_str(), event.actor_id);
        if event.actor_id.is_none() || owner.is_some_and(|prior| prior != identity) {
            return Err(ReplayError::InvalidFieldActivityEvidence);
        }
        owner = Some(identity);
        if event.payload.starts_with("meridian-v1:") {
            if phase != 0
                || events.iter().any(|other| {
                    other.id < event.id
                        && matches!(
                            other.kind,
                            ReplayEventKind::BossSpawned | ReplayEventKind::ActivityCompleted
                        )
                })
            {
                return Err(ReplayError::InvalidFieldActivityEvidence);
            }
            if event.payload == "meridian-v1:started" && meridian.is_none() {
                meridian = Some(
                    revenant_activities::meridian::Expedition::start()
                        .map_err(|_| ReplayError::InvalidFieldActivityEvidence)?,
                );
                continue;
            }
            let id = event.payload.trim_start_matches("meridian-v1:");
            let station = revenant_activities::meridian::sector()
                .stations
                .iter()
                .find(|station| station.id == id)
                .ok_or(ReplayError::InvalidFieldActivityEvidence)?;
            let run = meridian
                .as_mut()
                .ok_or(ReplayError::InvalidFieldActivityEvidence)?;
            let (payload, updates) = run
                .visit(station.position)
                .ok_or(ReplayError::InvalidFieldActivityEvidence)?;
            if payload != event.payload {
                return Err(ReplayError::InvalidFieldActivityEvidence);
            }
            for update in updates {
                if let revenant_activities::ActivityEvent::ObjectiveUpdated(objective) = update {
                    field_objectives.insert(objective.id, format!("{:?}", objective.state));
                }
            }
            continue;
        }
        if meridian.is_some() {
            return Err(ReplayError::InvalidFieldActivityEvidence);
        }
        phase = match (phase, event.payload.as_str()) {
            (0 | 3, "coolant_run:started") => 1,
            (1, "coolant_run:transfer") => 2,
            (1 | 2, "coolant_run:expired") => 3,
            (2, "coolant_run:completed") => 4,
            _ => return Err(ReplayError::InvalidFieldActivityEvidence),
        };
    }
    if owner.is_some()
        && events
            .iter()
            .any(|event| event.kind.is_route_event() || event.kind.is_cooperation_event())
    {
        return Err(ReplayError::InvalidFieldActivityEvidence);
    }
    if let Some(run) = meridian {
        for update in run.objectives() {
            if let revenant_activities::ActivityEvent::ObjectiveUpdated(objective) = update {
                field_objectives.insert(objective.id, format!("{:?}", objective.state));
            }
        }
    }
    Ok(field_objectives)
}

/// Reconstructs the observable state and textual timeline of one session.
///
/// # Errors
///
/// Returns an error if events are empty, mix sessions, are not in append order,
/// or contain incomplete/inconsistent structured module evidence.
#[allow(clippy::too_many_lines)] // Keep the event reducer and cross-domain validation together.
pub fn reconstruct(events: &[ReplayEvent]) -> Result<ReconstructedSession, ReplayError> {
    let first = events.first().ok_or(ReplayError::EmptySession)?;
    validate_event_order(events, &first.session_id)?;
    let mut state = initial_session(first, events.len());
    state.field_objectives = validate_field_activity_events(events)?;
    if let Some(status) = lancer::validate(events)? {
        state
            .field_objectives
            .insert("glass_lancer".to_owned(), status.to_owned());
    }
    if let Some(status) = bulwark::validate(events)? {
        state
            .field_objectives
            .insert("steel_bulwark".to_owned(), status.to_owned());
    }
    if let Some(status) = support::validate(events)? {
        state
            .field_objectives
            .insert("relay_mender".to_owned(), status.to_owned());
    }
    if let Some(status) = prism::validate(events)? {
        state
            .field_objectives
            .insert("prism_warden".to_owned(), status.to_owned());
    }
    if let Some((composition, status)) = elite::validate(events)? {
        state
            .field_objectives
            .insert(composition.id().to_owned(), status.to_owned());
    }
    let mut participants = BTreeMap::<String, ReconstructedModuleParticipant>::new();
    let mut saw_module_event = false;
    for (index, event) in events.iter().enumerate() {
        if event.activity_id.is_some() {
            state.activity_id.clone_from(&event.activity_id);
        }
        match event.kind {
            ReplayEventKind::PlayerJoined => state.player_actor_id = event.actor_id,
            ReplayEventKind::EnemySpawned => {
                state.spawned_enemies += if event.payload.starts_with(support::PREFIX)
                    || event.payload.starts_with(elite::PREFIX)
                {
                    2
                } else {
                    1
                };
            }
            ReplayEventKind::EnemyDied => state.defeated_enemies += 1,
            ReplayEventKind::BossSpawned => state.boss_spawned = true,
            ReplayEventKind::ActivityCompleted => state.completed = true,
            ReplayEventKind::LootGranted => {
                state.loot_grants += 1;
                apply_fragment_reward(event, &mut participants)?;
            }
            ReplayEventKind::ProgressionGranted => state.progression_grants += 1,
            ReplayEventKind::EquipmentChanged => state.equipment_changes += 1,
            ReplayEventKind::ModuleStateSnapshot => {
                saw_module_event = true;
                apply_snapshot(events, index, event, &mut state, &mut participants)?;
            }
            ReplayEventKind::ModuleCombined => {
                saw_module_event = true;
                apply_combination(event, state.completed, &mut state, &mut participants)?;
            }
            ReplayEventKind::ModuleLoadoutChanged => {
                saw_module_event = true;
                apply_loadout(event, state.completed, &mut state, &mut participants)?;
            }
            ReplayEventKind::AcquisitionClaimed => {
                acquisition::apply_claim(event, &mut state, &mut participants)?;
            }
            ReplayEventKind::ActivityStarted
            | ReplayEventKind::CampaignCheckpoint
            | ReplayEventKind::CampaignResumed
            | ReplayEventKind::CampaignObjective
            | ReplayEventKind::CampaignStory
            | ReplayEventKind::ChallengeCheckpoint
            | ReplayEventKind::ChallengeObjective
            | ReplayEventKind::FieldActivity
            | ReplayEventKind::RouteSelected
            | ReplayEventKind::RouteOperationSucceeded
            | ReplayEventKind::RouteOperationFailed
            | ReplayEventKind::CooperationStarted
            | ReplayEventKind::CooperationPinged
            | ReplayEventKind::PlayerDowned
            | ReplayEventKind::PlayerRevived
            | ReplayEventKind::CooperationSucceeded
            | ReplayEventKind::CooperationFailed => {}
        }
        state.timeline.push(format!(
            "{} | {} | {}",
            event.timestamp, event.kind, event.payload
        ));
    }
    if saw_module_event {
        validate_snapshot_adjacency(events)?;
        state.module_replay_legacy = false;
    }
    state.module_participants = participants.into_values().collect();
    state.campaign = campaign::reconstruct(events, &state.module_participants)?;
    state.challenge = challenge_authority::reconstruct(events)?;
    state.route = route::reconstruct_route_replay(events)?;
    state.cooperation = cooperation::reconstruct_cooperation_replay(events)?;
    Ok(state)
}

fn initial_session(first: &ReplayEvent, event_count: usize) -> ReconstructedSession {
    ReconstructedSession {
        session_id: first.session_id.clone(),
        activity_id: None,
        player_actor_id: None,
        spawned_enemies: 0,
        defeated_enemies: 0,
        boss_spawned: false,
        completed: false,
        loot_grants: 0,
        progression_grants: 0,
        equipment_changes: 0,
        module_replay_legacy: true,
        module_snapshots: 0,
        module_combinations: 0,
        module_loadout_changes: 0,
        module_participants: Vec::new(),
        acquisition_claims: Vec::new(),
        campaign: Vec::new(),
        challenge: None,
        route: ReconstructedRouteReplay::default(),
        cooperation: ReconstructedCooperationReplay::default(),
        field_objectives: BTreeMap::new(),
        timeline: Vec::with_capacity(event_count),
    }
}

fn validate_event_order(events: &[ReplayEvent], session_id: &str) -> Result<(), ReplayError> {
    let mut prior_id = None;
    for event in events {
        if event.session_id != session_id {
            return Err(ReplayError::MixedSessions);
        }
        if prior_id.is_some_and(|prior| event.id <= prior) {
            return Err(ReplayError::NonIncreasingAppendId);
        }
        prior_id = Some(event.id);
    }
    Ok(())
}

fn apply_snapshot(
    events: &[ReplayEvent],
    index: usize,
    event: &ReplayEvent,
    session: &mut ReconstructedSession,
    participants: &mut BTreeMap<String, ReconstructedModuleParticipant>,
) -> Result<(), ReplayError> {
    let Some(DecodedModuleReplayPayload::ModuleStateSnapshot(payload)) =
        decode_module_payload(event.kind, &event.payload)?
    else {
        return Err(invalid_module("snapshot event decoded as another payload"));
    };
    let prior = index
        .checked_sub(1)
        .and_then(|prior_index| events.get(prior_index))
        .ok_or_else(|| invalid_module("module snapshot has no preceding player join"))?;
    if prior.kind != ReplayEventKind::PlayerJoined || prior.actor_id != event.actor_id {
        return Err(invalid_module(
            "module snapshot is not adjacent to its matching player join",
        ));
    }
    let actor_id = event
        .actor_id
        .ok_or_else(|| invalid_module("module snapshot has no actor"))?;
    if participants.contains_key(&payload.character_id)
        || participants
            .values()
            .any(|participant| participant.actor_id == actor_id)
    {
        return Err(invalid_module("participant module snapshot is duplicated"));
    }
    participants.insert(
        payload.character_id.clone(),
        ReconstructedModuleParticipant {
            character_id: payload.character_id,
            actor_id,
            protocol_generation: payload.protocol_generation,
            state: payload.state,
        },
    );
    session.module_snapshots += 1;
    Ok(())
}

fn apply_combination(
    event: &ReplayEvent,
    completed: bool,
    session: &mut ReconstructedSession,
    participants: &mut BTreeMap<String, ReconstructedModuleParticipant>,
) -> Result<(), ReplayError> {
    if !completed {
        return Err(invalid_module(
            "module combination precedes activity completion",
        ));
    }
    let Some(DecodedModuleReplayPayload::ModuleCombined(payload)) =
        decode_module_payload(event.kind, &event.payload)?
    else {
        return Err(invalid_module(
            "combination event decoded as another payload",
        ));
    };
    let participant = matching_participant(event, &payload.character_id, participants)?;
    if participant.state != payload.before {
        return Err(invalid_module(
            "module combination before-state does not match reconstructed state",
        ));
    }
    participant.state = payload.after;
    session.module_combinations += 1;
    Ok(())
}

fn apply_loadout(
    event: &ReplayEvent,
    completed: bool,
    session: &mut ReconstructedSession,
    participants: &mut BTreeMap<String, ReconstructedModuleParticipant>,
) -> Result<(), ReplayError> {
    if !completed {
        return Err(invalid_module(
            "module loadout change precedes activity completion",
        ));
    }
    let Some(DecodedModuleReplayPayload::ModuleLoadoutChanged(payload)) =
        decode_module_payload(event.kind, &event.payload)?
    else {
        return Err(invalid_module("loadout event decoded as another payload"));
    };
    let participant = matching_participant(event, &payload.character_id, participants)?;
    if participant.state != payload.before {
        return Err(invalid_module(
            "module loadout before-state does not match reconstructed state",
        ));
    }
    participant.state = payload.after;
    session.module_loadout_changes += 1;
    Ok(())
}

fn matching_participant<'a>(
    event: &ReplayEvent,
    character_id: &str,
    participants: &'a mut BTreeMap<String, ReconstructedModuleParticipant>,
) -> Result<&'a mut ReconstructedModuleParticipant, ReplayError> {
    let participant = participants
        .get_mut(character_id)
        .ok_or_else(|| invalid_module("module mutation has no participant snapshot"))?;
    if event.actor_id != Some(participant.actor_id) {
        return Err(invalid_module(
            "module mutation actor disagrees with snapshot",
        ));
    }
    if participant.protocol_generation != ReplayProtocolGeneration::V2 {
        return Err(invalid_module(
            "frozen V1 participant cannot mutate modules",
        ));
    }
    Ok(participant)
}

fn apply_fragment_reward(
    event: &ReplayEvent,
    participants: &mut BTreeMap<String, ReconstructedModuleParticipant>,
) -> Result<(), ReplayError> {
    let Some(total) = relay_fragment_total(&event.payload) else {
        return Ok(());
    };
    let Some(actor_id) = event.actor_id else {
        return Err(invalid_module("fragment reward has no participant actor"));
    };
    if let Some(participant) = participants
        .values_mut()
        .find(|participant| participant.actor_id == actor_id)
    {
        if total < participant.state.fragments {
            return Err(invalid_module(
                "fragment reward reduces reconstructed balance",
            ));
        }
        participant.state.fragments = total;
    }
    Ok(())
}

fn relay_fragment_total(payload: &str) -> Option<u32> {
    let suffix = payload.strip_prefix("loot granted: relay_core_fragment x")?;
    let (_, total) = suffix.rsplit_once(" (total ")?;
    total.strip_suffix(')')?.parse().ok()
}

fn validate_snapshot_adjacency(events: &[ReplayEvent]) -> Result<(), ReplayError> {
    for (index, event) in events.iter().enumerate() {
        if event.kind == ReplayEventKind::PlayerJoined {
            let next = events
                .get(index + 1)
                .ok_or_else(|| invalid_module("M26 player join is missing module snapshot"))?;
            if next.kind != ReplayEventKind::ModuleStateSnapshot || next.actor_id != event.actor_id
            {
                return Err(invalid_module(
                    "M26 player join is missing adjacent module snapshot",
                ));
            }
        }
    }
    Ok(())
}

fn validate_snapshot_payload(payload: &ModuleStateSnapshotPayloadV1) -> Result<(), ReplayError> {
    validate_schema(payload.schema_version)?;
    if payload.character_id.is_empty() || payload.character_id.len() > 128 {
        return Err(invalid_module(
            "snapshot character identifier is outside bounds",
        ));
    }
    let expected_active = payload.protocol_generation == ReplayProtocolGeneration::V2;
    if payload.state.module_effects_active != expected_active {
        return Err(invalid_module(
            "protocol generation and module activation disagree",
        ));
    }
    validate_module_state(&payload.state)
}

fn validate_combination_payload(payload: &ModuleCombinedPayloadV1) -> Result<(), ReplayError> {
    validate_schema(payload.schema_version)?;
    validate_mutation_identity(&payload.character_id, &payload.operation_id)?;
    validate_module_state(&payload.before)?;
    validate_module_state(&payload.after)?;
    if !payload.before.module_effects_active || !payload.after.module_effects_active {
        return Err(invalid_module(
            "module mutation evidence must use active V2 arithmetic",
        ));
    }
    let definition = module_definition(payload.module_id);
    if payload.recipe_fragments != definition.recipe_fragments
        || payload.before.fragments < payload.recipe_fragments
    {
        return Err(invalid_module(
            "module combination recipe evidence is impossible",
        ));
    }
    let mut owned = payload.before.owned_modules.clone();
    if owned.contains(&payload.module_id) {
        return Err(invalid_module(
            "module combination starts with an owned module",
        ));
    }
    owned.push(payload.module_id);
    let expected = module_state_evidence_for_catalog(
        &payload.before.catalog_revision,
        payload.before.fragments - payload.recipe_fragments,
        &owned,
        &payload.before.persisted_loadout,
        payload.before.loadout_revision,
        ReplayProtocolGeneration::V2,
    )?;
    if payload.after != expected {
        return Err(invalid_module(
            "module combination after-state is impossible",
        ));
    }
    Ok(())
}

fn validate_loadout_payload(payload: &ModuleLoadoutChangedPayloadV1) -> Result<(), ReplayError> {
    validate_schema(payload.schema_version)?;
    validate_mutation_identity(&payload.character_id, &payload.operation_id)?;
    validate_module_state(&payload.before)?;
    validate_module_state(&payload.after)?;
    if !payload.before.module_effects_active || !payload.after.module_effects_active {
        return Err(invalid_module(
            "module mutation evidence must use active V2 arithmetic",
        ));
    }
    let resulting_revision = payload
        .before
        .loadout_revision
        .checked_add(1)
        .ok_or_else(|| invalid_module("module loadout revision overflowed"))?;
    if payload.after.persisted_loadout == payload.before.persisted_loadout {
        return Err(invalid_module("module loadout event records no change"));
    }
    let expected = module_state_evidence_for_catalog(
        &payload.before.catalog_revision,
        payload.before.fragments,
        &payload.before.owned_modules,
        &payload.after.persisted_loadout,
        resulting_revision,
        ReplayProtocolGeneration::V2,
    )?;
    if payload.after != expected {
        return Err(invalid_module("module loadout after-state is impossible"));
    }
    Ok(())
}

fn validate_module_state(state: &ReplayModuleStateEvidence) -> Result<(), ReplayError> {
    let generation = if state.module_effects_active {
        ReplayProtocolGeneration::V2
    } else {
        ReplayProtocolGeneration::V1
    };
    let expected = module_state_evidence_for_catalog(
        &state.catalog_revision,
        state.fragments,
        &state.owned_modules,
        &state.persisted_loadout,
        state.loadout_revision,
        generation,
    )?;
    if *state != expected {
        return Err(invalid_module(
            "module state catalog, order, activation, modifiers, or arithmetic disagrees",
        ));
    }
    Ok(())
}

fn validate_mutation_identity(character_id: &str, operation_id: &str) -> Result<(), ReplayError> {
    if character_id.is_empty() || character_id.len() > 128 {
        return Err(invalid_module(
            "mutation character identifier is outside bounds",
        ));
    }
    validate_operation_id(operation_id).map_err(|error| invalid_module(error.to_string()))
}

fn validate_schema(schema_version: u8) -> Result<(), ReplayError> {
    if schema_version != MODULE_REPLAY_SCHEMA_VERSION {
        return Err(invalid_module(
            "unknown module replay payload schema version",
        ));
    }
    Ok(())
}

fn encode_module_payload(payload: &impl Serialize) -> Result<String, ReplayError> {
    let encoded = serde_json::to_string(payload).map_err(|error| {
        invalid_module(format!("module replay JSON serialization failed: {error}"))
    })?;
    validate_payload_size(&encoded)?;
    Ok(encoded)
}

fn decode_json<'a, T: Deserialize<'a>>(payload: &'a str) -> Result<T, ReplayError> {
    serde_json::from_str(payload)
        .map_err(|error| invalid_module(format!("invalid module replay JSON: {error}")))
}

fn validate_payload_size(payload: &str) -> Result<(), ReplayError> {
    if payload.len() > MAX_MODULE_REPLAY_PAYLOAD_BYTES {
        return Err(invalid_module(
            "module replay payload exceeds 8192 UTF-8 bytes",
        ));
    }
    Ok(())
}

fn invalid_module(message: impl Into<String>) -> ReplayError {
    ReplayError::InvalidModuleEvidence(message.into())
}

fn route_error(message: impl Into<String>) -> ReplayError {
    ReplayError::InvalidRouteEvidence(message.into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    EmptySession,
    MixedSessions,
    NonIncreasingAppendId,
    InvalidModuleEvidence(String),
    InvalidRouteEvidence(String),
    InvalidCooperationEvidence(String),
    InvalidFieldActivityEvidence,
}

impl Display for ReplayError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFieldActivityEvidence => {
                formatter.write_str("invalid field activity order or identity")
            }
            Self::EmptySession => formatter.write_str("session has no replay events"),
            Self::MixedSessions => {
                formatter.write_str("replay events belong to different sessions")
            }
            Self::NonIncreasingAppendId => {
                formatter.write_str("replay append identifiers are not strictly increasing")
            }
            Self::InvalidModuleEvidence(message) => {
                write!(formatter, "invalid module replay evidence: {message}")
            }
            Self::InvalidRouteEvidence(message) => {
                write!(formatter, "invalid route replay evidence: {message}")
            }
            Self::InvalidCooperationEvidence(message) => {
                write!(formatter, "invalid cooperation replay evidence: {message}")
            }
        }
    }
}

impl Error for ReplayError {}

#[cfg(test)]
mod tests {
    #[test]
    fn meridian_trace_reconstructs_independent_paths_and_rejects_corruption() {
        let payloads = [
            "started",
            "meridian_arrival",
            "meridian_log",
            "meridian_return",
            "meridian_lens",
            "meridian_gallery",
            "meridian_memory",
        ];
        let events: Vec<_> = payloads
            .iter()
            .enumerate()
            .map(|(index, payload)| {
                let mut entry = event(
                    i64::try_from(index).unwrap() + 1,
                    ReplayEventKind::FieldActivity,
                );
                entry.payload = format!("meridian-v1:{payload}");
                entry
            })
            .collect();
        for end in 1..=events.len() {
            let result = reconstruct(&events[..end]).unwrap();
            assert!(!result.completed);
            assert_eq!(result.loot_grants, 0);
            assert_eq!(result.progression_grants, 0);
            assert_eq!(
                result.field_objectives["meridian_gallery"],
                if end >= 6 {
                    "Completed"
                } else if end == 5 {
                    "Active"
                } else {
                    "Pending"
                }
            );
        }
        for index in 0..events.len() {
            let mut invalid = events.clone();
            invalid[index].payload = "meridian-v1:meridian_gallery".to_owned();
            if index != 5 {
                assert!(reconstruct(&invalid).is_err());
            }
        }
        let mut duplicate = events.clone();
        duplicate.last_mut().unwrap().payload = "meridian-v1:meridian_log".to_owned();
        assert!(reconstruct(&duplicate).is_err());
        let mut mixed = events.clone();
        mixed.last_mut().unwrap().payload = "coolant_run:started".to_owned();
        assert!(reconstruct(&mixed).is_err());
        let mut owner = events;
        owner.last_mut().unwrap().actor_id = Some(999);
        assert!(reconstruct(&owner).is_err());
    }

    #[test]
    fn field_trace_retains_retry_order_without_fabricating_mission_rewards() {
        let mut events: Vec<_> = ["started", "expired", "started", "transfer", "completed"]
            .iter()
            .enumerate()
            .map(|(index, phase)| {
                let mut entry = event(
                    i64::try_from(index).unwrap() + 1,
                    ReplayEventKind::FieldActivity,
                );
                entry.payload = format!("coolant_run:{phase}");
                entry
            })
            .collect();
        let result = reconstruct(&events).unwrap();
        assert!(!result.completed);
        assert_eq!(result.loot_grants, 0);
        assert_eq!(result.progression_grants, 0);
        events[3].payload = "coolant_run:completed".to_owned();
        assert!(reconstruct(&events).is_err());
    }

    #[test]
    fn historical_catalogs_reconstruct_without_inventing_the_lance() {
        for revision in [super::CATALOG_REVISION, super::CONTENT_CATALOG_REVISION] {
            let state = super::module_state_evidence_for_catalog(
                revision,
                4,
                &[super::ModuleId::ForceMatrix],
                &[super::ModuleId::ForceMatrix],
                1,
                super::ReplayProtocolGeneration::V2,
            )
            .unwrap();
            super::validate_module_state(&state).unwrap();
            assert_eq!(
                state.weapons.len(),
                if revision == super::CATALOG_REVISION {
                    2
                } else {
                    3
                }
            );
            let mut corrupted = state;
            corrupted.catalog_revision = "unknown-revision".to_owned();
            assert!(super::validate_module_state(&corrupted).is_err());
        }
    }

    use super::{
        encode_module_combined, encode_module_loadout_changed, encode_module_state_snapshot,
        module_state_evidence, reconstruct, ModuleCombinedPayloadV1, ModuleLoadoutChangedPayloadV1,
        ModuleStateSnapshotPayloadV1, ReconstructedCooperationState, ReconstructedRouteState,
        ReplayEvent, ReplayEventKind, ReplayProtocolGeneration, MODULE_REPLAY_SCHEMA_VERSION,
    };
    use revenant_modules::ModuleId;

    fn event(id: i64, kind: ReplayEventKind) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            timestamp: format!("2026-01-01 00:00:{id:02}+00"),
            session_id: "session-1".to_owned(),
            account_id: "account-1".to_owned(),
            activity_id: Some("relay_awakening".to_owned()),
            actor_id: Some(10),
            payload: kind.to_string().replace('_', " "),
        }
    }

    fn snapshot_payload(
        generation: ReplayProtocolGeneration,
        fragments: u32,
        owned: &[ModuleId],
        loadout: &[ModuleId],
        revision: u64,
    ) -> ModuleStateSnapshotPayloadV1 {
        ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: "character-1".to_owned(),
            protocol_generation: generation,
            state: module_state_evidence(fragments, owned, loadout, revision, generation)
                .expect("test state should resolve"),
        }
    }

    #[test]
    fn reconstructs_legacy_completed_session_and_timeline() {
        let kinds = [
            ReplayEventKind::PlayerJoined,
            ReplayEventKind::ActivityStarted,
            ReplayEventKind::EnemySpawned,
            ReplayEventKind::EnemyDied,
            ReplayEventKind::BossSpawned,
            ReplayEventKind::EnemyDied,
            ReplayEventKind::ActivityCompleted,
            ReplayEventKind::LootGranted,
            ReplayEventKind::ProgressionGranted,
            ReplayEventKind::EquipmentChanged,
        ];
        let events = kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| event(i64::try_from(index + 1).expect("small index"), kind))
            .collect::<Vec<_>>();
        let state = reconstruct(&events).expect("valid legacy session should reconstruct");
        assert_eq!(state.spawned_enemies, 1);
        assert_eq!(state.defeated_enemies, 2);
        assert!(state.boss_spawned);
        assert!(state.completed);
        assert_eq!(state.loot_grants, 1);
        assert_eq!(state.progression_grants, 1);
        assert_eq!(state.equipment_changes, 1);
        assert!(state.module_replay_legacy);
        assert!(state.module_participants.is_empty());
        assert_eq!(state.route.state, ReconstructedRouteState::Legacy);
        assert!(state.route.selection.is_none());
        assert!(state.route.terminal.is_none());
        assert_eq!(
            state.cooperation.state,
            ReconstructedCooperationState::Legacy
        );
        assert!(state.cooperation.started.is_none());
        assert!(state.cooperation.terminal.is_none());
        assert_eq!(state.timeline.len(), events.len());
    }

    #[test]
    fn reconstructs_v2_snapshot_combination_and_loadout_exactly() {
        let snapshot = snapshot_payload(ReplayProtocolGeneration::V2, 4, &[], &[], 0);
        let combined_after = module_state_evidence(
            2,
            &[ModuleId::ForceMatrix],
            &[],
            0,
            ReplayProtocolGeneration::V2,
        )
        .expect("combination result should resolve");
        let combined = ModuleCombinedPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: "character-1".to_owned(),
            operation_id: "combine-1".to_owned(),
            module_id: ModuleId::ForceMatrix,
            recipe_fragments: 2,
            before: snapshot.state.clone(),
            after: combined_after.clone(),
        };
        let loadout_after = module_state_evidence(
            2,
            &[ModuleId::ForceMatrix],
            &[ModuleId::ForceMatrix],
            1,
            ReplayProtocolGeneration::V2,
        )
        .expect("loadout result should resolve");
        let loadout = ModuleLoadoutChangedPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: "character-1".to_owned(),
            operation_id: "loadout-1".to_owned(),
            before: combined_after,
            after: loadout_after.clone(),
        };
        let mut events = vec![
            event(1, ReplayEventKind::PlayerJoined),
            event(2, ReplayEventKind::ModuleStateSnapshot),
            event(3, ReplayEventKind::ActivityStarted),
            event(4, ReplayEventKind::ActivityCompleted),
            event(5, ReplayEventKind::ModuleCombined),
            event(6, ReplayEventKind::ModuleLoadoutChanged),
        ];
        events[1].payload =
            encode_module_state_snapshot(&snapshot).expect("snapshot should encode");
        events[4].payload = encode_module_combined(&combined).expect("combination should encode");
        events[5].payload = encode_module_loadout_changed(&loadout).expect("loadout should encode");
        let reconstructed = reconstruct(&events).expect("valid M26 replay should reconstruct");
        assert!(!reconstructed.module_replay_legacy);
        assert_eq!(reconstructed.module_snapshots, 1);
        assert_eq!(reconstructed.module_combinations, 1);
        assert_eq!(reconstructed.module_loadout_changes, 1);
        assert_eq!(reconstructed.module_participants[0].state, loadout_after);
    }

    #[test]
    fn frozen_v1_snapshot_records_persisted_but_applies_empty_loadout() {
        let snapshot = snapshot_payload(
            ReplayProtocolGeneration::V1,
            3,
            &[ModuleId::ForceMatrix],
            &[ModuleId::ForceMatrix],
            7,
        );
        assert_eq!(snapshot.state.persisted_loadout, [ModuleId::ForceMatrix]);
        assert!(snapshot.state.applied_loadout.is_empty());
        assert!(!snapshot.state.module_effects_active);
        let weapon = &snapshot.state.weapons[0];
        assert_eq!(weapon.base.damage, weapon.effective.damage);
        assert_eq!(weapon.base.range, weapon.effective.range);
        assert_eq!(weapon.base.cooldown_ms, weapon.effective.cooldown_ms);
        assert_eq!(weapon.base.max_health, weapon.effective.max_health);
        let mut events = vec![
            event(1, ReplayEventKind::PlayerJoined),
            event(2, ReplayEventKind::ModuleStateSnapshot),
        ];
        events[1].payload =
            encode_module_state_snapshot(&snapshot).expect("snapshot should encode");
        let reconstructed = reconstruct(&events).expect("valid V1 evidence should reconstruct");
        assert_eq!(
            reconstructed.module_participants[0].protocol_generation,
            ReplayProtocolGeneration::V1
        );
    }

    #[test]
    fn rejects_missing_snapshot_order_and_non_increasing_append_id() {
        let snapshot = snapshot_payload(ReplayProtocolGeneration::V2, 0, &[], &[], 0);
        let mut events = vec![
            event(1, ReplayEventKind::PlayerJoined),
            event(2, ReplayEventKind::ActivityStarted),
            event(3, ReplayEventKind::ModuleStateSnapshot),
        ];
        events[2].payload =
            encode_module_state_snapshot(&snapshot).expect("snapshot should encode");
        assert!(reconstruct(&events).is_err());
        events[1].id = 1;
        assert!(reconstruct(&events).is_err());
    }

    #[test]
    fn rejects_arithmetic_recipe_revision_and_json_corruption() {
        let snapshot = snapshot_payload(ReplayProtocolGeneration::V2, 4, &[], &[], 0);
        let mut after = module_state_evidence(
            2,
            &[ModuleId::ForceMatrix],
            &[],
            0,
            ReplayProtocolGeneration::V2,
        )
        .expect("test state should resolve");
        after.weapons[0].effective.damage += 1;
        let corrupt = ModuleCombinedPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: "character-1".to_owned(),
            operation_id: "combine-1".to_owned(),
            module_id: ModuleId::ForceMatrix,
            recipe_fragments: 2,
            before: snapshot.state.clone(),
            after,
        };
        let mut events = vec![
            event(1, ReplayEventKind::PlayerJoined),
            event(2, ReplayEventKind::ModuleStateSnapshot),
            event(3, ReplayEventKind::ActivityCompleted),
            event(4, ReplayEventKind::ModuleCombined),
        ];
        events[1].payload =
            encode_module_state_snapshot(&snapshot).expect("snapshot should encode");
        events[3].payload = encode_module_combined(&corrupt).expect("JSON should encode");
        assert!(reconstruct(&events).is_err());

        events[3].payload = events[3].payload.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"extra\":true",
            1,
        );
        assert!(reconstruct(&events).is_err());
    }

    #[test]
    fn applies_recorded_fragment_reward_before_combination() {
        let snapshot = snapshot_payload(ReplayProtocolGeneration::V2, 1, &[], &[], 0);
        let rewarded = module_state_evidence(2, &[], &[], 0, ReplayProtocolGeneration::V2)
            .expect("rewarded state should resolve");
        let after = module_state_evidence(
            0,
            &[ModuleId::TempoRegulator],
            &[],
            0,
            ReplayProtocolGeneration::V2,
        )
        .expect("combined state should resolve");
        let combined = ModuleCombinedPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: "character-1".to_owned(),
            operation_id: "combine-reward".to_owned(),
            module_id: ModuleId::TempoRegulator,
            recipe_fragments: 2,
            before: rewarded,
            after,
        };
        let mut events = vec![
            event(1, ReplayEventKind::PlayerJoined),
            event(2, ReplayEventKind::ModuleStateSnapshot),
            event(3, ReplayEventKind::ActivityCompleted),
            event(4, ReplayEventKind::LootGranted),
            event(5, ReplayEventKind::ModuleCombined),
        ];
        events[1].payload =
            encode_module_state_snapshot(&snapshot).expect("snapshot should encode");
        events[3].payload = "loot granted: relay_core_fragment x1 (total 2)".to_owned();
        events[4].payload = encode_module_combined(&combined).expect("combination should encode");
        let reconstructed = reconstruct(&events).expect("reward transition should reconstruct");
        assert_eq!(reconstructed.module_participants[0].state.fragments, 0);
    }
}
