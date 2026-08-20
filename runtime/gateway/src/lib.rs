use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use revenant_activities::{ActivityEvent, ScriptedActivity};
use revenant_actors::{Actor, ActorKind, ActorRegistry};
use revenant_ai::{AiController, AiEvent};
use revenant_combat::CombatRuntime;
use revenant_compatibility::{CanonicalClientMessage, ProtocolAdapter};
use revenant_identity::LocalIdentityService;
use revenant_objectives::{Objective, ObjectiveKind, ObjectiveState, WorldTrigger};
use revenant_persistence::{NewReplayEvent, Persistence};
use revenant_protocol::{
    read_message, write_message, ActivityComplete, ActivityStart, ActorDestroy, ActorSpawn,
    ActorUpdate, AuthResponse, CharacterListResponse, CharacterSummary, ClientMessage,
    DamageApplied, DoorState, ObjectiveUpdate, ServerHello, ServerMessage, WorldJoinResponse,
    PROTOCOL_VERSION,
};
use revenant_replay::ReplayEventKind;
use revenant_world::WorldService;

pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_GAME_ADDR: &str = "127.0.0.1:7000";
pub const DEFAULT_DATABASE_URL: &str = "postgres://revenant:revenant_local@127.0.0.1:5432/revenant";

/// Runs the gateway until the process is terminated.
///
/// # Errors
///
/// Returns an I/O error when infrastructure initialization or listener operations fail.
pub fn run(bind_addr: &str, game_addr: &str) -> io::Result<()> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    Persistence::connect(&database_url).map_err(|error| {
        io::Error::other(format!(
            "failed to initialize PostgreSQL persistence: {error}"
        ))
    })?;
    let expected_players = env::var("REVENANT_EXPECTED_PLAYERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    if expected_players == 0 {
        return Err(io::Error::other(
            "REVENANT_EXPECTED_PLAYERS must be positive",
        ));
    }
    let script_path = env::var("REVENANT_ACTIVITY_SCRIPT")
        .unwrap_or_else(|_| "scripts/activities/relay_awakening.lua".to_owned());
    let activity = ScriptedActivity::load(script_path)
        .map_err(|error| io::Error::other(format!("activity load failed: {error}")))?;
    let session = SharedSession::new(&database_url, activity, expected_players)?;
    let (command_tx, command_rx) = mpsc::channel();
    thread::spawn(move || session.run(command_rx));

    let health_listener = TcpListener::bind(bind_addr)?;
    let game_listener = TcpListener::bind(game_addr)?;
    println!(
        "{{\"event\":\"gateway_started\",\"health_address\":\"{bind_addr}\",\"game_address\":\"{game_addr}\",\"protocol_version\":{PROTOCOL_VERSION},\"expected_players\":{expected_players}}}"
    );
    let inspector_database_url = database_url.clone();
    thread::spawn(move || serve_http(&health_listener, &inspector_database_url));

    for stream in game_listener.incoming() {
        match stream {
            Ok(stream) => {
                let database_url = database_url.clone();
                let command_tx = command_tx.clone();
                thread::spawn(move || {
                    if let Err(error) = handle_game_connection(stream, &database_url, &command_tx) {
                        eprintln!("{{\"event\":\"connection_failed\",\"error\":\"{error}\"}}");
                    }
                });
            }
            Err(error) => eprintln!("{{\"event\":\"accept_failed\",\"error\":\"{error}\"}}"),
        }
    }
    Ok(())
}

