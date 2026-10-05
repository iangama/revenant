use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const CATALOG_REVISION: &str = "m27-v1";
pub const RESOLVER_REVISION: &str = "m27-permute63-v1";
pub const ROUTE_OPERATION_DURATION_MS: u64 = 90_000;
pub const MAX_ROUTE_SEED: u64 = i64::MAX.unsigned_abs();
pub const MAX_ROUTE_OPERATION_ID_BYTES: usize = 32;
pub const MAX_ROUTE_PARTICIPANTS: usize = 2;
pub const MAX_ROUTE_OBJECTIVES: usize = 4;
pub const MAX_OPTIMAL_INCOMING_DAMAGE: u32 = 50;

const BASIS_POINTS: u64 = 10_000;
const BASE_WARDEN_COUNTER_DAMAGE: u32 = 15;
const MIN_WARDEN_COUNTER_DAMAGE: u32 = 10;
const MAX_WARDEN_COUNTER_DAMAGE: u32 = 20;
const MIN_WARDEN_HEALTH_BASIS_POINTS: u32 = 10_000;
const MAX_WARDEN_HEALTH_BASIS_POINTS: u32 = 12_000;
const MIN_ROUTE_DURATION_MS: u64 = 30_000;
const MAX_ROUTE_DURATION_MS: u64 = 120_000;
const MAX_PARTICIPANT_ID_BYTES: usize = 64;
const BREACH_SALT: u64 = 0x4252_4541_4348_0001;
const STABILIZE_SALT: u64 = 0x5354_4142_494c_0002;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteId {
    Breach,
    Stabilize,
}

impl RouteId {
    pub const ALL: [Self; 2] = [Self::Breach, Self::Stabilize];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Breach => "breach",
            Self::Stabilize => "stabilize",
        }
    }
}

impl Display for RouteId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for RouteId {
    type Err = RouteError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|route_id| route_id.as_str() == value)
            .ok_or_else(|| RouteError::UnknownRoute(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteEventId {
    OverchargedArmor,
    ArcSurge,
    ShieldedChannel,
    ResidualFeedback,
}

impl RouteEventId {
    pub const ALL: [Self; 4] = [
        Self::OverchargedArmor,
        Self::ArcSurge,
        Self::ShieldedChannel,
        Self::ResidualFeedback,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OverchargedArmor => "overcharged_armor",
            Self::ArcSurge => "arc_surge",
            Self::ShieldedChannel => "shielded_channel",
            Self::ResidualFeedback => "residual_feedback",
        }
    }

    #[must_use]
    pub const fn route(self) -> RouteId {
        match self {
            Self::OverchargedArmor | Self::ArcSurge => RouteId::Breach,
            Self::ShieldedChannel | Self::ResidualFeedback => RouteId::Stabilize,
        }
    }
}

impl Display for RouteEventId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteObjectiveId {
    ClearDroneGroup,
    ReachRelayStabilizer,
    ReachRelayDoor,
    DefeatWarden,
}

impl RouteObjectiveId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClearDroneGroup => "clear_drone_group",
            Self::ReachRelayStabilizer => "reach_relay_stabilizer",
            Self::ReachRelayDoor => "reach_relay_door",
            Self::DefeatWarden => "defeat_warden",
        }
    }
}

