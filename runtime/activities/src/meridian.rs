//! Authored, untimed exploration. The enclosing mission owns all rewards.
use std::sync::OnceLock;

use revenant_objectives::{Objective, ObjectiveKind, ObjectiveState, WorldTrigger};
use serde::Deserialize;

use crate::{ActivityError, ActivityEvent, ScriptedActivity};

pub const ENTRANCE: [i32; 3] = [-12, 0, 0];
pub const REVISION: &str = "meridian-v1";

/// Authored safe boundaries; partial work between them restarts on reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkpoint {
    Entrance,
    Arrival,
    SurveyComplete,
}

impl Checkpoint {
    #[must_use]
    pub const fn position(self) -> [i32; 3] {
        match self {
            Self::Entrance => ENTRANCE,
            Self::Arrival => [-16, 0, 0],
            Self::SurveyComplete => [-28, 0, 7],
        }
    }
}

#[derive(Deserialize)]
pub struct Sector {
    pub revision: String,
    pub floors: Vec<[i32; 4]>,
    pub stations: Vec<Station>,
}

#[derive(Deserialize)]
pub struct Station {
    pub id: String,
    pub position: [i32; 3],
}

/// # Panics
/// Panics if the embedded, repository-owned JSON is malformed.
#[must_use]
pub fn sector() -> &'static Sector {
    static SECTOR: OnceLock<Sector> = OnceLock::new();
    SECTOR.get_or_init(|| {
        serde_json::from_str(include_str!("../../../client/game/world/meridian.json"))
            .expect("embedded Meridian geometry is validated at build time by its content tests")
    })
}

#[must_use]
pub fn walkable(position: [i32; 3]) -> bool {
    position[1] == 0
        && sector()
            .floors
            .iter()
            .any(|r| (r[0]..=r[1]).contains(&position[0]) && (r[2]..=r[3]).contains(&position[2]))
}

/// Meridian crossing uses adjacent cells; diagonal moves cannot cut rail corners.
#[must_use]
pub fn valid_step(from: [i32; 3], to: [i32; 3]) -> bool {
    let dx = i64::from(from[0]) - i64::from(to[0]);
    let dz = i64::from(from[2]) - i64::from(to[2]);
    let navigable = |p: [i32; 3]| {
        walkable(p) || (p[1] == 0 && (-12..=12).contains(&p[0]) && (-12..=12).contains(&p[2]))
    };
    dx.abs() <= 1
        && dz.abs() <= 1
        && from[1] == 0
        && navigable(to)
        && navigable([to[0], 0, from[2]])
        && navigable([from[0], 0, to[2]])
}

#[derive(Clone)]
pub struct Expedition {
    survey: ScriptedActivity,
    recovery: ScriptedActivity,
    memory: bool,
}

impl Expedition {
    /// Restores authored objective state only. This produces no visit evidence
    /// or rewards; the caller must supply a verified campaign admission.
    ///
    /// # Errors
    /// Returns an error if the embedded field activities are invalid.
    pub fn restore(checkpoint: Checkpoint) -> Result<Self, ActivityError> {
        let mut run = Self::start()?;
        let completed: &[&str] = match checkpoint {
            Checkpoint::Entrance => &[],
            Checkpoint::Arrival => &["meridian_arrival"],
            Checkpoint::SurveyComplete => {
                &["meridian_arrival", "meridian_lens", "meridian_gallery"]
            }
        };
        for id in completed {
            // Use the authored transitions to initialize the snapshot, without
            // publishing their events as actions in the new session.
            run.survey.apply_trigger(&WorldTrigger::AreaReached {
                area_id: (*id).to_owned(),
            });
        }
        Ok(run)
    }

    /// # Errors
    /// Returns an error if either embedded field activity is invalid.
    pub fn start() -> Result<Self, ActivityError> {
        Ok(Self {
            survey: ScriptedActivity::from_source(include_str!(
                "../../../scripts/activities/meridian_survey.lua"
            ))?,
            recovery: ScriptedActivity::from_source(include_str!(
                "../../../scripts/activities/meridian_return.lua"
            ))?,
            memory: false,
        })
    }

    #[must_use]
    pub fn objectives(&self) -> Vec<ActivityEvent> {
        [&self.survey, &self.recovery]
            .into_iter()
            .flat_map(|script| {
                script
                    .objective_order
                    .iter()
                    .map(|id| ActivityEvent::ObjectiveUpdated(script.objectives[id].clone()))
            })
            .collect()
    }

