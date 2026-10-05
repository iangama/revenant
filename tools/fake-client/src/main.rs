use std::collections::BTreeMap;
use std::env;
use std::net::TcpStream;
use std::process::ExitCode;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use revenant_protocol::{
    read_message, write_message, AuthRequest, CharacterListRequest, ClientHello, ClientMessage,
    DamageApplied, ModuleCombineIntent, ModuleLoadoutIntent, ModulePreviewRequest, ModuleSnapshot,
    ModuleStateRequest, RouteChoiceIntent, RouteObjectiveId, RouteObjectiveState, RoutePhase,
    RouteSelection, RouteState, RouteStateRequest, RouteTerminalOutcome, ServerMessage,
    PROTOCOL_VERSION,
};
use revenant_protocol::{AttackIntent, MoveIntent, WorldJoinRequest};
use serde::Serialize;

mod security_probe;

const SMOKE_IO_TIMEOUT: Duration = Duration::from_secs(15);
const MATRIX_SYNC_DELAY: Duration = Duration::from_millis(400);
const MATRIX_EVENT_POLL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestedRoute {
    Breach,
    Stabilize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RouteProbeMode {
    Success,
    Disconnect,
    Timeout,
    CapabilityMismatch,
}

impl RouteProbeMode {
    fn parse(value: &str) -> Result<Self, &'static str> {
        match value {
            "" | "success" => Ok(Self::Success),
            "disconnect" => Ok(Self::Disconnect),
            "timeout" => Ok(Self::Timeout),
            "capability-mismatch" => Ok(Self::CapabilityMismatch),
            _ => Err(
                "REVENANT_BOT_ROUTE_MODE must be success, disconnect, timeout, or capability-mismatch",
            ),
        }
    }
}