impl Display for RouteObjectiveId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteReward {
    pub fragments: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteEffect {
    pub warden_health_basis_points: u32,
    pub warden_counter_damage: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RouteDefinition {
    pub route_id: RouteId,
    pub reward: RouteReward,
    pub duration_ms: u64,
    pub objective_path: &'static [RouteObjectiveId],
    pub events: [RouteEventId; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEventDefinition {
    pub event_id: RouteEventId,
    pub route_id: RouteId,
    pub effect: RouteEffect,
}

const BREACH_OBJECTIVES: [RouteObjectiveId; 3] = [
    RouteObjectiveId::ClearDroneGroup,
    RouteObjectiveId::ReachRelayDoor,
    RouteObjectiveId::DefeatWarden,
];
const STABILIZE_OBJECTIVES: [RouteObjectiveId; 4] = [
    RouteObjectiveId::ClearDroneGroup,
    RouteObjectiveId::ReachRelayStabilizer,
    RouteObjectiveId::ReachRelayDoor,
    RouteObjectiveId::DefeatWarden,
];

pub const ROUTE_CATALOG: [RouteDefinition; 2] = [
    RouteDefinition {
        route_id: RouteId::Breach,
        reward: RouteReward {
            fragments: 2,
            experience: 100,
        },
        duration_ms: ROUTE_OPERATION_DURATION_MS,
        objective_path: &BREACH_OBJECTIVES,
        events: [RouteEventId::OverchargedArmor, RouteEventId::ArcSurge],
    },
    RouteDefinition {
        route_id: RouteId::Stabilize,
        reward: RouteReward {
            fragments: 1,
            experience: 150,
        },
        duration_ms: ROUTE_OPERATION_DURATION_MS,
        objective_path: &STABILIZE_OBJECTIVES,
        events: [
            RouteEventId::ShieldedChannel,
            RouteEventId::ResidualFeedback,
        ],
    },
];

pub const ROUTE_EVENT_CATALOG: [RouteEventDefinition; 4] = [
    RouteEventDefinition {
        event_id: RouteEventId::OverchargedArmor,
        route_id: RouteId::Breach,
        effect: RouteEffect {
            warden_health_basis_points: 12_000,
            warden_counter_damage: BASE_WARDEN_COUNTER_DAMAGE,
        },
    },
    RouteEventDefinition {
        event_id: RouteEventId::ArcSurge,
        route_id: RouteId::Breach,
        effect: RouteEffect {
            warden_health_basis_points: 10_000,
            warden_counter_damage: 20,
        },
    },
    RouteEventDefinition {
        event_id: RouteEventId::ShieldedChannel,
        route_id: RouteId::Stabilize,
        effect: RouteEffect {
            warden_health_basis_points: 10_000,
            warden_counter_damage: 10,
        },
    },
    RouteEventDefinition {
        event_id: RouteEventId::ResidualFeedback,
        route_id: RouteId::Stabilize,
        effect: RouteEffect {
            warden_health_basis_points: 11_000,
            warden_counter_damage: BASE_WARDEN_COUNTER_DAMAGE,
        },
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteSeed(u64);

impl RouteSeed {
    /// Creates a server-owned non-negative 63-bit route seed.
    ///
    /// # Errors
    ///
    /// Returns [`RouteError::InvalidSeed`] when `value` exceeds the signed
    /// 64-bit persistence envelope.
    pub const fn new(value: u64) -> Result<Self, RouteError> {
        if value > MAX_ROUTE_SEED {
            return Err(RouteError::InvalidSeed(value));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRouteEvent {
    pub catalog_revision: &'static str,
    pub resolver_revision: &'static str,
    pub route_id: RouteId,
    pub seed: RouteSeed,
    pub event_id: RouteEventId,
    pub effect: RouteEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePhase {
    Drone,
    ChoiceOpen,
    BaselineLocked,
    Routed,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationDisposition {
    Applied,
    Replayed,
}

#[derive(Debug, Clone, Copy)]
pub struct RouteSelectionRequest<'a> {
    pub operation_id: &'a str,
    pub route_id: RouteId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSelectionOutcome {
    pub operation_id: String,
    pub leader_id: String,
    pub route_id: RouteId,
    pub seed: RouteSeed,
    pub event_id: RouteEventId,
    pub effect: RouteEffect,
    pub reward: RouteReward,
    pub objective_path: Vec<RouteObjectiveId>,
    pub duration_budget_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationReceipt<T> {
    pub disposition: OperationDisposition,
    pub outcome: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    Succeeded,
    FailedTimeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteOperationSummary {
    pub catalog_revision: String,
    pub resolver_revision: String,
    pub operation_id: String,
    pub leader_id: String,
    pub participant_ids: Vec<String>,
    pub route_id: RouteId,
    pub seed: RouteSeed,
    pub event_id: RouteEventId,
    pub effect: RouteEffect,
    pub objective_path: Vec<RouteObjectiveId>,
    pub duration_budget_ms: u64,
    pub elapsed_ms: u64,
    pub outcome: TerminalOutcome,
    pub reward: Option<RouteReward>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParticipantCapability {
    participant_id: String,
    capable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteOperationState {
    participants: Vec<ParticipantCapability>,
    phase: RoutePhase,
    selection: Option<RouteSelectionOutcome>,
    terminal: Option<RouteOperationSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    UnknownRoute(String),
    InvalidSeed(u64),
    InvalidCatalog(&'static str),
    InvalidOperationId,
    InvalidParticipants,
    UnknownParticipant(String),
    NotLeader,
    CapabilityIncomplete,
    WrongPhase(RoutePhase),
    IdempotencyConflict,
    RouteAlreadySelected,
    DeadlineExceeded { elapsed_ms: u64, budget_ms: u64 },
    DeadlineNotReached { elapsed_ms: u64, budget_ms: u64 },
    TerminalConflict,
    ArithmeticOverflow,
}

impl Display for RouteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRoute(value) => write!(formatter, "unknown route: {value}"),
            Self::InvalidSeed(value) => write!(formatter, "route seed exceeds 63 bits: {value}"),
            Self::InvalidCatalog(message) => write!(formatter, "invalid route catalog: {message}"),
            Self::InvalidOperationId => formatter.write_str(
                "operation identifier must contain 1-32 ASCII alphanumeric/hyphen characters",
            ),
            Self::InvalidParticipants => formatter.write_str(
                "route operation requires one or two unique bounded participant identifiers",
            ),
            Self::UnknownParticipant(value) => {
                write!(formatter, "unknown route participant: {value}")
            }
            Self::NotLeader => {
                formatter.write_str("only the first admitted participant may choose")
            }
            Self::CapabilityIncomplete => {
                formatter.write_str("every participant must opt into route capability")
            }
            Self::WrongPhase(phase) => {
                write!(formatter, "route operation is locked during {phase:?}")
            }
            Self::IdempotencyConflict => formatter
                .write_str("operation identifier was already used with different route input"),
            Self::RouteAlreadySelected => {
                formatter.write_str("a route is already selected for this session")
            }
            Self::DeadlineExceeded {
                elapsed_ms,
                budget_ms,
            } => write!(
                formatter,
                "route completion at {elapsed_ms} ms exceeds {budget_ms} ms budget"
            ),
            Self::DeadlineNotReached {
                elapsed_ms,
                budget_ms,
            } => write!(
                formatter,
                "route timeout at {elapsed_ms} ms has not exceeded {budget_ms} ms budget"
            ),
            Self::TerminalConflict => {
                formatter.write_str("route operation already has a different terminal result")
            }
            Self::ArithmeticOverflow => formatter.write_str("route arithmetic overflowed"),
        }
    }
}

impl Error for RouteError {}

impl RouteOperationState {
    /// Creates a bounded session operation in the pre-choice drone phase.
    ///
    /// Participant order is authoritative join order and the first entry is the
    /// leader.
    ///
    /// # Errors
    ///
    /// Returns [`RouteError::InvalidParticipants`] unless there are one or two
    /// unique bounded ASCII identifiers.
    pub fn new(participant_ids: &[&str]) -> Result<Self, RouteError> {
        if participant_ids.is_empty() || participant_ids.len() > MAX_ROUTE_PARTICIPANTS {
            return Err(RouteError::InvalidParticipants);
        }
        let mut unique = BTreeSet::new();
        let mut participants = Vec::with_capacity(participant_ids.len());
        for participant_id in participant_ids {
            if !valid_participant_id(participant_id) || !unique.insert(*participant_id) {
                return Err(RouteError::InvalidParticipants);
            }
            participants.push(ParticipantCapability {
                participant_id: (*participant_id).to_owned(),
                capable: false,
            });
        }
        Ok(Self {
            participants,
            phase: RoutePhase::Drone,
            selection: None,
            terminal: None,
        })
    }

    #[must_use]
    pub const fn phase(&self) -> RoutePhase {
        self.phase
    }

    #[must_use]
    pub fn leader_id(&self) -> &str {
        &self.participants[0].participant_id
    }

    #[must_use]
    pub fn all_capable(&self) -> bool {
        self.participants
            .iter()
            .all(|participant| participant.capable)
    }

    #[must_use]
    pub const fn selection(&self) -> Option<&RouteSelectionOutcome> {
        self.selection.as_ref()
    }

    #[must_use]
    pub const fn terminal(&self) -> Option<&RouteOperationSummary> {
        self.terminal.as_ref()
    }

    /// Marks one admitted participant as explicitly route-capable.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown participant or after the choice window.
    pub fn mark_capable(&mut self, participant_id: &str) -> Result<(), RouteError> {
        if !matches!(self.phase, RoutePhase::Drone | RoutePhase::ChoiceOpen) {
            return Err(RouteError::WrongPhase(self.phase));
        }
        let participant = self
            .participants
            .iter_mut()
            .find(|participant| participant.participant_id == participant_id)
            .ok_or_else(|| RouteError::UnknownParticipant(participant_id.to_owned()))?;
        participant.capable = true;
        Ok(())
    }

    /// Opens the route-choice window after the drone objective completes.
    ///
    /// # Errors
    ///
    /// Returns [`RouteError::WrongPhase`] unless the operation is in Drone.
    pub fn open_choice(&mut self) -> Result<(), RouteError> {
        if self.phase != RoutePhase::Drone {
            return Err(RouteError::WrongPhase(self.phase));
        }
        self.phase = RoutePhase::ChoiceOpen;
        Ok(())
    }

    /// Locks the established unrouted compatibility path.
    ///
    /// # Errors
    ///
    /// Returns [`RouteError::WrongPhase`] outside an unselected choice window.
    pub fn lock_baseline(&mut self) -> Result<(), RouteError> {
        if self.phase != RoutePhase::ChoiceOpen || self.selection.is_some() {
            return Err(RouteError::WrongPhase(self.phase));
        }
        self.phase = RoutePhase::BaselineLocked;
        Ok(())
    }

    /// Accepts one leader-owned route choice and resolves exactly one event.
    ///
    /// A same-input retry returns the stored result and ignores the supplied
    /// candidate seed, preventing a caller mistake from rerolling the event.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed/conflicting input, unknown or non-leader
    /// requester, incomplete capability, wrong phase, or invalid catalog.
    pub fn select(
        &mut self,
        requester_id: &str,
        request: RouteSelectionRequest<'_>,
        candidate_seed: RouteSeed,
    ) -> Result<OperationReceipt<RouteSelectionOutcome>, RouteError> {
        validate_operation_id(request.operation_id)?;
        if !self
            .participants
            .iter()
            .any(|participant| participant.participant_id == requester_id)
        {
            return Err(RouteError::UnknownParticipant(requester_id.to_owned()));
        }
        if let Some(selection) = &self.selection {
            if selection.operation_id == request.operation_id {
                if selection.route_id != request.route_id || selection.leader_id != requester_id {
                    return Err(RouteError::IdempotencyConflict);
                }
                return Ok(OperationReceipt {
                    disposition: OperationDisposition::Replayed,
                    outcome: selection.clone(),
                });
            }
            return Err(RouteError::RouteAlreadySelected);
        }
        if self.phase != RoutePhase::ChoiceOpen {
            return Err(RouteError::WrongPhase(self.phase));
        }
        if requester_id != self.leader_id() {
            return Err(RouteError::NotLeader);
        }
        if !self.all_capable() {
            return Err(RouteError::CapabilityIncomplete);
        }
        validate_catalog(&ROUTE_CATALOG, &ROUTE_EVENT_CATALOG)?;
        let definition = route_definition(request.route_id);
        let resolved = resolve_event(request.route_id, candidate_seed);
        let outcome = RouteSelectionOutcome {
            operation_id: request.operation_id.to_owned(),
            leader_id: requester_id.to_owned(),
            route_id: request.route_id,
            seed: candidate_seed,
            event_id: resolved.event_id,
            effect: resolved.effect,
            reward: definition.reward,
            objective_path: definition.objective_path.to_vec(),
            duration_budget_ms: definition.duration_ms,
        };
        self.phase = RoutePhase::Routed;
        self.selection = Some(outcome.clone());
        Ok(OperationReceipt {
            disposition: OperationDisposition::Applied,
            outcome,
        })
    }

    /// Completes a routed operation at or before its monotonic deadline.
    ///
    /// # Errors
    ///
    /// Returns an error when no route is active, the deadline was exceeded, or
    /// a different terminal result already exists.
    pub fn complete(
        &mut self,
        elapsed_ms: u64,
    ) -> Result<OperationReceipt<RouteOperationSummary>, RouteError> {
        self.finish(TerminalOutcome::Succeeded, elapsed_ms)
    }

    /// Fails a routed operation on the first observation after its deadline.
    ///
    /// # Errors
    ///
    /// Returns an error when no route is active, the deadline has not been
    /// exceeded, or a different terminal result already exists.
    pub fn fail_timeout(
        &mut self,
        elapsed_ms: u64,
    ) -> Result<OperationReceipt<RouteOperationSummary>, RouteError> {
        self.finish(TerminalOutcome::FailedTimeout, elapsed_ms)
    }

    fn finish(
        &mut self,
        requested_outcome: TerminalOutcome,
        elapsed_ms: u64,
    ) -> Result<OperationReceipt<RouteOperationSummary>, RouteError> {
        if let Some(summary) = &self.terminal {
            if summary.outcome == requested_outcome && summary.elapsed_ms == elapsed_ms {
                return Ok(OperationReceipt {
                    disposition: OperationDisposition::Replayed,
                    outcome: summary.clone(),
                });
            }
            return Err(RouteError::TerminalConflict);
        }
        if self.phase != RoutePhase::Routed {
            return Err(RouteError::WrongPhase(self.phase));
        }
        let selection = self
            .selection
            .as_ref()
            .ok_or(RouteError::WrongPhase(self.phase))?;
        match requested_outcome {
            TerminalOutcome::Succeeded if elapsed_ms > selection.duration_budget_ms => {
                return Err(RouteError::DeadlineExceeded {
                    elapsed_ms,
                    budget_ms: selection.duration_budget_ms,
                });
            }
            TerminalOutcome::FailedTimeout if elapsed_ms <= selection.duration_budget_ms => {
                return Err(RouteError::DeadlineNotReached {
                    elapsed_ms,
                    budget_ms: selection.duration_budget_ms,
                });
            }
            _ => {}
        }
        let summary = RouteOperationSummary {
            catalog_revision: CATALOG_REVISION.to_owned(),
            resolver_revision: RESOLVER_REVISION.to_owned(),
            operation_id: selection.operation_id.clone(),
            leader_id: selection.leader_id.clone(),
            participant_ids: self
                .participants
                .iter()
                .map(|participant| participant.participant_id.clone())
                .collect(),
            route_id: selection.route_id,
            seed: selection.seed,
            event_id: selection.event_id,
            effect: selection.effect,
            objective_path: selection.objective_path.clone(),
            duration_budget_ms: selection.duration_budget_ms,
            elapsed_ms,
            outcome: requested_outcome,
            reward: (requested_outcome == TerminalOutcome::Succeeded).then_some(selection.reward),
        };
        self.phase = match requested_outcome {
            TerminalOutcome::Succeeded => RoutePhase::Succeeded,
            TerminalOutcome::FailedTimeout => RoutePhase::Failed,
        };
        self.terminal = Some(summary.clone());
        Ok(OperationReceipt {
            disposition: OperationDisposition::Applied,
            outcome: summary,
        })
    }
}

/// Returns the fixed M27 route definition.
#[must_use]
pub const fn route_definition(route_id: RouteId) -> RouteDefinition {
    match route_id {
        RouteId::Breach => ROUTE_CATALOG[0],
        RouteId::Stabilize => ROUTE_CATALOG[1],
    }
}

/// Returns the fixed M27 event definition.
#[must_use]
pub const fn event_definition(event_id: RouteEventId) -> RouteEventDefinition {
    match event_id {
        RouteEventId::OverchargedArmor => ROUTE_EVENT_CATALOG[0],
        RouteEventId::ArcSurge => ROUTE_EVENT_CATALOG[1],
        RouteEventId::ShieldedChannel => ROUTE_EVENT_CATALOG[2],
        RouteEventId::ResidualFeedback => ROUTE_EVENT_CATALOG[3],
    }
}

/// Resolves one of the selected route's two events from a stable 63-bit seed.
#[must_use]
pub fn resolve_event(route_id: RouteId, seed: RouteSeed) -> ResolvedRouteEvent {
    let definition = route_definition(route_id);
    let salt = match route_id {
        RouteId::Breach => BREACH_SALT,
        RouteId::Stabilize => STABILIZE_SALT,
    };
    let ticket = permute_63(seed.value() ^ salt);
    let index = usize::from((ticket & 1) != 0);
    let event_id = definition.events[index];
    ResolvedRouteEvent {
        catalog_revision: CATALOG_REVISION,
        resolver_revision: RESOLVER_REVISION,
        route_id,
        seed,
        event_id,
        effect: event_definition(event_id).effect,
    }
}

/// Applies a bounded Warden health multiplier with checked round-half-up math.
///
/// # Errors
///
/// Returns an error for zero health, an out-of-catalog multiplier, or checked
/// arithmetic/conversion failure.
pub fn apply_warden_health_effect(
    base_health: u32,
    effect: RouteEffect,
) -> Result<u32, RouteError> {
    if base_health == 0
        || !(MIN_WARDEN_HEALTH_BASIS_POINTS..=MAX_WARDEN_HEALTH_BASIS_POINTS)
            .contains(&effect.warden_health_basis_points)
        || !(MIN_WARDEN_COUNTER_DAMAGE..=MAX_WARDEN_COUNTER_DAMAGE)
            .contains(&effect.warden_counter_damage)
    {
        return Err(RouteError::InvalidCatalog("event effect is outside bounds"));
    }
    let scaled = u64::from(base_health)
        .checked_mul(u64::from(effect.warden_health_basis_points))
        .and_then(|value| value.checked_add(BASIS_POINTS / 2))
        .ok_or(RouteError::ArithmeticOverflow)?
        / BASIS_POINTS;
    u32::try_from(scaled).map_err(|_| RouteError::ArithmeticOverflow)
}

/// Validates the finite route and event catalogs and all Gate 2 envelopes.
///
/// # Errors
///
/// Returns [`RouteError::InvalidCatalog`] when cardinality, identity, reward,
/// objective, duration, or effect bounds are violated.
pub fn validate_catalog(
    routes: &[RouteDefinition],
    events: &[RouteEventDefinition],
) -> Result<(), RouteError> {
    if routes.len() != RouteId::ALL.len() || events.len() != RouteEventId::ALL.len() {
        return Err(RouteError::InvalidCatalog(
            "catalog must contain exactly two routes and four events",
        ));
    }
    let mut route_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    for route in routes {
        if !route_ids.insert(route.route_id) {
            return Err(RouteError::InvalidCatalog(
                "route identifiers must be unique",
            ));
        }
        if !(1..=2).contains(&route.reward.fragments)
            || !matches!(route.reward.experience, 100 | 125 | 150 | 175)
        {
            return Err(RouteError::InvalidCatalog("route reward is outside bounds"));
        }
        if !(MIN_ROUTE_DURATION_MS..=MAX_ROUTE_DURATION_MS).contains(&route.duration_ms) {
            return Err(RouteError::InvalidCatalog(
                "route duration is outside bounds",
            ));
        }
        if route.objective_path.len() < 3
            || route.objective_path.len() > MAX_ROUTE_OBJECTIVES
            || route.objective_path.first() != Some(&RouteObjectiveId::ClearDroneGroup)
            || route.objective_path.last() != Some(&RouteObjectiveId::DefeatWarden)
            || route.objective_path.iter().collect::<BTreeSet<_>>().len()
                != route.objective_path.len()
        {
            return Err(RouteError::InvalidCatalog(
                "route objective path is invalid",
            ));
        }
        if route.events[0] == route.events[1]
            || route
                .events
                .iter()
                .any(|event_id| event_id.route() != route.route_id)
        {
            return Err(RouteError::InvalidCatalog(
                "route event membership is invalid",
            ));
        }
    }
    for event in events {
        if !event_ids.insert(event.event_id) || event.event_id.route() != event.route_id {
            return Err(RouteError::InvalidCatalog(
                "event identifiers/routes must be unique",
            ));
        }
        if !(MIN_WARDEN_HEALTH_BASIS_POINTS..=MAX_WARDEN_HEALTH_BASIS_POINTS)
            .contains(&event.effect.warden_health_basis_points)
            || !(MIN_WARDEN_COUNTER_DAMAGE..=MAX_WARDEN_COUNTER_DAMAGE)
                .contains(&event.effect.warden_counter_damage)
            || (event.effect.warden_health_basis_points == MIN_WARDEN_HEALTH_BASIS_POINTS
                && event.effect.warden_counter_damage == BASE_WARDEN_COUNTER_DAMAGE)
        {
            return Err(RouteError::InvalidCatalog(
                "event effect is invalid or inert",
            ));
        }
    }
    if route_ids != RouteId::ALL.into_iter().collect()
        || event_ids != RouteEventId::ALL.into_iter().collect()
    {
        return Err(RouteError::InvalidCatalog(
            "catalog identifiers are incomplete",
        ));
    }
    let breach = routes
        .iter()
        .find(|route| route.route_id == RouteId::Breach)
        .ok_or(RouteError::InvalidCatalog("breach route is missing"))?;
    let stabilize = routes
        .iter()
        .find(|route| route.route_id == RouteId::Stabilize)
        .ok_or(RouteError::InvalidCatalog("stabilize route is missing"))?;
    if breach.reward.fragments <= stabilize.reward.fragments
        || breach.reward.experience >= stabilize.reward.experience
        || !stabilize
            .objective_path
            .contains(&RouteObjectiveId::ReachRelayStabilizer)
        || breach
            .objective_path
            .contains(&RouteObjectiveId::ReachRelayStabilizer)
    {
        return Err(RouteError::InvalidCatalog(
            "routes must retain their fragment/experience/objective tradeoff",
        ));
    }
    Ok(())
}

/// Checks the bounded operation identifier accepted by domain and later persistence.
///
/// # Errors
///
/// Returns [`RouteError::InvalidOperationId`] unless the value is 1-32 ASCII
/// alphanumeric/hyphen characters.
pub fn validate_operation_id(value: &str) -> Result<(), RouteError> {
    if value.is_empty()
        || value.len() > MAX_ROUTE_OPERATION_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(RouteError::InvalidOperationId);
    }
    Ok(())
}

const fn valid_participant_id(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_PARTICIPANT_ID_BYTES {
        return false;
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b':') {
            return false;
        }
        index += 1;
    }
    true
}

const fn permute_63(value: u64) -> u64 {
    let mut mixed = value & MAX_ROUTE_SEED;
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0x3f79_d71b_4cb0_a89d) & MAX_ROUTE_SEED;
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x1c69_b3f7_4ac4_ae35) & MAX_ROUTE_SEED;
    mixed ^= mixed >> 31;
    mixed & MAX_ROUTE_SEED
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{
        apply_warden_health_effect, event_definition, resolve_event, route_definition,
        validate_catalog, validate_operation_id, OperationDisposition, RouteEffect, RouteError,
        RouteEventId, RouteId, RouteOperationState, RoutePhase, RouteSeed, RouteSelectionRequest,
        TerminalOutcome, MAX_ROUTE_SEED, RESOLVER_REVISION, ROUTE_CATALOG, ROUTE_EVENT_CATALOG,
        ROUTE_OPERATION_DURATION_MS,
    };

    fn capable_state(participant_ids: &[&str]) -> RouteOperationState {
        let mut state =
            RouteOperationState::new(participant_ids).expect("participants should work");
        for participant_id in participant_ids {
            state
                .mark_capable(participant_id)
                .expect("capability should apply");
        }
        state.open_choice().expect("choice should open");
        state
    }

    fn select_breach(state: &mut RouteOperationState) {
        state
            .select(
                "leader",
                RouteSelectionRequest {
                    operation_id: "route-1",
                    route_id: RouteId::Breach,
                },
                RouteSeed::new(7).expect("seed should work"),
            )
            .expect("selection should apply");
    }

    #[test]
    fn fixed_catalog_matches_every_finite_bound() {
        validate_catalog(&ROUTE_CATALOG, &ROUTE_EVENT_CATALOG)
            .expect("fixed catalog should validate");
        assert_eq!(RouteId::ALL.len(), 2);
        assert_eq!(RouteEventId::ALL.len(), 4);
        assert_eq!(route_definition(RouteId::Breach).events.len(), 2);
        assert_eq!(route_definition(RouteId::Stabilize).events.len(), 2);
        assert_eq!(ROUTE_OPERATION_DURATION_MS, 90_000);
    }

    #[test]
    fn invalid_catalog_cardinality_reward_membership_and_effect_are_rejected() {
        assert!(matches!(
            validate_catalog(&ROUTE_CATALOG[..1], &ROUTE_EVENT_CATALOG),
            Err(RouteError::InvalidCatalog(_))
        ));

        let mut routes = ROUTE_CATALOG;
        routes[0].reward.fragments = 3;
        assert!(matches!(
            validate_catalog(&routes, &ROUTE_EVENT_CATALOG),
            Err(RouteError::InvalidCatalog(_))
        ));

        let mut routes = ROUTE_CATALOG;
        routes[0].events[1] = routes[0].events[0];
        assert!(matches!(
            validate_catalog(&routes, &ROUTE_EVENT_CATALOG),
            Err(RouteError::InvalidCatalog(_))
        ));

        let mut events = ROUTE_EVENT_CATALOG;
        events[0].effect = RouteEffect {
            warden_health_basis_points: 10_000,
            warden_counter_damage: 15,
        };
        assert!(matches!(
            validate_catalog(&ROUTE_CATALOG, &events),
            Err(RouteError::InvalidCatalog(_))
        ));
    }

    #[test]
    fn invalid_catalog_duration_and_objective_path_are_rejected() {
        const INVALID_PATH: [super::RouteObjectiveId; 2] = [
            super::RouteObjectiveId::ClearDroneGroup,
            super::RouteObjectiveId::DefeatWarden,
        ];

        let mut routes = ROUTE_CATALOG;
        routes[0].duration_ms = 120_001;
        assert!(matches!(
            validate_catalog(&routes, &ROUTE_EVENT_CATALOG),
            Err(RouteError::InvalidCatalog(_))
        ));

        let mut routes = ROUTE_CATALOG;
        routes[0].objective_path = &INVALID_PATH;
        assert!(matches!(
            validate_catalog(&routes, &ROUTE_EVENT_CATALOG),
            Err(RouteError::InvalidCatalog(_))
        ));
    }

    #[test]
    fn route_parsing_is_exact() {
        assert_eq!(RouteId::from_str("breach"), Ok(RouteId::Breach));
        assert_eq!(RouteId::from_str("stabilize"), Ok(RouteId::Stabilize));
        assert_eq!(
            RouteId::from_str("third_route"),
            Err(RouteError::UnknownRoute("third_route".to_owned()))
        );
    }

    #[test]
    fn seed_boundaries_are_checked_and_repeatable() {
        let maximum = RouteSeed::new(MAX_ROUTE_SEED).expect("maximum seed should work");
        assert_eq!(maximum.value(), MAX_ROUTE_SEED);
        assert_eq!(
            RouteSeed::new(MAX_ROUTE_SEED + 1),
            Err(RouteError::InvalidSeed(MAX_ROUTE_SEED + 1))
        );
        for route_id in RouteId::ALL {
            for seed in [0, 1, 2, 3, MAX_ROUTE_SEED - 1, MAX_ROUTE_SEED] {
                let seed = RouteSeed::new(seed).expect("boundary seed should work");
                let first = resolve_event(route_id, seed);
                let second = resolve_event(route_id, seed);
                assert_eq!(first, second);
                assert_eq!(first.resolver_revision, RESOLVER_REVISION);
                assert_eq!(first.event_id.route(), route_id);
                assert_eq!(first.effect, event_definition(first.event_id).effect);
            }
        }
    }

    #[test]
    fn resolver_reaches_both_events_and_is_balanced_over_power_of_two_vector() {
        for route_id in RouteId::ALL {
            let mut counts = [0_u32; 2];
            let events = route_definition(route_id).events;
            for seed in 0..65_536 {
                let resolved = resolve_event(
                    route_id,
                    RouteSeed::new(seed).expect("vector seed should work"),
                );
                let index = usize::from(resolved.event_id == events[1]);
                counts[index] += 1;
            }
            assert!(counts.iter().all(|count| (31_744..=33_792).contains(count)));
        }
    }

    #[test]
    fn route_salts_produce_distinct_mapping_vectors() {
        let differences = (0..256)
            .filter(|seed| {
                let seed = RouteSeed::new(*seed).expect("seed should work");
                let breach_index =
                    resolve_event(RouteId::Breach, seed).event_id == RouteEventId::ArcSurge;
                let stabilize_index = resolve_event(RouteId::Stabilize, seed).event_id
                    == RouteEventId::ResidualFeedback;
                breach_index != stabilize_index
            })
            .count();
        assert!((96..=160).contains(&differences));
    }

    #[test]
    fn health_effect_uses_checked_round_half_up_arithmetic() {
        let effect = RouteEffect {
            warden_health_basis_points: 11_000,
            warden_counter_damage: 15,
        };
        assert_eq!(
            apply_warden_health_effect(241, effect).expect("effect should apply"),
            265
        );
        assert!(matches!(
            apply_warden_health_effect(u32::MAX, effect),
            Err(RouteError::ArithmeticOverflow)
        ));
    }

    #[test]
    fn leader_selection_applies_once_and_retry_cannot_reroll() {
        let mut state = capable_state(&["leader", "peer"]);
        let request = RouteSelectionRequest {
            operation_id: "route-1",
            route_id: RouteId::Breach,
        };
        let first = state
            .select(
                "leader",
                request,
                RouteSeed::new(7).expect("seed should work"),
            )
            .expect("selection should apply");
        let retry = state
            .select(
                "leader",
                request,
                RouteSeed::new(8).expect("different candidate should be ignored"),
            )
            .expect("retry should replay");
        assert_eq!(first.disposition, OperationDisposition::Applied);
        assert_eq!(retry.disposition, OperationDisposition::Replayed);
        assert_eq!(first.outcome, retry.outcome);
        assert_eq!(state.phase(), RoutePhase::Routed);
    }

    #[test]
    fn conflict_and_second_operation_leave_selected_state_unchanged() {
        let mut state = capable_state(&["leader"]);
        select_breach(&mut state);
        let before = state.clone();
        assert_eq!(
            state.select(
                "leader",
                RouteSelectionRequest {
                    operation_id: "route-1",
                    route_id: RouteId::Stabilize,
                },
                RouteSeed::new(1).expect("seed should work"),
            ),
            Err(RouteError::IdempotencyConflict)
        );
        assert_eq!(state, before);
        assert_eq!(
            state.select(
                "leader",
                RouteSelectionRequest {
                    operation_id: "route-2",
                    route_id: RouteId::Breach,
                },
                RouteSeed::new(1).expect("seed should work"),
            ),
            Err(RouteError::RouteAlreadySelected)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn capability_and_leader_rejections_do_not_reserve_operation() {
        let mut state =
            RouteOperationState::new(&["leader", "peer"]).expect("participants should work");
        state.open_choice().expect("choice should open");
        let request = RouteSelectionRequest {
            operation_id: "route-1",
            route_id: RouteId::Stabilize,
        };
        let before = state.clone();
        assert_eq!(
            state.select(
                "leader",
                request,
                RouteSeed::new(3).expect("seed should work"),
            ),
            Err(RouteError::CapabilityIncomplete)
        );
        assert_eq!(state, before);
        state.mark_capable("leader").expect("leader should opt in");
        state.mark_capable("peer").expect("peer should opt in");
        let capable = state.clone();
        assert_eq!(
            state.select(
                "peer",
                request,
                RouteSeed::new(3).expect("seed should work"),
            ),
            Err(RouteError::NotLeader)
        );
        assert_eq!(state, capable);
        assert_eq!(
            state
                .select(
                    "leader",
                    request,
                    RouteSeed::new(3).expect("seed should work"),
                )
                .expect("unreserved operation should apply")
                .disposition,
            OperationDisposition::Applied
        );
    }

    #[test]
    fn baseline_lock_is_explicit_and_terminal_for_route_choice() {
        let mut state = capable_state(&["leader"]);
        state.lock_baseline().expect("baseline should lock");
        let before = state.clone();
        assert_eq!(
            state.select(
                "leader",
                RouteSelectionRequest {
                    operation_id: "late-route",
                    route_id: RouteId::Breach,
                },
                RouteSeed::new(0).expect("seed should work"),
            ),
            Err(RouteError::WrongPhase(RoutePhase::BaselineLocked))
        );
        assert_eq!(state, before);
        assert!(state.selection().is_none());
    }

    #[test]
    fn deadline_boundary_separates_success_and_failure_without_reward_leak() {
        let mut success = capable_state(&["leader"]);
        select_breach(&mut success);
        let applied = success
            .complete(ROUTE_OPERATION_DURATION_MS)
            .expect("deadline completion should succeed");
        assert_eq!(applied.outcome.outcome, TerminalOutcome::Succeeded);
        assert!(applied.outcome.reward.is_some());
        assert_eq!(success.phase(), RoutePhase::Succeeded);
        let retry = success
            .complete(ROUTE_OPERATION_DURATION_MS)
            .expect("same terminal result should replay");
        assert_eq!(retry.disposition, OperationDisposition::Replayed);

        let mut failure = capable_state(&["leader"]);
        select_breach(&mut failure);
        let before = failure.clone();
        assert!(matches!(
            failure.complete(ROUTE_OPERATION_DURATION_MS + 1),
            Err(RouteError::DeadlineExceeded { .. })
        ));
        assert_eq!(failure, before);
        assert!(matches!(
            failure.fail_timeout(ROUTE_OPERATION_DURATION_MS),
            Err(RouteError::DeadlineNotReached { .. })
        ));
        assert_eq!(failure, before);
        let summary = failure
            .fail_timeout(ROUTE_OPERATION_DURATION_MS + 1)
            .expect("post-deadline observation should fail");
        assert_eq!(summary.outcome.outcome, TerminalOutcome::FailedTimeout);
        assert_eq!(summary.outcome.reward, None);
        assert_eq!(failure.phase(), RoutePhase::Failed);
    }

    #[test]
    fn selected_operation_can_remain_reconstructibly_incomplete() {
        let mut state = capable_state(&["leader", "peer"]);
        select_breach(&mut state);
        assert_eq!(state.phase(), RoutePhase::Routed);
        assert!(state.selection().is_some());
        assert!(state.terminal().is_none());
    }

    #[test]
    fn identifiers_and_participant_bounds_are_strict() {
        for valid in ["a", "Route-123", "x2345678901234567890123456789012"] {
            validate_operation_id(valid).expect("identifier should be valid");
        }
        for invalid in ["", "invalid_id", "x23456789012345678901234567890123"] {
            assert_eq!(
                validate_operation_id(invalid),
                Err(RouteError::InvalidOperationId)
            );
        }
        assert_eq!(
            RouteOperationState::new(&[]),
            Err(RouteError::InvalidParticipants)
        );
        assert_eq!(
            RouteOperationState::new(&["same", "same"]),
            Err(RouteError::InvalidParticipants)
        );
        assert_eq!(
            RouteOperationState::new(&["one", "two", "three"]),
            Err(RouteError::InvalidParticipants)
        );
    }
}
