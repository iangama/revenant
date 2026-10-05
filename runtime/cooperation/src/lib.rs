use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

pub const CATALOG_REVISION: &str = "m28-v1";
pub const OPERATION_DURATION_MS: u64 = 60_000;
pub const PING_TTL_MS: u64 = 5_000;
pub const REVIVE_WINDOW_MS: u64 = 15_000;
pub const REVIVE_CHANNEL_MS: u64 = 2_000;
pub const REVIVE_HEALTH: u32 = 50;
pub const MAX_REVIVE_DISTANCE_SQUARED: i64 = 4;
pub const MAX_OPERATION_ID_BYTES: usize = 32;
pub const PARTICIPANT_COUNT: usize = 2;
pub const REWARD_FRAGMENTS_PER_PARTICIPANT: u32 = 2;
pub const REWARD_EXPERIENCE_PER_PARTICIPANT: u64 = 125;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Anchor,
    Runner,
}

impl Role {
    pub const ALL: [Self; PARTICIPANT_COUNT] = [Self::Anchor, Self::Runner];

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Anchor => 0,
            Self::Runner => 1,
        }
    }
}

impl Display for Role {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Anchor => "anchor",
            Self::Runner => "runner",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    RelayAnchor,
    RelayConsole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    AwaitingAnchor,
    AwaitingPing,
    AwaitingRunner,
    RunnerDowned,
    ReviveChannel,
    EncounterActive,
    Succeeded,
    Failed,
}

