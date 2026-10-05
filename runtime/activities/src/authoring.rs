use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use mlua::{
    ChunkMode, Error as LuaError, FromLua, HookTriggers, Lua, LuaOptions, StdLib, Table, Value,
    VmState,
};
use revenant_inventory::{Reward, RELAY_CORE_FRAGMENT};
use revenant_objectives::{Objective, ObjectiveKind, ObjectiveState};
use revenant_operations::{
    event_definition, route_definition, RouteEventId, RouteId, RouteObjectiveId, CATALOG_REVISION,
    MAX_ROUTE_OBJECTIVES,
};
use revenant_progression::ExperienceReward;
use serde::Serialize;

pub const MAX_ACTIVITY_LUA_BYTES: usize = 64 * 1024;
pub const MAX_ACTIVITY_LUA_MEMORY_BYTES: usize = 1024 * 1024;
pub const MAX_ACTIVITY_LUA_INSTRUCTIONS: u32 = 100_000;

const MAX_ACTIVITY_ID_BYTES: usize = 32;
const MAX_AUTHORING_STRING_BYTES: usize = 128;
const MAX_AUTHORING_TABLES: usize = 64;
const MAX_AUTHORING_VALUES: usize = 256;
const MAX_AUTHORING_DEPTH: usize = 8;
const MAX_OBJECTIVES: usize = 8;
const MAX_TRIGGERS: usize = 12;
const MAX_ROUTES: usize = 2;
const MAX_EVENTS_PER_ROUTE: usize = 2;
const MAX_OBJECTIVE_TARGET: u32 = 100;
const INSTRUCTION_LIMIT_MESSAGE: &str = "activity Lua instruction limit exceeded";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AuthoringLimits {
    pub source_bytes: usize,
    pub memory_bytes: usize,
    pub instructions: u32,
    pub string_bytes: usize,
    pub tables: usize,
    pub values: usize,
    pub objectives: usize,
    pub triggers: usize,
    pub routes: usize,
    pub events_per_route: usize,
    pub objectives_per_route: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivityAuthoringManifest {
    pub schema: &'static str,
    pub activity_id: String,
    pub activity_kind: &'static str,
    pub authoring_revision: Option<String>,
    pub supports_compatibility_baseline: bool,
    pub source_size_bytes: usize,
    pub reward_item_id: String,
    pub reward_quantity: u32,
    pub experience: u64,
    pub objectives: Vec<AuthoringObjective>,
    pub triggers: Vec<AuthoringTrigger>,
    pub routes: Vec<AuthoringRoute>,
    pub limits: AuthoringLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringObjective {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringTrigger {
    pub event: String,
    pub subject: String,
    pub complete: Vec<String>,
    pub activate: Vec<String>,
    pub open_door: Option<String>,
    pub spawn_boss: Option<String>,
    pub complete_activity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringRoute {
    pub route_id: RouteId,
    pub duration_ms: u64,
    pub reward_item_id: String,
    pub reward_quantity: u32,
    pub experience: u64,
    pub objective_path: Vec<RouteObjectiveId>,
    pub events: Vec<AuthoringRouteEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AuthoringRouteEvent {
    pub event_id: RouteEventId,
    pub warden_health_basis_points: u32,
    pub warden_counter_damage: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityError(String);

impl ActivityError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl Display for ActivityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ActivityError {}

#[derive(Debug)]
pub(crate) struct ParsedActivity {
    pub(crate) id: String,
    pub(crate) reward: Reward,
    pub(crate) experience_reward: ExperienceReward,
    pub(crate) objectives: HashMap<String, Objective>,
    pub(crate) objective_order: Vec<String>,
    pub(crate) triggers: Vec<TriggerDefinition>,
    pub(crate) manifest: ActivityAuthoringManifest,
}

#[derive(Debug, Clone)]
pub(crate) struct TriggerDefinition {
    pub(crate) event: String,
    pub(crate) subject: String,
    pub(crate) complete: Vec<String>,
    pub(crate) activate: Vec<String>,
    pub(crate) open_door: Option<String>,
    pub(crate) spawn_boss: Option<String>,
    pub(crate) complete_activity: bool,
}

type ParsedObjectives = (
    HashMap<String, Objective>,
    Vec<String>,
    Vec<AuthoringObjective>,
);
type ParsedTrigger = (TriggerDefinition, AuthoringTrigger);

/// Validates a bounded activity file and returns its stable authoring manifest.
///
/// # Errors
///
/// Returns an error for file I/O, invalid UTF-8, resource-limit violation,
/// executable/static-graph violation, or schema/graph/catalog mismatch.
pub fn validate_activity_file(
    path: impl AsRef<Path>,
) -> Result<ActivityAuthoringManifest, ActivityError> {
    parse_activity_file(path.as_ref()).map(|parsed| parsed.manifest)
}

/// Validates bounded Lua source and returns its stable authoring manifest.
///
/// # Errors
///
/// Returns an error for source/resource-limit, static-graph, schema, objective
/// graph, or exact route-catalog violations.
pub fn validate_activity_source(source: &str) -> Result<ActivityAuthoringManifest, ActivityError> {
    parse_activity_source(source).map(|parsed| parsed.manifest)
}

pub(crate) fn parse_activity_file(path: &Path) -> Result<ParsedActivity, ActivityError> {
    let source = read_bounded_source(path)?;
    parse_activity_source(&source)
}

pub(crate) fn parse_activity_source(source: &str) -> Result<ParsedActivity, ActivityError> {
    if source.len() > MAX_ACTIVITY_LUA_BYTES {
        return Err(ActivityError::new(format!(
            "activity Lua source is {} bytes; maximum is {MAX_ACTIVITY_LUA_BYTES}",
            source.len()
        )));
    }
    let lua = restricted_lua()?;
    let value = lua
        .load(source)
        .set_name("activity")
        .set_mode(ChunkMode::Text)
        .eval::<Value>()
        .map_err(lua_evaluation_error)?;
    validate_static_graph(&value)?;
    let Value::Table(definition) = value else {
        return Err(ActivityError::new(
            "activity Lua must return one declarative table",
        ));
    };
    parse_definition(&definition, source.len())
}

fn read_bounded_source(path: &Path) -> Result<String, ActivityError> {
    let file = File::open(path)
        .map_err(|error| ActivityError::new(format!("activity script open failed: {error}")))?;
    let mut bytes = Vec::with_capacity(MAX_ACTIVITY_LUA_BYTES.min(8 * 1024));
    file.take(u64::try_from(MAX_ACTIVITY_LUA_BYTES + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| ActivityError::new(format!("activity script read failed: {error}")))?;
    if bytes.len() > MAX_ACTIVITY_LUA_BYTES {
        return Err(ActivityError::new(format!(
            "activity Lua source exceeds {MAX_ACTIVITY_LUA_BYTES} bytes"
        )));
    }
    String::from_utf8(bytes)
        .map_err(|_| ActivityError::new("activity Lua source must be valid UTF-8"))
}

fn restricted_lua() -> Result<Lua, ActivityError> {
    let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING, LuaOptions::default())
        .map_err(|error| ActivityError::new(format!("activity Lua setup failed: {error}")))?;
    if lua.used_memory() >= MAX_ACTIVITY_LUA_MEMORY_BYTES {
        return Err(ActivityError::new(
            "activity Lua initial state exceeds memory limit",
        ));
    }
    lua.set_memory_limit(MAX_ACTIVITY_LUA_MEMORY_BYTES)
        .map_err(|error| {
            ActivityError::new(format!("activity Lua memory setup failed: {error}"))
        })?;
    let remaining = Arc::new(AtomicU32::new(MAX_ACTIVITY_LUA_INSTRUCTIONS));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(1),
        move |_lua, _debug| {
            if remaining
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_sub(1)
                })
                .is_err()
            {
                return Err(LuaError::runtime(INSTRUCTION_LIMIT_MESSAGE));
            }
            Ok(VmState::Continue)
        },
    )
    .map_err(|error| ActivityError::new(format!("activity Lua hook setup failed: {error}")))?;
    Ok(lua)
}

fn lua_evaluation_error(error: LuaError) -> ActivityError {
    match error {
        LuaError::MemoryError(_) => ActivityError::new("activity Lua memory limit exceeded"),
        other if other.to_string().contains(INSTRUCTION_LIMIT_MESSAGE) => {
            ActivityError::new(INSTRUCTION_LIMIT_MESSAGE)
        }
        other => ActivityError::new(format!("activity Lua evaluation failed: {other}")),
    }
}

struct StaticGraphState {
    tables: HashSet<usize>,
    values: usize,
}

fn validate_static_graph(value: &Value) -> Result<(), ActivityError> {
    let mut state = StaticGraphState {
        tables: HashSet::new(),
        values: 0,
    };
    visit_static_value(value, 0, &mut state)
}

fn visit_static_value(
    value: &Value,
    depth: usize,
    state: &mut StaticGraphState,
) -> Result<(), ActivityError> {
    state.values = state
        .values
        .checked_add(1)
        .ok_or_else(|| ActivityError::new("activity static value count overflowed"))?;
    if state.values > MAX_AUTHORING_VALUES {
        return Err(ActivityError::new(format!(
            "activity static graph exceeds {MAX_AUTHORING_VALUES} values"
        )));
    }
    match value {
        Value::Nil | Value::Boolean(_) | Value::Integer(_) => Ok(()),
        Value::String(value) => validate_lua_string(&value.as_bytes()),
        Value::Table(table) => {
            if depth >= MAX_AUTHORING_DEPTH {
                return Err(ActivityError::new(
                    "activity static graph is nested too deeply",
                ));
            }
            if table.has_metatable() {
                return Err(ActivityError::new("activity tables cannot have metatables"));
            }
            let identity = table.to_pointer() as usize;
            if !state.tables.insert(identity) {
                return Err(ActivityError::new(
                    "activity tables must be acyclic and cannot be aliased",
                ));
            }
            if state.tables.len() > MAX_AUTHORING_TABLES {
                return Err(ActivityError::new(format!(
                    "activity static graph exceeds {MAX_AUTHORING_TABLES} tables"
                )));
            }
            for pair in table.clone().pairs::<Value, Value>() {
                let (key, nested) = pair.map_err(activity_schema_error)?;
                match &key {
                    Value::Integer(_) => {}
                    Value::String(value) => validate_lua_string(&value.as_bytes())?,
                    other => {
                        return Err(ActivityError::new(format!(
                            "activity table key type {} is not declarative",
                            other.type_name()
                        )))
                    }
                }
                visit_static_value(&nested, depth + 1, state)?;
            }
            Ok(())
        }
        other => Err(ActivityError::new(format!(
            "activity value type {} is not declarative",
            other.type_name()
        ))),
    }
}

fn validate_lua_string(value: &[u8]) -> Result<(), ActivityError> {
    if value.len() > MAX_AUTHORING_STRING_BYTES {
        return Err(ActivityError::new(format!(
            "activity string exceeds {MAX_AUTHORING_STRING_BYTES} bytes"
        )));
    }
    std::str::from_utf8(value)
        .map(|_| ())
        .map_err(|_| ActivityError::new("activity strings must be valid UTF-8"))
}

fn parse_definition(
    definition: &Table,
    source_size: usize,
) -> Result<ParsedActivity, ActivityError> {
    validate_map_keys(
        definition,
        &[
            "id",
            "authoring_revision",
            "kind",
            "reward",
            "progression",
            "objectives",
            "triggers",
            "routes",
        ],
        "activity",
    )?;
    let id: String = required_field(definition, "id", "activity")?;
    validate_identifier(&id, "activity id")?;
    let activity_kind = match optional_field::<String>(definition, "kind", "activity")?.as_deref() {
        None => "mission",
        Some("field_excursion") => "field_excursion",
        Some("campaign_chapter") => "campaign_chapter",
        Some(_) => return Err(ActivityError::new("unknown activity kind")),
    };
    let authoring_revision: Option<String> =
        optional_field(definition, "authoring_revision", "activity")?;
    let reward_table = required_table(definition, "reward", "activity")?;
    let (reward, reward_item_id, reward_quantity) = parse_reward(&reward_table, "activity reward")?;
    let progression_table = required_table(definition, "progression", "activity")?;
    let experience_reward = parse_progression(&progression_table, "activity progression")?;
    let objective_tables = required_table(definition, "objectives", "activity")?;
    let (objectives, objective_order, objective_manifest) = parse_objectives(&objective_tables)?;
    let trigger_tables = required_table(definition, "triggers", "activity")?;
    let (triggers, trigger_manifest) = parse_triggers(&trigger_tables, &objectives)?;
    let routes = parse_routes(definition, authoring_revision.as_deref(), &objectives)?;
    validate_objective_graph(&objectives, &triggers, &routes, activity_kind)?;
    let manifest = ActivityAuthoringManifest {
        schema: "RevenantActivityAuthoringV1",
        activity_id: id.clone(),
        activity_kind,
        authoring_revision,
        supports_compatibility_baseline: activity_kind == "mission",
        source_size_bytes: source_size,
        reward_item_id,
        reward_quantity,
        experience: experience_reward.experience(),
        objectives: objective_manifest,
        triggers: trigger_manifest,
        routes,
        limits: authoring_limits(),
    };
    Ok(ParsedActivity {
        id,
        reward,
        experience_reward,
        objectives,
        objective_order,
        triggers,
        manifest,
    })
}

fn parse_reward(table: &Table, context: &str) -> Result<(Reward, String, u32), ActivityError> {
    validate_map_keys(table, &["item_id", "quantity"], context)?;
    let item_id: String = required_field(table, "item_id", context)?;
    validate_identifier(&item_id, "reward item id")?;
    let quantity: u32 = required_field(table, "quantity", context)?;
    let reward = Reward::validated(item_id.clone(), quantity).map_err(activity_schema_error)?;
    Ok((reward, item_id, quantity))
}

fn parse_progression(table: &Table, context: &str) -> Result<ExperienceReward, ActivityError> {
    validate_map_keys(table, &["experience"], context)?;
    ExperienceReward::validated(required_field(table, "experience", context)?)
        .map_err(activity_schema_error)
}

fn parse_objectives(tables: &Table) -> Result<ParsedObjectives, ActivityError> {
    validate_sequence_shape(tables, MAX_OBJECTIVES, "objectives")?;
    if tables.raw_len() == 0 {
        return Err(ActivityError::new(
            "activity requires at least one objective",
        ));
    }
    let mut objectives = HashMap::new();
    let mut order = Vec::with_capacity(tables.raw_len());
    let mut manifest = Vec::with_capacity(tables.raw_len());
    let mut active_count = 0;
    for (index, value) in tables.clone().sequence_values::<Table>().enumerate() {
        let table = value.map_err(activity_schema_error)?;
        let context = format!("objective {}", index + 1);
        validate_map_keys(&table, &["id", "kind", "state", "target"], &context)?;
        let id: String = required_field(&table, "id", &context)?;
        validate_identifier(&id, "objective id")?;
        let kind_text: String = required_field(&table, "kind", &context)?;
        let kind = match kind_text.as_str() {
            "KillActors" => ObjectiveKind::KillActors,
            "ReachArea" => ObjectiveKind::ReachArea,
            "Boss" => ObjectiveKind::Boss,
            value => {
                return Err(ActivityError::new(format!(
                    "unknown objective kind: {value}"
                )))
            }
        };
        let state_text: String = required_field(&table, "state", &context)?;
        let state = match state_text.as_str() {
            "Pending" => ObjectiveState::Pending,
            "Active" => {
                active_count += 1;
                ObjectiveState::Active
            }
            value => {
                return Err(ActivityError::new(format!(
                    "invalid initial objective state: {value}"
                )))
            }
        };
        let target: u32 = required_field(&table, "target", &context)?;
        if !(1..=MAX_OBJECTIVE_TARGET).contains(&target) {
            return Err(ActivityError::new(format!(
                "objective target must be 1-{MAX_OBJECTIVE_TARGET}"
            )));
        }
        if objectives.contains_key(&id) {
            return Err(ActivityError::new(format!("duplicate objective id: {id}")));
        }
        order.push(id.clone());
        manifest.push(AuthoringObjective {
            id: id.clone(),
            kind: kind_text,
            state: state_text,
            target,
        });
        objectives.insert(
            id.clone(),
            Objective {
                id,
                kind,
                state,
                progress: 0,
                target,
            },
        );
    }
    if active_count != 1 {
        return Err(ActivityError::new(
            "activity must contain exactly one initially active objective",
        ));
    }
    Ok((objectives, order, manifest))
}

fn parse_triggers(
    tables: &Table,
    objectives: &HashMap<String, Objective>,
) -> Result<(Vec<TriggerDefinition>, Vec<AuthoringTrigger>), ActivityError> {
    validate_sequence_shape(tables, MAX_TRIGGERS, "triggers")?;
    if tables.raw_len() == 0 {
        return Err(ActivityError::new("activity requires at least one trigger"));
    }
    let mut triggers = Vec::with_capacity(tables.raw_len());
    let mut manifest = Vec::with_capacity(tables.raw_len());
    let mut identities = BTreeSet::new();
    let mut terminal_count = 0;
    for (index, value) in tables.clone().sequence_values::<Table>().enumerate() {
        let table = value.map_err(activity_schema_error)?;
        let (trigger, authoring_trigger) = parse_trigger(&table, index, objectives)?;
        if !identities.insert((trigger.event.clone(), trigger.subject.clone())) {
            return Err(ActivityError::new(format!(
                "duplicate trigger identity: {}/{}",
                trigger.event, trigger.subject
            )));
        }
        terminal_count += usize::from(trigger.complete_activity);
        triggers.push(trigger);
        manifest.push(authoring_trigger);
    }
    if terminal_count != 1 {
        return Err(ActivityError::new(
            "activity must contain exactly one terminal trigger",
        ));
    }
    Ok((triggers, manifest))
}

fn parse_trigger(
    table: &Table,
    index: usize,
    objectives: &HashMap<String, Objective>,
) -> Result<ParsedTrigger, ActivityError> {
    let context = format!("trigger {}", index + 1);
    validate_map_keys(
        table,
        &[
            "event",
            "subject",
            "complete",
            "activate",
            "open_door",
            "spawn_boss",
            "complete_activity",
        ],
        &context,
    )?;
    let event: String = required_field(table, "event", &context)?;
    if !matches!(event.as_str(), "ActorGroupDead" | "AreaReached") {
        return Err(ActivityError::new(format!(
            "unknown trigger event: {event}"
        )));
    }
    let subject: String = required_field(table, "subject", &context)?;
    validate_identifier(&subject, "trigger subject")?;
    let complete = optional_string_sequence(table, "complete", MAX_OBJECTIVES, &context)?;
    let activate = optional_string_sequence(table, "activate", MAX_OBJECTIVES, &context)?;
    validate_unique_references(&complete, "trigger complete")?;
    validate_unique_references(&activate, "trigger activate")?;
    for objective_id in complete.iter().chain(&activate) {
        if !objectives.contains_key(objective_id) {
            return Err(ActivityError::new(format!(
                "trigger references unknown objective: {objective_id}"
            )));
        }
    }
    if complete.iter().any(|id| activate.contains(id)) {
        return Err(ActivityError::new(
            "trigger cannot complete and activate the same objective",
        ));
    }
    let open_door: Option<String> = optional_field(table, "open_door", &context)?;
    let spawn_boss: Option<String> = optional_field(table, "spawn_boss", &context)?;
    if let Some(id) = &open_door {
        validate_identifier(id, "door id")?;
    }
    if let Some(id) = &spawn_boss {
        validate_identifier(id, "boss archetype")?;
    }
    let complete_activity =
        optional_field::<bool>(table, "complete_activity", &context)?.unwrap_or(false);
    if complete_activity && (!activate.is_empty() || open_door.is_some() || spawn_boss.is_some()) {
        return Err(ActivityError::new(
            "terminal trigger cannot activate or spawn later state",
        ));
    }
    if complete.is_empty()
        && activate.is_empty()
        && open_door.is_none()
        && spawn_boss.is_none()
        && !complete_activity
    {
        return Err(ActivityError::new("trigger must have one bounded effect"));
    }
    let trigger = TriggerDefinition {
        event: event.clone(),
        subject: subject.clone(),
        complete: complete.clone(),
        activate: activate.clone(),
        open_door: open_door.clone(),
        spawn_boss: spawn_boss.clone(),
        complete_activity,
    };
    let authoring_trigger = AuthoringTrigger {
        event,
        subject,
        complete,
        activate,
        open_door,
        spawn_boss,
        complete_activity,
    };
    Ok((trigger, authoring_trigger))
}

fn parse_routes(
    definition: &Table,
    authoring_revision: Option<&str>,
    objectives: &HashMap<String, Objective>,
) -> Result<Vec<AuthoringRoute>, ActivityError> {
    let route_tables = optional_field::<Table>(definition, "routes", "activity")?;
    match (authoring_revision, route_tables) {
        (None, None) => Ok(Vec::new()),
        (None, Some(_)) => Err(ActivityError::new(
            "routed activity requires an authoring revision",
        )),
        (Some(_), None) => Err(ActivityError::new(
            "authoring revision requires exactly two routes",
        )),
        (Some(revision), Some(tables)) => {
            if revision != CATALOG_REVISION {
                return Err(ActivityError::new(format!(
                    "unknown activity authoring revision: {revision}"
                )));
            }
            validate_sequence_shape(&tables, MAX_ROUTES, "routes")?;
            if tables.raw_len() != MAX_ROUTES {
                return Err(ActivityError::new(
                    "activity must contain exactly two routes",
                ));
            }
            parse_exact_routes(&tables, objectives)
        }
    }
}

fn parse_exact_routes(
    tables: &Table,
    objectives: &HashMap<String, Objective>,
) -> Result<Vec<AuthoringRoute>, ActivityError> {
    let mut routes = Vec::with_capacity(MAX_ROUTES);
    for (index, value) in tables.clone().sequence_values::<Table>().enumerate() {
        let table = value.map_err(activity_schema_error)?;
        let context = format!("route {}", index + 1);
        validate_map_keys(
            &table,
            &[
                "id",
                "duration_ms",
                "reward",
                "progression",
                "objective_path",
                "events",
            ],
            &context,
        )?;
        let route_text: String = required_field(&table, "id", &context)?;
        let route_id = RouteId::from_str(&route_text).map_err(activity_schema_error)?;
        if route_id != RouteId::ALL[index] {
            return Err(ActivityError::new(
                "routes must use canonical catalog order",
            ));
        }
        let expected = route_definition(route_id);
        let duration_ms: u64 = required_field(&table, "duration_ms", &context)?;
        if duration_ms != expected.duration_ms {
            return Err(ActivityError::new(format!(
                "route {route_id} duration does not match {CATALOG_REVISION}"
            )));
        }
        let reward_table = required_table(&table, "reward", &context)?;
        let (_, reward_item_id, reward_quantity) = parse_reward(&reward_table, "route reward")?;
        if reward_item_id != RELAY_CORE_FRAGMENT || reward_quantity != expected.reward.fragments {
            return Err(ActivityError::new(format!(
                "route {route_id} reward does not match {CATALOG_REVISION}"
            )));
        }
        let progression_table = required_table(&table, "progression", &context)?;
        let experience = parse_progression(&progression_table, "route progression")?.experience();
        if experience != expected.reward.experience {
            return Err(ActivityError::new(format!(
                "route {route_id} experience does not match {CATALOG_REVISION}"
            )));
        }
        let objective_table = required_table(&table, "objective_path", &context)?;
        let objective_path = parse_route_objective_path(&objective_table, objectives)?;
        if objective_path.as_slice() != expected.objective_path {
            return Err(ActivityError::new(format!(
                "route {route_id} objective path does not match {CATALOG_REVISION}"
            )));
        }
        let event_tables = required_table(&table, "events", &context)?;
        let events = parse_route_events(&event_tables, route_id)?;
        routes.push(AuthoringRoute {
            route_id,
            duration_ms,
            reward_item_id,
            reward_quantity,
            experience,
            objective_path,
            events,
        });
    }
    Ok(routes)
}

fn parse_route_objective_path(
    table: &Table,
    objectives: &HashMap<String, Objective>,
) -> Result<Vec<RouteObjectiveId>, ActivityError> {
    validate_sequence_shape(table, MAX_ROUTE_OBJECTIVES, "route objective path")?;
    if table.raw_len() < 3 {
        return Err(ActivityError::new(
            "route objective path requires three or four entries",
        ));
    }
    let mut path = Vec::with_capacity(table.raw_len());
    let mut unique = BTreeSet::new();
    for value in table.clone().sequence_values::<String>() {
        let value = value.map_err(activity_schema_error)?;
        if !objectives.contains_key(&value) {
            return Err(ActivityError::new(format!(
                "route path references unknown objective: {value}"
            )));
        }
        let objective_id = route_objective_id(&value)?;
        if !unique.insert(objective_id) {
            return Err(ActivityError::new(format!(
                "route path repeats objective: {value}"
            )));
        }
        path.push(objective_id);
    }
    Ok(path)
}

fn parse_route_events(
    tables: &Table,
    route_id: RouteId,
) -> Result<Vec<AuthoringRouteEvent>, ActivityError> {
    validate_sequence_shape(tables, MAX_EVENTS_PER_ROUTE, "route events")?;
    if tables.raw_len() != MAX_EVENTS_PER_ROUTE {
        return Err(ActivityError::new(
            "route must contain exactly two event definitions",
        ));
    }
    let expected = route_definition(route_id);
    let mut events = Vec::with_capacity(MAX_EVENTS_PER_ROUTE);
    for (index, value) in tables.clone().sequence_values::<Table>().enumerate() {
        let table = value.map_err(activity_schema_error)?;
        let context = format!("route event {}", index + 1);
        validate_map_keys(
            &table,
            &["id", "warden_health_basis_points", "warden_counter_damage"],
            &context,
        )?;
        let event_text: String = required_field(&table, "id", &context)?;
        let event_id = route_event_id(&event_text)?;
        if event_id != expected.events[index] {
            return Err(ActivityError::new(format!(
                "route {route_id} events must use canonical catalog order"
            )));
        }
        let health: u32 = required_field(&table, "warden_health_basis_points", &context)?;
        let counter: u32 = required_field(&table, "warden_counter_damage", &context)?;
        let effect = event_definition(event_id).effect;
        if health != effect.warden_health_basis_points || counter != effect.warden_counter_damage {
            return Err(ActivityError::new(format!(
                "event {event_id} effect does not match {CATALOG_REVISION}"
            )));
        }
        events.push(AuthoringRouteEvent {
            event_id,
            warden_health_basis_points: health,
            warden_counter_damage: counter,
        });
    }
    Ok(events)
}

fn validate_objective_graph(
    objectives: &HashMap<String, Objective>,
    triggers: &[TriggerDefinition],
    routes: &[AuthoringRoute],
    activity_kind: &str,
) -> Result<(), ActivityError> {
    let mut edges = BTreeSet::new();
    let mut completed = BTreeSet::new();
    for trigger in triggers {
        for objective_id in &trigger.complete {
            if !completed.insert(objective_id.clone()) {
                return Err(ActivityError::new(format!(
                    "objective has multiple completion transitions: {objective_id}"
                )));
            }
        }
        for from in &trigger.complete {
            for to in &trigger.activate {
                if from == to || !edges.insert((from.clone(), to.clone())) {
                    return Err(ActivityError::new(
                        "objective graph contains a self-edge or duplicate edge",
                    ));
                }
            }
        }
    }
    if objectives.keys().any(|id| !completed.contains(id)) {
        return Err(ActivityError::new(
            "every objective must have one authored completion transition",
        ));
    }
    validate_acyclic(objectives, &edges)?;
    let mut reachable = objectives
        .values()
        .filter(|objective| objective.state == ObjectiveState::Active)
        .map(|objective| objective.id.clone())
        .collect::<BTreeSet<_>>();
    loop {
        let before = reachable.len();
        for (from, to) in &edges {
            if reachable.contains(from) {
                reachable.insert(to.clone());
            }
        }
        if reachable.len() == before {
            break;
        }
    }
    for route in routes {
        reachable.extend(route.objective_path.iter().map(ToString::to_string));
    }
    if let Some(unreachable) = objectives.keys().find(|id| !reachable.contains(*id)) {
        return Err(ActivityError::new(format!(
            "objective is unreachable from baseline or route path: {unreachable}"
        )));
    }
    let terminal = triggers
        .iter()
        .find(|trigger| trigger.complete_activity)
        .ok_or_else(|| ActivityError::new("activity terminal trigger is missing"))?;
    if activity_kind == "field_excursion" {
        if !routes.is_empty()
            || terminal.complete.len() != 1
            || !terminal.activate.is_empty()
            || objectives
                .values()
                .any(|objective| objective.kind != ObjectiveKind::ReachArea)
            || triggers.iter().any(|trigger| {
                trigger.event != "AreaReached"
                    || trigger.spawn_boss.is_some()
                    || trigger.open_door.is_some()
            })
        {
            return Err(ActivityError::new("field excursions require area objectives and one final terminal, without combat, doors, or routes"));
        }
    } else if activity_kind == "campaign_chapter" {
        if !routes.is_empty()
            || terminal.complete.len() != 1
            || triggers
                .iter()
                .any(|trigger| trigger.spawn_boss.is_some() || trigger.open_door.is_some())
        {
            return Err(ActivityError::new("campaign chapters require one final objective and gateway staging, without route or spawn side effects"));
        }
    } else if terminal.complete.as_slice() != [RouteObjectiveId::DefeatWarden.as_str()] {
        return Err(ActivityError::new(
            "terminal trigger must complete defeat_warden exactly",
        ));
    }
    Ok(())
}

fn validate_acyclic(
    objectives: &HashMap<String, Objective>,
    edges: &BTreeSet<(String, String)>,
) -> Result<(), ActivityError> {
    let mut indegree = objectives
        .keys()
        .map(|id| (id.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for (_, to) in edges {
        let value = indegree
            .get_mut(to)
            .ok_or_else(|| ActivityError::new("objective graph edge target is unknown"))?;
        *value = value
            .checked_add(1)
            .ok_or_else(|| ActivityError::new("objective graph indegree overflowed"))?;
    }
    let mut queue = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect::<VecDeque<_>>();
    let mut visited = 0;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        for (_, to) in edges.iter().filter(|(from, _)| from == &id) {
            let degree = indegree
                .get_mut(to)
                .ok_or_else(|| ActivityError::new("objective graph target disappeared"))?;
            *degree = degree
                .checked_sub(1)
                .ok_or_else(|| ActivityError::new("objective graph indegree underflowed"))?;
            if *degree == 0 {
                queue.push_back(to.clone());
            }
        }
    }
    if visited != objectives.len() {
        return Err(ActivityError::new("objective graph contains a cycle"));
    }
    Ok(())
}

fn validate_map_keys(table: &Table, allowed: &[&str], context: &str) -> Result<(), ActivityError> {
    for pair in table.clone().pairs::<Value, Value>() {
        let (key, _) = pair.map_err(activity_schema_error)?;
        let Value::String(key) = key else {
            return Err(ActivityError::new(format!(
                "{context} must use named string fields"
            )));
        };
        let key = key
            .to_str()
            .map_err(|_| ActivityError::new(format!("{context} field must be UTF-8")))?;
        if !allowed.contains(&key.as_ref()) {
            return Err(ActivityError::new(format!(
                "unknown {context} field: {key}"
            )));
        }
    }
    Ok(())
}

fn validate_sequence_shape(
    table: &Table,
    maximum: usize,
    context: &str,
) -> Result<(), ActivityError> {
    if table.has_metatable() {
        return Err(ActivityError::new(format!(
            "{context} cannot have a metatable"
        )));
    }
    let length = table.raw_len();
    if length > maximum {
        return Err(ActivityError::new(format!(
            "{context} contains {length} entries; maximum is {maximum}"
        )));
    }
    let mut indices = BTreeSet::new();
    for pair in table.clone().pairs::<Value, Value>() {
        let (key, _) = pair.map_err(activity_schema_error)?;
        let Value::Integer(index) = key else {
            return Err(ActivityError::new(format!(
                "{context} must be a dense sequence"
            )));
        };
        let index = usize::try_from(index)
            .map_err(|_| ActivityError::new(format!("{context} index is invalid")))?;
        if index == 0 || index > length || !indices.insert(index) {
            return Err(ActivityError::new(format!(
                "{context} must be a dense sequence"
            )));
        }
    }
    if indices.len() != length {
        return Err(ActivityError::new(format!(
            "{context} must be a dense sequence"
        )));
    }
    Ok(())
}

fn optional_string_sequence(
    table: &Table,
    key: &str,
    maximum: usize,
    context: &str,
) -> Result<Vec<String>, ActivityError> {
    let Some(values) = optional_field::<Table>(table, key, context)? else {
        return Ok(Vec::new());
    };
    validate_sequence_shape(&values, maximum, key)?;
    values
        .sequence_values::<String>()
        .map(|value| {
            let value = value.map_err(activity_schema_error)?;
            validate_identifier(&value, key)?;
            Ok(value)
        })
        .collect()
}

fn validate_unique_references(values: &[String], context: &str) -> Result<(), ActivityError> {
    let unique = values.iter().collect::<BTreeSet<_>>();
    if unique.len() != values.len() {
        return Err(ActivityError::new(format!(
            "{context} contains a duplicate objective"
        )));
    }
    Ok(())
}

fn validate_identifier(value: &str, context: &str) -> Result<(), ActivityError> {
    if value.is_empty()
        || value.len() > MAX_ACTIVITY_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(ActivityError::new(format!(
            "{context} must match [a-z0-9_]{{1,32}}"
        )));
    }
    Ok(())
}

fn route_objective_id(value: &str) -> Result<RouteObjectiveId, ActivityError> {
    [
        RouteObjectiveId::ClearDroneGroup,
        RouteObjectiveId::ReachRelayStabilizer,
        RouteObjectiveId::ReachRelayDoor,
        RouteObjectiveId::DefeatWarden,
    ]
    .into_iter()
    .find(|objective_id| objective_id.as_str() == value)
    .ok_or_else(|| ActivityError::new(format!("unknown route objective: {value}")))
}

fn route_event_id(value: &str) -> Result<RouteEventId, ActivityError> {
    RouteEventId::ALL
        .into_iter()
        .find(|event_id| event_id.as_str() == value)
        .ok_or_else(|| ActivityError::new(format!("unknown route event: {value}")))
}

fn required_table(table: &Table, key: &str, context: &str) -> Result<Table, ActivityError> {
    required_field(table, key, context)
}

fn required_field<T: FromLua>(table: &Table, key: &str, context: &str) -> Result<T, ActivityError> {
    table
        .get(key)
        .map_err(|error| ActivityError::new(format!("invalid or missing {context}.{key}: {error}")))
}

fn optional_field<T: FromLua>(
    table: &Table,
    key: &str,
    context: &str,
) -> Result<Option<T>, ActivityError> {
    table
        .get(key)
        .map_err(|error| ActivityError::new(format!("invalid {context}.{key}: {error}")))
}

const fn authoring_limits() -> AuthoringLimits {
    AuthoringLimits {
        source_bytes: MAX_ACTIVITY_LUA_BYTES,
        memory_bytes: MAX_ACTIVITY_LUA_MEMORY_BYTES,
        instructions: MAX_ACTIVITY_LUA_INSTRUCTIONS,
        string_bytes: MAX_AUTHORING_STRING_BYTES,
        tables: MAX_AUTHORING_TABLES,
        values: MAX_AUTHORING_VALUES,
        objectives: MAX_OBJECTIVES,
        triggers: MAX_TRIGGERS,
        routes: MAX_ROUTES,
        events_per_route: MAX_EVENTS_PER_ROUTE,
        objectives_per_route: MAX_ROUTE_OBJECTIVES,
    }
}

fn activity_schema_error(error: impl Display) -> ActivityError {
    ActivityError::new(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        validate_activity_file, validate_activity_source, CATALOG_REVISION, MAX_ACTIVITY_LUA_BYTES,
        MAX_ACTIVITY_LUA_INSTRUCTIONS, MAX_ACTIVITY_LUA_MEMORY_BYTES,
    };

    const CURRENT: &str = include_str!("../../../scripts/activities/relay_awakening.lua");
    const GOLDEN: &str = include_str!("../../../tests/fixtures/m27/relay_awakening_m26.lua");

    #[test]
    fn tactical_campaign_has_separate_authoring_contract() {
        let source = include_str!("../../../scripts/activities/campaign_supply.lua");
        let manifest = validate_activity_source(source).unwrap();
        assert_eq!(manifest.activity_kind, "campaign_chapter");
        assert!(!manifest.supports_compatibility_baseline);
        assert_eq!(manifest.objectives.len(), 4);
        // The old exploration and baseline contracts are not broadened.
        assert!(
            validate_activity_source(&source.replace("campaign_chapter", "field_excursion"))
                .is_err()
        );
        assert!(validate_activity_source(
            &source.replace("    kind = \"campaign_chapter\",\n", "")
        )
        .is_err());
        assert!(validate_activity_source(&source.replace(
            "activate = { \"supply_guard\" }",
            "activate = { \"supply_guard\" }, spawn_boss = \"warden\""
        ))
        .is_err());
    }

    #[test]
    fn current_routed_authoring_matches_exact_catalog_and_limits() {
        let manifest = validate_activity_source(CURRENT).expect("current activity should validate");
        assert_eq!(
            manifest.authoring_revision.as_deref(),
            Some(CATALOG_REVISION)
        );
        assert_eq!(manifest.objectives.len(), 4);
        assert_eq!(manifest.triggers.len(), 4);
        assert_eq!(manifest.routes.len(), 2);
        assert_eq!(manifest.routes[0].route_id.to_string(), "breach");
        assert_eq!(manifest.routes[1].route_id.to_string(), "stabilize");
        assert_eq!(manifest.routes[0].events.len(), 2);
        assert_eq!(manifest.routes[1].events.len(), 2);
        assert_eq!(manifest.limits.source_bytes, MAX_ACTIVITY_LUA_BYTES);
        assert_eq!(manifest.limits.memory_bytes, MAX_ACTIVITY_LUA_MEMORY_BYTES);
        assert_eq!(manifest.limits.instructions, MAX_ACTIVITY_LUA_INSTRUCTIONS);
    }

    #[test]
    fn original_m26_table_remains_a_valid_compatibility_definition() {
        let manifest = validate_activity_source(GOLDEN).expect("golden activity should validate");
        assert_eq!(manifest.authoring_revision, None);
        assert_eq!(manifest.objectives.len(), 3);
        assert_eq!(manifest.triggers.len(), 3);
        assert!(manifest.routes.is_empty());
        assert_eq!(manifest.reward_quantity, 1);
        assert_eq!(manifest.experience, 100);
    }

    #[test]
    fn source_instruction_and_memory_limits_reject() {
        let oversized = " ".repeat(MAX_ACTIVITY_LUA_BYTES + 1);
        assert!(validate_activity_source(&oversized)
            .expect_err("oversized source should fail")
            .to_string()
            .contains("source"));
        assert!(validate_activity_source("while true do end")
            .expect_err("infinite source should fail")
            .to_string()
            .contains("instruction limit"));
        assert!(validate_activity_source(
            "return { id = string.rep('x', 1048576), reward = {}, progression = {}, objectives = {}, triggers = {} }"
        )
        .expect_err("memory-heavy source should fail")
        .to_string()
            .contains("memory limit"));
    }

    #[test]
    fn static_graph_and_collection_limits_reject() {
        for (source, expected) in [
            (
                "local root = {}; for i = 1, 65 do root[i] = {} end; return root",
                "64 tables",
            ),
            (
                "local root = {}; for i = 1, 257 do root[i] = true end; return root",
                "256 values",
            ),
            (
                "local root = {}; for _ = 1, 8 do root = { root } end; return root",
                "nested too deeply",
            ),
            ("return { id = string.rep('a', 129) }", "128 bytes"),
            ("return { id = string.char(255) }", "valid UTF-8"),
        ] {
            assert!(
                validate_activity_source(source)
                    .expect_err("static graph limit should fail")
                    .to_string()
                    .contains(expected),
                "expected {expected} rejection"
            );
        }

        for invalid in [
            CURRENT.replacen("objectives = {", "objectives = { {}, {}, {}, {}, {},", 1),
            CURRENT.replacen(
                "triggers = {",
                "triggers = { {}, {}, {}, {}, {}, {}, {}, {}, {},",
                1,
            ),
            CURRENT.replacen("routes = {", "routes = { {},", 1),
            CURRENT.replacen("events = {", "events = { {},", 1),
            CURRENT.replacen(
                "objective_path = {",
                "objective_path = { \"clear_drone_group\", \"reach_relay_door\",",
                1,
            ),
        ] {
            assert!(validate_activity_source(&invalid).is_err());
        }
    }

    #[test]
    fn file_loader_rejects_non_utf8_source() {
        let path = std::env::temp_dir().join(format!(
            "revenant-m27-invalid-utf8-{}.lua",
            std::process::id()
        ));
        fs::write(&path, [0xff, 0xfe]).expect("invalid UTF-8 fixture should write");
        let error = validate_activity_file(&path)
            .expect_err("invalid UTF-8 source should fail")
            .to_string();
        fs::remove_file(&path).expect("temporary fixture should be removable");
        assert!(error.contains("valid UTF-8"));
    }

    #[test]
    fn executable_metatable_cycle_alias_and_float_values_reject() {
        for source in [
            "return { id = function() end }",
            "local t = {}; return setmetatable(t, {})",
            "local t = {}; t[1] = t; return t",
            "local t = {}; return { id = t, reward = t }",
            "return { id = 1.5 }",
            "return require('activity')",
            "return dofile('activity.lua')",
        ] {
            assert!(
                validate_activity_source(source).is_err(),
                "source should reject: {source}"
            );
        }
    }

    #[test]
    fn unknown_fields_and_mixed_or_sparse_sequences_reject() {
        let unknown = CURRENT.replacen("return {", "return { unknown = true,", 1);
        assert!(validate_activity_source(&unknown)
            .expect_err("unknown field should fail")
            .to_string()
            .contains("unknown activity field"));
        let mixed = GOLDEN.replace(
            "objectives = {",
            "objectives = { named = { id = 'bad', kind = 'Boss', state = 'Pending', target = 1 },",
        );
        assert!(validate_activity_source(&mixed).is_err());
        let sparse = GOLDEN.replace(
            r#"    objectives = {
        { id = "clear_drone_group", kind = "KillActors", state = "Active", target = 1 },
        { id = "reach_relay_door", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "defeat_warden", kind = "Boss", state = "Pending", target = 1 },
    },"#,
            r#"    objectives = {
        [1] = { id = "clear_drone_group", kind = "KillActors", state = "Active", target = 1 },
        [3] = { id = "reach_relay_door", kind = "ReachArea", state = "Pending", target = 1 },
        [4] = { id = "defeat_warden", kind = "Boss", state = "Pending", target = 1 },
    },"#,
        );
        assert!(validate_activity_source(&sparse).is_err());
    }

    #[test]
    fn duplicate_unknown_and_cyclic_objective_graphs_reject() {
        let duplicate = GOLDEN.replacen("reach_relay_door", "clear_drone_group", 1);
        assert!(validate_activity_source(&duplicate)
            .expect_err("duplicate objective should fail")
            .to_string()
            .contains("duplicate objective"));
        let unknown = GOLDEN.replacen(
            "activate = { \"reach_relay_door\" }",
            "activate = { \"unknown_objective\" }",
            1,
        );
        assert!(validate_activity_source(&unknown)
            .expect_err("unknown reference should fail")
            .to_string()
            .contains("unknown objective"));
        let cycle = GOLDEN.replacen(
            "activate = { \"defeat_warden\" }",
            "activate = { \"clear_drone_group\" }",
            1,
        );
        assert!(validate_activity_source(&cycle)
            .expect_err("cycle should fail")
            .to_string()
            .contains("cycle"));

        let multiple_completion = GOLDEN.replacen(
            "complete = { \"defeat_warden\" }",
            "complete = { \"defeat_warden\", \"reach_relay_door\" }",
            1,
        );
        assert!(validate_activity_source(&multiple_completion)
            .expect_err("multiple completion should fail")
            .to_string()
            .contains("multiple completion"));

        let ambiguous = GOLDEN.replacen(
            "event = \"AreaReached\",\n            subject = \"relay_door\"",
            "event = \"ActorGroupDead\",\n            subject = \"relay_drones\"",
            1,
        );
        assert!(validate_activity_source(&ambiguous)
            .expect_err("duplicate trigger identity should fail")
            .to_string()
            .contains("duplicate trigger identity"));

        let unreachable =
            GOLDEN.replacen("activate = { \"reach_relay_door\" }", "activate = {}", 1);
        assert!(validate_activity_source(&unreachable)
            .expect_err("unreachable objective should fail")
            .to_string()
            .contains("unreachable"));
    }

    #[test]
    fn terminal_trigger_cardinality_and_late_activation_reject() {
        let multiple = GOLDEN.replacen(
            "activate = { \"reach_relay_door\" },",
            "activate = { \"reach_relay_door\" }, complete_activity = true,",
            1,
        );
        assert!(validate_activity_source(&multiple).is_err());
        let late = GOLDEN.replacen(
            "complete_activity = true,",
            "complete_activity = true, activate = { \"reach_relay_door\" },",
            1,
        );
        assert!(validate_activity_source(&late)
            .expect_err("terminal activation should fail")
            .to_string()
            .contains("terminal trigger"));
        let missing = GOLDEN.replacen("complete_activity = true,", "", 1);
        assert!(validate_activity_source(&missing)
            .expect_err("missing terminal should fail")
            .to_string()
            .contains("exactly one terminal"));
    }

    #[test]
    fn route_revision_count_order_reward_path_and_effect_are_exact() {
        for invalid in [
            CURRENT.replace("m27-v1", "m27-v2"),
            CURRENT.replacen("id = \"breach\"", "id = \"stabilize\"", 1),
            CURRENT.replacen("quantity = 2", "quantity = 1", 1),
            CURRENT.replacen("duration_ms = 90000", "duration_ms = 90001", 1),
            CURRENT.replacen("\"reach_relay_door\",", "\"reach_relay_stabilizer\",", 1),
            CURRENT.replacen(
                "warden_counter_damage = 20",
                "warden_counter_damage = 19",
                1,
            ),
            CURRENT.replacen("id = \"arc_surge\"", "id = \"overcharged_armor\"", 1),
        ] {
            assert!(validate_activity_source(&invalid).is_err());
        }
    }

    #[test]
    fn manifest_serialization_is_stable() {
        let first = serde_json::to_string(
            &validate_activity_source(CURRENT).expect("first validation should pass"),
        )
        .expect("manifest should serialize");
        let second = serde_json::to_string(
            &validate_activity_source(CURRENT).expect("second validation should pass"),
        )
        .expect("manifest should serialize");
        assert_eq!(first, second);
    }
}