fn serve_http(listener: &TcpListener, database_url: &str) {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => match handle_http_connection(&stream, database_url) {
                Ok((status, body)) => {
                    let response = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
                Err(error) => {
                    let body = serde_json::json!({ "error": error.to_string() }).to_string();
                    let response = format!(
                        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            },
            Err(error) => eprintln!("{{\"event\":\"health_accept_failed\",\"error\":\"{error}\"}}"),
        }
    }
}

fn handle_http_connection(
    stream: &TcpStream,
    database_url: &str,
) -> Result<(&'static str, String), Box<dyn std::error::Error>> {
    let mut request_line = String::new();
    BufReader::new(stream).read_line(&mut request_line)?;
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::other("HTTP request path is missing"))?;
    match inspector_route(path) {
        InspectorRoute::Health => Ok((
            "200 OK",
            "{\"status\":\"ok\",\"service\":\"revenant-gateway\"}\n".to_owned(),
        )),
        InspectorRoute::Sessions => {
            let mut persistence = Persistence::connect(database_url)?;
            let sessions = persistence.replay_sessions(100)?;
            let sessions = sessions
                .into_iter()
                .map(|session| {
                    serde_json::json!({
                        "session_id": session.session_id,
                        "activity_id": session.activity_id,
                        "started_at": session.started_at,
                        "ended_at": session.ended_at,
                        "event_count": session.event_count,
                        "participant_count": session.participant_count,
                        "completed": session.completed,
                    })
                })
                .collect::<Vec<_>>();
            Ok((
                "200 OK",
                serde_json::json!({ "sessions": sessions }).to_string(),
            ))
        }
        InspectorRoute::Events(session_id) => {
            let mut persistence = Persistence::connect(database_url)?;
            let events = persistence.replay_events(session_id)?;
            let events = events
                .into_iter()
                .map(|event| {
                    serde_json::json!({
                        "event_id": event.id,
                        "event_type": event.event_type,
                        "timestamp": event.timestamp,
                        "session_id": event.session_id,
                        "activity_id": event.activity_id,
                        "actor_id": event.actor_id,
                        "payload": event.payload,
                    })
                })
                .collect::<Vec<_>>();
            Ok((
                "200 OK",
                serde_json::json!({ "events": events }).to_string(),
            ))
        }
        InspectorRoute::NotFound => Ok(("404 Not Found", "{\"error\":\"not_found\"}\n".to_owned())),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum InspectorRoute<'a> {
    Health,
    Sessions,
    Events(&'a str),
    NotFound,
}

fn inspector_route(path: &str) -> InspectorRoute<'_> {
    if path == "/health" {
        InspectorRoute::Health
    } else if path == "/api/inspector/sessions" {
        InspectorRoute::Sessions
    } else if let Some(session_id) = path
        .strip_prefix("/api/inspector/sessions/")
        .and_then(|suffix| suffix.strip_suffix("/events"))
        .filter(|session_id| {
            !session_id.is_empty()
                && session_id
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
    {
        InspectorRoute::Events(session_id)
    } else {
        InspectorRoute::NotFound
    }
}

#[allow(clippy::too_many_lines)]
fn handle_game_connection(
    mut stream: TcpStream,
    database_url: &str,
    command_tx: &Sender<SessionCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(15)))?;
    let ClientMessage::ClientHello(hello) = read_message(&mut stream)? else {
        return Err(unexpected_message("ClientHello"));
    };
    let negotiated = ProtocolAdapter::negotiate(hello.protocol_version);
    let accepted = negotiated.is_ok();
    let selected_version = negotiated
        .as_ref()
        .map_or(PROTOCOL_VERSION, |adapter| adapter.wire_version());
    write_message(
        &mut stream,
        &ServerMessage::ServerHello(ServerHello {
            protocol_version: selected_version,
            server_name: "revenant-core".to_owned(),
            accepted,
            message: if accepted {
                "handshake accepted"
            } else {
                "unsupported protocol version"
            }
            .to_owned(),
        }),
    )?;
    if !accepted {
        return Ok(());
    }
    let adapter = negotiated.expect("accepted negotiation must contain an adapter");
    println!(
        "{{\"event\":\"protocol_negotiated\",\"client_version\":{},\"generation\":\"{:?}\"}}",
        hello.protocol_version,
        adapter.generation()
    );

    let CanonicalClientMessage::AuthRequest(request) = read_canonical(&mut stream, adapter)? else {
        return Err(unexpected_message("AuthRequest"));
    };
    let identity = LocalIdentityService;
    let account = match identity.authenticate(&request.username) {
        Ok(account) => account,
        Err(error) => {
            write_message(
                &mut stream,
                &ServerMessage::AuthResponse(AuthResponse {
                    authenticated: false,
                    account_id: String::new(),
                    message: error.to_string(),
                }),
            )?;
            return Ok(());
        }
    };
    write_message(
        &mut stream,
        &ServerMessage::AuthResponse(AuthResponse {
            authenticated: true,
            account_id: account.id.clone(),
            message: "local authentication accepted".to_owned(),
        }),
    )?;
    let mut persistence = Persistence::connect(database_url)?;
    persistence.ensure_local_account(&account.id, &account.username)?;

    let CanonicalClientMessage::CharacterListRequest(_) = read_canonical(&mut stream, adapter)?
    else {
        return Err(unexpected_message("CharacterListRequest"));
    };
    let characters = persistence
        .characters_for(&account.id)?
        .into_iter()
        .map(|character| {
            Ok(CharacterSummary {
                character_id: character.id,
                display_name: character.display_name,
                class_name: character.class_name,
                level: u16::try_from(character.level)?,
            })
        })
        .collect::<Result<Vec<_>, std::num::TryFromIntError>>()?;
    let character_ids = characters
        .iter()
        .map(|character| character.character_id.clone())
        .collect::<Vec<_>>();
    write_message(
        &mut stream,
        &ServerMessage::CharacterListResponse(CharacterListResponse { characters }),
    )?;

    let CanonicalClientMessage::WorldJoinRequest(request) = read_canonical(&mut stream, adapter)?
    else {
        return Err(unexpected_message("WorldJoinRequest"));
    };
    if !character_ids.contains(&request.character_id) {
        write_message(
            &mut stream,
            &ServerMessage::WorldJoinResponse(WorldJoinResponse {
                accepted: false,
                world_id: String::new(),
                player_actor_id: 0,
                spawn_position: [0, 0, 0],
                message: "character does not belong to authenticated account".to_owned(),
            }),
        )?;
        return Ok(());
    }
    load_character_state(&mut persistence, &request.character_id)?;
    let player = WorldService.join(&request.character_id);
    write_message(
        &mut stream,
        &ServerMessage::WorldJoinResponse(WorldJoinResponse {
            accepted: true,
            world_id: player.world_id,
            player_actor_id: player.actor_id,
            spawn_position: [0, 0, 0],
            message: "world join accepted".to_owned(),
        }),
    )?;

    let (outbound_tx, outbound_rx) = mpsc::channel();
    let mut writer = stream.try_clone()?;
    thread::spawn(move || write_outbound(&mut writer, outbound_rx));
    command_tx.send(SessionCommand::Join(Participant {
        account_id: account.id,
        character_id: request.character_id,
        actor: Actor {
            id: player.actor_id,
            kind: ActorKind::Player,
            archetype: "operator".to_owned(),
            position: [0, 0, 0],
            health: 100,
            max_health: 100,
        },
        outbound: outbound_tx,
    }))?;

    loop {
        match read_canonical(&mut stream, adapter) {
            Ok(CanonicalClientMessage::AttackIntent(intent)) => {
                command_tx.send(SessionCommand::Attack {
                    player_id: player.actor_id,
                    target_id: intent.target_actor_id,
                })?;
            }
            Ok(CanonicalClientMessage::MoveIntent(intent)) => {
                command_tx.send(SessionCommand::Move {
                    player_id: player.actor_id,
                    position: intent.position,
                })?;
            }
            Ok(_) => return Err(unexpected_message("AttackIntent or MoveIntent")),
            Err(_) => {
                let _ = command_tx.send(SessionCommand::Disconnect(player.actor_id));
                return Ok(());
            }
        }
    }
}

fn read_canonical(
    stream: &mut TcpStream,
    adapter: ProtocolAdapter,
) -> Result<CanonicalClientMessage, Box<dyn std::error::Error>> {
    let wire_message: ClientMessage = read_message(stream)?;
    Ok(adapter.canonicalize(wire_message)?)
}

fn write_outbound(stream: &mut TcpStream, messages: Receiver<ServerMessage>) {
    for message in messages {
        if write_message(stream, &message).is_err() {
            break;
        }
    }
}

enum SessionCommand {
    Join(Participant),
    Attack { player_id: u64, target_id: u64 },
    Move { player_id: u64, position: [i32; 3] },
    Disconnect(u64),
}

struct Participant {
    account_id: String,
    character_id: String,
    actor: Actor,
    outbound: Sender<ServerMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionStage {
    Waiting,
    Drone,
    Door,
    Boss,
    Complete,
}

struct SharedSession {
    expected_players: usize,
    participants: Vec<Participant>,
    actors: ActorRegistry,
    activity: ScriptedActivity,
    combat: CombatRuntime,
    combat_started: Instant,
    enemy_id: Option<u64>,
    stage: SessionStage,
    persistence: Persistence,
    session_id: String,
}

impl SharedSession {
    fn new(
        database_url: &str,
        activity: ScriptedActivity,
        expected_players: usize,
    ) -> io::Result<Self> {
        Ok(Self {
            expected_players,
            participants: Vec::new(),
            actors: ActorRegistry::default(),
            activity,
            combat: CombatRuntime::default(),
            combat_started: Instant::now(),
            enemy_id: None,
            stage: SessionStage::Waiting,
            persistence: Persistence::connect(database_url).map_err(|error| {
                io::Error::other(format!(
                    "failed to initialize PostgreSQL persistence: {error}"
                ))
            })?,
            session_id: new_session_id()?,
        })
    }

    fn run(mut self, commands: Receiver<SessionCommand>) {
        for command in commands {
            let result = match command {
                SessionCommand::Join(participant) => self.join(participant),
                SessionCommand::Attack {
                    player_id,
                    target_id,
                } => self.attack(player_id, target_id),
                SessionCommand::Move {
                    player_id,
                    position,
                } => self.move_player(player_id, position),
                SessionCommand::Disconnect(player_id) => {
                    self.disconnect(player_id);
                    Ok(())
                }
            };
            if let Err(error) = result {
                eprintln!("{{\"event\":\"session_command_failed\",\"error\":\"{error}\"}}");
            }
        }
    }

    fn join(&mut self, participant: Participant) -> Result<(), Box<dyn std::error::Error>> {
        if self.stage != SessionStage::Waiting || self.participants.len() >= self.expected_players {
            return Err(io::Error::other("shared session is not accepting more players").into());
        }
        self.actors.insert(participant.actor.clone());
        self.append_event(
            ReplayEventKind::PlayerJoined,
            &participant.account_id,
            Some(participant.actor.id),
            "player joined",
        )?;
        self.participants.push(participant);
        if self.participants.len() == self.expected_players {
            self.start_activity()?;
        }
        Ok(())
    }

    fn start_activity(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::ActivityStarted,
            &owner,
            None,
            "activity started",
        )?;
        for message in activity_messages(self.activity.start()).0 {
            self.broadcast(message);
        }
        let player_actors = self
            .participants
            .iter()
            .map(|participant| participant.actor.clone())
            .collect::<Vec<_>>();
        for actor in &player_actors {
            self.broadcast(actor_spawn_message(actor));
        }
        let enemy = self
            .actors
            .spawn(ActorKind::Enemy, "relay-drone", [4, 0, 2], 100);
        self.enemy_id = Some(enemy.id);
        self.stage = SessionStage::Drone;
        self.append_event(
            ReplayEventKind::EnemySpawned,
            &owner,
            Some(enemy.id),
            "enemy spawned: relay-drone",
        )?;
        self.broadcast(actor_spawn_message(&enemy));
        self.run_enemy_ai(enemy.id)?;
        Ok(())
    }

    fn run_enemy_ai(&mut self, enemy_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let player_id = self
            .participants
            .first()
            .ok_or_else(|| io::Error::other("session has no player"))?
            .actor
            .id;
        let mut ai = AiController::default();
        for _ in 0..16 {
            for event in ai.tick(&mut self.actors, enemy_id, player_id) {
                match event {
                    AiEvent::StateChanged(_) => {}
                    AiEvent::Moved { actor_id, position } => {
                        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
                            actor_id,
                            position,
                        }));
                    }
                    AiEvent::Attacked {
                        source_actor_id,
                        target_actor_id,
                        damage,
                        remaining_health,
                    } => {
                        self.broadcast(ServerMessage::DamageApplied(DamageApplied {
                            source_actor_id,
                            target_actor_id,
                            damage,
                            remaining_health,
                            killed: false,
                        }));
                        return Ok(());
                    }
                }
            }
        }
        Err(io::Error::other("enemy AI did not reach attack state").into())
    }

    fn attack(&mut self, player_id: u64, target_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        if !matches!(self.stage, SessionStage::Drone | SessionStage::Boss) {
            return Err(io::Error::other("combat is not active").into());
        }
        if Some(target_id) != self.enemy_id {
            return Err(io::Error::other("target is not the active encounter enemy").into());
        }
        let elapsed_ms =
            u64::try_from(self.combat_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let result = self
            .combat
            .attack(&mut self.actors, player_id, target_id, elapsed_ms)?;
        self.broadcast(ServerMessage::DamageApplied(DamageApplied {
            source_actor_id: player_id,
            target_actor_id: target_id,
            damage: result.damage,
            remaining_health: result.remaining_health,
            killed: result.killed,
        }));
        if result.killed {
            self.enemy_defeated(target_id)?;
        }
        Ok(())
    }

    fn enemy_defeated(&mut self, target_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let defeated = self
            .actors
            .destroy(target_id)
            .ok_or_else(|| io::Error::other("defeated actor disappeared"))?;
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::EnemyDied,
            &owner,
            Some(target_id),
            &format!("enemy died: {}", defeated.archetype),
        )?;
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: target_id,
        }));
        let group_id = if self.stage == SessionStage::Drone {
            "relay_drones"
        } else {
            "warden"
        };
        let (messages, _) =
            activity_messages(self.activity.apply_trigger(&WorldTrigger::ActorGroupDead {
                group_id: group_id.to_owned(),
            }));
        let (completion_messages, state_messages): (Vec<_>, Vec<_>) = messages
            .into_iter()
            .partition(|message| matches!(message, ServerMessage::ActivityComplete(_)));
        for message in state_messages {
            self.broadcast(message);
        }
        if self.stage == SessionStage::Drone {
            self.stage = SessionStage::Door;
            self.enemy_id = None;
        } else {
            self.complete_activity(&owner)?;
            for message in completion_messages {
                self.broadcast(message);
            }
        }
        Ok(())
    }

    fn move_player(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.stage != SessionStage::Door || position != [6, 0, 0] {
            return Err(io::Error::other("relay door is not reachable now").into());
        }
        self.actors
            .update_position(player_id, position)
            .ok_or_else(|| io::Error::other("player actor was not found"))?;
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: player_id,
            position,
        }));
        let (messages, boss_archetype) =
            activity_messages(self.activity.apply_trigger(&WorldTrigger::AreaReached {
                area_id: "relay_door".to_owned(),
            }));
        for message in messages {
            self.broadcast(message);
        }
        let archetype =
            boss_archetype.ok_or_else(|| io::Error::other("activity did not request a boss"))?;
        let boss = self
            .actors
            .spawn(ActorKind::Enemy, &archetype, [8, 0, 0], 120);
        self.enemy_id = Some(boss.id);
        self.stage = SessionStage::Boss;
        self.combat = CombatRuntime::default();
        self.combat_started = Instant::now();
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::BossSpawned,
            &owner,
            Some(boss.id),
            &format!("boss spawned: {archetype}"),
        )?;
        self.broadcast(actor_spawn_message(&boss));
        Ok(())
    }

    fn complete_activity(&mut self, owner: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.stage = SessionStage::Complete;
        self.enemy_id = None;
        self.append_event(
            ReplayEventKind::ActivityCompleted,
            owner,
            None,
            "activity completed",
        )?;
        for participant in &self.participants {
            self.persistence.record_activity_completion(
                &participant.account_id,
                &participant.character_id,
                self.activity.id(),
            )?;
        }
        Ok(())
    }

    fn append_event(
        &mut self,
        kind: ReplayEventKind,
        account_id: &str,
        actor_id: Option<u64>,
        payload: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.persistence.append_replay_event(&NewReplayEvent {
            event_type: &kind.to_string(),
            session_id: &self.session_id,
            account_id,
            activity_id: Some(self.activity.id()),
            actor_id: actor_id.map(i64::try_from).transpose()?,
            payload,
        })?;
        Ok(())
    }

    fn owner_account(&self) -> Result<&str, io::Error> {
        self.participants
            .first()
            .map(|participant| participant.account_id.as_str())
            .ok_or_else(|| io::Error::other("session has no owner"))
    }

    #[allow(clippy::needless_pass_by_value)]
    fn broadcast(&mut self, message: ServerMessage) {
        self.participants
            .retain(|participant| participant.outbound.send(message.clone()).is_ok());
    }

    fn disconnect(&mut self, player_id: u64) {
        if self.stage == SessionStage::Complete {
            self.participants
                .retain(|participant| participant.actor.id != player_id);
        }
    }
}

