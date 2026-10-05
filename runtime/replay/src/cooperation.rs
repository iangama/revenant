use std::collections::{BTreeMap, BTreeSet};

use revenant_cooperation::{
    CombatProfile, CooperationError, CooperationState, LifeState, Phase, Role, Target,
    TerminalOutcome, CATALOG_REVISION, MAX_REVIVE_DISTANCE_SQUARED, OPERATION_DURATION_MS,
    PARTICIPANT_COUNT, PING_TTL_MS, REVIVE_CHANNEL_MS, REVIVE_HEALTH, REVIVE_WINDOW_MS,
    SUCCESS_REWARD,
};
use serde::{Deserialize, Serialize};

use super::{
    decode_module_payload, validate_module_state, DecodedModuleReplayPayload, ReplayError,
    ReplayEvent, ReplayEventKind, ReplayModuleStateEvidence, ReplayProtocolGeneration,
};

pub const COOPERATION_REPLAY_SCHEMA_VERSION: u8 = 1;
pub const MAX_COOPERATION_REPLAY_PAYLOAD_BYTES: usize = 8_192;
const ACTIVITY_ID: &str = "relay_awakening";
const RELAY_CORE_FRAGMENT: &str = "relay_core_fragment";
const ANCHOR_POSITION: [i32; 3] = [3, 0, 3];
const RUNNER_POSITION: [i32; 3] = [4, 0, 3];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCooperationParticipantEvidence {
    pub participant_id: String,
    pub character_id: String,
    pub actor_id: u64,
    pub role: Role,
    pub weapon_item_id: String,
    pub module_state: ReplayModuleStateEvidence,
    pub admitted_health: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCooperationTargetEvidence {
    pub target: Target,
    pub position: [i32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCooperationTimingEvidence {
    pub operation_duration_ms: u64,
    pub ping_ttl_ms: u64,
    pub revive_window_ms: u64,
    pub revive_channel_ms: u64,
    pub revive_health: u32,
    pub maximum_distance_squared: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCooperationRewardEvidence {
    pub item_id: String,
    pub item_quantity: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationStartedPayloadV1 {
    pub schema_version: u8,
    pub activity_id: String,
    pub catalog_revision: String,
    pub start_operation_id: String,
    pub participants: Vec<ReplayCooperationParticipantEvidence>,
    pub anchor_target: ReplayCooperationTargetEvidence,
    pub runner_target: ReplayCooperationTargetEvidence,
    pub timing: ReplayCooperationTimingEvidence,
    pub reward: ReplayCooperationRewardEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationPingedPayloadV1 {
    pub schema_version: u8,
    pub catalog_revision: String,
    pub start_operation_id: String,
    pub anchor_elapsed_ms: u64,
    pub ping_operation_id: String,
    pub source_participant_id: String,
    pub source_actor_id: u64,
    pub target: Target,
    pub ping_elapsed_ms: u64,
    pub expires_elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayCooperationHazard {
    RelayFeedback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDownedPayloadV1 {
    pub schema_version: u8,
    pub catalog_revision: String,
    pub start_operation_id: String,
    pub participant_id: String,
    pub actor_id: u64,
    pub role: Role,
    pub target: Target,
    pub runner_elapsed_ms: u64,
    pub hazard: ReplayCooperationHazard,
    pub health_before: u32,
    pub damage: u32,
    pub health_after: u32,
    pub life_before: LifeState,
    pub life_after: LifeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerRevivedPayloadV1 {
    pub schema_version: u8,
    pub catalog_revision: String,
    pub start_operation_id: String,
    pub revive_operation_id: String,
    pub source_participant_id: String,
    pub source_actor_id: u64,
    pub target_participant_id: String,
    pub target_actor_id: u64,
    pub target_role: Role,
    pub revive_started_elapsed_ms: u64,
    pub revive_completed_elapsed_ms: u64,
    pub channel_duration_ms: u64,
    pub maximum_distance_squared: i64,
    pub health_before: u32,
    pub health_after: u32,
    pub life_before: LifeState,
    pub life_after: LifeState,
    pub revive_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct ReplayCooperationContributionEvidence {
    pub anchor_arrived: bool,
    pub anchor_elapsed_ms: Option<u64>,
    pub pinged: bool,
    pub ping_operation_id: Option<String>,
    pub ping_elapsed_ms: Option<u64>,
    pub runner_arrived: bool,
    pub runner_elapsed_ms: Option<u64>,
    pub downed_elapsed_ms: Option<u64>,
    pub revive_operation_id: Option<String>,
    pub revive_started_elapsed_ms: Option<u64>,
    pub revived: bool,
    pub revive_completed_elapsed_ms: Option<u64>,
    pub revive_count: u8,
    pub warden_completed: bool,
    pub warden_elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCooperationTerminalParticipantEvidence {
    pub participant_id: String,
    pub character_id: String,
    pub actor_id: u64,
    pub role: Role,
    pub current_health: u32,
    pub life: LifeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCooperationGrantEvidence {
    pub participant_id: String,
    pub character_id: String,
    pub actor_id: u64,
    pub item_id: String,
    pub item_quantity: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationTerminalPayloadV1 {
    pub schema_version: u8,
    pub catalog_revision: String,
    pub start_operation_id: String,
    pub last_nonterminal_phase: Phase,
    pub contributions: ReplayCooperationContributionEvidence,
    pub participants: Vec<ReplayCooperationTerminalParticipantEvidence>,
    pub outcome: TerminalOutcome,
    pub subject_role: Option<Role>,
    pub terminal_elapsed_ms: u64,
    pub grants: Vec<ReplayCooperationGrantEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event_type", content = "payload", rename_all = "snake_case")]
pub enum DecodedCooperationReplayPayload {
    CooperationStarted(CooperationStartedPayloadV1),
    CooperationPinged(CooperationPingedPayloadV1),
    PlayerDowned(PlayerDownedPayloadV1),
    PlayerRevived(PlayerRevivedPayloadV1),
    CooperationSucceeded(CooperationTerminalPayloadV1),
    CooperationFailed(CooperationTerminalPayloadV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconstructedCooperationState {
    Legacy,
    Active,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedCooperationReplay {
    pub state: ReconstructedCooperationState,
    pub started: Option<CooperationStartedPayloadV1>,
    pub pinged: Option<CooperationPingedPayloadV1>,
    pub downed: Option<PlayerDownedPayloadV1>,
    pub revived: Option<PlayerRevivedPayloadV1>,
    pub terminal: Option<CooperationTerminalPayloadV1>,
}

impl Default for ReconstructedCooperationReplay {
    fn default() -> Self {
        Self {
            state: ReconstructedCooperationState::Legacy,
            started: None,
            pinged: None,
            downed: None,
            revived: None,
            terminal: None,
        }
    }
}

impl ReconstructedCooperationReplay {
    #[must_use]
    pub fn last_phase(&self) -> Option<Phase> {
        if let Some(terminal) = &self.terminal {
            return Some(terminal.last_nonterminal_phase);
        }
        if self.revived.is_some() {
            Some(Phase::EncounterActive)
        } else if self.downed.is_some() {
            Some(Phase::RunnerDowned)
        } else if self.pinged.is_some() {
            Some(Phase::AwaitingRunner)
        } else {
            self.started.as_ref().map(|_| Phase::AwaitingAnchor)
        }
    }

    #[must_use]
    pub fn contribution_count(&self) -> usize {
        if let Some(terminal) = &self.terminal {
            let evidence = &terminal.contributions;
            return [
                evidence.anchor_arrived,
                evidence.pinged,
                evidence.runner_arrived,
                evidence.revived,
                evidence.warden_completed,
            ]
            .into_iter()
            .filter(|value| *value)
            .count();
        }
        if self.revived.is_some() {
            4
        } else if self.downed.is_some() {
            3
        } else if self.pinged.is_some() {
            2
        } else {
            0
        }
    }
}

/// Encodes and validates one immutable cooperation admission payload.
///
/// # Errors
///
/// Returns an error for invalid bounded identity, constants, module/profile
/// evidence, health, schema, or encoded size.
pub fn encode_cooperation_started(
    payload: &CooperationStartedPayloadV1,
) -> Result<String, ReplayError> {
    validate_started_payload(payload)?;
    encode_payload(payload)
}

/// Encodes one accepted anchor-arrival and ping payload.
///
/// # Errors
///
/// Returns an error when identity, target, timing, schema, or size disagrees
/// with the immutable start.
pub fn encode_cooperation_pinged(
    payload: &CooperationPingedPayloadV1,
    started: &CooperationStartedPayloadV1,
) -> Result<String, ReplayError> {
    validate_pinged_payload(payload, started)?;
    encode_payload(payload)
}

/// Encodes one scripted runner-downing payload.
///
/// # Errors
///
/// Returns an error for lifecycle, identity, timing, health/life, schema, or
/// size drift.
pub fn encode_player_downed(
    payload: &PlayerDownedPayloadV1,
    started: &CooperationStartedPayloadV1,
    pinged: &CooperationPingedPayloadV1,
) -> Result<String, ReplayError> {
    validate_downed_payload(payload, started, pinged)?;
    encode_payload(payload)
}

/// Encodes one successfully completed revive payload.
///
/// # Errors
///
/// Returns an error for lifecycle, identity, channel/window, health/life,
/// schema, or size drift.
pub fn encode_player_revived(
    payload: &PlayerRevivedPayloadV1,
    started: &CooperationStartedPayloadV1,
    downed: &PlayerDownedPayloadV1,
) -> Result<String, ReplayError> {
    validate_revived_payload(payload, started, downed)?;
    encode_payload(payload)
}

/// Encodes one successful terminal payload against its exact replay prefix.
///
/// # Errors
///
/// Returns an error for invalid contribution, participant, reward, lifecycle,
/// schema, or size evidence.
pub fn encode_cooperation_succeeded(
    payload: &CooperationTerminalPayloadV1,
    started: &CooperationStartedPayloadV1,
    prefix: CooperationReplayPrefix<'_>,
) -> Result<String, ReplayError> {
    validate_terminal_payload(
        ReplayEventKind::CooperationSucceeded,
        payload,
        started,
        prefix,
    )?;
    encode_payload(payload)
}

/// Encodes one failed terminal payload against its exact replay prefix.
///
/// # Errors
///
/// Returns an error for invalid outcome/subject, contribution, participant,
/// deadline, schema, or size evidence.
pub fn encode_cooperation_failed(
    payload: &CooperationTerminalPayloadV1,
    started: &CooperationStartedPayloadV1,
    prefix: CooperationReplayPrefix<'_>,
) -> Result<String, ReplayError> {
    validate_terminal_payload(ReplayEventKind::CooperationFailed, payload, started, prefix)?;
    encode_payload(payload)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CooperationReplayPrefix<'a> {
    pub pinged: Option<&'a CooperationPingedPayloadV1>,
    pub downed: Option<&'a PlayerDownedPayloadV1>,
    pub revived: Option<&'a PlayerRevivedPayloadV1>,
}

/// Decodes one bounded cooperation event payload for read-only display.
///
/// # Errors
///
/// Returns an error for oversized, malformed, wrong-version, or invalid
/// cooperation evidence. Non-cooperation kinds return `Ok(None)`.
pub fn decode_cooperation_payload(
    kind: ReplayEventKind,
    payload: &str,
) -> Result<Option<DecodedCooperationReplayPayload>, ReplayError> {
    if !kind.is_cooperation_event() {
        return Ok(None);
    }
    validate_payload_size(payload)?;
    let decoded = match kind {
        ReplayEventKind::CooperationStarted => {
            let value: CooperationStartedPayloadV1 = decode_json(payload)?;
            validate_started_payload(&value)?;
            DecodedCooperationReplayPayload::CooperationStarted(value)
        }
        ReplayEventKind::CooperationPinged => {
            DecodedCooperationReplayPayload::CooperationPinged(decode_json(payload)?)
        }
        ReplayEventKind::PlayerDowned => {
            DecodedCooperationReplayPayload::PlayerDowned(decode_json(payload)?)
        }
        ReplayEventKind::PlayerRevived => {
            DecodedCooperationReplayPayload::PlayerRevived(decode_json(payload)?)
        }
        ReplayEventKind::CooperationSucceeded => {
            DecodedCooperationReplayPayload::CooperationSucceeded(decode_json(payload)?)
        }
        ReplayEventKind::CooperationFailed => {
            DecodedCooperationReplayPayload::CooperationFailed(decode_json(payload)?)
        }
        _ => return Ok(None),
    };
    Ok(Some(decoded))
}

pub(super) fn reconstruct_cooperation_replay(
    events: &[ReplayEvent],
) -> Result<ReconstructedCooperationReplay, ReplayError> {
    let mut accumulator = CooperationReplayAccumulator::default();
    for event in events {
        accumulator.apply(event)?;
    }
    accumulator.finish()
}

#[derive(Debug, Clone)]
struct SnapshotEvidence {
    account_id: String,
    actor_id: u64,
    generation: ReplayProtocolGeneration,
    state: ReplayModuleStateEvidence,
}

#[derive(Debug, Default)]
struct CooperationReplayAccumulator {
    snapshots: BTreeMap<String, SnapshotEvidence>,
    snapshot_order: Vec<String>,
    started: Option<CooperationStartedPayloadV1>,
    pinged: Option<CooperationPingedPayloadV1>,
    downed: Option<PlayerDownedPayloadV1>,
    revived: Option<PlayerRevivedPayloadV1>,
    terminal: Option<CooperationTerminalPayloadV1>,
    activity_completions: usize,
    loot_grants: BTreeMap<String, (u64, u32)>,
    progression_grants: BTreeMap<String, (u64, u64)>,
    saw_activity_started: bool,
    enemy_defeats: usize,
    boss_spawned: bool,
    saw_route_event: bool,
}

impl CooperationReplayAccumulator {
    fn apply(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        if event.kind.is_route_event() {
            self.saw_route_event = true;
            if self.started.is_some() {
                return Err(cooperation_error("route and cooperation replay coexist"));
            }
        }
        if event.kind == ReplayEventKind::ModuleStateSnapshot {
            if self.started.is_some() {
                return Err(cooperation_error(
                    "cooperation admission snapshot appears after start",
                ));
            }
            self.record_snapshot(event)?;
        }
        if self.terminal.is_some()
            && (event.kind.is_cooperation_event()
                || matches!(
                    event.kind,
                    ReplayEventKind::ActivityCompleted
                        | ReplayEventKind::LootGranted
                        | ReplayEventKind::ProgressionGranted
                ))
        {
            return Err(cooperation_error(
                "cooperation or generic reward evidence appears after terminal",
            ));
        }
        match event.kind {
            ReplayEventKind::ActivityStarted => self.saw_activity_started = true,
            ReplayEventKind::EnemyDied => {
                self.enemy_defeats = self
                    .enemy_defeats
                    .checked_add(1)
                    .ok_or_else(|| cooperation_error("enemy defeat count overflowed"))?;
            }
            ReplayEventKind::BossSpawned => self.boss_spawned = true,
            _ => {}
        }
        if self.started.is_some() && self.terminal.is_none() {
            self.record_generic_event(event)?;
        }
        self.record_cooperation_event(event)
    }

    fn record_snapshot(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        let Some(DecodedModuleReplayPayload::ModuleStateSnapshot(payload)) =
            decode_module_payload(event.kind, &event.payload)?
        else {
            return Err(cooperation_error("module snapshot decoded incorrectly"));
        };
        let actor_id = event
            .actor_id
            .ok_or_else(|| cooperation_error("module snapshot has no actor"))?;
        if self.snapshots.contains_key(&payload.character_id) {
            return Err(cooperation_error("module snapshot is duplicated"));
        }
        self.snapshot_order.push(payload.character_id.clone());
        self.snapshots.insert(
            payload.character_id,
            SnapshotEvidence {
                account_id: event.account_id.clone(),
                actor_id,
                generation: payload.protocol_generation,
                state: payload.state,
            },
        );
        Ok(())
    }

    fn record_generic_event(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        let started = self
            .started
            .as_ref()
            .ok_or_else(|| cooperation_error("cooperation start disappeared"))?;
        match event.kind {
            ReplayEventKind::ActivityCompleted => {
                let anchor = &started.participants[Role::Anchor.index()];
                if event.activity_id.as_deref() != Some(started.activity_id.as_str())
                    || event.account_id != anchor.participant_id
                    || event.actor_id != Some(anchor.actor_id)
                    || event.payload != "activity completed"
                {
                    return Err(cooperation_error(
                        "cooperation completion row disagrees with anchor",
                    ));
                }
                self.activity_completions = self
                    .activity_completions
                    .checked_add(1)
                    .ok_or_else(|| cooperation_error("completion count overflowed"))?;
            }
            ReplayEventKind::LootGranted => {
                let activity_id = started.activity_id.clone();
                self.record_loot(event, &activity_id)?;
            }
            ReplayEventKind::ProgressionGranted => {
                let activity_id = started.activity_id.clone();
                self.record_progression(event, &activity_id)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn record_loot(&mut self, event: &ReplayEvent, activity_id: &str) -> Result<(), ReplayError> {
        if event.activity_id.as_deref() != Some(activity_id) {
            return Err(cooperation_error("cooperation loot activity is wrong"));
        }
        let quantity = fragment_quantity(&event.payload)
            .ok_or_else(|| cooperation_error("cooperation loot payload is malformed"))?;
        let actor = event
            .actor_id
            .ok_or_else(|| cooperation_error("cooperation loot has no actor"))?;
        if self
            .loot_grants
            .insert(event.account_id.clone(), (actor, quantity))
            .is_some()
        {
            return Err(cooperation_error("cooperation loot is duplicated"));
        }
        Ok(())
    }

    fn record_progression(
        &mut self,
        event: &ReplayEvent,
        activity_id: &str,
    ) -> Result<(), ReplayError> {
        if event.activity_id.as_deref() != Some(activity_id) {
            return Err(cooperation_error(
                "cooperation progression activity is wrong",
            ));
        }
        let experience = experience_quantity(&event.payload)
            .ok_or_else(|| cooperation_error("cooperation progression payload is malformed"))?;
        let actor = event
            .actor_id
            .ok_or_else(|| cooperation_error("cooperation progression has no actor"))?;
        if self
            .progression_grants
            .insert(event.account_id.clone(), (actor, experience))
            .is_some()
        {
            return Err(cooperation_error("cooperation progression is duplicated"));
        }
        Ok(())
    }

    fn record_cooperation_event(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        match event.kind {
            ReplayEventKind::CooperationStarted => self.record_started(event),
            ReplayEventKind::CooperationPinged => self.record_pinged(event),
            ReplayEventKind::PlayerDowned => self.record_downed(event),
            ReplayEventKind::PlayerRevived => self.record_revived(event),
            ReplayEventKind::CooperationSucceeded | ReplayEventKind::CooperationFailed => {
                self.record_terminal(event)
            }
            _ => Ok(()),
        }
    }

    fn record_started(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        if self.started.is_some() || self.saw_route_event || !self.saw_activity_started {
            return Err(cooperation_error(
                "cooperation start is duplicate, routed, or precedes activity",
            ));
        }
        if self.enemy_defeats != 1 || self.boss_spawned {
            return Err(cooperation_error(
                "cooperation start is outside the Drone-to-Warden boundary",
            ));
        }
        let Some(DecodedCooperationReplayPayload::CooperationStarted(payload)) =
            decode_cooperation_payload(event.kind, &event.payload)?
        else {
            return Err(cooperation_error("cooperation start decoded incorrectly"));
        };
        validate_started_event(event, &payload, &self.snapshots, &self.snapshot_order)?;
        self.started = Some(payload);
        Ok(())
    }

    fn record_pinged(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        if self.pinged.is_some() || self.downed.is_some() || self.revived.is_some() {
            return Err(cooperation_error("cooperation ping is duplicate or late"));
        }
        let started = self
            .started
            .as_ref()
            .ok_or_else(|| cooperation_error("cooperation ping precedes start"))?;
        let Some(DecodedCooperationReplayPayload::CooperationPinged(payload)) =
            decode_cooperation_payload(event.kind, &event.payload)?
        else {
            return Err(cooperation_error("cooperation ping decoded incorrectly"));
        };
        validate_pinged_payload(&payload, started)?;
        validate_event_identity(event, started, Role::Anchor)?;
        self.pinged = Some(payload);
        Ok(())
    }

    fn record_downed(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        if self.downed.is_some() || self.revived.is_some() {
            return Err(cooperation_error("player downing is duplicate or late"));
        }
        let started = self
            .started
            .as_ref()
            .ok_or_else(|| cooperation_error("player downing precedes start"))?;
        let pinged = self
            .pinged
            .as_ref()
            .ok_or_else(|| cooperation_error("player downing precedes ping"))?;
        let Some(DecodedCooperationReplayPayload::PlayerDowned(payload)) =
            decode_cooperation_payload(event.kind, &event.payload)?
        else {
            return Err(cooperation_error("player downing decoded incorrectly"));
        };
        validate_downed_payload(&payload, started, pinged)?;
        validate_event_identity(event, started, Role::Runner)?;
        self.downed = Some(payload);
        Ok(())
    }

    fn record_revived(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        if self.revived.is_some() {
            return Err(cooperation_error("player revive is duplicated"));
        }
        let started = self
            .started
            .as_ref()
            .ok_or_else(|| cooperation_error("player revive precedes start"))?;
        let downed = self
            .downed
            .as_ref()
            .ok_or_else(|| cooperation_error("player revive precedes downing"))?;
        let Some(DecodedCooperationReplayPayload::PlayerRevived(payload)) =
            decode_cooperation_payload(event.kind, &event.payload)?
        else {
            return Err(cooperation_error("player revive decoded incorrectly"));
        };
        validate_revived_payload(&payload, started, downed)?;
        validate_event_identity(event, started, Role::Runner)?;
        self.revived = Some(payload);
        Ok(())
    }

    fn record_terminal(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        let started = self
            .started
            .as_ref()
            .ok_or_else(|| cooperation_error("cooperation terminal precedes start"))?;
        let Some(
            DecodedCooperationReplayPayload::CooperationSucceeded(payload)
            | DecodedCooperationReplayPayload::CooperationFailed(payload),
        ) = decode_cooperation_payload(event.kind, &event.payload)?
        else {
            return Err(cooperation_error(
                "cooperation terminal decoded incorrectly",
            ));
        };
        validate_terminal_payload(event.kind, &payload, started, self.prefix())?;
        validate_event_identity(event, started, Role::Anchor)?;
        validate_generic_terminal(
            event.kind,
            started,
            &payload,
            self.activity_completions,
            &self.loot_grants,
            &self.progression_grants,
        )?;
        self.terminal = Some(payload);
        Ok(())
    }

    fn prefix(&self) -> CooperationReplayPrefix<'_> {
        CooperationReplayPrefix {
            pinged: self.pinged.as_ref(),
            downed: self.downed.as_ref(),
            revived: self.revived.as_ref(),
        }
    }

    fn finish(self) -> Result<ReconstructedCooperationReplay, ReplayError> {
        let Some(started) = self.started else {
            return Ok(ReconstructedCooperationReplay::default());
        };
        if self.terminal.is_none()
            && (self.activity_completions != 0
                || !self.loot_grants.is_empty()
                || !self.progression_grants.is_empty())
        {
            return Err(cooperation_error(
                "incomplete cooperation has completion or reward evidence",
            ));
        }
        let state = match self.terminal.as_ref().map(|terminal| terminal.outcome) {
            Some(TerminalOutcome::Succeeded) => ReconstructedCooperationState::Succeeded,
            Some(_) => ReconstructedCooperationState::Failed,
            None => ReconstructedCooperationState::Active,
        };
        Ok(ReconstructedCooperationReplay {
            state,
            started: Some(started),
            pinged: self.pinged,
            downed: self.downed,
            revived: self.revived,
            terminal: self.terminal,
        })
    }
}

fn validate_started_event(
    event: &ReplayEvent,
    payload: &CooperationStartedPayloadV1,
    snapshots: &BTreeMap<String, SnapshotEvidence>,
    snapshot_order: &[String],
) -> Result<(), ReplayError> {
    validate_event_identity(event, payload, Role::Anchor)?;
    if snapshots.len() != PARTICIPANT_COUNT
        || snapshot_order.len() != PARTICIPANT_COUNT
        || snapshot_order
            .iter()
            .zip(&payload.participants)
            .any(|(character, participant)| character != &participant.character_id)
    {
        return Err(cooperation_error(
            "cooperation participants disagree with admission order",
        ));
    }
    for participant in &payload.participants {
        let snapshot = snapshots
            .get(&participant.character_id)
            .ok_or_else(|| cooperation_error("cooperation participant has no snapshot"))?;
        if snapshot.account_id != participant.participant_id
            || snapshot.actor_id != participant.actor_id
            || snapshot.generation != ReplayProtocolGeneration::V2
            || snapshot.state != participant.module_state
        {
            return Err(cooperation_error(
                "cooperation participant disagrees with admission snapshot",
            ));
        }
    }
    Ok(())
}

fn validate_event_identity(
    event: &ReplayEvent,
    started: &CooperationStartedPayloadV1,
    role: Role,
) -> Result<(), ReplayError> {
    let participant = started
        .participants
        .get(role.index())
        .ok_or_else(|| cooperation_error("cooperation participant is missing"))?;
    if event.activity_id.as_deref() != Some(started.activity_id.as_str())
        || event.account_id != participant.participant_id
        || event.actor_id != Some(participant.actor_id)
    {
        return Err(cooperation_error(
            "cooperation replay row identity disagrees with participant",
        ));
    }
    Ok(())
}

fn validate_started_payload(payload: &CooperationStartedPayloadV1) -> Result<(), ReplayError> {
    validate_schema_revision(
        payload.schema_version,
        &payload.catalog_revision,
        &payload.start_operation_id,
    )?;
    if payload.activity_id != ACTIVITY_ID
        || payload.participants.len() != PARTICIPANT_COUNT
        || payload.anchor_target
            != (ReplayCooperationTargetEvidence {
                target: Target::RelayAnchor,
                position: ANCHOR_POSITION,
            })
        || payload.runner_target
            != (ReplayCooperationTargetEvidence {
                target: Target::RelayConsole,
                position: RUNNER_POSITION,
            })
        || payload.timing
            != (ReplayCooperationTimingEvidence {
                operation_duration_ms: OPERATION_DURATION_MS,
                ping_ttl_ms: PING_TTL_MS,
                revive_window_ms: REVIVE_WINDOW_MS,
                revive_channel_ms: REVIVE_CHANNEL_MS,
                revive_health: REVIVE_HEALTH,
                maximum_distance_squared: MAX_REVIVE_DISTANCE_SQUARED,
            })
        || payload.reward
            != (ReplayCooperationRewardEvidence {
                item_id: RELAY_CORE_FRAGMENT.to_owned(),
                item_quantity: SUCCESS_REWARD.fragments,
                experience: SUCCESS_REWARD.experience,
            })
    {
        return Err(cooperation_error(
            "cooperation start constants or cardinality drifted",
        ));
    }
    let mut accounts = BTreeSet::new();
    let mut characters = BTreeSet::new();
    let mut actors = BTreeSet::new();
    for (index, participant) in payload.participants.iter().enumerate() {
        validate_bounded_identity(&participant.participant_id, "participant")?;
        validate_bounded_identity(&participant.character_id, "character")?;
        validate_snake_identifier(&participant.weapon_item_id, "weapon")?;
        let role = Role::ALL[index];
        if participant.role != role
            || !accounts.insert(participant.participant_id.as_str())
            || !characters.insert(participant.character_id.as_str())
            || !actors.insert(participant.actor_id)
            || !participant.module_state.module_effects_active
        {
            return Err(cooperation_error(
                "cooperation participant identity, role, or generation is invalid",
            ));
        }
        validate_module_state(&participant.module_state)?;
        let profile = participant_profile(participant)?;
        if participant.admitted_health == 0 || participant.admitted_health > profile.max_health {
            return Err(cooperation_error(
                "cooperation admitted health is outside profile bounds",
            ));
        }
    }
    Ok(())
}

fn validate_pinged_payload(
    payload: &CooperationPingedPayloadV1,
    started: &CooperationStartedPayloadV1,
) -> Result<(), ReplayError> {
    validate_schema_revision(
        payload.schema_version,
        &payload.catalog_revision,
        &payload.start_operation_id,
    )?;
    let anchor = &started.participants[Role::Anchor.index()];
    let expires = payload
        .ping_elapsed_ms
        .checked_add(PING_TTL_MS)
        .ok_or_else(|| cooperation_error("ping expiry overflowed"))?;
    validate_operation_id(&payload.ping_operation_id)?;
    if payload.catalog_revision != started.catalog_revision
        || payload.start_operation_id != started.start_operation_id
        || payload.source_participant_id != anchor.participant_id
        || payload.source_actor_id != anchor.actor_id
        || payload.target != Target::RelayConsole
        || payload.anchor_elapsed_ms > payload.ping_elapsed_ms
        || payload.ping_elapsed_ms > OPERATION_DURATION_MS
        || payload.expires_elapsed_ms != expires
    {
        return Err(cooperation_error("cooperation ping evidence is invalid"));
    }
    Ok(())
}

fn validate_downed_payload(
    payload: &PlayerDownedPayloadV1,
    started: &CooperationStartedPayloadV1,
    pinged: &CooperationPingedPayloadV1,
) -> Result<(), ReplayError> {
    validate_schema_revision(
        payload.schema_version,
        &payload.catalog_revision,
        &payload.start_operation_id,
    )?;
    let runner = &started.participants[Role::Runner.index()];
    if payload.catalog_revision != started.catalog_revision
        || payload.start_operation_id != started.start_operation_id
        || payload.participant_id != runner.participant_id
        || payload.actor_id != runner.actor_id
        || payload.role != Role::Runner
        || payload.target != Target::RelayConsole
        || payload.runner_elapsed_ms < pinged.ping_elapsed_ms
        || payload.runner_elapsed_ms > pinged.expires_elapsed_ms
        || payload.runner_elapsed_ms > OPERATION_DURATION_MS
        || payload.health_before != runner.admitted_health
        || payload.damage != payload.health_before
        || payload.health_after != 0
        || payload.life_before != LifeState::Active
        || payload.life_after != LifeState::Downed
    {
        return Err(cooperation_error("player downing evidence is invalid"));
    }
    Ok(())
}

fn validate_revived_payload(
    payload: &PlayerRevivedPayloadV1,
    started: &CooperationStartedPayloadV1,
    downed: &PlayerDownedPayloadV1,
) -> Result<(), ReplayError> {
    validate_schema_revision(
        payload.schema_version,
        &payload.catalog_revision,
        &payload.start_operation_id,
    )?;
    validate_operation_id(&payload.revive_operation_id)?;
    let anchor = &started.participants[Role::Anchor.index()];
    let runner = &started.participants[Role::Runner.index()];
    let channel_end = payload
        .revive_started_elapsed_ms
        .checked_add(REVIVE_CHANNEL_MS)
        .ok_or_else(|| cooperation_error("revive channel overflowed"))?;
    let window_end = downed
        .runner_elapsed_ms
        .checked_add(REVIVE_WINDOW_MS)
        .ok_or_else(|| cooperation_error("revive window overflowed"))?;
    if payload.catalog_revision != started.catalog_revision
        || payload.start_operation_id != started.start_operation_id
        || payload.source_participant_id != anchor.participant_id
        || payload.source_actor_id != anchor.actor_id
        || payload.target_participant_id != runner.participant_id
        || payload.target_actor_id != runner.actor_id
        || payload.target_role != Role::Runner
        || payload.revive_started_elapsed_ms < downed.runner_elapsed_ms
        || payload.revive_started_elapsed_ms > window_end
        || payload.revive_completed_elapsed_ms < channel_end
        || payload.revive_completed_elapsed_ms > window_end
        || payload.revive_completed_elapsed_ms > OPERATION_DURATION_MS
        || payload.channel_duration_ms != REVIVE_CHANNEL_MS
        || payload.maximum_distance_squared != MAX_REVIVE_DISTANCE_SQUARED
        || payload.health_before != 0
        || payload.health_after != REVIVE_HEALTH
        || payload.life_before != LifeState::Downed
        || payload.life_after != LifeState::Active
        || payload.revive_count != 1
    {
        return Err(cooperation_error("player revive evidence is invalid"));
    }
    Ok(())
}

fn validate_terminal_payload(
    kind: ReplayEventKind,
    payload: &CooperationTerminalPayloadV1,
    started: &CooperationStartedPayloadV1,
    prefix: CooperationReplayPrefix<'_>,
) -> Result<(), ReplayError> {
    validate_schema_revision(
        payload.schema_version,
        &payload.catalog_revision,
        &payload.start_operation_id,
    )?;
    if payload.catalog_revision != started.catalog_revision
        || payload.start_operation_id != started.start_operation_id
        || payload.participants.len() != PARTICIPANT_COUNT
    {
        return Err(cooperation_error("cooperation terminal identity drifted"));
    }
    let expected_success = kind == ReplayEventKind::CooperationSucceeded;
    if expected_success != (payload.outcome == TerminalOutcome::Succeeded) {
        return Err(cooperation_error(
            "cooperation terminal kind and outcome disagree",
        ));
    }
    validate_terminal_prefix(payload, started, prefix)?;
    validate_terminal_grants(payload, started)?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn validate_terminal_prefix(
    payload: &CooperationTerminalPayloadV1,
    started: &CooperationStartedPayloadV1,
    prefix: CooperationReplayPrefix<'_>,
) -> Result<(), ReplayError> {
    let evidence = &payload.contributions;
    validate_contribution_nullability(evidence)?;
    let profiles = [
        participant_profile(&started.participants[0])?,
        participant_profile(&started.participants[1])?,
    ];
    let health = [
        started.participants[0].admitted_health,
        started.participants[1].admitted_health,
    ];
    let mut state =
        CooperationState::start_with_health(&started.start_operation_id, profiles, health)
            .map_err(domain_error)?;
    if evidence.anchor_arrived {
        state
            .arrive_anchor(
                Role::Anchor,
                Target::RelayAnchor,
                evidence
                    .anchor_elapsed_ms
                    .expect("validated anchor elapsed"),
            )
            .map_err(domain_error)?;
    }
    if evidence.pinged {
        let pinged = prefix
            .pinged
            .ok_or_else(|| cooperation_error("terminal ping has no ping event"))?;
        if evidence.ping_operation_id.as_deref() != Some(pinged.ping_operation_id.as_str())
            || evidence.ping_elapsed_ms != Some(pinged.ping_elapsed_ms)
            || evidence.anchor_elapsed_ms != Some(pinged.anchor_elapsed_ms)
        {
            return Err(cooperation_error("terminal ping disagrees with prefix"));
        }
        state
            .ping(
                Role::Anchor,
                Target::RelayConsole,
                &pinged.ping_operation_id,
                pinged.ping_elapsed_ms,
            )
            .map_err(domain_error)?;
    } else if prefix.pinged.is_some() {
        return Err(cooperation_error("terminal omits accepted ping"));
    }
    if evidence.runner_arrived {
        let downed = prefix
            .downed
            .ok_or_else(|| cooperation_error("terminal downing has no event"))?;
        if evidence.runner_elapsed_ms != Some(downed.runner_elapsed_ms)
            || evidence.downed_elapsed_ms != Some(downed.runner_elapsed_ms)
        {
            return Err(cooperation_error("terminal downing disagrees with prefix"));
        }
        state
            .arrive_runner(Role::Runner, Target::RelayConsole, downed.runner_elapsed_ms)
            .map_err(domain_error)?;
    } else if prefix.downed.is_some() {
        return Err(cooperation_error("terminal omits accepted downing"));
    }
    if let (Some(operation_id), Some(started_elapsed)) = (
        evidence.revive_operation_id.as_deref(),
        evidence.revive_started_elapsed_ms,
    ) {
        state
            .start_revive(
                Role::Anchor,
                Role::Runner,
                MAX_REVIVE_DISTANCE_SQUARED,
                operation_id,
                started_elapsed,
            )
            .map_err(domain_error)?;
        if evidence.revived {
            let revived = prefix
                .revived
                .ok_or_else(|| cooperation_error("terminal revive has no event"))?;
            if operation_id != revived.revive_operation_id
                || evidence.revive_started_elapsed_ms != Some(revived.revive_started_elapsed_ms)
                || evidence.revive_completed_elapsed_ms != Some(revived.revive_completed_elapsed_ms)
            {
                return Err(cooperation_error("terminal revive disagrees with prefix"));
            }
            state
                .observe_revive(
                    Role::Anchor,
                    Role::Runner,
                    MAX_REVIVE_DISTANCE_SQUARED,
                    operation_id,
                    revived.revive_completed_elapsed_ms,
                )
                .map_err(domain_error)?;
        }
    } else if evidence.revived || prefix.revived.is_some() {
        return Err(cooperation_error("terminal revive evidence is partial"));
    }
    if state.phase() != payload.last_nonterminal_phase {
        return Err(cooperation_error(
            "terminal last phase disagrees with lifecycle",
        ));
    }
    apply_terminal(&mut state, payload)?;
    validate_terminal_state(payload, started, &state)
}

fn apply_terminal(
    state: &mut CooperationState,
    payload: &CooperationTerminalPayloadV1,
) -> Result<(), ReplayError> {
    match payload.outcome {
        TerminalOutcome::Succeeded => {
            if payload.subject_role.is_some()
                || payload.contributions.warden_elapsed_ms != Some(payload.terminal_elapsed_ms)
            {
                return Err(cooperation_error("success terminal evidence is invalid"));
            }
            state
                .complete_warden(payload.terminal_elapsed_ms)
                .map_err(domain_error)?;
        }
        TerminalOutcome::FailedPingTimeout
        | TerminalOutcome::FailedReviveTimeout
        | TerminalOutcome::FailedOperationTimeout => {
            if payload.subject_role.is_some()
                || !matches!(
                    state.observe_deadlines(payload.terminal_elapsed_ms),
                    Err(CooperationError::DeadlineExpired(outcome)) if outcome == payload.outcome
                )
            {
                return Err(cooperation_error("timeout terminal evidence is invalid"));
            }
        }
        TerminalOutcome::FailedParticipantDefeated => {
            let subject = payload
                .subject_role
                .ok_or_else(|| cooperation_error("defeat terminal has no subject"))?;
            state
                .defeat_participant(subject, payload.terminal_elapsed_ms)
                .map_err(domain_error)?;
        }
        TerminalOutcome::AbandonedDisconnect => {
            let subject = payload
                .subject_role
                .ok_or_else(|| cooperation_error("abandonment has no subject"))?;
            state
                .disconnect(subject, payload.terminal_elapsed_ms)
                .map_err(domain_error)?;
        }
    }
    Ok(())
}

fn validate_terminal_state(
    payload: &CooperationTerminalPayloadV1,
    started: &CooperationStartedPayloadV1,
    state: &CooperationState,
) -> Result<(), ReplayError> {
    let contribution = state.contributions();
    let evidence = &payload.contributions;
    if contribution.anchor_arrived != evidence.anchor_arrived
        || contribution.pinged != evidence.pinged
        || contribution.runner_arrived != evidence.runner_arrived
        || contribution.revived != evidence.revived
        || contribution.warden_completed != evidence.warden_completed
        || state.revive_count() != evidence.revive_count
        || state.terminal() != Some(payload.outcome)
    {
        return Err(cooperation_error(
            "terminal contribution state disagrees with pure domain",
        ));
    }
    for (index, (participant, actual)) in payload
        .participants
        .iter()
        .zip(state.participants())
        .enumerate()
    {
        let admitted = &started.participants[index];
        if participant.participant_id != admitted.participant_id
            || participant.character_id != admitted.character_id
            || participant.actor_id != admitted.actor_id
            || participant.role != admitted.role
            || participant.current_health != actual.current_health
            || participant.life != actual.life
        {
            return Err(cooperation_error(
                "terminal participant state disagrees with pure domain",
            ));
        }
    }
    Ok(())
}

fn validate_terminal_grants(
    payload: &CooperationTerminalPayloadV1,
    started: &CooperationStartedPayloadV1,
) -> Result<(), ReplayError> {
    if payload.outcome != TerminalOutcome::Succeeded {
        if !payload.grants.is_empty() || payload.contributions.warden_completed {
            return Err(cooperation_error("failed cooperation has success evidence"));
        }
        return Ok(());
    }
    if payload.grants.len() != PARTICIPANT_COUNT {
        return Err(cooperation_error(
            "successful cooperation grant count is wrong",
        ));
    }
    for (grant, participant) in payload.grants.iter().zip(&started.participants) {
        if grant.participant_id != participant.participant_id
            || grant.character_id != participant.character_id
            || grant.actor_id != participant.actor_id
            || grant.item_id != RELAY_CORE_FRAGMENT
            || grant.item_quantity != SUCCESS_REWARD.fragments
            || grant.experience != SUCCESS_REWARD.experience
        {
            return Err(cooperation_error("cooperation grant evidence drifted"));
        }
    }
    Ok(())
}

fn validate_generic_terminal(
    kind: ReplayEventKind,
    started: &CooperationStartedPayloadV1,
    payload: &CooperationTerminalPayloadV1,
    completions: usize,
    loot: &BTreeMap<String, (u64, u32)>,
    progression: &BTreeMap<String, (u64, u64)>,
) -> Result<(), ReplayError> {
    if kind == ReplayEventKind::CooperationFailed {
        if completions != 0 || !loot.is_empty() || !progression.is_empty() {
            return Err(cooperation_error("failed cooperation has generic rewards"));
        }
        return Ok(());
    }
    if completions != 1 || loot.len() != PARTICIPANT_COUNT || progression.len() != PARTICIPANT_COUNT
    {
        return Err(cooperation_error(
            "successful cooperation generic evidence is incomplete",
        ));
    }
    for participant in &started.participants {
        if loot.get(&participant.participant_id)
            != Some(&(participant.actor_id, SUCCESS_REWARD.fragments))
            || progression.get(&participant.participant_id)
                != Some(&(participant.actor_id, SUCCESS_REWARD.experience))
        {
            return Err(cooperation_error(
                "successful cooperation generic grant disagrees",
            ));
        }
    }
    if payload.grants.len() != PARTICIPANT_COUNT {
        return Err(cooperation_error(
            "successful cooperation grants are incomplete",
        ));
    }
    Ok(())
}

fn validate_contribution_nullability(
    evidence: &ReplayCooperationContributionEvidence,
) -> Result<(), ReplayError> {
    if evidence.anchor_arrived != evidence.anchor_elapsed_ms.is_some()
        || evidence.pinged != evidence.ping_operation_id.is_some()
        || evidence.pinged != evidence.ping_elapsed_ms.is_some()
        || evidence.runner_arrived != evidence.runner_elapsed_ms.is_some()
        || evidence.runner_arrived != evidence.downed_elapsed_ms.is_some()
        || evidence.revive_operation_id.is_some() != evidence.revive_started_elapsed_ms.is_some()
        || evidence.revived != evidence.revive_completed_elapsed_ms.is_some()
        || evidence.revived != (evidence.revive_count == 1)
        || evidence.warden_completed != evidence.warden_elapsed_ms.is_some()
    {
        return Err(cooperation_error(
            "cooperation contribution nullability is invalid",
        ));
    }
    if evidence.revive_count > 1
        || evidence.pinged && !evidence.anchor_arrived
        || evidence.runner_arrived && !evidence.pinged
        || evidence.revive_operation_id.is_some() && !evidence.runner_arrived
        || evidence.revived && evidence.revive_operation_id.is_none()
        || evidence.warden_completed && !evidence.revived
    {
        return Err(cooperation_error(
            "cooperation contribution prefix is invalid",
        ));
    }
    Ok(())
}

fn participant_profile(
    participant: &ReplayCooperationParticipantEvidence,
) -> Result<CombatProfile, ReplayError> {
    let weapon = participant
        .module_state
        .weapons
        .iter()
        .filter(|weapon| weapon.item_id == participant.weapon_item_id)
        .collect::<Vec<_>>();
    if weapon.len() != 1 {
        return Err(cooperation_error(
            "selected weapon does not identify one effective profile",
        ));
    }
    let effective = weapon[0].effective;
    Ok(CombatProfile {
        damage: effective.damage,
        range: effective.range,
        cooldown_ms: effective.cooldown_ms,
        max_health: effective.max_health,
    })
}

fn validate_schema_revision(
    schema: u8,
    catalog: &str,
    operation_id: &str,
) -> Result<(), ReplayError> {
    if schema != COOPERATION_REPLAY_SCHEMA_VERSION || catalog != CATALOG_REVISION {
        return Err(cooperation_error(
            "cooperation schema or catalog revision is unknown",
        ));
    }
    validate_operation_id(operation_id)
}

fn validate_operation_id(value: &str) -> Result<(), ReplayError> {
    revenant_cooperation::validate_operation_id(value)
        .map_err(|error| cooperation_error(error.to_string()))
}

fn validate_bounded_identity(value: &str, context: &str) -> Result<(), ReplayError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
    {
        return Err(cooperation_error(format!(
            "cooperation {context} identifier is outside bounds"
        )));
    }
    Ok(())
}

fn validate_snake_identifier(value: &str, context: &str) -> Result<(), ReplayError> {
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(cooperation_error(format!(
            "cooperation {context} identifier is outside bounds"
        )));
    }
    Ok(())
}

fn fragment_quantity(payload: &str) -> Option<u32> {
    let suffix = payload.strip_prefix("loot granted: relay_core_fragment x")?;
    let (quantity, total) = suffix.split_once(" (total ")?;
    total.strip_suffix(')')?.parse::<u32>().ok()?;
    quantity.parse().ok()
}

fn experience_quantity(payload: &str) -> Option<u64> {
    let suffix = payload.strip_prefix("progression granted: +")?;
    let (experience, detail) = suffix.split_once(" XP (total ")?;
    let (_, level) = detail.split_once(", level ")?;
    let (prior, current) = level.strip_suffix(')')?.split_once(" -> ")?;
    detail.split_once(", level ")?.0.parse::<u64>().ok()?;
    prior.parse::<u32>().ok()?;
    current.parse::<u32>().ok()?;
    experience.parse().ok()
}

fn encode_payload(payload: &impl Serialize) -> Result<String, ReplayError> {
    let encoded = serde_json::to_string(payload).map_err(|error| {
        cooperation_error(format!(
            "cooperation replay JSON serialization failed: {error}"
        ))
    })?;
    validate_payload_size(&encoded)?;
    Ok(encoded)
}

fn decode_json<'a, T: Deserialize<'a>>(payload: &'a str) -> Result<T, ReplayError> {
    serde_json::from_str(payload)
        .map_err(|error| cooperation_error(format!("invalid cooperation replay JSON: {error}")))
}

fn validate_payload_size(payload: &str) -> Result<(), ReplayError> {
    if payload.len() > MAX_COOPERATION_REPLAY_PAYLOAD_BYTES {
        return Err(cooperation_error(
            "cooperation replay payload exceeds 8192 UTF-8 bytes",
        ));
    }
    Ok(())
}

fn domain_error(error: CooperationError) -> ReplayError {
    cooperation_error(error.to_string())
}

fn cooperation_error(message: impl Into<String>) -> ReplayError {
    ReplayError::InvalidCooperationEvidence(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encode_module_state_snapshot, module_state_evidence, reconstruct,
        ModuleStateSnapshotPayloadV1, ReplayEvent, ReplayEventKind, ReplayProtocolGeneration,
        MODULE_REPLAY_SCHEMA_VERSION,
    };

    const SESSION_ID: &str = "coop-session";
    const ACTIVITY: &str = "relay_awakening";
    const ANCHOR_ACCOUNT: &str = "anchor-account";
    const RUNNER_ACCOUNT: &str = "runner-account";
    const ANCHOR_CHARACTER: &str = "anchor-character";
    const RUNNER_CHARACTER: &str = "runner-character";
    const ANCHOR_ACTOR: u64 = 10;
    const RUNNER_ACTOR: u64 = 20;

    fn module_state() -> ReplayModuleStateEvidence {
        module_state_evidence(0, &[], &[], 0, ReplayProtocolGeneration::V2)
            .expect("empty active-V2 module state should resolve")
    }

    fn started_payload() -> CooperationStartedPayloadV1 {
        CooperationStartedPayloadV1 {
            schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
            activity_id: ACTIVITY.to_owned(),
            catalog_revision: CATALOG_REVISION.to_owned(),
            start_operation_id: "coop-start-1".to_owned(),
            participants: vec![
                ReplayCooperationParticipantEvidence {
                    participant_id: ANCHOR_ACCOUNT.to_owned(),
                    character_id: ANCHOR_CHARACTER.to_owned(),
                    actor_id: ANCHOR_ACTOR,
                    role: Role::Anchor,
                    weapon_item_id: "pulse_rifle".to_owned(),
                    module_state: module_state(),
                    admitted_health: 100,
                },
                ReplayCooperationParticipantEvidence {
                    participant_id: RUNNER_ACCOUNT.to_owned(),
                    character_id: RUNNER_CHARACTER.to_owned(),
                    actor_id: RUNNER_ACTOR,
                    role: Role::Runner,
                    weapon_item_id: "arc_sidearm".to_owned(),
                    module_state: module_state(),
                    admitted_health: 100,
                },
            ],
            anchor_target: ReplayCooperationTargetEvidence {
                target: Target::RelayAnchor,
                position: ANCHOR_POSITION,
            },
            runner_target: ReplayCooperationTargetEvidence {
                target: Target::RelayConsole,
                position: RUNNER_POSITION,
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
        }
    }

    fn pinged_payload() -> CooperationPingedPayloadV1 {
        CooperationPingedPayloadV1 {
            schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
            catalog_revision: CATALOG_REVISION.to_owned(),
            start_operation_id: "coop-start-1".to_owned(),
            anchor_elapsed_ms: 500,
            ping_operation_id: "coop-ping-1".to_owned(),
            source_participant_id: ANCHOR_ACCOUNT.to_owned(),
            source_actor_id: ANCHOR_ACTOR,
            target: Target::RelayConsole,
            ping_elapsed_ms: 1_000,
            expires_elapsed_ms: 6_000,
        }
    }

    fn downed_payload() -> PlayerDownedPayloadV1 {
        PlayerDownedPayloadV1 {
            schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
            catalog_revision: CATALOG_REVISION.to_owned(),
            start_operation_id: "coop-start-1".to_owned(),
            participant_id: RUNNER_ACCOUNT.to_owned(),
            actor_id: RUNNER_ACTOR,
            role: Role::Runner,
            target: Target::RelayConsole,
            runner_elapsed_ms: 4_000,
            hazard: ReplayCooperationHazard::RelayFeedback,
            health_before: 100,
            damage: 100,
            health_after: 0,
            life_before: LifeState::Active,
            life_after: LifeState::Downed,
        }
    }

    fn revived_payload() -> PlayerRevivedPayloadV1 {
        PlayerRevivedPayloadV1 {
            schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
            catalog_revision: CATALOG_REVISION.to_owned(),
            start_operation_id: "coop-start-1".to_owned(),
            revive_operation_id: "coop-revive-1".to_owned(),
            source_participant_id: ANCHOR_ACCOUNT.to_owned(),
            source_actor_id: ANCHOR_ACTOR,
            target_participant_id: RUNNER_ACCOUNT.to_owned(),
            target_actor_id: RUNNER_ACTOR,
            target_role: Role::Runner,
            revive_started_elapsed_ms: 5_000,
            revive_completed_elapsed_ms: 7_000,
            channel_duration_ms: REVIVE_CHANNEL_MS,
            maximum_distance_squared: MAX_REVIVE_DISTANCE_SQUARED,
            health_before: 0,
            health_after: REVIVE_HEALTH,
            life_before: LifeState::Downed,
            life_after: LifeState::Active,
            revive_count: 1,
        }
    }

    fn contributions(success: bool) -> ReplayCooperationContributionEvidence {
        ReplayCooperationContributionEvidence {
            anchor_arrived: true,
            anchor_elapsed_ms: Some(500),
            pinged: true,
            ping_operation_id: Some("coop-ping-1".to_owned()),
            ping_elapsed_ms: Some(1_000),
            runner_arrived: true,
            runner_elapsed_ms: Some(4_000),
            downed_elapsed_ms: Some(4_000),
            revive_operation_id: Some("coop-revive-1".to_owned()),
            revive_started_elapsed_ms: Some(5_000),
            revived: true,
            revive_completed_elapsed_ms: Some(7_000),
            revive_count: 1,
            warden_completed: success,
            warden_elapsed_ms: success.then_some(12_000),
        }
    }

    fn terminal_payload(outcome: TerminalOutcome, elapsed_ms: u64) -> CooperationTerminalPayloadV1 {
        let succeeded = outcome == TerminalOutcome::Succeeded;
        CooperationTerminalPayloadV1 {
            schema_version: COOPERATION_REPLAY_SCHEMA_VERSION,
            catalog_revision: CATALOG_REVISION.to_owned(),
            start_operation_id: "coop-start-1".to_owned(),
            last_nonterminal_phase: Phase::EncounterActive,
            contributions: contributions(succeeded),
            participants: vec![
                ReplayCooperationTerminalParticipantEvidence {
                    participant_id: ANCHOR_ACCOUNT.to_owned(),
                    character_id: ANCHOR_CHARACTER.to_owned(),
                    actor_id: ANCHOR_ACTOR,
                    role: Role::Anchor,
                    current_health: 100,
                    life: LifeState::Active,
                },
                ReplayCooperationTerminalParticipantEvidence {
                    participant_id: RUNNER_ACCOUNT.to_owned(),
                    character_id: RUNNER_CHARACTER.to_owned(),
                    actor_id: RUNNER_ACTOR,
                    role: Role::Runner,
                    current_health: REVIVE_HEALTH,
                    life: LifeState::Active,
                },
            ],
            outcome,
            subject_role: None,
            terminal_elapsed_ms: elapsed_ms,
            grants: if succeeded {
                {
                    [
                        (ANCHOR_ACCOUNT, ANCHOR_CHARACTER, ANCHOR_ACTOR),
                        (RUNNER_ACCOUNT, RUNNER_CHARACTER, RUNNER_ACTOR),
                    ]
                    .into_iter()
                    .map(|(participant_id, character_id, actor_id)| {
                        ReplayCooperationGrantEvidence {
                            participant_id: participant_id.to_owned(),
                            character_id: character_id.to_owned(),
                            actor_id,
                            item_id: RELAY_CORE_FRAGMENT.to_owned(),
                            item_quantity: SUCCESS_REWARD.fragments,
                            experience: SUCCESS_REWARD.experience,
                        }
                    })
                    .collect()
                }
            } else {
                Vec::new()
            },
        }
    }

    fn event(
        id: i64,
        kind: ReplayEventKind,
        account_id: &str,
        actor_id: u64,
        payload: String,
    ) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            timestamp: format!("2026-09-06 12:00:{id:02}+00"),
            session_id: SESSION_ID.to_owned(),
            account_id: account_id.to_owned(),
            activity_id: Some(ACTIVITY.to_owned()),
            actor_id: Some(actor_id),
            payload,
        }
    }

    fn admission_events(started: &CooperationStartedPayloadV1) -> Vec<ReplayEvent> {
        let anchor_snapshot = ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: ANCHOR_CHARACTER.to_owned(),
            protocol_generation: ReplayProtocolGeneration::V2,
            state: started.participants[0].module_state.clone(),
        };
        let runner_snapshot = ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: RUNNER_CHARACTER.to_owned(),
            protocol_generation: ReplayProtocolGeneration::V2,
            state: started.participants[1].module_state.clone(),
        };
        vec![
            event(
                1,
                ReplayEventKind::PlayerJoined,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "player joined".to_owned(),
            ),
            event(
                2,
                ReplayEventKind::ModuleStateSnapshot,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                encode_module_state_snapshot(&anchor_snapshot).expect("snapshot should encode"),
            ),
            event(
                3,
                ReplayEventKind::PlayerJoined,
                RUNNER_ACCOUNT,
                RUNNER_ACTOR,
                "player joined".to_owned(),
            ),
            event(
                4,
                ReplayEventKind::ModuleStateSnapshot,
                RUNNER_ACCOUNT,
                RUNNER_ACTOR,
                encode_module_state_snapshot(&runner_snapshot).expect("snapshot should encode"),
            ),
            event(
                5,
                ReplayEventKind::ActivityStarted,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "activity started".to_owned(),
            ),
            event(
                6,
                ReplayEventKind::EnemyDied,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "enemy died: relay_drone".to_owned(),
            ),
            event(
                7,
                ReplayEventKind::CooperationStarted,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                encode_cooperation_started(started).expect("start should encode"),
            ),
        ]
    }

    fn successful_events() -> Vec<ReplayEvent> {
        let started = started_payload();
        let pinged = pinged_payload();
        let downed = downed_payload();
        let revived = revived_payload();
        let terminal = terminal_payload(TerminalOutcome::Succeeded, 12_000);
        let mut events = admission_events(&started);
        events.extend([
            event(
                8,
                ReplayEventKind::CooperationPinged,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                encode_cooperation_pinged(&pinged, &started).expect("ping should encode"),
            ),
            event(
                9,
                ReplayEventKind::PlayerDowned,
                RUNNER_ACCOUNT,
                RUNNER_ACTOR,
                encode_player_downed(&downed, &started, &pinged).expect("downing should encode"),
            ),
            event(
                10,
                ReplayEventKind::PlayerRevived,
                RUNNER_ACCOUNT,
                RUNNER_ACTOR,
                encode_player_revived(&revived, &started, &downed).expect("revive should encode"),
            ),
            event(
                11,
                ReplayEventKind::BossSpawned,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "boss spawned: warden".to_owned(),
            ),
            event(
                12,
                ReplayEventKind::EnemyDied,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "enemy died: warden".to_owned(),
            ),
            event(
                13,
                ReplayEventKind::ActivityCompleted,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "activity completed".to_owned(),
            ),
            event(
                14,
                ReplayEventKind::LootGranted,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "loot granted: relay_core_fragment x2 (total 2)".to_owned(),
            ),
            event(
                15,
                ReplayEventKind::ProgressionGranted,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "progression granted: +125 XP (total 125, level 1 -> 1)".to_owned(),
            ),
            event(
                16,
                ReplayEventKind::LootGranted,
                RUNNER_ACCOUNT,
                RUNNER_ACTOR,
                "loot granted: relay_core_fragment x2 (total 2)".to_owned(),
            ),
            event(
                17,
                ReplayEventKind::ProgressionGranted,
                RUNNER_ACCOUNT,
                RUNNER_ACTOR,
                "progression granted: +125 XP (total 125, level 1 -> 1)".to_owned(),
            ),
            event(
                18,
                ReplayEventKind::CooperationSucceeded,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                encode_cooperation_succeeded(
                    &terminal,
                    &started,
                    CooperationReplayPrefix {
                        pinged: Some(&pinged),
                        downed: Some(&downed),
                        revived: Some(&revived),
                    },
                )
                .expect("terminal should encode"),
            ),
        ]);
        events
    }

    #[test]
    fn reconstructs_complete_success_from_six_kind_contract() {
        let reconstructed = reconstruct(&successful_events()).expect("success should reconstruct");
        assert_eq!(
            reconstructed.cooperation.state,
            ReconstructedCooperationState::Succeeded
        );
        assert_eq!(
            reconstructed.cooperation.last_phase(),
            Some(Phase::EncounterActive)
        );
        assert_eq!(reconstructed.cooperation.contribution_count(), 5);
        assert_eq!(
            reconstructed
                .cooperation
                .terminal
                .as_ref()
                .expect("terminal should exist")
                .grants
                .len(),
            PARTICIPANT_COUNT
        );
    }

    #[test]
    fn reconstructs_incomplete_prefix_without_fabricating_later_state() {
        let started = started_payload();
        let pinged = pinged_payload();
        let mut events = admission_events(&started);
        events.push(event(
            8,
            ReplayEventKind::CooperationPinged,
            ANCHOR_ACCOUNT,
            ANCHOR_ACTOR,
            encode_cooperation_pinged(&pinged, &started).expect("ping should encode"),
        ));
        let reconstructed = reconstruct(&events).expect("prefix should reconstruct");
        assert_eq!(
            reconstructed.cooperation.state,
            ReconstructedCooperationState::Active
        );
        assert_eq!(
            reconstructed.cooperation.last_phase(),
            Some(Phase::AwaitingRunner)
        );
        assert!(reconstructed.cooperation.downed.is_none());
        assert!(reconstructed.cooperation.terminal.is_none());
    }

    #[test]
    fn reconstructs_exact_ping_timeout_failure() {
        let started = started_payload();
        let pinged = pinged_payload();
        let mut participants = terminal_payload(TerminalOutcome::Succeeded, 12_000).participants;
        participants[1].current_health = 100;
        let terminal = CooperationTerminalPayloadV1 {
            last_nonterminal_phase: Phase::AwaitingRunner,
            contributions: ReplayCooperationContributionEvidence {
                anchor_arrived: true,
                anchor_elapsed_ms: Some(500),
                pinged: true,
                ping_operation_id: Some("coop-ping-1".to_owned()),
                ping_elapsed_ms: Some(1_000),
                runner_arrived: false,
                runner_elapsed_ms: None,
                downed_elapsed_ms: None,
                revive_operation_id: None,
                revive_started_elapsed_ms: None,
                revived: false,
                revive_completed_elapsed_ms: None,
                revive_count: 0,
                warden_completed: false,
                warden_elapsed_ms: None,
            },
            participants,
            outcome: TerminalOutcome::FailedPingTimeout,
            terminal_elapsed_ms: 6_001,
            grants: Vec::new(),
            ..terminal_payload(TerminalOutcome::FailedPingTimeout, 6_001)
        };
        let mut events = admission_events(&started);
        events.push(event(
            8,
            ReplayEventKind::CooperationPinged,
            ANCHOR_ACCOUNT,
            ANCHOR_ACTOR,
            encode_cooperation_pinged(&pinged, &started).expect("ping should encode"),
        ));
        events.push(event(
            9,
            ReplayEventKind::CooperationFailed,
            ANCHOR_ACCOUNT,
            ANCHOR_ACTOR,
            encode_cooperation_failed(
                &terminal,
                &started,
                CooperationReplayPrefix {
                    pinged: Some(&pinged),
                    ..CooperationReplayPrefix::default()
                },
            )
            .expect("failure should encode"),
        ));
        let reconstructed = reconstruct(&events).expect("timeout should reconstruct");
        assert_eq!(
            reconstructed.cooperation.state,
            ReconstructedCooperationState::Failed
        );
        assert_eq!(
            reconstructed
                .cooperation
                .terminal
                .expect("terminal should exist")
                .outcome,
            TerminalOutcome::FailedPingTimeout
        );
    }

    #[test]
    fn rejects_unknown_fields_oversize_and_route_coexistence() {
        let mut events = successful_events();
        events[7].payload = events[7].payload.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"unknown\":true",
            1,
        );
        assert!(reconstruct(&events).is_err());

        assert!(decode_cooperation_payload(
            ReplayEventKind::CooperationStarted,
            &"x".repeat(MAX_COOPERATION_REPLAY_PAYLOAD_BYTES + 1),
        )
        .is_err());

        let mut events = successful_events();
        events.insert(
            8,
            event(
                9_000,
                ReplayEventKind::RouteSelected,
                ANCHOR_ACCOUNT,
                ANCHOR_ACTOR,
                "{}".to_owned(),
            ),
        );
        for (index, event) in events.iter_mut().enumerate() {
            event.id = i64::try_from(index + 1).expect("small index");
        }
        assert!(reconstruct(&events).is_err());
    }

    #[test]
    fn rejects_skipped_transition_and_wrong_timeout_precedence() {
        let mut skipped = successful_events();
        skipped.remove(7);
        for (index, event) in skipped.iter_mut().enumerate() {
            event.id = i64::try_from(index + 1).expect("small index");
        }
        assert!(reconstruct(&skipped).is_err());

        let started = started_payload();
        let pinged = pinged_payload();
        let downed = downed_payload();
        let mut terminal = terminal_payload(TerminalOutcome::FailedOperationTimeout, 60_001);
        terminal.last_nonterminal_phase = Phase::RunnerDowned;
        terminal.contributions = ReplayCooperationContributionEvidence {
            anchor_arrived: true,
            anchor_elapsed_ms: Some(500),
            pinged: true,
            ping_operation_id: Some("coop-ping-1".to_owned()),
            ping_elapsed_ms: Some(1_000),
            runner_arrived: true,
            runner_elapsed_ms: Some(4_000),
            downed_elapsed_ms: Some(4_000),
            revive_operation_id: None,
            revive_started_elapsed_ms: None,
            revived: false,
            revive_completed_elapsed_ms: None,
            revive_count: 0,
            warden_completed: false,
            warden_elapsed_ms: None,
        };
        terminal.participants[1].current_health = 0;
        terminal.participants[1].life = LifeState::Downed;
        assert!(encode_cooperation_failed(
            &terminal,
            &started,
            CooperationReplayPrefix {
                pinged: Some(&pinged),
                downed: Some(&downed),
                revived: None,
            },
        )
        .is_err());
    }

    #[test]
    fn rejects_duplicate_identity_generation_reward_and_post_terminal_corruption() {
        let mut duplicate = successful_events();
        let mut second_terminal = duplicate.last().expect("terminal should exist").clone();
        second_terminal.id = 19;
        duplicate.push(second_terminal);
        assert!(reconstruct(&duplicate).is_err());

        let mut identity = successful_events();
        identity[8].account_id = ANCHOR_ACCOUNT.to_owned();
        identity[8].actor_id = Some(ANCHOR_ACTOR);
        assert!(reconstruct(&identity).is_err());

        let mut inactive_generation = successful_events();
        inactive_generation[1].payload = inactive_generation[1].payload.replacen(
            "\"protocol_generation\":\"v2\"",
            "\"protocol_generation\":\"v1\"",
            1,
        );
        assert!(reconstruct(&inactive_generation).is_err());

        let mut missing_reward = successful_events();
        missing_reward.remove(13);
        for (index, event) in missing_reward.iter_mut().enumerate() {
            event.id = i64::try_from(index + 1).expect("small index");
        }
        assert!(reconstruct(&missing_reward).is_err());

        let mut post_terminal = successful_events();
        post_terminal.push(event(
            19,
            ReplayEventKind::LootGranted,
            ANCHOR_ACCOUNT,
            ANCHOR_ACTOR,
            "loot granted: relay_core_fragment x2 (total 4)".to_owned(),
        ));
        assert!(reconstruct(&post_terminal).is_err());

        let mut wrong_schema = successful_events();
        let terminal = wrong_schema.last_mut().expect("terminal should exist");
        terminal.payload =
            terminal
                .payload
                .replacen("\"schema_version\":1", "\"schema_version\":2", 1);
        assert!(reconstruct(&wrong_schema).is_err());
    }
}
