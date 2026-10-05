use revenant_objectives::{Objective, ObjectiveKind, ObjectiveState, WorldTrigger};

use crate::{ActivityError, ActivityEvent, ScriptedActivity};

pub const INTAKE: [i32; 3] = [-1, 0, -5];
pub const TRANSFER: [i32; 3] = [4, 0, -5];
pub const DELIVERY: [i32; 3] = [4, 0, -1];
pub const DWELL_MS: u64 = 1_200;
pub const DEADLINE_MS: u64 = 8_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Transfer,
    Delivery,
    Completed,
    Expired,
}

#[derive(Debug, Clone)]
pub struct CoolantRun {
    script: ScriptedActivity,
    phase: Phase,
    dwell_started: Option<u64>,
}

impl CoolantRun {
    /// Loads the bounded field activity. Its reward is paid by the enclosing
    /// Warden mission, so this state machine only produces objective events.
    ///
    /// # Errors
    ///
    /// Returns an error if embedded authored content fails validation.
    pub fn start() -> Result<(Self, Vec<ActivityEvent>), ActivityError> {
        let mut script = ScriptedActivity::from_source(include_str!(
            "../../../scripts/activities/coolant_run.lua"
        ))?;
        script.apply_trigger(&WorldTrigger::AreaReached {
            area_id: "coolant_intake".to_owned(),
        });
        let events = script
            .objective_order
            .iter()
            .map(|id| ActivityEvent::ObjectiveUpdated(script.objectives[id].clone()))
            .collect();
        Ok((
            Self {
                script,
                phase: Phase::Transfer,
                dwell_started: None,
            },
            events,
        ))
    }

    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    pub fn update(&mut self, position: [i32; 3], elapsed_ms: u64) -> Vec<ActivityEvent> {
        let (station, objective_id) = match self.phase {
            Phase::Transfer => (TRANSFER, "coolant_transfer"),
            Phase::Delivery => (DELIVERY, "coolant_delivery"),
            Phase::Completed | Phase::Expired => return Vec::new(),
        };
        // The deadline wins even when the last dwell would complete this tick.
        if elapsed_ms >= DEADLINE_MS {
            self.phase = Phase::Expired;
            return vec![ActivityEvent::ObjectiveUpdated(Objective {
                id: objective_id.to_owned(),
                kind: ObjectiveKind::ReachArea,
                state: ObjectiveState::Failed,
                progress: 0,
                target: 1,
            })];
        }
        if position != station {
            self.dwell_started = None;
            return Vec::new();
        }
        let started = *self.dwell_started.get_or_insert(elapsed_ms);
        if elapsed_ms.saturating_sub(started) < DWELL_MS {
            return Vec::new();
        }
        self.dwell_started = None;
        self.phase = if self.phase == Phase::Transfer {
            Phase::Delivery
        } else {
            Phase::Completed
        };
        self.script
            .apply_trigger(&WorldTrigger::AreaReached {
                area_id: objective_id.to_owned(),
            })
            .into_iter()
            .filter(|event| matches!(event, ActivityEvent::ObjectiveUpdated(_)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_authoring_is_explicit_and_does_not_relax_the_mission_contract() {
        let source = include_str!("../../../scripts/activities/coolant_run.lua");
        let manifest = crate::validate_activity_source(source).unwrap();
        assert_eq!(manifest.activity_kind, "field_excursion");
        assert!(!manifest.supports_compatibility_baseline);
        assert!(crate::validate_activity_source(
            &source.replace("    kind = \"field_excursion\",\n", "")
        )
        .is_err());
        assert!(crate::validate_activity_source(&source.replacen(
            "kind = \"ReachArea\"",
            "kind = \"KillActors\"",
            1
        ))
        .is_err());
    }

    #[test]
    fn stations_require_order_and_continuous_dwell_without_issuing_rewards() {
        let (mut run, _) = CoolantRun::start().unwrap();
        assert!(run.update(DELIVERY, 0).is_empty());
        assert!(run.update(TRANSFER, 100).is_empty());
        assert!(run.update(INTAKE, 900).is_empty());
        assert!(run.update(TRANSFER, 1_000).is_empty());
        assert!(run.update(TRANSFER, 1_300).is_empty());
        assert_eq!(run.update(TRANSFER, 2_200).len(), 2);
        assert_eq!(run.phase(), Phase::Delivery);
        assert!(run.update(DELIVERY, 3_000).is_empty());
        let events = run.update(DELIVERY, 4_200);
        assert_eq!(run.phase(), Phase::Completed);
        assert!(
            matches!(&events[..], [ActivityEvent::ObjectiveUpdated(objective)] if objective.state == ObjectiveState::Completed)
        );
        assert!(run.update(DELIVERY, 9_000).is_empty());
    }

    #[test]
    fn expiration_precedes_dwell_and_new_attempt_starts_clean() {
        let (mut run, _) = CoolantRun::start().unwrap();
        run.update(TRANSFER, 0);
        run.update(TRANSFER, DWELL_MS);
        run.update(DELIVERY, DEADLINE_MS - DWELL_MS);
        let events = run.update(DELIVERY, DEADLINE_MS);
        assert_eq!(run.phase(), Phase::Expired);
        assert!(
            matches!(&events[..], [ActivityEvent::ObjectiveUpdated(objective)] if objective.state == ObjectiveState::Failed)
        );
        let (retry, _) = CoolantRun::start().unwrap();
        assert_eq!(retry.phase(), Phase::Transfer);
    }
}
