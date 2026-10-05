use std::collections::HashMap;
use std::path::Path;

mod authoring;
pub mod coolant;
pub mod meridian;

pub use authoring::{
    validate_activity_file, validate_activity_source, ActivityAuthoringManifest, ActivityError,
    AuthoringLimits, AuthoringObjective, AuthoringRoute, AuthoringRouteEvent, AuthoringTrigger,
    MAX_ACTIVITY_LUA_BYTES, MAX_ACTIVITY_LUA_INSTRUCTIONS, MAX_ACTIVITY_LUA_MEMORY_BYTES,
};

use authoring::parse_activity_source;
use authoring::{parse_activity_file, ParsedActivity, TriggerDefinition};
use revenant_inventory::Reward;
use revenant_objectives::{Objective, ObjectiveState, WorldTrigger};
use revenant_progression::ExperienceReward;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityEvent {
    Started { activity_id: String },
    ObjectiveUpdated(Objective),
    DoorOpened { door_id: String },
    BossRequested { archetype: String },
    Completed { activity_id: String },
}

#[derive(Debug, Clone)]
pub struct ScriptedActivity {
    id: String,
    reward: Reward,
    experience_reward: ExperienceReward,
    objectives: HashMap<String, Objective>,
    objective_order: Vec<String>,
    triggers: Vec<TriggerDefinition>,
    manifest: ActivityAuthoringManifest,
}