fn activity_messages(events: Vec<ActivityEvent>) -> (Vec<ServerMessage>, Option<String>) {
    let mut messages = Vec::new();
    let mut boss_archetype = None;
    for event in events {
        match event {
            ActivityEvent::Started { activity_id } => {
                messages.push(ServerMessage::ActivityStart(ActivityStart { activity_id }));
            }
            ActivityEvent::ObjectiveUpdated(objective) => {
                messages.push(ServerMessage::ObjectiveUpdate(objective_message(
                    &objective,
                )));
            }
            ActivityEvent::DoorOpened { door_id } => {
                messages.push(ServerMessage::DoorState(DoorState {
                    door_id,
                    open: true,
                }));
            }
            ActivityEvent::BossRequested { archetype } => boss_archetype = Some(archetype),
            ActivityEvent::Completed { activity_id } => {
                messages.push(ServerMessage::ActivityComplete(ActivityComplete {
                    activity_id,
                }));
            }
        }
    }
    (messages, boss_archetype)
}

fn objective_message(objective: &Objective) -> ObjectiveUpdate {
    ObjectiveUpdate {
        objective_id: objective.id.clone(),
        objective_type: match objective.kind {
            ObjectiveKind::KillActors => "KillActors",
            ObjectiveKind::ReachArea => "ReachArea",
            ObjectiveKind::Boss => "Boss",
        }
        .to_owned(),
        state: match objective.state {
            ObjectiveState::Pending => "Pending",
            ObjectiveState::Active => "Active",
            ObjectiveState::Completed => "Completed",
            ObjectiveState::Failed => "Failed",
        }
        .to_owned(),
        progress: objective.progress,
        target: objective.target,
    }
}