impl RequestedRoute {
    fn parse(value: &str) -> Result<Option<Self>, &'static str> {
        match value {
            "" => Ok(None),
            "breach" => Ok(Some(Self::Breach)),
            "stabilize" => Ok(Some(Self::Stabilize)),
            _ => Err("REVENANT_BOT_ROUTE must be empty, breach, or stabilize"),
        }
    }

    const fn id(self) -> &'static str {
        match self {
            Self::Breach => "breach",
            Self::Stabilize => "stabilize",
        }
    }

    const fn other(self) -> &'static str {
        match self {
            Self::Breach => "stabilize",
            Self::Stabilize => "breach",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatrixRole {
    Primary,
    Secondary,
}

impl MatrixRole {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "matrix-primary" => Some(Self::Primary),
            "matrix-secondary" => Some(Self::Secondary),
            _ => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModuleBuild {
    Empty,
    Force,
    Ward,
    ForceWard,
}

impl ModuleBuild {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "" | "empty" => Some(Self::Empty),
            "force" => Some(Self::Force),
            "ward" => Some(Self::Ward),
            "force-ward" => Some(Self::ForceWard),
            _ => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Force => "force",
            Self::Ward => "ward",
            Self::ForceWard => "force-ward",
        }
    }

    const fn maximum_health(self) -> u32 {
        match self {
            Self::Empty | Self::Force => 100,
            Self::Ward | Self::ForceWard => 120,
        }
    }

    fn weapon_damage(self, weapon: &str) -> Option<u32> {
        match (self, weapon) {
            (Self::Empty | Self::Ward, "pulse_rifle") => Some(40),
            (Self::Force | Self::ForceWard, "pulse_rifle") => Some(48),
            (Self::Empty | Self::Ward, "arc_sidearm") => Some(25),
            (Self::Force | Self::ForceWard, "arc_sidearm") => Some(30),
            _ => None,
        }
    }

    const fn profiles(self) -> [(&'static str, u32, i32, u64); 2] {
        match self {
            Self::Empty => [("pulse_rifle", 40, 6, 250), ("arc_sidearm", 25, 8, 150)],
            Self::Force => [("pulse_rifle", 48, 6, 300), ("arc_sidearm", 30, 8, 180)],
            Self::Ward => [("pulse_rifle", 40, 6, 275), ("arc_sidearm", 25, 8, 165)],
            Self::ForceWard => [("pulse_rifle", 48, 6, 325), ("arc_sidearm", 30, 8, 195)],
        }
    }

    fn modules(self) -> Vec<String> {
        match self {
            Self::Empty => Vec::new(),
            Self::Force => vec!["module_force_matrix".to_owned()],
            Self::Ward => vec!["module_ward_capacitor".to_owned()],
            Self::ForceWard => vec![
                "module_force_matrix".to_owned(),
                "module_ward_capacitor".to_owned(),
            ],
        }
    }
}

fn matrix_weapon_damage_values(weapon: &str) -> Option<[u32; 2]> {
    match weapon {
        "pulse_rifle" => Some([40, 48]),
        "arc_sidearm" => Some([25, 30]),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
struct EncounterEvidence {
    enemy: &'static str,
    target_health: u32,
    accepted_hits: u32,
    accepted_damage: Vec<u32>,
    hit_sources: Vec<u64>,
    remaining_health: Vec<u32>,
    hits_by_actor: BTreeMap<u64, u32>,
    ttk_ms: u64,
    hostile_damage: Vec<u32>,
}

struct EncounterTracker {
    enemy: &'static str,
    enemy_id: u64,
    target_health: u32,
    accepted_damage: Vec<u32>,
    hit_sources: Vec<u64>,
    remaining_health: Vec<u32>,
    hits_by_actor: BTreeMap<u64, u32>,
    first_hit_at: Option<Instant>,
    lethal_hit_at: Option<Instant>,
    hostile_damage: Vec<u32>,
}

impl EncounterTracker {
    fn new(enemy: &'static str, enemy_id: u64, target_health: u32) -> Self {
        Self {
            enemy,
            enemy_id,
            target_health,
            accepted_damage: Vec::new(),
            hit_sources: Vec::new(),
            remaining_health: Vec::new(),
            hits_by_actor: BTreeMap::new(),
            first_hit_at: None,
            lethal_hit_at: None,
            hostile_damage: Vec::new(),
        }
    }

    fn player_hit(&mut self, damage: &DamageApplied, received_at: Instant) {
        self.first_hit_at.get_or_insert(received_at);
        if damage.killed {
            self.lethal_hit_at = Some(received_at);
        }
        self.accepted_damage.push(damage.damage);
        self.hit_sources.push(damage.source_actor_id);
        self.remaining_health.push(damage.remaining_health);
        *self
            .hits_by_actor
            .entry(damage.source_actor_id)
            .or_default() += 1;
    }

    fn hostile_hit(&mut self, damage: &DamageApplied) {
        self.hostile_damage.push(damage.damage);
    }

    fn evidence(
        self,
        player_ids: &[u64],
        allowed_weapon_damage: &[u32],
    ) -> Result<EncounterEvidence, Box<dyn std::error::Error>> {
        if self.accepted_damage.len() != self.remaining_health.len()
            || self.hit_sources.len() != self.remaining_health.len()
            || self.accepted_damage.is_empty()
            || self
                .accepted_damage
                .iter()
                .any(|damage| !allowed_weapon_damage.contains(damage))
            || self
                .hit_sources
                .iter()
                .any(|source| !player_ids.contains(source))
        {
            return Err(format!(
                "{} accepted-hit evidence diverged: sources={:?} damage={:?} remaining={:?}",
                self.enemy, self.hit_sources, self.accepted_damage, self.remaining_health
            )
            .into());
        }
        let mut health = self.target_health;
        let expected_health = self
            .accepted_damage
            .iter()
            .map(|damage| {
                health = health.saturating_sub(*damage);
                health
            })
            .collect::<Vec<_>>();
        if self.remaining_health != expected_health {
            return Err(format!(
                "{} health sequence diverged: expected {expected_health:?}, got {:?}",
                self.enemy, self.remaining_health
            )
            .into());
        }
        if player_ids
            .iter()
            .any(|player_id| self.hits_by_actor.get(player_id).copied().unwrap_or(0) == 0)
        {
            return Err(format!("{} was not attacked by every active player", self.enemy).into());
        }
        let first_hit = self
            .first_hit_at
            .ok_or_else(|| format!("{} has no first accepted hit", self.enemy))?;
        let lethal_hit = self
            .lethal_hit_at
            .ok_or_else(|| format!("{} has no lethal accepted hit", self.enemy))?;
        let ttk_ms = u64::try_from(lethal_hit.duration_since(first_hit).as_millis())?;
        Ok(EncounterEvidence {
            enemy: self.enemy,
            target_health: self.target_health,
            accepted_hits: u32::try_from(self.remaining_health.len())?,
            accepted_damage: self.accepted_damage,
            hit_sources: self.hit_sources,
            remaining_health: self.remaining_health,
            hits_by_actor: self.hits_by_actor,
            ttk_ms,
            hostile_damage: self.hostile_damage,
        })
    }
}

#[derive(Debug, Serialize)]
struct MatrixEvidence {
    schema: &'static str,
    account_id: String,
    role: &'static str,
    weapon: String,
    module_build: &'static str,
    maximum_health: u32,
    participants: usize,
    simulated_rtt_ms: u64,
    attack_interval_ms: u64,
    drone: EncounterEvidence,
    warden: EncounterEvidence,
    player_health: BTreeMap<u64, Vec<u32>>,
    loot_grants: u32,
    progression_grants: u32,
    experience_granted: u64,
    next_module_build: Option<&'static str>,
    next_loadout_revision: Option<u64>,
    next_loadout_retry_replayed: bool,
    completed: bool,
}

#[derive(Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct RouteMatrixEvidence {
    schema: &'static str,
    account_id: String,
    role: &'static str,
    weapon: String,
    module_build: &'static str,
    maximum_health: u32,
    participants: usize,
    attack_interval_ms: u64,
    route_id: String,
    event_id: String,
    seed: u64,
    warden_health_basis_points: u32,
    warden_counter_damage: u32,
    objective_path: Vec<String>,
    duration_budget_ms: u64,
    elapsed_ms: u64,
    transition_count: usize,
    drone: EncounterEvidence,
    warden: EncounterEvidence,
    player_health: BTreeMap<u64, Vec<u32>>,
    loot_quantity: u32,
    experience_granted: u64,
    terminal_grant_count: usize,
    retry_replayed: bool,
    conflict_rejected: bool,
    non_leader_rejected: bool,
    next_module_build: Option<&'static str>,
    next_loadout_revision: Option<u64>,
    next_loadout_retry_replayed: bool,
    completed: bool,
}

struct MatrixConfig {
    account_id: String,
    role: MatrixRole,
    requested_weapon: String,
    expected_players: usize,
    player_id: u64,
    player_ids: Vec<u64>,
    first_enemy_id: u64,
    initial_pressure: DamageApplied,
    attack_cooldown_ms: u64,
    simulated_rtt_ms: u64,
    module_build: ModuleBuild,
    next_module_build: Option<ModuleBuild>,
    next_operation_id: Option<String>,
    requested_route: Option<RequestedRoute>,
}

fn main() -> ExitCode {
    if let Ok(mode) = env::var("REVENANT_BOT_SECURITY_PROBE") {
        return match security_probe::run(&mode) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("M30 security probe failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    match handshake() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("revenant-bot handshake failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
fn handshake() -> Result<(), Box<dyn std::error::Error>> {
    let game_addr = env::var("REVENANT_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7000".to_owned());
    let username = env::var("REVENANT_BOT_USERNAME").unwrap_or_else(|_| "revenant-bot".to_owned());
    let role = env::var("REVENANT_BOT_ROLE").unwrap_or_else(|_| "driver".to_owned());
    let requested_route_value = env::var("REVENANT_BOT_ROUTE").unwrap_or_default();
    let requested_route = RequestedRoute::parse(&requested_route_value)?;
    let route_probe_mode =
        RouteProbeMode::parse(&env::var("REVENANT_BOT_ROUTE_MODE").unwrap_or_default())?;
    let module_flow = env::var("REVENANT_BOT_MODULE_FLOW").unwrap_or_default();
    if !matches!(module_flow.as_str(), "" | "mutate") {
        return Err("REVENANT_BOT_MODULE_FLOW must be empty or mutate".into());
    }
    let expected_module_build_value =
        env::var("REVENANT_BOT_EXPECT_MODULE_BUILD").unwrap_or_default();
    let expected_module_build = ModuleBuild::parse(&expected_module_build_value)
        .ok_or("REVENANT_BOT_EXPECT_MODULE_BUILD must be empty, force, ward, or force-ward")?;
    let next_module_build_value = env::var("REVENANT_BOT_NEXT_MODULE_BUILD").unwrap_or_default();
    let next_module_build =
        if next_module_build_value.is_empty() {
            None
        } else {
            Some(ModuleBuild::parse(&next_module_build_value).ok_or(
                "REVENANT_BOT_NEXT_MODULE_BUILD must be empty, force, ward, or force-ward",
            )?)
        };
    let next_operation_id = env::var("REVENANT_BOT_NEXT_OPERATION_ID").ok();
    let matrix_role = MatrixRole::parse(&role);
    if next_module_build.is_some() && matrix_role.is_none() {
        return Err("a next module build is only valid for a matrix role".into());
    }
    if next_module_build.is_some() && next_module_build == Some(expected_module_build) {
        return Err("the next module build must differ from the admitted build".into());
    }
    if next_module_build.is_some() && next_operation_id.as_deref().is_none_or(str::is_empty) {
        return Err("REVENANT_BOT_NEXT_OPERATION_ID is required for a next module build".into());
    }
    let simulated_rtt_ms = env::var("REVENANT_BOT_ROUND_TRIP_MS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(0);
    if simulated_rtt_ms > 2_000 {
        return Err("REVENANT_BOT_ROUND_TRIP_MS must be at most 2000".into());
    }
    let requested_weapon =
        env::var("REVENANT_BOT_WEAPON").unwrap_or_else(|_| "arc_sidearm".to_owned());
    if !matches!(requested_weapon.as_str(), "pulse_rifle" | "arc_sidearm") {
        return Err("REVENANT_BOT_WEAPON must be pulse_rifle or arc_sidearm".into());
    }
    let expected_players = env::var("REVENANT_EXPECTED_PLAYERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let requested_protocol = env::var("REVENANT_BOT_PROTOCOL_VERSION")
        .ok()
        .map(|value| value.parse::<u16>())
        .transpose()?
        .unwrap_or(PROTOCOL_VERSION);
    let expect_protocol_rejection =
        env::var("REVENANT_BOT_EXPECT_PROTOCOL_REJECTION").is_ok_and(|value| value == "1");
    if !matches!(requested_protocol, 1 | PROTOCOL_VERSION) && !expect_protocol_rejection {
        return Err(
            "an unsupported REVENANT_BOT_PROTOCOL_VERSION requires REVENANT_BOT_EXPECT_PROTOCOL_REJECTION=1"
                .into(),
        );
    }
    if matches!(requested_protocol, 1 | PROTOCOL_VERSION) && expect_protocol_rejection {
        return Err("the protocol-rejection probe requires an unsupported generation".into());
    }
    if route_probe_mode != RouteProbeMode::Success && requested_route.is_none() {
        return Err("a route probe mode requires REVENANT_BOT_ROUTE".into());
    }
    if requested_route.is_some() && requested_protocol != PROTOCOL_VERSION {
        return Err("route probes require current Protocol V2".into());
    }
    if matrix_role.is_some() && route_probe_mode != RouteProbeMode::Success {
        return Err("route matrix roles support only the success mode".into());
    }
    if requested_route.is_some() && matrix_role.is_none() {
        let valid_solo = expected_players == 1
            && role == "driver"
            && route_probe_mode != RouteProbeMode::CapabilityMismatch;
        let valid_capability_mismatch = expected_players == 2
            && role == "driver"
            && route_probe_mode == RouteProbeMode::CapabilityMismatch;
        if !valid_solo && !valid_capability_mismatch {
            return Err(
                "route probes require a matrix role, solo driver, or two-player capability-mismatch driver"
                    .into(),
            );
        }
    }
    let mut stream = TcpStream::connect(&game_addr)?;
    stream.set_read_timeout(Some(SMOKE_IO_TIMEOUT))?;
    stream.set_write_timeout(Some(SMOKE_IO_TIMEOUT))?;

    write_message(
        &mut stream,
        &ClientMessage::ClientHello(ClientHello {
            protocol_version: requested_protocol,
            client_name: "revenant-bot".to_owned(),
            client_build: env!("CARGO_PKG_VERSION").to_owned(),
            content_revision: None,
        }),
    )?;

    let ServerMessage::ServerHello(hello) = read_message(&mut stream)? else {
        return Err("expected ServerHello".into());
    };
    if !hello.accepted {
        if expect_protocol_rejection
            && hello.protocol_version == PROTOCOL_VERSION
            && hello.message == "unsupported protocol version"
        {
            println!("unsupported protocol v{requested_protocol} refused without negotiation");
            return Ok(());
        }
        return Err(format!(
            "server rejected protocol {requested_protocol}: {}",
            hello.message
        )
        .into());
    }
    if hello.protocol_version != requested_protocol {
        return Err(format!(
            "server selected protocol {} for requested protocol {requested_protocol}",
            hello.protocol_version
        )
        .into());
    }

    println!(
        "handshake accepted by {} using protocol v{}",
        hello.server_name, hello.protocol_version
    );
    if requested_protocol == 1 {
        if expected_players != 1
            || !module_flow.is_empty()
            || expected_module_build != ModuleBuild::Empty
            || next_module_build.is_some()
        {
            return Err("the diagnostic V1 probe requires a solo base-flow configuration".into());
        }
        return run_v1_gameplay_probe(&mut stream, &username);
    }

    write_message(
        &mut stream,
        &ClientMessage::AuthRequest(AuthRequest { username }),
    )?;
    let ServerMessage::AuthResponse(auth) = read_message(&mut stream)? else {
        return Err("expected AuthResponse".into());
    };
    if !auth.authenticated {
        return Err(format!("local authentication rejected: {}", auth.message).into());
    }
    println!("authenticated local account {}", auth.account_id);

    write_message(
        &mut stream,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )?;
    let ServerMessage::CharacterListResponse(character_list) = read_message(&mut stream)? else {
        return Err("expected CharacterListResponse".into());
    };
    let character = character_list
        .characters
        .first()
        .ok_or("server returned no local character")?;
    println!(
        "received character {} ({}, level {})",
        character.display_name, character.class_name, character.level
    );
    let character_id = character.character_id.clone();
    write_message(
        &mut stream,
        &ClientMessage::WorldJoinRequest(WorldJoinRequest { character_id }),
    )?;
    let ServerMessage::WorldJoinResponse(world) = read_message(&mut stream)? else {
        return Err("expected WorldJoinResponse".into());
    };
    if !world.accepted {
        return Err(format!("world join rejected: {}", world.message).into());
    }
    println!(
        "joined world {} as player actor {} at {:?}",
        world.world_id, world.player_actor_id, world.spawn_position
    );
    let ServerMessage::InventorySnapshot(inventory) = read_message(&mut stream)? else {
        return Err("expected InventorySnapshot".into());
    };
    if !inventory
        .items
        .iter()
        .any(|item| item.item_id == "pulse_rifle" && item.quantity == 1)
    {
        return Err("starter inventory snapshot is missing pulse_rifle".into());
    }
    println!("received authoritative inventory snapshot");
    let ServerMessage::ProgressionSnapshot(progression) = read_message(&mut stream)? else {
        return Err("expected ProgressionSnapshot".into());
    };
    if progression.level == 0 || progression.experience_to_next_level == 0 {
        return Err("authoritative progression snapshot is invalid".into());
    }
    println!(
        "received authoritative progression: level {} with {} XP",
        progression.level, progression.experience
    );
    let ServerMessage::EquipmentSnapshot(equipment) = read_message(&mut stream)? else {
        return Err("expected EquipmentSnapshot".into());
    };
    let expected_profiles = expected_module_build.profiles();
    if equipment.weapons.len() != 2
        || expected_profiles
            .iter()
            .any(|(item_id, damage, range, cooldown_ms)| {
                !equipment.weapons.iter().any(|weapon| {
                    weapon.item_id == *item_id
                        && weapon.damage == *damage
                        && weapon.range == *range
                        && weapon.cooldown_ms == *cooldown_ms
                })
            })
    {
        return Err("authoritative equipment profiles are incomplete".into());
    }
    println!(
        "received authoritative loadout: {}",
        equipment.equipped_weapon_item_id
    );
    if role != "observer"
        && matrix_role.is_none()
        && route_probe_mode != RouteProbeMode::CapabilityMismatch
    {
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: "relay_core_fragment".to_owned(),
            }),
        )?;
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: requested_weapon.clone(),
            }),
        )?;
    }
    let ServerMessage::ActivityStart(activity) = read_message(&mut stream)? else {
        return Err("expected ActivityStart".into());
    };
    let ServerMessage::ObjectiveUpdate(initial_objective) = read_message(&mut stream)? else {
        return Err("expected initial ObjectiveUpdate".into());
    };
    println!(
        "activity {} started with {} objective {}",
        activity.activity_id, initial_objective.state, initial_objective.objective_id
    );
    let mut enemy_id = None;
    let mut player_ids = Vec::new();
    let expected_player_health = expected_module_build.maximum_health();
    for _ in 0..=expected_players {
        let ServerMessage::ActorSpawn(actor) = read_message(&mut stream)? else {
            return Err("expected ActorSpawn".into());
        };
        println!(
            "actor {} spawned as {} ({}) at {:?}",
            actor.actor_id, actor.actor_kind, actor.archetype, actor.position
        );
        if actor.actor_kind == "enemy" {
            enemy_id = Some(actor.actor_id);
        } else if actor.actor_kind == "player" {
            if actor.actor_id == world.player_actor_id
                && (actor.health != expected_player_health
                    || actor.max_health != expected_player_health)
            {
                return Err(format!("admitted player health diverged: {actor:?}").into());
            }
            player_ids.push(actor.actor_id);
        }
    }
    if player_ids.len() != expected_players {
        return Err(format!(
            "expected {expected_players} replicated players, got {}",
            player_ids.len()
        )
        .into());
    }
    let enemy_id = enemy_id.ok_or("server did not spawn the first enemy actor")?;
    if role == "observer" {
        return observe_shared_activity(&mut stream, enemy_id, expected_players);
    }
    let initial_pressure = observe_enemy_ai(&mut stream, enemy_id)?;
    if let Some(matrix_role) = matrix_role {
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: "relay_core_fragment".to_owned(),
            }),
        )?;
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: requested_weapon.clone(),
            }),
        )?;
        let attack_cooldown_ms =
            receive_matrix_equipment(&mut stream, world.player_actor_id, &requested_weapon)?;
        let route_state = if requested_route.is_some() {
            let state = request_matrix_route_capability(&mut stream)?;
            validate_matrix_route_state(
                &state,
                RoutePhase::Drone,
                world.player_actor_id,
                &player_ids,
                expected_players,
                false,
            )?;
            Some(state)
        } else {
            None
        };
        let config = MatrixConfig {
            account_id: auth.account_id,
            role: matrix_role,
            requested_weapon,
            expected_players,
            player_id: world.player_actor_id,
            player_ids,
            first_enemy_id: enemy_id,
            initial_pressure,
            attack_cooldown_ms,
            simulated_rtt_ms,
            module_build: expected_module_build,
            next_module_build,
            next_operation_id,
            requested_route,
        };
        return if route_state.is_some() {
            run_route_matrix_activity(stream, config)
        } else {
            run_matrix_activity(stream, config)
        };
    }
    if route_probe_mode == RouteProbeMode::CapabilityMismatch {
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: "relay_core_fragment".to_owned(),
            }),
        )?;
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: requested_weapon.clone(),
            }),
        )?;
    }
    let ServerMessage::EquipmentChanged(rejected) = read_message(&mut stream)? else {
        return Err("expected rejected EquipmentChanged".into());
    };
    if rejected.accepted || !rejected.message.contains("not an equipable weapon") {
        return Err("non-weapon equipment intent was not rejected".into());
    }
    let ServerMessage::EquipmentChanged(equipment) = read_message(&mut stream)? else {
        return Err("expected EquipmentChanged".into());
    };
    if !equipment.accepted || equipment.equipped_weapon_item_id != requested_weapon {
        return Err(format!("weapon equip rejected: {}", equipment.message).into());
    }
    if equipment.cooldown_ms == 0 {
        return Err("equipment returned an invalid cooldown".into());
    }
    println!(
        "equipped authoritative weapon {} with {} ms cooldown",
        equipment.equipped_weapon_item_id, equipment.cooldown_ms
    );
    if module_flow == "mutate" {
        verify_active_module_rejection(&mut stream)?;
    }
    if requested_route.is_some() {
        let state = request_route_capability(&mut stream)?;
        if route_probe_mode == RouteProbeMode::CapabilityMismatch {
            validate_matrix_route_state(
                &state,
                RoutePhase::Drone,
                world.player_actor_id,
                &player_ids,
                expected_players,
                true,
            )?;
        } else {
            validate_route_state(&state, RoutePhase::Drone, world.player_actor_id, true)?;
        }
    }
    defeat_enemy(&mut stream, enemy_id, equipment.cooldown_ms)?;
    verify_objective_progression(&mut stream)?;
    if let Some(requested_route) = requested_route {
        let ServerMessage::RouteState(choice_state) = read_message(&mut stream)? else {
            return Err("expected choice-open RouteState".into());
        };
        if route_probe_mode == RouteProbeMode::CapabilityMismatch {
            validate_matrix_route_state(
                &choice_state,
                RoutePhase::ChoiceOpen,
                world.player_actor_id,
                &player_ids,
                expected_players,
                true,
            )?;
            verify_capability_mismatch(
                &mut stream,
                requested_route,
                world.player_actor_id,
                equipment.cooldown_ms,
            )?;
            return Ok(());
        }
        validate_route_state(
            &choice_state,
            RoutePhase::ChoiceOpen,
            world.player_actor_id,
            true,
        )?;
        let selection = choose_route(
            &mut stream,
            requested_route,
            world.player_actor_id,
            &auth.account_id,
        )?;
        match route_probe_mode {
            RouteProbeMode::Success => complete_route_activity(
                &mut stream,
                world.player_actor_id,
                equipment.cooldown_ms,
                requested_route,
                &selection,
            )?,
            RouteProbeMode::Disconnect => {
                println!(
                    "M27_ROUTE_DISCONNECT {}",
                    serde_json::json!({
                        "schema": "RevenantM27RouteDisconnectV1",
                        "route_id": selection.route_id,
                        "event_id": selection.event_id,
                        "seed": selection.seed,
                        "selected": true,
                        "terminal": false,
                    })
                );
            }
            RouteProbeMode::Timeout => verify_route_timeout(&mut stream, &selection)?,
            RouteProbeMode::CapabilityMismatch => unreachable!(),
        }
    } else {
        complete_activity(&mut stream, world.player_actor_id, equipment.cooldown_ms)?;
    }
    if module_flow == "mutate" {
        run_module_mutation_flow(&mut stream, &auth.account_id)?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run_v1_gameplay_probe(
    stream: &mut TcpStream,
    username: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::AuthRequest(AuthRequest {
            username: username.to_owned(),
        }),
    )?;
    let ServerMessage::AuthResponse(auth) = read_message(stream)? else {
        return Err("V1 expected AuthResponse".into());
    };
    if !auth.authenticated {
        return Err(format!("V1 authentication rejected: {}", auth.message).into());
    }
    write_message(
        stream,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )?;
    let ServerMessage::CharacterListResponse(characters) = read_message(stream)? else {
        return Err("V1 expected CharacterListResponse".into());
    };
    let character_id = characters
        .characters
        .first()
        .ok_or("V1 account has no character")?
        .character_id
        .clone();
    write_message(
        stream,
        &ClientMessage::WorldJoinRequest(WorldJoinRequest { character_id }),
    )?;
    let ServerMessage::WorldJoinResponse(world) = read_message(stream)? else {
        return Err("V1 expected WorldJoinResponse".into());
    };
    if !world.accepted {
        return Err(format!("V1 world join rejected: {}", world.message).into());
    }
    let ServerMessage::ActivityStart(activity) = read_message(stream)? else {
        return Err("V1 join emitted an unexpected message before ActivityStart".into());
    };
    let ServerMessage::ObjectiveUpdate(objective) = read_message(stream)? else {
        return Err("V1 expected initial ObjectiveUpdate".into());
    };
    if activity.activity_id != "relay_awakening" || objective.state != "Active" {
        return Err("V1 activity start diverged".into());
    }
    let ServerMessage::ActorSpawn(player) = read_message(stream)? else {
        return Err("V1 expected player ActorSpawn".into());
    };
    let ServerMessage::ActorSpawn(drone) = read_message(stream)? else {
        return Err("V1 expected enemy ActorSpawn".into());
    };
    if player.actor_id != world.player_actor_id
        || player.actor_kind != "player"
        || player.health != 100
        || player.max_health != 100
        || drone.archetype != "relay-drone"
    {
        return Err(format!("V1 admission diverged: player={player:?} drone={drone:?}").into());
    }
    let initial_pressure = observe_enemy_ai(stream, drone.actor_id)?;
    if initial_pressure.remaining_health != 90 {
        return Err(format!("V1 initial health projection diverged: {initial_pressure:?}").into());
    }
    defeat_v1_enemy(stream, drone.actor_id, &[100, 60, 20, 0])?;
    verify_objective_progression(stream)?;

    write_message(
        stream,
        &ClientMessage::MoveIntent(MoveIntent {
            position: [6, 0, 0],
        }),
    )?;
    let ServerMessage::ActorUpdate(update) = read_message(stream)? else {
        return Err("V1 expected player ActorUpdate".into());
    };
    let ServerMessage::ObjectiveUpdate(reach) = read_message(stream)? else {
        return Err("V1 expected completed ReachArea".into());
    };
    let ServerMessage::ObjectiveUpdate(boss_objective) = read_message(stream)? else {
        return Err("V1 expected active Boss objective".into());
    };
    let ServerMessage::DoorState(door) = read_message(stream)? else {
        return Err("V1 expected DoorState".into());
    };
    let ServerMessage::ActorSpawn(warden) = read_message(stream)? else {
        return Err("V1 expected Warden ActorSpawn".into());
    };
    if update.actor_id != world.player_actor_id
        || reach.state != "Completed"
        || boss_objective.state != "Active"
        || !door.open
        || warden.archetype != "warden"
    {
        return Err("V1 boss-stage transition diverged".into());
    }
    defeat_v1_enemy(stream, warden.actor_id, &[200, 160, 120, 80, 40, 0])?;
    let ServerMessage::ObjectiveUpdate(completed_boss) = read_message(stream)? else {
        return Err("V1 expected completed Boss objective".into());
    };
    let ServerMessage::ActivityComplete(completed) = read_message(stream)? else {
        return Err("V1 expected ActivityComplete without V2 reward projections".into());
    };
    if completed_boss.state != "Completed" || completed.activity_id != "relay_awakening" {
        return Err("V1 completion diverged".into());
    }
    println!(
        "M26_V1_PROBE {}",
        serde_json::json!({
            "schema": "RevenantM26V1GameplayProbeV1",
            "account_id": auth.account_id,
            "player_actor_id": world.player_actor_id,
            "max_health": player.max_health,
            "weapon": "pulse_rifle",
            "damage": 40,
            "cooldown_ms": 250,
            "completed": true,
        })
    );
    Ok(())
}

