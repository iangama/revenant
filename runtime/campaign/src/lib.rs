//! Finite campaign policy. The gateway supplies confirmed world milestones;
//! persistence commits each transition and its first-clear grant atomically.
//! Reconnecting reads the saved run and restarts only the interrupted encounter.
use std::fmt;

pub mod story;

use serde::{Deserialize, Serialize};

pub const LEGACY_REVISION: &str = "m36-campaign-v1";
pub const SUPPLY_REVISION: &str = "m36-campaign-v2";
pub const COUNTER_REVISION: &str = "m36-campaign-v3";
pub const BREACH_REVISION: &str = "m36-campaign-v4";
pub const PRISM_REVISION: &str = "m36-campaign-v5";
pub const REVISION: &str = "m36-campaign-v6";
pub const AVAILABLE_CHAPTERS: u8 = 6;
pub const PRISM_APPROACH: [i32; 3] = [5, 0, 2];
pub const PRISM_SOURCE: [i32; 3] = [8, 0, 0];
pub const PRISM_RETURN: [i32; 3] = [0, 0, 0];
pub const BREACH_DOOR: [i32; 3] = [6, 0, 0];
pub const BREACH_STABILIZER: [i32; 3] = [3, 0, 3];
pub const BREACH_CORE: [i32; 3] = [10, 0, 0];
pub const COUNTER_APPROACH: [i32; 3] = [2, 0, 6];
pub const COUNTER_TRANSMISSION: [i32; 3] = [7, 0, 6];
pub const COUNTER_BASTION: [i32; 3] = [5, 0, -6];
pub const COUNTER_EMITTER: [i32; 3] = [9, 0, -6];
pub const SUPPLY_APPROACH: [i32; 3] = [-4, 0, 4];
pub const SUPPLY_CELL: [i32; 3] = [-4, 0, -3];
pub const SUPPLY_DELIVERY: [i32; 3] = [-1, 0, -5];
pub const TOTAL_CHAPTERS: u8 = 6;
pub const FIRST_CLEAR_FRAGMENTS: u32 = 1;
pub const FIRST_CLEAR_EXPERIENCE: u64 = 100;
pub const MAX_REVISION: u64 = i64::MAX as u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterId {
    ReturnSignal,
    MeridianReadings,
    BrokenSupplyLine,
    CounterSignal,
    TheBreach,
    PrismCore,
}

