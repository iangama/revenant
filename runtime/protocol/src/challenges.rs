use serde::{Deserialize, Serialize};

/// Board query after authentication and character listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeStateRequest {
    pub character_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeJoinRequest {
    pub character_id: String,
    pub operation_id: String,
    pub expected_revision: u64,
    pub contract_id: String,
    pub retry_run_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeAbandonIntent {
    pub operation_id: String,
    pub expected_revision: u64,
    pub run_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeActionResult {
    pub operation_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeContract {
    pub contract_id: String,
    pub contract_revision: String,
    pub gameplay_revision: String,
    pub seed: u64,
    pub objective_count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeEquipment {
    pub loadout_revision: u64,
    pub catalog_revision: String,
    pub weapon_id: String,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeRun {
    pub run_id: u64,
    pub contract: ChallengeContract,
    pub equipment: ChallengeEquipment,
    pub objectives: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeResult {
    pub run: ChallengeRun,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeSnapshot {
    pub character_id: String,
    pub policy_revision: String,
    pub state_revision: u64,
    pub contracts: Vec<ChallengeContract>,
    pub active: Option<ChallengeRun>,
    pub last_result: Option<ChallengeResult>,
    pub records: Vec<ChallengeRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<ChallengeVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mastery: Option<Box<ChallengeMasteryArchive>>,
}

/// Opt-in v9 board projection. Proofs remain in the immutable challenge journal;
/// this archive never changes equipment, XP or the challenge gameplay policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeMasteryArchive {
    pub revision: String,
    pub records: Vec<ChallengeMasteryRecord>,
    pub badges: Vec<ChallengeBadgeRecord>,
    pub last_attempt: Option<ChallengeMasteryAttempt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeMasteryRecord {
    pub goal: String,
    pub run_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeBadgeRecord {
    pub badge: String,
    pub run_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeMasteryAttempt {
    pub run_id: u64,
    pub assessments: Vec<ChallengeMasteryAssessment>,
    pub route_moves: Option<u64>,
    pub prism_damage: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeMasteryAssessment {
    pub goal: String,
    pub reason: String,
}

/// Committed Last Reserve world state, sent only in an opted-in solo v6 run.
/// Hold durations describe the last accepted server observation, not a client clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeSurvivalState {
    pub run_id: u64,
    pub sequence: u64,
    pub player_actor_id: u64,
    pub phase: String,
    pub health: u32,
    pub max_health: u32,
    pub reserve_used: bool,
    pub recovered_health: u32,
    pub charge_remaining_ms: Option<u64>,
    pub recovery_remaining_ms: Option<u64>,
}

/// Stage/transfer facts for the opted-in solo gauntlet; no predicted healing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeGauntletState {
    pub run_id: u64,
    pub sequence: u64,
    pub player_actor_id: u64,
    pub stage: u8,
    pub phase: String,
    pub health: u32,
    pub max_health: u32,
    pub recovered_health: u32,
    pub transfer_remaining_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<ChallengeGauntletModifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeVariant {
    pub contract_id: String,
    pub preset_id: String,
    pub time_goal_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeGauntletModifier {
    pub preset_id: String,
    pub stage_index: u8,
    pub reserve_used: bool,
    pub reserve_remaining_ms: Option<u64>,
    pub elapsed_ms: u64,
}