fn defeat_v1_enemy(
    stream: &mut TcpStream,
    enemy_id: u64,
    expected_remaining_health: &[u32],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut observed = Vec::new();
    loop {
        write_message(
            stream,
            &ClientMessage::AttackIntent(AttackIntent {
                target_actor_id: enemy_id,
            }),
        )?;
        let damage = loop {
            match read_message(stream)? {
                ServerMessage::DamageApplied(damage) if damage.source_actor_id == enemy_id => {
                    if damage.killed {
                        return Err("V1 hostile pressure defeated the player".into());
                    }
                }
                ServerMessage::DamageApplied(damage) => break damage,
                unexpected => {
                    return Err(
                        format!("V1 attack received unexpected message: {unexpected:?}").into(),
                    )
                }
            }
        };
        if damage.target_actor_id != enemy_id || damage.damage != 40 {
            return Err(format!("V1 attack arithmetic diverged: {damage:?}").into());
        }
        observed.push(damage.remaining_health);
        if damage.killed {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    if observed != expected_remaining_health {
        return Err(format!(
            "V1 health sequence diverged: expected {expected_remaining_health:?}, got {observed:?}"
        )
        .into());
    }
    let ServerMessage::ActorDestroy(destroy) = read_message(stream)? else {
        return Err("V1 expected ActorDestroy".into());
    };
    if destroy.actor_id != enemy_id {
        return Err("V1 destroyed the wrong enemy".into());
    }
    Ok(())
}

fn request_route_capability(
    stream: &mut TcpStream,
) -> Result<RouteState, Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::RouteStateRequest(RouteStateRequest {}),
    )?;
    let ServerMessage::RouteState(state) = read_message(stream)? else {
        return Err("expected RouteState capability response".into());
    };
    Ok(state)
}

fn request_matrix_route_capability(
    stream: &mut TcpStream,
) -> Result<RouteState, Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::RouteStateRequest(RouteStateRequest {}),
    )?;
    loop {
        match read_message(stream)? {
            ServerMessage::RouteState(state) => return Ok(state),
            ServerMessage::EquipmentChanged(_) => {}
            unexpected => {
                return Err(format!(
                    "matrix route capability received an unexpected message: {unexpected:?}"
                )
                .into())
            }
        }
    }
}

fn validate_route_state(
    state: &RouteState,
    phase: RoutePhase,
    player_id: u64,
    selection_absent: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !state.accepted
        || state.schema_version != 1
        || state.activity_id != "relay_awakening"
        || state.phase != phase
        || state.leader_actor_id != player_id
        || state.participant_actor_ids != [player_id]
        || state.capable_actor_ids != [player_id]
        || !state.all_capable
        || state.routes.len() != 2
        || state.routes[0].route_id != "breach"
        || state.routes[1].route_id != "stabilize"
        || state.routes.iter().any(|route| {
            route.events.len() != 2
                || !(3..=4).contains(&route.objective_path.len())
                || route.duration_budget_ms != 90_000
                || route.reward.item_id != "relay_core_fragment"
        })
        || (selection_absent && state.selection.is_some())
    {
        return Err(format!("route state diverged: {state:?}").into());
    }
    Ok(())
}

fn validate_matrix_route_state(
    state: &RouteState,
    phase: RoutePhase,
    player_id: u64,
    player_ids: &[u64],
    expected_players: usize,
    require_incomplete_capability: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_leader = player_ids
        .first()
        .copied()
        .ok_or("matrix route state has no admitted leader")?;
    let capability_shape_valid = if require_incomplete_capability {
        !state.all_capable
            && state.capable_actor_ids == [player_id]
            && expected_players == 2
            && player_id == expected_leader
    } else if phase == RoutePhase::ChoiceOpen {
        state.all_capable && state.capable_actor_ids == player_ids
    } else {
        state.capable_actor_ids.contains(&player_id)
            && state
                .capable_actor_ids
                .iter()
                .all(|actor_id| player_ids.contains(actor_id))
            && state.all_capable == (state.capable_actor_ids.len() == expected_players)
    };
    if !state.accepted
        || state.schema_version != 1
        || state.activity_id != "relay_awakening"
        || state.phase != phase
        || state.leader_actor_id != expected_leader
        || state.participant_actor_ids != player_ids
        || state.participant_actor_ids.len() != expected_players
        || !capability_shape_valid
        || state.routes.len() != 2
        || state.routes[0].route_id != "breach"
        || state.routes[1].route_id != "stabilize"
        || state.routes.iter().any(|route| {
            route.events.len() != 2
                || !(3..=4).contains(&route.objective_path.len())
                || route.duration_budget_ms != 90_000
                || route.reward.item_id != "relay_core_fragment"
        })
        || state.selection.is_some()
    {
        return Err(format!("matrix route state diverged: {state:?}").into());
    }
    Ok(())
}