    /// Resolves only active authored stations. Repeated visits produce no events.
    pub fn visit(&mut self, position: [i32; 3]) -> Option<(String, Vec<ActivityEvent>)> {
        let station = sector().stations.iter().find(|station| {
            station.position == position
                && (station.id == "meridian_memory" && !self.memory
                    || [&self.survey, &self.recovery].iter().any(|script| {
                        script
                            .objectives
                            .get(&station.id)
                            .is_some_and(|objective| objective.state == ObjectiveState::Active)
                    }))
        })?;
        let mut events = Vec::new();
        if station.id == "meridian_memory" {
            self.memory = true;
            events.push(ActivityEvent::ObjectiveUpdated(Objective {
                id: station.id.clone(),
                kind: ObjectiveKind::ReachArea,
                state: ObjectiveState::Completed,
                progress: 1,
                target: 1,
            }));
        } else {
            for script in [&mut self.survey, &mut self.recovery] {
                if script.objectives.contains_key(&station.id) {
                    events.extend(script.apply_trigger(&WorldTrigger::AreaReached {
                        area_id: station.id.clone(),
                    }));
                }
            }
        }
        events.retain(|event| matches!(event, ActivityEvent::ObjectiveUpdated(_)));
        Some((format!("{REVISION}:{}", station.id), events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};

    #[test]
    fn restored_boundaries_keep_completed_survey_and_restart_unconfirmed_work() {
        for (checkpoint, next_station, completed) in [
            (Checkpoint::Entrance, "meridian_arrival", 0),
            (Checkpoint::Arrival, "meridian_lens", 1),
            (Checkpoint::SurveyComplete, "meridian_log", 3),
        ] {
            let mut run = Expedition::restore(checkpoint).unwrap();
            assert!(walkable(checkpoint.position()));
            assert_eq!(
                run.objectives()
                    .iter()
                    .filter(|e| matches!(e,
                ActivityEvent::ObjectiveUpdated(o) if o.state == ObjectiveState::Completed))
                    .count(),
                completed
            );
            if checkpoint != Checkpoint::Entrance {
                assert!(run.visit(Checkpoint::Arrival.position()).is_none());
            }
            assert!(run.objectives().iter().any(|e| matches!(e,
                ActivityEvent::ObjectiveUpdated(o) if o.id == "meridian_return" && o.state == ObjectiveState::Pending)));
            let station = sector()
                .stations
                .iter()
                .find(|s| s.id == next_station)
                .unwrap();
            let (payload, events) = run.visit(station.position).unwrap();
            assert_eq!(payload, format!("{REVISION}:{next_station}"));
            assert!(events
                .iter()
                .all(|e| matches!(e, ActivityEvent::ObjectiveUpdated(_))));
        }
    }

    #[test]
    fn every_floor_cell_and_station_has_a_return_path_without_diagonal_corner_cutting() {
        assert_eq!(sector().revision, REVISION);
        let mut reached = BTreeSet::from([ENTRANCE]);
        let mut queue = VecDeque::from([ENTRANCE]);
        while let Some(p) = queue.pop_front() {
            for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                let next = [p[0] + dx, 0, p[2] + dz];
                if walkable(next) && valid_step(p, next) && reached.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        for x in -32..=-12 {
            for z in -10..=10 {
                assert_eq!(walkable([x, 0, z]), reached.contains(&[x, 0, z]));
            }
        }
        assert!(sector()
            .stations
            .iter()
            .all(|s| reached.contains(&s.position)));
        assert!(!valid_step([-26, 0, -2], [-25, 0, -1]));
        assert!(!valid_step(ENTRANCE, [-28, 0, -7]));
        assert!(!valid_step(ENTRANCE, [i32::MIN, 0, i32::MAX]));
    }

    #[test]
    fn independent_optional_activities_require_order_and_never_issue_rewards() {
        let mut run = Expedition::start().unwrap();
        assert!(run.visit([-28, 0, 7]).is_none());
        let mut steps = Vec::new();
        for p in [
            [-16, 0, 0],
            [-30, 0, 0],
            [-16, 0, 0],
            [-28, 0, -7],
            [-28, 0, 7],
            [-31, 0, 8],
        ] {
            let (payload, events) = run.visit(p).unwrap();
            steps.push(payload);
            assert!(events
                .iter()
                .all(|e| matches!(e, ActivityEvent::ObjectiveUpdated(_))));
            assert!(run.visit(p).is_none());
        }
        assert_eq!(steps.len(), 6);
        assert!(run.objectives().iter().all(|e| matches!(e, ActivityEvent::ObjectiveUpdated(o) if o.state == ObjectiveState::Completed)));
    }
}
