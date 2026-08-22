use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::Path;

use mlua::{Lua, LuaOptions, StdLib, Table};
use revenant_inventory::Reward;
use revenant_objectives::{Objective, ObjectiveKind, ObjectiveState, WorldTrigger};
use revenant_progression::ExperienceReward;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityEvent {
    Started { activity_id: String },
    ObjectiveUpdated(Objective),
    DoorOpened { door_id: String },
    BossRequested { archetype: String },
    Completed { activity_id: String },
}

#[derive(Debug)]
pub struct ScriptedActivity {
    id: String,
    reward: Reward,
    experience_reward: ExperienceReward,
    objectives: HashMap<String, Objective>,
    objective_order: Vec<String>,
    triggers: Vec<TriggerDefinition>,
}

#[derive(Debug, Clone)]
struct TriggerDefinition {
    event: String,
    subject: String,
    complete: Vec<String>,
    activate: Vec<String>,
    open_door: Option<String>,
    spawn_boss: Option<String>,
    complete_activity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityError(String);

impl Display for ActivityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ActivityError {}

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
        let source = fs::read_to_string(path.as_ref())
            .map_err(|error| ActivityError(format!("activity script read failed: {error}")))?;
        Self::from_lua(&source)
    }

    fn from_lua(source: &str) -> Result<Self, ActivityError> {
        let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING, LuaOptions::default())
            .map_err(activity_error)?;
        let definition: Table = lua.load(source).eval().map_err(activity_error)?;
        let id = definition.get("id").map_err(activity_error)?;
        let reward_table: Table = definition.get("reward").map_err(activity_error)?;
        let reward = Reward::validated(
            reward_table
                .get::<String>("item_id")
                .map_err(activity_error)?,
            reward_table
                .get::<u32>("quantity")
                .map_err(activity_error)?,
        )
        .map_err(activity_error)?;
        let progression_table: Table = definition.get("progression").map_err(activity_error)?;
        let experience_reward = ExperienceReward::validated(
            progression_table
                .get::<u64>("experience")
                .map_err(activity_error)?,
        )
        .map_err(activity_error)?;
        let objective_tables: Table = definition.get("objectives").map_err(activity_error)?;
        let mut objectives = HashMap::new();
        let mut objective_order = Vec::new();
        for table in objective_tables.sequence_values::<Table>() {
            let objective = parse_objective(&table.map_err(activity_error)?)?;
            objective_order.push(objective.id.clone());
            objectives.insert(objective.id.clone(), objective);
        }
        let trigger_tables: Table = definition.get("triggers").map_err(activity_error)?;
        let triggers = trigger_tables
            .sequence_values::<Table>()
            .map(|table| parse_trigger(&table.map_err(activity_error)?))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            id,
            reward,
            experience_reward,
            objectives,
            objective_order,
            triggers,
        })
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

fn parse_objective(table: &Table) -> Result<Objective, ActivityError> {
    let kind = match table
        .get::<String>("kind")
        .map_err(activity_error)?
        .as_str()
    {
        "KillActors" => ObjectiveKind::KillActors,
        "ReachArea" => ObjectiveKind::ReachArea,
        "Boss" => ObjectiveKind::Boss,
        value => return Err(ActivityError(format!("unknown objective kind: {value}"))),
    };
    let state = match table
        .get::<String>("state")
        .map_err(activity_error)?
        .as_str()
    {
        "Pending" => ObjectiveState::Pending,
        "Active" => ObjectiveState::Active,
        value => {
            return Err(ActivityError(format!(
                "invalid initial objective state: {value}"
            )))
        }
    };
    Ok(Objective {
        id: table.get("id").map_err(activity_error)?,
        kind,
        state,
        progress: 0,
        target: table.get("target").map_err(activity_error)?,
    })
}

fn parse_trigger(table: &Table) -> Result<TriggerDefinition, ActivityError> {
    Ok(TriggerDefinition {
        event: table.get("event").map_err(activity_error)?,
        subject: table.get("subject").map_err(activity_error)?,
        complete: string_sequence(table, "complete")?,
        activate: string_sequence(table, "activate")?,
        open_door: table.get("open_door").map_err(activity_error)?,
        spawn_boss: table.get("spawn_boss").map_err(activity_error)?,
        complete_activity: table
            .get::<Option<bool>>("complete_activity")
            .map_err(activity_error)?
            .unwrap_or(false),
    })
}

fn string_sequence(table: &Table, key: &str) -> Result<Vec<String>, ActivityError> {
    let Some(values) = table.get::<Option<Table>>(key).map_err(activity_error)? else {
        return Ok(Vec::new());
    };
    values
        .sequence_values::<String>()
        .map(|value| value.map_err(activity_error))
        .collect()
}

fn activity_error(error: impl Display) -> ActivityError {
    ActivityError(error.to_string())
}

#[cfg(test)]
mod tests {
    use revenant_objectives::WorldTrigger;

    use super::{ActivityEvent, ScriptedActivity};

    const SCRIPT: &str = include_str!("../../../scripts/activities/relay_awakening.lua");

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
}