impl ScriptedActivity {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Loads an activity definition from a Lua file using restricted standard libraries.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or its Lua table does not match
    /// the activity schema.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ActivityError> {
        parse_activity_file(path.as_ref()).map(Self::from_parsed)
    }

    #[cfg(test)]
    fn from_lua(source: &str) -> Result<Self, ActivityError> {
        Self::from_source(source)
    }

    /// Loads embedded authored content with the same limits as a Lua file.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid, unbounded, or non-declarative content.
    pub fn from_source(source: &str) -> Result<Self, ActivityError> {
        parse_activity_source(source).map(Self::from_parsed)
    }

    fn from_parsed(parsed: ParsedActivity) -> Self {
        Self {
            id: parsed.id,
            reward: parsed.reward,
            experience_reward: parsed.experience_reward,
            objectives: parsed.objectives,
            objective_order: parsed.objective_order,
            triggers: parsed.triggers,
            manifest: parsed.manifest,
        }
    }

    #[must_use]
    pub fn reward(&self) -> &Reward {
        &self.reward
    }

    #[must_use]
    pub fn experience_reward(&self) -> ExperienceReward {
        self.experience_reward
    }

    #[must_use]
    pub const fn authoring_manifest(&self) -> &ActivityAuthoringManifest {
        &self.manifest
    }

    #[must_use]
    pub fn start(&self) -> Vec<ActivityEvent> {
        let mut events = vec![ActivityEvent::Started {
            activity_id: self.id.clone(),
        }];
        events.extend(self.objective_order.iter().filter_map(|id| {
            let objective = self.objectives.get(id)?;
            (objective.state == ObjectiveState::Active)
                .then(|| ActivityEvent::ObjectiveUpdated(objective.clone()))
        }));
        events
    }

    /// Projects every accepted objective when resuming an authored chapter.
    #[must_use]
    pub fn objective_snapshot(&self) -> Vec<ActivityEvent> {
        self.objective_order
            .iter()
            .filter_map(|id| self.objectives.get(id))
            .map(|objective| ActivityEvent::ObjectiveUpdated(objective.clone()))
            .collect()
    }

    pub fn apply_trigger(&mut self, trigger: &WorldTrigger) -> Vec<ActivityEvent> {
        let (event, subject) = match trigger {
            WorldTrigger::ActorGroupDead { group_id } => ("ActorGroupDead", group_id.as_str()),
            WorldTrigger::AreaReached { area_id } => ("AreaReached", area_id.as_str()),
        };
        let Some(definition) = self
            .triggers
            .iter()
            .find(|definition| definition.event == event && definition.subject == subject)
            .cloned()
        else {
            return Vec::new();
        };
        let mut events = Vec::new();
        for id in definition.complete {
            if let Some(objective) = self.objectives.get_mut(&id) {
                objective.progress = objective.target;
                objective.state = ObjectiveState::Completed;
                events.push(ActivityEvent::ObjectiveUpdated(objective.clone()));
            }
        }
        for id in definition.activate {
            if let Some(objective) = self.objectives.get_mut(&id) {
                objective.state = ObjectiveState::Active;
                events.push(ActivityEvent::ObjectiveUpdated(objective.clone()));
            }
        }
        if let Some(door_id) = definition.open_door {
            events.push(ActivityEvent::DoorOpened { door_id });
        }
        if let Some(archetype) = definition.spawn_boss {
            events.push(ActivityEvent::BossRequested { archetype });
        }
        if definition.complete_activity {
            events.push(ActivityEvent::Completed {
                activity_id: self.id.clone(),
            });
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use revenant_objectives::WorldTrigger;

    use super::{ActivityEvent, ScriptedActivity};

    const SCRIPT: &str = include_str!("../../../scripts/activities/relay_awakening.lua");
    const GOLDEN: &str = include_str!("../../../tests/fixtures/m27/relay_awakening_m26.lua");

    #[test]
    fn campaign_prism_boss_and_shutdown_do_not_emit_completion() {
        let mut activity = ScriptedActivity::from_source(include_str!(
            "../../../scripts/activities/campaign_prism.lua"
        ))
        .unwrap();
        activity.apply_trigger(&WorldTrigger::AreaReached {
            area_id: "prism_arrival".to_owned(),
        });
        let boss = activity.apply_trigger(&WorldTrigger::ActorGroupDead {
            group_id: "prism_guard".to_owned(),
        });
        assert!(boss
            .iter()
            .all(|e| matches!(e, ActivityEvent::ObjectiveUpdated(_))));
        assert!(activity.objective_snapshot().iter().any(|e| matches!(e,
            ActivityEvent::ObjectiveUpdated(o) if o.id == "prism_shutdown" && o.state == super::ObjectiveState::Active
        )));
        let shutdown = activity.apply_trigger(&WorldTrigger::AreaReached {
            area_id: "prism_shutdown".to_owned(),
        });
        assert!(shutdown
            .iter()
            .all(|e| matches!(e, ActivityEvent::ObjectiveUpdated(_))));
        let returned = activity.apply_trigger(&WorldTrigger::AreaReached {
            area_id: "prism_return".to_owned(),
        });
        assert_eq!(
            returned
                .iter()
                .filter(|e| matches!(e, ActivityEvent::Completed { .. }))
                .count(),
            1
        );
        assert_eq!(activity.reward().quantity, 1);
        assert_eq!(activity.experience_reward().experience(), 100);
    }

    #[test]
    fn lua_activity_opens_door_spawns_boss_and_completes() {
        let mut activity = ScriptedActivity::from_lua(SCRIPT).expect("script should load");
        activity.apply_trigger(&WorldTrigger::ActorGroupDead {
            group_id: "relay_drones".to_owned(),
        });
        let door = activity.apply_trigger(&WorldTrigger::AreaReached {
            area_id: "relay_door".to_owned(),
        });
        assert!(door
            .iter()
            .any(|event| matches!(event, ActivityEvent::DoorOpened { .. })));
        assert!(door
            .iter()
            .any(|event| matches!(event, ActivityEvent::BossRequested { .. })));
        let boss = activity.apply_trigger(&WorldTrigger::ActorGroupDead {
            group_id: "warden".to_owned(),
        });
        assert!(boss
            .iter()
            .any(|event| matches!(event, ActivityEvent::Completed { .. })));
        assert_eq!(activity.reward().item_id, "relay_core_fragment");
        assert_eq!(activity.reward().quantity, 1);
        assert_eq!(activity.experience_reward().experience(), 100);
    }

    #[test]
    fn extended_authoring_preserves_exact_m26_runtime_projection() {
        let mut current = ScriptedActivity::from_lua(SCRIPT).expect("current script should load");
        let mut golden = ScriptedActivity::from_lua(GOLDEN).expect("golden script should load");
        assert_eq!(current.start(), golden.start());
        for trigger in [
            WorldTrigger::ActorGroupDead {
                group_id: "relay_drones".to_owned(),
            },
            WorldTrigger::AreaReached {
                area_id: "relay_door".to_owned(),
            },
            WorldTrigger::ActorGroupDead {
                group_id: "warden".to_owned(),
            },
        ] {
            assert_eq!(
                current.apply_trigger(&trigger),
                golden.apply_trigger(&trigger)
            );
        }
        assert_eq!(current.reward(), golden.reward());
        assert_eq!(current.experience_reward(), golden.experience_reward());
        assert_eq!(current.authoring_manifest().routes.len(), 2);
        assert!(golden.authoring_manifest().routes.is_empty());
    }
}
