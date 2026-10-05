use std::collections::{BTreeMap, BTreeSet};

use revenant_operations::{
    apply_warden_health_effect, resolve_event, route_definition, validate_operation_id,
    RouteEffect, RouteEventId, RouteId, RouteObjectiveId, RouteOperationSummary, RouteReward,
    RouteSeed, TerminalOutcome, CATALOG_REVISION, MAX_OPTIMAL_INCOMING_DAMAGE, RESOLVER_REVISION,
};
use serde::{Deserialize, Serialize};

use super::{
    decode_module_payload, route_error, validate_module_state, DecodedModuleReplayPayload,
    ReplayError, ReplayEvent, ReplayEventKind, ReplayModuleStateEvidence, ReplayProtocolGeneration,
};

pub const ROUTE_REPLAY_SCHEMA_VERSION: u8 = 1;
pub const MAX_ROUTE_REPLAY_PAYLOAD_BYTES: usize = 8_192;
pub const AUTHORING_REVISION: &str = "m27-v1";
const ACTIVITY_ID: &str = "relay_awakening";
const RELAY_CORE_FRAGMENT: &str = "relay_core_fragment";
const MAX_ROUTE_TRANSITIONS: usize = 8;
const SOLO_DRONE_HEALTH: u32 = 140;
const TWO_PLAYER_DRONE_HEALTH: u32 = 210;
const DRONE_DAMAGE: u32 = 10;
const SOLO_WARDEN_HEALTH: u32 = 240;
const TWO_PLAYER_WARDEN_HEALTH: u32 = 360;
const MAX_ACCEPTED_PLAYER_HITS: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRouteParticipantEvidence {
    pub participant_id: String,
    pub character_id: String,
    pub actor_id: u64,
    pub weapon_item_id: String,
    pub module_state: ReplayModuleStateEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayRouteObjectiveKind {
    KillActors,
    ReachArea,
    Boss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayRouteObjectiveState {
    Pending,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRouteObjectiveEvidence {
    pub objective_id: RouteObjectiveId,
    pub kind: ReplayRouteObjectiveKind,
    pub initial_state: ReplayRouteObjectiveState,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteSelectedPayloadV1 {
    pub schema_version: u8,
    pub activity_id: String,
    pub authoring_revision: String,
    pub catalog_revision: String,
    pub resolver_revision: String,
    pub operation_id: String,
    pub leader_id: String,
    pub participants: Vec<ReplayRouteParticipantEvidence>,
    pub route_id: RouteId,
    pub seed: RouteSeed,
    pub event_id: RouteEventId,
    pub effect: RouteEffect,
    pub objectives: Vec<ReplayRouteObjectiveEvidence>,
    pub duration_budget_ms: u64,
    pub reward: RouteReward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayObjectiveTransition {
    pub ordinal: u8,
    pub objective_id: RouteObjectiveId,
    pub from: ReplayRouteObjectiveState,
    pub to: ReplayRouteObjectiveState,
    pub progress: u32,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRouteEncounterEvidence {
    pub participant_count: u8,
    pub drone_health: u32,
    pub drone_damage: u32,
    pub drone_defeated: bool,
    pub warden_spawned: bool,
    pub warden_base_health: u32,
    pub warden_effective_health: u32,
    pub warden_remaining_health: u32,
    pub accepted_player_hits: u32,
    pub warden_counter_count: u8,
    pub warden_counter_damage: u32,
    pub total_hostile_damage: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRouteGrantEvidence {
    pub participant_id: String,
    pub character_id: String,
    pub actor_id: u64,
    pub item_id: String,
    pub item_quantity: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteTerminalPayloadV1 {
    pub schema_version: u8,
    pub authoring_revision: String,
    pub catalog_revision: String,
    pub resolver_revision: String,
    pub operation_id: String,
    pub route_id: RouteId,
    pub event_id: RouteEventId,
    pub transitions: Vec<ReplayObjectiveTransition>,
    pub encounter: ReplayRouteEncounterEvidence,
    pub summary: RouteOperationSummary,
    pub grants: Vec<ReplayRouteGrantEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event_type", content = "payload", rename_all = "snake_case")]
pub enum DecodedRouteReplayPayload {
    RouteSelected(RouteSelectedPayloadV1),
    RouteOperationSucceeded(RouteTerminalPayloadV1),
    RouteOperationFailed(RouteTerminalPayloadV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconstructedRouteState {
    Legacy,
    RoutedIncomplete,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedRouteReplay {
    pub state: ReconstructedRouteState,
    pub selection: Option<RouteSelectedPayloadV1>,
    pub terminal: Option<RouteTerminalPayloadV1>,
}

impl Default for ReconstructedRouteReplay {
    fn default() -> Self {
        Self {
            state: ReconstructedRouteState::Legacy,
            selection: None,
            terminal: None,
        }
    }
}

/// Serializes one validated route-selection payload.
///
/// # Errors
///
/// Returns an error for structurally invalid evidence, JSON failure, or an
/// encoded payload over 8,192 UTF-8 bytes.
pub fn encode_route_selected(payload: &RouteSelectedPayloadV1) -> Result<String, ReplayError> {
    validate_selection_payload(payload)?;
    encode_route_payload(payload)
}

/// Serializes one validated successful terminal payload.
///
/// # Errors
///
/// Returns an error when the terminal contradicts its selection or exceeds
/// the bounded payload envelope.
pub fn encode_route_operation_succeeded(
    payload: &RouteTerminalPayloadV1,
    selection: &RouteSelectedPayloadV1,
) -> Result<String, ReplayError> {
    validate_terminal_payload(ReplayEventKind::RouteOperationSucceeded, payload, selection)?;
    encode_route_payload(payload)
}

/// Serializes one validated failed terminal payload.
///
/// # Errors
///
/// Returns an error when the terminal contradicts its selection or exceeds
/// the bounded payload envelope.
pub fn encode_route_operation_failed(
    payload: &RouteTerminalPayloadV1,
    selection: &RouteSelectedPayloadV1,
) -> Result<String, ReplayError> {
    validate_terminal_payload(ReplayEventKind::RouteOperationFailed, payload, selection)?;
    encode_route_payload(payload)
}

/// Decodes and validates one structured route event.
///
/// Selection-independent terminal validation is completed during session
/// reconstruction. Non-route kinds return `Ok(None)`.
///
/// # Errors
///
/// Returns an error for oversized, malformed, wrong-version, or structurally
/// invalid route evidence.
pub fn decode_route_payload(
    kind: ReplayEventKind,
    payload: &str,
) -> Result<Option<DecodedRouteReplayPayload>, ReplayError> {
    if !kind.is_route_event() {
        return Ok(None);
    }
    validate_route_payload_size(payload)?;
    let decoded = match kind {
        ReplayEventKind::RouteSelected => {
            let value: RouteSelectedPayloadV1 = decode_route_json(payload)?;
            validate_selection_payload(&value)?;
            DecodedRouteReplayPayload::RouteSelected(value)
        }
        ReplayEventKind::RouteOperationSucceeded => {
            let value: RouteTerminalPayloadV1 = decode_route_json(payload)?;
            validate_terminal_shape(&value)?;
            DecodedRouteReplayPayload::RouteOperationSucceeded(value)
        }
        ReplayEventKind::RouteOperationFailed => {
            let value: RouteTerminalPayloadV1 = decode_route_json(payload)?;
            validate_terminal_shape(&value)?;
            DecodedRouteReplayPayload::RouteOperationFailed(value)
        }
        _ => return Ok(None),
    };
    Ok(Some(decoded))
}

pub(super) fn reconstruct_route_replay(
    events: &[ReplayEvent],
) -> Result<ReconstructedRouteReplay, ReplayError> {
    let mut accumulator = RouteReplayAccumulator::default();
    for event in events {
        accumulator.apply(event)?;
    }
    accumulator.finish()
}

#[derive(Debug, Default)]
struct RouteReplayAccumulator {
    snapshots: BTreeMap<String, SnapshotEvidence>,
    snapshot_order: Vec<String>,
    selection: Option<RouteSelectedPayloadV1>,
    terminal: Option<RouteTerminalPayloadV1>,
    activity_completions: usize,
    loot_grants: BTreeMap<String, (u64, u32)>,
    progression_grants: BTreeMap<String, (u64, u64)>,
    observed_activity_id: Option<String>,
    mixed_activity: bool,
}

impl RouteReplayAccumulator {
    fn apply(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        if let Some(activity_id) = &event.activity_id {
            if self
                .observed_activity_id
                .as_ref()
                .is_some_and(|observed| observed != activity_id)
            {
                self.mixed_activity = true;
            } else if self.observed_activity_id.is_none() {
                self.observed_activity_id = Some(activity_id.clone());
            }
        }
        if event.kind == ReplayEventKind::ModuleStateSnapshot {
            if self.selection.is_some() {
                return Err(route_error(
                    "routed admission snapshot appears after route selection",
                ));
            }
            record_snapshot(event, &mut self.snapshots, &mut self.snapshot_order)?;
        }
        if self.terminal.is_some()
            && matches!(
                event.kind,
                ReplayEventKind::ActivityCompleted
                    | ReplayEventKind::LootGranted
                    | ReplayEventKind::ProgressionGranted
            )
        {
            return Err(route_error(
                "generic completion or reward evidence appears after route terminal",
            ));
        }
        if self.selection.is_some() && self.terminal.is_none() {
            self.record_generic_event(event)?;
        }
        self.record_route_event(event)?;
        if self.selection.is_some() && self.mixed_activity {
            return Err(route_error("routed session mixes activity identifiers"));
        }
        Ok(())
    }

    fn record_generic_event(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        match event.kind {
            ReplayEventKind::ActivityCompleted => {
                let selection = self
                    .selection
                    .as_ref()
                    .ok_or_else(|| route_error("route selection disappeared"))?;
                let leader = &selection.participants[0];
                if event.activity_id.as_deref() != Some(selection.activity_id.as_str())
                    || event.account_id != leader.participant_id
                    || event.actor_id != Some(leader.actor_id)
                {
                    return Err(route_error(
                        "routed activity completion row disagrees with its leader",
                    ));
                }
                self.activity_completions = self
                    .activity_completions
                    .checked_add(1)
                    .ok_or_else(|| route_error("activity completion count overflowed"))?;
            }
            ReplayEventKind::LootGranted => {
                let selection = self
                    .selection
                    .as_ref()
                    .ok_or_else(|| route_error("route selection disappeared"))?;
                if event.activity_id.as_deref() != Some(selection.activity_id.as_str()) {
                    return Err(route_error(
                        "routed loot grant activity disagrees with selection",
                    ));
                }
                let quantity = route_fragment_quantity(&event.payload).ok_or_else(|| {
                    route_error("routed loot replay payload is not the exact fragment grant")
                })?;
                let actor_id = event
                    .actor_id
                    .ok_or_else(|| route_error("routed loot grant has no actor"))?;
                if self
                    .loot_grants
                    .insert(event.account_id.clone(), (actor_id, quantity))
                    .is_some()
                {
                    return Err(route_error("routed participant has duplicate loot grants"));
                }
            }
            ReplayEventKind::ProgressionGranted => {
                let selection = self
                    .selection
                    .as_ref()
                    .ok_or_else(|| route_error("route selection disappeared"))?;
                if event.activity_id.as_deref() != Some(selection.activity_id.as_str()) {
                    return Err(route_error(
                        "routed progression grant activity disagrees with selection",
                    ));
                }
                let experience = route_experience_quantity(&event.payload)
                    .ok_or_else(|| route_error("routed progression replay payload is malformed"))?;
                let actor_id = event
                    .actor_id
                    .ok_or_else(|| route_error("routed progression grant has no actor"))?;
                if self
                    .progression_grants
                    .insert(event.account_id.clone(), (actor_id, experience))
                    .is_some()
                {
                    return Err(route_error(
                        "routed participant has duplicate progression grants",
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn record_route_event(&mut self, event: &ReplayEvent) -> Result<(), ReplayError> {
        match event.kind {
            ReplayEventKind::RouteSelected => {
                if self.selection.is_some() || self.terminal.is_some() {
                    return Err(route_error(
                        "route selection is duplicated or post-terminal",
                    ));
                }
                let Some(DecodedRouteReplayPayload::RouteSelected(payload)) =
                    decode_route_payload(event.kind, &event.payload)?
                else {
                    return Err(route_error("route selection decoded as another payload"));
                };
                validate_selection_event(event, &payload, &self.snapshots, &self.snapshot_order)?;
                self.selection = Some(payload);
            }
            ReplayEventKind::RouteOperationSucceeded | ReplayEventKind::RouteOperationFailed => {
                if self.terminal.is_some() {
                    return Err(route_error("route terminal evidence is duplicated"));
                }
                let selected = self
                    .selection
                    .as_ref()
                    .ok_or_else(|| route_error("route terminal precedes route selection"))?;
                let Some(
                    DecodedRouteReplayPayload::RouteOperationSucceeded(payload)
                    | DecodedRouteReplayPayload::RouteOperationFailed(payload),
                ) = decode_route_payload(event.kind, &event.payload)?
                else {
                    return Err(route_error("route terminal decoded as another payload"));
                };
                validate_terminal_event(event, &payload, selected)?;
                validate_generic_terminal_evidence(
                    event.kind,
                    selected,
                    &payload,
                    self.activity_completions,
                    &self.loot_grants,
                    &self.progression_grants,
                )?;
                self.terminal = Some(payload);
            }
            _ => {}
        }
        Ok(())
    }

    fn finish(self) -> Result<ReconstructedRouteReplay, ReplayError> {
        let Some(selected) = self.selection else {
            return Ok(ReconstructedRouteReplay::default());
        };
        let state = match self
            .terminal
            .as_ref()
            .map(|payload| payload.summary.outcome)
        {
            None => {
                if self.activity_completions != 0
                    || !self.loot_grants.is_empty()
                    || !self.progression_grants.is_empty()
                {
                    return Err(route_error(
                        "incomplete route has completion or reward replay evidence",
                    ));
                }
                ReconstructedRouteState::RoutedIncomplete
            }
            Some(TerminalOutcome::Succeeded) => ReconstructedRouteState::Succeeded,
            Some(TerminalOutcome::FailedTimeout) => ReconstructedRouteState::Failed,
        };
        Ok(ReconstructedRouteReplay {
            state,
            selection: Some(selected),
            terminal: self.terminal,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotEvidence {
    account_id: String,
    actor_id: u64,
    protocol_generation: ReplayProtocolGeneration,
    state: ReplayModuleStateEvidence,
}

fn record_snapshot(
    event: &ReplayEvent,
    snapshots: &mut BTreeMap<String, SnapshotEvidence>,
    snapshot_order: &mut Vec<String>,
) -> Result<(), ReplayError> {
    let Some(DecodedModuleReplayPayload::ModuleStateSnapshot(payload)) =
        decode_module_payload(event.kind, &event.payload)?
    else {
        return Err(route_error("module snapshot decoded as another payload"));
    };
    let actor_id = event
        .actor_id
        .ok_or_else(|| route_error("module snapshot has no actor"))?;
    if snapshots.contains_key(&payload.character_id) {
        return Err(route_error("participant module snapshot is duplicated"));
    }
    snapshot_order.push(payload.character_id.clone());
    snapshots.insert(
        payload.character_id,
        SnapshotEvidence {
            account_id: event.account_id.clone(),
            actor_id,
            protocol_generation: payload.protocol_generation,
            state: payload.state,
        },
    );
    Ok(())
}

fn validate_selection_event(
    event: &ReplayEvent,
    payload: &RouteSelectedPayloadV1,
    snapshots: &BTreeMap<String, SnapshotEvidence>,
    snapshot_order: &[String],
) -> Result<(), ReplayError> {
    validate_bounded_identity(&event.session_id, "session")?;
    if event.activity_id.as_deref() != Some(payload.activity_id.as_str()) {
        return Err(route_error("selection row activity disagrees with payload"));
    }
    let leader = payload
        .participants
        .first()
        .ok_or_else(|| route_error("route selection has no leader"))?;
    if event.account_id != leader.participant_id
        || event.actor_id != Some(leader.actor_id)
        || payload.leader_id != leader.participant_id
    {
        return Err(route_error(
            "selection row identity disagrees with the admitted leader",
        ));
    }
    if snapshots.len() != payload.participants.len()
        || snapshot_order.len() != payload.participants.len()
        || snapshot_order
            .iter()
            .zip(&payload.participants)
            .any(|(character_id, participant)| character_id != &participant.character_id)
    {
        return Err(route_error(
            "route participants disagree with immutable admission order",
        ));
    }
    for participant in &payload.participants {
        let snapshot = snapshots
            .get(&participant.character_id)
            .ok_or_else(|| route_error("route participant has no M26 module snapshot"))?;
        if snapshot.account_id != participant.participant_id
            || snapshot.actor_id != participant.actor_id
            || snapshot.protocol_generation != ReplayProtocolGeneration::V2
            || snapshot.state != participant.module_state
        {
            return Err(route_error(
                "route participant disagrees with immutable admission evidence",
            ));
        }
    }
    Ok(())
}

fn validate_terminal_event(
    event: &ReplayEvent,
    payload: &RouteTerminalPayloadV1,
    selection: &RouteSelectedPayloadV1,
) -> Result<(), ReplayError> {
    validate_terminal_payload(event.kind, payload, selection)?;
    let leader = selection
        .participants
        .first()
        .ok_or_else(|| route_error("route selection has no leader"))?;
    if event.activity_id.as_deref() != Some(selection.activity_id.as_str())
        || event.account_id != leader.participant_id
        || event.actor_id != Some(leader.actor_id)
    {
        return Err(route_error(
            "terminal row identity disagrees with the admitted leader",
        ));
    }
    Ok(())
}

fn validate_selection_payload(payload: &RouteSelectedPayloadV1) -> Result<(), ReplayError> {
    validate_route_schema(payload.schema_version)?;
    if payload.activity_id != ACTIVITY_ID
        || payload.authoring_revision != AUTHORING_REVISION
        || payload.catalog_revision != CATALOG_REVISION
        || payload.resolver_revision != RESOLVER_REVISION
    {
        return Err(route_error(
            "route selection revision or activity is unknown",
        ));
    }
    validate_operation_id(&payload.operation_id).map_err(|error| route_error(error.to_string()))?;
    validate_bounded_identity(&payload.leader_id, "leader")?;
    if payload.participants.is_empty() || payload.participants.len() > 2 {
        return Err(route_error(
            "route selection requires one or two participants",
        ));
    }
    if payload.participants[0].participant_id != payload.leader_id {
        return Err(route_error("first route participant is not the leader"));
    }
    let mut participant_ids = BTreeSet::new();
    let mut character_ids = BTreeSet::new();
    let mut actor_ids = BTreeSet::new();
    for participant in &payload.participants {
        validate_bounded_identity(&participant.participant_id, "participant")?;
        validate_bounded_identity(&participant.character_id, "character")?;
        validate_snake_identifier(&participant.weapon_item_id, "weapon")?;
        if !participant_ids.insert(participant.participant_id.as_str())
            || !character_ids.insert(participant.character_id.as_str())
            || !actor_ids.insert(participant.actor_id)
        {
            return Err(route_error("route participant identity is duplicated"));
        }
        if !participant.module_state.module_effects_active {
            return Err(route_error(
                "routed participant has inactive V1 module evidence",
            ));
        }
        validate_module_state(&participant.module_state)?;
        if participant
            .module_state
            .weapons
            .iter()
            .filter(|weapon| weapon.item_id == participant.weapon_item_id)
            .count()
            != 1
        {
            return Err(route_error(
                "selected weapon does not identify one immutable combat profile",
            ));
        }
    }

    let definition = route_definition(payload.route_id);
    let resolved = resolve_event(payload.route_id, payload.seed);
    if payload.event_id != resolved.event_id
        || payload.effect != resolved.effect
        || payload.reward != definition.reward
        || payload.duration_budget_ms != definition.duration_ms
    {
        return Err(route_error(
            "route selection seed, event, effect, duration, or reward drifted",
        ));
    }
    if payload.objectives.len() != definition.objective_path.len() {
        return Err(route_error("route objective path cardinality drifted"));
    }
    for (index, (objective, expected_id)) in payload
        .objectives
        .iter()
        .zip(definition.objective_path)
        .enumerate()
    {
        let expected_state = if index == 0 {
            ReplayRouteObjectiveState::Active
        } else {
            ReplayRouteObjectiveState::Pending
        };
        if objective.objective_id != *expected_id
            || objective.kind != objective_kind(*expected_id)
            || objective.initial_state != expected_state
            || objective.target != 1
        {
            return Err(route_error("route objective evidence drifted from catalog"));
        }
    }
    Ok(())
}

fn validate_terminal_shape(payload: &RouteTerminalPayloadV1) -> Result<(), ReplayError> {
    validate_route_schema(payload.schema_version)?;
    if payload.authoring_revision != AUTHORING_REVISION
        || payload.catalog_revision != CATALOG_REVISION
        || payload.resolver_revision != RESOLVER_REVISION
    {
        return Err(route_error("route terminal revision is unknown"));
    }
    validate_operation_id(&payload.operation_id).map_err(|error| route_error(error.to_string()))?;
    if payload.transitions.is_empty() || payload.transitions.len() > MAX_ROUTE_TRANSITIONS {
        return Err(route_error("route transition count is outside bounds"));
    }
    Ok(())
}

fn validate_terminal_payload(
    kind: ReplayEventKind,
    payload: &RouteTerminalPayloadV1,
    selection: &RouteSelectedPayloadV1,
) -> Result<(), ReplayError> {
    validate_terminal_shape(payload)?;
    let expected_outcome = match kind {
        ReplayEventKind::RouteOperationSucceeded => TerminalOutcome::Succeeded,
        ReplayEventKind::RouteOperationFailed => TerminalOutcome::FailedTimeout,
        _ => return Err(route_error("non-terminal kind used for route terminal")),
    };
    if payload.operation_id != selection.operation_id
        || payload.route_id != selection.route_id
        || payload.event_id != selection.event_id
        || payload.authoring_revision != selection.authoring_revision
        || payload.catalog_revision != selection.catalog_revision
        || payload.resolver_revision != selection.resolver_revision
    {
        return Err(route_error(
            "route terminal identity disagrees with selection",
        ));
    }
    let summary = &payload.summary;
    let participant_ids = selection
        .participants
        .iter()
        .map(|participant| participant.participant_id.clone())
        .collect::<Vec<_>>();
    let objective_path = selection
        .objectives
        .iter()
        .map(|objective| objective.objective_id)
        .collect::<Vec<_>>();
    let expected_reward =
        (expected_outcome == TerminalOutcome::Succeeded).then_some(selection.reward);
    if summary.catalog_revision != selection.catalog_revision
        || summary.resolver_revision != selection.resolver_revision
        || summary.operation_id != selection.operation_id
        || summary.leader_id != selection.leader_id
        || summary.participant_ids != participant_ids
        || summary.route_id != selection.route_id
        || summary.seed != selection.seed
        || summary.event_id != selection.event_id
        || summary.effect != selection.effect
        || summary.objective_path != objective_path
        || summary.duration_budget_ms != selection.duration_budget_ms
        || summary.outcome != expected_outcome
        || summary.reward != expected_reward
    {
        return Err(route_error(
            "route terminal summary disagrees with selection",
        ));
    }
    match expected_outcome {
        TerminalOutcome::Succeeded if summary.elapsed_ms > selection.duration_budget_ms => {
            return Err(route_error("successful route exceeds its deadline"));
        }
        TerminalOutcome::FailedTimeout if summary.elapsed_ms <= selection.duration_budget_ms => {
            return Err(route_error("failed route does not exceed its deadline"));
        }
        _ => {}
    }
    validate_transitions(payload, selection, expected_outcome)?;
    validate_encounter(payload, selection, expected_outcome)?;
    validate_grants(payload, selection, expected_outcome)
}

fn validate_transitions(
    payload: &RouteTerminalPayloadV1,
    selection: &RouteSelectedPayloadV1,
    outcome: TerminalOutcome,
) -> Result<(), ReplayError> {
    let mut states = selection
        .objectives
        .iter()
        .map(|objective| objective.initial_state)
        .collect::<Vec<_>>();
    let mut failed = false;
    for (index, transition) in payload.transitions.iter().enumerate() {
        if usize::from(transition.ordinal) != index || transition.target != 1 {
            return Err(route_error("route transition ordinal or target is invalid"));
        }
        let objective_index = selection
            .objectives
            .iter()
            .position(|objective| objective.objective_id == transition.objective_id)
            .ok_or_else(|| route_error("route transition names an unknown objective"))?;
        if failed || states[objective_index] != transition.from {
            return Err(route_error("route transition source state is impossible"));
        }
        match (transition.from, transition.to) {
            (ReplayRouteObjectiveState::Pending, ReplayRouteObjectiveState::Active) => {
                if transition.progress != 0
                    || objective_index == 0
                    || states[..objective_index]
                        .iter()
                        .any(|state| *state != ReplayRouteObjectiveState::Completed)
                {
                    return Err(route_error("route objective activation is impossible"));
                }
            }
            (ReplayRouteObjectiveState::Active, ReplayRouteObjectiveState::Completed) => {
                if transition.progress != transition.target {
                    return Err(route_error("completed objective has wrong progress"));
                }
            }
            (ReplayRouteObjectiveState::Active, ReplayRouteObjectiveState::Failed) => {
                if transition.progress >= transition.target
                    || index + 1 != payload.transitions.len()
                {
                    return Err(route_error("failed objective transition is impossible"));
                }
                failed = true;
            }
            _ => return Err(route_error("route objective transition is illegal")),
        }
        states[objective_index] = transition.to;
        if !failed
            && states
                .iter()
                .filter(|state| **state == ReplayRouteObjectiveState::Active)
                .count()
                > 1
        {
            return Err(route_error("multiple route objectives are active"));
        }
    }
    match outcome {
        TerminalOutcome::Succeeded
            if failed
                || states
                    .iter()
                    .any(|state| *state != ReplayRouteObjectiveState::Completed) =>
        {
            Err(route_error(
                "successful route did not complete its exact path",
            ))
        }
        TerminalOutcome::FailedTimeout
            if !failed
                || states
                    .iter()
                    .filter(|state| **state == ReplayRouteObjectiveState::Failed)
                    .count()
                    != 1 =>
        {
            Err(route_error(
                "failed route lacks one terminal objective failure",
            ))
        }
        _ => Ok(()),
    }
}

fn validate_encounter(
    payload: &RouteTerminalPayloadV1,
    selection: &RouteSelectedPayloadV1,
    outcome: TerminalOutcome,
) -> Result<(), ReplayError> {
    let encounter = &payload.encounter;
    let participant_count = u8::try_from(selection.participants.len())
        .map_err(|_| route_error("route participant count does not fit u8"))?;
    let (drone_health, warden_health) = match participant_count {
        1 => (SOLO_DRONE_HEALTH, SOLO_WARDEN_HEALTH),
        2 => (TWO_PLAYER_DRONE_HEALTH, TWO_PLAYER_WARDEN_HEALTH),
        _ => return Err(route_error("route encounter participant count is invalid")),
    };
    let effective_health = apply_warden_health_effect(warden_health, selection.effect)
        .map_err(|error| route_error(error.to_string()))?;
    if encounter.participant_count != participant_count
        || encounter.drone_health != drone_health
        || encounter.drone_damage != DRONE_DAMAGE
        || encounter.warden_base_health != warden_health
        || encounter.warden_effective_health != effective_health
        || encounter.warden_counter_damage != selection.effect.warden_counter_damage
        || encounter.warden_remaining_health > encounter.warden_effective_health
        || encounter.accepted_player_hits > MAX_ACCEPTED_PLAYER_HITS
        || encounter.warden_counter_count > 2
        || encounter.total_hostile_damage > MAX_OPTIMAL_INCOMING_DAMAGE
    {
        return Err(route_error(
            "route encounter evidence is outside exact bounds",
        ));
    }
    if !encounter.warden_spawned
        && (encounter.warden_remaining_health != 0
            || encounter.accepted_player_hits != 0
            || encounter.warden_counter_count != 0)
    {
        return Err(route_error("unspawned Warden has recorded combat results"));
    }
    let counter_damage = u32::from(encounter.warden_counter_count)
        .checked_mul(encounter.warden_counter_damage)
        .ok_or_else(|| route_error("hostile damage arithmetic overflowed"))?;
    if encounter.total_hostile_damage != counter_damage
        && encounter.total_hostile_damage != counter_damage.saturating_add(DRONE_DAMAGE)
    {
        return Err(route_error(
            "hostile damage disagrees with recorded attacks",
        ));
    }
    if outcome == TerminalOutcome::Succeeded
        && (!encounter.drone_defeated
            || !encounter.warden_spawned
            || encounter.warden_remaining_health != 0
            || encounter.accepted_player_hits == 0)
    {
        return Err(route_error("successful encounter evidence is incomplete"));
    }
    if encounter.warden_spawned && !encounter.drone_defeated {
        return Err(route_error("Warden spawned before the drone was defeated"));
    }
    match outcome {
        TerminalOutcome::Succeeded => {
            let expected_damage = DRONE_DAMAGE
                .checked_add(2_u32.saturating_mul(encounter.warden_counter_damage))
                .ok_or_else(|| route_error("successful hostile damage overflowed"))?;
            if encounter.warden_counter_count != 2
                || encounter.total_hostile_damage != expected_damage
            {
                return Err(route_error(
                    "successful encounter pressure disagrees with M25/M27 evidence",
                ));
            }
        }
        TerminalOutcome::FailedTimeout => validate_failed_encounter_path(payload)?,
    }
    validate_player_hit_envelope(encounter, selection)
}

fn validate_failed_encounter_path(payload: &RouteTerminalPayloadV1) -> Result<(), ReplayError> {
    let failed_objective = payload
        .transitions
        .iter()
        .find(|transition| transition.to == ReplayRouteObjectiveState::Failed)
        .ok_or_else(|| route_error("failed route has no failed objective"))?
        .objective_id;
    let encounter = &payload.encounter;
    let consistent = match failed_objective {
        RouteObjectiveId::ClearDroneGroup => !encounter.drone_defeated && !encounter.warden_spawned,
        RouteObjectiveId::ReachRelayStabilizer | RouteObjectiveId::ReachRelayDoor => {
            encounter.drone_defeated && !encounter.warden_spawned
        }
        RouteObjectiveId::DefeatWarden => {
            encounter.drone_defeated
                && encounter.warden_spawned
                && encounter.warden_remaining_health > 0
        }
    };
    if !consistent {
        return Err(route_error(
            "failed encounter disagrees with its terminal objective",
        ));
    }
    Ok(())
}

fn validate_player_hit_envelope(
    encounter: &ReplayRouteEncounterEvidence,
    selection: &RouteSelectedPayloadV1,
) -> Result<(), ReplayError> {
    if !encounter.warden_spawned || encounter.accepted_player_hits == 0 {
        return Ok(());
    }
    let damages = selection
        .participants
        .iter()
        .map(|participant| {
            participant
                .module_state
                .weapons
                .iter()
                .find(|weapon| weapon.item_id == participant.weapon_item_id)
                .map(|weapon| weapon.effective.damage)
                .ok_or_else(|| route_error("selected weapon profile disappeared"))
        })
        .collect::<Result<Vec<_>, ReplayError>>()?;
    let maximum_damage = damages
        .into_iter()
        .max()
        .ok_or_else(|| route_error("route has no admitted weapon damage"))?;
    let damage_done = encounter
        .warden_effective_health
        .checked_sub(encounter.warden_remaining_health)
        .ok_or_else(|| route_error("Warden remaining health exceeds effective health"))?;
    let admitted_capacity = encounter
        .accepted_player_hits
        .checked_mul(maximum_damage)
        .ok_or_else(|| route_error("player hit arithmetic overflowed"))?;
    if damage_done == 0 || admitted_capacity < damage_done {
        return Err(route_error(
            "accepted player hits cannot produce recorded Warden damage",
        ));
    }
    Ok(())
}

fn validate_grants(
    payload: &RouteTerminalPayloadV1,
    selection: &RouteSelectedPayloadV1,
    outcome: TerminalOutcome,
) -> Result<(), ReplayError> {
    if outcome == TerminalOutcome::FailedTimeout {
        if payload.grants.is_empty() {
            return Ok(());
        }
        return Err(route_error("failed route contains reward grants"));
    }
    if payload.grants.len() != selection.participants.len() {
        return Err(route_error("successful route grant count is incomplete"));
    }
    for (grant, participant) in payload.grants.iter().zip(&selection.participants) {
        if grant.participant_id != participant.participant_id
            || grant.character_id != participant.character_id
            || grant.actor_id != participant.actor_id
            || grant.item_id != RELAY_CORE_FRAGMENT
            || grant.item_quantity != selection.reward.fragments
            || grant.experience != selection.reward.experience
        {
            return Err(route_error("route grant disagrees with participant reward"));
        }
    }
    Ok(())
}

fn validate_generic_terminal_evidence(
    kind: ReplayEventKind,
    selection: &RouteSelectedPayloadV1,
    terminal: &RouteTerminalPayloadV1,
    activity_completions: usize,
    loot_grants: &BTreeMap<String, (u64, u32)>,
    progression_grants: &BTreeMap<String, (u64, u64)>,
) -> Result<(), ReplayError> {
    if kind == ReplayEventKind::RouteOperationFailed {
        if activity_completions == 0 && loot_grants.is_empty() && progression_grants.is_empty() {
            return Ok(());
        }
        return Err(route_error(
            "failed route contains generic completion or reward evidence",
        ));
    }
    if activity_completions != 1
        || loot_grants.len() != selection.participants.len()
        || progression_grants.len() != selection.participants.len()
    {
        return Err(route_error(
            "successful route generic completion/reward evidence is incomplete",
        ));
    }
    for (participant, grant) in selection.participants.iter().zip(&terminal.grants) {
        if loot_grants.get(&participant.participant_id)
            != Some(&(participant.actor_id, grant.item_quantity))
            || progression_grants.get(&participant.participant_id)
                != Some(&(participant.actor_id, grant.experience))
        {
            return Err(route_error(
                "generic reward replay disagrees with route terminal grants",
            ));
        }
    }
    Ok(())
}

const fn objective_kind(objective_id: RouteObjectiveId) -> ReplayRouteObjectiveKind {
    match objective_id {
        RouteObjectiveId::ClearDroneGroup => ReplayRouteObjectiveKind::KillActors,
        RouteObjectiveId::ReachRelayStabilizer | RouteObjectiveId::ReachRelayDoor => {
            ReplayRouteObjectiveKind::ReachArea
        }
        RouteObjectiveId::DefeatWarden => ReplayRouteObjectiveKind::Boss,
    }
}

fn validate_route_schema(schema_version: u8) -> Result<(), ReplayError> {
    if schema_version != ROUTE_REPLAY_SCHEMA_VERSION {
        return Err(route_error("unknown route replay payload schema version"));
    }
    Ok(())
}

fn validate_bounded_identity(value: &str, label: &str) -> Result<(), ReplayError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
    {
        return Err(route_error(format!(
            "route {label} identifier is outside bounds"
        )));
    }
    Ok(())
}

fn validate_snake_identifier(value: &str, label: &str) -> Result<(), ReplayError> {
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(route_error(format!(
            "route {label} identifier is outside bounds"
        )));
    }
    Ok(())
}

fn encode_route_payload(payload: &impl Serialize) -> Result<String, ReplayError> {
    let encoded = serde_json::to_string(payload)
        .map_err(|error| route_error(format!("route replay JSON serialization failed: {error}")))?;
    validate_route_payload_size(&encoded)?;
    Ok(encoded)
}

fn decode_route_json<'a, T: Deserialize<'a>>(payload: &'a str) -> Result<T, ReplayError> {
    serde_json::from_str(payload)
        .map_err(|error| route_error(format!("invalid route replay JSON: {error}")))
}

fn validate_route_payload_size(payload: &str) -> Result<(), ReplayError> {
    if payload.len() > MAX_ROUTE_REPLAY_PAYLOAD_BYTES {
        return Err(route_error("route replay payload exceeds 8192 UTF-8 bytes"));
    }
    Ok(())
}

fn route_fragment_quantity(payload: &str) -> Option<u32> {
    let suffix = payload.strip_prefix("loot granted: relay_core_fragment x")?;
    let (quantity, total) = suffix.split_once(" (total ")?;
    total.strip_suffix(')')?.parse::<u32>().ok()?;
    quantity.parse().ok()
}

fn route_experience_quantity(payload: &str) -> Option<u64> {
    let suffix = payload.strip_prefix("progression granted: +")?;
    let (experience, suffix) = suffix.split_once(" XP (total ")?;
    let (total, levels) = suffix.split_once(", level ")?;
    let levels = levels.strip_suffix(')')?;
    let (previous_level, level) = levels.split_once(" -> ")?;
    total.parse::<u64>().ok()?;
    previous_level.parse::<u32>().ok()?;
    level.parse::<u32>().ok()?;
    experience.parse().ok()
}

#[cfg(test)]
mod tests {
    use revenant_modules::ModuleId;
    use revenant_operations::{
        apply_warden_health_effect, resolve_event, route_definition, RouteId,
        RouteOperationSummary, RouteSeed, TerminalOutcome, CATALOG_REVISION, RESOLVER_REVISION,
    };

    use super::{
        encode_route_operation_failed, encode_route_operation_succeeded, encode_route_selected,
        ReconstructedRouteState, ReplayObjectiveTransition, ReplayRouteEncounterEvidence,
        ReplayRouteGrantEvidence, ReplayRouteObjectiveEvidence, ReplayRouteObjectiveKind,
        ReplayRouteObjectiveState, ReplayRouteParticipantEvidence, RouteSelectedPayloadV1,
        RouteTerminalPayloadV1, AUTHORING_REVISION, ROUTE_REPLAY_SCHEMA_VERSION,
    };
    use crate::{
        encode_module_state_snapshot, module_state_evidence, reconstruct,
        ModuleStateSnapshotPayloadV1, ReplayEvent, ReplayEventKind, ReplayProtocolGeneration,
        MODULE_REPLAY_SCHEMA_VERSION,
    };

    fn event(id: i64, kind: ReplayEventKind, payload: String) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            timestamp: format!("2026-01-01 00:00:{id:02}+00"),
            session_id: "route-session-1".to_owned(),
            account_id: "account-1".to_owned(),
            activity_id: Some("relay_awakening".to_owned()),
            actor_id: Some(10),
            payload,
        }
    }

    fn selection(route_id: RouteId, seed: u64) -> RouteSelectedPayloadV1 {
        let seed = RouteSeed::new(seed).expect("test seed should validate");
        let definition = route_definition(route_id);
        let resolved = resolve_event(route_id, seed);
        let module_state = module_state_evidence(
            0,
            &[ModuleId::ForceMatrix],
            &[ModuleId::ForceMatrix],
            1,
            ReplayProtocolGeneration::V2,
        )
        .expect("test module state should resolve");
        RouteSelectedPayloadV1 {
            schema_version: ROUTE_REPLAY_SCHEMA_VERSION,
            activity_id: "relay_awakening".to_owned(),
            authoring_revision: AUTHORING_REVISION.to_owned(),
            catalog_revision: CATALOG_REVISION.to_owned(),
            resolver_revision: RESOLVER_REVISION.to_owned(),
            operation_id: "route-operation-1".to_owned(),
            leader_id: "account-1".to_owned(),
            participants: vec![ReplayRouteParticipantEvidence {
                participant_id: "account-1".to_owned(),
                character_id: "account-1:operator".to_owned(),
                actor_id: 10,
                weapon_item_id: "pulse_rifle".to_owned(),
                module_state,
            }],
            route_id,
            seed,
            event_id: resolved.event_id,
            effect: resolved.effect,
            objectives: definition
                .objective_path
                .iter()
                .enumerate()
                .map(|(index, objective_id)| ReplayRouteObjectiveEvidence {
                    objective_id: *objective_id,
                    kind: match objective_id {
                        revenant_operations::RouteObjectiveId::ClearDroneGroup => {
                            ReplayRouteObjectiveKind::KillActors
                        }
                        revenant_operations::RouteObjectiveId::ReachRelayStabilizer
                        | revenant_operations::RouteObjectiveId::ReachRelayDoor => {
                            ReplayRouteObjectiveKind::ReachArea
                        }
                        revenant_operations::RouteObjectiveId::DefeatWarden => {
                            ReplayRouteObjectiveKind::Boss
                        }
                    },
                    initial_state: if index == 0 {
                        ReplayRouteObjectiveState::Active
                    } else {
                        ReplayRouteObjectiveState::Pending
                    },
                    target: 1,
                })
                .collect(),
            duration_budget_ms: definition.duration_ms,
            reward: definition.reward,
        }
    }

    fn admission_events(selection: &RouteSelectedPayloadV1) -> Vec<ReplayEvent> {
        let snapshot = ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: selection.participants[0].character_id.clone(),
            protocol_generation: ReplayProtocolGeneration::V2,
            state: selection.participants[0].module_state.clone(),
        };
        vec![
            event(1, ReplayEventKind::PlayerJoined, "player joined".to_owned()),
            event(
                2,
                ReplayEventKind::ModuleStateSnapshot,
                encode_module_state_snapshot(&snapshot).expect("snapshot should encode"),
            ),
            event(
                3,
                ReplayEventKind::ActivityStarted,
                "activity started".to_owned(),
            ),
            event(
                4,
                ReplayEventKind::RouteSelected,
                encode_route_selected(selection).expect("selection should encode"),
            ),
        ]
    }

    fn successful_terminal(selection: &RouteSelectedPayloadV1) -> RouteTerminalPayloadV1 {
        let mut transitions = Vec::new();
        let mut ordinal = 0_u8;
        for (index, objective) in selection.objectives.iter().enumerate() {
            if index > 0 {
                transitions.push(ReplayObjectiveTransition {
                    ordinal,
                    objective_id: objective.objective_id,
                    from: ReplayRouteObjectiveState::Pending,
                    to: ReplayRouteObjectiveState::Active,
                    progress: 0,
                    target: 1,
                });
                ordinal += 1;
            }
            transitions.push(ReplayObjectiveTransition {
                ordinal,
                objective_id: objective.objective_id,
                from: ReplayRouteObjectiveState::Active,
                to: ReplayRouteObjectiveState::Completed,
                progress: 1,
                target: 1,
            });
            ordinal += 1;
        }
        let warden_health = 240;
        let effective_health = apply_warden_health_effect(warden_health, selection.effect)
            .expect("event effect should apply");
        let weapon_damage = selection.participants[0].module_state.weapons[0]
            .effective
            .damage;
        let accepted_player_hits = effective_health.div_ceil(weapon_damage);
        let summary = summary(selection, 90_000, TerminalOutcome::Succeeded);
        RouteTerminalPayloadV1 {
            schema_version: ROUTE_REPLAY_SCHEMA_VERSION,
            authoring_revision: AUTHORING_REVISION.to_owned(),
            catalog_revision: CATALOG_REVISION.to_owned(),
            resolver_revision: RESOLVER_REVISION.to_owned(),
            operation_id: selection.operation_id.clone(),
            route_id: selection.route_id,
            event_id: selection.event_id,
            transitions,
            encounter: ReplayRouteEncounterEvidence {
                participant_count: 1,
                drone_health: 140,
                drone_damage: 10,
                drone_defeated: true,
                warden_spawned: true,
                warden_base_health: warden_health,
                warden_effective_health: effective_health,
                warden_remaining_health: 0,
                accepted_player_hits,
                warden_counter_count: 2,
                warden_counter_damage: selection.effect.warden_counter_damage,
                total_hostile_damage: 10 + 2 * selection.effect.warden_counter_damage,
            },
            summary,
            grants: vec![ReplayRouteGrantEvidence {
                participant_id: "account-1".to_owned(),
                character_id: "account-1:operator".to_owned(),
                actor_id: 10,
                item_id: "relay_core_fragment".to_owned(),
                item_quantity: selection.reward.fragments,
                experience: selection.reward.experience,
            }],
        }
    }

    fn failed_terminal(selection: &RouteSelectedPayloadV1) -> RouteTerminalPayloadV1 {
        RouteTerminalPayloadV1 {
            schema_version: ROUTE_REPLAY_SCHEMA_VERSION,
            authoring_revision: AUTHORING_REVISION.to_owned(),
            catalog_revision: CATALOG_REVISION.to_owned(),
            resolver_revision: RESOLVER_REVISION.to_owned(),
            operation_id: selection.operation_id.clone(),
            route_id: selection.route_id,
            event_id: selection.event_id,
            transitions: vec![ReplayObjectiveTransition {
                ordinal: 0,
                objective_id: selection.objectives[0].objective_id,
                from: ReplayRouteObjectiveState::Active,
                to: ReplayRouteObjectiveState::Failed,
                progress: 0,
                target: 1,
            }],
            encounter: ReplayRouteEncounterEvidence {
                participant_count: 1,
                drone_health: 140,
                drone_damage: 10,
                drone_defeated: false,
                warden_spawned: false,
                warden_base_health: 240,
                warden_effective_health: apply_warden_health_effect(240, selection.effect)
                    .expect("event effect should apply"),
                warden_remaining_health: 0,
                accepted_player_hits: 0,
                warden_counter_count: 0,
                warden_counter_damage: selection.effect.warden_counter_damage,
                total_hostile_damage: 0,
            },
            summary: summary(selection, 90_001, TerminalOutcome::FailedTimeout),
            grants: Vec::new(),
        }
    }

    fn summary(
        selection: &RouteSelectedPayloadV1,
        elapsed_ms: u64,
        outcome: TerminalOutcome,
    ) -> RouteOperationSummary {
        RouteOperationSummary {
            catalog_revision: CATALOG_REVISION.to_owned(),
            resolver_revision: RESOLVER_REVISION.to_owned(),
            operation_id: selection.operation_id.clone(),
            leader_id: selection.leader_id.clone(),
            participant_ids: vec![selection.leader_id.clone()],
            route_id: selection.route_id,
            seed: selection.seed,
            event_id: selection.event_id,
            effect: selection.effect,
            objective_path: selection
                .objectives
                .iter()
                .map(|objective| objective.objective_id)
                .collect(),
            duration_budget_ms: selection.duration_budget_ms,
            elapsed_ms,
            outcome,
            reward: (outcome == TerminalOutcome::Succeeded).then_some(selection.reward),
        }
    }

    #[test]
    fn acquisition_requires_success_and_preserves_each_route_identity() {
        use revenant_modules::acquisition::Milestone;
        for (route_id, expected) in [
            (RouteId::Breach, Milestone::BreachCompleted),
            (RouteId::Stabilize, Milestone::StabilizeCompleted),
        ] {
            let selection = selection(route_id, 0);
            let mut events = admission_events(&selection);
            assert!(crate::acquisition_milestones(&events, "account-1:operator")
                .unwrap()
                .is_empty());
            events.extend([
                event(
                    5,
                    ReplayEventKind::ActivityCompleted,
                    "activity completed".to_owned(),
                ),
                event(
                    6,
                    ReplayEventKind::LootGranted,
                    format!(
                        "loot granted: relay_core_fragment x{} (total {})",
                        selection.reward.fragments, selection.reward.fragments
                    ),
                ),
                event(
                    7,
                    ReplayEventKind::ProgressionGranted,
                    format!(
                        "progression granted: +{} XP (total {}, level 1 -> 1)",
                        selection.reward.experience, selection.reward.experience
                    ),
                ),
                event(
                    8,
                    ReplayEventKind::RouteOperationSucceeded,
                    encode_route_operation_succeeded(&successful_terminal(&selection), &selection)
                        .unwrap(),
                ),
            ]);
            assert_eq!(
                crate::acquisition_milestones(&events, "account-1:operator").unwrap(),
                [expected]
            );
            assert!(crate::acquisition_milestones(&events, "another-character")
                .unwrap()
                .is_empty());
        }
    }

    #[test]
    fn reconstructs_success_from_selection_generic_grants_and_terminal() {
        let selection = selection(RouteId::Breach, 0);
        let terminal = successful_terminal(&selection);
        let mut events = admission_events(&selection);
        events.extend([
            event(
                5,
                ReplayEventKind::ActivityCompleted,
                "activity completed".to_owned(),
            ),
            event(
                6,
                ReplayEventKind::LootGranted,
                format!(
                    "loot granted: relay_core_fragment x{} (total {})",
                    selection.reward.fragments, selection.reward.fragments
                ),
            ),
            event(
                7,
                ReplayEventKind::ProgressionGranted,
                format!(
                    "progression granted: +{} XP (total {}, level 1 -> 1)",
                    selection.reward.experience, selection.reward.experience
                ),
            ),
            event(
                8,
                ReplayEventKind::RouteOperationSucceeded,
                encode_route_operation_succeeded(&terminal, &selection)
                    .expect("terminal should encode"),
            ),
        ]);
        let reconstructed = reconstruct(&events).expect("route replay should reconstruct");
        assert_eq!(
            reconstructed.route.state,
            ReconstructedRouteState::Succeeded
        );
        assert_eq!(
            reconstructed
                .route
                .selection
                .as_ref()
                .expect("selection")
                .route_id,
            RouteId::Breach
        );
        assert_eq!(
            reconstructed
                .route
                .terminal
                .as_ref()
                .expect("terminal")
                .grants
                .len(),
            1
        );
    }

    #[test]
    fn reconstructs_incomplete_and_failed_routes_without_fabricated_rewards() {
        let selection = selection(RouteId::Stabilize, 3);
        let events = admission_events(&selection);
        let incomplete = reconstruct(&events).expect("incomplete route should reconstruct");
        assert_eq!(
            incomplete.route.state,
            ReconstructedRouteState::RoutedIncomplete
        );
        assert!(incomplete.route.terminal.is_none());

        let terminal = failed_terminal(&selection);
        let mut events = events;
        events.push(event(
            5,
            ReplayEventKind::RouteOperationFailed,
            encode_route_operation_failed(&terminal, &selection)
                .expect("failed terminal should encode"),
        ));
        let failed = reconstruct(&events).expect("failed route should reconstruct");
        assert_eq!(failed.route.state, ReconstructedRouteState::Failed);
        assert!(!failed.completed);
        assert_eq!(failed.loot_grants, 0);
        assert_eq!(failed.progression_grants, 0);
    }

    #[test]
    fn rejects_seed_transition_generic_grant_and_unknown_field_corruption() {
        let selection = selection(RouteId::Breach, 0);
        let terminal = successful_terminal(&selection);
        let mut events = admission_events(&selection);
        events[3].payload = events[3].payload.replacen("\"seed\":0", "\"seed\":1", 1);
        assert!(reconstruct(&events).is_err());

        let nested_selection = encode_route_selected(&selection)
            .expect("selection should encode")
            .replacen("\"effect\":{", "\"effect\":{\"unexpected\":true,", 1);
        assert!(
            super::decode_route_payload(ReplayEventKind::RouteSelected, &nested_selection).is_err()
        );

        let nested_terminal = encode_route_operation_succeeded(&terminal, &selection)
            .expect("terminal should encode")
            .replacen("\"summary\":{", "\"summary\":{\"unexpected\":true,", 1);
        assert!(super::decode_route_payload(
            ReplayEventKind::RouteOperationSucceeded,
            &nested_terminal
        )
        .is_err());

        let mut events = admission_events(&selection);
        let mut corrupt_terminal = terminal.clone();
        corrupt_terminal.transitions[0].ordinal = 1;
        assert!(encode_route_operation_succeeded(&corrupt_terminal, &selection).is_err());

        events.extend([
            event(
                5,
                ReplayEventKind::ActivityCompleted,
                "activity completed".to_owned(),
            ),
            event(
                6,
                ReplayEventKind::LootGranted,
                "loot granted: relay_core_fragment x1 (total 1)".to_owned(),
            ),
            event(
                7,
                ReplayEventKind::ProgressionGranted,
                format!(
                    "progression granted: +{} XP (total {}, level 1 -> 1)",
                    selection.reward.experience, selection.reward.experience
                ),
            ),
            event(
                8,
                ReplayEventKind::RouteOperationSucceeded,
                encode_route_operation_succeeded(&terminal, &selection)
                    .expect("terminal should encode")
                    .replacen(
                        "\"schema_version\":1",
                        "\"schema_version\":1,\"extra\":true",
                        1,
                    ),
            ),
        ]);
        assert!(reconstruct(&events).is_err());
    }

    #[test]
    fn rejects_order_identity_generation_deadline_and_size_corruption() {
        let selection = selection(RouteId::Breach, 0);
        let terminal = successful_terminal(&selection);

        let terminal_only = [event(
            1,
            ReplayEventKind::RouteOperationSucceeded,
            encode_route_operation_succeeded(&terminal, &selection)
                .expect("terminal should encode"),
        )];
        assert!(reconstruct(&terminal_only).is_err());

        let mut duplicate_selection = admission_events(&selection);
        duplicate_selection.push(event(
            5,
            ReplayEventKind::RouteSelected,
            encode_route_selected(&selection).expect("selection should encode"),
        ));
        assert!(reconstruct(&duplicate_selection).is_err());

        let mut actor_mismatch = admission_events(&selection);
        actor_mismatch[3].actor_id = Some(11);
        assert!(reconstruct(&actor_mismatch).is_err());

        let mut pair_selection = selection.clone();
        pair_selection
            .participants
            .push(ReplayRouteParticipantEvidence {
                participant_id: "account-2".to_owned(),
                character_id: "account-2:operator".to_owned(),
                actor_id: 20,
                weapon_item_id: "pulse_rifle".to_owned(),
                module_state: selection.participants[0].module_state.clone(),
            });
        let partner_snapshot = ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: pair_selection.participants[1].character_id.clone(),
            protocol_generation: ReplayProtocolGeneration::V2,
            state: pair_selection.participants[1].module_state.clone(),
        };
        let leader_snapshot = ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: pair_selection.participants[0].character_id.clone(),
            protocol_generation: ReplayProtocolGeneration::V2,
            state: pair_selection.participants[0].module_state.clone(),
        };
        let mut partner_join = event(1, ReplayEventKind::PlayerJoined, "player joined".to_owned());
        partner_join.account_id = "account-2".to_owned();
        partner_join.actor_id = Some(20);
        let mut partner_snapshot_event = event(
            2,
            ReplayEventKind::ModuleStateSnapshot,
            encode_module_state_snapshot(&partner_snapshot).expect("snapshot should encode"),
        );
        partner_snapshot_event.account_id = "account-2".to_owned();
        partner_snapshot_event.actor_id = Some(20);
        let wrong_admission_order = vec![
            partner_join,
            partner_snapshot_event,
            event(3, ReplayEventKind::PlayerJoined, "player joined".to_owned()),
            event(
                4,
                ReplayEventKind::ModuleStateSnapshot,
                encode_module_state_snapshot(&leader_snapshot).expect("snapshot should encode"),
            ),
            event(
                5,
                ReplayEventKind::ActivityStarted,
                "activity started".to_owned(),
            ),
            event(
                6,
                ReplayEventKind::RouteSelected,
                encode_route_selected(&pair_selection).expect("selection should encode"),
            ),
        ];
        assert!(reconstruct(&wrong_admission_order).is_err());

        let mut late_admission = admission_events(&selection);
        let mut partner_join = event(5, ReplayEventKind::PlayerJoined, "player joined".to_owned());
        partner_join.account_id = "account-2".to_owned();
        partner_join.actor_id = Some(20);
        let mut partner_snapshot_event = event(
            6,
            ReplayEventKind::ModuleStateSnapshot,
            encode_module_state_snapshot(&partner_snapshot).expect("snapshot should encode"),
        );
        partner_snapshot_event.account_id = "account-2".to_owned();
        partner_snapshot_event.actor_id = Some(20);
        late_admission.extend([partner_join, partner_snapshot_event]);
        assert!(reconstruct(&late_admission).is_err());

        let mut inactive = selection.clone();
        inactive.participants[0].module_state =
            module_state_evidence(0, &[], &[], 0, ReplayProtocolGeneration::V1)
                .expect("V1 module state should resolve");
        assert!(encode_route_selected(&inactive).is_err());

        let mut late_success = terminal.clone();
        late_success.summary.elapsed_ms = 90_001;
        assert!(encode_route_operation_succeeded(&late_success, &selection).is_err());

        let oversized = format!("{{\"padding\":\"{}\"}}", "x".repeat(8_192));
        assert!(super::decode_route_payload(ReplayEventKind::RouteSelected, &oversized).is_err());
    }

    #[test]
    fn rejects_duplicate_terminal_and_failure_with_generic_completion() {
        let selection = selection(RouteId::Stabilize, 3);
        let failure = failed_terminal(&selection);
        let failure_payload =
            encode_route_operation_failed(&failure, &selection).expect("failure should encode");
        let mut duplicate_terminal = admission_events(&selection);
        duplicate_terminal.extend([
            event(
                5,
                ReplayEventKind::RouteOperationFailed,
                failure_payload.clone(),
            ),
            event(
                6,
                ReplayEventKind::RouteOperationFailed,
                failure_payload.clone(),
            ),
        ]);
        assert!(reconstruct(&duplicate_terminal).is_err());

        let mut rewarded_failure = admission_events(&selection);
        rewarded_failure.extend([
            event(
                5,
                ReplayEventKind::ActivityCompleted,
                "activity completed".to_owned(),
            ),
            event(6, ReplayEventKind::RouteOperationFailed, failure_payload),
        ]);
        assert!(reconstruct(&rewarded_failure).is_err());

        let mut post_terminal_completion = admission_events(&selection);
        post_terminal_completion.extend([
            event(
                5,
                ReplayEventKind::RouteOperationFailed,
                encode_route_operation_failed(&failure, &selection).expect("failure should encode"),
            ),
            event(
                6,
                ReplayEventKind::ActivityCompleted,
                "activity completed".to_owned(),
            ),
        ]);
        assert!(reconstruct(&post_terminal_completion).is_err());

        assert!(super::route_fragment_quantity(
            "loot granted: relay_core_fragment x1 (total 1) trailing"
        )
        .is_none());
        assert!(super::route_experience_quantity(
            "progression granted: +100 XP (total 100, level 1 -> 1) trailing"
        )
        .is_none());
    }
}
