//! Bounded narrative policy attached to v6 campaign journals.
//! Callers must construct context from the admitted run and authoritative actor.
//! No command here creates rewards or changes the six-chapter clear prefix.
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ChapterId;

pub const REVISION: &str = "m36-story-v1";
pub const MEMORY: [i32; 3] = [-31, 0, 8];
pub const DEPARTURE_BOARD: [i32; 3] = [-18, 0, 0];
pub const MAINTENANCE_NOTE: [i32; 3] = [-6, 0, -3];
pub const MAINTENANCE_BENCH: [i32; 3] = [2, 0, 4];
pub const CORE_DECISION: [i32; 3] = [7, 0, -6];
pub const SERVICE_APPROACH: [i32; 3] = [-8, 0, -3];
pub const SERVICE_OBJECTIVE: &str = "supply-service-v1:route:-8,0,-3";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcProgress {
    #[default]
    Unseen,
    Found,
    Returned,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupplyApproach {
    #[default]
    Covered,
    Service,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreApproach {
    #[default]
    Grounded,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Epilogue {
    SignalSilent,
    RouteLit,
    OpenPassage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Discovery {
    EmptySeatMemory,
    EmptySeatReturned,
    HeldConnectionNote,
    HeldConnectionReturned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Discover { discovery: Discovery },
    ChooseSupply { approach: SupplyApproach },
    ChooseCore { approach: CoreApproach },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    pub chapter: ChapterId,
    pub checkpoint: u8,
    pub position: [i32; 3],
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StoryState {
    pub empty_seat: ArcProgress,
    pub held_connection: ArcProgress,
    /// None distinguishes the inherited default from an explicit decision.
    pub supply: Option<SupplyApproach>,
    pub core: Option<CoreApproach>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryError {
    WrongContext,
    DiscoveryRequired,
}

impl fmt::Display for StoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WrongContext => "story interaction is outside its admitted chapter boundary",
            Self::DiscoveryRequired => "recover the optional record before returning it",
        })
    }
}
impl std::error::Error for StoryError {}

impl StoryState {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    #[must_use]
    pub fn supply_approach(&self) -> SupplyApproach {
        self.supply.unwrap_or_default()
    }

    #[must_use]
    pub fn core_approach(&self) -> CoreApproach {
        self.core.unwrap_or_default()
    }

    #[must_use]
    pub fn epilogue(&self) -> Epilogue {
        match (self.supply_approach(), self.core_approach()) {
            (_, CoreApproach::Direct) => Epilogue::OpenPassage,
            (SupplyApproach::Service, CoreApproach::Grounded) => Epilogue::RouteLit,
            (SupplyApproach::Covered, CoreApproach::Grounded) => Epilogue::SignalSilent,
        }
    }

    /// Proposes a new story state without mutating the accepted one. Repeating
    /// a discovery never regresses it; a later replay may replace a choice only
    /// at its original decision point. Persistence owns revision/idempotency.
    ///
    /// # Errors
    /// Rejects a wrong chapter, checkpoint or position, and returning an unseen record.
    pub fn propose(&self, context: Context, command: Command) -> Result<Self, StoryError> {
        if !allowed(context, command) {
            return Err(StoryError::WrongContext);
        }
        let mut next = self.clone();
        match command {
            Command::ChooseSupply { approach } => next.supply = Some(approach),
            Command::ChooseCore { approach } => next.core = Some(approach),
            Command::Discover { discovery } => {
                let (progress, returning) = match discovery {
                    Discovery::EmptySeatMemory => (&mut next.empty_seat, false),
                    Discovery::EmptySeatReturned => (&mut next.empty_seat, true),
                    Discovery::HeldConnectionNote => (&mut next.held_connection, false),
                    Discovery::HeldConnectionReturned => (&mut next.held_connection, true),
                };
                *progress = match (*progress, returning) {
                    (ArcProgress::Unseen, true) => return Err(StoryError::DiscoveryRequired),
                    (ArcProgress::Unseen | ArcProgress::Found, false) => ArcProgress::Found,
                    (ArcProgress::Found, true) | (ArcProgress::Returned, _) => {
                        ArcProgress::Returned
                    }
                };
            }
        }
        Ok(next)
    }
}

fn allowed(context: Context, command: Command) -> bool {
    let (chapter, checkpoints, position): (_, &[u8], _) = match command {
        Command::Discover {
            discovery: Discovery::EmptySeatMemory,
        } => (ChapterId::MeridianReadings, &[1, 2], MEMORY),
        Command::Discover {
            discovery: Discovery::EmptySeatReturned,
        } => (ChapterId::MeridianReadings, &[1, 2], DEPARTURE_BOARD),
        Command::Discover {
            discovery: Discovery::HeldConnectionNote,
        } => (ChapterId::BrokenSupplyLine, &[2, 3], MAINTENANCE_NOTE),
        Command::Discover {
            discovery: Discovery::HeldConnectionReturned,
        } => (ChapterId::BrokenSupplyLine, &[2, 3], MAINTENANCE_BENCH),
        Command::ChooseSupply { .. } => (ChapterId::MeridianReadings, &[2], DEPARTURE_BOARD),
        Command::ChooseCore { .. } => (ChapterId::CounterSignal, &[5], CORE_DECISION),
    };
    context.chapter == chapter
        && checkpoints.contains(&context.checkpoint)
        && context.position == position
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(chapter: ChapterId, checkpoint: u8, position: [i32; 3]) -> Context {
        Context {
            chapter,
            checkpoint,
            position,
        }
    }

    #[test]
    fn optional_records_require_discovery_and_never_regress_on_revisit() {
        let mut state = StoryState::default();
        for (chapter, checkpoint, find, back, point, return_point) in [
            (
                ChapterId::MeridianReadings,
                1,
                Discovery::EmptySeatMemory,
                Discovery::EmptySeatReturned,
                MEMORY,
                DEPARTURE_BOARD,
            ),
            (
                ChapterId::BrokenSupplyLine,
                2,
                Discovery::HeldConnectionNote,
                Discovery::HeldConnectionReturned,
                MAINTENANCE_NOTE,
                MAINTENANCE_BENCH,
            ),
        ] {
            let return_context = context(chapter, checkpoint, return_point);
            assert_eq!(
                state.propose(return_context, Command::Discover { discovery: back }),
                Err(StoryError::DiscoveryRequired)
            );
            let find_context = context(chapter, checkpoint, point);
            let command = Command::Discover { discovery: find };
            state = state.propose(find_context, command).unwrap();
            assert_eq!(state.propose(find_context, command).unwrap(), state);
            state = state
                .propose(return_context, Command::Discover { discovery: back })
                .unwrap();
            assert_eq!(state.propose(find_context, command).unwrap(), state);
        }
        assert_eq!(
            (state.empty_seat, state.held_connection),
            (ArcProgress::Returned, ArcProgress::Returned)
        );
        assert_eq!(state.epilogue(), Epilogue::SignalSilent);
    }

    #[test]
    fn choices_are_bounded_to_their_safe_decision_points_and_three_endings() {
        let empty = StoryState::default();
        assert_eq!(
            (empty.supply_approach(), empty.core_approach()),
            (SupplyApproach::Covered, CoreApproach::Grounded)
        );
        let supply = Command::ChooseSupply {
            approach: SupplyApproach::Service,
        };
        for invalid in [
            context(ChapterId::MeridianReadings, 1, DEPARTURE_BOARD),
            context(ChapterId::BrokenSupplyLine, 0, DEPARTURE_BOARD),
            context(ChapterId::MeridianReadings, 2, MEMORY),
        ] {
            assert_eq!(
                empty.propose(invalid, supply),
                Err(StoryError::WrongContext)
            );
        }
        let choice_point = context(ChapterId::MeridianReadings, 2, DEPARTURE_BOARD);
        let service = empty.propose(choice_point, supply).unwrap();
        assert_eq!(service.epilogue(), Epilogue::RouteLit);
        assert_eq!(service.propose(choice_point, supply).unwrap(), service);
        let rerecorded = service
            .propose(
                choice_point,
                Command::ChooseSupply {
                    approach: SupplyApproach::Covered,
                },
            )
            .unwrap();
        assert_eq!(rerecorded.epilogue(), Epilogue::SignalSilent);
        assert_eq!(service.supply, Some(SupplyApproach::Service));
        let direct = Command::ChooseCore {
            approach: CoreApproach::Direct,
        };
        assert_eq!(
            service.propose(context(ChapterId::TheBreach, 0, CORE_DECISION), direct),
            Err(StoryError::WrongContext)
        );
        assert_eq!(
            service.propose(context(ChapterId::CounterSignal, 4, CORE_DECISION), direct),
            Err(StoryError::WrongContext)
        );
        for state in [empty, service] {
            let next = state
                .propose(context(ChapterId::CounterSignal, 5, CORE_DECISION), direct)
                .unwrap();
            assert_eq!(next.epilogue(), Epilogue::OpenPassage);
        }
    }

    #[test]
    fn omitted_story_defaults_and_unknown_fields_or_states_are_rejected() {
        let state: StoryState = serde_json::from_str("{}").unwrap();
        assert_eq!(state, StoryState::default());
        for invalid in [
            r#"{"empty_seat":"skipped"}"#,
            r#"{"empty_seat":3}"#,
            r#"{"supply":"teleport"}"#,
            r#"{"reward":1}"#,
        ] {
            assert!(serde_json::from_str::<StoryState>(invalid).is_err());
        }
        let found = state
            .propose(
                context(ChapterId::MeridianReadings, 1, MEMORY),
                Command::Discover {
                    discovery: Discovery::EmptySeatMemory,
                },
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<StoryState>(&serde_json::to_string(&found).unwrap()).unwrap(),
            found
        );
        for invalid in [
            context(ChapterId::MeridianReadings, 0, MEMORY),
            context(ChapterId::TheBreach, 1, MEMORY),
            context(ChapterId::MeridianReadings, 1, DEPARTURE_BOARD),
        ] {
            assert_eq!(
                found.propose(
                    invalid,
                    Command::Discover {
                        discovery: Discovery::EmptySeatMemory
                    }
                ),
                Err(StoryError::WrongContext)
            );
        }
    }
}