impl Phase {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeState {
    Active,
    Downed,
    Defeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    Succeeded,
    FailedPingTimeout,
    FailedReviveTimeout,
    FailedOperationTimeout,
    FailedParticipantDefeated,
    AbandonedDisconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatProfile {
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
    pub max_health: u32,
}

impl CombatProfile {
    const fn is_valid(self) -> bool {
        self.damage > 0
            && self.range >= 0
            && self.cooldown_ms > 0
            && self.max_health >= REVIVE_HEALTH
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantState {
    pub role: Role,
    pub profile: CombatProfile,
    pub admitted_health: u32,
    pub current_health: u32,
    pub life: LifeState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Contributions {
    pub anchor_arrived: bool,
    pub pinged: bool,
    pub runner_arrived: bool,
    pub revived: bool,
    pub warden_completed: bool,
}

impl Contributions {
    #[must_use]
    pub const fn complete(self) -> bool {
        self.anchor_arrived
            && self.pinged
            && self.runner_arrived
            && self.revived
            && self.warden_completed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantReward {
    pub fragments: u32,
    pub experience: u64,
}

pub const SUCCESS_REWARD: ParticipantReward = ParticipantReward {
    fragments: REWARD_FRAGMENTS_PER_PARTICIPANT,
    experience: REWARD_EXPERIENCE_PER_PARTICIPANT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationDisposition {
    Applied,
    Replayed,
    Pending,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct OperationRecord {
    operation_id: String,
    accepted_elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CooperationState {
    catalog_revision: &'static str,
    start_operation_id: String,
    phase: Phase,
    participants: [ParticipantState; PARTICIPANT_COUNT],
    contributions: Contributions,
    ping: Option<OperationRecord>,
    downed_elapsed_ms: Option<u64>,
    revive_channel: Option<OperationRecord>,
    completed_revive_operation_id: Option<String>,
    revive_count: u8,
    terminal: Option<TerminalOutcome>,
}

impl CooperationState {
    /// Creates a committed, pure cooperation operation from immutable admission
    /// profiles. The caller supplies already-authoritative elapsed time to later
    /// transitions; this type never reads a clock.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid operation identifier or combat profile.
    pub fn start(
        operation_id: &str,
        profiles: [CombatProfile; PARTICIPANT_COUNT],
    ) -> Result<Self, CooperationError> {
        Self::start_with_health(
            operation_id,
            profiles,
            [profiles[0].max_health, profiles[1].max_health],
        )
    }

    /// Creates a committed operation while retaining each participant's exact
    /// authoritative current health at the cooperation boundary.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid identifiers, profiles, zero current health,
    /// or current health above the immutable admitted maximum.
    pub fn start_with_health(
        operation_id: &str,
        profiles: [CombatProfile; PARTICIPANT_COUNT],
        current_health: [u32; PARTICIPANT_COUNT],
    ) -> Result<Self, CooperationError> {
        validate_operation_id(operation_id)?;
        if !profiles.into_iter().all(CombatProfile::is_valid) {
            return Err(CooperationError::InvalidProfile);
        }
        if current_health
            .into_iter()
            .zip(profiles)
            .any(|(health, profile)| health == 0 || health > profile.max_health)
        {
            return Err(CooperationError::InvalidCurrentHealth);
        }
        Ok(Self {
            catalog_revision: CATALOG_REVISION,
            start_operation_id: operation_id.to_owned(),
            phase: Phase::AwaitingAnchor,
            participants: [
                ParticipantState {
                    role: Role::Anchor,
                    profile: profiles[0],
                    admitted_health: current_health[0],
                    current_health: current_health[0],
                    life: LifeState::Active,
                },
                ParticipantState {
                    role: Role::Runner,
                    profile: profiles[1],
                    admitted_health: current_health[1],
                    current_health: current_health[1],
                    life: LifeState::Active,
                },
            ],
            contributions: Contributions::default(),
            ping: None,
            downed_elapsed_ms: None,
            revive_channel: None,
            completed_revive_operation_id: None,
            revive_count: 0,
            terminal: None,
        })
    }

    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    #[must_use]
    pub const fn participants(&self) -> &[ParticipantState; PARTICIPANT_COUNT] {
        &self.participants
    }

    #[must_use]
    pub const fn contributions(&self) -> Contributions {
        self.contributions
    }

    #[must_use]
    pub const fn revive_count(&self) -> u8 {
        self.revive_count
    }

    #[must_use]
    pub const fn terminal(&self) -> Option<TerminalOutcome> {
        self.terminal
    }

    #[must_use]
    pub const fn reward(&self) -> Option<[ParticipantReward; PARTICIPANT_COUNT]> {
        if matches!(self.terminal, Some(TerminalOutcome::Succeeded)) {
            Some([SUCCESS_REWARD; PARTICIPANT_COUNT])
        } else {
            None
        }
    }

    /// Replays the already-committed cooperation start without creating a
    /// second operation or changing admission-derived roles.
    ///
    /// # Errors
    ///
    /// Rejects an invalid identifier, a different identifier, or conflicting
    /// immutable admission profiles.
    pub fn retry_start(
        &self,
        operation_id: &str,
        profiles: [CombatProfile; PARTICIPANT_COUNT],
        current_health: [u32; PARTICIPANT_COUNT],
    ) -> Result<MutationDisposition, CooperationError> {
        validate_operation_id(operation_id)?;
        if operation_id != self.start_operation_id {
            return Err(CooperationError::AlreadyStarted);
        }
        if self.participants[0].profile != profiles[0]
            || self.participants[1].profile != profiles[1]
            || self.participants[0].admitted_health != current_health[0]
            || self.participants[1].admitted_health != current_health[1]
        {
            return Err(CooperationError::OperationConflict);
        }
        Ok(MutationDisposition::Replayed)
    }

    /// Records the fixed anchor contribution.
    ///
    /// # Errors
    ///
    /// Rejects expired, terminal, wrong-role, wrong-target, or out-of-order input.
    pub fn arrive_anchor(
        &mut self,
        role: Role,
        target: Target,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        self.observe_deadlines(elapsed_ms)?;
        require_role_target(role, Role::Anchor, target, Target::RelayAnchor)?;
        self.require_phase(Phase::AwaitingAnchor)?;
        self.contributions.anchor_arrived = true;
        self.phase = Phase::AwaitingPing;
        Ok(MutationDisposition::Applied)
    }

    /// Records the one fixed contextual ping.
    ///
    /// # Errors
    ///
    /// Rejects malformed identifiers, expired/terminal operations, authority,
    /// target, ordering, second-ping, and identifier-conflict violations.
    pub fn ping(
        &mut self,
        role: Role,
        target: Target,
        operation_id: &str,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        self.observe_deadlines(elapsed_ms)?;
        require_role_target(role, Role::Anchor, target, Target::RelayConsole)?;
        validate_operation_id(operation_id)?;
        if let Some(record) = &self.ping {
            return if record.operation_id == operation_id {
                Ok(MutationDisposition::Replayed)
            } else {
                Err(CooperationError::AlreadyPinged)
            };
        }
        self.require_phase(Phase::AwaitingPing)?;
        self.ping = Some(OperationRecord {
            operation_id: operation_id.to_owned(),
            accepted_elapsed_ms: elapsed_ms,
        });
        self.contributions.pinged = true;
        self.phase = Phase::AwaitingRunner;
        Ok(MutationDisposition::Applied)
    }

    /// Records runner arrival and applies the fixed scripted downing.
    ///
    /// # Errors
    ///
    /// Rejects expired/terminal, wrong-role, wrong-target, or out-of-order input.
    pub fn arrive_runner(
        &mut self,
        role: Role,
        target: Target,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        self.observe_deadlines(elapsed_ms)?;
        require_role_target(role, Role::Runner, target, Target::RelayConsole)?;
        self.require_phase(Phase::AwaitingRunner)?;
        let runner = &mut self.participants[Role::Runner.index()];
        if runner.life != LifeState::Active || runner.current_health == 0 {
            return Err(CooperationError::InvalidLifeState);
        }
        runner.current_health = 0;
        runner.life = LifeState::Downed;
        self.contributions.runner_arrived = true;
        self.downed_elapsed_ms = Some(elapsed_ms);
        self.phase = Phase::RunnerDowned;
        Ok(MutationDisposition::Applied)
    }

    /// Starts the one server-authoritative revive channel.
    ///
    /// # Errors
    ///
    /// Rejects malformed identifiers, expiry, authority, target, range,
    /// ordering, active-channel conflict, or a second revive.
    pub fn start_revive(
        &mut self,
        role: Role,
        target_role: Role,
        distance_squared: i64,
        operation_id: &str,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        self.observe_deadlines(elapsed_ms)?;
        require_revive_authority(role, target_role)?;
        validate_distance(distance_squared)?;
        validate_operation_id(operation_id)?;
        if self.completed_revive_operation_id.as_deref() == Some(operation_id) {
            return Ok(MutationDisposition::Replayed);
        }
        if self.revive_count > 0 {
            return Err(CooperationError::AlreadyRevived);
        }
        if let Some(record) = &self.revive_channel {
            return if record.operation_id == operation_id {
                Ok(MutationDisposition::Replayed)
            } else {
                Err(CooperationError::ReviveChannelActive)
            };
        }
        self.require_phase(Phase::RunnerDowned)?;
        if self.participants[Role::Anchor.index()].life != LifeState::Active
            || self.participants[Role::Runner.index()].life != LifeState::Downed
        {
            return Err(CooperationError::InvalidLifeState);
        }
        self.revive_channel = Some(OperationRecord {
            operation_id: operation_id.to_owned(),
            accepted_elapsed_ms: elapsed_ms,
        });
        self.phase = Phase::ReviveChannel;
        Ok(MutationDisposition::Applied)
    }

    /// Observes a revive channel using caller-supplied authoritative elapsed time.
    ///
    /// # Errors
    ///
    /// Rejects expiry, authority, target, identifier, ordering, or time reversal.
    pub fn observe_revive(
        &mut self,
        role: Role,
        target_role: Role,
        distance_squared: i64,
        operation_id: &str,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        self.observe_deadlines(elapsed_ms)?;
        require_revive_authority(role, target_role)?;
        validate_operation_id(operation_id)?;
        if self.completed_revive_operation_id.as_deref() == Some(operation_id) {
            return Ok(MutationDisposition::Replayed);
        }
        self.require_phase(Phase::ReviveChannel)?;
        let record = self
            .revive_channel
            .as_ref()
            .ok_or(CooperationError::WrongPhase {
                expected: Phase::ReviveChannel,
                actual: self.phase,
            })?;
        if record.operation_id != operation_id {
            return Err(CooperationError::OperationConflict);
        }
        if distance_squared < 0 {
            return Err(CooperationError::OutOfRange(distance_squared));
        }
        if distance_squared > MAX_REVIVE_DISTANCE_SQUARED {
            self.revive_channel = None;
            self.phase = Phase::RunnerDowned;
            return Ok(MutationDisposition::Cancelled);
        }
        let channel_elapsed = elapsed_ms
            .checked_sub(record.accepted_elapsed_ms)
            .ok_or(CooperationError::TimeReversal)?;
        if channel_elapsed < REVIVE_CHANNEL_MS {
            return Ok(MutationDisposition::Pending);
        }
        let operation_id = record.operation_id.clone();
        let runner = &mut self.participants[Role::Runner.index()];
        runner.current_health = REVIVE_HEALTH.min(runner.profile.max_health);
        runner.life = LifeState::Active;
        self.revive_count = self
            .revive_count
            .checked_add(1)
            .ok_or(CooperationError::ArithmeticOverflow)?;
        self.contributions.revived = true;
        self.completed_revive_operation_id = Some(operation_id);
        self.revive_channel = None;
        self.phase = Phase::EncounterActive;
        Ok(MutationDisposition::Applied)
    }

    /// Completes the unchanged baseline Warden encounter and the cooperation.
    ///
    /// # Errors
    ///
    /// Rejects expiry, terminal replay with conflicting input, incomplete
    /// contributions, or an out-of-order completion.
    pub fn complete_warden(
        &mut self,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        if self.terminal == Some(TerminalOutcome::Succeeded) {
            return Ok(MutationDisposition::Replayed);
        }
        self.observe_deadlines(elapsed_ms)?;
        self.require_phase(Phase::EncounterActive)?;
        self.contributions.warden_completed = true;
        if !self.contributions.complete() {
            return Err(CooperationError::IncompleteContributions);
        }
        self.commit_terminal(TerminalOutcome::Succeeded);
        Ok(MutationDisposition::Applied)
    }

    /// Records a post-revive participant defeat.
    ///
    /// # Errors
    ///
    /// Rejects expiry, terminal, or a defeat outside the encounter phase.
    pub fn defeat_participant(
        &mut self,
        role: Role,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        self.observe_deadlines(elapsed_ms)?;
        self.require_phase(Phase::EncounterActive)?;
        let participant = &mut self.participants[role.index()];
        participant.current_health = 0;
        participant.life = LifeState::Defeated;
        self.commit_terminal(TerminalOutcome::FailedParticipantDefeated);
        Ok(MutationDisposition::Applied)
    }

    /// Records deterministic abandonment for either immutable role.
    ///
    /// # Errors
    ///
    /// Rejects an observation after a different terminal result.
    pub fn disconnect(
        &mut self,
        _role: Role,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        if self.terminal == Some(TerminalOutcome::AbandonedDisconnect) {
            return Ok(MutationDisposition::Replayed);
        }
        self.observe_deadlines(elapsed_ms)?;
        self.commit_terminal(TerminalOutcome::AbandonedDisconnect);
        Ok(MutationDisposition::Applied)
    }

    /// Observes all phase-specific and overall deadlines without any clock IO.
    ///
    /// # Errors
    ///
    /// Returns `Terminal` if the state was already terminal, or `DeadlineExpired`
    /// after committing the first applicable deterministic timeout.
    pub fn observe_deadlines(
        &mut self,
        elapsed_ms: u64,
    ) -> Result<MutationDisposition, CooperationError> {
        if let Some(outcome) = self.terminal {
            return Err(CooperationError::Terminal(outcome));
        }
        let timeout = match self.phase {
            Phase::AwaitingRunner => self.ping.as_ref().and_then(|ping| {
                is_strictly_after(elapsed_ms, ping.accepted_elapsed_ms, PING_TTL_MS)
                    .then_some(TerminalOutcome::FailedPingTimeout)
            }),
            Phase::RunnerDowned | Phase::ReviveChannel => {
                self.downed_elapsed_ms.and_then(|downed_at| {
                    is_strictly_after(elapsed_ms, downed_at, REVIVE_WINDOW_MS)
                        .then_some(TerminalOutcome::FailedReviveTimeout)
                })
            }
            _ => None,
        }
        .or_else(|| {
            (elapsed_ms > OPERATION_DURATION_MS).then_some(TerminalOutcome::FailedOperationTimeout)
        });
        if let Some(outcome) = timeout {
            self.commit_terminal(outcome);
            return Err(CooperationError::DeadlineExpired(outcome));
        }
        Ok(MutationDisposition::Pending)
    }

    fn require_phase(&self, expected: Phase) -> Result<(), CooperationError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(CooperationError::WrongPhase {
                expected,
                actual: self.phase,
            })
        }
    }

    fn commit_terminal(&mut self, outcome: TerminalOutcome) {
        self.terminal = Some(outcome);
        self.phase = if outcome == TerminalOutcome::Succeeded {
            Phase::Succeeded
        } else {
            Phase::Failed
        };
        self.revive_channel = None;
    }
}

fn is_strictly_after(elapsed_ms: u64, start_ms: u64, duration_ms: u64) -> bool {
    start_ms
        .checked_add(duration_ms)
        .is_none_or(|deadline| elapsed_ms > deadline)
}

fn require_role_target(
    actual_role: Role,
    expected_role: Role,
    actual_target: Target,
    expected_target: Target,
) -> Result<(), CooperationError> {
    if actual_role != expected_role {
        return Err(CooperationError::WrongRole {
            expected: expected_role,
            actual: actual_role,
        });
    }
    if actual_target != expected_target {
        return Err(CooperationError::WrongTarget {
            expected: expected_target,
            actual: actual_target,
        });
    }
    Ok(())
}

fn require_revive_authority(role: Role, target_role: Role) -> Result<(), CooperationError> {
    if role != Role::Anchor {
        return Err(CooperationError::WrongRole {
            expected: Role::Anchor,
            actual: role,
        });
    }
    if target_role != Role::Runner {
        return Err(CooperationError::WrongReviveTarget(target_role));
    }
    Ok(())
}

fn validate_distance(distance_squared: i64) -> Result<(), CooperationError> {
    if !(0..=MAX_REVIVE_DISTANCE_SQUARED).contains(&distance_squared) {
        return Err(CooperationError::OutOfRange(distance_squared));
    }
    Ok(())
}

/// Validates the shared 1-32-byte ASCII alphanumeric/hyphen operation-ID bound.
///
/// # Errors
///
/// Returns `InvalidOperationId` outside the frozen grammar.
pub fn validate_operation_id(operation_id: &str) -> Result<(), CooperationError> {
    if operation_id.is_empty()
        || operation_id.len() > MAX_OPERATION_ID_BYTES
        || !operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(CooperationError::InvalidOperationId);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooperationError {
    InvalidOperationId,
    InvalidProfile,
    InvalidCurrentHealth,
    WrongRole { expected: Role, actual: Role },
    WrongTarget { expected: Target, actual: Target },
    WrongReviveTarget(Role),
    WrongPhase { expected: Phase, actual: Phase },
    AlreadyPinged,
    AlreadyStarted,
    AlreadyRevived,
    ReviveChannelActive,
    OperationConflict,
    OutOfRange(i64),
    InvalidLifeState,
    TimeReversal,
    IncompleteContributions,
    ArithmeticOverflow,
    Terminal(TerminalOutcome),
    DeadlineExpired(TerminalOutcome),
}

impl Display for CooperationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOperationId => formatter.write_str("invalid cooperation operation id"),
            Self::InvalidProfile => formatter.write_str("invalid admitted combat profile"),
            Self::InvalidCurrentHealth => formatter.write_str("invalid admitted current health"),
            Self::WrongRole { expected, actual } => {
                write!(formatter, "expected role {expected}, received {actual}")
            }
            Self::WrongTarget { expected, actual } => {
                write!(
                    formatter,
                    "expected target {expected:?}, received {actual:?}"
                )
            }
            Self::WrongReviveTarget(actual) => {
                write!(
                    formatter,
                    "expected runner revive target, received {actual}"
                )
            }
            Self::WrongPhase { expected, actual } => {
                write!(
                    formatter,
                    "expected phase {expected:?}, received {actual:?}"
                )
            }
            Self::AlreadyPinged => formatter.write_str("cooperation ping already accepted"),
            Self::AlreadyStarted => formatter.write_str("cooperation operation already started"),
            Self::AlreadyRevived => formatter.write_str("cooperation revive already completed"),
            Self::ReviveChannelActive => formatter.write_str("a revive channel is already active"),
            Self::OperationConflict => formatter.write_str("cooperation operation id conflict"),
            Self::OutOfRange(distance) => {
                write!(formatter, "revive distance {distance} is invalid")
            }
            Self::InvalidLifeState => formatter.write_str("participant life state is invalid"),
            Self::TimeReversal => formatter.write_str("elapsed time predates accepted operation"),
            Self::IncompleteContributions => {
                formatter.write_str("required cooperation contributions are incomplete")
            }
            Self::ArithmeticOverflow => formatter.write_str("cooperation arithmetic overflowed"),
            Self::Terminal(outcome) => write!(formatter, "operation is terminal: {outcome:?}"),
            Self::DeadlineExpired(outcome) => {
                write!(formatter, "operation deadline expired: {outcome:?}")
            }
        }
    }
}

impl Error for CooperationError {}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: CombatProfile = CombatProfile {
        damage: 40,
        range: 6,
        cooldown_ms: 250,
        max_health: 100,
    };

    fn state() -> CooperationState {
        CooperationState::start("start-1", [PROFILE, PROFILE]).expect("valid state")
    }

    fn drive_to_ping(state: &mut CooperationState) {
        state
            .arrive_anchor(Role::Anchor, Target::RelayAnchor, 1_000)
            .expect("anchor");
        state
            .ping(Role::Anchor, Target::RelayConsole, "ping-1", 2_000)
            .expect("ping");
    }

    fn drive_to_downed(state: &mut CooperationState) {
        drive_to_ping(state);
        state
            .arrive_runner(Role::Runner, Target::RelayConsole, 3_000)
            .expect("runner");
    }

    fn drive_to_channel(state: &mut CooperationState) {
        drive_to_downed(state);
        state
            .start_revive(Role::Anchor, Role::Runner, 4, "revive-1", 4_000)
            .expect("revive start");
    }

    fn drive_to_encounter(state: &mut CooperationState) {
        drive_to_channel(state);
        state
            .observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 6_000)
            .expect("revive complete");
    }

    #[test]
    fn successful_lifecycle_requires_both_roles_and_exact_reward() {
        let mut state = state();
        drive_to_encounter(&mut state);
        assert_eq!(state.participants()[1].current_health, REVIVE_HEALTH);
        assert_eq!(state.revive_count(), 1);
        assert_eq!(
            state.complete_warden(60_000),
            Ok(MutationDisposition::Applied)
        );
        assert_eq!(state.phase(), Phase::Succeeded);
        assert!(state.contributions().complete());
        assert_eq!(state.reward(), Some([SUCCESS_REWARD; PARTICIPANT_COUNT]));
        assert_eq!(
            state.complete_warden(u64::MAX),
            Ok(MutationDisposition::Replayed)
        );
    }

    #[test]
    fn roles_and_profiles_are_fixed_by_admission_order() {
        let anchor = CombatProfile {
            max_health: 120,
            ..PROFILE
        };
        let runner = CombatProfile {
            damage: 25,
            max_health: 130,
            ..PROFILE
        };
        let state = CooperationState::start("fixed", [anchor, runner]).expect("valid state");
        assert_eq!(state.participants()[0].role, Role::Anchor);
        assert_eq!(state.participants()[0].profile, anchor);
        assert_eq!(state.participants()[1].role, Role::Runner);
        assert_eq!(state.participants()[1].profile, runner);
        assert_eq!(
            state.retry_start("fixed", [anchor, runner], [120, 130]),
            Ok(MutationDisposition::Replayed)
        );
        assert_eq!(
            state.retry_start("second", [anchor, runner], [120, 130]),
            Err(CooperationError::AlreadyStarted)
        );
        assert_eq!(
            state.retry_start("fixed", [runner, anchor], [120, 130]),
            Err(CooperationError::OperationConflict)
        );
    }

    #[test]
    fn operation_id_and_profile_bounds_reject() {
        for operation_id in [
            "",
            "bad_id",
            "bad space",
            "123456789012345678901234567890123",
        ] {
            assert_eq!(
                CooperationState::start(operation_id, [PROFILE, PROFILE]),
                Err(CooperationError::InvalidOperationId)
            );
        }
        let invalid = CombatProfile {
            max_health: REVIVE_HEALTH - 1,
            ..PROFILE
        };
        assert_eq!(
            CooperationState::start("valid", [PROFILE, invalid]),
            Err(CooperationError::InvalidProfile)
        );
        assert_eq!(
            CooperationState::start_with_health("valid", [PROFILE, PROFILE], [100, 0]),
            Err(CooperationError::InvalidCurrentHealth)
        );
        assert_eq!(
            CooperationState::start_with_health("valid", [PROFILE, PROFILE], [101, 100]),
            Err(CooperationError::InvalidCurrentHealth)
        );
    }

    #[test]
    fn wrong_role_target_and_order_leave_state_unchanged() {
        let mut state = state();
        for result in [
            state.arrive_anchor(Role::Runner, Target::RelayAnchor, 0),
            state.arrive_anchor(Role::Anchor, Target::RelayConsole, 0),
        ] {
            assert!(result.is_err());
            assert_eq!(state.phase(), Phase::AwaitingAnchor);
            assert_eq!(state.contributions(), Contributions::default());
        }
        assert!(state
            .arrive_runner(Role::Runner, Target::RelayConsole, 0)
            .is_err());
        assert_eq!(state.phase(), Phase::AwaitingAnchor);
    }

    #[test]
    fn ping_retry_is_replayed_and_second_ping_rejects_without_extension() {
        let mut state = state();
        state
            .arrive_anchor(Role::Anchor, Target::RelayAnchor, 0)
            .expect("anchor");
        state
            .ping(Role::Anchor, Target::RelayConsole, "ping-1", 100)
            .expect("ping");
        assert_eq!(
            state.ping(Role::Anchor, Target::RelayConsole, "ping-1", 4_000),
            Ok(MutationDisposition::Replayed)
        );
        assert_eq!(
            state.ping(Role::Anchor, Target::RelayConsole, "ping-2", 4_000),
            Err(CooperationError::AlreadyPinged)
        );
        assert_eq!(
            state.observe_deadlines(5_101),
            Err(CooperationError::DeadlineExpired(
                TerminalOutcome::FailedPingTimeout
            ))
        );
    }

    #[test]
    fn ping_exact_boundary_is_live_and_next_millisecond_fails() {
        let mut exact = state();
        drive_to_ping(&mut exact);
        assert_eq!(
            exact.arrive_runner(Role::Runner, Target::RelayConsole, 7_000),
            Ok(MutationDisposition::Applied)
        );
        let mut expired = state();
        drive_to_ping(&mut expired);
        assert_eq!(
            expired.arrive_runner(Role::Runner, Target::RelayConsole, 7_001),
            Err(CooperationError::DeadlineExpired(
                TerminalOutcome::FailedPingTimeout
            ))
        );
        assert_eq!(expired.reward(), None);
    }

    #[test]
    fn scripted_hazard_uses_exact_current_health() {
        let runner = CombatProfile {
            max_health: 130,
            ..PROFILE
        };
        let mut state = CooperationState::start_with_health("start", [PROFILE, runner], [90, 75])
            .expect("state");
        drive_to_downed(&mut state);
        assert_eq!(state.participants()[1].profile.max_health, 130);
        assert_eq!(state.participants()[1].admitted_health, 75);
        assert_eq!(state.participants()[1].current_health, 0);
        assert_eq!(state.participants()[1].life, LifeState::Downed);
        assert_eq!(state.revive_count(), 0);
        assert!(state
            .arrive_runner(Role::Runner, Target::RelayConsole, 3_001)
            .is_err());
        assert_eq!(state.participants()[1].life, LifeState::Downed);
    }

    #[test]
    fn revive_authority_range_and_identifier_are_checked() {
        let mut state = state();
        drive_to_downed(&mut state);
        for result in [
            state.start_revive(Role::Runner, Role::Runner, 4, "revive-1", 4_000),
            state.start_revive(Role::Anchor, Role::Anchor, 4, "revive-1", 4_000),
            state.start_revive(Role::Anchor, Role::Runner, 5, "revive-1", 4_000),
            state.start_revive(Role::Anchor, Role::Runner, -1, "revive-1", 4_000),
            state.start_revive(Role::Anchor, Role::Runner, 4, "bad_id", 4_000),
        ] {
            assert!(result.is_err());
            assert_eq!(state.phase(), Phase::RunnerDowned);
        }
    }

    #[test]
    fn channel_exact_boundary_pending_then_applies() {
        let mut state = state();
        drive_to_channel(&mut state);
        assert_eq!(
            state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 5_999),
            Ok(MutationDisposition::Pending)
        );
        assert_eq!(state.participants()[1].life, LifeState::Downed);
        assert_eq!(
            state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 6_000),
            Ok(MutationDisposition::Applied)
        );
        assert_eq!(state.participants()[1].current_health, REVIVE_HEALTH);
    }