impl ChapterId {
    pub const ALL: [Self; 6] = [
        Self::ReturnSignal,
        Self::MeridianReadings,
        Self::BrokenSupplyLine,
        Self::CounterSignal,
        Self::TheBreach,
        Self::PrismCore,
    ];

    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::ReturnSignal => 0,
            Self::MeridianReadings => 1,
            Self::BrokenSupplyLine => 2,
            Self::CounterSignal => 3,
            Self::TheBreach => 4,
            Self::PrismCore => 5,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReturnSignal => "return_signal",
            Self::MeridianReadings => "meridian_readings",
            Self::BrokenSupplyLine => "broken_supply_line",
            Self::CounterSignal => "counter_signal",
            Self::TheBreach => "the_breach",
            Self::PrismCore => "prism_core",
        }
    }

    #[must_use]
    pub const fn milestones(self) -> &'static [Milestone] {
        match self {
            Self::ReturnSignal => &[
                Milestone::RelayGuardCleared,
                Milestone::ReturnSignalRecovered,
            ],
            Self::MeridianReadings => &[
                Milestone::MeridianReached,
                Milestone::SurveyLinked,
                Milestone::RecoveryDelivered,
            ],
            Self::BrokenSupplyLine => &[
                Milestone::SupplyRouteReached,
                Milestone::SupplyGuardCleared,
                Milestone::SupplyCellRecovered,
                Milestone::SupplyDelivered,
            ],
            Self::CounterSignal => &[
                Milestone::CounterApproachReached,
                Milestone::CounterSupportCleared,
                Milestone::CounterSignalDecoded,
                Milestone::CounterBastionReached,
                Milestone::CounterBastionCleared,
                Milestone::CounterSignalIsolated,
            ],
            Self::TheBreach => &[
                Milestone::BreachDoorReached,
                Milestone::BreachCircuitStabilized,
                Milestone::BreachGuardCleared,
                Milestone::BreachCoreReached,
            ],
            Self::PrismCore => &[
                Milestone::PrismChamberReached,
                Milestone::PrismWardenCleared,
                Milestone::PrismSourceStopped,
                Milestone::PrismReturnCompleted,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Milestone {
    RelayGuardCleared,
    ReturnSignalRecovered,
    MeridianReached,
    SurveyLinked,
    RecoveryDelivered,
    SupplyRouteReached,
    SupplyGuardCleared,
    SupplyCellRecovered,
    SupplyDelivered,
    CounterApproachReached,
    CounterSupportCleared,
    CounterSignalDecoded,
    CounterBastionReached,
    CounterBastionCleared,
    CounterSignalIsolated,
    BreachDoorReached,
    BreachCircuitStabilized,
    BreachGuardCleared,
    BreachCoreReached,
    PrismChamberReached,
    PrismWardenCleared,
    PrismSourceStopped,
    PrismReturnCompleted,
}

/// Stable evidence identifiers for the four chapter-six boundaries.
#[must_use]
pub const fn prism_objective(milestone: Milestone) -> Option<&'static str> {
    match milestone {
        Milestone::PrismChamberReached => Some("prism-core-v1:arrival:5,0,2"),
        Milestone::PrismWardenCleared => Some("prism-core-v1:warden-cleared"),
        Milestone::PrismSourceStopped => Some("prism-core-v1:shutdown:8,0,0"),
        Milestone::PrismReturnCompleted => Some("prism-core-v1:return:0,0,0"),
        _ => None,
    }
}

/// Spatial evidence for the chapter-five relay and core boundaries.
#[must_use]
pub const fn breach_objective(milestone: Milestone) -> Option<&'static str> {
    match milestone {
        Milestone::BreachDoorReached => Some("breach-v1:door:6,0,0"),
        Milestone::BreachCircuitStabilized => Some("breach-v1:stabilizer:3,0,3"),
        Milestone::BreachCoreReached => Some("breach-v1:core:10,0,0"),
        _ => None,
    }
}