fn verify_capability_mismatch(
    stream: &mut TcpStream,
    requested_route: RequestedRoute,
    player_id: u64,
    attack_cooldown_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let operation_id = "route-capability-mismatch";
    write_message(
        stream,
        &ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
            operation_id: operation_id.to_owned(),
            route_id: requested_route.id().to_owned(),
        }),
    )?;
    let ServerMessage::RouteChoiceResult(rejected) = read_message(stream)? else {
        return Err("capability mismatch expected RouteChoiceResult".into());
    };
    if rejected.accepted
        || rejected.replayed
        || rejected.operation_id != operation_id
        || rejected.selection.is_some()
        || !(rejected.message.contains("capable") || rejected.message.contains("capability"))
    {
        return Err(format!("capability mismatch changed route state: {rejected:?}").into());
    }
    complete_activity(stream, player_id, attack_cooldown_ms)?;
    println!(
        "M27_ROUTE_CAPABILITY_MISMATCH {}",
        serde_json::json!({
            "schema": "RevenantM27CapabilityMismatchV1",
            "route_id": requested_route.id(),
            "accepted": false,
            "completed_baseline": true,
        })
    );
    Ok(())
}

fn verify_route_timeout(
    stream: &mut TcpStream,
    selection: &RouteSelection,
) -> Result<(), Box<dyn std::error::Error>> {
    thread::sleep(Duration::from_millis(
        selection
            .duration_budget_ms
            .checked_add(150)
            .ok_or("route timeout wait overflowed")?,
    ));
    write_message(
        stream,
        &ClientMessage::RouteStateRequest(RouteStateRequest {}),
    )?;
    let ServerMessage::ObjectiveUpdate(failed) = read_message(stream)? else {
        return Err("route timeout expected a failed ObjectiveUpdate".into());
    };
    let ServerMessage::RouteOperationSummary(summary) = read_message(stream)? else {
        return Err("route timeout expected RouteOperationSummary".into());
    };
    if failed.state != "Failed"
        || summary.outcome != RouteTerminalOutcome::FailedTimeout
        || summary.route_id != selection.route_id
        || summary.seed != selection.seed
        || summary.event_id != selection.event_id
        || summary.effect != selection.effect
        || summary.objective_path != selection.objective_path
        || summary.elapsed_ms <= summary.duration_budget_ms
        || summary.reward.is_some()
        || !summary.grants.is_empty()
        || summary.no_reward_reason.as_deref() != Some("deadline_exceeded")
        || summary.transitions.last().is_none_or(|transition| {
            route_objective_label(transition.objective_id) != failed.objective_id
                || transition.to != RouteObjectiveState::Failed
        })
    {
        return Err(format!("route timeout terminal truth diverged: {summary:?}").into());
    }
    println!(
        "M27_ROUTE_TIMEOUT {}",
        serde_json::json!({
            "schema": "RevenantM27RouteTimeoutV1",
            "route_id": summary.route_id,
            "event_id": summary.event_id,
            "seed": summary.seed,
            "elapsed_ms": summary.elapsed_ms,
            "duration_budget_ms": summary.duration_budget_ms,
            "failed_objective": failed.objective_id,
            "transition_count": summary.transitions.len(),
            "completed": false,
            "rewarded": false,
        })
    );
    Ok(())
}

const fn route_objective_label(objective_id: RouteObjectiveId) -> &'static str {
    match objective_id {
        RouteObjectiveId::ClearDroneGroup => "clear_drone_group",
        RouteObjectiveId::ReachRelayStabilizer => "reach_relay_stabilizer",
        RouteObjectiveId::ReachRelayDoor => "reach_relay_door",
        RouteObjectiveId::DefeatWarden => "defeat_warden",
    }
}

fn choose_route(
    stream: &mut TcpStream,
    requested_route: RequestedRoute,
    player_id: u64,
    account_id: &str,
) -> Result<RouteSelection, Box<dyn std::error::Error>> {
    let operation_id = "route-bot-choice";
    let intent = RouteChoiceIntent {
        operation_id: operation_id.to_owned(),
        route_id: requested_route.id().to_owned(),
    };
    write_message(stream, &ClientMessage::RouteChoiceIntent(intent.clone()))?;
    if requested_route == RequestedRoute::Stabilize {
        let ServerMessage::ObjectiveUpdate(door) = read_message(stream)? else {
            return Err("stabilize choice expected pending door objective".into());
        };
        let ServerMessage::ObjectiveUpdate(stabilizer) = read_message(stream)? else {
            return Err("stabilize choice expected active stabilizer objective".into());
        };
        if door.objective_id != "reach_relay_door"
            || door.state != "Pending"
            || stabilizer.objective_id != "reach_relay_stabilizer"
            || stabilizer.state != "Active"
        {
            return Err("stabilize objective projection diverged".into());
        }
    }
    let ServerMessage::RouteChoiceResult(accepted) = read_message(stream)? else {
        return Err("expected accepted RouteChoiceResult".into());
    };
    let selection = accepted
        .selection
        .clone()
        .ok_or("accepted route result omitted its selection")?;
    if !accepted.accepted
        || accepted.replayed
        || accepted.operation_id != operation_id
        || selection.leader_actor_id != player_id
        || selection.participant_actor_ids != [player_id]
        || selection.route_id != requested_route.id()
        || selection.seed > i64::MAX.unsigned_abs()
        || selection.duration_budget_ms != 90_000
        || selection.reward.item_id != "relay_core_fragment"
        || selection.objective_path.len()
            != if requested_route == RequestedRoute::Breach {
                3
            } else {
                4
            }
    {
        return Err(format!("accepted route selection diverged: {accepted:?}").into());
    }
    let ServerMessage::RouteState(committed) = read_message(stream)? else {
        return Err("expected committed RouteState".into());
    };
    validate_route_state(&committed, RoutePhase::Routed, player_id, false)?;
    if committed.selection.as_ref() != Some(&selection) {
        return Err("committed route state disagrees with accepted selection".into());
    }

    write_message(stream, &ClientMessage::RouteChoiceIntent(intent.clone()))?;
    let ServerMessage::RouteChoiceResult(retry) = read_message(stream)? else {
        return Err("expected replayed RouteChoiceResult".into());
    };
    if !retry.accepted || !retry.replayed || retry.selection.as_ref() != Some(&selection) {
        return Err(format!("route retry diverged: {retry:?}").into());
    }

    write_message(
        stream,
        &ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
            operation_id: operation_id.to_owned(),
            route_id: requested_route.other().to_owned(),
        }),
    )?;
    let ServerMessage::RouteChoiceResult(conflict) = read_message(stream)? else {
        return Err("expected conflicting RouteChoiceResult".into());
    };
    if conflict.accepted || conflict.replayed || conflict.selection.is_some() {
        return Err(format!("route conflict exposed accepted state: {conflict:?}").into());
    }
    println!(
        "route {} selected for {} with server event {}",
        selection.route_id, account_id, selection.event_id
    );
    Ok(selection)
}

#[allow(clippy::too_many_lines)]
fn complete_route_activity(
    stream: &mut TcpStream,
    player_id: u64,
    attack_cooldown_ms: u64,
    requested_route: RequestedRoute,
    selection: &RouteSelection,
) -> Result<(), Box<dyn std::error::Error>> {
    if requested_route == RequestedRoute::Stabilize {
        write_message(
            stream,
            &ClientMessage::MoveIntent(MoveIntent {
                position: [3, 0, 3],
            }),
        )?;
        let ServerMessage::ActorUpdate(update) = read_message(stream)? else {
            return Err("stabilize route expected ActorUpdate".into());
        };
        let ServerMessage::ObjectiveUpdate(stabilizer) = read_message(stream)? else {
            return Err("stabilize route expected completed stabilizer".into());
        };
        let ServerMessage::ObjectiveUpdate(door) = read_message(stream)? else {
            return Err("stabilize route expected active door".into());
        };
        let ServerMessage::RouteState(state) = read_message(stream)? else {
            return Err("stabilize route expected refreshed RouteState".into());
        };
        if update.actor_id != player_id
            || stabilizer.objective_id != "reach_relay_stabilizer"
            || stabilizer.state != "Completed"
            || door.objective_id != "reach_relay_door"
            || door.state != "Active"
        {
            return Err("stabilizer transition diverged".into());
        }
        validate_route_state(&state, RoutePhase::Routed, player_id, false)?;
    }

    write_message(
        stream,
        &ClientMessage::MoveIntent(MoveIntent {
            position: [6, 0, 0],
        }),
    )?;
    let ServerMessage::ActorUpdate(update) = read_message(stream)? else {
        return Err("routed door expected ActorUpdate".into());
    };
    let ServerMessage::ObjectiveUpdate(door) = read_message(stream)? else {
        return Err("routed door expected completed objective".into());
    };
    let ServerMessage::ObjectiveUpdate(warden_objective) = read_message(stream)? else {
        return Err("routed door expected active Warden objective".into());
    };
    let ServerMessage::DoorState(door_state) = read_message(stream)? else {
        return Err("routed door expected DoorState".into());
    };
    let ServerMessage::ActorSpawn(warden) = read_message(stream)? else {
        return Err("routed door expected Warden spawn".into());
    };
    if update.actor_id != player_id
        || door.objective_id != "reach_relay_door"
        || door.state != "Completed"
        || warden_objective.objective_id != "defeat_warden"
        || warden_objective.state != "Active"
        || !door_state.open
        || warden.archetype != "warden"
        || warden.health == 0
        || warden.health != warden.max_health
    {
        return Err("routed Warden transition diverged".into());
    }
    defeat_enemy(stream, warden.actor_id, attack_cooldown_ms)?;
    let ServerMessage::ObjectiveUpdate(completed_warden) = read_message(stream)? else {
        return Err("routed completion expected completed Warden objective".into());
    };
    let ServerMessage::LootGranted(loot) = read_message(stream)? else {
        return Err("routed completion expected LootGranted".into());
    };
    let ServerMessage::ProgressionGranted(progression) = read_message(stream)? else {
        return Err("routed completion expected ProgressionGranted".into());
    };
    let ServerMessage::ActivityComplete(activity) = read_message(stream)? else {
        return Err("routed completion expected ActivityComplete".into());
    };
    let ServerMessage::RouteOperationSummary(summary) = read_message(stream)? else {
        return Err("routed completion expected RouteOperationSummary".into());
    };
    if completed_warden.objective_id != "defeat_warden"
        || completed_warden.state != "Completed"
        || loot.item_id != selection.reward.item_id
        || loot.quantity != selection.reward.item_quantity
        || progression.experience_granted != selection.reward.experience
        || activity.activity_id != "relay_awakening"
        || summary.outcome != RouteTerminalOutcome::Succeeded
        || summary.route_id != selection.route_id
        || summary.seed != selection.seed
        || summary.event_id != selection.event_id
        || summary.effect != selection.effect
        || summary.objective_path != selection.objective_path
        || summary.elapsed_ms > summary.duration_budget_ms
        || summary.reward.as_ref() != Some(&selection.reward)
        || summary.grants.len() != 1
        || summary.grants[0].actor_id != player_id
        || summary.grants[0].item_quantity != selection.reward.item_quantity
        || summary.grants[0].experience != selection.reward.experience
        || summary.no_reward_reason.is_some()
        || summary.transitions.len() != selection.objective_path.len() * 2 - 1
    {
        return Err(format!("routed terminal truth diverged: {summary:?}").into());
    }
    println!(
        "M27_ROUTE_FLOW {}",
        serde_json::json!({
            "schema": "RevenantM27GatewayRouteFlowV1",
            "route_id": summary.route_id,
            "event_id": summary.event_id,
            "seed": summary.seed,
            "elapsed_ms": summary.elapsed_ms,
            "reward_item_quantity": selection.reward.item_quantity,
            "reward_experience": selection.reward.experience,
            "transition_count": summary.transitions.len(),
            "completed": true,
        })
    );
    Ok(())
}