    #[test]
    fn channel_cancellation_reserves_no_identifier_or_retry_budget() {
        let mut state = state();
        drive_to_channel(&mut state);
        assert_eq!(
            state.observe_revive(Role::Anchor, Role::Runner, 5, "revive-1", 5_000),
            Ok(MutationDisposition::Cancelled)
        );
        assert_eq!(state.phase(), Phase::RunnerDowned);
        assert_eq!(
            state.start_revive(Role::Anchor, Role::Runner, 4, "revive-1", 5_500),
            Ok(MutationDisposition::Applied)
        );
    }

    #[test]
    fn revive_retry_conflict_and_second_revive_are_bounded() {
        let mut state = state();
        drive_to_channel(&mut state);
        assert_eq!(
            state.start_revive(Role::Anchor, Role::Runner, 4, "revive-1", 4_500),
            Ok(MutationDisposition::Replayed)
        );
        assert_eq!(
            state.start_revive(Role::Anchor, Role::Runner, 4, "revive-2", 4_500),
            Err(CooperationError::ReviveChannelActive)
        );
        state
            .observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 6_000)
            .expect("complete");
        assert_eq!(
            state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 7_000),
            Ok(MutationDisposition::Replayed)
        );
        assert_eq!(
            state.start_revive(Role::Anchor, Role::Runner, 4, "revive-2", 7_000),
            Err(CooperationError::AlreadyRevived)
        );
    }

