//! Frozen, non-power distinctions derived from verified attempts. The immutable
//! challenge journal is their durable source; no mutable counter or grant exists.
use revenant_modules::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{AttemptResult, ContractId, Equipment, Outcome};

pub const REVISION: &str = "m37-mastery-v1";
pub const WEST_ROUTE_MOVE_LIMIT: u64 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Goal {
    Breacher,
    Marksman,
    Guard,
    RoutePlanner,
    TargetPriority,
    PrismExecution,
}

impl Goal {
    pub const ALL: [Self; 6] = [
        Self::Breacher,
        Self::Marksman,
        Self::Guard,
        Self::RoutePlanner,
        Self::TargetPriority,
        Self::PrismExecution,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Breacher => "breacher",
            Self::Marksman => "marksman",
            Self::Guard => "guard",
            Self::RoutePlanner => "route_planner",
            Self::TargetPriority => "target_priority",
            Self::PrismExecution => "prism_execution",
        }
    }

    #[must_use]
    pub const fn contract(self) -> ContractId {
        match self {
            Self::Breacher => ContractId::CloseQuarters,
            Self::Marksman => ContractId::DistantSignal,
            Self::Guard => ContractId::LastReserve,
            Self::RoutePlanner => ContractId::MeridianCircuit,
            Self::TargetPriority => ContractId::BastionLink,
            Self::PrismExecution => ContractId::PrismDiscipline,
        }
    }

    #[must_use]
    pub fn accepts_equipment(self, equipment: &Equipment) -> bool {
        let (weapon, modules) = match self {
            Self::Breacher => (
                "scatter_caster",
                [ModuleId::FocusLens, ModuleId::BreachShunt],
            ),
            Self::Marksman => (
                "rail_driver",
                [ModuleId::StandoffOptic, ModuleId::SkirmishDrive],
            ),
            Self::Guard => (
                "pulse_rifle",
                [ModuleId::CycleBypass, ModuleId::AblativeShell],
            ),
            _ => return true,
        };
        equipment.weapon_id == weapon && modules.iter().all(|id| equipment.modules.contains(id))
    }
}

/// Evidence summaries are supplied by the replay adapter after full validation.
/// Defaults never assert clean play, an unused reserve or an efficient route.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub reserve_unused: bool,
    pub west_route_moves: Option<u64>,
    pub mender_before_bulwark_damage: bool,
    pub prism_damage: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Achieved,
    CompleteContract,
    UseBuild,
    PreserveReserve,
    UseWestRoute,
    ReduceRouteMoves,
    MenderFirst,
    AvoidPrismDamage,
}

impl Reason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Achieved => "achieved",
            Self::CompleteContract => "complete_contract",
            Self::UseBuild => "use_build",
            Self::PreserveReserve => "preserve_reserve",
            Self::UseWestRoute => "use_west_route",
            Self::ReduceRouteMoves => "reduce_route_moves",
            Self::MenderFirst => "mender_first",
            Self::AvoidPrismDamage => "avoid_prism_damage",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assessment {
    pub goal: Goal,
    pub reason: Reason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub run_id: u64,
    pub assessments: Vec<Assessment>,
    pub route_moves: Option<u64>,
    pub prism_damage: Option<u64>,
}

/// A pure classification, not proof of authority. Replay must validate the full
/// session and persistence must bind it to the authenticated character's journal.
#[must_use]
pub fn assess(result: &AttemptResult, facts: &Facts) -> Attempt {
    let run = &result.run;
    let assessments = Goal::ALL
        .into_iter()
        .filter(|goal| goal.contract() == run.rules.contract)
        .map(|goal| {
            let reason = if result.outcome != Outcome::Completed {
                Reason::CompleteContract
            } else if !goal.accepts_equipment(&run.equipment) {
                Reason::UseBuild
            } else {
                match goal {
                    Goal::Guard if !facts.reserve_unused => Reason::PreserveReserve,
                    Goal::RoutePlanner => match facts.west_route_moves {
                        None => Reason::UseWestRoute,
                        Some(0) => Reason::ReduceRouteMoves,
                        Some(moves) if moves > WEST_ROUTE_MOVE_LIMIT => Reason::ReduceRouteMoves,
                        _ => Reason::Achieved,
                    },
                    Goal::TargetPriority if !facts.mender_before_bulwark_damage => {
                        Reason::MenderFirst
                    }
                    Goal::PrismExecution if facts.prism_damage != Some(0) => {
                        Reason::AvoidPrismDamage
                    }
                    _ => Reason::Achieved,
                }
            };
            Assessment { goal, reason }
        })
        .collect();
    Attempt {
        run_id: run.run_id,
        assessments,
        route_moves: facts.west_route_moves,
        prism_damage: facts.prism_damage,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub goal: Goal,
    pub run_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Badge {
    ArsenalAdept,
    FieldTactician,
    PrismAdept,
}

impl Badge {
    pub const ALL: [Self; 3] = [Self::ArsenalAdept, Self::FieldTactician, Self::PrismAdept];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArsenalAdept => "arsenal_adept",
            Self::FieldTactician => "field_tactician",
            Self::PrismAdept => "prism_adept",
        }
    }

    #[must_use]
    pub const fn goals(self) -> &'static [Goal] {
        match self {
            Self::ArsenalAdept => &[Goal::Breacher, Goal::Marksman, Goal::Guard],
            Self::FieldTactician => &[Goal::RoutePlanner, Goal::TargetPriority],
            Self::PrismAdept => &[Goal::PrismExecution],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BadgeRecord {
    pub badge: Badge,
    pub run_id: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Archive {
    pub records: Vec<Record>,
    pub last_attempt: Option<Attempt>,
}

impl Archive {
    /// Idempotent even if history is reread or visited out of order. The earliest
    /// qualifying attempt remains the proof and the latest terminal the feedback.
    pub fn observe(&mut self, attempt: Attempt) {
        for assessment in &attempt.assessments {
            if assessment.reason != Reason::Achieved {
                continue;
            }
            if let Some(record) = self.records.iter_mut().find(|r| r.goal == assessment.goal) {
                record.run_id = record.run_id.min(attempt.run_id);
            } else {
                self.records.push(Record {
                    goal: assessment.goal,
                    run_id: attempt.run_id,
                });
            }
        }
        self.records.sort_by_key(|record| record.goal);
        if self
            .last_attempt
            .as_ref()
            .is_none_or(|last| last.run_id < attempt.run_id)
        {
            self.last_attempt = Some(attempt);
        }
    }

    #[must_use]
    pub fn badges(&self) -> Vec<BadgeRecord> {
        Badge::ALL
            .into_iter()
            .filter_map(|badge| {
                let runs: Option<Vec<_>> = badge
                    .goals()
                    .iter()
                    .map(|goal| {
                        self.records
                            .iter()
                            .find(|record| record.goal == *goal)
                            .map(|record| record.run_id)
                    })
                    .collect();
                Some(BadgeRecord {
                    badge,
                    run_id: *runs?.iter().max()?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