fn complete_activity(
    stream: &mut TcpStream,
    player_id: u64,
    attack_cooldown_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::MoveIntent(MoveIntent {
            position: [6, 0, 0],
        }),
    )?;
    let ServerMessage::ActorUpdate(player_update) = read_message(stream)? else {
        return Err("expected player ActorUpdate".into());
    };
    if player_update.actor_id != player_id {
        return Err("server updated the wrong player actor".into());
    }
    let ServerMessage::ObjectiveUpdate(reach) = read_message(stream)? else {
        return Err("expected completed ReachArea".into());
    };
    let ServerMessage::ObjectiveUpdate(boss_objective) = read_message(stream)? else {
        return Err("expected active Boss objective".into());
    };
    let ServerMessage::DoorState(door) = read_message(stream)? else {
        return Err("expected open door".into());
    };
    if reach.state != "Completed" || boss_objective.state != "Active" || !door.open {
        return Err("activity did not open the boss stage".into());
    }
    let ServerMessage::ActorSpawn(boss) = read_message(stream)? else {
        return Err("expected boss ActorSpawn".into());
    };
    if boss.archetype != "warden" {
        return Err("unexpected boss archetype".into());
    }
    println!(
        "door {} opened; boss {} spawned",
        door.door_id, boss.archetype
    );
    defeat_enemy(stream, boss.actor_id, attack_cooldown_ms)?;
    let ServerMessage::ObjectiveUpdate(completed_boss) = read_message(stream)? else {
        return Err("expected completed Boss objective".into());
    };
    let ServerMessage::LootGranted(loot) = read_message(stream)? else {
        return Err("expected LootGranted".into());
    };
    if loot.item_id != "relay_core_fragment" || loot.quantity != 1 {
        return Err("activity granted unexpected loot".into());
    }
    let ServerMessage::ProgressionGranted(progression) = read_message(stream)? else {
        return Err("expected ProgressionGranted".into());
    };
    if progression.experience_granted != 100 || progression.level < progression.previous_level {
        return Err("activity granted unexpected progression".into());
    }
    let ServerMessage::ActivityComplete(completed) = read_message(stream)? else {
        return Err("expected ActivityComplete".into());
    };
    if completed_boss.state != "Completed" {
        return Err("boss objective did not complete".into());
    }
    println!(
        "activity {} completed; received {} x{} (total {})",
        completed.activity_id, loot.item_id, loot.quantity, loot.resulting_quantity
    );
    println!(
        "received {} XP (total {}, level {})",
        progression.experience_granted, progression.experience, progression.level
    );
    Ok(())
}

fn verify_active_module_rejection(
    stream: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let rejected = request_combination(stream, "active-blocked", "module_force_matrix")?;
    if rejected.accepted
        || rejected.replayed
        || !rejected
            .message
            .contains("locked until activity completion")
    {
        return Err(format!("active module mutation was not rejected: {rejected:?}").into());
    }
    println!("active module mutation rejected without disconnect");
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run_module_mutation_flow(
    stream: &mut TcpStream,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::ModuleStateRequest(ModuleStateRequest {}),
    )?;
    let ServerMessage::ModuleSnapshot(initial) = read_message(stream)? else {
        return Err("expected initial ModuleSnapshot".into());
    };
    validate_module_snapshot(&initial)?;
    if initial.fragments < 5
        || !initial.owned_modules.is_empty()
        || !initial.equipped_modules.is_empty()
        || initial.loadout_revision != 0
    {
        return Err(format!("initial module state diverged: {initial:?}").into());
    }

    let requested_modules = vec![
        "module_ward_capacitor".to_owned(),
        "module_force_matrix".to_owned(),
    ];
    write_message(
        stream,
        &ClientMessage::ModulePreviewRequest(ModulePreviewRequest {
            modules: requested_modules.clone(),
        }),
    )?;
    let ServerMessage::ModulePreview(preview) = read_message(stream)? else {
        return Err("expected accepted ModulePreview".into());
    };
    if !preview.accepted
        || preview.max_health != 120
        || preview.requested_modules
            != [
                "module_force_matrix".to_owned(),
                "module_ward_capacitor".to_owned(),
            ]
        || !preview.weapons.iter().any(|profile| {
            profile.item_id == "pulse_rifle"
                && profile.effective_damage == 48
                && profile.effective_cooldown_ms == 325
        })
    {
        return Err(format!("module preview diverged: {preview:?}").into());
    }
    write_message(
        stream,
        &ClientMessage::ModulePreviewRequest(ModulePreviewRequest {
            modules: vec![
                "module_force_matrix".to_owned(),
                "module_force_matrix".to_owned(),
            ],
        }),
    )?;
    let ServerMessage::ModulePreview(duplicate_preview) = read_message(stream)? else {
        return Err("expected rejected ModulePreview".into());
    };
    if duplicate_preview.accepted
        || !duplicate_preview.weapons.is_empty()
        || duplicate_preview.max_health != 0
    {
        return Err(format!("duplicate preview was not bounded: {duplicate_preview:?}").into());
    }

    let malformed = request_combination(stream, "invalid_id", "module_force_matrix")?;
    if malformed.accepted || malformed.state != initial {
        return Err(format!("malformed operation changed state: {malformed:?}").into());
    }
    let unknown = request_combination(stream, "combine-unknown", "module_unknown")?;
    if unknown.accepted || unknown.state != initial {
        return Err(format!("unknown module changed state: {unknown:?}").into());
    }

    let force = request_combination(stream, "combine-force", "module_force_matrix")?;
    if !force.accepted
        || force.replayed
        || force.state.fragments != initial.fragments - 2
        || force.state.owned_modules != ["module_force_matrix".to_owned()]
    {
        return Err(format!("force combination diverged: {force:?}").into());
    }
    let force_retry = request_combination(stream, "combine-force", "module_force_matrix")?;
    if !force_retry.accepted || !force_retry.replayed || force_retry.state != force.state {
        return Err(format!("force retry diverged: {force_retry:?}").into());
    }
    let conflict = request_combination(stream, "combine-force", "module_ward_capacitor")?;
    if conflict.accepted || conflict.state != force.state {
        return Err(format!("idempotency conflict changed state: {conflict:?}").into());
    }
    let already_owned = request_combination(stream, "combine-force-again", "module_force_matrix")?;
    if already_owned.accepted || already_owned.state != force.state {
        return Err(format!("already-owned rejection changed state: {already_owned:?}").into());
    }
    let ward = request_combination(stream, "combine-ward", "module_ward_capacitor")?;
    if !ward.accepted
        || ward.replayed
        || ward.state.fragments != initial.fragments - 4
        || ward.state.owned_modules.len() != 2
    {
        return Err(format!("ward combination diverged: {ward:?}").into());
    }
    let insufficient = request_combination(stream, "combine-tempo", "module_tempo_regulator")?;
    if insufficient.accepted || insufficient.state != ward.state {
        return Err(
            format!("insufficient-fragment rejection changed state: {insufficient:?}").into(),
        );
    }

    let stale = request_loadout(stream, "loadout-stale", u64::MAX, &requested_modules)?;
    if stale.accepted || stale.state != ward.state {
        return Err(format!("stale loadout changed state: {stale:?}").into());
    }
    let fourth = request_loadout(
        stream,
        "loadout-fourth",
        0,
        &[
            "module_force_matrix".to_owned(),
            "module_tempo_regulator".to_owned(),
            "module_reach_lattice".to_owned(),
            "module_ward_capacitor".to_owned(),
        ],
    )?;
    if fourth.accepted || fourth.state != ward.state {
        return Err(format!("four-entry loadout changed state: {fourth:?}").into());
    }
    let loadout = request_loadout(stream, "loadout-force-ward", 0, &requested_modules)?;
    if !loadout.accepted
        || loadout.replayed
        || loadout.state.loadout_revision != 1
        || loadout.state.max_health != 120
    {
        return Err(format!("module loadout diverged: {loadout:?}").into());
    }
    let loadout_retry = request_loadout(stream, "loadout-force-ward", 0, &requested_modules)?;
    if !loadout_retry.accepted || !loadout_retry.replayed || loadout_retry.state != loadout.state {
        return Err(format!("loadout retry diverged: {loadout_retry:?}").into());
    }
    let unchanged = request_loadout(stream, "loadout-unchanged", 1, &requested_modules)?;
    if unchanged.accepted || unchanged.state != loadout.state {
        return Err(format!("unchanged loadout changed state: {unchanged:?}").into());
    }

    write_message(
        stream,
        &ClientMessage::ModuleStateRequest(ModuleStateRequest {}),
    )?;
    let ServerMessage::ModuleSnapshot(final_state) = read_message(stream)? else {
        return Err("expected final ModuleSnapshot".into());
    };
    if final_state != loadout.state {
        return Err("final module state disagrees with accepted loadout".into());
    }
    println!(
        "M26_MODULE_FLOW {}",
        serde_json::json!({
            "schema": "RevenantM26GatewayFlowV1",
            "account_id": account_id,
            "initial_fragments": initial.fragments,
            "resulting_fragments": final_state.fragments,
            "owned_modules": final_state.owned_modules,
            "equipped_modules": final_state.equipped_modules,
            "loadout_revision": final_state.loadout_revision,
            "max_health_next_admission": final_state.max_health,
            "force_retry_replayed": force_retry.replayed,
            "loadout_retry_replayed": loadout_retry.replayed,
        })
    );
    Ok(())
}

fn request_combination(
    stream: &mut TcpStream,
    operation_id: &str,
    module_id: &str,
) -> Result<revenant_protocol::ModuleCombined, Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::ModuleCombineIntent(ModuleCombineIntent {
            operation_id: operation_id.to_owned(),
            module_id: module_id.to_owned(),
        }),
    )?;
    let ServerMessage::ModuleCombined(result) = read_message(stream)? else {
        return Err("expected ModuleCombined".into());
    };
    Ok(result)
}

fn request_loadout(
    stream: &mut TcpStream,
    operation_id: &str,
    expected_revision: u64,
    modules: &[String],
) -> Result<revenant_protocol::ModuleLoadoutChanged, Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::ModuleLoadoutIntent(ModuleLoadoutIntent {
            operation_id: operation_id.to_owned(),
            expected_revision,
            modules: modules.to_vec(),
        }),
    )?;
    let ServerMessage::ModuleLoadoutChanged(result) = read_message(stream)? else {
        return Err("expected ModuleLoadoutChanged".into());
    };
    Ok(result)
}

fn validate_module_snapshot(snapshot: &ModuleSnapshot) -> Result<(), Box<dyn std::error::Error>> {
    if snapshot.catalog_revision != "m26-v1"
        || snapshot.catalog.len() != 4
        || snapshot.weapons.len() != 2
        || snapshot.maximum_slots != 3
        || snapshot.owned_modules.len() > 4
        || snapshot.equipped_modules.len() > 3
    {
        return Err(format!("module snapshot shape diverged: {snapshot:?}").into());
    }
    Ok(())
}

fn verify_objective_progression(stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let ServerMessage::ObjectiveUpdate(kill) = read_message(stream)? else {
        return Err("expected completed KillActors objective".into());
    };
    let ServerMessage::ObjectiveUpdate(reach) = read_message(stream)? else {
        return Err("expected active ReachArea objective".into());
    };
    if kill.objective_type != "KillActors" || kill.state != "Completed" {
        return Err("KillActors objective did not complete".into());
    }
    if reach.objective_type != "ReachArea" || reach.state != "Active" {
        return Err("ReachArea objective did not open".into());
    }
    println!(
        "objective {} completed; next objective {} is active",
        kill.objective_id, reach.objective_id
    );
    Ok(())
}

fn observe_enemy_ai(
    stream: &mut TcpStream,
    enemy_id: u64,
) -> Result<DamageApplied, Box<dyn std::error::Error>> {
    let mut moved = false;
    loop {
        match read_message(stream)? {
            ServerMessage::ActorUpdate(update) if update.actor_id == enemy_id => {
                moved = true;
                println!("enemy actor {enemy_id} chased to {:?}", update.position);
            }
            ServerMessage::DamageApplied(damage) if damage.source_actor_id == enemy_id => {
                if !moved {
                    return Err("enemy attacked without a chase update".into());
                }
                println!(
                    "enemy actor {enemy_id} dealt {} damage; player has {} HP",
                    damage.damage, damage.remaining_health
                );
                return Ok(damage);
            }
            _ => return Err("unexpected message while observing enemy AI".into()),
        }
    }
}

fn receive_matrix_equipment(
    stream: &mut TcpStream,
    player_id: u64,
    requested_weapon: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut rejected_non_weapon = false;
    loop {
        let ServerMessage::EquipmentChanged(equipment) = read_message(stream)? else {
            return Err("expected matrix EquipmentChanged".into());
        };
        if equipment.actor_id != player_id {
            continue;
        }
        if !equipment.accepted {
            if !equipment.message.contains("not an equipable weapon") {
                return Err(format!(
                    "matrix equipment rejection was unexpected: {}",
                    equipment.message
                )
                .into());
            }
            rejected_non_weapon = true;
            continue;
        }
        if !rejected_non_weapon {
            return Err("matrix weapon was accepted before the invalid-item boundary".into());
        }
        if equipment.equipped_weapon_item_id != requested_weapon || equipment.cooldown_ms == 0 {
            return Err(format!("matrix weapon equip diverged: {equipment:?}").into());
        }
        println!(
            "equipped authoritative matrix weapon {} with {} ms cooldown",
            equipment.equipped_weapon_item_id, equipment.cooldown_ms
        );
        return Ok(equipment.cooldown_ms);
    }
}