    #[test]
    fn revive_window_exact_boundary_is_eligible_and_next_millisecond_fails() {
        let mut exact = state();
        drive_to_downed(&mut exact);
        exact
            .start_revive(Role::Anchor, Role::Runner, 4, "late", 16_000)
            .expect("start within window");
        assert_eq!(
            exact.observe_revive(Role::Anchor, Role::Runner, 4, "late", 18_000),
            Ok(MutationDisposition::Applied)
        );

        let mut exact_start = state();
        drive_to_downed(&mut exact_start);
        assert_eq!(
            exact_start.start_revive(Role::Anchor, Role::Runner, 4, "exact", 18_000),
            Ok(MutationDisposition::Applied)
        );

        let mut expired = state();
        drive_to_downed(&mut expired);
        assert_eq!(
            expired.start_revive(Role::Anchor, Role::Runner, 4, "late", 18_001),
            Err(CooperationError::DeadlineExpired(
                TerminalOutcome::FailedReviveTimeout
            ))
        );
    }

    #[test]
    fn overall_deadline_exact_boundary_succeeds_and_next_millisecond_fails() {
        let mut exact = state();
        drive_to_encounter(&mut exact);
        assert_eq!(
            exact.complete_warden(OPERATION_DURATION_MS),
            Ok(MutationDisposition::Applied)
        );

        let mut expired = state();
        drive_to_encounter(&mut expired);
        assert_eq!(
            expired.complete_warden(OPERATION_DURATION_MS + 1),
            Err(CooperationError::DeadlineExpired(
                TerminalOutcome::FailedOperationTimeout
            ))
        );
    }