fn actor_spawn_message(actor: &Actor) -> ServerMessage {
    ServerMessage::ActorSpawn(ActorSpawn {
        actor_id: actor.id,
        actor_kind: match actor.kind {
            ActorKind::Player => "player",
            ActorKind::Enemy => "enemy",
        }
        .to_owned(),
        archetype: actor.archetype.clone(),
        position: actor.position,
        health: actor.health,
        max_health: actor.max_health,
    })
}

fn load_character_state(
    persistence: &mut Persistence,
    character_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _inventory = persistence.inventory_for(character_id)?;
    persistence
        .progression_for(character_id)?
        .ok_or_else(|| io::Error::other("character progression is missing"))?;
    Ok(())
}

fn new_session_id() -> io::Result<String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::other(format!("system clock error: {error}")))?
        .as_nanos();
    Ok(format!("session-{timestamp}"))
}

fn unexpected_message(expected: &str) -> Box<dyn std::error::Error> {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expected {expected} in the current connection state"),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::{
        inspector_route, InspectorRoute, SessionStage, DEFAULT_BIND_ADDR, DEFAULT_DATABASE_URL,
        DEFAULT_GAME_ADDR,
    };

    #[test]
    fn default_address_is_local_only() {
        assert_eq!(DEFAULT_BIND_ADDR, "127.0.0.1:8080");
        assert_eq!(DEFAULT_GAME_ADDR, "127.0.0.1:7000");
        assert!(DEFAULT_DATABASE_URL.starts_with("postgres://"));
    }

    #[test]
    fn multiplayer_session_stages_are_explicit() {
        assert_ne!(SessionStage::Waiting, SessionStage::Drone);
        assert_ne!(SessionStage::Door, SessionStage::Boss);
        assert_ne!(SessionStage::Boss, SessionStage::Complete);
    }

    #[test]
    fn inspector_routes_only_accept_canonical_session_ids() {
        assert_eq!(
            inspector_route("/api/inspector/sessions"),
            InspectorRoute::Sessions
        );
        assert_eq!(
            inspector_route("/api/inspector/sessions/session-123/events"),
            InspectorRoute::Events("session-123")
        );
        assert_eq!(
            inspector_route("/api/inspector/sessions/../events"),
            InspectorRoute::NotFound
        );
    }
}