#[allow(clippy::too_many_lines)]
fn run_route_matrix_activity(
    mut stream: TcpStream,
    config: MatrixConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let requested_route = config
        .requested_route
        .ok_or("route matrix is missing its requested route")?;
    if config.expected_players == 1 && config.role != MatrixRole::Primary {
        return Err("a solo route matrix requires the primary role".into());
    }
    if config.expected_players == 2 && config.player_ids.len() != 2 {
        return Err("a two-active route matrix requires two replicated players".into());
    }
    let weapon_damage = config
        .module_build
        .weapon_damage(&config.requested_weapon)
        .ok_or("route matrix weapon is not catalogued")?;
    let allowed_weapon_damage = matrix_weapon_damage_values(&config.requested_weapon)
        .ok_or("route matrix weapon damage set is not catalogued")?;
    let attack_interval_ms = config.attack_cooldown_ms.max(config.simulated_rtt_ms);
    let role_phase_ms = if config.role == MatrixRole::Secondary {
        attack_interval_ms / 2
    } else {
        0
    };
    let initial_delay =
        MATRIX_SYNC_DELAY + Duration::from_millis(config.simulated_rtt_ms / 2 + role_phase_ms);
    let drone_health = scaled_matrix_health(140, config.expected_players)?;
    let mut drone = Some(EncounterTracker::new(
        "relay_drone",
        config.first_enemy_id,
        drone_health,
    ));
    if config.initial_pressure.source_actor_id != config.first_enemy_id
        || !config
            .player_ids
            .contains(&config.initial_pressure.target_actor_id)
        || config.initial_pressure.damage != 10
        || config.initial_pressure.killed
    {
        return Err(format!(
            "route matrix initial hostile pressure diverged: {:?}",
            config.initial_pressure
        )
        .into());
    }
    drone
        .as_mut()
        .expect("route matrix drone tracker must exist")
        .hostile_hit(&config.initial_pressure);
    let mut warden: Option<EncounterTracker> = None;
    let mut player_health = config
        .player_ids
        .iter()
        .copied()
        .map(|player_id| (player_id, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let mut current_player_health = config
        .player_ids
        .iter()
        .copied()
        .map(|player_id| {
            let maximum_health = if player_id == config.player_id {
                config.module_build.maximum_health()
            } else {
                0
            };
            (player_id, maximum_health)
        })
        .collect::<BTreeMap<_, _>>();
    record_matrix_player_damage(
        &config.initial_pressure,
        &mut current_player_health,
        &mut player_health,
    )?;

    let (incoming_tx, incoming_rx) = mpsc::channel();
    let mut reader = stream.try_clone()?;
    thread::spawn(move || loop {
        let message = read_message(&mut reader).map_err(|error| error.to_string());
        let terminal = message.is_err();
        if incoming_tx.send(message).is_err() || terminal {
            break;
        }
    });

    let operation_id = "route-matrix-choice";
    let mut active_enemy_id = Some(config.first_enemy_id);
    let mut next_attack_at = Some(Instant::now() + initial_delay);
    let mut route_options = None;
    let mut selection: Option<RouteSelection> = None;
    let mut route_state_committed = false;
    let mut leader_choice_sent = false;
    let mut non_leader_choice_sent = false;
    let mut non_leader_rejected = config.expected_players == 1;
    let mut retry_sent = false;
    let mut retry_replayed = false;
    let mut conflict_sent = false;
    let mut conflict_rejected = false;
    let mut stabilizer_active = false;
    let mut stabilizer_movement_sent = false;
    let mut door_active = false;
    let mut door_movement_sent = false;
    let mut activity_completed = false;
    let mut loot_quantity = 0_u32;
    let mut progression_granted = 0_u64;
    let mut terminal_summary = None;

    loop {
        if next_attack_at.is_some_and(|deadline| Instant::now() >= deadline) {
            let target_actor_id = active_enemy_id
                .ok_or("route matrix attack deadline remained after encounter termination")?;
            write_message(
                &mut stream,
                &ClientMessage::AttackIntent(AttackIntent { target_actor_id }),
            )?;
            next_attack_at = None;
        }
        let wait = next_attack_at.map_or(MATRIX_EVENT_POLL, |deadline| {
            deadline
                .saturating_duration_since(Instant::now())
                .min(MATRIX_EVENT_POLL)
        });
        let message = match incoming_rx.recv_timeout(wait) {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => return Err(format!("route matrix receive failed: {error}").into()),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err("route matrix receive worker disconnected".into())
            }
        };
        let received_at = Instant::now();
        match message {
            ServerMessage::DamageApplied(damage) => {
                let enemy_id = active_enemy_id.ok_or_else(|| {
                    format!("route matrix damage arrived without an active enemy: {damage:?}")
                })?;
                let tracker =
                    matrix_tracker_mut(enemy_id, config.first_enemy_id, &mut drone, &mut warden)?;
                if damage.source_actor_id == enemy_id {
                    if !config.player_ids.contains(&damage.target_actor_id) || damage.killed {
                        return Err(
                            format!("route matrix hostile damage diverged: {damage:?}").into()
                        );
                    }
                    tracker.hostile_hit(&damage);
                    record_matrix_player_damage(
                        &damage,
                        &mut current_player_health,
                        &mut player_health,
                    )?;
                } else if damage.target_actor_id == enemy_id {
                    if !config.player_ids.contains(&damage.source_actor_id)
                        || if damage.source_actor_id == config.player_id {
                            damage.damage != weapon_damage
                        } else {
                            !allowed_weapon_damage.contains(&damage.damage)
                        }
                    {
                        return Err(
                            format!("route matrix player damage diverged: {damage:?}").into()
                        );
                    }
                    tracker.player_hit(&damage, received_at);
                    if damage.killed {
                        active_enemy_id = None;
                        next_attack_at = None;
                    } else if damage.source_actor_id == config.player_id {
                        next_attack_at =
                            Some(received_at + Duration::from_millis(attack_interval_ms));
                    }
                } else {
                    return Err(format!(
                        "route matrix damage missed the active exchange: {damage:?}"
                    )
                    .into());
                }
            }
            ServerMessage::ActorDestroy(destroy) => {
                if destroy.actor_id != config.first_enemy_id
                    && warden
                        .as_ref()
                        .is_none_or(|tracker| tracker.enemy_id != destroy.actor_id)
                {
                    return Err(format!(
                        "route matrix destroyed an unexpected actor {}",
                        destroy.actor_id
                    )
                    .into());
                }
                active_enemy_id = None;
                next_attack_at = None;
            }
            ServerMessage::ObjectiveUpdate(objective) => {
                if objective.objective_id == "reach_relay_stabilizer" && objective.state == "Active"
                {
                    stabilizer_active = true;
                }
                if objective.objective_id == "reach_relay_door" && objective.state == "Active" {
                    door_active = true;
                }
            }
            ServerMessage::RouteState(state) if state.phase == RoutePhase::Drone => {
                validate_matrix_route_state(
                    &state,
                    RoutePhase::Drone,
                    config.player_id,
                    &config.player_ids,
                    config.expected_players,
                    false,
                )?;
                route_options = Some(state.routes);
            }
            ServerMessage::RouteState(state) if state.phase == RoutePhase::ChoiceOpen => {
                validate_matrix_route_state(
                    &state,
                    RoutePhase::ChoiceOpen,
                    config.player_id,
                    &config.player_ids,
                    config.expected_players,
                    false,
                )?;
                route_options = Some(state.routes);
                if config.role == MatrixRole::Secondary && !non_leader_choice_sent {
                    write_message(
                        &mut stream,
                        &ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                            operation_id: "route-matrix-nonleader".to_owned(),
                            route_id: requested_route.id().to_owned(),
                        }),
                    )?;
                    non_leader_choice_sent = true;
                } else if config.role == MatrixRole::Primary && !leader_choice_sent {
                    if config.expected_players == 2 {
                        thread::sleep(Duration::from_millis(300));
                    }
                    write_message(
                        &mut stream,
                        &ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                            operation_id: operation_id.to_owned(),
                            route_id: requested_route.id().to_owned(),
                        }),
                    )?;
                    leader_choice_sent = true;
                }
            }
            ServerMessage::RouteState(state) if state.phase == RoutePhase::Routed => {
                let committed = state
                    .selection
                    .as_ref()
                    .ok_or("committed route state omitted selection")?;
                if !state.accepted
                    || state.schema_version != 1
                    || state.leader_actor_id != config.player_ids[0]
                    || state.participant_actor_ids != config.player_ids
                    || !state.all_capable
                    || state.capable_actor_ids != config.player_ids
                    || state.routes.len() != 2
                    || committed.route_id != requested_route.id()
                    || selection.as_ref().is_some_and(|value| value != committed)
                {
                    return Err(format!("committed route matrix state diverged: {state:?}").into());
                }
                selection = Some(committed.clone());
                route_state_committed = true;
            }
            ServerMessage::RouteChoiceResult(result) => {
                if result.accepted {
                    let accepted = result
                        .selection
                        .as_ref()
                        .ok_or("accepted route matrix result omitted selection")?;
                    validate_matrix_selection(
                        accepted,
                        requested_route,
                        &config.player_ids,
                        route_options.as_deref(),
                    )?;
                    if selection.as_ref().is_some_and(|value| value != accepted) {
                        return Err("route matrix accepted selections disagreed".into());
                    }
                    selection = Some(accepted.clone());
                    if config.role == MatrixRole::Primary {
                        if !result.replayed && !retry_sent {
                            write_message(
                                &mut stream,
                                &ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                                    operation_id: operation_id.to_owned(),
                                    route_id: requested_route.id().to_owned(),
                                }),
                            )?;
                            retry_sent = true;
                        } else if result.replayed && !conflict_sent {
                            retry_replayed = true;
                            write_message(
                                &mut stream,
                                &ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                                    operation_id: operation_id.to_owned(),
                                    route_id: requested_route.other().to_owned(),
                                }),
                            )?;
                            conflict_sent = true;
                        }
                    }
                } else if result.operation_id == "route-matrix-nonleader"
                    && config.role == MatrixRole::Secondary
                    && result.selection.is_none()
                    && (result.message.contains("leader")
                        || result.message.contains("first admitted participant"))
                {
                    non_leader_rejected = true;
                } else if result.operation_id == operation_id
                    && config.role == MatrixRole::Primary
                    && conflict_sent
                    && result.selection.is_none()
                {
                    conflict_rejected = true;
                } else {
                    return Err(format!("unexpected route matrix rejection: {result:?}").into());
                }
            }
            ServerMessage::ActorSpawn(actor) if actor.archetype == "warden" => {
                if warden.is_some() || actor.health == 0 || actor.health != actor.max_health {
                    return Err(format!("route matrix Warden spawn diverged: {actor:?}").into());
                }
                warden = Some(EncounterTracker::new(
                    "warden",
                    actor.actor_id,
                    actor.health,
                ));
                active_enemy_id = Some(actor.actor_id);
                next_attack_at = Some(Instant::now() + initial_delay);
            }
            ServerMessage::LootGranted(loot) => {
                let selected = selection
                    .as_ref()
                    .ok_or("route matrix received loot before selection")?;
                if loot.item_id != selected.reward.item_id
                    || loot.quantity != selected.reward.item_quantity
                    || loot_quantity != 0
                {
                    return Err(format!("route matrix loot diverged: {loot:?}").into());
                }
                loot_quantity = loot.quantity;
            }
            ServerMessage::ProgressionGranted(progression) => {
                let selected = selection
                    .as_ref()
                    .ok_or("route matrix received progression before selection")?;
                if progression.experience_granted != selected.reward.experience
                    || progression.level < progression.previous_level
                    || progression_granted != 0
                {
                    return Err(
                        format!("route matrix progression diverged: {progression:?}").into(),
                    );
                }
                progression_granted = progression.experience_granted;
            }
            ServerMessage::ActivityComplete(activity) => {
                if activity.activity_id != "relay_awakening" {
                    return Err(format!("route matrix activity diverged: {activity:?}").into());
                }
                activity_completed = true;
            }
            ServerMessage::RouteOperationSummary(summary) => {
                let selected = selection
                    .as_ref()
                    .ok_or("route matrix received terminal before selection")?;
                if summary.outcome != RouteTerminalOutcome::Succeeded
                    || summary.route_id != selected.route_id
                    || summary.seed != selected.seed
                    || summary.event_id != selected.event_id
                    || summary.effect != selected.effect
                    || summary.objective_path != selected.objective_path
                    || summary.elapsed_ms > summary.duration_budget_ms
                    || summary.reward.as_ref() != Some(&selected.reward)
                    || summary.grants.len() != config.expected_players
                    || !summary
                        .grants
                        .iter()
                        .any(|grant| grant.actor_id == config.player_id)
                    || summary.no_reward_reason.is_some()
                    || summary.transitions.len() != selected.objective_path.len() * 2 - 1
                {
                    return Err(format!("route matrix terminal diverged: {summary:?}").into());
                }
                terminal_summary = Some(summary);
            }
            ServerMessage::ActorUpdate(_)
            | ServerMessage::DoorState(_)
            | ServerMessage::EquipmentChanged(_) => {}
            unexpected => {
                return Err(
                    format!("unexpected route matrix runtime message: {unexpected:?}").into(),
                )
            }
        }

        let choice_checks_complete = match config.role {
            MatrixRole::Primary => retry_replayed && conflict_rejected,
            MatrixRole::Secondary => non_leader_rejected,
        };
        if route_state_committed && choice_checks_complete {
            if requested_route == RequestedRoute::Stabilize
                && stabilizer_active
                && !stabilizer_movement_sent
            {
                write_message(
                    &mut stream,
                    &ClientMessage::MoveIntent(MoveIntent {
                        position: [3, 0, 3],
                    }),
                )?;
                stabilizer_movement_sent = true;
            } else if door_active
                && !door_movement_sent
                && (requested_route == RequestedRoute::Breach || stabilizer_movement_sent)
            {
                write_message(
                    &mut stream,
                    &ClientMessage::MoveIntent(MoveIntent {
                        position: [6, 0, 0],
                    }),
                )?;
                door_movement_sent = true;
            }
        }
        if activity_completed && terminal_summary.is_some() {
            break;
        }
    }

    let selected = selection.ok_or("route matrix never received an accepted selection")?;
    let summary = terminal_summary.ok_or("route matrix never received a terminal summary")?;
    if !route_state_committed
        || !door_movement_sent
        || config.role == MatrixRole::Primary && (!retry_replayed || !conflict_rejected)
        || config.role == MatrixRole::Secondary && !non_leader_rejected
        || loot_quantity != selected.reward.item_quantity
        || progression_granted != selected.reward.experience
    {
        return Err("route matrix lifecycle evidence is incomplete".into());
    }
    let drone = drone
        .ok_or("route matrix drone tracker disappeared")?
        .evidence(&config.player_ids, &allowed_weapon_damage)?;
    let warden = warden
        .ok_or("route matrix Warden never spawned")?
        .evidence(&config.player_ids, &allowed_weapon_damage)?;
    if drone.hostile_damage != [10]
        || warden.hostile_damage
            != [
                selected.effect.warden_counter_damage,
                selected.effect.warden_counter_damage,
            ]
    {
        return Err(format!(
            "route matrix pressure diverged: drone={:?} warden={:?}",
            drone.hostile_damage, warden.hostile_damage
        )
        .into());
    }
    let (next_loadout_revision, next_loadout_retry_replayed) =
        if let Some(next_module_build) = config.next_module_build {
            let operation_id = config
                .next_operation_id
                .as_deref()
                .ok_or("route matrix next build is missing its operation id")?;
            let (revision, replayed) = transition_matrix_loadout(
                &mut stream,
                &incoming_rx,
                config.module_build,
                next_module_build,
                operation_id,
            )?;
            (Some(revision), replayed)
        } else {
            (None, false)
        };
    let evidence = RouteMatrixEvidence {
        schema: "RevenantM27RuntimeMatrixV1",
        account_id: config.account_id,
        role: config.role.label(),
        weapon: config.requested_weapon,
        module_build: config.module_build.label(),
        maximum_health: config.module_build.maximum_health(),
        participants: config.expected_players,
        attack_interval_ms,
        route_id: selected.route_id,
        event_id: selected.event_id,
        seed: selected.seed,
        warden_health_basis_points: selected.effect.warden_health_basis_points,
        warden_counter_damage: selected.effect.warden_counter_damage,
        objective_path: selected
            .objective_path
            .iter()
            .copied()
            .map(route_objective_label)
            .map(str::to_owned)
            .collect(),
        duration_budget_ms: selected.duration_budget_ms,
        elapsed_ms: summary.elapsed_ms,
        transition_count: summary.transitions.len(),
        drone,
        warden,
        player_health,
        loot_quantity,
        experience_granted: progression_granted,
        terminal_grant_count: summary.grants.len(),
        retry_replayed: config.role == MatrixRole::Primary && retry_replayed,
        conflict_rejected: config.role == MatrixRole::Primary && conflict_rejected,
        non_leader_rejected: config.role == MatrixRole::Secondary && non_leader_rejected,
        next_module_build: config.next_module_build.map(ModuleBuild::label),
        next_loadout_revision,
        next_loadout_retry_replayed,
        completed: true,
    };
    println!("M27_MATRIX {}", serde_json::to_string(&evidence)?);
    Ok(())
}

