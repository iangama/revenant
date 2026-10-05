use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read, Write};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

mod challenges;
pub use challenges::*;

pub const PROTOCOL_VERSION: u16 = 2;
pub const CONTENT_REVISION: &str = "m31-v1";
pub const EXPLORATION_CONTENT_REVISION: &str = "m33-v1";
pub const ENCOUNTER_CONTENT_REVISION: &str = "m34-v1";
pub const DEFENSE_CONTENT_REVISION: &str = "m34-v2";
pub const SUPPORT_CONTENT_REVISION: &str = "m34-v3";
pub const ELITE_CONTENT_REVISION: &str = "m34-v4";
pub const PRISM_CONTENT_REVISION: &str = "m34-v5";
pub const ARSENAL_CONTENT_REVISION: &str = "m35-v1";
pub const BUILD_CONTENT_REVISION: &str = "m35-v2";
pub const ACQUISITION_CONTENT_REVISION: &str = "m35-v3";
pub const CAMPAIGN_CONTENT_REVISION: &str = "m36-v2";
pub const CHALLENGE_CONTENT_REVISION: &str = "m37-v1";
pub const RECOVERY_CONTENT_REVISION: &str = "m37-v2";
pub const ELITE_CHALLENGE_CONTENT_REVISION: &str = "m37-v3";
pub const SURVIVAL_CHALLENGE_CONTENT_REVISION: &str = "m37-v6";
pub const MODIFIER_CHALLENGE_CONTENT_REVISION: &str = "m37-v8";
pub const MASTERY_CHALLENGE_CONTENT_REVISION: &str = "m37-v9";
pub const GAUNTLET_CHALLENGE_CONTENT_REVISION: &str = "m37-v7";
pub const SIGNAL_CHALLENGE_CONTENT_REVISION: &str = "m37-v5";
pub const PRISM_CHALLENGE_CONTENT_REVISION: &str = "m37-v4";
pub const MAX_FRAME_SIZE: usize = 64 * 1024;
pub const MAX_MODULE_OPERATION_ID_BYTES: usize = 32;
pub const MAX_MODULE_ID_BYTES: usize = 32;
pub const MAX_MODULE_LOADOUT_ENTRIES: usize = 3;
pub const ROUTE_WIRE_SCHEMA_VERSION: u16 = 1;
pub const MAX_ROUTE_OPERATION_ID_BYTES: usize = 32;
pub const MAX_ROUTE_ID_BYTES: usize = 32;
pub const MAX_ROUTE_IDENTITY_BYTES: usize = 64;
pub const MAX_ROUTE_REVISION_BYTES: usize = 32;
pub const MAX_ROUTE_MESSAGE_BYTES: usize = 128;
pub const MAX_ROUTE_PARTICIPANTS: usize = 2;
pub const MAX_ROUTE_OPTIONS: usize = 2;
pub const MAX_ROUTE_EVENT_CANDIDATES: usize = 2;
pub const MAX_ROUTE_OBJECTIVES: usize = 4;
pub const MAX_ROUTE_TRANSITIONS: usize = 8;
pub const MAX_ROUTE_GRANTS: usize = 2;
pub const MAX_ROUTE_WIRE_PAYLOAD_SIZE: usize = 8 * 1024;
pub const COOPERATION_WIRE_SCHEMA_VERSION: u16 = 1;
pub const MAX_COOPERATION_OPERATION_ID_BYTES: usize = 32;
pub const MAX_COOPERATION_MESSAGE_BYTES: usize = 128;
pub const MAX_COOPERATION_PARTICIPANTS: usize = 2;
pub const MAX_COOPERATION_TARGETS: usize = 2;
pub const MAX_COOPERATION_GRANTS: usize = 2;
pub const MAX_COOPERATION_WIRE_PAYLOAD_SIZE: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    ClientHello(ClientHello),
    AuthRequest(AuthRequest),
    CharacterListRequest(CharacterListRequest),
    WorldJoinRequest(WorldJoinRequest),
    CampaignStateRequest(CampaignStateRequest),
    CampaignJoinRequest(CampaignJoinRequest),
    CampaignStoryIntent(CampaignStoryIntent),
    ChallengeStateRequest(ChallengeStateRequest),
    ChallengeJoinRequest(ChallengeJoinRequest),
    ChallengeAbandonIntent(ChallengeAbandonIntent),
    AttackIntent(AttackIntent),
    MoveIntent(MoveIntent),
    EquipIntent(EquipIntent),
    ModuleStateRequest(ModuleStateRequest),
    ModulePreviewRequest(ModulePreviewRequest),
    ModuleCombineIntent(ModuleCombineIntent),
    ModuleLoadoutIntent(ModuleLoadoutIntent),
    AcquisitionStateRequest(AcquisitionStateRequest),
    AcquisitionClaimIntent(AcquisitionClaimIntent),
    RouteStateRequest(RouteStateRequest),
    RouteChoiceIntent(RouteChoiceIntent),
    CooperationStateRequest(CooperationStateRequest),
    CooperationStartIntent(CooperationStartIntent),
    CooperationPingIntent(CooperationPingIntent),
    CooperationReviveIntent(CooperationReviveIntent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    ServerHello(ServerHello),
    AuthResponse(AuthResponse),
    CharacterListResponse(CharacterListResponse),
    WorldJoinResponse(WorldJoinResponse),
    CampaignSnapshot(CampaignSnapshot),
    CampaignStoryResult(CampaignStoryResult),
    ChallengeSnapshot(ChallengeSnapshot),
    ChallengeActionResult(ChallengeActionResult),
    ChallengeSurvivalState(ChallengeSurvivalState),
    ChallengeGauntletState(ChallengeGauntletState),
    InventorySnapshot(InventorySnapshot),
    ProgressionSnapshot(ProgressionSnapshot),
    EquipmentSnapshot(EquipmentSnapshot),
    ActorSpawn(ActorSpawn),
    ActorUpdate(ActorUpdate),
    ActorDestroy(ActorDestroy),
    DamageApplied(DamageApplied),
    RepairApplied(RepairApplied),
    PrismState(PrismState),
    ActivityStart(ActivityStart),
    ObjectiveUpdate(ObjectiveUpdate),
    DoorState(DoorState),
    ActivityComplete(ActivityComplete),
    LootGranted(LootGranted),
    ProgressionGranted(ProgressionGranted),
    EquipmentChanged(EquipmentChanged),
    ModuleSnapshot(ModuleSnapshot),
    ModulePreview(ModulePreview),
    ModuleCombined(ModuleCombined),
    ModuleLoadoutChanged(ModuleLoadoutChanged),
    AcquisitionSnapshot(AcquisitionSnapshot),
    AcquisitionClaimed(AcquisitionClaimed),
    RouteState(RouteState),
    RouteChoiceResult(RouteChoiceResult),
    RouteOperationSummary(RouteOperationSummary),
    CooperationState(CooperationState),
    CooperationStartResult(CooperationStartResult),
    CooperationPingResult(CooperationPingResult),
    CooperationLifeState(CooperationLifeState),
    CooperationReviveResult(CooperationReviveResult),
    CooperationOperationSummary(CooperationOperationSummary),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub protocol_version: u16,
    pub client_name: String,
    pub client_build: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerHello {
    pub protocol_version: u16,
    pub server_name: String,
    pub accepted: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub authenticated: bool,
    pub account_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterListRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterListResponse {
    pub characters: Vec<CharacterSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub character_id: String,
    pub display_name: String,
    pub class_name: String,
    pub level: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldJoinRequest {
    pub character_id: String,
}

/// Campaign selection happens after character listing and before world entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignStateRequest {
    pub character_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignEntryMode {
    Progress,
    Practice,
    Resume,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJoinRequest {
    pub character_id: String,
    pub chapter_id: String,
    pub mode: CampaignEntryMode,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignRun {
    pub run_id: String,
    pub chapter_id: String,
    pub practice: bool,
    pub checkpoint: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignSnapshot {
    pub revision: String,
    pub state_revision: u64,
    pub cleared_chapters: u8,
    pub available_chapters: u8,
    pub total_chapters: u8,
    pub active: Option<CampaignRun>,
    pub first_clear_fragments: u32,
    pub first_clear_experience: u64,
    pub story: CampaignStory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignStory {
    pub revision: String,
    pub empty_seat: String,
    pub held_connection: String,
    pub supply: Option<String>,
    pub core: Option<String>,
    pub epilogue: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStoryAction {
    EmptySeatMemory,
    EmptySeatReturned,
    HeldConnectionNote,
    HeldConnectionReturned,
    SupplyCovered,
    SupplyService,
    CoreGrounded,
    CoreDirect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignStoryIntent {
    pub operation_id: String,
    pub run_id: String,
    pub expected_revision: u64,
    pub action: CampaignStoryAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignStoryResult {
    pub operation_id: String,
    /// accepted / rejected / unconfirmed (storage acknowledgement failed).
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldJoinResponse {
    pub accepted: bool,
    pub world_id: String,
    pub player_actor_id: u64,
    pub spawn_position: [i32; 3],
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySnapshot {
    pub items: Vec<InventoryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionSnapshot {
    pub level: u32,
    pub experience: u64,
    pub experience_to_next_level: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentSnapshot {
    pub equipped_weapon_item_id: String,
    pub weapons: Vec<WeaponProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponProfile {
    pub item_id: String,
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSpawn {
    pub actor_id: u64,
    pub actor_kind: String,
    pub archetype: String,
    pub position: [i32; 3],
    pub health: u32,
    pub max_health: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorUpdate {
    pub actor_id: u64,
    pub position: [i32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<ChargeTelegraph>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defense: Option<DefenseTelegraph>,
}

/// The authoritative shield direction and announced close-range strike.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefenseTelegraph {
    pub facing: [i32; 2],
    pub braced: bool,
    pub warning_ms: u32,
}

/// Optional M34 presentation of a server-owned, locked charge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeTelegraph {
    pub target: [i32; 3],
    pub winding_up: bool,
    pub warning_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorDestroy {
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackIntent {
    pub target_actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipIntent {
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageApplied {
    pub source_actor_id: u64,
    pub target_actor_id: u64,
    pub damage: u32,
    pub remaining_health: u32,
    pub killed: bool,
}

/// Confirmed optional core-boss presentation, only in m34-v5 sessions.
/// The client keeps this state until a later confirmation, even after `interval_ms`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrismState {
    pub actor_id: u64,
    pub phase: PrismPhase,
    pub mode: PrismMode,
    pub pattern: Option<PrismPattern>,
    pub interval_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrismPhase {
    Lanes,
    Pulses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrismMode {
    Opening,
    Warning,
    PulseGap,
    Recovery,
    Shifting,
    Defeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "shape")]
pub enum PrismPattern {
    AcrossX { z: i32 },
    AcrossZ { x: i32 },
    Center,
    Perimeter,
}

/// A durable allied repair, sent only to negotiated support-capable sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairApplied {
    pub source_actor_id: u64,
    pub target_actor_id: u64,
    pub amount: u32,
    pub remaining_health: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityStart {
    pub activity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveUpdate {
    pub objective_id: String,
    pub objective_type: String,
    pub state: String,
    pub progress: u32,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveIntent {
    pub position: [i32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorState {
    pub door_id: String,
    pub open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityComplete {
    pub activity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootGranted {
    pub activity_id: String,
    pub item_id: String,
    pub quantity: u32,
    pub resulting_quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionGranted {
    pub activity_id: String,
    pub experience_granted: u64,
    pub experience: u64,
    pub previous_level: u32,
    pub level: u32,
    pub experience_to_next_level: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentChanged {
    pub accepted: bool,
    pub message: String,
    pub actor_id: u64,
    pub equipped_weapon_item_id: String,
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleStateRequest {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionStateRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionClaimIntent {
    pub arc_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionRequirement {
    pub milestone: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionArc {
    pub arc_id: String,
    pub status: String,
    pub reward_fragments: u32,
    pub requirements: Vec<AcquisitionRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionSnapshot {
    pub revision: String,
    pub can_claim: bool,
    pub arcs: Vec<AcquisitionArc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionClaimed {
    pub accepted: bool,
    pub replayed: bool,
    pub message: String,
    pub arc_id: String,
    pub granted_fragments: u32,
    pub state: AcquisitionSnapshot,
    pub modules: ModuleSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModulePreviewRequest {
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleCombineIntent {
    pub operation_id: String,
    pub module_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleLoadoutIntent {
    pub operation_id: String,
    pub expected_revision: u64,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleCatalogEntry {
    pub module_id: String,
    pub family: String,
    pub recipe_fragments: u32,
    pub damage_basis_points: i32,
    pub cooldown_basis_points: i32,
    pub range_delta: i32,
    pub max_health_delta: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleWeaponProfile {
    pub item_id: String,
    pub base_damage: u32,
    pub base_range: i32,
    pub base_cooldown_ms: u64,
    pub effective_damage: u32,
    pub effective_range: i32,
    pub effective_cooldown_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSnapshot {
    pub catalog_revision: String,
    pub fragments: u32,
    pub owned_modules: Vec<String>,
    pub loadout_revision: u64,
    pub equipped_modules: Vec<String>,
    pub maximum_slots: u8,
    pub catalog: Vec<ModuleCatalogEntry>,
    pub weapons: Vec<ModuleWeaponProfile>,
    pub max_health: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModulePreview {
    pub accepted: bool,
    pub message: String,
    pub requested_modules: Vec<String>,
    pub weapons: Vec<ModuleWeaponProfile>,
    pub max_health: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleCombined {
    pub accepted: bool,
    pub replayed: bool,
    pub message: String,
    pub operation_id: String,
    pub module_id: String,
    pub state: ModuleSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleLoadoutChanged {
    pub accepted: bool,
    pub replayed: bool,
    pub message: String,
    pub operation_id: String,
    pub state: ModuleSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteStateRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteChoiceIntent {
    pub operation_id: String,
    pub route_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePhase {
    Waiting,
    Drone,
    ChoiceOpen,
    BaselineLocked,
    Routed,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteObjectiveId {
    ClearDroneGroup,
    ReachRelayStabilizer,
    ReachRelayDoor,
    DefeatWarden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteObjectiveState {
    Pending,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteTerminalOutcome {
    Succeeded,
    FailedTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteEffect {
    pub warden_health_basis_points: u32,
    pub warden_counter_damage: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteReward {
    pub item_id: String,
    pub item_quantity: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteEventCandidate {
    pub event_id: String,
    pub effect: RouteEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteOption {
    pub route_id: String,
    pub objective_path: Vec<RouteObjectiveId>,
    pub duration_budget_ms: u64,
    pub reward: RouteReward,
    pub events: Vec<RouteEventCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteSelection {
    pub leader_actor_id: u64,
    pub participant_actor_ids: Vec<u64>,
    pub route_id: String,
    pub seed: u64,
    pub event_id: String,
    pub effect: RouteEffect,
    pub objective_path: Vec<RouteObjectiveId>,
    pub duration_budget_ms: u64,
    pub reward: RouteReward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteState {
    pub schema_version: u16,
    pub accepted: bool,
    pub message: String,
    pub session_id: String,
    pub activity_id: String,
    pub phase: RoutePhase,
    pub leader_actor_id: u64,
    pub participant_actor_ids: Vec<u64>,
    pub capable_actor_ids: Vec<u64>,
    pub all_capable: bool,
    pub routes: Vec<RouteOption>,
    pub selection: Option<RouteSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteChoiceResult {
    pub schema_version: u16,
    pub accepted: bool,
    pub replayed: bool,
    pub message: String,
    pub session_id: String,
    pub operation_id: String,
    pub selection: Option<RouteSelection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteObjectiveTransition {
    pub ordinal: u8,
    pub objective_id: RouteObjectiveId,
    pub from: RouteObjectiveState,
    pub to: RouteObjectiveState,
    pub progress: u32,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteGrant {
    pub actor_id: u64,
    pub item_id: String,
    pub item_quantity: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteOperationSummary {
    pub schema_version: u16,
    pub session_id: String,
    pub authoring_revision: String,
    pub catalog_revision: String,
    pub resolver_revision: String,
    pub operation_id: String,
    pub leader_actor_id: u64,
    pub participant_actor_ids: Vec<u64>,
    pub route_id: String,
    pub seed: u64,
    pub event_id: String,
    pub effect: RouteEffect,
    pub objective_path: Vec<RouteObjectiveId>,
    pub transitions: Vec<RouteObjectiveTransition>,
    pub duration_budget_ms: u64,
    pub elapsed_ms: u64,
    pub outcome: RouteTerminalOutcome,
    pub reward: Option<RouteReward>,
    pub grants: Vec<RouteGrant>,
    pub no_reward_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationStateRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationStartIntent {
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationPingIntent {
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationReviveIntent {
    pub operation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationPhase {
    Waiting,
    Drone,
    Eligible,
    AwaitingAnchor,
    AwaitingPing,
    AwaitingRunner,
    RunnerDowned,
    ReviveChannel,
    EncounterActive,
    Succeeded,
    Failed,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationRole {
    Anchor,
    Runner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationTarget {
    RelayAnchor,
    RelayConsole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationLife {
    Active,
    Downed,
    Defeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationTerminalOutcome {
    Succeeded,
    FailedPingTimeout,
    FailedReviveTimeout,
    FailedOperationTimeout,
    FailedParticipantDefeated,
    AbandonedDisconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationReviveStatus {
    Rejected,
    Started,
    Replayed,
    Pending,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationLifeCause {
    RelayFeedback,
    Revive,
    Warden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationNoRewardReason {
    PingTimeout,
    ReviveTimeout,
    OperationTimeout,
    ParticipantDefeated,
    ParticipantDisconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationParticipantState {
    pub actor_id: u64,
    pub role: CooperationRole,
    pub current_health: u32,
    pub max_health: u32,
    pub life: CooperationLife,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct CooperationContributions {
    pub anchor_arrived: bool,
    pub pinged: bool,
    pub runner_arrived: bool,
    pub revived: bool,
    pub warden_completed: bool,
    pub revive_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationTargetState {
    pub target: CooperationTarget,
    pub position: [i32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationTiming {
    pub operation_duration_ms: u64,
    pub ping_ttl_ms: u64,
    pub revive_window_ms: u64,
    pub revive_channel_ms: u64,
    pub revive_health: u32,
    pub maximum_distance_squared: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationReward {
    pub item_id: String,
    pub item_quantity: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationPingState {
    pub operation_id: String,
    pub source_actor_id: u64,
    pub target: CooperationTarget,
    pub accepted_elapsed_ms: u64,
    pub expires_elapsed_ms: u64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationReviveState {
    pub operation_id: String,
    pub source_actor_id: u64,
    pub target_actor_id: u64,
    pub started_elapsed_ms: u64,
    pub observed_elapsed_ms: u64,
    pub required_duration_ms: u64,
    pub maximum_distance_squared: i64,
    pub distance_squared: i64,
    pub status: CooperationReviveStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationOperationState {
    pub catalog_revision: String,
    pub start_operation_id: String,
    pub phase: CooperationPhase,
    pub participants: Vec<CooperationParticipantState>,
    pub targets: Vec<CooperationTargetState>,
    pub timing: CooperationTiming,
    pub reward: CooperationReward,
    pub contributions: CooperationContributions,
    pub observed_elapsed_ms: u64,
    pub ping: Option<CooperationPingState>,
    pub revive: Option<CooperationReviveState>,
    pub terminal_outcome: Option<CooperationTerminalOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationState {
    pub schema_version: u16,
    pub accepted: bool,
    pub message: String,
    pub session_id: String,
    pub activity_id: String,
    pub phase: CooperationPhase,
    pub participant_actor_ids: Vec<u64>,
    pub capable_actor_ids: Vec<u64>,
    pub all_capable: bool,
    pub operation: Option<CooperationOperationState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationStartResult {
    pub schema_version: u16,
    pub accepted: bool,
    pub replayed: bool,
    pub message: String,
    pub session_id: String,
    pub operation_id: String,
    pub operation: Option<CooperationOperationState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationPingResult {
    pub schema_version: u16,
    pub accepted: bool,
    pub replayed: bool,
    pub message: String,
    pub session_id: String,
    pub operation_id: String,
    pub ping: Option<CooperationPingState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationLifeState {
    pub schema_version: u16,
    pub session_id: String,
    pub actor_id: u64,
    pub role: CooperationRole,
    pub source_actor_id: Option<u64>,
    pub cause: CooperationLifeCause,
    pub elapsed_ms: u64,
    pub health_before: u32,
    pub health_after: u32,
    pub max_health: u32,
    pub life_before: CooperationLife,
    pub life_after: CooperationLife,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationReviveResult {
    pub schema_version: u16,
    pub accepted: bool,
    pub replayed: bool,
    pub message: String,
    pub session_id: String,
    pub operation_id: String,
    pub status: CooperationReviveStatus,
    pub revive: Option<CooperationReviveState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationGrant {
    pub actor_id: u64,
    pub item_id: String,
    pub item_quantity: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationOperationSummary {
    pub schema_version: u16,
    pub session_id: String,
    pub catalog_revision: String,
    pub start_operation_id: String,
    pub last_nonterminal_phase: CooperationPhase,
    pub participants: Vec<CooperationParticipantState>,
    pub contributions: CooperationContributions,
    pub outcome: CooperationTerminalOutcome,
    pub subject_role: Option<CooperationRole>,
    pub terminal_elapsed_ms: u64,
    pub reward: Option<CooperationReward>,
    pub grants: Vec<CooperationGrant>,
    pub no_reward_reason: Option<CooperationNoRewardReason>,
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    Encode(rmp_serde::encode::Error),
    Decode(rmp_serde::decode::Error),
    FrameTooLarge(usize),
}

impl Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "protocol I/O failed: {error}"),
            Self::Encode(error) => write!(formatter, "MessagePack encoding failed: {error}"),
            Self::Decode(error) => write!(formatter, "MessagePack decoding failed: {error}"),
            Self::FrameTooLarge(size) => write!(
                formatter,
                "protocol frame is {size} bytes; maximum is {MAX_FRAME_SIZE}"
            ),
        }
    }
}

impl Error for ProtocolError {}

impl From<io::Error> for ProtocolError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rmp_serde::encode::Error> for ProtocolError {
    fn from(error: rmp_serde::encode::Error) -> Self {
        Self::Encode(error)
    }
}

impl From<rmp_serde::decode::Error> for ProtocolError {
    fn from(error: rmp_serde::decode::Error) -> Self {
        Self::Decode(error)
    }
}

/// Writes one length-prefixed `MessagePack` message.
///
/// # Errors
///
/// Returns an error if serialization fails, the payload exceeds the frame limit,
/// or the writer cannot accept the complete frame.
pub fn write_message<W, T>(writer: &mut W, message: &T) -> Result<(), ProtocolError>
where
    W: Write,
    T: Serialize,
{
    let payload = rmp_serde::to_vec_named(message)?;
    if payload.len() > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(payload.len()));
    }

    let size =
        u32::try_from(payload.len()).map_err(|_| ProtocolError::FrameTooLarge(payload.len()))?;
    writer.write_all(&size.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

/// Reads one length-prefixed `MessagePack` message.
///
/// # Errors
///
/// Returns an error if the frame is incomplete, exceeds the frame limit, or does
/// not contain a valid message of the requested type.
pub fn read_message<R, T>(reader: &mut R) -> Result<T, ProtocolError>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut size_bytes = [0_u8; 4];
    reader.read_exact(&mut size_bytes)?;
    let size = usize::try_from(u32::from_be_bytes(size_bytes))
        .map_err(|_| ProtocolError::FrameTooLarge(usize::MAX))?;
    if size > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(size));
    }

    let mut payload = vec![0_u8; size];
    reader.read_exact(&mut payload)?;
    Ok(rmp_serde::from_slice(&payload)?)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde::Serialize;

    use super::{
        read_message, write_message, CharacterListRequest, ClientHello, ClientMessage,
        CooperationContributions, CooperationGrant, CooperationLife, CooperationLifeCause,
        CooperationLifeState, CooperationNoRewardReason, CooperationOperationState,
        CooperationOperationSummary, CooperationParticipantState, CooperationPhase,
        CooperationPingIntent, CooperationPingResult, CooperationPingState,
        CooperationReviveIntent, CooperationReviveResult, CooperationReviveState,
        CooperationReviveStatus, CooperationReward, CooperationRole, CooperationStartIntent,
        CooperationStartResult, CooperationState, CooperationStateRequest, CooperationTarget,
        CooperationTargetState, CooperationTerminalOutcome, CooperationTiming, EquipmentSnapshot,
        InventoryItem, InventorySnapshot, ModuleCatalogEntry, ModuleCombineIntent, ModuleCombined,
        ModuleLoadoutChanged, ModuleLoadoutIntent, ModulePreview, ModulePreviewRequest,
        ModuleSnapshot, ModuleStateRequest, ModuleWeaponProfile, ProgressionSnapshot,
        RouteChoiceIntent, RouteChoiceResult, RouteEffect, RouteEventCandidate, RouteGrant,
        RouteObjectiveId, RouteObjectiveState, RouteObjectiveTransition, RouteOperationSummary,
        RouteOption, RoutePhase, RouteReward, RouteSelection, RouteState, RouteStateRequest,
        RouteTerminalOutcome, ServerMessage, WeaponProfile, COOPERATION_WIRE_SCHEMA_VERSION,
        MAX_COOPERATION_GRANTS, MAX_COOPERATION_MESSAGE_BYTES, MAX_COOPERATION_OPERATION_ID_BYTES,
        MAX_COOPERATION_PARTICIPANTS, MAX_COOPERATION_TARGETS, MAX_COOPERATION_WIRE_PAYLOAD_SIZE,
        MAX_FRAME_SIZE, MAX_MODULE_OPERATION_ID_BYTES, MAX_ROUTE_EVENT_CANDIDATES,
        MAX_ROUTE_GRANTS, MAX_ROUTE_IDENTITY_BYTES, MAX_ROUTE_MESSAGE_BYTES, MAX_ROUTE_OBJECTIVES,
        MAX_ROUTE_OPERATION_ID_BYTES, MAX_ROUTE_OPTIONS, MAX_ROUTE_PARTICIPANTS,
        MAX_ROUTE_REVISION_BYTES, MAX_ROUTE_TRANSITIONS, MAX_ROUTE_WIRE_PAYLOAD_SIZE,
        PROTOCOL_VERSION, ROUTE_WIRE_SCHEMA_VERSION,
    };

    #[test]
    fn client_hello_round_trips_through_framing() {
        let expected = ClientMessage::ClientHello(ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "protocol-test".to_owned(),
            client_build: env!("CARGO_PKG_VERSION").to_owned(),
            content_revision: None,
        });
        let mut frame = Vec::new();

        write_message(&mut frame, &expected).expect("client hello should encode");
        let actual: ClientMessage =
            read_message(&mut Cursor::new(frame)).expect("client hello should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn prism_state_round_trip_retains_signed_lanes_and_explicit_phases() {
        for pattern in [
            super::PrismPattern::AcrossX { z: -3 },
            super::PrismPattern::AcrossZ { x: 5 },
            super::PrismPattern::Center,
            super::PrismPattern::Perimeter,
        ] {
            let expected = ServerMessage::PrismState(super::PrismState {
                actor_id: 7,
                phase: super::PrismPhase::Pulses,
                mode: super::PrismMode::Warning,
                pattern: Some(pattern),
                interval_ms: 2200,
            });
            let mut frame = Vec::new();
            write_message(&mut frame, &expected).unwrap();
            let actual: ServerMessage = read_message(&mut Cursor::new(frame)).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn content_negotiation_is_an_optional_v2_extension() {
        let mut hello = ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "content-test".to_owned(),
            client_build: env!("CARGO_PKG_VERSION").to_owned(),
            content_revision: None,
        };
        let legacy = rmp_serde::to_vec_named(&hello).unwrap();
        assert!(!legacy
            .windows("content_revision".len())
            .any(|part| part == b"content_revision"));
        assert_eq!(
            rmp_serde::from_slice::<ClientHello>(&legacy)
                .unwrap()
                .content_revision,
            None
        );
        hello.content_revision = Some(super::CONTENT_REVISION.to_owned());
        let modern = rmp_serde::to_vec_named(&hello).unwrap();
        assert_eq!(
            rmp_serde::from_slice::<ClientHello>(&modern).unwrap(),
            hello
        );
    }

    #[test]
    fn lancer_charge_is_optional_and_preserves_legacy_actor_update_bytes() {
        #[derive(Serialize)]
        struct LegacyUpdate {
            actor_id: u64,
            position: [i32; 3],
        }
        let legacy = rmp_serde::to_vec_named(&LegacyUpdate {
            actor_id: 3,
            position: [9, 0, 8],
        })
        .unwrap();
        let mut update: super::ActorUpdate = rmp_serde::from_slice(&legacy).unwrap();
        assert_eq!(update.charge, None);
        assert_eq!(update.defense, None);
        assert_eq!(rmp_serde::to_vec_named(&update).unwrap(), legacy);
        update.charge = Some(super::ChargeTelegraph {
            target: [4, 0, 8],
            winding_up: true,
            warning_ms: 1800,
        });
        update.defense = Some(super::DefenseTelegraph {
            facing: [-1, 0],
            braced: true,
            warning_ms: 1600,
        });
        let message = ServerMessage::ActorUpdate(update);
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).unwrap();
        assert_eq!(
            read_message::<_, ServerMessage>(&mut Cursor::new(bytes)).unwrap(),
            message
        );
    }

    #[test]
    fn oversized_frame_is_rejected_before_allocation() {
        let announced_size = u32::try_from(super::MAX_FRAME_SIZE + 1)
            .expect("frame limit should fit in a u32")
            .to_be_bytes();

        let result = read_message::<_, ClientMessage>(&mut Cursor::new(announced_size));

        assert!(matches!(
            result,
            Err(super::ProtocolError::FrameTooLarge(_))
        ));
    }

    #[test]
    fn empty_character_list_request_round_trips() {
        let expected = ClientMessage::CharacterListRequest(CharacterListRequest {});
        let mut frame = Vec::new();

        write_message(&mut frame, &expected).expect("request should encode");
        let actual: ClientMessage =
            read_message(&mut Cursor::new(frame)).expect("request should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn inventory_snapshot_round_trips() {
        let expected = ServerMessage::InventorySnapshot(InventorySnapshot {
            items: vec![InventoryItem {
                item_id: "pulse_rifle".to_owned(),
                quantity: 1,
            }],
        });
        let mut frame = Vec::new();
        write_message(&mut frame, &expected).expect("inventory should encode");
        let actual: ServerMessage =
            read_message(&mut Cursor::new(frame)).expect("inventory should decode");
        assert_eq!(actual, expected);
    }

    #[test]
    fn progression_snapshot_round_trips() {
        let expected = ServerMessage::ProgressionSnapshot(ProgressionSnapshot {
            level: 2,
            experience: 500,
            experience_to_next_level: 500,
        });
        let mut frame = Vec::new();
        write_message(&mut frame, &expected).expect("progression should encode");
        let actual: ServerMessage =
            read_message(&mut Cursor::new(frame)).expect("progression should decode");
        assert_eq!(actual, expected);
    }

    #[test]
    fn equipment_snapshot_round_trips() {
        let expected = ServerMessage::EquipmentSnapshot(EquipmentSnapshot {
            equipped_weapon_item_id: "arc_sidearm".to_owned(),
            weapons: vec![WeaponProfile {
                item_id: "arc_sidearm".to_owned(),
                damage: 25,
                range: 8,
                cooldown_ms: 150,
            }],
        });
        let mut frame = Vec::new();
        write_message(&mut frame, &expected).expect("equipment should encode");
        let actual: ServerMessage =
            read_message(&mut Cursor::new(frame)).expect("equipment should decode");
        assert_eq!(actual, expected);
    }

    fn module_weapon() -> ModuleWeaponProfile {
        ModuleWeaponProfile {
            item_id: "pulse_rifle".to_owned(),
            base_damage: 40,
            base_range: 6,
            base_cooldown_ms: 250,
            effective_damage: 48,
            effective_range: 6,
            effective_cooldown_ms: 300,
        }
    }

    fn module_snapshot() -> ModuleSnapshot {
        ModuleSnapshot {
            catalog_revision: "m26-v1".to_owned(),
            fragments: 4,
            owned_modules: vec!["module_force_matrix".to_owned()],
            loadout_revision: 1,
            equipped_modules: vec!["module_force_matrix".to_owned()],
            maximum_slots: 3,
            catalog: vec![ModuleCatalogEntry {
                module_id: "module_force_matrix".to_owned(),
                family: "force".to_owned(),
                recipe_fragments: 2,
                damage_basis_points: 2_000,
                cooldown_basis_points: 2_000,
                range_delta: 0,
                max_health_delta: 0,
            }],
            weapons: vec![module_weapon()],
            max_health: 100,
        }
    }

    #[test]
    fn acquisition_messages_round_trip_with_bounded_state() {
        for message in [
            super::ClientMessage::AcquisitionStateRequest(super::AcquisitionStateRequest {}),
            super::ClientMessage::AcquisitionClaimIntent(super::AcquisitionClaimIntent {
                arc_id: "routes".to_owned(),
            }),
        ] {
            let mut bytes = Vec::new();
            super::write_message(&mut bytes, &message).unwrap();
            assert_eq!(
                super::read_message::<_, super::ClientMessage>(&mut bytes.as_slice()).unwrap(),
                message
            );
        }
        let state = super::AcquisitionSnapshot {
            revision: "m35-acquisition-v1".to_owned(),
            can_claim: true,
            arcs: vec![super::AcquisitionArc {
                arc_id: "meridian".to_owned(),
                status: "claimed".to_owned(),
                reward_fragments: 2,
                requirements: vec![super::AcquisitionRequirement {
                    milestone: "meridian_recovered".to_owned(),
                    completed: true,
                }],
            }],
        };
        for message in [
            super::ServerMessage::AcquisitionSnapshot(state.clone()),
            super::ServerMessage::AcquisitionClaimed(super::AcquisitionClaimed {
                accepted: true,
                replayed: false,
                message: "commission claimed".to_owned(),
                arc_id: "meridian".to_owned(),
                granted_fragments: 2,
                state,
                modules: module_snapshot(),
            }),
        ] {
            let mut bytes = Vec::new();
            super::write_message(&mut bytes, &message).unwrap();
            assert!(bytes.len() < 8192);
            assert_eq!(
                super::read_message::<_, super::ServerMessage>(&mut bytes.as_slice()).unwrap(),
                message
            );
        }
    }

    #[test]
    fn module_client_messages_round_trip() {
        let messages = [
            ClientMessage::ModuleStateRequest(ModuleStateRequest {}),
            ClientMessage::ModulePreviewRequest(ModulePreviewRequest {
                modules: vec!["module_force_matrix".to_owned()],
            }),
            ClientMessage::ModuleCombineIntent(ModuleCombineIntent {
                operation_id: "combine-1".to_owned(),
                module_id: "module_force_matrix".to_owned(),
            }),
            ClientMessage::ModuleLoadoutIntent(ModuleLoadoutIntent {
                operation_id: "loadout-1".to_owned(),
                expected_revision: 0,
                modules: vec!["module_force_matrix".to_owned()],
            }),
        ];
        for expected in messages {
            let mut frame = Vec::new();
            write_message(&mut frame, &expected).expect("module request should encode");
            let actual: ClientMessage =
                read_message(&mut Cursor::new(frame)).expect("module request should decode");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn module_server_messages_round_trip() {
        let snapshot = module_snapshot();
        let messages = [
            ServerMessage::ModuleSnapshot(snapshot.clone()),
            ServerMessage::ModulePreview(ModulePreview {
                accepted: true,
                message: "preview resolved".to_owned(),
                requested_modules: vec!["module_force_matrix".to_owned()],
                weapons: vec![module_weapon()],
                max_health: 100,
            }),
            ServerMessage::ModuleCombined(ModuleCombined {
                accepted: true,
                replayed: false,
                message: "module combined".to_owned(),
                operation_id: "combine-1".to_owned(),
                module_id: "module_force_matrix".to_owned(),
                state: snapshot.clone(),
            }),
            ServerMessage::ModuleLoadoutChanged(ModuleLoadoutChanged {
                accepted: true,
                replayed: false,
                message: "module loadout changed".to_owned(),
                operation_id: "loadout-1".to_owned(),
                state: snapshot,
            }),
        ];
        for expected in messages {
            let mut frame = Vec::new();
            write_message(&mut frame, &expected).expect("module response should encode");
            assert!(frame.len() < super::MAX_FRAME_SIZE);
            let actual: ServerMessage =
                read_message(&mut Cursor::new(frame)).expect("module response should decode");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn maximum_shaped_module_request_and_snapshot_stay_bounded() {
        let modules = vec![
            "module_force_matrix".to_owned(),
            "module_tempo_regulator".to_owned(),
            "module_reach_lattice".to_owned(),
        ];
        let request = ClientMessage::ModuleLoadoutIntent(ModuleLoadoutIntent {
            operation_id: "a".repeat(MAX_MODULE_OPERATION_ID_BYTES),
            expected_revision: u64::MAX,
            modules: modules.clone(),
        });
        let mut request_frame = Vec::new();
        write_message(&mut request_frame, &request).expect("maximum request should encode");
        assert!(request_frame.len() <= MAX_FRAME_SIZE + 4);
        let decoded_request: ClientMessage =
            read_message(&mut Cursor::new(request_frame)).expect("maximum request should decode");
        assert_eq!(decoded_request, request);

        let mut snapshot = module_snapshot();
        snapshot.fragments = u32::MAX;
        snapshot.owned_modules = vec![
            "module_force_matrix".to_owned(),
            "module_tempo_regulator".to_owned(),
            "module_reach_lattice".to_owned(),
            "module_ward_capacitor".to_owned(),
        ];
        snapshot.equipped_modules = modules;
        snapshot.loadout_revision = u64::MAX;
        snapshot.catalog = [
            ("module_force_matrix", "force", 2_000, 2_000, 0, 0),
            ("module_tempo_regulator", "tempo", -1_000, -1_500, 0, 0),
            ("module_reach_lattice", "reach", -1_000, 0, 2, 0),
            ("module_ward_capacitor", "ward", 0, 1_000, 0, 20),
        ]
        .into_iter()
        .map(
            |(
                module_id,
                family,
                damage_basis_points,
                cooldown_basis_points,
                range_delta,
                max_health_delta,
            )| ModuleCatalogEntry {
                module_id: module_id.to_owned(),
                family: family.to_owned(),
                recipe_fragments: 2,
                damage_basis_points,
                cooldown_basis_points,
                range_delta,
                max_health_delta,
            },
        )
        .collect();
        snapshot.weapons.push(ModuleWeaponProfile {
            item_id: "arc_sidearm".to_owned(),
            base_damage: 25,
            base_range: 8,
            base_cooldown_ms: 150,
            effective_damage: 25,
            effective_range: 10,
            effective_cooldown_ms: 158,
        });
        snapshot.max_health = 120;
        let response = ServerMessage::ModuleSnapshot(snapshot);
        let mut response_frame = Vec::new();
        write_message(&mut response_frame, &response).expect("maximum snapshot should encode");
        assert!(response_frame.len() <= MAX_FRAME_SIZE + 4);
        let decoded_response: ServerMessage =
            read_message(&mut Cursor::new(response_frame)).expect("maximum snapshot should decode");
        assert_eq!(decoded_response, response);
    }

    fn route_effect() -> RouteEffect {
        RouteEffect {
            warden_health_basis_points: 12_000,
            warden_counter_damage: 20,
        }
    }

    fn route_reward() -> RouteReward {
        RouteReward {
            item_id: "fragment".to_owned(),
            item_quantity: 2,
            experience: 150,
        }
    }

    fn route_option(route_id: &str, stabilizer: bool) -> RouteOption {
        let mut objective_path = vec![RouteObjectiveId::ClearDroneGroup];
        if stabilizer {
            objective_path.push(RouteObjectiveId::ReachRelayStabilizer);
        }
        objective_path.extend([
            RouteObjectiveId::ReachRelayDoor,
            RouteObjectiveId::DefeatWarden,
        ]);
        RouteOption {
            route_id: route_id.to_owned(),
            objective_path,
            duration_budget_ms: 90_000,
            reward: route_reward(),
            events: (0..MAX_ROUTE_EVENT_CANDIDATES)
                .map(|index| RouteEventCandidate {
                    event_id: format!("event_{index}"),
                    effect: route_effect(),
                })
                .collect(),
        }
    }

    fn route_selection() -> RouteSelection {
        RouteSelection {
            leader_actor_id: 1,
            participant_actor_ids: vec![1, 2],
            route_id: "breach".to_owned(),
            seed: i64::MAX.unsigned_abs(),
            event_id: "overcharged_armor".to_owned(),
            effect: route_effect(),
            objective_path: vec![
                RouteObjectiveId::ClearDroneGroup,
                RouteObjectiveId::ReachRelayDoor,
                RouteObjectiveId::DefeatWarden,
            ],
            duration_budget_ms: 90_000,
            reward: route_reward(),
        }
    }

    fn route_state() -> RouteState {
        RouteState {
            schema_version: ROUTE_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: "route state".to_owned(),
            session_id: "session-route-1".to_owned(),
            activity_id: "relay_awakening".to_owned(),
            phase: RoutePhase::ChoiceOpen,
            leader_actor_id: 1,
            participant_actor_ids: vec![1, 2],
            capable_actor_ids: vec![1, 2],
            all_capable: true,
            routes: vec![
                route_option("breach", false),
                route_option("stabilize", true),
            ],
            selection: Some(route_selection()),
        }
    }

    fn route_summary() -> RouteOperationSummary {
        RouteOperationSummary {
            schema_version: ROUTE_WIRE_SCHEMA_VERSION,
            session_id: "session-route-1".to_owned(),
            authoring_revision: "m27-v1".to_owned(),
            catalog_revision: "m27-v1".to_owned(),
            resolver_revision: "m27-permute63-v1".to_owned(),
            operation_id: "route-operation-1".to_owned(),
            leader_actor_id: 1,
            participant_actor_ids: vec![1, 2],
            route_id: "breach".to_owned(),
            seed: i64::MAX.unsigned_abs(),
            event_id: "overcharged_armor".to_owned(),
            effect: route_effect(),
            objective_path: vec![
                RouteObjectiveId::ClearDroneGroup,
                RouteObjectiveId::ReachRelayDoor,
                RouteObjectiveId::DefeatWarden,
            ],
            transitions: vec![
                RouteObjectiveTransition {
                    ordinal: 0,
                    objective_id: RouteObjectiveId::ClearDroneGroup,
                    from: RouteObjectiveState::Active,
                    to: RouteObjectiveState::Completed,
                    progress: 1,
                    target: 1,
                },
                RouteObjectiveTransition {
                    ordinal: 1,
                    objective_id: RouteObjectiveId::ReachRelayDoor,
                    from: RouteObjectiveState::Pending,
                    to: RouteObjectiveState::Active,
                    progress: 0,
                    target: 1,
                },
            ],
            duration_budget_ms: 90_000,
            elapsed_ms: 90_000,
            outcome: RouteTerminalOutcome::Succeeded,
            reward: Some(route_reward()),
            grants: vec![
                RouteGrant {
                    actor_id: 1,
                    item_id: "fragment".to_owned(),
                    item_quantity: 2,
                    experience: 150,
                },
                RouteGrant {
                    actor_id: 2,
                    item_id: "fragment".to_owned(),
                    item_quantity: 2,
                    experience: 150,
                },
            ],
            no_reward_reason: None,
        }
    }

    #[test]
    fn route_client_messages_round_trip() {
        let messages = [
            ClientMessage::RouteStateRequest(RouteStateRequest {}),
            ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                operation_id: "route-operation-1".to_owned(),
                route_id: "breach".to_owned(),
            }),
        ];
        for expected in messages {
            let mut frame = Vec::new();
            write_message(&mut frame, &expected).expect("route request should encode");
            let actual: ClientMessage =
                read_message(&mut Cursor::new(frame)).expect("route request should decode");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn route_server_messages_round_trip() {
        let messages = [
            ServerMessage::RouteState(route_state()),
            ServerMessage::RouteChoiceResult(RouteChoiceResult {
                schema_version: ROUTE_WIRE_SCHEMA_VERSION,
                accepted: true,
                replayed: false,
                message: "route selected".to_owned(),
                session_id: "session-route-1".to_owned(),
                operation_id: "route-operation-1".to_owned(),
                selection: Some(route_selection()),
            }),
            ServerMessage::RouteOperationSummary(route_summary()),
        ];
        for expected in messages {
            let mut frame = Vec::new();
            write_message(&mut frame, &expected).expect("route response should encode");
            let actual: ServerMessage =
                read_message(&mut Cursor::new(frame)).expect("route response should decode");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn maximum_shaped_route_messages_stay_below_private_and_frame_bounds() {
        let request = ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
            operation_id: "a".repeat(MAX_ROUTE_OPERATION_ID_BYTES),
            route_id: "b".repeat(super::MAX_ROUTE_ID_BYTES),
        });
        let mut request_frame = Vec::new();
        write_message(&mut request_frame, &request).expect("maximum route request should encode");
        assert!(request_frame.len() - 4 < MAX_ROUTE_WIRE_PAYLOAD_SIZE);
        assert!(request_frame.len() <= MAX_FRAME_SIZE + 4);

        let maximum_identity = "s".repeat(MAX_ROUTE_IDENTITY_BYTES);
        let maximum_revision = "r".repeat(MAX_ROUTE_REVISION_BYTES);
        let maximum_message = "m".repeat(MAX_ROUTE_MESSAGE_BYTES);
        let maximum_path = vec![RouteObjectiveId::DefeatWarden; MAX_ROUTE_OBJECTIVES];
        let maximum_option = RouteOption {
            route_id: "r".repeat(super::MAX_ROUTE_ID_BYTES),
            objective_path: maximum_path.clone(),
            duration_budget_ms: u64::MAX,
            reward: route_reward(),
            events: vec![
                RouteEventCandidate {
                    event_id: "e".repeat(super::MAX_ROUTE_ID_BYTES),
                    effect: route_effect(),
                };
                MAX_ROUTE_EVENT_CANDIDATES
            ],
        };
        let maximum_selection = RouteSelection {
            leader_actor_id: u64::MAX,
            participant_actor_ids: vec![u64::MAX; MAX_ROUTE_PARTICIPANTS],
            route_id: "r".repeat(super::MAX_ROUTE_ID_BYTES),
            seed: i64::MAX.unsigned_abs(),
            event_id: "e".repeat(super::MAX_ROUTE_ID_BYTES),
            effect: route_effect(),
            objective_path: maximum_path.clone(),
            duration_budget_ms: u64::MAX,
            reward: route_reward(),
        };
        let state = ServerMessage::RouteState(RouteState {
            schema_version: ROUTE_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: maximum_message,
            session_id: maximum_identity,
            activity_id: "a".repeat(super::MAX_ROUTE_ID_BYTES),
            phase: RoutePhase::Routed,
            leader_actor_id: u64::MAX,
            participant_actor_ids: vec![u64::MAX; MAX_ROUTE_PARTICIPANTS],
            capable_actor_ids: vec![u64::MAX; MAX_ROUTE_PARTICIPANTS],
            all_capable: true,
            routes: vec![maximum_option; MAX_ROUTE_OPTIONS],
            selection: Some(maximum_selection),
        });
        let mut state_frame = Vec::new();
        write_message(&mut state_frame, &state).expect("maximum route state should encode");
        assert!(state_frame.len() - 4 < MAX_ROUTE_WIRE_PAYLOAD_SIZE);
        assert!(state_frame.len() <= MAX_FRAME_SIZE + 4);

        let mut maximum_summary = route_summary();
        maximum_summary.session_id = "s".repeat(MAX_ROUTE_IDENTITY_BYTES);
        maximum_summary.authoring_revision = maximum_revision.clone();
        maximum_summary.catalog_revision = maximum_revision.clone();
        maximum_summary.resolver_revision = maximum_revision;
        maximum_summary.operation_id = "o".repeat(MAX_ROUTE_OPERATION_ID_BYTES);
        maximum_summary.participant_actor_ids = vec![u64::MAX; MAX_ROUTE_PARTICIPANTS];
        maximum_summary.objective_path = maximum_path;
        maximum_summary.transitions = vec![
            RouteObjectiveTransition {
                ordinal: u8::MAX,
                objective_id: RouteObjectiveId::DefeatWarden,
                from: RouteObjectiveState::Active,
                to: RouteObjectiveState::Completed,
                progress: u32::MAX,
                target: u32::MAX,
            };
            MAX_ROUTE_TRANSITIONS
        ];
        maximum_summary.grants = vec![
            RouteGrant {
                actor_id: u64::MAX,
                item_id: "i".repeat(super::MAX_ROUTE_ID_BYTES),
                item_quantity: u32::MAX,
                experience: u64::MAX,
            };
            MAX_ROUTE_GRANTS
        ];
        maximum_summary.no_reward_reason = Some("r".repeat(super::MAX_ROUTE_ID_BYTES));
        let mut summary_frame = Vec::new();
        write_message(
            &mut summary_frame,
            &ServerMessage::RouteOperationSummary(maximum_summary),
        )
        .expect("maximum route summary should encode");
        assert!(summary_frame.len() - 4 < MAX_ROUTE_WIRE_PAYLOAD_SIZE);
        assert!(summary_frame.len() <= MAX_FRAME_SIZE + 4);
    }

    #[test]
    fn route_nested_unknown_fields_and_invalid_enums_are_rejected() {
        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum UnknownClientMessage {
            RouteChoiceIntent(UnknownRouteChoiceIntent),
        }

        #[derive(Serialize)]
        struct UnknownRouteChoiceIntent {
            operation_id: String,
            route_id: String,
            client_seed: u64,
        }

        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum InvalidServerMessage {
            RouteState(InvalidRouteState),
        }

        #[derive(Serialize)]
        struct InvalidRouteState {
            schema_version: u16,
            accepted: bool,
            message: String,
            session_id: String,
            activity_id: String,
            phase: String,
            leader_actor_id: u64,
            participant_actor_ids: Vec<u64>,
            capable_actor_ids: Vec<u64>,
            all_capable: bool,
            routes: Vec<RouteOption>,
            selection: Option<RouteSelection>,
        }

        let payload = rmp_serde::to_vec_named(&UnknownClientMessage::RouteChoiceIntent(
            UnknownRouteChoiceIntent {
                operation_id: "route-1".to_owned(),
                route_id: "breach".to_owned(),
                client_seed: 7,
            },
        ))
        .expect("unknown-field fixture should encode");
        assert!(rmp_serde::from_slice::<ClientMessage>(&payload).is_err());

        let invalid = InvalidRouteState {
            schema_version: ROUTE_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: String::new(),
            session_id: "session".to_owned(),
            activity_id: "relay_awakening".to_owned(),
            phase: "client_authored_phase".to_owned(),
            leader_actor_id: 1,
            participant_actor_ids: vec![1],
            capable_actor_ids: vec![1],
            all_capable: true,
            routes: vec![
                route_option("breach", false),
                route_option("stabilize", true),
            ],
            selection: None,
        };
        let payload = rmp_serde::to_vec_named(&InvalidServerMessage::RouteState(invalid))
            .expect("invalid-enum fixture should encode");
        assert!(rmp_serde::from_slice::<ServerMessage>(&payload).is_err());
    }

    fn cooperation_participant(
        actor_id: u64,
        role: CooperationRole,
    ) -> CooperationParticipantState {
        CooperationParticipantState {
            actor_id,
            role,
            current_health: 100,
            max_health: 100,
            life: CooperationLife::Active,
        }
    }

    fn cooperation_operation_state() -> CooperationOperationState {
        CooperationOperationState {
            catalog_revision: "m28-v1".to_owned(),
            start_operation_id: "cooperation-start-1".to_owned(),
            phase: CooperationPhase::EncounterActive,
            participants: vec![
                cooperation_participant(1, CooperationRole::Anchor),
                cooperation_participant(2, CooperationRole::Runner),
            ],
            targets: vec![
                CooperationTargetState {
                    target: CooperationTarget::RelayAnchor,
                    position: [3, 0, 3],
                },
                CooperationTargetState {
                    target: CooperationTarget::RelayConsole,
                    position: [4, 0, 3],
                },
            ],
            timing: CooperationTiming {
                operation_duration_ms: 60_000,
                ping_ttl_ms: 5_000,
                revive_window_ms: 15_000,
                revive_channel_ms: 2_000,
                revive_health: 50,
                maximum_distance_squared: 4,
            },
            reward: CooperationReward {
                item_id: "relay_core_fragment".to_owned(),
                item_quantity: 2,
                experience: 125,
            },
            contributions: CooperationContributions {
                anchor_arrived: true,
                pinged: true,
                runner_arrived: true,
                revived: true,
                warden_completed: false,
                revive_count: 1,
            },
            observed_elapsed_ms: 2_400,
            ping: Some(CooperationPingState {
                operation_id: "cooperation-ping-1".to_owned(),
                source_actor_id: 1,
                target: CooperationTarget::RelayConsole,
                accepted_elapsed_ms: 200,
                expires_elapsed_ms: 5_200,
                active: false,
            }),
            revive: Some(CooperationReviveState {
                operation_id: "cooperation-revive-1".to_owned(),
                source_actor_id: 1,
                target_actor_id: 2,
                started_elapsed_ms: 400,
                observed_elapsed_ms: 2_400,
                required_duration_ms: 2_000,
                maximum_distance_squared: 4,
                distance_squared: 1,
                status: CooperationReviveStatus::Completed,
            }),
            terminal_outcome: None,
        }
    }

    fn cooperation_state() -> CooperationState {
        CooperationState {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: "cooperation state refreshed".to_owned(),
            session_id: "session-cooperation-1".to_owned(),
            activity_id: "relay_awakening".to_owned(),
            phase: CooperationPhase::EncounterActive,
            participant_actor_ids: vec![1, 2],
            capable_actor_ids: vec![1, 2],
            all_capable: true,
            operation: Some(cooperation_operation_state()),
        }
    }

    fn cooperation_summary() -> CooperationOperationSummary {
        CooperationOperationSummary {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            session_id: "session-cooperation-1".to_owned(),
            catalog_revision: "m28-v1".to_owned(),
            start_operation_id: "cooperation-start-1".to_owned(),
            last_nonterminal_phase: CooperationPhase::EncounterActive,
            participants: vec![
                cooperation_participant(1, CooperationRole::Anchor),
                CooperationParticipantState {
                    current_health: 50,
                    ..cooperation_participant(2, CooperationRole::Runner)
                },
            ],
            contributions: CooperationContributions {
                anchor_arrived: true,
                pinged: true,
                runner_arrived: true,
                revived: true,
                warden_completed: true,
                revive_count: 1,
            },
            outcome: CooperationTerminalOutcome::Succeeded,
            subject_role: None,
            terminal_elapsed_ms: 3_000,
            reward: Some(CooperationReward {
                item_id: "relay_core_fragment".to_owned(),
                item_quantity: 2,
                experience: 125,
            }),
            grants: vec![
                CooperationGrant {
                    actor_id: 1,
                    item_id: "relay_core_fragment".to_owned(),
                    item_quantity: 2,
                    experience: 125,
                },
                CooperationGrant {
                    actor_id: 2,
                    item_id: "relay_core_fragment".to_owned(),
                    item_quantity: 2,
                    experience: 125,
                },
            ],
            no_reward_reason: None,
        }
    }

    #[test]
    fn cooperation_client_messages_round_trip() {
        let messages = [
            ClientMessage::CooperationStateRequest(CooperationStateRequest {}),
            ClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id: "cooperation-start-1".to_owned(),
            }),
            ClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id: "cooperation-ping-1".to_owned(),
            }),
            ClientMessage::CooperationReviveIntent(CooperationReviveIntent {
                operation_id: "cooperation-revive-1".to_owned(),
            }),
        ];
        for expected in messages {
            let mut frame = Vec::new();
            write_message(&mut frame, &expected).expect("cooperation request should encode");
            let actual: ClientMessage =
                read_message(&mut Cursor::new(frame)).expect("cooperation request should decode");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn cooperation_server_messages_round_trip() {
        let operation = cooperation_operation_state();
        let ping = operation.ping.clone();
        let revive = operation.revive.clone();
        let messages = [
            ServerMessage::CooperationState(cooperation_state()),
            ServerMessage::CooperationStartResult(CooperationStartResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: true,
                replayed: false,
                message: "cooperation started".to_owned(),
                session_id: "session-cooperation-1".to_owned(),
                operation_id: "cooperation-start-1".to_owned(),
                operation: Some(operation),
            }),
            ServerMessage::CooperationPingResult(CooperationPingResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: true,
                replayed: false,
                message: "cooperation ping accepted".to_owned(),
                session_id: "session-cooperation-1".to_owned(),
                operation_id: "cooperation-ping-1".to_owned(),
                ping,
            }),
            ServerMessage::CooperationLifeState(CooperationLifeState {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                session_id: "session-cooperation-1".to_owned(),
                actor_id: 2,
                role: CooperationRole::Runner,
                source_actor_id: None,
                cause: CooperationLifeCause::RelayFeedback,
                elapsed_ms: 300,
                health_before: 90,
                health_after: 0,
                max_health: 100,
                life_before: CooperationLife::Active,
                life_after: CooperationLife::Downed,
            }),
            ServerMessage::CooperationReviveResult(CooperationReviveResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: true,
                replayed: false,
                message: "cooperation revive completed".to_owned(),
                session_id: "session-cooperation-1".to_owned(),
                operation_id: "cooperation-revive-1".to_owned(),
                status: CooperationReviveStatus::Completed,
                revive,
            }),
            ServerMessage::CooperationOperationSummary(cooperation_summary()),
        ];
        for expected in messages {
            let mut frame = Vec::new();
            write_message(&mut frame, &expected).expect("cooperation response should encode");
            let actual: ServerMessage =
                read_message(&mut Cursor::new(frame)).expect("cooperation response should decode");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn maximum_shaped_cooperation_messages_stay_below_private_and_frame_bounds() {
        for request in [
            ClientMessage::CooperationStateRequest(CooperationStateRequest {}),
            ClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id: "s".repeat(MAX_COOPERATION_OPERATION_ID_BYTES),
            }),
            ClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id: "p".repeat(MAX_COOPERATION_OPERATION_ID_BYTES),
            }),
            ClientMessage::CooperationReviveIntent(CooperationReviveIntent {
                operation_id: "r".repeat(MAX_COOPERATION_OPERATION_ID_BYTES),
            }),
        ] {
            let mut frame = Vec::new();
            write_message(&mut frame, &request).expect("maximum cooperation request should encode");
            assert!(frame.len() - 4 < MAX_COOPERATION_WIRE_PAYLOAD_SIZE);
            assert!(frame.len() <= MAX_FRAME_SIZE + 4);
        }

        let maximum_message = "m".repeat(MAX_COOPERATION_MESSAGE_BYTES);
        let maximum_id = "o".repeat(MAX_COOPERATION_OPERATION_ID_BYTES);
        let maximum_participant = CooperationParticipantState {
            actor_id: u64::MAX,
            role: CooperationRole::Runner,
            current_health: u32::MAX,
            max_health: u32::MAX,
            life: CooperationLife::Defeated,
        };
        let mut operation = cooperation_operation_state();
        operation.catalog_revision = "c".repeat(MAX_COOPERATION_OPERATION_ID_BYTES);
        operation.start_operation_id = maximum_id.clone();
        operation.participants = vec![maximum_participant.clone(); MAX_COOPERATION_PARTICIPANTS];
        operation.targets = vec![
            CooperationTargetState {
                target: CooperationTarget::RelayConsole,
                position: [i32::MAX; 3],
            };
            MAX_COOPERATION_TARGETS
        ];
        operation.observed_elapsed_ms = u64::MAX;
        let state = CooperationState {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: maximum_message.clone(),
            session_id: "s".repeat(64),
            activity_id: "a".repeat(MAX_COOPERATION_OPERATION_ID_BYTES),
            phase: CooperationPhase::EncounterActive,
            participant_actor_ids: vec![u64::MAX; MAX_COOPERATION_PARTICIPANTS],
            capable_actor_ids: vec![u64::MAX; MAX_COOPERATION_PARTICIPANTS],
            all_capable: true,
            operation: Some(operation.clone()),
        };
        let revive = operation.revive.clone();
        let ping = operation.ping.clone();
        let mut summary = cooperation_summary();
        summary.session_id = "s".repeat(64);
        summary.catalog_revision = "c".repeat(MAX_COOPERATION_OPERATION_ID_BYTES);
        summary.start_operation_id = maximum_id.clone();
        summary.participants = vec![maximum_participant; MAX_COOPERATION_PARTICIPANTS];
        summary.grants = vec![
            CooperationGrant {
                actor_id: u64::MAX,
                item_id: "i".repeat(MAX_COOPERATION_OPERATION_ID_BYTES),
                item_quantity: u32::MAX,
                experience: u64::MAX,
            };
            MAX_COOPERATION_GRANTS
        ];
        summary.no_reward_reason = Some(CooperationNoRewardReason::ParticipantDisconnected);

        for response in [
            ServerMessage::CooperationState(state),
            ServerMessage::CooperationStartResult(CooperationStartResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: true,
                replayed: true,
                message: maximum_message.clone(),
                session_id: "s".repeat(64),
                operation_id: maximum_id.clone(),
                operation: Some(operation),
            }),
            ServerMessage::CooperationPingResult(CooperationPingResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: true,
                replayed: true,
                message: maximum_message.clone(),
                session_id: "s".repeat(64),
                operation_id: maximum_id.clone(),
                ping,
            }),
            ServerMessage::CooperationLifeState(CooperationLifeState {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                session_id: "s".repeat(64),
                actor_id: u64::MAX,
                role: CooperationRole::Runner,
                source_actor_id: Some(u64::MAX),
                cause: CooperationLifeCause::Warden,
                elapsed_ms: u64::MAX,
                health_before: u32::MAX,
                health_after: 0,
                max_health: u32::MAX,
                life_before: CooperationLife::Active,
                life_after: CooperationLife::Defeated,
            }),
            ServerMessage::CooperationReviveResult(CooperationReviveResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: true,
                replayed: true,
                message: maximum_message,
                session_id: "s".repeat(64),
                operation_id: maximum_id,
                status: CooperationReviveStatus::Replayed,
                revive,
            }),
            ServerMessage::CooperationOperationSummary(summary),
        ] {
            let mut frame = Vec::new();
            write_message(&mut frame, &response)
                .expect("maximum cooperation response should encode");
            assert!(frame.len() - 4 < MAX_COOPERATION_WIRE_PAYLOAD_SIZE);
            assert!(frame.len() <= MAX_FRAME_SIZE + 4);
        }
    }

    #[test]
    fn cooperation_unknown_fields_and_invalid_enums_are_rejected() {
        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum UnknownClientMessage {
            CooperationStartIntent(UnknownCooperationStartIntent),
        }

        #[derive(Serialize)]
        struct UnknownCooperationStartIntent {
            operation_id: String,
            participant_actor_ids: Vec<u64>,
        }

        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum InvalidServerMessage {
            CooperationState(InvalidCooperationState),
        }

        #[derive(Serialize)]
        struct InvalidCooperationState {
            schema_version: u16,
            accepted: bool,
            message: String,
            session_id: String,
            activity_id: String,
            phase: String,
            participant_actor_ids: Vec<u64>,
            capable_actor_ids: Vec<u64>,
            all_capable: bool,
            operation: Option<CooperationOperationState>,
        }

        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum UnknownNestedServerMessage {
            CooperationState(UnknownNestedCooperationState),
        }

        #[derive(Serialize)]
        struct UnknownNestedCooperationState {
            schema_version: u16,
            accepted: bool,
            message: String,
            session_id: String,
            activity_id: String,
            phase: CooperationPhase,
            participant_actor_ids: Vec<u64>,
            capable_actor_ids: Vec<u64>,
            all_capable: bool,
            operation: Option<UnknownOperationState>,
        }

        #[derive(Serialize)]
        struct UnknownOperationState {
            #[serde(flatten)]
            state: CooperationOperationState,
            client_owned_timer_ms: u64,
        }

        let payload = rmp_serde::to_vec_named(&UnknownClientMessage::CooperationStartIntent(
            UnknownCooperationStartIntent {
                operation_id: "cooperation-start-1".to_owned(),
                participant_actor_ids: vec![1, 2],
            },
        ))
        .expect("unknown client field fixture should encode");
        assert!(rmp_serde::from_slice::<ClientMessage>(&payload).is_err());

        let invalid = InvalidCooperationState {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: String::new(),
            session_id: "session".to_owned(),
            activity_id: "relay_awakening".to_owned(),
            phase: "client_authored_phase".to_owned(),
            participant_actor_ids: vec![1, 2],
            capable_actor_ids: vec![1, 2],
            all_capable: true,
            operation: None,
        };
        let payload = rmp_serde::to_vec_named(&InvalidServerMessage::CooperationState(invalid))
            .expect("invalid cooperation enum fixture should encode");
        assert!(rmp_serde::from_slice::<ServerMessage>(&payload).is_err());

        let nested = UnknownNestedCooperationState {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: String::new(),
            session_id: "session".to_owned(),
            activity_id: "relay_awakening".to_owned(),
            phase: CooperationPhase::EncounterActive,
            participant_actor_ids: vec![1, 2],
            capable_actor_ids: vec![1, 2],
            all_capable: true,
            operation: Some(UnknownOperationState {
                state: cooperation_operation_state(),
                client_owned_timer_ms: 1,
            }),
        };
        let payload =
            rmp_serde::to_vec_named(&UnknownNestedServerMessage::CooperationState(nested))
                .expect("unknown nested cooperation field fixture should encode");
        assert!(rmp_serde::from_slice::<ServerMessage>(&payload).is_err());
    }
}