    #[test]
    fn phase_specific_timeout_precedes_overall_timeout() {
        let mut ping = state();
        ping.arrive_anchor(Role::Anchor, Target::RelayAnchor, 0)
            .expect("anchor");
        ping.ping(Role::Anchor, Target::RelayConsole, "ping", 0)
            .expect("ping");
        assert_eq!(
            ping.observe_deadlines(OPERATION_DURATION_MS + 1),
            Err(CooperationError::DeadlineExpired(
                TerminalOutcome::FailedPingTimeout
            ))
        );
        let mut revive = state();
        drive_to_downed(&mut revive);
        assert_eq!(
            revive.observe_deadlines(OPERATION_DURATION_MS + 1),
            Err(CooperationError::DeadlineExpired(
                TerminalOutcome::FailedReviveTimeout
            ))
        );
    }

    #[test]
    fn participant_defeat_is_terminal_and_never_rewards() {
        for role in Role::ALL {
            let mut state = state();
            drive_to_encounter(&mut state);
            assert_eq!(
                state.defeat_participant(role, 7_000),
                Ok(MutationDisposition::Applied)
            );
            assert_eq!(
                state.terminal(),
                Some(TerminalOutcome::FailedParticipantDefeated)
            );
            assert_eq!(state.reward(), None);
        }
    }