fn validate_matrix_selection(
    selection: &RouteSelection,
    requested_route: RequestedRoute,
    player_ids: &[u64],
    options: Option<&[revenant_protocol::RouteOption]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let option = options
        .and_then(|options| {
            options
                .iter()
                .find(|option| option.route_id == requested_route.id())
        })
        .ok_or("route matrix selection has no matching server option")?;
    let event = option
        .events
        .iter()
        .find(|event| event.event_id == selection.event_id)
        .ok_or("route matrix selection event was not offered by the server")?;
    if selection.leader_actor_id != player_ids[0]
        || selection.participant_actor_ids != player_ids
        || selection.route_id != requested_route.id()
        || selection.seed > i64::MAX.unsigned_abs()
        || selection.effect != event.effect
        || selection.objective_path != option.objective_path
        || selection.duration_budget_ms != option.duration_budget_ms
        || selection.reward != option.reward
    {
        return Err(format!("route matrix selection diverged: {selection:?}").into());
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run_matrix_activity(
    mut stream: TcpStream,
    config: MatrixConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.requested_route.is_some() {
        return Err("baseline matrix received an unexpected route".into());
    }
    if config.expected_players == 1 && config.role != MatrixRole::Primary {
        return Err("a solo matrix requires the primary role".into());
    }
    if config.expected_players == 2 && config.player_ids.len() != 2 {
        return Err("a two-active matrix requires exactly two replicated players".into());
    }
    let weapon_damage = config
        .module_build
        .weapon_damage(&config.requested_weapon)
        .ok_or("matrix weapon is not catalogued")?;
    let allowed_weapon_damage = matrix_weapon_damage_values(&config.requested_weapon)
        .ok_or("matrix weapon damage set is not catalogued")?;
    let attack_interval_ms = config.attack_cooldown_ms.max(config.simulated_rtt_ms);
    let role_phase_ms = if config.role == MatrixRole::Secondary {
        attack_interval_ms / 2
    } else {
        0
    };
    let initial_delay =
        MATRIX_SYNC_DELAY + Duration::from_millis(config.simulated_rtt_ms / 2 + role_phase_ms);
    let drone_health = scaled_matrix_health(140, config.expected_players)?;
    let warden_health = scaled_matrix_health(240, config.expected_players)?;
    let mut drone = Some(EncounterTracker::new(
        "relay_drone",
        config.first_enemy_id,
        drone_health,
    ));
    if config.initial_pressure.source_actor_id != config.first_enemy_id
        || !config
            .player_ids
            .contains(&config.initial_pressure.target_actor_id)
        || config.initial_pressure.damage != 10
        || config.initial_pressure.killed
    {
        return Err(format!(
            "matrix initial hostile pressure diverged: {:?}",
            config.initial_pressure
        )
        .into());
    }
    drone
        .as_mut()
        .expect("matrix drone tracker must exist")
        .hostile_hit(&config.initial_pressure);
    let mut warden: Option<EncounterTracker> = None;
    let mut player_health = config
        .player_ids
        .iter()
        .copied()
        .map(|player_id| (player_id, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let mut current_player_health = config
        .player_ids
        .iter()
        .copied()
        .map(|player_id| {
            let maximum_health = if player_id == config.player_id {
                config.module_build.maximum_health()
            } else {
                0
            };
            (player_id, maximum_health)
        })
        .collect::<BTreeMap<_, _>>();
    record_matrix_player_damage(
        &config.initial_pressure,
        &mut current_player_health,
        &mut player_health,
    )?;

    let (incoming_tx, incoming_rx) = mpsc::channel();
    let mut reader = stream.try_clone()?;
    thread::spawn(move || loop {
        let message = read_message(&mut reader).map_err(|error| error.to_string());
        let terminal = message.is_err();
        if incoming_tx.send(message).is_err() || terminal {
            break;
        }
    });

    let mut active_enemy_id = Some(config.first_enemy_id);
    let mut next_attack_at = Some(Instant::now() + initial_delay);
    let mut movement_sent = false;
    let mut loot_grants = 0_u32;
    let mut progression_grants = 0_u32;
    let mut experience_granted = 0_u64;

    loop {
        if next_attack_at.is_some_and(|deadline| Instant::now() >= deadline) {
            let target_actor_id = active_enemy_id
                .ok_or("matrix attack deadline remained after encounter termination")?;
            write_message(
                &mut stream,
                &ClientMessage::AttackIntent(AttackIntent { target_actor_id }),
            )?;
            next_attack_at = None;
        }
        let wait = next_attack_at.map_or(MATRIX_EVENT_POLL, |deadline| {
            deadline
                .saturating_duration_since(Instant::now())
                .min(MATRIX_EVENT_POLL)
        });
        let message = match incoming_rx.recv_timeout(wait) {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => return Err(format!("matrix receive failed: {error}").into()),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err("matrix receive worker disconnected".into())
            }
        };
        let received_at = Instant::now();
        match message {
            ServerMessage::DamageApplied(damage) => {
                let enemy_id = active_enemy_id.ok_or_else(|| {
                    format!("damage arrived without an active matrix enemy: {damage:?}")
                })?;
                let tracker =
                    matrix_tracker_mut(enemy_id, config.first_enemy_id, &mut drone, &mut warden)?;
                if damage.source_actor_id == enemy_id {
                    if !config.player_ids.contains(&damage.target_actor_id) || damage.killed {
                        return Err(format!("matrix hostile damage diverged: {damage:?}").into());
                    }
                    tracker.hostile_hit(&damage);
                    record_matrix_player_damage(
                        &damage,
                        &mut current_player_health,
                        &mut player_health,
                    )?;
                } else if damage.target_actor_id == enemy_id {
                    if !config.player_ids.contains(&damage.source_actor_id)
                        || if damage.source_actor_id == config.player_id {
                            damage.damage != weapon_damage
                        } else {
                            !allowed_weapon_damage.contains(&damage.damage)
                        }
                    {
                        return Err(format!("matrix player damage diverged: {damage:?}").into());
                    }
                    tracker.player_hit(&damage, received_at);
                    if damage.killed {
                        active_enemy_id = None;
                        next_attack_at = None;
                    } else if damage.source_actor_id == config.player_id {
                        next_attack_at =
                            Some(received_at + Duration::from_millis(attack_interval_ms));
                    }
                } else {
                    return Err(
                        format!("matrix damage missed the active exchange: {damage:?}").into(),
                    );
                }
            }
            ServerMessage::ActorDestroy(destroy) => {
                if destroy.actor_id != config.first_enemy_id
                    && warden
                        .as_ref()
                        .is_none_or(|tracker| tracker.enemy_id != destroy.actor_id)
                {
                    return Err(format!(
                        "matrix destroyed an unexpected actor {}",
                        destroy.actor_id
                    )
                    .into());
                }
                active_enemy_id = None;
                next_attack_at = None;
            }
            ServerMessage::ObjectiveUpdate(objective) => {
                if objective.objective_id == "reach_relay_door"
                    && objective.state == "Active"
                    && !movement_sent
                {
                    write_message(
                        &mut stream,
                        &ClientMessage::MoveIntent(MoveIntent {
                            position: [6, 0, 0],
                        }),
                    )?;
                    movement_sent = true;
                }
            }
            ServerMessage::ActorSpawn(actor) if actor.archetype == "warden" => {
                if warden.is_some() || actor.health != warden_health {
                    return Err(format!("matrix Warden spawn diverged: {actor:?}").into());
                }
                warden = Some(EncounterTracker::new(
                    "warden",
                    actor.actor_id,
                    warden_health,
                ));
                active_enemy_id = Some(actor.actor_id);
                next_attack_at = Some(Instant::now() + initial_delay);
            }
            ServerMessage::LootGranted(loot) => {
                if loot.item_id != "relay_core_fragment" || loot.quantity != 1 {
                    return Err(format!("matrix loot diverged: {loot:?}").into());
                }
                loot_grants += 1;
            }
            ServerMessage::ProgressionGranted(progression) => {
                if progression.experience_granted != 100
                    || progression.level < progression.previous_level
                {
                    return Err(format!("matrix progression diverged: {progression:?}").into());
                }
                progression_grants += 1;
                experience_granted = experience_granted
                    .checked_add(progression.experience_granted)
                    .ok_or("matrix experience counter overflowed")?;
            }
            ServerMessage::ActivityComplete(activity) => {
                if activity.activity_id != "relay_awakening" {
                    return Err(format!("matrix activity diverged: {activity:?}").into());
                }
                break;
            }
            ServerMessage::ActorUpdate(_)
            | ServerMessage::DoorState(_)
            | ServerMessage::EquipmentChanged(_) => {}
            unexpected => {
                return Err(format!("unexpected matrix runtime message: {unexpected:?}").into())
            }
        }
    }

    if !movement_sent || loot_grants != 1 || progression_grants != 1 || experience_granted != 100 {
        return Err(format!(
            "matrix terminal state diverged: movement={movement_sent} loot={loot_grants} progression={progression_grants} xp={experience_granted}"
        )
        .into());
    }
    let drone = drone
        .ok_or("matrix drone tracker disappeared")?
        .evidence(&config.player_ids, &allowed_weapon_damage)?;
    let warden = warden
        .ok_or("matrix Warden never spawned")?
        .evidence(&config.player_ids, &allowed_weapon_damage)?;
    if drone.hostile_damage != [10] || warden.hostile_damage != [15, 15] {
        return Err(format!(
            "matrix hostile pressure sequence diverged: drone={:?} warden={:?}",
            drone.hostile_damage, warden.hostile_damage
        )
        .into());
    }
    let observed_damage = drone
        .hostile_damage
        .iter()
        .chain(&warden.hostile_damage)
        .copied()
        .map(u64::from)
        .sum::<u64>();
    if observed_damage != 40 {
        return Err(format!("matrix incoming damage was {observed_damage}, expected 40").into());
    }
    let (next_loadout_revision, next_loadout_retry_replayed) =
        if let Some(next_module_build) = config.next_module_build {
            let operation_id = config
                .next_operation_id
                .as_deref()
                .ok_or("matrix next module build is missing its operation id")?;
            let (revision, replayed) = transition_matrix_loadout(
                &mut stream,
                &incoming_rx,
                config.module_build,
                next_module_build,
                operation_id,
            )?;
            (Some(revision), replayed)
        } else {
            (None, false)
        };
    let evidence = MatrixEvidence {
        schema: "RevenantM25RuntimeMatrixV1",
        account_id: config.account_id,
        role: config.role.label(),
        weapon: config.requested_weapon,
        module_build: config.module_build.label(),
        maximum_health: config.module_build.maximum_health(),
        participants: config.expected_players,
        simulated_rtt_ms: config.simulated_rtt_ms,
        attack_interval_ms,
        drone,
        warden,
        player_health,
        loot_grants,
        progression_grants,
        experience_granted,
        next_module_build: config.next_module_build.map(ModuleBuild::label),
        next_loadout_revision,
        next_loadout_retry_replayed,
        completed: true,
    };
    println!("M25_MATRIX {}", serde_json::to_string(&evidence)?);
    Ok(())
}

fn receive_matrix_message(
    incoming: &mpsc::Receiver<Result<ServerMessage, String>>,
) -> Result<ServerMessage, Box<dyn std::error::Error>> {
    match incoming.recv_timeout(SMOKE_IO_TIMEOUT) {
        Ok(Ok(message)) => Ok(message),
        Ok(Err(error)) => Err(format!("matrix receive failed: {error}").into()),
        Err(RecvTimeoutError::Timeout) => Err("matrix receive timed out".into()),
        Err(RecvTimeoutError::Disconnected) => Err("matrix receive worker disconnected".into()),
    }
}

fn transition_matrix_loadout(
    stream: &mut TcpStream,
    incoming: &mpsc::Receiver<Result<ServerMessage, String>>,
    admitted_build: ModuleBuild,
    next_build: ModuleBuild,
    operation_id: &str,
) -> Result<(u64, bool), Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::ModuleStateRequest(ModuleStateRequest {}),
    )?;
    let ServerMessage::ModuleSnapshot(initial) = receive_matrix_message(incoming)? else {
        return Err("matrix loadout transition expected ModuleSnapshot".into());
    };
    validate_module_snapshot(&initial)?;
    if initial.equipped_modules != admitted_build.modules() {
        return Err(format!("matrix admitted module state diverged: {initial:?}").into());
    }
    let requested_modules = next_build.modules();
    if requested_modules
        .iter()
        .any(|module| !initial.owned_modules.contains(module))
    {
        return Err(format!("matrix next build is not owned: {initial:?}").into());
    }
    write_message(
        stream,
        &ClientMessage::ModuleLoadoutIntent(ModuleLoadoutIntent {
            operation_id: operation_id.to_owned(),
            expected_revision: initial.loadout_revision,
            modules: requested_modules.clone(),
        }),
    )?;
    let ServerMessage::ModuleLoadoutChanged(changed) = receive_matrix_message(incoming)? else {
        return Err("matrix loadout transition expected ModuleLoadoutChanged".into());
    };
    if !changed.accepted
        || changed.replayed
        || changed.state.loadout_revision != initial.loadout_revision + 1
        || changed.state.equipped_modules != requested_modules
        || changed.state.max_health != next_build.maximum_health()
    {
        return Err(format!("matrix loadout transition diverged: {changed:?}").into());
    }
    write_message(
        stream,
        &ClientMessage::ModuleLoadoutIntent(ModuleLoadoutIntent {
            operation_id: operation_id.to_owned(),
            expected_revision: initial.loadout_revision,
            modules: requested_modules,
        }),
    )?;
    let ServerMessage::ModuleLoadoutChanged(retry) = receive_matrix_message(incoming)? else {
        return Err("matrix loadout retry expected ModuleLoadoutChanged".into());
    };
    if !retry.accepted || !retry.replayed || retry.state != changed.state {
        return Err(format!("matrix loadout retry diverged: {retry:?}").into());
    }
    Ok((changed.state.loadout_revision, retry.replayed))
}

fn scaled_matrix_health(
    base_health: u32,
    expected_players: usize,
) -> Result<u32, Box<dyn std::error::Error>> {
    let players = u32::try_from(expected_players)?;
    base_health
        .checked_mul(
            players
                .checked_add(1)
                .ok_or("matrix player count overflowed")?,
        )
        .map(|scaled| scaled / 2)
        .ok_or_else(|| "matrix health scaling overflowed".into())
}

fn matrix_tracker_mut<'a>(
    enemy_id: u64,
    drone_id: u64,
    drone: &'a mut Option<EncounterTracker>,
    warden: &'a mut Option<EncounterTracker>,
) -> Result<&'a mut EncounterTracker, Box<dyn std::error::Error>> {
    let tracker = if enemy_id == drone_id { drone } else { warden };
    tracker
        .as_mut()
        .filter(|tracker| tracker.enemy_id == enemy_id)
        .ok_or_else(|| format!("matrix tracker is missing for enemy {enemy_id}").into())
}