/// Stable world evidence for the six authored chapter-four boundaries.
#[must_use]
pub const fn counter_objective(milestone: Milestone) -> Option<&'static str> {
    match milestone {
        Milestone::CounterApproachReached => Some("counter-v1:approach:2,0,6"),
        Milestone::CounterSupportCleared => Some("counter-v1:support-cleared"),
        Milestone::CounterSignalDecoded => Some("counter-v1:transmission:7,0,6"),
        Milestone::CounterBastionReached => Some("counter-v1:bastion:5,0,-6"),
        Milestone::CounterBastionCleared => Some("counter-v1:bastion-cleared"),
        Milestone::CounterSignalIsolated => Some("counter-v1:emitter:9,0,-6"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Progress,
    Practice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterRun {
    pub run_id: String,
    pub chapter: ChapterId,
    pub mode: RunMode,
    /// Number of accepted safe boundaries; the next encounter restarts from
    /// this boundary, never from an unconfirmed client position or health value.
    pub checkpoint: u8,
}

impl ChapterRun {
    #[must_use]
    pub fn next_milestone(&self) -> Option<Milestone> {
        self.chapter
            .milestones()
            .get(usize::from(self.checkpoint))
            .copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignState {
    pub revision: u64,
    /// A cleared prefix of the six chapters. Later clears cannot skip a chapter.
    pub cleared_chapters: u8,
    pub active: Option<ChapterRun>,
    /// Absent in frozen v1-v5 journals; omitted defaults preserve their encoding.
    #[serde(default, skip_serializing_if = "story::StoryState::is_default")]
    pub story: story::StoryState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Enter {
        run_id: String,
        chapter: ChapterId,
        mode: RunMode,
    },
    Confirm {
        run_id: String,
        milestone: Milestone,
    },
    Story {
        run_id: String,
        /// Supplied by the authoritative actor, never by a client coordinate.
        position: [i32; 3],
        action: story::Command,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirstClear {
    pub chapter: ChapterId,
    pub fragments: u32,
    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub revision: String,
    pub before: CampaignState,
    pub command: Command,
    pub after: CampaignState,
    pub first_clear: Option<FirstClear>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignError {
    InvalidState,
    InvalidRunId,
    StaleRevision,
    RevisionOverflow,
    ChapterUnavailable,
    ChapterLocked,
    AlreadyCleared,
    PracticeRequiresClear,
    ResumeRequired,
    NoActiveRun,
    WrongRun,
    WrongMilestone,
    InvalidTransition,
    InvalidStory(story::StoryError),
}

impl fmt::Display for CampaignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidState => "campaign checkpoint is inconsistent",
            Self::InvalidRunId => "campaign run identity is invalid",
            Self::StaleRevision => "campaign state changed; refresh before continuing",
            Self::RevisionOverflow => "campaign revision exceeds persistence bounds",
            Self::ChapterUnavailable => "chapter is not available in this content revision",
            Self::ChapterLocked => "complete the preceding chapter first",
            Self::AlreadyCleared => "cleared chapters require practice mode",
            Self::PracticeRequiresClear => "practice requires a cleared chapter",
            Self::ResumeRequired => "resume the saved chapter before starting another",
            Self::NoActiveRun => "no campaign chapter is active",
            Self::WrongRun => "milestone belongs to a different campaign run",
            Self::WrongMilestone => "milestone is not the next accepted campaign boundary",
            Self::InvalidTransition => "campaign transition or reward evidence is inconsistent",
            Self::InvalidStory(_) => "story interaction is not valid at this campaign boundary",
        })
    }
}
impl std::error::Error for CampaignError {}

impl CampaignState {
    /// The direct approach fights the existing Warden before draining the relay.
    /// Frozen policies cannot contain a recorded choice and retain the old order.
    #[must_use]
    pub fn milestones(&self, chapter: ChapterId) -> &'static [Milestone] {
        if chapter == ChapterId::TheBreach
            && self.story.core_approach() == story::CoreApproach::Direct
        {
            &[
                Milestone::BreachDoorReached,
                Milestone::BreachGuardCleared,
                Milestone::BreachCircuitStabilized,
                Milestone::BreachCoreReached,
            ]
        } else {
            chapter.milestones()
        }
    }

    #[must_use]
    pub fn next_milestone(&self) -> Option<Milestone> {
        self.active.as_ref().and_then(|run| {
            self.milestones(run.chapter)
                .get(usize::from(run.checkpoint))
                .copied()
        })
    }

    /// Validates loaded state; legacy characters use `Default` without a write.
    ///
    /// # Errors
    /// Rejects unsupported chapter progress, invalid identities or checkpoints.
    pub fn validate(&self) -> Result<(), CampaignError> {
        self.validate_available(AVAILABLE_CHAPTERS)
    }

    fn validate_available(&self, available: u8) -> Result<(), CampaignError> {
        if self.revision > MAX_REVISION || self.cleared_chapters > available {
            return Err(CampaignError::InvalidState);
        }
        if let Some(run) = &self.active {
            validate_run_id(&run.run_id)?;
            if run.chapter.index() >= available
                || usize::from(run.checkpoint) >= run.chapter.milestones().len()
                || match run.mode {
                    RunMode::Progress => run.chapter.index() != self.cleared_chapters,
                    RunMode::Practice => run.chapter.index() >= self.cleared_chapters,
                }
                || self.revision == 0
            {
                return Err(CampaignError::InvalidState);
            }
        }
        if self.cleared_chapters > 0 && self.revision == 0 {
            return Err(CampaignError::InvalidState);
        }
        Ok(())
    }

    /// Returns a proposed transition without mutating the accepted state. A
    /// database retry must first resolve its durable operation identity; it must
    /// not run this command again and credit another grant.
    ///
    /// # Errors
    /// Rejects stale state, invalid progression, run identity, order or overflow.
    pub fn propose(
        &self,
        expected_revision: u64,
        command: Command,
    ) -> Result<Transition, CampaignError> {
        self.propose_policy(expected_revision, command, REVISION, AVAILABLE_CHAPTERS)
    }

    fn propose_policy(
        &self,
        expected_revision: u64,
        command: Command,
        policy: &str,
        available: u8,
    ) -> Result<Transition, CampaignError> {
        self.validate_available(available)?;
        if policy != REVISION
            && (!self.story.is_default() || matches!(command, Command::Story { .. }))
        {
            return Err(CampaignError::InvalidTransition);
        }
        if expected_revision != self.revision {
            return Err(CampaignError::StaleRevision);
        }
        let mut after = self.clone();
        after.revision = self
            .revision
            .checked_add(1)
            .filter(|r| *r <= MAX_REVISION)
            .ok_or(CampaignError::RevisionOverflow)?;
        let first_clear = match &command {
            Command::Story {
                run_id,
                position,
                action,
            } => {
                after.story = self.propose_story(run_id, *position, *action)?;
                None
            }
            Command::Enter {
                run_id,
                chapter,
                mode,
            } => {
                validate_run_id(run_id)?;
                if self.active.is_some() {
                    return Err(CampaignError::ResumeRequired);
                }
                if chapter.index() >= available {
                    return Err(CampaignError::ChapterUnavailable);
                }
                match mode {
                    RunMode::Progress if chapter.index() < self.cleared_chapters => {
                        return Err(CampaignError::AlreadyCleared)
                    }
                    RunMode::Progress if chapter.index() > self.cleared_chapters => {
                        return Err(CampaignError::ChapterLocked)
                    }
                    RunMode::Practice if chapter.index() >= self.cleared_chapters => {
                        return Err(CampaignError::PracticeRequiresClear)
                    }
                    _ => {}
                }
                after.active = Some(ChapterRun {
                    run_id: run_id.clone(),
                    chapter: *chapter,
                    mode: *mode,
                    checkpoint: 0,
                });
                None
            }
            Command::Confirm { run_id, milestone } => {
                let run = after.active.as_mut().ok_or(CampaignError::NoActiveRun)?;
                if &run.run_id != run_id {
                    return Err(CampaignError::WrongRun);
                }
                if self.next_milestone() != Some(*milestone) {
                    return Err(CampaignError::WrongMilestone);
                }
                run.checkpoint += 1;
                if usize::from(run.checkpoint) == run.chapter.milestones().len() {
                    let clear = (run.mode == RunMode::Progress).then_some(FirstClear {
                        chapter: run.chapter,
                        fragments: FIRST_CLEAR_FRAGMENTS,
                        experience: FIRST_CLEAR_EXPERIENCE,
                    });
                    if clear.is_some() {
                        after.cleared_chapters += 1;
                    }
                    after.active = None;
                    clear
                } else {
                    None
                }
            }
        };
        after.validate_available(available)?;
        Ok(Transition {
            revision: policy.to_owned(),
            before: self.clone(),
            command,
            after,
            first_clear,
        })
    }

    fn propose_story(
        &self,
        run_id: &str,
        position: [i32; 3],
        action: story::Command,
    ) -> Result<story::StoryState, CampaignError> {
        let run = self.active.as_ref().ok_or(CampaignError::NoActiveRun)?;
        if run.run_id != run_id {
            return Err(CampaignError::WrongRun);
        }
        self.story
            .propose(
                story::Context {
                    chapter: run.chapter,
                    checkpoint: run.checkpoint,
                    position,
                },
                action,
            )
            .map_err(CampaignError::InvalidStory)
    }
}

impl Transition {
    /// Checks the entire transition against the frozen policy, including the
    /// first-clear grant. Presence in JSON alone is never accepted as evidence.
    ///
    /// # Errors
    /// Rejects altered checkpoints, rewards, commands or policy revisions.
    pub fn validate(&self) -> Result<(), CampaignError> {
        let available = match self.revision.as_str() {
            LEGACY_REVISION => 2,
            SUPPLY_REVISION => 3,
            COUNTER_REVISION => 4,
            BREACH_REVISION => 5,
            PRISM_REVISION => 6,
            REVISION => AVAILABLE_CHAPTERS,
            _ => return Err(CampaignError::InvalidTransition),
        };
        if self.before.propose_policy(
            self.before.revision,
            self.command.clone(),
            &self.revision,
            available,
        )? != *self
        {
            return Err(CampaignError::InvalidTransition);
        }
        Ok(())
    }
}

fn validate_run_id(id: &str) -> Result<(), CampaignError> {
    if id.is_empty() || id.len() > 96 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(CampaignError::InvalidRunId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter(chapter: ChapterId, mode: RunMode) -> Command {
        Command::Enter {
            run_id: format!("run-{}", chapter.as_str().replace('_', "-")),
            chapter,
            mode,
        }
    }

    fn advance(state: &CampaignState, milestone: Milestone) -> Transition {
        state
            .propose(
                state.revision,
                Command::Confirm {
                    run_id: state.active.as_ref().unwrap().run_id.clone(),
                    milestone,
                },
            )
            .unwrap()
    }

    #[test]
    fn story_journal_preserves_frozen_policies_and_requires_its_original_boundary() {
        use story::{Command as S, SupplyApproach};
        let state = CampaignState {
            revision: 5,
            cleared_chapters: 1,
            active: Some(ChapterRun {
                run_id: "story-run".to_owned(),
                chapter: ChapterId::MeridianReadings,
                mode: RunMode::Progress,
                checkpoint: 2,
            }),
            ..CampaignState::default()
        };
        let command = Command::Story {
            run_id: "story-run".to_owned(),
            position: story::DEPARTURE_BOARD,
            action: S::ChooseSupply {
                approach: SupplyApproach::Service,
            },
        };
        let choice = state.propose(5, command.clone()).unwrap();
        choice.validate().unwrap();
        assert!(choice.first_clear.is_none());
        assert_eq!(choice.after.active, state.active);
        assert_eq!(choice.after.cleared_chapters, 1);
        assert_eq!(
            state.propose(4, command.clone()),
            Err(CampaignError::StaleRevision)
        );
        for policy in [
            LEGACY_REVISION,
            SUPPLY_REVISION,
            COUNTER_REVISION,
            BREACH_REVISION,
            PRISM_REVISION,
        ] {
            let mut forged = choice.clone();
            forged.revision = policy.to_owned();
            assert!(forged.validate().is_err());
            let empty_entry = CampaignState::default()
                .propose_policy(
                    0,
                    enter(ChapterId::ReturnSignal, RunMode::Progress),
                    policy,
                    2,
                )
                .unwrap();
            empty_entry.validate().unwrap();
            let encoded = serde_json::to_string(&empty_entry).unwrap();
            assert!(!encoded.contains("story"));
            serde_json::from_str::<Transition>(&encoded)
                .unwrap()
                .validate()
                .unwrap();
            let mut injected = empty_entry;
            injected.before.story = choice.after.story.clone();
            injected.after.story = choice.after.story.clone();
            assert!(injected.validate().is_err());
        }
        let mut wrong_run = command.clone();
        if let Command::Story { run_id, .. } = &mut wrong_run {
            *run_id = "another-run".to_owned();
        }
        assert_eq!(state.propose(5, wrong_run), Err(CampaignError::WrongRun));
        let mut wrong_context = state;
        wrong_context.cleared_chapters = 2;
        wrong_context.active.as_mut().unwrap().chapter = ChapterId::BrokenSupplyLine;
        assert!(wrong_context.propose(5, command).is_err());
    }

    #[test]
    fn direct_breach_requires_guard_then_stabilizer_and_preserves_practice_rewards() {
        let mut state = CampaignState {
            revision: 20,
            cleared_chapters: 4,
            story: story::StoryState {
                core: Some(story::CoreApproach::Direct),
                ..Default::default()
            },
            ..CampaignState::default()
        };
        for mode in [RunMode::Progress, RunMode::Practice] {
            state = state
                .propose(state.revision, enter(ChapterId::TheBreach, mode))
                .unwrap()
                .after;
            state = advance(&state, Milestone::BreachDoorReached).after;
            assert!(state
                .propose(
                    state.revision,
                    Command::Confirm {
                        run_id: state.active.as_ref().unwrap().run_id.clone(),
                        milestone: Milestone::BreachCircuitStabilized
                    }
                )
                .is_err());
            state = advance(&state, Milestone::BreachGuardCleared).after;
            state = advance(&state, Milestone::BreachCircuitStabilized).after;
            let completed = advance(&state, Milestone::BreachCoreReached);
            assert_eq!(completed.first_clear.is_some(), mode == RunMode::Progress);
            completed.validate().unwrap();
            state = completed.after;
        }
        assert_eq!(state.cleared_chapters, 5);
    }

    #[test]
    fn two_chapters_resume_from_confirmed_boundary_and_practice_never_regrants() {
        let mut state = CampaignState::default();
        let mut grants = Vec::new();
        for chapter in [ChapterId::ReturnSignal, ChapterId::MeridianReadings] {
            state = state
                .propose(state.revision, enter(chapter, RunMode::Progress))
                .unwrap()
                .after;
            for milestone in chapter.milestones() {
                let next = advance(&state, *milestone);
                next.validate().unwrap();
                assert_eq!(next.before, state);
                if let Some(grant) = next.first_clear {
                    grants.push(grant);
                }
                // A reconnect restores only accepted state, not a partially
                // completed encounter. It cannot advance by reading the save.
                state = next.after.clone();
                state.validate().unwrap();
            }
        }
        assert_eq!(state.cleared_chapters, 2);
        assert!(state.active.is_none());
        assert_eq!(grants.len(), 2);
        assert_eq!(grants.iter().map(|g| g.fragments).sum::<u32>(), 2);
        for chapter in [ChapterId::ReturnSignal, ChapterId::MeridianReadings] {
            state = state
                .propose(state.revision, enter(chapter, RunMode::Practice))
                .unwrap()
                .after;
            for milestone in chapter.milestones() {
                let transition = advance(&state, *milestone);
                assert!(transition.first_clear.is_none());
                transition.validate().unwrap();
                state = transition.after;
            }
        }
        assert_eq!(state.cleared_chapters, 2);
    }

    #[test]
    fn rejected_commands_leave_accepted_checkpoint_untouched() {
        let empty = CampaignState::default();
        assert_eq!(
            empty.propose(0, enter(ChapterId::MeridianReadings, RunMode::Progress)),
            Err(CampaignError::ChapterLocked)
        );
        assert_eq!(
            empty.propose(0, enter(ChapterId::PrismCore, RunMode::Progress)),
            Err(CampaignError::ChapterLocked)
        );
        assert_eq!(
            empty.propose(0, enter(ChapterId::ReturnSignal, RunMode::Practice)),
            Err(CampaignError::PracticeRequiresClear)
        );
        let state = empty
            .propose(0, enter(ChapterId::ReturnSignal, RunMode::Progress))
            .unwrap()
            .after;
        assert_eq!(
            state.propose(1, enter(ChapterId::ReturnSignal, RunMode::Progress)),
            Err(CampaignError::ResumeRequired)
        );
        for (revision, id, milestone, error) in [
            (
                0,
                "run-return-signal",
                Milestone::RelayGuardCleared,
                CampaignError::StaleRevision,
            ),
            (
                1,
                "another-run",
                Milestone::RelayGuardCleared,
                CampaignError::WrongRun,
            ),
            (
                1,
                "run-return-signal",
                Milestone::ReturnSignalRecovered,
                CampaignError::WrongMilestone,
            ),
        ] {
            assert_eq!(
                state.propose(
                    revision,
                    Command::Confirm {
                        run_id: id.to_owned(),
                        milestone
                    }
                ),
                Err(error)
            );
            assert_eq!(state.active.as_ref().unwrap().checkpoint, 0);
        }
    }

    #[test]
    fn prism_grants_only_on_return_and_freezes_the_five_chapter_policy() {
        let mut state = CampaignState {
            story: story::StoryState::default(),
            revision: 25,
            cleared_chapters: 5,
            active: None,
        };
        assert_eq!(
            state.propose_policy(
                state.revision,
                enter(ChapterId::PrismCore, RunMode::Progress),
                BREACH_REVISION,
                5
            ),
            Err(CampaignError::ChapterUnavailable)
        );
        for mode in [RunMode::Progress, RunMode::Practice] {
            let entry = state
                .propose(state.revision, enter(ChapterId::PrismCore, mode))
                .unwrap();
            entry.validate().unwrap();
            let mut old = entry.clone();
            old.revision = BREACH_REVISION.to_owned();
            assert!(old.validate().is_err());
            state = entry.after;
            for (index, milestone) in ChapterId::PrismCore.milestones().iter().enumerate() {
                let run_id = state.active.as_ref().unwrap().run_id.clone();
                if index < 3 {
                    assert_eq!(
                        state.propose(
                            state.revision,
                            Command::Confirm {
                                run_id: run_id.clone(),
                                milestone: Milestone::PrismReturnCompleted
                            }
                        ),
                        Err(CampaignError::WrongMilestone)
                    );
                }
                let transition = state
                    .propose(
                        state.revision,
                        Command::Confirm {
                            run_id,
                            milestone: *milestone,
                        },
                    )
                    .unwrap();
                transition.validate().unwrap();
                assert_eq!(
                    transition.first_clear.is_some(),
                    mode == RunMode::Progress && index == 3
                );
                state = transition.after;
            }
            assert_eq!(state.cleared_chapters, 6);
        }
    }

    #[test]
    fn supply_progress_preserves_frozen_two_chapter_policy() {
        let mut state = CampaignState::default();
        for chapter in [ChapterId::ReturnSignal, ChapterId::MeridianReadings] {
            let entry = state
                .propose_policy(
                    state.revision,
                    enter(chapter, RunMode::Progress),
                    LEGACY_REVISION,
                    2,
                )
                .unwrap();
            entry.validate().unwrap();
            state = entry.after;
            for milestone in chapter.milestones() {
                let command = Command::Confirm {
                    run_id: state.active.as_ref().unwrap().run_id.clone(),
                    milestone: *milestone,
                };
                let transition = state
                    .propose_policy(state.revision, command, LEGACY_REVISION, 2)
                    .unwrap();
                transition.validate().unwrap();
                state = transition.after;
            }
        }
        let entry = state
            .propose(
                state.revision,
                enter(ChapterId::BrokenSupplyLine, RunMode::Progress),
            )
            .unwrap();
        entry.validate().unwrap();
        let mut forged_legacy = entry.clone();
        forged_legacy.revision = LEGACY_REVISION.to_owned();
        assert!(forged_legacy.validate().is_err());
        state = entry.after;
        for milestone in ChapterId::BrokenSupplyLine.milestones() {
            let transition = advance(&state, *milestone);
            transition.validate().unwrap();
            assert_eq!(
                transition.first_clear.is_some(),
                *milestone == Milestone::SupplyDelivered
            );
            state = transition.after;
        }
        assert_eq!(state.cleared_chapters, 3);
        state = state
            .propose(
                state.revision,
                enter(ChapterId::BrokenSupplyLine, RunMode::Practice),
            )
            .unwrap()
            .after;
        for milestone in ChapterId::BrokenSupplyLine.milestones() {
            let transition = advance(&state, *milestone);
            assert!(transition.first_clear.is_none());
            state = transition.after;
        }
        assert_eq!(state.cleared_chapters, 3);
    }

    #[test]
    fn counter_chapter_preserves_v2_limits_and_rewards_only_the_final_boundary() {
        let mut state = CampaignState {
            story: story::StoryState::default(),
            revision: 12,
            cleared_chapters: 3,
            active: None,
        };
        assert_eq!(
            state.propose_policy(
                12,
                enter(ChapterId::CounterSignal, RunMode::Progress),
                SUPPLY_REVISION,
                3
            ),
            Err(CampaignError::ChapterUnavailable)
        );
        for mode in [RunMode::Progress, RunMode::Practice] {
            let entry = state
                .propose(state.revision, enter(ChapterId::CounterSignal, mode))
                .unwrap();
            let mut forged = entry.clone();
            forged.revision = SUPPLY_REVISION.to_owned();
            assert!(forged.validate().is_err());
            state = entry.after;
            for milestone in ChapterId::CounterSignal.milestones() {
                let transition = advance(&state, *milestone);
                transition.validate().unwrap();
                assert_eq!(
                    transition.first_clear.is_some(),
                    mode == RunMode::Progress && *milestone == Milestone::CounterSignalIsolated
                );
                state = transition.after;
            }
            assert_eq!(state.cleared_chapters, 4);
        }
        assert_eq!(
            state.propose(
                state.revision,
                enter(ChapterId::PrismCore, RunMode::Progress)
            ),
            Err(CampaignError::ChapterLocked)
        );
    }

    #[test]
    fn breach_freezes_v3_and_rewards_only_crossing_the_cleared_core() {
        let mut state = CampaignState {
            story: story::StoryState::default(),
            revision: 20,
            cleared_chapters: 4,
            active: None,
        };
        for mode in [RunMode::Progress, RunMode::Practice] {
            let entry = state
                .propose(state.revision, enter(ChapterId::TheBreach, mode))
                .unwrap();
            let mut old = entry.clone();
            old.revision = COUNTER_REVISION.to_owned();
            assert!(old.validate().is_err());
            state = entry.after;
            assert_eq!(
                advance_error(&state, Milestone::BreachGuardCleared),
                CampaignError::WrongMilestone
            );
            for milestone in ChapterId::TheBreach.milestones() {
                let transition = advance(&state, *milestone);
                transition.validate().unwrap();
                assert_eq!(
                    transition.first_clear.is_some(),
                    mode == RunMode::Progress && *milestone == Milestone::BreachCoreReached
                );
                state = transition.after;
            }
            assert_eq!(state.cleared_chapters, 5);
        }
    }

    fn advance_error(state: &CampaignState, milestone: Milestone) -> CampaignError {
        state
            .propose(
                state.revision,
                Command::Confirm {
                    run_id: state.active.as_ref().unwrap().run_id.clone(),
                    milestone,
                },
            )
            .unwrap_err()
    }

    #[test]
    fn corrupt_save_and_reward_evidence_fail_closed() {
        let start = CampaignState::default()
            .propose(0, enter(ChapterId::ReturnSignal, RunMode::Progress))
            .unwrap();
        let checkpoint = advance(&start.after, Milestone::RelayGuardCleared);
        let clear = advance(&checkpoint.after, Milestone::ReturnSignalRecovered);
        for field in 0..4 {
            let mut corrupt = clear.clone();
            match field {
                0 => corrupt.first_clear.as_mut().unwrap().fragments += 1,
                1 => corrupt.after.cleared_chapters += 1,
                2 => corrupt.after.revision += 1,
                _ => corrupt.revision = "unknown".to_owned(),
            }
            assert!(corrupt.validate().is_err());
        }
        let mut corrupt = start.after.clone();
        corrupt.active.as_mut().unwrap().checkpoint = 2;
        assert!(corrupt.validate().is_err());
        corrupt = start.after;
        corrupt.active.as_mut().unwrap().mode = RunMode::Practice;
        assert!(corrupt.validate().is_err());
        let limit = CampaignState {
            story: story::StoryState::default(),
            revision: MAX_REVISION,
            ..CampaignState::default()
        };
        assert_eq!(
            limit.propose(
                MAX_REVISION,
                enter(ChapterId::ReturnSignal, RunMode::Progress)
            ),
            Err(CampaignError::RevisionOverflow)
        );
    }
}