    #[test]
    fn both_disconnect_identities_abandon_every_nonterminal_phase() {
        for phase_depth in 0..=5 {
            for role in Role::ALL {
                let mut state = state();
                if phase_depth >= 1 {
                    state
                        .arrive_anchor(Role::Anchor, Target::RelayAnchor, 100)
                        .expect("anchor");
                }
                if phase_depth >= 2 {
                    state
                        .ping(Role::Anchor, Target::RelayConsole, "ping", 200)
                        .expect("ping");
                }
                if phase_depth >= 3 {
                    state
                        .arrive_runner(Role::Runner, Target::RelayConsole, 300)
                        .expect("runner");
                }
                if phase_depth >= 4 {
                    state
                        .start_revive(Role::Anchor, Role::Runner, 4, "revive", 400)
                        .expect("revive start");
                }
                if phase_depth >= 5 {
                    state
                        .observe_revive(Role::Anchor, Role::Runner, 4, "revive", 2_400)
                        .expect("revive complete");
                }
                assert_eq!(
                    state.disconnect(role, 3_000),
                    Ok(MutationDisposition::Applied)
                );
                assert_eq!(state.terminal(), Some(TerminalOutcome::AbandonedDisconnect));
                assert_eq!(state.reward(), None);
                assert_eq!(
                    state.disconnect(role, u64::MAX),
                    Ok(MutationDisposition::Replayed)
                );
            }
        }
    }

    #[test]
    fn terminal_state_rejects_post_terminal_mutation() {
        let mut state = state();
        state.disconnect(Role::Anchor, 0).expect("abandon");
        assert!(matches!(
            state.arrive_anchor(Role::Anchor, Target::RelayAnchor, 0),
            Err(CooperationError::Terminal(
                TerminalOutcome::AbandonedDisconnect
            ))
        ));
        assert_eq!(state.reward(), None);
    }

    #[test]
    fn crash_snapshot_remains_nonterminal_and_serializable() {
        let mut state = state();
        drive_to_channel(&mut state);
        let encoded = serde_json::to_string(&state).expect("serialize snapshot");
        assert!(encoded.contains("revive_channel"));
        assert_eq!(state.terminal(), None);
        assert_eq!(state.reward(), None);
    }
}