fn record_matrix_player_damage(
    damage: &DamageApplied,
    current_health: &mut BTreeMap<u64, u32>,
    health_sequences: &mut BTreeMap<u64, Vec<u32>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let health = current_health
        .get_mut(&damage.target_actor_id)
        .ok_or("matrix hostile damage targeted an unknown player")?;
    if *health == 0 {
        let inferred_maximum = damage
            .remaining_health
            .checked_add(damage.damage)
            .ok_or("matrix inferred player health overflowed")?;
        if !matches!(inferred_maximum, 100 | 120) {
            return Err(format!("matrix inferred player maximum diverged: {damage:?}").into());
        }
        *health = inferred_maximum;
    }
    let expected = health.saturating_sub(damage.damage);
    if damage.remaining_health != expected || damage.killed || expected == 0 {
        return Err(format!("matrix player health diverged: {damage:?}").into());
    }
    *health = expected;
    health_sequences
        .get_mut(&damage.target_actor_id)
        .ok_or("matrix player health sequence is missing")?
        .push(expected);
    Ok(())
}

fn observe_shared_activity(
    stream: &mut TcpStream,
    first_enemy_id: u64,
    expected_players: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut enemy_destroyed = false;
    let mut boss_spawned = false;
    let mut boss_destroyed = false;
    let mut objective_updates = 1;
    let mut loot_received = false;
    let mut progression_received = false;
    let mut equipment_change_received = false;
    loop {
        match read_message(stream)? {
            ServerMessage::ActorDestroy(destroy) if destroy.actor_id == first_enemy_id => {
                enemy_destroyed = true;
            }
            ServerMessage::ActorSpawn(actor) if actor.archetype == "warden" => {
                boss_spawned = true;
            }
            ServerMessage::ActorDestroy(_) if boss_spawned => boss_destroyed = true,
            ServerMessage::ObjectiveUpdate(_) => objective_updates += 1,
            ServerMessage::LootGranted(loot) if loot.item_id == "relay_core_fragment" => {
                loot_received = true;
            }
            ServerMessage::ProgressionGranted(progression)
                if progression.experience_granted == 100 =>
            {
                progression_received = true;
            }
            ServerMessage::EquipmentChanged(equipment) if equipment.accepted => {
                equipment_change_received = true;
            }
            ServerMessage::ActivityComplete(activity) => {
                if !enemy_destroyed
                    || !boss_spawned
                    || !boss_destroyed
                    || objective_updates < 5
                    || !loot_received
                    || !progression_received
                    || !equipment_change_received
                {
                    return Err("observer missed shared activity state".into());
                }
                println!(
                    "observer received {expected_players} players and shared completion for {}",
                    activity.activity_id
                );
                return Ok(());
            }
            ServerMessage::ActorUpdate(_)
            | ServerMessage::DamageApplied(_)
            | ServerMessage::DoorState(_) => {}
            _ => return Err("observer received an unexpected shared message".into()),
        }
    }
}

fn defeat_enemy(
    stream: &mut TcpStream,
    enemy_id: u64,
    attack_cooldown_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        write_message(
            stream,
            &ClientMessage::AttackIntent(AttackIntent {
                target_actor_id: enemy_id,
            }),
        )?;
        let damage = loop {
            let damage = match read_message(stream)? {
                ServerMessage::DamageApplied(damage) => damage,
                ServerMessage::RouteState(_) => continue,
                _ => return Err("expected DamageApplied".into()),
            };
            if damage.source_actor_id == enemy_id {
                println!(
                    "enemy actor {enemy_id} counterattacked for {}; player has {} HP",
                    damage.damage, damage.remaining_health
                );
                if damage.killed {
                    return Err("enemy pressure defeated the active player".into());
                }
                continue;
            }
            if damage.target_actor_id != enemy_id {
                return Err("DamageApplied did not target the active enemy".into());
            }
            break damage;
        };
        println!(
            "dealt {} damage to actor {}; {} HP remains",
            damage.damage, damage.target_actor_id, damage.remaining_health
        );
        if damage.killed {
            break;
        }
        std::thread::sleep(Duration::from_millis(attack_cooldown_ms));
    }
    let ServerMessage::ActorDestroy(destroy) = read_message(stream)? else {
        return Err("expected ActorDestroy".into());
    };
    println!("enemy actor {} destroyed", destroy.actor_id);
    Ok(())
}
