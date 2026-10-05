use std::collections::VecDeque;
use std::env;
use std::fmt::{self, Display, Formatter};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod acquisition;
mod arsenal;
mod bulwark;
mod campaign;
mod campaign_breach;
mod campaign_counter;
mod campaign_prism;
mod campaign_story;
mod campaign_supply;
mod challenges;
mod coolant;
mod elite;
mod lancer;
mod meridian;
mod prism;
mod signal;
mod support;

use acquisition::{AcquisitionMutationResult, AcquisitionStateResult};
use revenant_activities::{ActivityEvent, ScriptedActivity};
use revenant_actors::{Actor, ActorKind, ActorRegistry};
use revenant_ai::{AiController, AiEvent, RELAY_DRONE_PRESSURE_PROFILE, WARDEN_PRESSURE_PROFILE};
use revenant_combat::{CombatRuntime, PressureProfile};
use revenant_compatibility::{CanonicalClientMessage, ProtocolAdapter, ProtocolGeneration};
use revenant_cooperation::{
    CombatProfile as CooperationCombatProfile, Contributions as DomainCooperationContributions,
    CooperationError, CooperationState as DomainCooperationState,
    LifeState as DomainCooperationLife, MutationDisposition as CooperationDisposition,
    Phase as DomainCooperationPhase, Role as DomainCooperationRole,
    TerminalOutcome as DomainCooperationTerminalOutcome,
    CATALOG_REVISION as COOPERATION_CATALOG_REVISION, MAX_REVIVE_DISTANCE_SQUARED,
    OPERATION_DURATION_MS, PARTICIPANT_COUNT as COOPERATION_PARTICIPANT_COUNT, PING_TTL_MS,
    REVIVE_CHANNEL_MS, REVIVE_HEALTH, REVIVE_WINDOW_MS, REWARD_EXPERIENCE_PER_PARTICIPANT,
    REWARD_FRAGMENTS_PER_PARTICIPANT,
};
use revenant_identity::LocalIdentityService;
use revenant_inventory::{
    weapon_profile, ItemStack, ARC_SIDEARM_PROFILE, COIL_LANCE_PROFILE, PULSE_RIFLE_PROFILE,
    RELAY_CORE_FRAGMENT,
};
use revenant_modules::{
    canonicalize_loadout, resolve_build, validate_operation_id,
    ActivityPhase as ModuleActivityPhase, BaseCombatProfile, CombinationOutcome, LoadoutOutcome,
    ModuleId, OperationDisposition, OperationReceipt, CATALOG_REVISION, CONTENT_CATALOG_REVISION,
    CONTENT_MODULE_CATALOG, MAX_EQUIPPED_MODULES, MODULE_CATALOG,
};
use revenant_objectives::{Objective, ObjectiveKind, ObjectiveState, WorldTrigger};
use revenant_operations::{
    apply_warden_health_effect, event_definition, route_definition,
    OperationDisposition as RouteDisposition, OperationReceipt as RouteReceipt, RouteEventId,
    RouteId, RouteObjectiveId as DomainRouteObjectiveId, RouteOperationState,
    RouteOperationSummary as DomainRouteOperationSummary, RouteSeed, RouteSelectionOutcome,
    RouteSelectionRequest, TerminalOutcome, CATALOG_REVISION as ROUTE_CATALOG_REVISION,
    RESOLVER_REVISION,
};
use revenant_persistence::{
    database_url_from_environment, ActivityCompletion, ActivityParticipantCompletion,
    CompletionRewards, CooperationOperationInput, CooperationParticipant,
    CooperationPersistenceError, CooperationReceipt, ModuleJoinReplayContext,
    ModuleMutationReplayContext, ModulePersistenceError, NewReplayEvent, PersistedModuleState,
    Persistence, Progression as PersistedProgression, RouteOperationSelection, RouteParticipant,
    RoutePersistenceError,
};
use revenant_progression::{Progression as DomainProgression, EXPERIENCE_PER_LEVEL};
use revenant_protocol::{
    read_message, write_message, ActivityComplete, ActivityStart, ActorDestroy, ActorSpawn,
    ActorUpdate, AuthResponse, CharacterListResponse, CharacterSummary, ClientMessage,
    CooperationContributions, CooperationGrant, CooperationLife as WireCooperationLife,
    CooperationLifeCause, CooperationLifeState as WireCooperationLifeState,
    CooperationNoRewardReason, CooperationOperationState,
    CooperationOperationSummary as WireCooperationOperationSummary, CooperationParticipantState,
    CooperationPhase as WireCooperationPhase, CooperationPingIntent, CooperationPingResult,
    CooperationPingState, CooperationReviveIntent, CooperationReviveResult, CooperationReviveState,
    CooperationReviveStatus, CooperationReward, CooperationRole as WireCooperationRole,
    CooperationStartIntent, CooperationStartResult, CooperationState as WireCooperationState,
    CooperationTarget as WireCooperationTarget, CooperationTargetState,
    CooperationTerminalOutcome as WireCooperationTerminalOutcome, CooperationTiming, DamageApplied,
    DoorState, EquipmentChanged, EquipmentSnapshot, InventoryItem, InventorySnapshot, LootGranted,
    ModuleCatalogEntry, ModuleCombineIntent, ModuleCombined, ModuleLoadoutChanged,
    ModuleLoadoutIntent, ModulePreview, ModulePreviewRequest, ModuleSnapshot, ModuleWeaponProfile,
    ObjectiveUpdate, ProgressionGranted, ProgressionSnapshot, ProtocolError, RouteChoiceIntent,
    RouteChoiceResult, RouteEffect as WireRouteEffect, RouteEventCandidate, RouteGrant,
    RouteObjectiveId as WireRouteObjectiveId, RouteObjectiveState as WireRouteObjectiveState,
    RouteObjectiveTransition as WireRouteObjectiveTransition,
    RouteOperationSummary as WireRouteOperationSummary, RouteOption, RoutePhase as WireRoutePhase,
    RouteReward as WireRouteReward, RouteSelection, RouteState, RouteTerminalOutcome, ServerHello,
    ServerMessage, WeaponProfile as WireWeaponProfile, WorldJoinResponse,
    COOPERATION_WIRE_SCHEMA_VERSION, MAX_COOPERATION_MESSAGE_BYTES,
    MAX_COOPERATION_OPERATION_ID_BYTES, MAX_MODULE_ID_BYTES, MAX_MODULE_LOADOUT_ENTRIES,
    MAX_MODULE_OPERATION_ID_BYTES, MAX_ROUTE_MESSAGE_BYTES, PROTOCOL_VERSION,
    ROUTE_WIRE_SCHEMA_VERSION,
};
use revenant_replay::{
    decode_cooperation_payload, decode_module_payload, decode_route_payload, module_state_evidence,
    ReplayEventKind, ReplayObjectiveTransition, ReplayProtocolGeneration,
    ReplayRouteEncounterEvidence, ReplayRouteGrantEvidence, ReplayRouteObjectiveEvidence,
    ReplayRouteObjectiveKind, ReplayRouteObjectiveState, ReplayRouteParticipantEvidence,
    RouteSelectedPayloadV1, RouteTerminalPayloadV1, AUTHORING_REVISION,
    ROUTE_REPLAY_SCHEMA_VERSION,
};
use revenant_world::WorldService;

pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_GAME_ADDR: &str = "127.0.0.1:7000";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const GAMEPLAY_IDLE_TIMEOUT: Duration = Duration::from_mins(5);
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_GAME_CONNECTIONS: usize = 64;
const MAX_UNAUTHENTICATED_CONNECTIONS: usize = 8;
const MAX_HTTP_REQUEST_LINE: usize = 4096;
const MAX_HTTP_HEADER_LINE: usize = 4096;
const MAX_HTTP_HEADER_LINES: usize = 32;
const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const OUTBOUND_QUEUE_CAPACITY: usize = 32;
const FRAME_BURST_LIMIT: usize = 32;
const FRAME_MINUTE_LIMIT: usize = 600;
const FRAME_BURST_WINDOW: Duration = Duration::from_secs(1);
const FRAME_MINUTE_WINDOW: Duration = Duration::from_mins(1);
const DEFAULT_INSPECTOR_ORIGIN: &str = "http://127.0.0.1:4173";
static HANDSHAKE_DATABASE: Mutex<()> = Mutex::new(());
const RELAY_DRONE_BASE_HEALTH: u32 = 140;
const WARDEN_BASE_HEALTH: u32 = 240;

/// Runs the gateway until the process is terminated.
///
/// # Errors
///
/// Returns an I/O error when infrastructure initialization or listener operations fail.
pub fn run(bind_addr: &str, game_addr: &str) -> io::Result<()> {
    let (database_url, expected_players, script_path, inspector_origin) = gateway_configuration()?;
    let session = SharedSession::new(&database_url, &script_path, expected_players)?;
    run_session(
        bind_addr,
        game_addr,
        &database_url,
        expected_players,
        session,
        &inspector_origin,
    )
}

/// Applies the idempotent schema using the explicitly configured migration
/// credential and exits without opening a listener.
///
/// # Errors
///
/// Returns a redacted configuration error or the `PostgreSQL` migration error.
pub fn migrate_database() -> io::Result<()> {
    let database_url = database_url_from_environment().map_err(io::Error::other)?;
    Persistence::connect(&database_url).map_err(|error| {
        io::Error::other(format!("failed to migrate PostgreSQL persistence: {error}"))
    })?;
    println!("{{\"event\":\"database_migration_complete\"}}");
    Ok(())
}

#[cfg(feature = "m27-runtime-matrix")]
/// Runs the separately built M27 matrix gateway with a finite server seed queue.
///
/// This function is absent from default and release builds. Seeds enter at the
/// same non-network domain boundary used by gateway tests.
///
/// # Errors
///
/// Returns an I/O error for invalid seed configuration or gateway failures.
pub fn run_m27_matrix(bind_addr: &str, game_addr: &str, seed_values: Vec<u64>) -> io::Result<()> {
    const MAX_MATRIX_SEEDS: usize = 64;
    if seed_values.is_empty() || seed_values.len() > MAX_MATRIX_SEEDS {
        return Err(io::Error::other(
            "M27 matrix seed queue must contain 1-64 entries",
        ));
    }
    let mut seeds = seed_values
        .into_iter()
        .map(|value| {
            RouteSeed::new(value)
                .map_err(|error| io::Error::other(format!("invalid M27 matrix seed: {error}")))
        })
        .collect::<io::Result<VecDeque<_>>>()?;
    let seed_count = seeds.len();
    let seed_source: RouteSeedSource = Box::new(move || {
        seeds
            .pop_front()
            .ok_or_else(|| io::Error::other("M27 matrix route seed queue was exhausted"))
    });
    let (database_url, expected_players, script_path, inspector_origin) = gateway_configuration()?;
    let session = SharedSession::new_with_route_seed_source(
        &database_url,
        &script_path,
        expected_players,
        seed_source,
    )?;
    println!("{{\"event\":\"m27_matrix_seed_injection_enabled\",\"seed_count\":{seed_count}}}");
    run_session(
        bind_addr,
        game_addr,
        &database_url,
        expected_players,
        session,
        &inspector_origin,
    )
}

fn gateway_configuration() -> io::Result<(String, usize, String, String)> {
    let database_url = database_url_from_environment().map_err(io::Error::other)?;
    let database_mode = database_startup_mode(env::var("REVENANT_DATABASE_MODE").ok().as_deref())?;
    let connection = match database_mode {
        DatabaseStartupMode::Migrate => Persistence::connect(&database_url),
        DatabaseStartupMode::Existing => Persistence::connect_existing(&database_url),
    };
    connection.map_err(|error| {
        io::Error::other(format!(
            "failed to initialize PostgreSQL persistence: {error}"
        ))
    })?;
    let expected_players = env::var("REVENANT_EXPECTED_PLAYERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    if !(1..=2).contains(&expected_players) {
        return Err(io::Error::other(
            "REVENANT_EXPECTED_PLAYERS must be one or two",
        ));
    }
    let script_path = env::var("REVENANT_ACTIVITY_SCRIPT")
        .unwrap_or_else(|_| "scripts/activities/relay_awakening.lua".to_owned());
    let inspector_origin =
        configured_inspector_origin(env::var("REVENANT_INSPECTOR_ORIGIN").ok().as_deref())?;
    Ok((
        database_url,
        expected_players,
        script_path,
        inspector_origin,
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DatabaseStartupMode {
    Migrate,
    Existing,
}

fn database_startup_mode(value: Option<&str>) -> io::Result<DatabaseStartupMode> {
    match value {
        None | Some("migrate") => Ok(DatabaseStartupMode::Migrate),
        Some("existing") => Ok(DatabaseStartupMode::Existing),
        Some(_) => Err(io::Error::other(
            "REVENANT_DATABASE_MODE must be migrate or existing",
        )),
    }
}

fn configured_inspector_origin(value: Option<&str>) -> io::Result<String> {
    let value = value.unwrap_or(DEFAULT_INSPECTOR_ORIGIN);
    let Some(port) = value.strip_prefix("http://127.0.0.1:") else {
        return Err(io::Error::other(
            "REVENANT_INSPECTOR_ORIGIN must use literal IPv4 loopback HTTP",
        ));
    };
    if port.is_empty()
        || !port.bytes().all(|byte| byte.is_ascii_digit())
        || port.parse::<u16>().is_err()
        || port.parse::<u16>().is_ok_and(|port| port == 0)
    {
        return Err(io::Error::other(
            "REVENANT_INSPECTOR_ORIGIN must contain one nonzero decimal port",
        ));
    }
    Ok(value.to_owned())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CorrelationDigest(String);

impl CorrelationDigest {
    fn generate() -> io::Result<Self> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes)
            .map_err(|_| io::Error::other("correlation randomness is unavailable"))?;
        let mut digest = String::with_capacity(32);
        for byte in bytes {
            digest.push(char::from(HEX[usize::from(byte >> 4)]));
            digest.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        Ok(Self(digest))
    }

    #[cfg(test)]
    fn fixture(value: &str) -> Self {
        Self(value.to_owned())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameRateExceeded {
    Second,
    Minute,
    ClockReversed,
}

impl Display for FrameRateExceeded {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Second => "per-second frame limit exceeded",
            Self::Minute => "per-minute frame limit exceeded",
            Self::ClockReversed => "frame clock moved backwards",
        })
    }
}

impl std::error::Error for FrameRateExceeded {}

#[derive(Debug, Default)]
struct FrameRateLimiter {
    second: VecDeque<Duration>,
    minute: VecDeque<Duration>,
}

impl FrameRateLimiter {
    fn observe_at(&mut self, now: Duration) -> Result<(), FrameRateExceeded> {
        if self.minute.back().is_some_and(|previous| now < *previous) {
            return Err(FrameRateExceeded::ClockReversed);
        }
        Self::expire(&mut self.second, now, FRAME_BURST_WINDOW);
        Self::expire(&mut self.minute, now, FRAME_MINUTE_WINDOW);
        if self.second.len() >= FRAME_BURST_LIMIT {
            return Err(FrameRateExceeded::Second);
        }
        if self.minute.len() >= FRAME_MINUTE_LIMIT {
            return Err(FrameRateExceeded::Minute);
        }
        self.second.push_back(now);
        self.minute.push_back(now);
        Ok(())
    }

    fn expire(timestamps: &mut VecDeque<Duration>, now: Duration, window: Duration) {
        while timestamps
            .front()
            .is_some_and(|timestamp| now.saturating_sub(*timestamp) >= window)
        {
            timestamps.pop_front();
        }
    }
}

fn deadline_expired(elapsed: Duration, deadline: Duration) -> bool {
    elapsed > deadline
}

struct DeadlineReader<'a> {
    stream: &'a TcpStream,
    started: Instant,
    budget: Duration,
}

impl Read for DeadlineReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let elapsed = self.started.elapsed();
        if deadline_expired(elapsed, self.budget) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "read deadline expired",
            ));
        }
        self.stream.set_read_timeout(Some(
            self.budget
                .saturating_sub(elapsed)
                .max(Duration::from_nanos(1)),
        ))?;
        let count = self.stream.read(bytes)?;
        if deadline_expired(self.started.elapsed(), self.budget) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "read deadline expired",
            ));
        }
        Ok(count)
    }
}

fn connection_error_category(error: &(dyn std::error::Error + 'static)) -> &'static str {
    if let Some(rate) = error.downcast_ref::<FrameRateExceeded>() {
        return match rate {
            FrameRateExceeded::Second => "rate_second",
            FrameRateExceeded::Minute => "rate_minute",
            FrameRateExceeded::ClockReversed => "internal",
        };
    }
    if let Some(protocol) = error.downcast_ref::<ProtocolError>() {
        return match protocol {
            ProtocolError::Io(error) if is_timeout(error.kind()) => "timeout",
            ProtocolError::Io(_) => "io",
            ProtocolError::Encode(_) | ProtocolError::Decode(_) => "invalid_message",
            ProtocolError::FrameTooLarge(_) => "frame_too_large",
        };
    }
    if let Some(error) = error.downcast_ref::<io::Error>() {
        return if is_timeout(error.kind()) {
            "timeout"
        } else if error.kind() == io::ErrorKind::InvalidData {
            "unexpected_message"
        } else {
            "io"
        };
    }
    if error.downcast_ref::<SessionPersistenceFailure>().is_some() {
        return "persistence";
    }
    "internal"
}

fn is_timeout(kind: io::ErrorKind) -> bool {
    matches!(kind, io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock)
}

#[allow(clippy::too_many_lines)]
fn run_session(
    bind_addr: &str,
    game_addr: &str,
    database_url: &str,
    expected_players: usize,
    session: SharedSession,
    inspector_origin: &str,
) -> io::Result<()> {
    let (command_tx, command_rx) = mpsc::sync_channel(MAX_GAME_CONNECTIONS);
    thread::spawn(move || session.run(command_rx));

    let health_listener = TcpListener::bind(bind_addr)?;
    let game_listener = TcpListener::bind(game_addr)?;
    println!(
        "{{\"event\":\"gateway_started\",\"transport\":\"loopback\",\"protocol_version\":{PROTOCOL_VERSION},\"expected_players\":{expected_players}}}"
    );
    let inspector_database_url = database_url.to_owned();
    let inspector_origin = inspector_origin.to_owned();
    thread::spawn(move || {
        serve_http(&health_listener, &inspector_database_url, &inspector_origin);
    });
    let active_connections = Arc::new(AtomicUsize::new(0));
    let unauthenticated_connections = Arc::new(AtomicUsize::new(0));

    for stream in game_listener.incoming() {
        match stream {
            Ok(stream) => {
                let Ok(correlation) = CorrelationDigest::generate() else {
                    eprintln!(
                        "{}",
                        serde_json::json!({
                            "event": "connection_rejected",
                            "category": "internal"
                        })
                    );
                    continue;
                };
                if active_connections.fetch_add(1, Ordering::Relaxed) >= MAX_GAME_CONNECTIONS {
                    active_connections.fetch_sub(1, Ordering::Relaxed);
                    eprintln!(
                        "{}",
                        serde_json::json!({
                            "event": "connection_rejected",
                            "category": "global_limit",
                            "correlation_digest": correlation.as_str()
                        })
                    );
                    continue;
                }
                let connection_guard = Arc::new(CounterGuard::new(Arc::clone(&active_connections)));
                if unauthenticated_connections.fetch_add(1, Ordering::Relaxed)
                    >= MAX_UNAUTHENTICATED_CONNECTIONS
                {
                    unauthenticated_connections.fetch_sub(1, Ordering::Relaxed);
                    eprintln!(
                        "{}",
                        serde_json::json!({
                            "event": "connection_rejected",
                            "category": "unauthenticated_limit",
                            "correlation_digest": correlation.as_str()
                        })
                    );
                    drop(connection_guard);
                    continue;
                }
                let unauthenticated_guard =
                    CounterGuard::new(Arc::clone(&unauthenticated_connections));
                let database_url = database_url.to_owned();
                let command_tx = command_tx.clone();
                println!(
                    "{}",
                    serde_json::json!({
                        "event": "connection_admitted",
                        "correlation_digest": correlation.as_str()
                    })
                );
                let _ = thread::Builder::new().spawn(move || {
                    let result = handle_game_connection(
                        stream,
                        &database_url,
                        &command_tx,
                        unauthenticated_guard,
                        &correlation,
                        &connection_guard,
                    );
                    if let Err(error) = result {
                        eprintln!(
                            "{}",
                            serde_json::json!({
                                "event": "connection_closed",
                                "category": connection_error_category(error.as_ref()),
                                "correlation_digest": correlation.as_str()
                            })
                        );
                    } else {
                        println!(
                            "{}",
                            serde_json::json!({
                                "event": "connection_closed",
                                "category": "complete",
                                "correlation_digest": correlation.as_str()
                            })
                        );
                    }
                });
            }
            Err(_) => eprintln!(
                "{}",
                serde_json::json!({
                    "event": "connection_rejected",
                    "category": "accept"
                })
            ),
        }
    }
    Ok(())
}

struct CounterGuard {
    counter: Arc<AtomicUsize>,
    active: bool,
}

impl CounterGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self {
            counter,
            active: true,
        }
    }

    fn release(&mut self) {
        if self.active {
            self.counter.fetch_sub(1, Ordering::Relaxed);
            self.active = false;
        }
    }
}

impl Drop for CounterGuard {
    fn drop(&mut self) {
        self.release();
    }
}

fn serve_http(listener: &TcpListener, database_url: &str, inspector_origin: &str) {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(HTTP_TIMEOUT));
                let _ = stream.set_write_timeout(Some(HTTP_TIMEOUT));
                let correlation = CorrelationDigest::generate().ok();
                let response = match read_http_request(DeadlineReader {
                    stream: &stream,
                    started: Instant::now(),
                    budget: HTTP_TIMEOUT,
                }) {
                    Ok(request) => http_response(
                        &request,
                        database_url,
                        inspector_origin,
                        correlation.as_ref(),
                    ),
                    Err(error) => {
                        emit_http_rejection(
                            correlation.as_ref(),
                            if is_timeout(error.kind()) {
                                "timeout"
                            } else {
                                "invalid_request"
                            },
                        );
                        HttpResponse::json("400 Bad Request", "{\"error\":\"bad_request\"}\n", None)
                    }
                };
                let _ = stream.write_all(response.render().as_bytes());
            }
            Err(_) => emit_http_rejection(None, "accept"),
        }
    }
}

fn http_response(
    request: &HttpRequest,
    database_url: &str,
    inspector_origin: &str,
    correlation: Option<&CorrelationDigest>,
) -> HttpResponse {
    handle_http_request(request, database_url, inspector_origin).unwrap_or_else(|_| {
        emit_http_rejection(correlation, "internal");
        let allowed_origin = (request.method == "GET"
            && request.path.starts_with("/api/inspector")
            && request.origin.as_deref() == Some(inspector_origin))
        .then_some(inspector_origin);
        HttpResponse::json(
            "500 Internal Server Error",
            "{\"error\":\"internal_error\"}\n",
            allowed_origin,
        )
    })
}

fn emit_http_rejection(correlation: Option<&CorrelationDigest>, category: &'static str) {
    eprintln!(
        "{}",
        serde_json::json!({
            "event": "http_request_rejected",
            "category": category,
            "correlation_digest": correlation.map(CorrelationDigest::as_str)
        })
    );
}

struct HttpRequest {
    method: String,
    path: String,
    origin: Option<String>,
}

struct HttpResponse {
    status: &'static str,
    body: String,
    allowed_origin: Option<String>,
}

impl HttpResponse {
    fn json(status: &'static str, body: impl Into<String>, allowed_origin: Option<&str>) -> Self {
        Self {
            status,
            body: body.into(),
            allowed_origin: allowed_origin.map(str::to_owned),
        }
    }

    fn render(&self) -> String {
        let cors = self
            .allowed_origin
            .as_ref()
            .map_or_else(String::new, |origin| {
                format!("Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n")
            });
        format!(
            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status,
            cors,
            self.body.len(),
            self.body
        )
    }
}

#[allow(clippy::too_many_lines)]
fn handle_http_request(
    request: &HttpRequest,
    database_url: &str,
    inspector_origin: &str,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    if request.method != "GET" {
        return Ok(HttpResponse::json(
            "405 Method Not Allowed",
            "{\"error\":\"method_not_allowed\"}\n",
            None,
        ));
    }
    let inspector_request = request.path.starts_with("/api/inspector");
    if inspector_request && request.origin.as_deref() != Some(inspector_origin) {
        return Ok(HttpResponse::json(
            "403 Forbidden",
            "{\"error\":\"origin_forbidden\"}\n",
            None,
        ));
    }
    let allowed_origin = inspector_request.then_some(inspector_origin);
    match inspector_route(&request.path) {
        InspectorRoute::Health => Ok(HttpResponse::json(
            "200 OK",
            "{\"status\":\"ok\",\"service\":\"revenant-gateway\"}\n",
            None,
        )),
        InspectorRoute::Sessions => {
            let mut persistence = Persistence::connect_existing(database_url)?;
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
            Ok(HttpResponse::json(
                "200 OK",
                serde_json::json!({ "sessions": sessions }).to_string(),
                allowed_origin,
            ))
        }
        InspectorRoute::Summary(session_id) => {
            let mut persistence = Persistence::connect_existing(database_url)?;
            let Some(summary) = persistence.authoritative_session_summary(session_id)? else {
                return Ok(HttpResponse::json(
                    "404 Not Found",
                    "{\"error\":\"session_not_found\"}\n",
                    allowed_origin,
                ));
            };
            Ok(HttpResponse::json(
                "200 OK",
                serde_json::json!({
                    "summary": {
                        "session_id": summary.session_id,
                        "activity_id": summary.activity_id,
                        "first_joined_at": summary.first_joined_at,
                        "activity_started_at": summary.activity_started_at,
                        "activity_ended_at": summary.activity_ended_at,
                        "join_to_start_ms": summary.join_to_start_ms,
                        "activity_duration_ms": summary.activity_duration_ms,
                        "participant_count": summary.participant_count,
                        "completed": summary.completed,
                        "enemy_spawn_count": summary.enemy_spawn_count,
                        "enemy_defeat_count": summary.enemy_defeat_count,
                        "boss_spawned": summary.boss_spawned,
                        "equipment_change_count": summary.equipment_change_count,
                        "loot_grant_count": summary.loot_grant_count,
                        "progression_grant_count": summary.progression_grant_count,
                        "module_replay_legacy": summary.module_replay_legacy,
                        "module_participant_count": summary.module_participant_count,
                        "module_snapshot_count": summary.module_snapshot_count,
                        "module_combination_count": summary.module_combination_count,
                        "module_loadout_change_count": summary.module_loadout_change_count,
                        "acquisition_claim_count": summary.acquisition_claim_count,
                        "route_replay_legacy": summary.route_replay_legacy,
                        "route_id": summary.route_id,
                        "route_event_id": summary.route_event_id,
                        "route_terminal_outcome": summary.route_terminal_outcome,
                        "route_elapsed_ms": summary.route_elapsed_ms,
                        "route_transition_count": summary.route_transition_count,
                        "route_reward_participant_count": summary.route_reward_participant_count,
                        "cooperation_replay_legacy": summary.cooperation_replay_legacy,
                        "cooperation_replay_state": summary.cooperation_replay_state,
                        "cooperation_last_phase": summary.cooperation_last_phase,
                        "cooperation_terminal_outcome": summary.cooperation_terminal_outcome,
                        "cooperation_terminal_elapsed_ms": summary.cooperation_terminal_elapsed_ms,
                        "cooperation_participant_count": summary.cooperation_participant_count,
                        "cooperation_contribution_count": summary.cooperation_contribution_count,
                        "cooperation_revive_count": summary.cooperation_revive_count,
                        "cooperation_reward_participant_count": summary.cooperation_reward_participant_count,
                        "event_count": summary.event_count,
                    }
                })
                .to_string(),
                allowed_origin,
            ))
        }
        InspectorRoute::Events(session_id) => Ok(HttpResponse::json(
            "200 OK",
            inspector_events_body(database_url, session_id)?,
            allowed_origin,
        )),
        InspectorRoute::NotFound => Ok(HttpResponse::json(
            "404 Not Found",
            "{\"error\":\"not_found\"}\n",
            allowed_origin,
        )),
    }
}

fn read_http_request(reader: impl Read) -> io::Result<HttpRequest> {
    let mut reader = BufReader::new(reader);
    let request_line = read_bounded_line(&mut reader, MAX_HTTP_REQUEST_LINE)?;
    let (method, path) = http_request_line_fields(&request_line)?;
    let mut origin = None;
    let mut header_lines = 0_usize;
    let mut header_bytes = 0_usize;
    loop {
        let line = read_bounded_line(&mut reader, MAX_HTTP_HEADER_LINE)?;
        header_bytes = header_bytes
            .checked_add(line.len())
            .ok_or_else(|| io::Error::other("HTTP header byte count overflowed"))?;
        if header_bytes > MAX_HTTP_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP headers are too large",
            ));
        }
        if matches!(line.as_str(), "\n" | "\r\n") {
            break;
        }
        header_lines += 1;
        if header_lines > MAX_HTTP_HEADER_LINES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP has too many header lines",
            ));
        }
        let header = line.trim_end_matches(['\r', '\n']);
        let Some((name, value)) = header.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP header has an invalid shape",
            ));
        };
        if name.eq_ignore_ascii_case("origin") {
            if origin.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "HTTP Origin header is duplicated",
                ));
            }
            origin = Some(value.trim().to_owned());
        }
    }
    Ok(HttpRequest {
        method: method.to_owned(),
        path: path.to_owned(),
        origin,
    })
}

fn http_request_line_fields(request_line: &str) -> io::Result<(&str, &str)> {
    let mut fields = request_line.split_whitespace();
    let method = fields
        .next()
        .ok_or_else(|| io::Error::other("HTTP request method is missing"))?;
    let path = fields
        .next()
        .ok_or_else(|| io::Error::other("HTTP request path is missing"))?;
    let version = fields
        .next()
        .ok_or_else(|| io::Error::other("HTTP request version is missing"))?;
    if fields.next().is_some() || !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "HTTP request line has an invalid shape or version",
        ));
    }
    Ok((method, path))
}

#[cfg(test)]
fn inspector_request_path(request_line: &str) -> io::Result<Option<&str>> {
    let (method, path) = http_request_line_fields(request_line)?;
    Ok((method == "GET").then_some(path))
}

fn inspector_events_body(
    database_url: &str,
    session_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut persistence = Persistence::connect_existing(database_url)?;
    let events = persistence.replay_events(session_id)?;
    if events.iter().any(|event| {
        event
            .event_type
            .parse::<ReplayEventKind>()
            .is_ok_and(|kind| kind.is_route_event() || kind.is_cooperation_event())
    }) {
        persistence
            .authoritative_session_summary(session_id)?
            .ok_or_else(|| io::Error::other("structured replay session has no summary"))?;
    }
    let events = events
        .into_iter()
        .map(|event| {
            let kind = event.event_type.parse::<ReplayEventKind>()?;
            let decoded_payload =
                if let Some(payload) = decode_module_payload(kind, &event.payload)? {
                    Some(serde_json::to_value(payload)?)
                } else if let Some(payload) = decode_route_payload(kind, &event.payload)? {
                    Some(serde_json::to_value(payload)?)
                } else if let Some(payload) = decode_cooperation_payload(kind, &event.payload)? {
                    Some(serde_json::to_value(payload)?)
                } else if kind == ReplayEventKind::AcquisitionClaimed {
                    Some(serde_json::to_value(
                        revenant_replay::decode_acquisition_claimed(&event.payload)?,
                    )?)
                } else {
                    None
                };
            Ok::<_, Box<dyn std::error::Error>>(serde_json::json!({
                "event_id": event.id,
                "event_type": event.event_type,
                "timestamp": event.timestamp,
                "session_id": event.session_id,
                "account_id": event.account_id,
                "activity_id": event.activity_id,
                "actor_id": event.actor_id,
                "payload": event.payload,
                "decoded_payload": decoded_payload,
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::json!({ "events": events }).to_string())
}

#[cfg(test)]
fn read_http_request_line(reader: impl Read) -> io::Result<String> {
    read_bounded_line(&mut BufReader::new(reader), MAX_HTTP_REQUEST_LINE)
}

fn read_bounded_line(reader: &mut impl BufRead, limit: usize) -> io::Result<String> {
    let mut bytes = Vec::with_capacity(128);
    reader
        .take((limit + 1) as u64)
        .read_until(b'\n', &mut bytes)?;
    if bytes.last() != Some(&b'\n') || bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "HTTP line is incomplete or too large",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "HTTP request line is not UTF-8"))
}

#[derive(Debug, PartialEq, Eq)]
enum InspectorRoute<'a> {
    Health,
    Sessions,
    Summary(&'a str),
    Events(&'a str),
    NotFound,
}

fn inspector_route(path: &str) -> InspectorRoute<'_> {
    if path == "/health" {
        InspectorRoute::Health
    } else if path == "/api/inspector/sessions" {
        InspectorRoute::Sessions
    } else {
        let Some(suffix) = path.strip_prefix("/api/inspector/sessions/") else {
            return InspectorRoute::NotFound;
        };
        let (session_id, route) = if let Some(session_id) = suffix.strip_suffix("/summary") {
            (session_id, InspectorRoute::Summary(session_id))
        } else if let Some(session_id) = suffix.strip_suffix("/events") {
            (session_id, InspectorRoute::Events(session_id))
        } else {
            return InspectorRoute::NotFound;
        };
        if !session_id.is_empty()
            && session_id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            route
        } else {
            InspectorRoute::NotFound
        }
    }
}

#[allow(clippy::too_many_lines)]
fn handle_game_connection(
    mut stream: TcpStream,
    database_url: &str,
    command_tx: &SyncSender<SessionCommand>,
    mut unauthenticated_guard: CounterGuard,
    correlation: &CorrelationDigest,
    connection_guard: &Arc<CounterGuard>,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
    stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
    let connection_started = Instant::now();
    let mut frame_rate = FrameRateLimiter::default();
    let ClientMessage::ClientHello(hello) =
        read_client_frame(&mut stream, &mut frame_rate, connection_started)?
    else {
        return Err(unexpected_message("ClientHello"));
    };
    let negotiated = ProtocolAdapter::negotiate(hello.protocol_version);
    let accepted = negotiated.is_ok();
    let mastery_challenge_capable = hello.protocol_version == PROTOCOL_VERSION
        && hello.content_revision.as_deref()
            == Some(revenant_protocol::MASTERY_CHALLENGE_CONTENT_REVISION);
    let modifier_challenge_capable = mastery_challenge_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::MODIFIER_CHALLENGE_CONTENT_REVISION));
    let gauntlet_challenge_capable = modifier_challenge_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::GAUNTLET_CHALLENGE_CONTENT_REVISION));
    let survival_challenge_capable = gauntlet_challenge_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::SURVIVAL_CHALLENGE_CONTENT_REVISION));
    let signal_challenge_capable = survival_challenge_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::SIGNAL_CHALLENGE_CONTENT_REVISION));
    let prism_challenge_capable = signal_challenge_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::PRISM_CHALLENGE_CONTENT_REVISION));
    let elite_challenge_capable = prism_challenge_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::ELITE_CHALLENGE_CONTENT_REVISION));
    let recovery_capable = elite_challenge_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::RECOVERY_CONTENT_REVISION));
    let challenge_policy = if modifier_challenge_capable {
        revenant_challenges::MODIFIER_REVISION
    } else if gauntlet_challenge_capable {
        revenant_challenges::GAUNTLET_REVISION
    } else if survival_challenge_capable {
        revenant_challenges::SURVIVAL_REVISION
    } else if signal_challenge_capable {
        revenant_challenges::SIGNAL_REVISION
    } else if prism_challenge_capable {
        revenant_challenges::PRISM_REVISION
    } else if elite_challenge_capable {
        revenant_challenges::ELITE_REVISION
    } else if recovery_capable {
        revenant_challenges::EXPANDED_REVISION
    } else {
        revenant_challenges::REVISION
    };
    let challenge_capable = recovery_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::CHALLENGE_CONTENT_REVISION));
    let campaign_capable = challenge_capable
        || hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::CAMPAIGN_CONTENT_REVISION);
    let acquisition_capable = campaign_capable
        || hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::ACQUISITION_CONTENT_REVISION);
    let build_capable = acquisition_capable
        || hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref() == Some(revenant_protocol::BUILD_CONTENT_REVISION);
    let arsenal_capable = build_capable
        || hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::ARSENAL_CONTENT_REVISION);
    let prism_capable = arsenal_capable
        || hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref() == Some(revenant_protocol::PRISM_CONTENT_REVISION);
    let elite_capable = prism_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::ELITE_CONTENT_REVISION));
    let support_capable = elite_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::SUPPORT_CONTENT_REVISION));
    let bulwark_capable = support_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::DEFENSE_CONTENT_REVISION));
    let encounter_capable = bulwark_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::ENCOUNTER_CONTENT_REVISION));
    let exploration_capable = encounter_capable
        || hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref()
                == Some(revenant_protocol::EXPLORATION_CONTENT_REVISION);
    let content_capable = exploration_capable
        || (hello.protocol_version == PROTOCOL_VERSION
            && hello.content_revision.as_deref() == Some(revenant_protocol::CONTENT_REVISION));
    let selected_version = negotiated
        .as_ref()
        .map_or(PROTOCOL_VERSION, |adapter| adapter.wire_version());
    write_message(
        &mut stream,
        &ServerMessage::ServerHello(ServerHello {
            protocol_version: selected_version,
            server_name: "revenant-core".to_owned(),
            content_revision: content_capable.then(|| hello.content_revision.clone().unwrap()),
            accepted,
            message: if accepted {
                "handshake accepted"
            } else {
                "unsupported protocol version"
            }
            .to_owned(),
        }),
    )?;
    println!(
        "{}",
        serde_json::json!({
            "event": "protocol_negotiated",
            "outcome": if accepted { "accepted" } else { "rejected" },
            "category": if accepted { "supported" } else { "unsupported_version" },
            "generation": negotiated.as_ref().ok().map(|adapter| match adapter.generation() {
                ProtocolGeneration::FrozenV1 => "v1",
                ProtocolGeneration::CurrentV2 => "v2"
            }),
            "correlation_digest": correlation.as_str()
        })
    );
    if !accepted {
        return Ok(());
    }
    let adapter = negotiated.expect("accepted negotiation must contain an adapter");

    let CanonicalClientMessage::AuthRequest(request) =
        read_canonical(&mut stream, adapter, &mut frame_rate, connection_started)?
    else {
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
            println!(
                "{}",
                serde_json::json!({
                    "event": "authentication_result",
                    "outcome": "rejected",
                    "category": "invalid_identity",
                    "correlation_digest": correlation.as_str()
                })
            );
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
    unauthenticated_guard.release();
    println!(
        "{}",
        serde_json::json!({
            "event": "authentication_result",
            "outcome": "accepted",
            "correlation_digest": correlation.as_str()
        })
    );
    let CanonicalClientMessage::CharacterListRequest(_) =
        read_canonical(&mut stream, adapter, &mut frame_rate, connection_started)?
    else {
        return Err(unexpected_message("CharacterListRequest"));
    };
    let database_guard = HANDSHAKE_DATABASE
        .lock()
        .map_err(|_| io::Error::other("handshake database admission failed"))?;
    let mut persistence = Persistence::connect_existing(database_url)?;
    persistence.ensure_local_account(&account.id, &account.username)?;
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
    drop(persistence);
    drop(database_guard);
    let character_ids = characters
        .iter()
        .map(|character| character.character_id.clone())
        .collect::<Vec<_>>();
    write_message(
        &mut stream,
        &ServerMessage::CharacterListResponse(CharacterListResponse { characters }),
    )?;

    let mut entry = read_canonical(&mut stream, adapter, &mut frame_rate, connection_started)?;
    if let CanonicalClientMessage::ChallengeStateRequest(query) = entry {
        let guard = HANDSHAKE_DATABASE
            .lock()
            .map_err(|_| io::Error::other("challenge database admission failed"))?;
        let board = challenges::board(
            &mut Persistence::connect_existing(database_url)?,
            &account.id,
            &character_ids,
            campaign_capable,
            challenge_policy,
            mastery_challenge_capable,
            &query,
        )?;
        drop(guard);
        write_message(&mut stream, &ServerMessage::ChallengeSnapshot(board))?;
        entry = read_canonical(&mut stream, adapter, &mut frame_rate, connection_started)?;
    }
    if let CanonicalClientMessage::CampaignStateRequest(query) = entry {
        if !campaign_capable || !character_ids.contains(&query.character_id) {
            return Err(io::Error::other(
                "campaign query requires current capability and owned character",
            )
            .into());
        }
        let guard = HANDSHAKE_DATABASE
            .lock()
            .map_err(|_| io::Error::other("campaign database admission failed"))?;
        let state = Persistence::connect_existing(database_url)?
            .campaign_state_for(&account.id, &query.character_id)
            .map_err(|e| e as Box<dyn std::error::Error>)?;
        drop(guard);
        write_message(
            &mut stream,
            &ServerMessage::CampaignSnapshot(campaign::snapshot(&state)),
        )?;
        entry = read_canonical(&mut stream, adapter, &mut frame_rate, connection_started)?;
    }
    let (request, campaign_entry, challenge_entry) = match entry {
        CanonicalClientMessage::WorldJoinRequest(request) => (request, None, None),
        CanonicalClientMessage::CampaignJoinRequest(request) if campaign_capable => (
            revenant_protocol::WorldJoinRequest {
                character_id: request.character_id.clone(),
            },
            Some(request),
            None,
        ),
        CanonicalClientMessage::ChallengeJoinRequest(request) if challenge_capable => (
            revenant_protocol::WorldJoinRequest {
                character_id: request.character_id.clone(),
            },
            None,
            Some(challenges::entry(&request, challenge_policy)?),
        ),
        _ => {
            return Err(unexpected_message(
                "WorldJoinRequest or negotiated campaign/challenge entry",
            ))
        }
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
    let database_guard = HANDSHAKE_DATABASE
        .lock()
        .map_err(|_| io::Error::other("handshake database admission failed"))?;
    let mut persistence = Persistence::connect_existing(database_url)?;
    if content_capable {
        persistence.ensure_content_equipment(&request.character_id)?;
    }
    if arsenal_capable {
        persistence.ensure_arsenal_equipment(&request.character_id)?;
    }
    let LoadedCharacterState {
        inventory,
        progression,
        equipped_weapon_item_id: persisted_weapon_item_id,
        module_state,
    } = load_character_state(&mut persistence, &request.character_id)?;
    drop(persistence);
    drop(database_guard);
    if adapter.generation() == ProtocolGeneration::CurrentV2
        && module_state.loadout.iter().any(|id| {
            (!content_capable && !ModuleId::LEGACY.contains(id))
                || (!build_capable && !ModuleId::CONTENT.contains(id))
        })
    {
        write_message(&mut stream, &ServerMessage::WorldJoinResponse(WorldJoinResponse {
            accepted: false, world_id: String::new(), player_actor_id: 0, spawn_position: [0, 0, 0],
            message: "This saved loadout requires the current content client. Unequip its new modules there to use an older client.".to_owned(),
        }))?;
        return Ok(());
    }
    let admitted_modules = resolved_arsenal_projection(
        &module_state,
        adapter.generation() == ProtocolGeneration::CurrentV2,
        content_capable,
        if build_capable {
            Some(revenant_modules::BUILD_CATALOG_REVISION)
        } else {
            arsenal_capable.then_some(revenant_modules::ARSENAL_CATALOG_REVISION)
        },
    )?;
    let equipped_weapon_item_id = if adapter.generation() == ProtocolGeneration::FrozenV1
        || (!content_capable && persisted_weapon_item_id == COIL_LANCE_PROFILE.item_id)
        || (!arsenal_capable
            && revenant_inventory::ARSENAL_WEAPONS.contains(&persisted_weapon_item_id.as_str()))
    {
        PULSE_RIFLE_PROFILE.item_id.to_owned()
    } else {
        persisted_weapon_item_id
    };
    let player = WorldService.join(&request.character_id);
    let (outbound_tx, outbound_rx) = OutboundSender::channel(correlation.clone());
    let (join_result_tx, join_result_rx) = mpsc::channel();
    command_tx.send(SessionCommand::Join {
        participant: Box::new(Participant {
            account_id: account.id,
            character_id: request.character_id,
            correlation: correlation.clone(),
            actor: Actor {
                id: player.actor_id,
                kind: ActorKind::Player,
                archetype: "operator".to_owned(),
                position: [0, 0, 0],
                health: admitted_modules.max_health,
                max_health: admitted_modules.max_health,
            },
            outbound: outbound_tx,
            protocol_generation: adapter.generation(),
            content_capable,
            exploration_capable,
            encounter_capable,
            bulwark_capable,
            support_capable,
            elite_capable,
            prism_capable,
            arsenal_capable,
            build_capable,
            acquisition_capable,
            campaign_entry,
            challenge_entry,
            owned_items: inventory.iter().map(|item| item.item_id.clone()).collect(),
            equipped_weapon_item_id: equipped_weapon_item_id.clone(),
            module_state,
            admitted_module_weapons: admitted_modules.weapons.clone(),
            admitted_max_health: admitted_modules.max_health,
            route_capable: false,
            cooperation_capable: false,
        }),
        result: join_result_tx,
    })?;
    let spawn_position = match join_result_rx.recv()? {
        Ok(position) => position,
        Err(message) => {
            write_message(
                &mut stream,
                &ServerMessage::WorldJoinResponse(WorldJoinResponse {
                    accepted: false,
                    world_id: player.world_id,
                    player_actor_id: 0,
                    spawn_position: [0, 0, 0],
                    message,
                }),
            )?;
            return Ok(());
        }
    };
    let _participant_guard = ParticipantConnectionGuard {
        command_tx: command_tx.clone(),
        player_id: player.actor_id,
    };
    write_message(
        &mut stream,
        &ServerMessage::WorldJoinResponse(WorldJoinResponse {
            accepted: true,
            world_id: player.world_id,
            player_actor_id: player.actor_id,
            spawn_position,
            message: "world join accepted".to_owned(),
        }),
    )?;
    if adapter.generation() == ProtocolGeneration::CurrentV2 {
        write_message(
            &mut stream,
            &ServerMessage::InventorySnapshot(InventorySnapshot {
                items: inventory
                    .into_iter()
                    .map(|item| InventoryItem {
                        item_id: item.item_id,
                        quantity: item.quantity,
                    })
                    .collect(),
            }),
        )?;
        write_message(
            &mut stream,
            &ServerMessage::ProgressionSnapshot(progression_message(progression)),
        )?;
        write_message(
            &mut stream,
            &ServerMessage::EquipmentSnapshot(EquipmentSnapshot {
                equipped_weapon_item_id,
                weapons: admitted_modules
                    .weapons
                    .iter()
                    .map(equipment_weapon_profile)
                    .collect(),
            }),
        )?;
    }

    let mut writer = stream.try_clone()?;
    let writer_connection_guard = Arc::clone(connection_guard);
    thread::Builder::new().spawn(move || {
        let _connection_guard = writer_connection_guard;
        write_outbound(&mut writer, outbound_rx);
    })?;
    command_tx.send(SessionCommand::Start(player.actor_id))?;
    stream.set_read_timeout(Some(GAMEPLAY_IDLE_TIMEOUT))?;

    loop {
        match read_canonical(&mut stream, adapter, &mut frame_rate, connection_started) {
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
            Ok(CanonicalClientMessage::ChallengeAbandonIntent(intent)) if challenge_capable => {
                command_tx.send(SessionCommand::ChallengeAbandon {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::CampaignStoryIntent(intent)) if campaign_capable => {
                command_tx.send(SessionCommand::CampaignStory {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::EquipIntent(intent)) => {
                command_tx.send(SessionCommand::Equip {
                    player_id: player.actor_id,
                    item_id: intent.item_id,
                })?;
            }
            Ok(CanonicalClientMessage::AcquisitionStateRequest(_)) => {
                command_tx.send(SessionCommand::AcquisitionState {
                    player_id: player.actor_id,
                })?;
            }
            Ok(CanonicalClientMessage::AcquisitionClaimIntent(intent)) => {
                command_tx.send(SessionCommand::AcquisitionClaim {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::ModuleStateRequest(_)) => {
                command_tx.send(SessionCommand::ModuleState {
                    player_id: player.actor_id,
                })?;
            }
            Ok(CanonicalClientMessage::ModulePreviewRequest(intent)) => {
                command_tx.send(SessionCommand::ModulePreview {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::ModuleCombineIntent(intent)) => {
                command_tx.send(SessionCommand::ModuleCombine {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::ModuleLoadoutIntent(intent)) => {
                command_tx.send(SessionCommand::ModuleLoadout {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::RouteStateRequest(_)) => {
                command_tx.send(SessionCommand::RouteState {
                    player_id: player.actor_id,
                })?;
            }
            Ok(CanonicalClientMessage::RouteChoiceIntent(intent)) => {
                command_tx.send(SessionCommand::RouteChoice {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::CooperationStateRequest(_)) => {
                command_tx.send(SessionCommand::CooperationState {
                    player_id: player.actor_id,
                })?;
            }
            Ok(CanonicalClientMessage::CooperationStartIntent(intent)) => {
                command_tx.send(SessionCommand::CooperationStart {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::CooperationPingIntent(intent)) => {
                command_tx.send(SessionCommand::CooperationPing {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(CanonicalClientMessage::CooperationReviveIntent(intent)) => {
                command_tx.send(SessionCommand::CooperationRevive {
                    player_id: player.actor_id,
                    intent,
                })?;
            }
            Ok(_) => return Err(unexpected_message("gameplay or module intent")),
            Err(error) => {
                if let Some(rate) = error.downcast_ref::<FrameRateExceeded>() {
                    eprintln!(
                        "{}",
                        serde_json::json!({
                            "event": "frame_rate_rejected",
                            "category": match rate {
                                FrameRateExceeded::Second => "rate_second",
                                FrameRateExceeded::Minute => "rate_minute",
                                FrameRateExceeded::ClockReversed => "internal"
                            },
                            "correlation_digest": correlation.as_str()
                        })
                    );
                }
                return Err(error);
            }
        }
    }
}

fn read_canonical(
    stream: &mut TcpStream,
    adapter: ProtocolAdapter,
    frame_rate: &mut FrameRateLimiter,
    connection_started: Instant,
) -> Result<CanonicalClientMessage, Box<dyn std::error::Error>> {
    let wire_message = read_client_frame(stream, frame_rate, connection_started)?;
    Ok(adapter.canonicalize(wire_message)?)
}

fn read_client_frame(
    stream: &mut TcpStream,
    frame_rate: &mut FrameRateLimiter,
    connection_started: Instant,
) -> Result<ClientMessage, Box<dyn std::error::Error>> {
    let budget = stream.read_timeout()?.unwrap_or(HANDSHAKE_TIMEOUT);
    let mut reader = DeadlineReader {
        stream,
        started: if budget == HANDSHAKE_TIMEOUT {
            connection_started
        } else {
            Instant::now()
        },
        budget,
    };
    let result = read_message(&mut reader);
    stream.set_read_timeout(Some(budget))?;
    let message = result?;
    frame_rate.observe_at(connection_started.elapsed())?;
    Ok(message)
}

struct ParticipantConnectionGuard {
    command_tx: SyncSender<SessionCommand>,
    player_id: u64,
}

impl Drop for ParticipantConnectionGuard {
    fn drop(&mut self) {
        let _ = self
            .command_tx
            .send(SessionCommand::Disconnect(self.player_id));
    }
}

#[derive(Clone)]
struct OutboundSender {
    sender: Arc<Mutex<Option<SyncSender<ServerMessage>>>>,
    correlation: CorrelationDigest,
}

impl OutboundSender {
    fn channel(correlation: CorrelationDigest) -> (Self, Receiver<ServerMessage>) {
        let (sender, receiver) = mpsc::sync_channel(OUTBOUND_QUEUE_CAPACITY);
        (
            Self {
                sender: Arc::new(Mutex::new(Some(sender))),
                correlation,
            },
            receiver,
        )
    }

    fn send(&self, message: ServerMessage) -> Result<(), ()> {
        let Ok(mut slot) = self.sender.lock() else {
            return Err(());
        };
        let Some(sender) = slot.as_ref() else {
            return Err(());
        };
        match sender.try_send(message) {
            Ok(()) => Ok(()),
            Err(error) => {
                let category = match error {
                    TrySendError::Full(_) => "queue_full",
                    TrySendError::Disconnected(_) => "queue_disconnected",
                };
                *slot = None;
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "event": "outbound_queue_closed",
                        "category": category,
                        "correlation_digest": self.correlation.as_str()
                    })
                );
                Err(())
            }
        }
    }
}

fn write_outbound(stream: &mut TcpStream, messages: Receiver<ServerMessage>) {
    for message in messages {
        if write_message(stream, &message).is_err() {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

enum SessionCommand {
    ChallengeAbandon {
        player_id: u64,
        intent: revenant_protocol::ChallengeAbandonIntent,
    },
    CampaignStory {
        player_id: u64,
        intent: revenant_protocol::CampaignStoryIntent,
    },
    Join {
        participant: Box<Participant>,
        result: Sender<Result<[i32; 3], String>>,
    },
    Start(u64),
    Attack {
        player_id: u64,
        target_id: u64,
    },
    Move {
        player_id: u64,
        position: [i32; 3],
    },
    Equip {
        player_id: u64,
        item_id: String,
    },
    AcquisitionState {
        player_id: u64,
    },
    AcquisitionClaim {
        player_id: u64,
        intent: revenant_protocol::AcquisitionClaimIntent,
    },
    ModuleState {
        player_id: u64,
    },
    ModulePreview {
        player_id: u64,
        intent: ModulePreviewRequest,
    },
    ModuleCombine {
        player_id: u64,
        intent: ModuleCombineIntent,
    },
    ModuleLoadout {
        player_id: u64,
        intent: ModuleLoadoutIntent,
    },
    RouteState {
        player_id: u64,
    },
    RouteChoice {
        player_id: u64,
        intent: RouteChoiceIntent,
    },
    CooperationState {
        player_id: u64,
    },
    CooperationStart {
        player_id: u64,
        intent: CooperationStartIntent,
    },
    CooperationPing {
        player_id: u64,
        intent: CooperationPingIntent,
    },
    CooperationRevive {
        player_id: u64,
        intent: CooperationReviveIntent,
    },
    Disconnect(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedModuleProjection {
    weapons: Vec<ModuleWeaponProfile>,
    max_health: u32,
}

// Independent negotiated features, not mutually exclusive session phases.
#[allow(clippy::struct_excessive_bools)]
struct Participant {
    account_id: String,
    character_id: String,
    correlation: CorrelationDigest,
    actor: Actor,
    outbound: OutboundSender,
    protocol_generation: ProtocolGeneration,
    content_capable: bool,
    owned_items: Vec<String>,
    exploration_capable: bool,
    encounter_capable: bool,
    bulwark_capable: bool,
    support_capable: bool,
    elite_capable: bool,
    prism_capable: bool,
    arsenal_capable: bool,
    build_capable: bool,
    acquisition_capable: bool,
    campaign_entry: Option<revenant_protocol::CampaignJoinRequest>,
    challenge_entry: Option<challenges::Entry>,
    equipped_weapon_item_id: String,
    module_state: PersistedModuleState,
    admitted_module_weapons: Vec<ModuleWeaponProfile>,
    admitted_max_health: u32,
    route_capable: bool,
    cooperation_capable: bool,
}

impl Participant {
    fn accepts_module(&self, module_id: ModuleId) -> bool {
        (self.content_capable || ModuleId::LEGACY.contains(&module_id))
            && (self.build_capable || ModuleId::CONTENT.contains(&module_id))
    }

    fn arsenal_catalog(&self) -> Option<&'static str> {
        if self.build_capable {
            Some(revenant_modules::BUILD_CATALOG_REVISION)
        } else {
            self.arsenal_capable
                .then_some(revenant_modules::ARSENAL_CATALOG_REVISION)
        }
    }

    fn replay_module_state(
        &self,
    ) -> Result<revenant_replay::ReplayModuleStateEvidence, revenant_replay::ReplayError> {
        let state = &self.module_state;
        if let Some(catalog) = self.arsenal_catalog() {
            let owned = state
                .owned_modules
                .iter()
                .copied()
                .filter(|id| self.build_capable || ModuleId::CONTENT.contains(id))
                .collect::<Vec<_>>();
            revenant_replay::module_state_evidence_for_catalog(
                catalog,
                state.fragments,
                &owned,
                &state.loadout,
                state.revision,
                ReplayProtocolGeneration::V2,
            )
        } else {
            module_state_evidence(
                state.fragments,
                &state.owned_modules,
                &state.loadout,
                state.revision,
                ReplayProtocolGeneration::V2,
            )
        }
    }

    fn admitted_weapon(&self) -> Option<&ModuleWeaponProfile> {
        self.admitted_module_weapons
            .iter()
            .find(|profile| profile.item_id == self.equipped_weapon_item_id)
    }
}

type TargetedMessage = (OutboundSender, ServerMessage);

fn emit_session_failure(event: &'static str, category: &'static str, correlation: Option<&str>) {
    eprintln!(
        "{}",
        serde_json::json!({
            "event": event,
            "category": category,
            "correlation_digest": correlation
        })
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionStage {
    Waiting,
    Drone,
    Stabilizer,
    Door,
    Boss,
    Complete,
    Failed,
}

/// Persistence boundary owned exclusively by the shared-session coordinator.
///
/// The production implementation remains `PostgreSQL`. Keeping this boundary
/// narrow allows deterministic failure fixtures to reject one operation without
/// changing the wire protocol or the coordinator's fail-closed ordering.
type CooperationMutationResult =
    Result<Result<CooperationReceipt, String>, Box<dyn std::error::Error>>;

trait SessionPersistence: Send {
    fn challenge_state_for(
        &mut self,
        _account_id: &str,
        _character_id: &str,
    ) -> Result<revenant_challenges::ChallengeState, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }
    fn apply_challenge_command(
        &mut self,
        _character_id: &str,
        _request: &revenant_persistence::ChallengeRequest<'_>,
        _context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }
    fn apply_challenge_combat_command(
        &mut self,
        _character_id: &str,
        _request: &revenant_persistence::ChallengeRequest<'_>,
        _context: ModuleMutationReplayContext<'_>,
        _evidence: &revenant_replay::SupportEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn apply_challenge_elite_command(
        &mut self,
        _character_id: &str,
        _request: &revenant_persistence::ChallengeRequest<'_>,
        _context: ModuleMutationReplayContext<'_>,
        _evidence: &revenant_replay::EliteEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn apply_challenge_prism_command(
        &mut self,
        _character_id: &str,
        _request: &revenant_persistence::ChallengeRequest<'_>,
        _context: ModuleMutationReplayContext<'_>,
        _boss_id: u64,
        _evidence: &revenant_replay::PrismEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn apply_challenge_recovery_step(
        &mut self,
        _character_id: &str,
        _evidence: &revenant_replay::RecoveryEvidence,
        _context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn apply_challenge_signal_step(
        &mut self,
        _character_id: &str,
        _evidence: &revenant_replay::SignalEvidence,
        _context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn apply_challenge_survival_step(
        &mut self,
        _character_id: &str,
        _evidence: &revenant_replay::SurvivalEvidence,
        _context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn apply_challenge_route_step(
        &mut self,
        _character_id: &str,
        _evidence: &revenant_replay::RouteEvidence,
        _context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn apply_challenge_gauntlet_step(
        &mut self,
        _character_id: &str,
        _evidence: &revenant_replay::GauntletEvidence,
        _context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Err(io::Error::other("challenge persistence unavailable").into())
    }

    fn campaign_state_for(
        &mut self,
        _account_id: &str,
        _character_id: &str,
    ) -> campaign::Result<revenant_campaign::CampaignState> {
        Err(io::Error::other("campaign persistence unavailable").into())
    }
    fn resume_campaign_with_replay(
        &mut self,
        _character_id: &str,
        _revision: u64,
        _context: ModuleMutationReplayContext<'_>,
    ) -> campaign::Result<revenant_campaign::CampaignState> {
        Err(io::Error::other("campaign persistence unavailable").into())
    }
    fn apply_campaign_command(
        &mut self,
        _character_id: &str,
        _request: &revenant_persistence::CampaignRequest<'_>,
        _context: ModuleMutationReplayContext<'_>,
    ) -> campaign::Result<revenant_persistence::CampaignReceipt> {
        Err(io::Error::other("campaign persistence unavailable").into())
    }
    fn refresh_acquisition_progress(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> AcquisitionStateResult;
    fn claim_acquisition_with_replay(
        &mut self,
        character_id: &str,
        arc_id: revenant_modules::acquisition::ArcId,
        context: ModuleMutationReplayContext<'_>,
    ) -> AcquisitionMutationResult;

    fn append_player_joined_with_module_snapshot(
        &mut self,
        character_id: &str,
        context: ModuleJoinReplayContext<'_>,
    ) -> Result<PersistedModuleState, Box<dyn std::error::Error>>;

    fn module_state_for(
        &mut self,
        character_id: &str,
    ) -> Result<PersistedModuleState, Box<dyn std::error::Error>>;

    fn combine_module_with_replay(
        &mut self,
        character_id: &str,
        phase: ModuleActivityPhase,
        operation_id: &str,
        module_id: ModuleId,
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<Result<OperationReceipt<CombinationOutcome>, String>, Box<dyn std::error::Error>>;

    fn set_module_loadout_with_replay(
        &mut self,
        character_id: &str,
        phase: ModuleActivityPhase,
        operation_id: &str,
        expected_revision: u64,
        modules: &[ModuleId],
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<Result<OperationReceipt<LoadoutOutcome>, String>, Box<dyn std::error::Error>>;

    fn equip_weapon_with_replay(
        &mut self,
        character_id: &str,
        item_id: &str,
        replay_event: &NewReplayEvent<'_>,
    ) -> Result<bool, Box<dyn std::error::Error>>;

    fn complete_activity_with_rewards_and_replay(
        &mut self,
        activity_completed: &NewReplayEvent<'_>,
        participants: &[ActivityParticipantCompletion<'_>],
    ) -> Result<Vec<Option<CompletionRewards>>, Box<dyn std::error::Error>>;

    fn select_route_operation_with_replay(
        &mut self,
        selection: &RouteOperationSelection<'_>,
        replay: &RouteSelectedPayloadV1,
    ) -> Result<Result<RouteReceipt<RouteSelectionOutcome>, String>, Box<dyn std::error::Error>>;

    fn complete_route_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteReceipt<DomainRouteOperationSummary>, Box<dyn std::error::Error>>;

    fn fail_route_operation_timeout_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteReceipt<DomainRouteOperationSummary>, Box<dyn std::error::Error>>;

    fn start_cooperation_operation_with_replay(
        &mut self,
        _input: &CooperationOperationInput<'_>,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn arrive_cooperation_anchor(
        &mut self,
        _session_id: &str,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn ping_cooperation_operation_with_replay(
        &mut self,
        _session_id: &str,
        _operation_id: &str,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn down_cooperation_runner_with_replay(
        &mut self,
        _session_id: &str,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn start_cooperation_revive(
        &mut self,
        _session_id: &str,
        _operation_id: &str,
        _distance_squared: i64,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn observe_cooperation_revive_with_replay(
        &mut self,
        _session_id: &str,
        _operation_id: &str,
        _distance_squared: i64,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn complete_cooperation_operation_with_replay(
        &mut self,
        _session_id: &str,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn timeout_cooperation_operation_with_replay(
        &mut self,
        _session_id: &str,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn defeat_cooperation_participant_with_replay(
        &mut self,
        _session_id: &str,
        _role: DomainCooperationRole,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn abandon_cooperation_operation_with_replay(
        &mut self,
        _session_id: &str,
        _role: DomainCooperationRole,
        _elapsed_ms: u64,
    ) -> CooperationMutationResult {
        Err(io::Error::other("cooperation persistence is not configured").into())
    }

    fn progression_for(
        &mut self,
        character_id: &str,
    ) -> Result<PersistedProgression, Box<dyn std::error::Error>>;

    fn append_replay_event(
        &mut self,
        event: &NewReplayEvent<'_>,
    ) -> Result<i64, Box<dyn std::error::Error>>;
}

impl SessionPersistence for Persistence {
    fn challenge_state_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> Result<revenant_challenges::ChallengeState, Box<dyn std::error::Error>> {
        Persistence::challenge_state_for(self, account_id, character_id)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }
    fn apply_challenge_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_command(self, character_id, request, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }
    fn apply_challenge_combat_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        evidence: &revenant_replay::SupportEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_combat_command(self, character_id, request, context, evidence)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn apply_challenge_elite_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        evidence: &revenant_replay::EliteEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_elite_command(self, character_id, request, context, evidence)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn apply_challenge_prism_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        boss_id: u64,
        evidence: &revenant_replay::PrismEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_prism_command(
            self,
            character_id,
            request,
            context,
            boss_id,
            evidence,
        )
        .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn apply_challenge_recovery_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::RecoveryEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_recovery_step(self, character_id, evidence, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn apply_challenge_signal_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::SignalEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_signal_step(self, character_id, evidence, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn apply_challenge_survival_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::SurvivalEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_survival_step(self, character_id, evidence, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn apply_challenge_route_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::RouteEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_route_step(self, character_id, evidence, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn apply_challenge_gauntlet_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::GauntletEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        Persistence::apply_challenge_gauntlet_step(self, character_id, evidence, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn campaign_state_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> campaign::Result<revenant_campaign::CampaignState> {
        Persistence::campaign_state_for(self, account_id, character_id)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }
    fn resume_campaign_with_replay(
        &mut self,
        character_id: &str,
        revision: u64,
        context: ModuleMutationReplayContext<'_>,
    ) -> campaign::Result<revenant_campaign::CampaignState> {
        Persistence::resume_campaign_with_replay(self, character_id, revision, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }
    fn apply_campaign_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::CampaignRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
    ) -> campaign::Result<revenant_persistence::CampaignReceipt> {
        Persistence::apply_campaign_command(self, character_id, request, context)
            .map_err(|e| e as Box<dyn std::error::Error>)
    }
    fn refresh_acquisition_progress(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> AcquisitionStateResult {
        Ok(Persistence::refresh_acquisition_progress(
            self,
            account_id,
            character_id,
        )?)
    }
    fn claim_acquisition_with_replay(
        &mut self,
        character_id: &str,
        arc_id: revenant_modules::acquisition::ArcId,
        context: ModuleMutationReplayContext<'_>,
    ) -> AcquisitionMutationResult {
        match Persistence::claim_acquisition_with_replay(self, character_id, arc_id, context) {
            Ok(grant) => Ok(Ok(grant)),
            Err(revenant_persistence::AcquisitionPersistenceError::Domain(error)) => {
                Ok(Err(error.to_string()))
            }
            Err(error) => Err(error.into()),
        }
    }

    fn append_player_joined_with_module_snapshot(
        &mut self,
        character_id: &str,
        context: ModuleJoinReplayContext<'_>,
    ) -> Result<PersistedModuleState, Box<dyn std::error::Error>> {
        let (_, _, state) =
            Persistence::append_player_joined_with_module_snapshot(self, character_id, context)?;
        Ok(state)
    }

    fn module_state_for(
        &mut self,
        character_id: &str,
    ) -> Result<PersistedModuleState, Box<dyn std::error::Error>> {
        Ok(Persistence::module_state_for(self, character_id)?)
    }

    fn combine_module_with_replay(
        &mut self,
        character_id: &str,
        phase: ModuleActivityPhase,
        operation_id: &str,
        module_id: ModuleId,
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<Result<OperationReceipt<CombinationOutcome>, String>, Box<dyn std::error::Error>>
    {
        match Persistence::combine_module_with_replay(
            self,
            character_id,
            phase,
            operation_id,
            module_id,
            replay,
        ) {
            Ok(receipt) => Ok(Ok(receipt)),
            Err(ModulePersistenceError::Domain(error)) => Ok(Err(error.to_string())),
            Err(error) => Err(error.into()),
        }
    }

    fn set_module_loadout_with_replay(
        &mut self,
        character_id: &str,
        phase: ModuleActivityPhase,
        operation_id: &str,
        expected_revision: u64,
        modules: &[ModuleId],
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<Result<OperationReceipt<LoadoutOutcome>, String>, Box<dyn std::error::Error>> {
        match Persistence::set_module_loadout_with_replay(
            self,
            character_id,
            phase,
            operation_id,
            expected_revision,
            modules,
            replay,
        ) {
            Ok(receipt) => Ok(Ok(receipt)),
            Err(ModulePersistenceError::Domain(error)) => Ok(Err(error.to_string())),
            Err(error) => Err(error.into()),
        }
    }

    fn equip_weapon_with_replay(
        &mut self,
        character_id: &str,
        item_id: &str,
        replay_event: &NewReplayEvent<'_>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(Persistence::equip_weapon_with_replay(
            self,
            character_id,
            item_id,
            replay_event,
        )?)
    }

    fn complete_activity_with_rewards_and_replay(
        &mut self,
        activity_completed: &NewReplayEvent<'_>,
        participants: &[ActivityParticipantCompletion<'_>],
    ) -> Result<Vec<Option<CompletionRewards>>, Box<dyn std::error::Error>> {
        Ok(Persistence::complete_activity_with_rewards_and_replay(
            self,
            activity_completed,
            participants,
        )?)
    }

    fn select_route_operation_with_replay(
        &mut self,
        selection: &RouteOperationSelection<'_>,
        replay: &RouteSelectedPayloadV1,
    ) -> Result<Result<RouteReceipt<RouteSelectionOutcome>, String>, Box<dyn std::error::Error>>
    {
        match Persistence::select_route_operation_with_replay(self, selection, replay) {
            Ok(receipt) => Ok(Ok(receipt)),
            Err(RoutePersistenceError::Domain(error)) => Ok(Err(error.to_string())),
            Err(error) => Err(error.into()),
        }
    }

    fn complete_route_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteReceipt<DomainRouteOperationSummary>, Box<dyn std::error::Error>> {
        Ok(Persistence::complete_route_operation_with_replay(
            self, session_id, elapsed_ms, replay,
        )?)
    }

    fn fail_route_operation_timeout_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteReceipt<DomainRouteOperationSummary>, Box<dyn std::error::Error>> {
        Ok(Persistence::fail_route_operation_timeout_with_replay(
            self, session_id, elapsed_ms, replay,
        )?)
    }

    fn start_cooperation_operation_with_replay(
        &mut self,
        input: &CooperationOperationInput<'_>,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::start_cooperation_operation_with_replay(
            self, input,
        ))
    }

    fn arrive_cooperation_anchor(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::arrive_cooperation_anchor(
            self, session_id, elapsed_ms,
        ))
    }

    fn ping_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        operation_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::ping_cooperation_operation_with_replay(
            self,
            session_id,
            operation_id,
            elapsed_ms,
        ))
    }

    fn down_cooperation_runner_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::down_cooperation_runner_with_replay(
            self, session_id, elapsed_ms,
        ))
    }

    fn start_cooperation_revive(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::start_cooperation_revive(
            self,
            session_id,
            operation_id,
            distance_squared,
            elapsed_ms,
        ))
    }

    fn observe_cooperation_revive_with_replay(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::observe_cooperation_revive_with_replay(
            self,
            session_id,
            operation_id,
            distance_squared,
            elapsed_ms,
        ))
    }

    fn complete_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::complete_cooperation_operation_with_replay(
            self, session_id, elapsed_ms,
        ))
    }

    fn timeout_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::timeout_cooperation_operation_with_replay(
            self, session_id, elapsed_ms,
        ))
    }

    fn defeat_cooperation_participant_with_replay(
        &mut self,
        session_id: &str,
        role: DomainCooperationRole,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::defeat_cooperation_participant_with_replay(
            self, session_id, role, elapsed_ms,
        ))
    }

    fn abandon_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        role: DomainCooperationRole,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        cooperation_persistence_result(Persistence::abandon_cooperation_operation_with_replay(
            self, session_id, role, elapsed_ms,
        ))
    }

    fn progression_for(
        &mut self,
        character_id: &str,
    ) -> Result<PersistedProgression, Box<dyn std::error::Error>> {
        Persistence::progression_for(self, character_id)?
            .ok_or_else(|| io::Error::other("character progression is missing").into())
    }

    fn append_replay_event(
        &mut self,
        event: &NewReplayEvent<'_>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(Persistence::append_replay_event(self, event)?)
    }
}

fn cooperation_persistence_result(
    result: Result<CooperationReceipt, CooperationPersistenceError>,
) -> CooperationMutationResult {
    match result {
        Ok(receipt) => Ok(Ok(receipt)),
        Err(CooperationPersistenceError::Domain(error)) => Ok(Err(error.to_string())),
        Err(error) => Err(error.into()),
    }
}

#[derive(Debug)]
struct SessionPersistenceFailure(String);

impl Display for SessionPersistenceFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SessionPersistenceFailure {}

type SessionPersistenceConnection = Box<dyn SessionPersistence>;
type SessionPersistenceConnector =
    Box<dyn FnMut() -> Result<SessionPersistenceConnection, Box<dyn std::error::Error>> + Send>;

struct RecoveringSessionPersistence {
    connection: Option<SessionPersistenceConnection>,
    connector: SessionPersistenceConnector,
}

impl RecoveringSessionPersistence {
    fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = database_url.to_owned();
        let connector: SessionPersistenceConnector = Box::new(move || {
            let persistence: SessionPersistenceConnection =
                Box::new(Persistence::connect_existing(&database_url)?);
            Ok(persistence)
        });
        Self::with_connector(connector)
    }

    fn with_connector(
        mut connector: SessionPersistenceConnector,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Some(connector()?);
        Ok(Self {
            connection,
            connector,
        })
    }

    fn execute<T>(
        &mut self,
        operation: impl FnOnce(&mut dyn SessionPersistence) -> Result<T, Box<dyn std::error::Error>>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        if self.connection.is_none() {
            self.connection = Some(
                (self.connector)().map_err(|error| SessionPersistenceFailure(error.to_string()))?,
            );
            println!("{{\"event\":\"session_persistence_reconnected\"}}");
        }
        let result = operation(
            self.connection
                .as_mut()
                .expect("session persistence connection should exist")
                .as_mut(),
        );
        if result.is_err() {
            self.connection = None;
        }
        result.map_err(|error| SessionPersistenceFailure(error.to_string()).into())
    }
}

impl SessionPersistence for RecoveringSessionPersistence {
    fn challenge_state_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> Result<revenant_challenges::ChallengeState, Box<dyn std::error::Error>> {
        self.execute(|p| p.challenge_state_for(account_id, character_id))
    }
    fn apply_challenge_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_command(character_id, request, context))
    }
    fn apply_challenge_combat_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        evidence: &revenant_replay::SupportEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_combat_command(character_id, request, context, evidence))
    }

    fn apply_challenge_elite_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        evidence: &revenant_replay::EliteEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_elite_command(character_id, request, context, evidence))
    }

    fn apply_challenge_prism_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::ChallengeRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
        boss_id: u64,
        evidence: &revenant_replay::PrismEvidence,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| {
            p.apply_challenge_prism_command(character_id, request, context, boss_id, evidence)
        })
    }

    fn apply_challenge_recovery_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::RecoveryEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_recovery_step(character_id, evidence, context))
    }

    fn apply_challenge_signal_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::SignalEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_signal_step(character_id, evidence, context))
    }

    fn apply_challenge_survival_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::SurvivalEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_survival_step(character_id, evidence, context))
    }

    fn apply_challenge_route_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::RouteEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_route_step(character_id, evidence, context))
    }

    fn apply_challenge_gauntlet_step(
        &mut self,
        character_id: &str,
        evidence: &revenant_replay::GauntletEvidence,
        context: ModuleMutationReplayContext<'_>,
    ) -> Result<revenant_persistence::ChallengeReceipt, Box<dyn std::error::Error>> {
        self.execute(|p| p.apply_challenge_gauntlet_step(character_id, evidence, context))
    }

    fn campaign_state_for(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> campaign::Result<revenant_campaign::CampaignState> {
        self.execute(|p| p.campaign_state_for(account_id, character_id))
    }
    fn resume_campaign_with_replay(
        &mut self,
        character_id: &str,
        revision: u64,
        context: ModuleMutationReplayContext<'_>,
    ) -> campaign::Result<revenant_campaign::CampaignState> {
        self.execute(|p| p.resume_campaign_with_replay(character_id, revision, context))
    }
    fn apply_campaign_command(
        &mut self,
        character_id: &str,
        request: &revenant_persistence::CampaignRequest<'_>,
        context: ModuleMutationReplayContext<'_>,
    ) -> campaign::Result<revenant_persistence::CampaignReceipt> {
        self.execute(|p| p.apply_campaign_command(character_id, request, context))
    }
    fn refresh_acquisition_progress(
        &mut self,
        account_id: &str,
        character_id: &str,
    ) -> AcquisitionStateResult {
        self.execute(|p| p.refresh_acquisition_progress(account_id, character_id))
    }
    fn claim_acquisition_with_replay(
        &mut self,
        character_id: &str,
        arc_id: revenant_modules::acquisition::ArcId,
        context: ModuleMutationReplayContext<'_>,
    ) -> AcquisitionMutationResult {
        self.execute(|p| p.claim_acquisition_with_replay(character_id, arc_id, context))
    }

    fn append_player_joined_with_module_snapshot(
        &mut self,
        character_id: &str,
        context: ModuleJoinReplayContext<'_>,
    ) -> Result<PersistedModuleState, Box<dyn std::error::Error>> {
        self.execute(|persistence| {
            persistence.append_player_joined_with_module_snapshot(character_id, context)
        })
    }

    fn module_state_for(
        &mut self,
        character_id: &str,
    ) -> Result<PersistedModuleState, Box<dyn std::error::Error>> {
        self.execute(|persistence| persistence.module_state_for(character_id))
    }

    fn combine_module_with_replay(
        &mut self,
        character_id: &str,
        phase: ModuleActivityPhase,
        operation_id: &str,
        module_id: ModuleId,
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<Result<OperationReceipt<CombinationOutcome>, String>, Box<dyn std::error::Error>>
    {
        self.execute(|persistence| {
            persistence.combine_module_with_replay(
                character_id,
                phase,
                operation_id,
                module_id,
                replay,
            )
        })
    }

    fn set_module_loadout_with_replay(
        &mut self,
        character_id: &str,
        phase: ModuleActivityPhase,
        operation_id: &str,
        expected_revision: u64,
        modules: &[ModuleId],
        replay: ModuleMutationReplayContext<'_>,
    ) -> Result<Result<OperationReceipt<LoadoutOutcome>, String>, Box<dyn std::error::Error>> {
        self.execute(|persistence| {
            persistence.set_module_loadout_with_replay(
                character_id,
                phase,
                operation_id,
                expected_revision,
                modules,
                replay,
            )
        })
    }

    fn equip_weapon_with_replay(
        &mut self,
        character_id: &str,
        item_id: &str,
        replay_event: &NewReplayEvent<'_>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        self.execute(|persistence| {
            persistence.equip_weapon_with_replay(character_id, item_id, replay_event)
        })
    }

    fn complete_activity_with_rewards_and_replay(
        &mut self,
        activity_completed: &NewReplayEvent<'_>,
        participants: &[ActivityParticipantCompletion<'_>],
    ) -> Result<Vec<Option<CompletionRewards>>, Box<dyn std::error::Error>> {
        self.execute(|persistence| {
            persistence.complete_activity_with_rewards_and_replay(activity_completed, participants)
        })
    }

    fn select_route_operation_with_replay(
        &mut self,
        selection: &RouteOperationSelection<'_>,
        replay: &RouteSelectedPayloadV1,
    ) -> Result<Result<RouteReceipt<RouteSelectionOutcome>, String>, Box<dyn std::error::Error>>
    {
        self.execute(|persistence| {
            persistence.select_route_operation_with_replay(selection, replay)
        })
    }

    fn complete_route_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteReceipt<DomainRouteOperationSummary>, Box<dyn std::error::Error>> {
        self.execute(|persistence| {
            persistence.complete_route_operation_with_replay(session_id, elapsed_ms, replay)
        })
    }

    fn fail_route_operation_timeout_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
        replay: &RouteTerminalPayloadV1,
    ) -> Result<RouteReceipt<DomainRouteOperationSummary>, Box<dyn std::error::Error>> {
        self.execute(|persistence| {
            persistence.fail_route_operation_timeout_with_replay(session_id, elapsed_ms, replay)
        })
    }

    fn start_cooperation_operation_with_replay(
        &mut self,
        input: &CooperationOperationInput<'_>,
    ) -> CooperationMutationResult {
        self.execute(|persistence| persistence.start_cooperation_operation_with_replay(input))
    }

    fn arrive_cooperation_anchor(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| persistence.arrive_cooperation_anchor(session_id, elapsed_ms))
    }

    fn ping_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        operation_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.ping_cooperation_operation_with_replay(session_id, operation_id, elapsed_ms)
        })
    }

    fn down_cooperation_runner_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.down_cooperation_runner_with_replay(session_id, elapsed_ms)
        })
    }

    fn start_cooperation_revive(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.start_cooperation_revive(
                session_id,
                operation_id,
                distance_squared,
                elapsed_ms,
            )
        })
    }

    fn observe_cooperation_revive_with_replay(
        &mut self,
        session_id: &str,
        operation_id: &str,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.observe_cooperation_revive_with_replay(
                session_id,
                operation_id,
                distance_squared,
                elapsed_ms,
            )
        })
    }

    fn complete_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.complete_cooperation_operation_with_replay(session_id, elapsed_ms)
        })
    }

    fn timeout_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.timeout_cooperation_operation_with_replay(session_id, elapsed_ms)
        })
    }

    fn defeat_cooperation_participant_with_replay(
        &mut self,
        session_id: &str,
        role: DomainCooperationRole,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.defeat_cooperation_participant_with_replay(session_id, role, elapsed_ms)
        })
    }

    fn abandon_cooperation_operation_with_replay(
        &mut self,
        session_id: &str,
        role: DomainCooperationRole,
        elapsed_ms: u64,
    ) -> CooperationMutationResult {
        self.execute(|persistence| {
            persistence.abandon_cooperation_operation_with_replay(session_id, role, elapsed_ms)
        })
    }

    fn progression_for(
        &mut self,
        character_id: &str,
    ) -> Result<PersistedProgression, Box<dyn std::error::Error>> {
        self.execute(|persistence| persistence.progression_for(character_id))
    }

    fn append_replay_event(
        &mut self,
        event: &NewReplayEvent<'_>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        self.execute(|persistence| persistence.append_replay_event(event))
    }
}

type RouteSeedSource = Box<dyn FnMut() -> io::Result<RouteSeed> + Send>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RouteEncounter {
    drone_health: u32,
    drone_damage: u32,
    drone_defeated: bool,
    warden_spawned: bool,
    warden_base_health: u32,
    warden_effective_health: u32,
    warden_remaining_health: u32,
    accepted_player_hits: u32,
    warden_counter_count: u8,
    total_hostile_damage: u32,
}

#[derive(Debug, Clone)]
struct RouteRuntime {
    operation: RouteOperationState,
    selection_replay: Option<RouteSelectedPayloadV1>,
    started_at: Option<Instant>,
    last_observed_elapsed_ms: u64,
    transitions: Vec<ReplayObjectiveTransition>,
    encounter: RouteEncounter,
}

#[derive(Debug, Clone)]
struct CooperationRuntime {
    start_operation_id: String,
    state: DomainCooperationState,
    started_at: Instant,
    last_observed_elapsed_ms: u64,
    last_nonterminal_phase: DomainCooperationPhase,
    ping: Option<CooperationPingState>,
    revive: Option<CooperationReviveState>,
    terminal_elapsed_ms: Option<u64>,
    subject_role: Option<DomainCooperationRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CooperationMovementEffect {
    None,
    AnchorArrived,
    RunnerDowned { health_before: u32 },
    Terminal,
}

struct SharedSession {
    expected_players: usize,
    participants: Vec<Participant>,
    actors: ActorRegistry,
    activity: ScriptedActivity,
    campaign: Option<revenant_campaign::CampaignState>,
    challenge: Option<challenges::Runtime>,
    combat: CombatRuntime,
    combat_started: Instant,
    enemy_ai: Option<AiController>,
    enemy_id: Option<u64>,
    stage: SessionStage,
    persistence: Box<dyn SessionPersistence>,
    session_id: String,
    activity_script: String,
    route: Option<RouteRuntime>,
    cooperation: Option<CooperationRuntime>,
    route_seed_source: RouteSeedSource,
    signal: Option<signal::SignalEncounter>,
    coolant: Option<coolant::CoolantEncounter>,
    meridian: Option<revenant_activities::meridian::Expedition>,
    lancer: Option<lancer::LancerEncounter>,
    bulwark: Option<bulwark::BulwarkEncounter>,
    support: Option<support::SupportEncounter>,
    elite: Option<elite::EliteEncounter>,
    prism: Option<prism::PrismEncounter>,
}

impl SharedSession {
    fn new(database_url: &str, activity_script: &str, expected_players: usize) -> io::Result<Self> {
        Self::new_with_route_seed_source(
            database_url,
            activity_script,
            expected_players,
            Box::new(os_route_seed),
        )
    }

    fn new_with_route_seed_source(
        database_url: &str,
        activity_script: &str,
        expected_players: usize,
        route_seed_source: RouteSeedSource,
    ) -> io::Result<Self> {
        let activity = load_activity(activity_script)?;
        Ok(Self {
            expected_players,
            participants: Vec::new(),
            actors: ActorRegistry::default(),
            activity,
            campaign: None,
            challenge: None,
            combat: CombatRuntime::default(),
            combat_started: Instant::now(),
            enemy_ai: None,
            enemy_id: None,
            stage: SessionStage::Waiting,
            persistence: Box::new(RecoveringSessionPersistence::new(database_url).map_err(
                |error| {
                    io::Error::other(format!(
                        "failed to initialize PostgreSQL persistence: {error}"
                    ))
                },
            )?),
            session_id: new_session_id()?,
            activity_script: activity_script.to_owned(),
            route: None,
            cooperation: None,
            route_seed_source,
            signal: None,
            coolant: None,
            meridian: None,
            lancer: None,
            bulwark: None,
            support: None,
            elite: None,
            prism: None,
        })
    }

    // The session thread owns its command channel through shutdown.
    #[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
    fn run(mut self, commands: Receiver<SessionCommand>) {
        loop {
            self.tick_signal();
            if self.tick_gauntlet().is_err() {
                emit_session_failure("session_command_failed", "gauntlet_persistence", None);
                self.abort_after_command_failure(None);
            }
            if self.tick_challenge_survival().is_err() {
                emit_session_failure("session_command_failed", "survival_persistence", None);
                self.abort_after_command_failure(None);
            }
            if self.tick_challenge_signal().is_err() {
                emit_session_failure("session_command_failed", "signal_persistence", None);
                self.abort_after_command_failure(None);
            }
            if self.tick_challenge_recovery().is_err() {
                emit_session_failure("session_command_failed", "recovery_persistence", None);
                self.abort_after_command_failure(None);
            }
            if self.tick_lancer().is_err() {
                emit_session_failure("session_command_failed", "lancer_persistence", None);
            }
            if self.tick_prism().is_err() {
                emit_session_failure("session_command_failed", "prism_persistence", None);
            }
            if self.tick_elite().is_err() {
                emit_session_failure("session_command_failed", "elite_persistence", None);
            }
            if self.tick_support().is_err() {
                emit_session_failure("session_command_failed", "support_persistence", None);
            }
            if self.tick_bulwark().is_err() {
                emit_session_failure("session_command_failed", "bulwark_persistence", None);
            }
            if self.tick_coolant().is_err() {
                emit_session_failure("session_command_failed", "field_activity_persistence", None);
            }
            let command = match commands.recv_timeout(Duration::from_millis(100)) {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            let correlation = self.correlation_for_command(&command);
            let admission_attempt = matches!(command, SessionCommand::Join { .. });
            let observes_route_deadline = matches!(
                &command,
                SessionCommand::Attack { .. }
                    | SessionCommand::Move { .. }
                    | SessionCommand::Equip { .. }
                    | SessionCommand::AcquisitionState { .. }
                    | SessionCommand::AcquisitionClaim { .. }
                    | SessionCommand::ModuleState { .. }
                    | SessionCommand::ModulePreview { .. }
                    | SessionCommand::ModuleCombine { .. }
                    | SessionCommand::ModuleLoadout { .. }
                    | SessionCommand::RouteState { .. }
                    | SessionCommand::RouteChoice { .. }
                    | SessionCommand::CooperationState { .. }
                    | SessionCommand::CooperationStart { .. }
                    | SessionCommand::CooperationPing { .. }
                    | SessionCommand::CooperationRevive { .. }
            );
            if observes_route_deadline {
                match self.observe_route_deadline() {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(error) => {
                        emit_session_failure(
                            "session_command_failed",
                            connection_error_category(error.as_ref()),
                            correlation.as_deref(),
                        );
                        if error.downcast_ref::<SessionPersistenceFailure>().is_some() {
                            self.abort_after_command_failure(correlation.as_deref());
                        }
                        continue;
                    }
                }
            }
            let observes_cooperation_clock = matches!(
                &command,
                SessionCommand::Attack { .. }
                    | SessionCommand::Move { .. }
                    | SessionCommand::Equip { .. }
                    | SessionCommand::AcquisitionState { .. }
                    | SessionCommand::AcquisitionClaim { .. }
                    | SessionCommand::ModuleState { .. }
                    | SessionCommand::ModulePreview { .. }
                    | SessionCommand::ModuleCombine { .. }
                    | SessionCommand::ModuleLoadout { .. }
                    | SessionCommand::RouteState { .. }
                    | SessionCommand::RouteChoice { .. }
                    | SessionCommand::CooperationState { .. }
                    | SessionCommand::CooperationStart { .. }
                    | SessionCommand::CooperationPing { .. }
                    | SessionCommand::CooperationRevive { .. }
            );
            if observes_cooperation_clock {
                match self.observe_cooperation_clock() {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(error) => {
                        emit_session_failure(
                            "session_command_failed",
                            connection_error_category(error.as_ref()),
                            correlation.as_deref(),
                        );
                        if error.downcast_ref::<SessionPersistenceFailure>().is_some() {
                            self.abort_after_command_failure(correlation.as_deref());
                        }
                        continue;
                    }
                }
            }
            let result = match command {
                SessionCommand::Join {
                    participant,
                    result,
                } => {
                    let actor_id = participant.actor.id;
                    let join_result = self.join(*participant);
                    let response = match &join_result {
                        Ok(()) => Ok(self
                            .actors
                            .get(actor_id)
                            .expect("joined actor exists")
                            .position),
                        Err(error) => Err(error.to_string()),
                    };
                    let _ = result.send(response);
                    join_result
                }
                SessionCommand::Start(player_id) => self.start_if_ready(player_id),
                SessionCommand::ChallengeAbandon { player_id, intent } => {
                    self.abandon_challenge(player_id, &intent)
                }
                SessionCommand::CampaignStory { player_id, intent } => {
                    self.interact_campaign_story(player_id, &intent)
                }
                SessionCommand::Attack {
                    player_id,
                    target_id,
                } => self.attack(player_id, target_id),
                SessionCommand::Move {
                    player_id,
                    position,
                } => self.move_player(player_id, position),
                SessionCommand::Equip { player_id, item_id } => {
                    self.equip_weapon(player_id, &item_id)
                }
                SessionCommand::AcquisitionState { player_id } => {
                    self.send_acquisition_state(player_id)
                }
                SessionCommand::AcquisitionClaim { player_id, intent } => {
                    self.claim_acquisition(player_id, &intent)
                }
                SessionCommand::ModuleState { player_id } => self.send_module_state(player_id),
                SessionCommand::ModulePreview { player_id, intent } => {
                    self.preview_modules(player_id, &intent)
                }
                SessionCommand::ModuleCombine { player_id, intent } => {
                    self.combine_module(player_id, &intent)
                }
                SessionCommand::ModuleLoadout { player_id, intent } => {
                    self.set_module_loadout(player_id, &intent)
                }
                SessionCommand::RouteState { player_id } => self.request_route_state(player_id),
                SessionCommand::RouteChoice { player_id, intent } => {
                    self.choose_route(player_id, &intent)
                }
                SessionCommand::CooperationState { player_id } => {
                    self.request_cooperation_state(player_id)
                }
                SessionCommand::CooperationStart { player_id, intent } => {
                    self.start_cooperation(player_id, &intent)
                }
                SessionCommand::CooperationPing { player_id, intent } => {
                    self.ping_cooperation(player_id, &intent)
                }
                SessionCommand::CooperationRevive { player_id, intent } => {
                    self.revive_cooperation(player_id, &intent)
                }
                SessionCommand::Disconnect(player_id) => self.disconnect(player_id),
            };
            if let Err(error) = result {
                emit_session_failure(
                    "session_command_failed",
                    connection_error_category(error.as_ref()),
                    correlation.as_deref(),
                );
                if !admission_attempt && error.downcast_ref::<SessionPersistenceFailure>().is_some()
                {
                    self.abort_after_command_failure(correlation.as_deref());
                }
            }
        }
    }

    fn correlation_for_command(&self, command: &SessionCommand) -> Option<String> {
        if let SessionCommand::Join { participant, .. } = command {
            return Some(participant.correlation.as_str().to_owned());
        }
        let player_id = match command {
            SessionCommand::Start(player_id)
            | SessionCommand::Disconnect(player_id)
            | SessionCommand::Attack { player_id, .. }
            | SessionCommand::Move { player_id, .. }
            | SessionCommand::CampaignStory { player_id, .. }
            | SessionCommand::ChallengeAbandon { player_id, .. }
            | SessionCommand::Equip { player_id, .. }
            | SessionCommand::AcquisitionState { player_id }
            | SessionCommand::AcquisitionClaim { player_id, .. }
            | SessionCommand::ModuleState { player_id }
            | SessionCommand::ModulePreview { player_id, .. }
            | SessionCommand::ModuleCombine { player_id, .. }
            | SessionCommand::ModuleLoadout { player_id, .. }
            | SessionCommand::RouteState { player_id }
            | SessionCommand::RouteChoice { player_id, .. }
            | SessionCommand::CooperationState { player_id }
            | SessionCommand::CooperationStart { player_id, .. }
            | SessionCommand::CooperationPing { player_id, .. }
            | SessionCommand::CooperationRevive { player_id, .. } => *player_id,
            SessionCommand::Join { .. } => unreachable!(),
        };
        self.participants
            .iter()
            .find(|participant| participant.actor.id == player_id)
            .map(|participant| participant.correlation.as_str().to_owned())
    }

    fn abort_after_command_failure(&mut self, correlation: Option<&str>) {
        self.participants.clear();
        if self.reset().is_err() {
            emit_session_failure("session_command_failed", "internal", correlation);
        } else {
            println!(
                "{}",
                serde_json::json!({
                    "event": "session_aborted_after_command_failure",
                    "correlation_digest": correlation
                })
            );
        }
    }

    fn join(&mut self, participant: Participant) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.join_inner(participant);
        if result.is_err() && self.participants.is_empty() {
            self.reset()?;
        }
        result
    }

    fn join_inner(
        &mut self,
        mut participant: Participant,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !session_accepts_join(self.stage, self.participants.len(), self.expected_players) {
            return Err(io::Error::other(
                "relay-hub session is already active; try again after the current players leave",
            )
            .into());
        }
        let replay_generation = match participant.protocol_generation {
            ProtocolGeneration::FrozenV1 => ReplayProtocolGeneration::V1,
            ProtocolGeneration::CurrentV2 => ReplayProtocolGeneration::V2,
        };
        let campaign_entry = self.prepare_campaign_entry(&participant)?;
        self.prepare_challenge_entry(&participant)?;
        let activity_id = self
            .challenge_activity_id()
            .unwrap_or_else(|| self.activity.id())
            .to_owned();
        let persisted = self.persistence.append_player_joined_with_module_snapshot(
            &participant.character_id,
            ModuleJoinReplayContext {
                catalog_revision: participant.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &participant.account_id,
                activity_id: &activity_id,
                actor_id: i64::try_from(participant.actor.id)?,
                player_joined_payload: "player joined",
                protocol_generation: replay_generation,
            },
        )?;
        if participant.protocol_generation == ProtocolGeneration::CurrentV2
            && persisted.loadout.iter().any(|id| {
                (!participant.content_capable && !ModuleId::LEGACY.contains(id))
                    || (!participant.build_capable && !ModuleId::CONTENT.contains(id))
            })
        {
            return Err(
                io::Error::other("saved loadout requires the current content client").into(),
            );
        }
        let admitted = resolved_arsenal_projection(
            &persisted,
            participant.protocol_generation == ProtocolGeneration::CurrentV2,
            participant.content_capable,
            participant.arsenal_catalog(),
        )?;
        participant.module_state = persisted;
        participant
            .admitted_module_weapons
            .clone_from(&admitted.weapons);
        participant.admitted_max_health = admitted.max_health;
        participant.actor.health = admitted.max_health;
        participant.actor.max_health = admitted.max_health;
        if participant.protocol_generation == ProtocolGeneration::FrozenV1 {
            PULSE_RIFLE_PROFILE
                .item_id
                .clone_into(&mut participant.equipped_weapon_item_id);
        }
        self.commit_campaign_entry(&mut participant, campaign_entry)?;
        self.commit_challenge_entry(&mut participant)?;
        self.actors.insert(participant.actor.clone());
        self.participants.push(participant);
        Ok(())
    }

    fn start_if_ready(&mut self, player_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        if !self
            .participants
            .iter()
            .any(|participant| participant.actor.id == player_id)
        {
            return Err(io::Error::other("player is not admitted to the shared session").into());
        }
        if self.stage == SessionStage::Waiting && self.participants.len() == self.expected_players {
            self.start_activity()?;
        }
        Ok(())
    }

    fn start_activity(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.challenge.is_some() {
            return self.start_challenge();
        }
        if self.campaign.is_some() {
            return self.start_campaign();
        }
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::ActivityStarted,
            &owner,
            None,
            "activity started",
        )?;
        let start_messages = activity_messages(self.activity.start()).0;
        let player_actors = self
            .participants
            .iter()
            .map(|participant| participant.actor.clone())
            .collect::<Vec<_>>();
        let pressure_target_id = player_actors
            .first()
            .ok_or_else(|| io::Error::other("session has no player"))?
            .id;
        let enemy_health = scaled_enemy_health(RELAY_DRONE_BASE_HEALTH, self.participants.len())?;
        let participant_ids = self
            .participants
            .iter()
            .map(|participant| participant.account_id.as_str())
            .collect::<Vec<_>>();
        let mut route_operation = RouteOperationState::new(&participant_ids)?;
        for participant in self
            .participants
            .iter()
            .filter(|participant| participant.route_capable)
        {
            route_operation.mark_capable(&participant.account_id)?;
        }
        let enemy = self
            .actors
            .spawn(ActorKind::Enemy, "relay-drone", [4, 0, 2], enemy_health);
        self.append_event(
            ReplayEventKind::EnemySpawned,
            &owner,
            Some(enemy.id),
            "enemy spawned: relay-drone",
        )?;
        self.enemy_id = Some(enemy.id);
        self.enemy_ai = Some(AiController::new(RELAY_DRONE_PRESSURE_PROFILE));
        self.stage = SessionStage::Drone;
        self.route = Some(RouteRuntime {
            operation: route_operation,
            selection_replay: None,
            started_at: None,
            last_observed_elapsed_ms: 0,
            transitions: Vec::new(),
            encounter: RouteEncounter {
                drone_health: enemy_health,
                drone_damage: RELAY_DRONE_PRESSURE_PROFILE.damage,
                drone_defeated: false,
                warden_spawned: false,
                warden_base_health: scaled_enemy_health(
                    WARDEN_BASE_HEALTH,
                    self.participants.len(),
                )?,
                warden_effective_health: 0,
                warden_remaining_health: 0,
                accepted_player_hits: 0,
                warden_counter_count: 0,
                total_hostile_damage: 0,
            },
        });
        for message in start_messages {
            self.broadcast(message);
        }
        for actor in &player_actors {
            self.broadcast(actor_spawn_message(actor));
        }
        self.broadcast(actor_spawn_message(&enemy));
        self.activate_enemy_ai(enemy.id, pressure_target_id)?;
        self.broadcast_route_state("route activity started")?;
        self.broadcast_cooperation_state("cooperation activity started")?;
        Ok(())
    }

    fn activate_enemy_ai(
        &mut self,
        enemy_id: u64,
        player_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..16 {
            let (events, pressure_ready) = {
                let ai = self
                    .enemy_ai
                    .as_mut()
                    .ok_or_else(|| io::Error::other("active enemy has no AI controller"))?;
                let events = ai.tick(&mut self.actors, enemy_id, player_id);
                (events, ai.pressure_ready())
            };
            for event in events {
                self.apply_ai_event(&event);
            }
            if pressure_ready {
                return Ok(());
            }
        }
        Err(io::Error::other("enemy AI did not reach pressure-ready state").into())
    }

    fn apply_ai_event(&mut self, event: &AiEvent) {
        match event {
            AiEvent::StateChanged(_) => {}
            AiEvent::Moved { actor_id, position } => {
                self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
                    actor_id: *actor_id,
                    position: *position,
                    charge: None,
                    defense: None,
                }));
            }
            AiEvent::Attacked {
                source_actor_id,
                target_actor_id,
                damage,
                remaining_health,
                killed,
            } => {
                if let Some(route) = self.route.as_mut() {
                    route.encounter.total_hostile_damage =
                        route.encounter.total_hostile_damage.saturating_add(*damage);
                    if self.stage == SessionStage::Boss {
                        route.encounter.warden_counter_count =
                            route.encounter.warden_counter_count.saturating_add(1);
                    }
                }
                println!(
                    "{{\"event\":\"enemy_attack_applied\",\"source_actor_id\":{source_actor_id},\"target_actor_id\":{target_actor_id},\"damage\":{damage},\"remaining_health\":{remaining_health},\"killed\":{killed}}}"
                );
                self.broadcast(ServerMessage::DamageApplied(DamageApplied {
                    source_actor_id: *source_actor_id,
                    target_actor_id: *target_actor_id,
                    damage: *damage,
                    remaining_health: *remaining_health,
                    killed: *killed,
                }));
            }
        }
    }

    fn counterattack_after_accepted_hit(
        &mut self,
        enemy_id: u64,
        player_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let elapsed_ms = self.cooperation.as_ref().map_or(0, |runtime| {
            u64::try_from(runtime.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
        self.counterattack_after_accepted_hit_at(enemy_id, player_id, elapsed_ms)
    }

    fn counterattack_after_accepted_hit_at(
        &mut self,
        enemy_id: u64,
        player_id: u64,
        cooperation_elapsed_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !matches!(self.stage, SessionStage::Drone | SessionStage::Boss)
            || self.enemy_id != Some(enemy_id)
        {
            return Ok(());
        }
        let target_id = self
            .participants
            .iter()
            .any(|participant| participant.actor.id == player_id)
            .then_some(player_id);
        let health_before = target_id
            .and_then(|target_id| self.actors.get(target_id))
            .map(|actor| actor.health);
        let Some(mut proposed_ai) = self.enemy_ai.clone() else {
            return Ok(());
        };
        let mut proposed_actors = self.actors.clone();
        let event = proposed_ai.accepted_hit(&mut proposed_actors, enemy_id, target_id);
        if let Some(event) = event {
            if let AiEvent::Attacked {
                source_actor_id,
                target_actor_id,
                damage,
                remaining_health,
                killed: true,
            } = &event
            {
                let cooperation_role = self.cooperation.as_ref().and_then(|runtime| {
                    (runtime.state.phase() == DomainCooperationPhase::EncounterActive)
                        .then(|| {
                            self.participants
                                .iter()
                                .position(|participant| participant.actor.id == *target_actor_id)
                                .and_then(|index| DomainCooperationRole::ALL.get(index).copied())
                        })
                        .flatten()
                });
                if let Some(role) = cooperation_role {
                    let result = self
                        .persistence
                        .defeat_cooperation_participant_with_replay(
                            &self.session_id,
                            role,
                            cooperation_elapsed_ms,
                        )?;
                    let receipt = result.map_err(io::Error::other)?;
                    self.enemy_ai = Some(proposed_ai);
                    self.actors = proposed_actors;
                    self.apply_ai_event(&event);
                    self.broadcast_cooperation_opted(&ServerMessage::CooperationLifeState(
                        WireCooperationLifeState {
                            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                            session_id: self.session_id.clone(),
                            actor_id: *target_actor_id,
                            role: wire_cooperation_role(role),
                            source_actor_id: Some(*source_actor_id),
                            cause: CooperationLifeCause::Warden,
                            elapsed_ms: cooperation_elapsed_ms,
                            health_before: health_before.ok_or_else(|| {
                                io::Error::other("defeated cooperation actor was missing")
                            })?,
                            health_after: *remaining_health,
                            max_health: self
                                .participants
                                .get(role.index())
                                .map(|participant| participant.admitted_max_health)
                                .ok_or_else(|| {
                                    io::Error::other("defeated cooperation participant was missing")
                                })?,
                            life_before: WireCooperationLife::Active,
                            life_after: WireCooperationLife::Defeated,
                        },
                    ));
                    debug_assert_eq!(*remaining_health, 0);
                    debug_assert!(*damage > 0);
                    self.finish_cooperation_terminal(receipt, cooperation_elapsed_ms, Some(role))?;
                    return Ok(());
                }
            }
            self.enemy_ai = Some(proposed_ai);
            self.actors = proposed_actors;
            self.apply_ai_event(&event);
        } else {
            self.enemy_ai = Some(proposed_ai);
            self.actors = proposed_actors;
        }
        Ok(())
    }

    fn attack(&mut self, player_id: u64, target_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let elapsed_ms =
            u64::try_from(self.combat_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.attack_at(player_id, target_id, elapsed_ms)
    }

    fn attack_at(
        &mut self,
        player_id: u64,
        target_id: u64,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.challenge.as_ref().is_some_and(|r| {
            r.state.active.is_none() || r.contract == revenant_challenges::ContractId::LastReserve
        }) {
            return Ok(());
        }
        if self
            .challenge
            .as_ref()
            .is_some_and(|r| r.contract == revenant_challenges::ContractId::DistantSignal)
        {
            return self.attack_challenge_signal(player_id, target_id, elapsed_ms);
        }
        if self.gauntlet_transferring() {
            return Ok(());
        }
        if self.prism_fighting() {
            return self.attack_prism(player_id, target_id, elapsed_ms);
        }
        if self.elite_fighting() {
            return self.attack_elite(player_id, target_id, elapsed_ms);
        }
        if self.support_fighting() {
            return self.attack_support(player_id, target_id, elapsed_ms);
        }
        if self.lancer_fighting() {
            return self.attack_lancer(player_id, target_id, elapsed_ms);
        }
        if self.bulwark_fighting() {
            return self.attack_bulwark(player_id, target_id, elapsed_ms);
        }
        if !matches!(self.stage, SessionStage::Drone | SessionStage::Boss)
            && !self.signal_fighting()
        {
            return Err(io::Error::other("combat is not active").into());
        }
        if Some(target_id) != self.enemy_id {
            return Err(io::Error::other("target is not the active encounter enemy").into());
        }
        if self.signal_fighting() && self.signal_shot_blocked(player_id, target_id) {
            return Err(io::Error::other("the signal barrier blocks this shot").into());
        }
        let participant_index = self
            .participants
            .iter()
            .position(|participant| participant.actor.id == player_id)
            .ok_or_else(|| io::Error::other("attacking participant was not found"))?;
        if self.cooperation.as_ref().is_some_and(|runtime| {
            runtime.state.participants()[participant_index].life != DomainCooperationLife::Active
        }) {
            return Err(io::Error::other("incapacitated participant cannot attack").into());
        }
        let mut profile = self.positioned_attack_profile(player_id, target_id)?;
        if self.breach_shield_active() {
            profile.damage = 0;
        }
        let result =
            self.combat
                .attack(&mut self.actors, player_id, target_id, elapsed_ms, profile)?;
        let damage_message = ServerMessage::DamageApplied(DamageApplied {
            source_actor_id: player_id,
            target_actor_id: target_id,
            damage: result.damage,
            remaining_health: result.remaining_health,
            killed: result.killed,
        });
        println!(
            "{{\"event\":\"attack_applied\",\"source_actor_id\":{player_id},\"target_actor_id\":{target_id},\"damage\":{},\"remaining_health\":{},\"killed\":{}}}",
            result.damage, result.remaining_health, result.killed
        );
        if self.stage == SessionStage::Boss {
            if let Some(route) = self.route.as_mut() {
                route.encounter.accepted_player_hits =
                    route.encounter.accepted_player_hits.saturating_add(1);
                route.encounter.warden_remaining_health = result.remaining_health;
            }
        }
        if result.killed {
            self.enemy_defeated(target_id, damage_message)?;
        } else {
            self.broadcast(damage_message);
            self.counterattack_after_accepted_hit(target_id, player_id)?;
            self.reposition_arc_warden(target_id, player_id);
        }
        Ok(())
    }

    fn reposition_arc_warden(&mut self, enemy_id: u64, player_id: u64) {
        let arc_surge = self
            .route
            .as_ref()
            .and_then(|route| route.operation.selection())
            .is_some_and(|selection| selection.event_id == RouteEventId::ArcSurge);
        if self.stage != SessionStage::Boss || !arc_surge {
            return;
        }
        if let Some(event) = self
            .enemy_ai
            .as_ref()
            .and_then(|ai| ai.reposition_arc_warden(&mut self.actors, enemy_id, player_id))
        {
            self.apply_ai_event(&event);
        }
    }

    fn equip_weapon(
        &mut self,
        player_id: u64,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let profile = weapon_profile(item_id);
        let Some(participant_index) = self
            .participants
            .iter()
            .position(|participant| participant.actor.id == player_id)
        else {
            return Err(io::Error::other("equipping participant was not found").into());
        };
        let rejection = if self.challenge.is_some() {
            Some("equipment is fixed for this challenge attempt")
        } else if matches!(self.stage, SessionStage::Complete | SessionStage::Failed) {
            Some("equipment is locked after activity completion")
        } else if self
            .cooperation
            .as_ref()
            .is_some_and(|runtime| !runtime.state.phase().is_terminal())
        {
            Some("equipment is locked during cooperation")
        } else if profile.is_none()
            || (item_id == COIL_LANCE_PROFILE.item_id
                && !self.participants[participant_index].content_capable)
            || (revenant_inventory::ARSENAL_WEAPONS.contains(&item_id)
                && !self.participants[participant_index].arsenal_capable)
        {
            Some("item is not an equipable weapon")
        } else if !self.participants[participant_index]
            .owned_items
            .iter()
            .any(|owned| owned == item_id)
        {
            Some("character does not own that weapon")
        } else {
            None
        };
        if let Some(message) = rejection {
            self.send_equipment_result(participant_index, false, message);
            return Ok(());
        }
        let character_id = self.participants[participant_index].character_id.clone();
        let account_id = self.participants[participant_index].account_id.clone();
        let replay_payload = format!("weapon equipped: {item_id}");
        let replay_kind = ReplayEventKind::EquipmentChanged.to_string();
        let actor_id = i64::try_from(player_id)?;
        if !self.persistence.equip_weapon_with_replay(
            &character_id,
            item_id,
            &NewReplayEvent {
                event_type: &replay_kind,
                session_id: &self.session_id,
                account_id: &account_id,
                activity_id: Some(self.activity.id()),
                actor_id: Some(actor_id),
                payload: &replay_payload,
            },
        )? {
            return Err(io::Error::other("persisted equipment loadout is missing").into());
        }
        self.participants[participant_index]
            .equipped_weapon_item_id
            .clone_from(&item_id.to_owned());
        let profile = self.participants[participant_index]
            .admitted_weapon()
            .cloned()
            .ok_or_else(|| io::Error::other("admitted weapon profile is missing"))?;
        let message = ServerMessage::EquipmentChanged(EquipmentChanged {
            accepted: true,
            message: "weapon equipped".to_owned(),
            actor_id: player_id,
            equipped_weapon_item_id: item_id.to_owned(),
            damage: profile.effective_damage,
            range: profile.effective_range,
            cooldown_ms: profile.effective_cooldown_ms,
        });
        self.broadcast_v2(&message);
        Ok(())
    }

    fn send_equipment_result(&self, participant_index: usize, accepted: bool, message: &str) {
        let participant = &self.participants[participant_index];
        if participant.protocol_generation != ProtocolGeneration::CurrentV2 {
            return;
        }
        let profile = participant.admitted_weapon();
        let (damage, range, cooldown_ms) = profile.map_or(
            (
                PULSE_RIFLE_PROFILE.damage,
                PULSE_RIFLE_PROFILE.range,
                PULSE_RIFLE_PROFILE.cooldown_ms,
            ),
            |profile| {
                (
                    profile.effective_damage,
                    profile.effective_range,
                    profile.effective_cooldown_ms,
                )
            },
        );
        let _ = participant
            .outbound
            .send(ServerMessage::EquipmentChanged(EquipmentChanged {
                accepted,
                message: message.to_owned(),
                actor_id: participant.actor.id,
                equipped_weapon_item_id: participant.equipped_weapon_item_id.clone(),
                damage,
                range,
                cooldown_ms,
            }));
    }

    fn send_module_state(&self, player_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.module_participant_index(player_id)?;
        let participant = &self.participants[participant_index];
        let state = arsenal_module_snapshot(
            &participant.module_state,
            participant.content_capable,
            participant.arsenal_catalog(),
        )?;
        let _ = participant
            .outbound
            .send(ServerMessage::ModuleSnapshot(state));
        Ok(())
    }

    fn preview_modules(
        &self,
        player_id: u64,
        intent: &ModulePreviewRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.module_participant_index(player_id)?;
        let participant = &self.participants[participant_index];
        let response = match parse_wire_modules(
            &intent.modules,
            participant.content_capable,
            participant.build_capable,
        ) {
            Ok(modules) => {
                let mut preview_state = participant.module_state.clone();
                preview_state.loadout.clone_from(&modules);
                let projection = resolved_arsenal_projection(
                    &preview_state,
                    true,
                    participant.content_capable,
                    participant.arsenal_catalog(),
                )?;
                ModulePreview {
                    accepted: true,
                    message: "module preview resolved".to_owned(),
                    requested_modules: modules
                        .into_iter()
                        .map(|module_id| module_id.to_string())
                        .collect(),
                    weapons: projection.weapons,
                    max_health: projection.max_health,
                }
            }
            Err(error) => ModulePreview {
                accepted: false,
                message: error.to_string(),
                requested_modules: Vec::new(),
                weapons: Vec::new(),
                max_health: 0,
            },
        };
        let _ = participant
            .outbound
            .send(ServerMessage::ModulePreview(response));
        Ok(())
    }

    fn combine_module(
        &mut self,
        player_id: u64,
        intent: &ModuleCombineIntent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.challenge.is_some() {
            return Err(io::Error::other("operation unavailable during a challenge").into());
        }
        let participant_index = self.module_participant_index(player_id)?;
        if let Err(error) = validate_wire_operation_id(&intent.operation_id) {
            return self.send_module_combination_rejection(
                participant_index,
                "",
                "",
                &error.to_string(),
            );
        }
        if intent.module_id.is_empty() || intent.module_id.len() > MAX_MODULE_ID_BYTES {
            return self.send_module_combination_rejection(
                participant_index,
                &intent.operation_id,
                "",
                "module identifier must contain 1-32 bytes",
            );
        }
        let module_id = match intent.module_id.parse::<ModuleId>() {
            Ok(module_id) => module_id,
            Err(error) => {
                return self.send_module_combination_rejection(
                    participant_index,
                    &intent.operation_id,
                    "",
                    &error.to_string(),
                );
            }
        };
        if !self.participants[participant_index].accepts_module(module_id) {
            return self.send_module_combination_rejection(
                participant_index,
                &intent.operation_id,
                module_id.as_str(),
                "module requires content negotiation",
            );
        }
        if self.stage != SessionStage::Complete {
            return self.send_module_combination_rejection(
                participant_index,
                &intent.operation_id,
                module_id.as_str(),
                "module mutation is locked until activity completion",
            );
        }
        let participant = &self.participants[participant_index];
        let character_id = participant.character_id.clone();
        let account_id = participant.account_id.clone();
        let actor_id = i64::try_from(participant.actor.id)?;
        let activity_id = self.activity.id().to_owned();
        let result = self.persistence.combine_module_with_replay(
            &character_id,
            ModuleActivityPhase::Complete,
            &intent.operation_id,
            module_id,
            ModuleMutationReplayContext {
                catalog_revision: participant.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &account_id,
                activity_id: &activity_id,
                actor_id,
            },
        )?;
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(message) => {
                return self.send_module_combination_rejection(
                    participant_index,
                    &intent.operation_id,
                    module_id.as_str(),
                    &message,
                );
            }
        };
        let persisted = self.persistence.module_state_for(&character_id)?;
        self.participants[participant_index].module_state = persisted;
        let state = arsenal_module_snapshot(
            &self.participants[participant_index].module_state,
            self.participants[participant_index].content_capable,
            self.participants[participant_index].arsenal_catalog(),
        )?;
        let replayed = receipt.disposition == OperationDisposition::Replayed;
        let message = if replayed {
            "module combination replayed"
        } else {
            "module combined"
        };
        let _ = self.participants[participant_index]
            .outbound
            .send(ServerMessage::ModuleCombined(ModuleCombined {
                accepted: true,
                replayed,
                message: message.to_owned(),
                operation_id: receipt.outcome.operation_id,
                module_id: receipt.outcome.module_id.to_string(),
                state,
            }));
        Ok(())
    }

    fn set_module_loadout(
        &mut self,
        player_id: u64,
        intent: &ModuleLoadoutIntent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.challenge.is_some() {
            return Err(io::Error::other("operation unavailable during a challenge").into());
        }
        let participant_index = self.module_participant_index(player_id)?;
        if let Err(error) = validate_wire_operation_id(&intent.operation_id) {
            return self.send_module_loadout_rejection(participant_index, "", &error.to_string());
        }
        let modules = match parse_wire_modules(
            &intent.modules,
            self.participants[participant_index].content_capable,
            self.participants[participant_index].build_capable,
        ) {
            Ok(modules) => modules,
            Err(error) => {
                return self.send_module_loadout_rejection(
                    participant_index,
                    &intent.operation_id,
                    &error.to_string(),
                );
            }
        };
        if self.stage != SessionStage::Complete {
            return self.send_module_loadout_rejection(
                participant_index,
                &intent.operation_id,
                "module mutation is locked until activity completion",
            );
        }
        let participant = &self.participants[participant_index];
        let character_id = participant.character_id.clone();
        let account_id = participant.account_id.clone();
        let actor_id = i64::try_from(participant.actor.id)?;
        let activity_id = self.activity.id().to_owned();
        let result = self.persistence.set_module_loadout_with_replay(
            &character_id,
            ModuleActivityPhase::Complete,
            &intent.operation_id,
            intent.expected_revision,
            &modules,
            ModuleMutationReplayContext {
                catalog_revision: participant.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &account_id,
                activity_id: &activity_id,
                actor_id,
            },
        )?;
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(message) => {
                return self.send_module_loadout_rejection(
                    participant_index,
                    &intent.operation_id,
                    &message,
                );
            }
        };
        let persisted = self.persistence.module_state_for(&character_id)?;
        self.participants[participant_index].module_state = persisted;
        let state = arsenal_module_snapshot(
            &self.participants[participant_index].module_state,
            self.participants[participant_index].content_capable,
            self.participants[participant_index].arsenal_catalog(),
        )?;
        let replayed = receipt.disposition == OperationDisposition::Replayed;
        let message = if replayed {
            "module loadout change replayed"
        } else {
            "module loadout changed"
        };
        let _ = self.participants[participant_index].outbound.send(
            ServerMessage::ModuleLoadoutChanged(ModuleLoadoutChanged {
                accepted: true,
                replayed,
                message: message.to_owned(),
                operation_id: receipt.outcome.operation_id,
                state,
            }),
        );
        Ok(())
    }

    fn module_participant_index(
        &self,
        player_id: u64,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let participant_index = self
            .participants
            .iter()
            .position(|participant| participant.actor.id == player_id)
            .ok_or_else(|| io::Error::other("module participant was not found"))?;
        if self.participants[participant_index].protocol_generation != ProtocolGeneration::CurrentV2
        {
            return Err(io::Error::other("frozen V1 does not expose modules").into());
        }
        Ok(participant_index)
    }

    fn send_module_combination_rejection(
        &self,
        participant_index: usize,
        operation_id: &str,
        module_id: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant = &self.participants[participant_index];
        let state = arsenal_module_snapshot(
            &participant.module_state,
            participant.content_capable,
            participant.arsenal_catalog(),
        )?;
        let _ = participant
            .outbound
            .send(ServerMessage::ModuleCombined(ModuleCombined {
                accepted: false,
                replayed: false,
                message: message.to_owned(),
                operation_id: operation_id.to_owned(),
                module_id: module_id.to_owned(),
                state,
            }));
        Ok(())
    }

    fn send_module_loadout_rejection(
        &self,
        participant_index: usize,
        operation_id: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant = &self.participants[participant_index];
        let state = arsenal_module_snapshot(
            &participant.module_state,
            participant.content_capable,
            participant.arsenal_catalog(),
        )?;
        let _ = participant
            .outbound
            .send(ServerMessage::ModuleLoadoutChanged(ModuleLoadoutChanged {
                accepted: false,
                replayed: false,
                message: message.to_owned(),
                operation_id: operation_id.to_owned(),
                state,
            }));
        Ok(())
    }

    fn cooperation_participant_index(
        &self,
        player_id: u64,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if self.challenge.is_some() {
            return Err(io::Error::other("operation unavailable during a challenge").into());
        }
        let index = self
            .participants
            .iter()
            .position(|participant| participant.actor.id == player_id)
            .ok_or_else(|| io::Error::other("cooperation participant was not found"))?;
        if self.participants[index].protocol_generation != ProtocolGeneration::CurrentV2 {
            return Err(io::Error::other("frozen V1 does not expose cooperation").into());
        }
        Ok(index)
    }

    fn request_cooperation_state(
        &mut self,
        player_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.cooperation_participant_index(player_id)?;
        let already_capable = self.participants[participant_index].cooperation_capable;
        let eligible = self.expected_players == COOPERATION_PARTICIPANT_COUNT
            && (self.cooperation.is_some()
                || self.stage == SessionStage::Waiting
                || self.stage == SessionStage::Drone
                || (self.stage == SessionStage::Door
                    && self.route.as_ref().is_some_and(|route| {
                        route.operation.phase() == revenant_operations::RoutePhase::ChoiceOpen
                    })));
        if !already_capable && !eligible {
            self.send_cooperation_state_rejection(
                participant_index,
                "cooperation capability is unavailable for this activity",
            );
            return Ok(());
        }
        if already_capable {
            let state = self.cooperation_state_projection("cooperation state refreshed")?;
            let _ = self.participants[participant_index]
                .outbound
                .send(ServerMessage::CooperationState(state));
        } else {
            self.participants[participant_index].cooperation_capable = true;
            self.broadcast_cooperation_state("cooperation capability accepted")?;
        }
        Ok(())
    }

    fn send_cooperation_state_rejection(&self, participant_index: usize, message: &str) {
        let participant = &self.participants[participant_index];
        let _ = participant
            .outbound
            .send(ServerMessage::CooperationState(WireCooperationState {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: false,
                message: bounded_cooperation_message(message),
                session_id: self.session_id.clone(),
                activity_id: self.activity.id().to_owned(),
                phase: WireCooperationPhase::Unavailable,
                participant_actor_ids: self
                    .participants
                    .iter()
                    .map(|participant| participant.actor.id)
                    .collect(),
                capable_actor_ids: self
                    .participants
                    .iter()
                    .filter(|participant| participant.cooperation_capable)
                    .map(|participant| participant.actor.id)
                    .collect(),
                all_capable: false,
                operation: None,
            }));
    }

    fn cooperation_state_projection(
        &self,
        message: &str,
    ) -> Result<WireCooperationState, Box<dyn std::error::Error>> {
        Ok(WireCooperationState {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: bounded_cooperation_message(message),
            session_id: self.session_id.clone(),
            activity_id: self.activity.id().to_owned(),
            phase: self.wire_cooperation_phase(),
            participant_actor_ids: self
                .participants
                .iter()
                .map(|participant| participant.actor.id)
                .collect(),
            capable_actor_ids: self
                .participants
                .iter()
                .filter(|participant| participant.cooperation_capable)
                .map(|participant| participant.actor.id)
                .collect(),
            all_capable: self.all_cooperation_capable(),
            operation: self
                .cooperation
                .as_ref()
                .map(|runtime| self.wire_cooperation_operation(runtime))
                .transpose()?,
        })
    }

    fn all_cooperation_capable(&self) -> bool {
        self.expected_players == COOPERATION_PARTICIPANT_COUNT
            && self.participants.len() == COOPERATION_PARTICIPANT_COUNT
            && self.participants.iter().all(|participant| {
                participant.protocol_generation == ProtocolGeneration::CurrentV2
                    && participant.cooperation_capable
            })
    }

    fn wire_cooperation_phase(&self) -> WireCooperationPhase {
        if let Some(runtime) = self.cooperation.as_ref() {
            return wire_cooperation_domain_phase(runtime.state.phase());
        }
        if self.expected_players != COOPERATION_PARTICIPANT_COUNT {
            return WireCooperationPhase::Unavailable;
        }
        match self.stage {
            SessionStage::Waiting => WireCooperationPhase::Waiting,
            SessionStage::Drone => WireCooperationPhase::Drone,
            SessionStage::Door
                if self.route.as_ref().is_some_and(|route| {
                    route.operation.phase() == revenant_operations::RoutePhase::ChoiceOpen
                }) =>
            {
                WireCooperationPhase::Eligible
            }
            SessionStage::Stabilizer
            | SessionStage::Door
            | SessionStage::Boss
            | SessionStage::Complete
            | SessionStage::Failed => WireCooperationPhase::Unavailable,
        }
    }

    fn start_cooperation(
        &mut self,
        player_id: u64,
        intent: &CooperationStartIntent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let elapsed_override = self.cooperation.as_ref().map(|runtime| {
            runtime
                .state
                .participants()
                .map(|participant| participant.admitted_health)
        });
        self.start_cooperation_with_health(player_id, intent, elapsed_override)
    }

    #[allow(clippy::too_many_lines)]
    fn start_cooperation_with_health(
        &mut self,
        player_id: u64,
        intent: &CooperationStartIntent,
        admitted_health: Option<[u32; COOPERATION_PARTICIPANT_COUNT]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.cooperation_participant_index(player_id)?;
        if validate_cooperation_operation_id(&intent.operation_id).is_err() {
            self.send_cooperation_start_rejection(
                participant_index,
                "",
                "invalid cooperation operation id",
            );
            return Ok(());
        }
        if participant_index != DomainCooperationRole::Anchor.index() {
            self.send_cooperation_start_rejection(
                participant_index,
                &intent.operation_id,
                "only the first admitted participant can start cooperation",
            );
            return Ok(());
        }
        if let Some(runtime) = self.cooperation.as_ref() {
            if runtime.start_operation_id != intent.operation_id {
                self.send_cooperation_start_rejection(
                    participant_index,
                    &intent.operation_id,
                    "cooperation operation already started",
                );
                return Ok(());
            }
        } else if self.stage != SessionStage::Door
            || !self.all_cooperation_capable()
            || !self.route.as_ref().is_some_and(|route| {
                route.operation.phase() == revenant_operations::RoutePhase::ChoiceOpen
            })
        {
            self.send_cooperation_start_rejection(
                participant_index,
                &intent.operation_id,
                "cooperation start is not eligible",
            );
            return Ok(());
        }

        let mut locked_route = self
            .route
            .as_ref()
            .map(|route| route.operation.clone())
            .ok_or_else(|| io::Error::other("route coordinator was not initialized"))?;
        if self.cooperation.is_none() {
            locked_route.lock_baseline()?;
        }
        let owned = self
            .participants
            .iter()
            .enumerate()
            .map(|(index, participant)| {
                let weapon = participant
                    .admitted_weapon()
                    .ok_or_else(|| io::Error::other("admitted cooperation weapon is missing"))?;
                let health = admitted_health.map_or_else(
                    || {
                        self.actors
                            .get(participant.actor.id)
                            .map(|actor| actor.health)
                            .ok_or_else(|| {
                                io::Error::other("admitted cooperation actor is missing")
                            })
                    },
                    |health| Ok(health[index]),
                )?;
                Ok((
                    participant.account_id.clone(),
                    participant.character_id.clone(),
                    participant.actor.id,
                    participant.equipped_weapon_item_id.clone(),
                    participant.module_state.loadout.clone(),
                    CooperationCombatProfile {
                        damage: weapon.effective_damage,
                        range: weapon.effective_range,
                        cooldown_ms: weapon.effective_cooldown_ms,
                        max_health: participant.admitted_max_health,
                    },
                    health,
                ))
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let participants = owned
            .iter()
            .map(
                |(account, character, actor, weapon, modules, profile, health)| {
                    CooperationParticipant {
                        account_id: account,
                        character_id: character,
                        actor_id: *actor,
                        weapon_item_id: weapon,
                        modules,
                        profile: *profile,
                        current_health: *health,
                    }
                },
            )
            .collect::<Vec<_>>();
        let requester = owned[0].0.clone();
        let result = self.persistence.start_cooperation_operation_with_replay(
            &CooperationOperationInput {
                session_id: &self.session_id,
                activity_id: self.activity.id(),
                requester_account_id: &requester,
                operation_id: &intent.operation_id,
                participants: &participants,
            },
        )?;
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(message) => {
                self.send_cooperation_start_rejection(
                    participant_index,
                    &intent.operation_id,
                    &message,
                );
                return Ok(());
            }
        };
        let replayed = receipt.disposition == CooperationDisposition::Replayed;
        if self.cooperation.is_none() {
            let route = self
                .route
                .as_mut()
                .ok_or_else(|| io::Error::other("route coordinator disappeared"))?;
            route.operation = locked_route;
            self.cooperation = Some(CooperationRuntime {
                start_operation_id: intent.operation_id.clone(),
                state: receipt.state,
                started_at: Instant::now(),
                last_observed_elapsed_ms: 0,
                last_nonterminal_phase: DomainCooperationPhase::AwaitingAnchor,
                ping: None,
                revive: None,
                terminal_elapsed_ms: None,
                subject_role: None,
            });
        }
        let operation = self
            .cooperation
            .as_ref()
            .map(|runtime| self.wire_cooperation_operation(runtime))
            .transpose()?;
        let result_message = ServerMessage::CooperationStartResult(CooperationStartResult {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            replayed,
            message: bounded_cooperation_message(if replayed {
                "cooperation start replayed"
            } else {
                "cooperation started"
            }),
            session_id: self.session_id.clone(),
            operation_id: intent.operation_id.clone(),
            operation,
        });
        if replayed {
            let _ = self.participants[participant_index]
                .outbound
                .send(result_message);
        } else {
            self.broadcast_cooperation_opted(&result_message);
            self.broadcast_route_state("route choice locked by cooperation")?;
            self.broadcast_cooperation_state("cooperation awaiting anchor")?;
        }
        Ok(())
    }

    fn send_cooperation_start_rejection(
        &self,
        participant_index: usize,
        operation_id: &str,
        message: &str,
    ) {
        let _ = self.participants[participant_index].outbound.send(
            ServerMessage::CooperationStartResult(CooperationStartResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: false,
                replayed: false,
                message: bounded_cooperation_message(message),
                session_id: self.session_id.clone(),
                operation_id: bounded_cooperation_operation_id(operation_id),
                operation: None,
            }),
        );
    }

    fn ping_cooperation(
        &mut self,
        player_id: u64,
        intent: &CooperationPingIntent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let elapsed_ms = self.cooperation_elapsed_ms()?;
        self.ping_cooperation_at(player_id, intent, elapsed_ms)
    }

    fn ping_cooperation_at(
        &mut self,
        player_id: u64,
        intent: &CooperationPingIntent,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.cooperation_participant_index(player_id)?;
        if validate_cooperation_operation_id(&intent.operation_id).is_err() {
            self.send_cooperation_ping_rejection(
                participant_index,
                "",
                "invalid cooperation operation id",
            );
            return Ok(());
        }
        if participant_index != DomainCooperationRole::Anchor.index() || self.cooperation.is_none()
        {
            self.send_cooperation_ping_rejection(
                participant_index,
                &intent.operation_id,
                "only the active anchor can ping",
            );
            return Ok(());
        }
        let result = self.persistence.ping_cooperation_operation_with_replay(
            &self.session_id,
            &intent.operation_id,
            elapsed_ms,
        )?;
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(message) => {
                self.send_cooperation_ping_rejection(
                    participant_index,
                    &intent.operation_id,
                    &message,
                );
                return Ok(());
            }
        };
        if receipt.state.terminal().is_some() {
            self.finish_cooperation_terminal(receipt, elapsed_ms, None)?;
            return Ok(());
        }
        let replayed = receipt.disposition == CooperationDisposition::Replayed;
        let anchor_actor_id = self.participants[DomainCooperationRole::Anchor.index()]
            .actor
            .id;
        let ping = if replayed {
            self.cooperation
                .as_ref()
                .and_then(|runtime| runtime.ping.clone())
                .ok_or_else(|| io::Error::other("replayed cooperation ping is missing"))?
        } else {
            CooperationPingState {
                operation_id: intent.operation_id.clone(),
                source_actor_id: anchor_actor_id,
                target: WireCooperationTarget::RelayConsole,
                accepted_elapsed_ms: elapsed_ms,
                expires_elapsed_ms: elapsed_ms
                    .checked_add(PING_TTL_MS)
                    .ok_or_else(|| io::Error::other("cooperation ping expiry overflowed"))?,
                active: true,
            }
        };
        if let Some(runtime) = self.cooperation.as_mut() {
            runtime.state = receipt.state;
            runtime.last_observed_elapsed_ms = elapsed_ms;
            runtime.last_nonterminal_phase = runtime.state.phase();
            runtime.ping = Some(ping.clone());
        }
        let result_message = ServerMessage::CooperationPingResult(CooperationPingResult {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            replayed,
            message: bounded_cooperation_message(if replayed {
                "cooperation ping replayed"
            } else {
                "cooperation ping accepted"
            }),
            session_id: self.session_id.clone(),
            operation_id: intent.operation_id.clone(),
            ping: Some(ping),
        });
        if replayed {
            let _ = self.participants[participant_index]
                .outbound
                .send(result_message);
        } else {
            self.broadcast_cooperation_opted(&result_message);
            self.broadcast_cooperation_state("cooperation ping is live")?;
        }
        Ok(())
    }

    fn send_cooperation_ping_rejection(
        &self,
        participant_index: usize,
        operation_id: &str,
        message: &str,
    ) {
        let _ = self.participants[participant_index].outbound.send(
            ServerMessage::CooperationPingResult(CooperationPingResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: false,
                replayed: false,
                message: bounded_cooperation_message(message),
                session_id: self.session_id.clone(),
                operation_id: bounded_cooperation_operation_id(operation_id),
                ping: None,
            }),
        );
    }

    fn revive_cooperation(
        &mut self,
        player_id: u64,
        intent: &CooperationReviveIntent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let elapsed_ms = self.cooperation_elapsed_ms()?;
        self.revive_cooperation_at(player_id, intent, elapsed_ms)
    }

    #[allow(clippy::too_many_lines)]
    fn revive_cooperation_at(
        &mut self,
        player_id: u64,
        intent: &CooperationReviveIntent,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.cooperation_participant_index(player_id)?;
        if validate_cooperation_operation_id(&intent.operation_id).is_err() {
            self.send_cooperation_revive_rejection(
                participant_index,
                "",
                "invalid cooperation operation id",
            );
            return Ok(());
        }
        if participant_index != DomainCooperationRole::Anchor.index() || self.cooperation.is_none()
        {
            self.send_cooperation_revive_rejection(
                participant_index,
                &intent.operation_id,
                "only the active anchor can revive",
            );
            return Ok(());
        }
        let distance_squared = self.cooperation_distance_squared(None)?;
        let result = self.persistence.start_cooperation_revive(
            &self.session_id,
            &intent.operation_id,
            distance_squared,
            elapsed_ms,
        )?;
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(message) => {
                self.send_cooperation_revive_rejection(
                    participant_index,
                    &intent.operation_id,
                    &message,
                );
                return Ok(());
            }
        };
        if receipt.state.terminal().is_some() {
            self.finish_cooperation_terminal(receipt, elapsed_ms, None)?;
            return Ok(());
        }
        let replayed = receipt.disposition == CooperationDisposition::Replayed;
        let existing = self
            .cooperation
            .as_ref()
            .and_then(|runtime| runtime.revive.clone());
        let revive = if replayed {
            let mut revive = existing
                .ok_or_else(|| io::Error::other("replayed cooperation revive is missing"))?;
            revive.observed_elapsed_ms = elapsed_ms;
            revive.distance_squared = distance_squared;
            revive.status = if revive.status == CooperationReviveStatus::Completed {
                CooperationReviveStatus::Replayed
            } else {
                CooperationReviveStatus::Pending
            };
            revive
        } else {
            CooperationReviveState {
                operation_id: intent.operation_id.clone(),
                source_actor_id: self.participants[DomainCooperationRole::Anchor.index()]
                    .actor
                    .id,
                target_actor_id: self.participants[DomainCooperationRole::Runner.index()]
                    .actor
                    .id,
                started_elapsed_ms: elapsed_ms,
                observed_elapsed_ms: elapsed_ms,
                required_duration_ms: REVIVE_CHANNEL_MS,
                maximum_distance_squared: MAX_REVIVE_DISTANCE_SQUARED,
                distance_squared,
                status: CooperationReviveStatus::Started,
            }
        };
        if let Some(runtime) = self.cooperation.as_mut() {
            runtime.state = receipt.state;
            runtime.last_observed_elapsed_ms = elapsed_ms;
            runtime.last_nonterminal_phase = runtime.state.phase();
            if !replayed {
                runtime.revive = Some(revive.clone());
            }
        }
        let status = revive.status;
        let result_message = ServerMessage::CooperationReviveResult(CooperationReviveResult {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            accepted: true,
            replayed,
            message: bounded_cooperation_message(if replayed {
                "cooperation revive replayed"
            } else {
                "cooperation revive started"
            }),
            session_id: self.session_id.clone(),
            operation_id: intent.operation_id.clone(),
            status,
            revive: Some(revive),
        });
        if replayed {
            let _ = self.participants[participant_index]
                .outbound
                .send(result_message);
        } else {
            self.broadcast_cooperation_opted(&result_message);
            self.broadcast_cooperation_state("cooperation revive channel started")?;
        }
        Ok(())
    }

    fn send_cooperation_revive_rejection(
        &self,
        participant_index: usize,
        operation_id: &str,
        message: &str,
    ) {
        let _ = self.participants[participant_index].outbound.send(
            ServerMessage::CooperationReviveResult(CooperationReviveResult {
                schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                accepted: false,
                replayed: false,
                message: bounded_cooperation_message(message),
                session_id: self.session_id.clone(),
                operation_id: bounded_cooperation_operation_id(operation_id),
                status: CooperationReviveStatus::Rejected,
                revive: None,
            }),
        );
    }

    fn cooperation_elapsed_ms(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let runtime = self
            .cooperation
            .as_ref()
            .ok_or_else(|| io::Error::other("cooperation operation is not active"))?;
        Ok(u64::try_from(runtime.started_at.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    fn cooperation_distance_squared(
        &self,
        prospective_anchor_position: Option<[i32; 3]>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let anchor_id = self
            .participants
            .get(DomainCooperationRole::Anchor.index())
            .map(|participant| participant.actor.id)
            .ok_or_else(|| io::Error::other("cooperation anchor is missing"))?;
        let runner_id = self
            .participants
            .get(DomainCooperationRole::Runner.index())
            .map(|participant| participant.actor.id)
            .ok_or_else(|| io::Error::other("cooperation runner is missing"))?;
        let anchor = self
            .actors
            .get(anchor_id)
            .ok_or_else(|| io::Error::other("cooperation anchor actor is missing"))?;
        let runner = self
            .actors
            .get(runner_id)
            .ok_or_else(|| io::Error::other("cooperation runner actor is missing"))?;
        checked_squared_distance(
            prospective_anchor_position.unwrap_or(anchor.position),
            runner.position,
        )
        .ok_or_else(|| io::Error::other("cooperation distance overflowed").into())
    }

    fn observe_cooperation_clock(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(runtime) = self.cooperation.as_ref() else {
            return Ok(false);
        };
        if runtime.state.phase().is_terminal() {
            return Ok(false);
        }
        let elapsed_ms =
            u64::try_from(runtime.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.observe_cooperation_clock_at(elapsed_ms, None)
    }

    fn observe_cooperation_clock_at(
        &mut self,
        elapsed_ms: u64,
        prospective_anchor_position: Option<[i32; 3]>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(runtime) = self.cooperation.as_ref() else {
            return Ok(false);
        };
        if runtime.state.phase().is_terminal() {
            return Ok(false);
        }
        if runtime.state.phase() == DomainCooperationPhase::ReviveChannel {
            let revive = runtime
                .revive
                .clone()
                .ok_or_else(|| io::Error::other("active cooperation revive is missing"))?;
            let distance_squared =
                self.cooperation_distance_squared(prospective_anchor_position)?;
            let result = self.persistence.observe_cooperation_revive_with_replay(
                &self.session_id,
                &revive.operation_id,
                distance_squared,
                elapsed_ms,
            )?;
            let receipt = result.map_err(io::Error::other)?;
            if receipt.state.terminal().is_some() {
                self.finish_cooperation_terminal(receipt, elapsed_ms, None)?;
                return Ok(true);
            }
            self.apply_cooperation_revive_observation(
                receipt,
                revive,
                distance_squared,
                elapsed_ms,
            )?;
            return Ok(false);
        }
        let mut observed = runtime.state.clone();
        match observed.observe_deadlines(elapsed_ms) {
            Ok(_) => {
                if let Some(runtime) = self.cooperation.as_mut() {
                    runtime.last_observed_elapsed_ms = elapsed_ms;
                }
                Ok(false)
            }
            Err(CooperationError::DeadlineExpired(_)) => {
                let result = self
                    .persistence
                    .timeout_cooperation_operation_with_replay(&self.session_id, elapsed_ms)?;
                let receipt = result.map_err(io::Error::other)?;
                self.finish_cooperation_terminal(receipt, elapsed_ms, None)?;
                Ok(true)
            }
            Err(CooperationError::Terminal(_)) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn apply_cooperation_revive_observation(
        &mut self,
        receipt: CooperationReceipt,
        mut revive: CooperationReviveState,
        distance_squared: i64,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        revive.observed_elapsed_ms = elapsed_ms;
        revive.distance_squared = distance_squared;
        let disposition = receipt.disposition;
        let status = match disposition {
            CooperationDisposition::Applied => CooperationReviveStatus::Completed,
            CooperationDisposition::Cancelled => CooperationReviveStatus::Cancelled,
            CooperationDisposition::Pending => CooperationReviveStatus::Pending,
            CooperationDisposition::Replayed => CooperationReviveStatus::Replayed,
        };
        revive.status = status;
        if let Some(runtime) = self.cooperation.as_mut() {
            runtime.state = receipt.state;
            runtime.last_observed_elapsed_ms = elapsed_ms;
            runtime.last_nonterminal_phase = runtime.state.phase();
            runtime.revive = Some(revive.clone());
            if let Some(ping) = runtime.ping.as_mut() {
                ping.active = false;
            }
        }
        if disposition == CooperationDisposition::Applied {
            let runner_index = DomainCooperationRole::Runner.index();
            let runner_id = self.participants[runner_index].actor.id;
            let before = self
                .actors
                .get(runner_id)
                .cloned()
                .ok_or_else(|| io::Error::other("revived runner actor is missing"))?;
            let mut after = before.clone();
            after.health = REVIVE_HEALTH.min(after.max_health);
            self.actors.insert(after.clone());
            self.broadcast_cooperation_opted(&ServerMessage::CooperationLifeState(
                WireCooperationLifeState {
                    schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                    session_id: self.session_id.clone(),
                    actor_id: runner_id,
                    role: WireCooperationRole::Runner,
                    source_actor_id: Some(
                        self.participants[DomainCooperationRole::Anchor.index()]
                            .actor
                            .id,
                    ),
                    cause: CooperationLifeCause::Revive,
                    elapsed_ms,
                    health_before: before.health,
                    health_after: after.health,
                    max_health: after.max_health,
                    life_before: WireCooperationLife::Downed,
                    life_after: WireCooperationLife::Active,
                },
            ));
        }
        if matches!(
            disposition,
            CooperationDisposition::Applied | CooperationDisposition::Cancelled
        ) {
            self.broadcast_cooperation_opted(&ServerMessage::CooperationReviveResult(
                CooperationReviveResult {
                    schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                    accepted: true,
                    replayed: false,
                    message: bounded_cooperation_message(
                        if disposition == CooperationDisposition::Applied {
                            "cooperation revive completed"
                        } else {
                            "cooperation revive cancelled"
                        },
                    ),
                    session_id: self.session_id.clone(),
                    operation_id: revive.operation_id.clone(),
                    status,
                    revive: Some(revive),
                },
            ));
            self.broadcast_cooperation_state(if disposition == CooperationDisposition::Applied {
                "cooperation encounter active"
            } else {
                "cooperation revive cancelled"
            })?;
        }
        Ok(())
    }

    fn finish_cooperation_terminal(
        &mut self,
        receipt: CooperationReceipt,
        elapsed_ms: u64,
        subject_role: Option<DomainCooperationRole>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = receipt
            .state
            .terminal()
            .ok_or_else(|| io::Error::other("cooperation terminal receipt is nonterminal"))?;
        let runtime = self
            .cooperation
            .as_mut()
            .ok_or_else(|| io::Error::other("cooperation runtime disappeared"))?;
        runtime.last_nonterminal_phase = runtime.state.phase();
        runtime.state = receipt.state;
        runtime.last_observed_elapsed_ms = elapsed_ms;
        runtime.terminal_elapsed_ms = Some(elapsed_ms);
        runtime.subject_role = subject_role;
        if let Some(ping) = runtime.ping.as_mut() {
            ping.active = false;
        }
        if outcome != DomainCooperationTerminalOutcome::Succeeded {
            self.stage = SessionStage::Failed;
        }
        let summary = self.wire_cooperation_summary()?;
        self.broadcast_cooperation_opted(&ServerMessage::CooperationOperationSummary(summary));
        Ok(())
    }

    fn wire_cooperation_operation(
        &self,
        runtime: &CooperationRuntime,
    ) -> Result<CooperationOperationState, Box<dyn std::error::Error>> {
        Ok(CooperationOperationState {
            catalog_revision: COOPERATION_CATALOG_REVISION.to_owned(),
            start_operation_id: runtime.start_operation_id.clone(),
            phase: wire_cooperation_domain_phase(runtime.state.phase()),
            participants: self.wire_cooperation_participants(&runtime.state)?,
            targets: vec![
                CooperationTargetState {
                    target: WireCooperationTarget::RelayAnchor,
                    position: [3, 0, 3],
                },
                CooperationTargetState {
                    target: WireCooperationTarget::RelayConsole,
                    position: [4, 0, 3],
                },
            ],
            timing: CooperationTiming {
                operation_duration_ms: OPERATION_DURATION_MS,
                ping_ttl_ms: PING_TTL_MS,
                revive_window_ms: REVIVE_WINDOW_MS,
                revive_channel_ms: REVIVE_CHANNEL_MS,
                revive_health: REVIVE_HEALTH,
                maximum_distance_squared: MAX_REVIVE_DISTANCE_SQUARED,
            },
            reward: cooperation_wire_reward(),
            contributions: wire_cooperation_contributions(
                runtime.state.contributions(),
                runtime.state.revive_count(),
            ),
            observed_elapsed_ms: runtime.last_observed_elapsed_ms,
            ping: runtime.ping.clone(),
            revive: runtime.revive.clone(),
            terminal_outcome: runtime.state.terminal().map(wire_cooperation_terminal),
        })
    }

    fn wire_cooperation_participants(
        &self,
        state: &DomainCooperationState,
    ) -> Result<Vec<CooperationParticipantState>, Box<dyn std::error::Error>> {
        if self.participants.len() != COOPERATION_PARTICIPANT_COUNT {
            return Err(io::Error::other("cooperation participant set is incomplete").into());
        }
        Ok(state
            .participants()
            .iter()
            .enumerate()
            .map(|(index, participant)| CooperationParticipantState {
                actor_id: self.participants[index].actor.id,
                role: wire_cooperation_role(participant.role),
                current_health: participant.current_health,
                max_health: participant.profile.max_health,
                life: wire_cooperation_life(participant.life),
            })
            .collect())
    }

    fn wire_cooperation_summary(
        &self,
    ) -> Result<WireCooperationOperationSummary, Box<dyn std::error::Error>> {
        let runtime = self
            .cooperation
            .as_ref()
            .ok_or_else(|| io::Error::other("cooperation runtime is missing"))?;
        let outcome = runtime
            .state
            .terminal()
            .ok_or_else(|| io::Error::other("cooperation summary is nonterminal"))?;
        let succeeded = outcome == DomainCooperationTerminalOutcome::Succeeded;
        Ok(WireCooperationOperationSummary {
            schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
            session_id: self.session_id.clone(),
            catalog_revision: COOPERATION_CATALOG_REVISION.to_owned(),
            start_operation_id: runtime.start_operation_id.clone(),
            last_nonterminal_phase: wire_cooperation_domain_phase(runtime.last_nonterminal_phase),
            participants: self.wire_cooperation_participants(&runtime.state)?,
            contributions: wire_cooperation_contributions(
                runtime.state.contributions(),
                runtime.state.revive_count(),
            ),
            outcome: wire_cooperation_terminal(outcome),
            subject_role: runtime.subject_role.map(wire_cooperation_role),
            terminal_elapsed_ms: runtime
                .terminal_elapsed_ms
                .ok_or_else(|| io::Error::other("cooperation terminal elapsed time is missing"))?,
            reward: succeeded.then(cooperation_wire_reward),
            grants: if succeeded {
                self.participants
                    .iter()
                    .map(|participant| CooperationGrant {
                        actor_id: participant.actor.id,
                        item_id: RELAY_CORE_FRAGMENT.to_owned(),
                        item_quantity: REWARD_FRAGMENTS_PER_PARTICIPANT,
                        experience: REWARD_EXPERIENCE_PER_PARTICIPANT,
                    })
                    .collect()
            } else {
                Vec::new()
            },
            no_reward_reason: cooperation_no_reward_reason(outcome),
        })
    }

    fn broadcast_cooperation_state(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.cooperation_state_projection(message)?;
        self.broadcast_cooperation_opted(&ServerMessage::CooperationState(state));
        Ok(())
    }

    fn broadcast_cooperation_opted(&self, message: &ServerMessage) {
        for participant in self
            .participants
            .iter()
            .filter(|participant| participant.cooperation_capable)
        {
            let _ = participant.outbound.send(message.clone());
        }
    }

    fn route_participant_index(&self, player_id: u64) -> Result<usize, Box<dyn std::error::Error>> {
        if self.challenge.is_some() {
            return Err(io::Error::other("operation unavailable during a challenge").into());
        }
        let index = self
            .participants
            .iter()
            .position(|participant| participant.actor.id == player_id)
            .ok_or_else(|| io::Error::other("route participant was not found"))?;
        if self.participants[index].protocol_generation != ProtocolGeneration::CurrentV2 {
            return Err(io::Error::other("frozen V1 does not expose routes").into());
        }
        Ok(index)
    }

    fn request_route_state(&mut self, player_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.route_participant_index(player_id)?;
        let already_capable = self.participants[participant_index].route_capable;
        let eligible = self.stage == SessionStage::Waiting
            || self.route.as_ref().is_some_and(|route| {
                matches!(
                    route.operation.phase(),
                    revenant_operations::RoutePhase::Drone
                        | revenant_operations::RoutePhase::ChoiceOpen
                )
            });
        if !already_capable && !eligible {
            self.send_route_state_rejection(
                participant_index,
                "route capability is locked for this activity",
            );
            return Ok(());
        }
        if !already_capable {
            let account_id = self.participants[participant_index].account_id.clone();
            if let Some(route) = self.route.as_mut() {
                route.operation.mark_capable(&account_id)?;
            }
            self.participants[participant_index].route_capable = true;
        }
        self.broadcast_route_state(if already_capable {
            "route state refreshed"
        } else {
            "route capability accepted"
        })
    }

    fn send_route_state_rejection(&self, participant_index: usize, message: &str) {
        let participant = &self.participants[participant_index];
        let _ = participant
            .outbound
            .send(ServerMessage::RouteState(RouteState {
                schema_version: ROUTE_WIRE_SCHEMA_VERSION,
                accepted: false,
                message: bounded_route_message(message),
                session_id: self.session_id.clone(),
                activity_id: self.activity.id().to_owned(),
                phase: self.wire_route_phase(),
                leader_actor_id: self
                    .participants
                    .first()
                    .map_or(0, |leader| leader.actor.id),
                participant_actor_ids: self
                    .participants
                    .iter()
                    .map(|participant| participant.actor.id)
                    .collect(),
                capable_actor_ids: Vec::new(),
                all_capable: false,
                routes: Vec::new(),
                selection: None,
            }));
    }

    fn broadcast_route_state(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.route_state_projection(message)?;
        for participant in self
            .participants
            .iter()
            .filter(|participant| participant.route_capable)
        {
            let _ = participant
                .outbound
                .send(ServerMessage::RouteState(state.clone()));
        }
        Ok(())
    }

    fn route_state_projection(
        &self,
        message: &str,
    ) -> Result<RouteState, Box<dyn std::error::Error>> {
        let selection = self
            .route
            .as_ref()
            .and_then(|route| route.operation.selection())
            .map(|selection| self.wire_route_selection(selection))
            .transpose()?;
        let all_capable = self.route.as_ref().map_or_else(
            || {
                self.participants.len() == self.expected_players
                    && self
                        .participants
                        .iter()
                        .all(|participant| participant.route_capable)
            },
            |route| route.operation.all_capable(),
        );
        Ok(RouteState {
            schema_version: ROUTE_WIRE_SCHEMA_VERSION,
            accepted: true,
            message: bounded_route_message(message),
            session_id: self.session_id.clone(),
            activity_id: self.activity.id().to_owned(),
            phase: self.wire_route_phase(),
            leader_actor_id: self
                .participants
                .first()
                .map_or(0, |participant| participant.actor.id),
            participant_actor_ids: self
                .participants
                .iter()
                .map(|participant| participant.actor.id)
                .collect(),
            capable_actor_ids: self
                .participants
                .iter()
                .filter(|participant| participant.route_capable)
                .map(|participant| participant.actor.id)
                .collect(),
            all_capable,
            routes: wire_route_options(),
            selection,
        })
    }

    fn wire_route_phase(&self) -> WireRoutePhase {
        let Some(route) = self.route.as_ref() else {
            return WireRoutePhase::Waiting;
        };
        match route.operation.phase() {
            revenant_operations::RoutePhase::Drone => WireRoutePhase::Drone,
            revenant_operations::RoutePhase::ChoiceOpen => WireRoutePhase::ChoiceOpen,
            revenant_operations::RoutePhase::BaselineLocked => WireRoutePhase::BaselineLocked,
            revenant_operations::RoutePhase::Routed => WireRoutePhase::Routed,
            revenant_operations::RoutePhase::Succeeded => WireRoutePhase::Succeeded,
            revenant_operations::RoutePhase::Failed => WireRoutePhase::Failed,
        }
    }

    fn wire_route_selection(
        &self,
        selection: &RouteSelectionOutcome,
    ) -> Result<RouteSelection, Box<dyn std::error::Error>> {
        let leader_actor_id = self
            .participants
            .iter()
            .find(|participant| participant.account_id == selection.leader_id)
            .map(|participant| participant.actor.id)
            .ok_or_else(|| io::Error::other("route leader actor was not found"))?;
        Ok(RouteSelection {
            leader_actor_id,
            participant_actor_ids: self
                .participants
                .iter()
                .map(|participant| participant.actor.id)
                .collect(),
            route_id: selection.route_id.to_string(),
            seed: selection.seed.value(),
            event_id: selection.event_id.to_string(),
            effect: wire_route_effect(selection.effect),
            objective_path: selection
                .objective_path
                .iter()
                .copied()
                .map(wire_route_objective_id)
                .collect(),
            duration_budget_ms: selection.duration_budget_ms,
            reward: wire_route_reward(selection.reward),
        })
    }

    #[allow(clippy::too_many_lines)]
    fn choose_route(
        &mut self,
        player_id: u64,
        intent: &RouteChoiceIntent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let participant_index = self.route_participant_index(player_id)?;
        let requester_id = self.participants[participant_index].account_id.clone();
        let route_id = match intent.route_id.parse::<RouteId>() {
            Ok(route_id) => route_id,
            Err(error) => {
                self.send_route_choice_rejection(
                    participant_index,
                    &intent.operation_id,
                    &error.to_string(),
                );
                return Ok(());
            }
        };
        let Some(current_operation) = self.route.as_ref().map(|route| route.operation.clone())
        else {
            self.send_route_choice_rejection(
                participant_index,
                &intent.operation_id,
                "route choice is unavailable before activity start",
            );
            return Ok(());
        };
        let request = RouteSelectionRequest {
            operation_id: &intent.operation_id,
            route_id,
        };
        let mut probe = current_operation.clone();
        let probe_receipt = match probe.select(
            &requester_id,
            request,
            RouteSeed::new(0).expect("zero route seed should validate"),
        ) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.send_route_choice_rejection(
                    participant_index,
                    &intent.operation_id,
                    &error.to_string(),
                );
                return Ok(());
            }
        };
        let (mut selected_operation, proposed, newly_selected) =
            if probe_receipt.disposition == RouteDisposition::Replayed {
                (probe, probe_receipt.outcome, false)
            } else {
                let seed = (self.route_seed_source)()?;
                let mut selected = current_operation;
                let receipt = selected.select(&requester_id, request, seed)?;
                (selected, receipt.outcome, true)
            };
        let proposed_replay = self.route_selection_replay(&proposed)?;
        let participant_inputs = self
            .participants
            .iter()
            .map(|participant| RouteParticipant {
                account_id: &participant.account_id,
                character_id: &participant.character_id,
                actor_id: participant.actor.id,
            })
            .collect::<Vec<_>>();
        let selection_input = RouteOperationSelection {
            session_id: &self.session_id,
            activity_id: self.activity.id(),
            requester_account_id: &requester_id,
            operation_id: &intent.operation_id,
            route_id,
            candidate_seed: proposed.seed,
            participants: &participant_inputs,
        };
        let persisted = match self
            .persistence
            .select_route_operation_with_replay(&selection_input, &proposed_replay)?
        {
            Ok(receipt) => receipt,
            Err(message) => {
                self.send_route_choice_rejection(participant_index, &intent.operation_id, &message);
                return Ok(());
            }
        };
        if persisted.outcome != proposed {
            let mut durable_operation = self
                .route
                .as_ref()
                .ok_or_else(|| io::Error::other("route operation disappeared"))?
                .operation
                .clone();
            durable_operation.select(&requester_id, request, persisted.outcome.seed)?;
            selected_operation = durable_operation;
        }
        let replay = self.route_selection_replay(&persisted.outcome)?;
        let replayed = persisted.disposition == RouteDisposition::Replayed || !newly_selected;
        let wire_selection = self.wire_route_selection(&persisted.outcome)?;
        let result = ServerMessage::RouteChoiceResult(RouteChoiceResult {
            schema_version: ROUTE_WIRE_SCHEMA_VERSION,
            accepted: true,
            replayed,
            message: bounded_route_message(if replayed {
                "route selection replayed"
            } else {
                "route selected"
            }),
            session_id: self.session_id.clone(),
            operation_id: persisted.outcome.operation_id.clone(),
            selection: Some(wire_selection),
        });
        let selection_already_installed = self
            .route
            .as_ref()
            .is_some_and(|route| route.operation.selection().is_some());
        if selection_already_installed {
            let _ = self.participants[participant_index].outbound.send(result);
        } else {
            let route = self
                .route
                .as_mut()
                .ok_or_else(|| io::Error::other("route operation disappeared"))?;
            route.operation = selected_operation;
            route.selection_replay = Some(replay);
            route.started_at = Some(Instant::now());
            route.last_observed_elapsed_ms = 0;
            route.transitions = initial_route_transitions(&persisted.outcome.objective_path)?;
            route.encounter.warden_effective_health = apply_warden_health_effect(
                route.encounter.warden_base_health,
                persisted.outcome.effect,
            )?;
            self.stage = match persisted.outcome.route_id {
                RouteId::Breach => SessionStage::Door,
                RouteId::Stabilize => SessionStage::Stabilizer,
            };
            if persisted.outcome.route_id == RouteId::Stabilize {
                self.broadcast(objective_state_message(
                    "reach_relay_door",
                    "reach_area",
                    "Pending",
                    0,
                ));
                self.broadcast(objective_state_message(
                    "reach_relay_stabilizer",
                    "reach_area",
                    "Active",
                    0,
                ));
            }
            self.broadcast_route_opted(&result);
            self.broadcast_route_state("route selection committed")?;
            self.broadcast_cooperation_state("cooperation unavailable after route selection")?;
        }
        Ok(())
    }

    fn send_route_choice_rejection(
        &self,
        participant_index: usize,
        operation_id: &str,
        message: &str,
    ) {
        let _ =
            self.participants[participant_index]
                .outbound
                .send(ServerMessage::RouteChoiceResult(RouteChoiceResult {
                    schema_version: ROUTE_WIRE_SCHEMA_VERSION,
                    accepted: false,
                    replayed: false,
                    message: bounded_route_message(message),
                    session_id: self.session_id.clone(),
                    operation_id: bounded_route_operation_id(operation_id),
                    selection: None,
                }));
    }

    fn route_selection_replay(
        &self,
        selection: &RouteSelectionOutcome,
    ) -> Result<RouteSelectedPayloadV1, Box<dyn std::error::Error>> {
        let participants = self
            .participants
            .iter()
            .map(|participant| {
                Ok(ReplayRouteParticipantEvidence {
                    participant_id: participant.account_id.clone(),
                    character_id: participant.character_id.clone(),
                    actor_id: participant.actor.id,
                    weapon_item_id: participant.equipped_weapon_item_id.clone(),
                    module_state: participant.replay_module_state()?,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let definition = route_definition(selection.route_id);
        Ok(RouteSelectedPayloadV1 {
            schema_version: ROUTE_REPLAY_SCHEMA_VERSION,
            activity_id: self.activity.id().to_owned(),
            authoring_revision: AUTHORING_REVISION.to_owned(),
            catalog_revision: ROUTE_CATALOG_REVISION.to_owned(),
            resolver_revision: RESOLVER_REVISION.to_owned(),
            operation_id: selection.operation_id.clone(),
            leader_id: selection.leader_id.clone(),
            participants,
            route_id: selection.route_id,
            seed: selection.seed,
            event_id: selection.event_id,
            effect: selection.effect,
            objectives: definition
                .objective_path
                .iter()
                .enumerate()
                .map(|(index, objective_id)| ReplayRouteObjectiveEvidence {
                    objective_id: *objective_id,
                    kind: replay_route_objective_kind(*objective_id),
                    initial_state: if index == 0 {
                        ReplayRouteObjectiveState::Active
                    } else {
                        ReplayRouteObjectiveState::Pending
                    },
                    target: 1,
                })
                .collect(),
            duration_budget_ms: selection.duration_budget_ms,
            reward: selection.reward,
        })
    }

    fn broadcast_route_opted(&self, message: &ServerMessage) {
        for participant in self
            .participants
            .iter()
            .filter(|participant| participant.route_capable)
        {
            let _ = participant.outbound.send(message.clone());
        }
    }

    fn broadcast_v2(&self, message: &ServerMessage) {
        for participant in &self.participants {
            if participant.protocol_generation == ProtocolGeneration::CurrentV2 {
                let _ = participant.outbound.send(message.clone());
            }
        }
    }

    fn observe_route_deadline(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(started_at) = self.route.as_ref().and_then(|route| route.started_at) else {
            return Ok(false);
        };
        let elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.observe_route_deadline_at(elapsed_ms)
    }

    fn observe_route_deadline_at(
        &mut self,
        elapsed_ms: u64,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(route) = self.route.as_mut() else {
            return Ok(false);
        };
        if route.operation.phase() != revenant_operations::RoutePhase::Routed {
            return Ok(false);
        }
        let budget_ms = route
            .operation
            .selection()
            .ok_or_else(|| io::Error::other("routed operation has no selection"))?
            .duration_budget_ms;
        route.last_observed_elapsed_ms = elapsed_ms;
        if elapsed_ms <= budget_ms {
            return Ok(false);
        }
        let mut failed_operation = route.operation.clone();
        let receipt = failed_operation.fail_timeout(elapsed_ms)?;
        let mut transitions = route.transitions.clone();
        let active_objective =
            active_route_objective(&receipt.outcome.objective_path, &transitions)?;
        push_route_transition(
            &mut transitions,
            active_objective,
            ReplayRouteObjectiveState::Active,
            ReplayRouteObjectiveState::Failed,
            0,
        )?;
        let selection_replay = route
            .selection_replay
            .clone()
            .ok_or_else(|| io::Error::other("routed operation has no replay selection"))?;
        let terminal = route_terminal_replay(
            &selection_replay,
            &receipt.outcome,
            transitions.clone(),
            &route.encounter,
            Vec::new(),
        )?;
        self.persistence.fail_route_operation_timeout_with_replay(
            &self.session_id,
            elapsed_ms,
            &terminal,
        )?;
        let failed_objective = objective_state_message(
            active_objective.as_str(),
            wire_objective_kind(active_objective),
            "Failed",
            0,
        );
        let wire_summary =
            self.wire_route_summary(&receipt.outcome, &transitions, &terminal.grants)?;
        let route = self
            .route
            .as_mut()
            .ok_or_else(|| io::Error::other("route operation disappeared after timeout"))?;
        route.operation = failed_operation;
        route.transitions = transitions;
        route.started_at = None;
        self.stage = SessionStage::Failed;
        self.enemy_ai = None;
        let destroyed_enemy = self.enemy_id.take();
        if let Some(enemy_id) = destroyed_enemy {
            self.actors.destroy(enemy_id);
        }
        self.broadcast(failed_objective);
        if let Some(enemy_id) = destroyed_enemy {
            self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
                actor_id: enemy_id,
            }));
        }
        self.broadcast_route_opted(&ServerMessage::RouteOperationSummary(wire_summary));
        Ok(true)
    }

    fn wire_route_summary(
        &self,
        summary: &DomainRouteOperationSummary,
        transitions: &[ReplayObjectiveTransition],
        grants: &[ReplayRouteGrantEvidence],
    ) -> Result<WireRouteOperationSummary, Box<dyn std::error::Error>> {
        let leader_actor_id = self
            .participants
            .iter()
            .find(|participant| participant.account_id == summary.leader_id)
            .map(|participant| participant.actor.id)
            .ok_or_else(|| io::Error::other("route summary leader actor was not found"))?;
        Ok(WireRouteOperationSummary {
            schema_version: ROUTE_WIRE_SCHEMA_VERSION,
            session_id: self.session_id.clone(),
            authoring_revision: AUTHORING_REVISION.to_owned(),
            catalog_revision: summary.catalog_revision.clone(),
            resolver_revision: summary.resolver_revision.clone(),
            operation_id: summary.operation_id.clone(),
            leader_actor_id,
            participant_actor_ids: self
                .participants
                .iter()
                .map(|participant| participant.actor.id)
                .collect(),
            route_id: summary.route_id.to_string(),
            seed: summary.seed.value(),
            event_id: summary.event_id.to_string(),
            effect: wire_route_effect(summary.effect),
            objective_path: summary
                .objective_path
                .iter()
                .copied()
                .map(wire_route_objective_id)
                .collect(),
            transitions: transitions.iter().map(wire_route_transition).collect(),
            duration_budget_ms: summary.duration_budget_ms,
            elapsed_ms: summary.elapsed_ms,
            outcome: match summary.outcome {
                TerminalOutcome::Succeeded => RouteTerminalOutcome::Succeeded,
                TerminalOutcome::FailedTimeout => RouteTerminalOutcome::FailedTimeout,
            },
            reward: summary.reward.map(wire_route_reward),
            grants: grants
                .iter()
                .map(|grant| RouteGrant {
                    actor_id: grant.actor_id,
                    item_id: grant.item_id.clone(),
                    item_quantity: grant.item_quantity,
                    experience: grant.experience,
                })
                .collect(),
            no_reward_reason: (summary.outcome == TerminalOutcome::FailedTimeout)
                .then(|| "deadline_exceeded".to_owned()),
        })
    }

    fn enemy_defeated(
        &mut self,
        target_id: u64,
        fatal_damage: ServerMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.campaign.is_some() {
            return self.campaign_enemy_defeated(target_id, fatal_damage);
        }
        if self.signal_fighting() {
            return self.signal_sentinel_defeated(target_id, fatal_damage);
        }
        let defeated = self
            .actors
            .get(target_id)
            .cloned()
            .ok_or_else(|| io::Error::other("defeated actor disappeared"))?;
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::EnemyDied,
            &owner,
            Some(target_id),
            &format!("enemy died: {}", defeated.archetype),
        )?;
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
        let mut reward_messages = Vec::new();
        let mut operation_summary = None;
        if self.stage == SessionStage::Drone {
            let route = self
                .route
                .as_mut()
                .ok_or_else(|| io::Error::other("route operation was not initialized"))?;
            route.operation.open_choice()?;
            route.encounter.drone_defeated = true;
            self.stage = SessionStage::Door;
            self.enemy_id = None;
        } else {
            (reward_messages, operation_summary) = self.complete_activity(&owner)?;
        }
        self.enemy_ai = None;
        self.actors.destroy(target_id);
        self.broadcast(fatal_damage);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: target_id,
        }));
        for message in state_messages {
            self.broadcast(message);
        }
        if defeated.archetype == "relay-drone" {
            self.broadcast_route_state("route choice opened")?;
            self.broadcast_cooperation_state("cooperation eligible")?;
        }
        for (outbound, message) in reward_messages {
            let _ = outbound.send(message);
        }
        for message in completion_messages {
            self.broadcast(message);
        }
        if let Some(summary) = operation_summary {
            match summary {
                ServerMessage::RouteOperationSummary(_) => self.broadcast_route_opted(&summary),
                ServerMessage::CooperationOperationSummary(_) => {
                    self.broadcast_cooperation_opted(&summary);
                }
                _ => {
                    return Err(io::Error::other("unexpected operation completion summary").into());
                }
            }
        }
        Ok(())
    }

    fn prepare_cooperation_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
        elapsed_ms: u64,
    ) -> Result<CooperationMovementEffect, Box<dyn std::error::Error>> {
        let Some(runtime) = self.cooperation.as_ref() else {
            return Ok(CooperationMovementEffect::None);
        };
        if runtime.state.phase().is_terminal() {
            return Ok(CooperationMovementEffect::Terminal);
        }
        let participant_index = self
            .participants
            .iter()
            .position(|participant| participant.actor.id == player_id)
            .ok_or_else(|| io::Error::other("moving cooperation participant was not found"))?;
        if runtime.state.phase() == DomainCooperationPhase::ReviveChannel
            && participant_index == DomainCooperationRole::Anchor.index()
            && self.observe_cooperation_clock_at(elapsed_ms, Some(position))?
        {
            return Ok(CooperationMovementEffect::Terminal);
        }
        let runtime = self
            .cooperation
            .as_ref()
            .ok_or_else(|| io::Error::other("cooperation runtime disappeared"))?;
        if runtime.state.participants()[participant_index].life != DomainCooperationLife::Active {
            return Err(io::Error::other("incapacitated participant cannot move").into());
        }
        let phase = runtime.state.phase();
        let effect = if participant_index == DomainCooperationRole::Anchor.index()
            && phase == DomainCooperationPhase::AwaitingAnchor
            && position == [3, 0, 3]
        {
            let result = self
                .persistence
                .arrive_cooperation_anchor(&self.session_id, elapsed_ms)?;
            let receipt = result.map_err(io::Error::other)?;
            if receipt.state.terminal().is_some() {
                self.finish_cooperation_terminal(receipt, elapsed_ms, None)?;
                CooperationMovementEffect::Terminal
            } else {
                if let Some(runtime) = self.cooperation.as_mut() {
                    runtime.state = receipt.state;
                    runtime.last_observed_elapsed_ms = elapsed_ms;
                    runtime.last_nonterminal_phase = runtime.state.phase();
                }
                CooperationMovementEffect::AnchorArrived
            }
        } else if participant_index == DomainCooperationRole::Runner.index()
            && phase == DomainCooperationPhase::AwaitingRunner
            && position == [4, 0, 3]
        {
            let health_before = self
                .actors
                .get(player_id)
                .map(|actor| actor.health)
                .ok_or_else(|| io::Error::other("cooperation runner actor is missing"))?;
            let result = self
                .persistence
                .down_cooperation_runner_with_replay(&self.session_id, elapsed_ms)?;
            let receipt = result.map_err(io::Error::other)?;
            if receipt.state.terminal().is_some() {
                self.finish_cooperation_terminal(receipt, elapsed_ms, None)?;
                CooperationMovementEffect::Terminal
            } else {
                if let Some(runtime) = self.cooperation.as_mut() {
                    runtime.state = receipt.state;
                    runtime.last_observed_elapsed_ms = elapsed_ms;
                    runtime.last_nonterminal_phase = runtime.state.phase();
                    if let Some(ping) = runtime.ping.as_mut() {
                        ping.active = false;
                    }
                }
                CooperationMovementEffect::RunnerDowned { health_before }
            }
        } else {
            CooperationMovementEffect::None
        };
        Ok(effect)
    }

    #[allow(clippy::too_many_lines)]
    fn move_player(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cooperation_elapsed_ms = self.cooperation.as_ref().map_or(0, |runtime| {
            u64::try_from(runtime.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
        self.move_player_at(player_id, position, cooperation_elapsed_ms)
    }

    #[allow(clippy::too_many_lines)]
    fn move_player_at(
        &mut self,
        player_id: u64,
        position: [i32; 3],
        cooperation_elapsed_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.challenge.is_some() {
            return self.move_challenge(player_id, position);
        }
        if self.campaign.is_some() {
            return self.move_campaign(player_id, position);
        }
        if matches!(self.stage, SessionStage::Complete | SessionStage::Failed) {
            return Err(io::Error::other("movement is unavailable after completion").into());
        }
        let Some(player) = self.actors.get(player_id) else {
            return Err(io::Error::other("player actor was not found").into());
        };
        let crosses_meridian = position[0] < -12 || player.position[0] < -12;
        if crosses_meridian {
            if self.meridian.is_none()
                || !self.meridian_traversal_available()
                || !revenant_activities::meridian::valid_step(player.position, position)
            {
                // Reject walls without disconnecting an ordinary held movement key.
                return Ok(());
            }
        } else if !valid_movement_target(position) {
            return Err(io::Error::other("movement target is outside relay-hub bounds").into());
        }
        if self.signal_movement_blocked(player_id, position) {
            return Ok(());
        }
        let cooperation_effect = if self.cooperation.is_some() {
            self.prepare_cooperation_movement(player_id, position, cooperation_elapsed_ms)?
        } else {
            CooperationMovementEffect::None
        };
        if cooperation_effect == CooperationMovementEffect::Terminal {
            return Ok(());
        }
        if self.handle_prism_movement(player_id, position)?
            || self.handle_elite_movement(player_id, position)?
            || self.handle_support_movement(player_id, position)?
            || self.handle_lancer_movement(player_id, position)?
            || self.handle_bulwark_movement(player_id, position)?
        {
            self.actors.update_position(player_id, position);
            self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
                actor_id: player_id,
                position,
                charge: None,
                defense: None,
            }));
            return Ok(());
        }
        // Persist discoveries before mutating position or confirming objectives.
        self.handle_meridian_movement(position)?;
        self.actors
            .update_position(player_id, position)
            .ok_or_else(|| io::Error::other("player actor was not found"))?;
        let actor_update = ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: player_id,
            position,
            charge: None,
            defense: None,
        });
        if self.handle_signal_movement(player_id, position)?
            || self.handle_coolant_movement(player_id, position)?
        {
            self.broadcast(actor_update);
            return Ok(());
        }
        println!(
            "{{\"event\":\"movement_applied\",\"actor_id\":{player_id},\"position\":[{},{},{}]}}",
            position[0], position[1], position[2]
        );
        match cooperation_effect {
            CooperationMovementEffect::AnchorArrived => {
                self.broadcast(actor_update);
                self.broadcast_cooperation_state("cooperation anchor arrived")?;
                return Ok(());
            }
            CooperationMovementEffect::RunnerDowned { health_before } => {
                let downed = self
                    .actors
                    .apply_damage(player_id, health_before)
                    .ok_or_else(|| io::Error::other("cooperation runner actor disappeared"))?;
                if downed.health != 0 {
                    return Err(
                        io::Error::other("cooperation runner did not reach zero health").into(),
                    );
                }
                self.broadcast(actor_update);
                self.broadcast_cooperation_opted(&ServerMessage::CooperationLifeState(
                    WireCooperationLifeState {
                        schema_version: COOPERATION_WIRE_SCHEMA_VERSION,
                        session_id: self.session_id.clone(),
                        actor_id: player_id,
                        role: WireCooperationRole::Runner,
                        source_actor_id: None,
                        cause: CooperationLifeCause::RelayFeedback,
                        elapsed_ms: self
                            .cooperation
                            .as_ref()
                            .map_or(0, |runtime| runtime.last_observed_elapsed_ms),
                        health_before,
                        health_after: 0,
                        max_health: downed.max_health,
                        life_before: WireCooperationLife::Active,
                        life_after: WireCooperationLife::Downed,
                    },
                ));
                self.broadcast_cooperation_state("cooperation runner downed")?;
                return Ok(());
            }
            CooperationMovementEffect::None => {}
            CooperationMovementEffect::Terminal => unreachable!(),
        }
        if self.stage == SessionStage::Stabilizer && position == [3, 0, 3] {
            let (messages, _) =
                activity_messages(self.activity.apply_trigger(&WorldTrigger::AreaReached {
                    area_id: "relay_stabilizer".to_owned(),
                }));
            let route = self
                .route
                .as_mut()
                .ok_or_else(|| io::Error::other("selected route disappeared"))?;
            push_route_transition(
                &mut route.transitions,
                DomainRouteObjectiveId::ReachRelayStabilizer,
                ReplayRouteObjectiveState::Active,
                ReplayRouteObjectiveState::Completed,
                1,
            )?;
            push_route_transition(
                &mut route.transitions,
                DomainRouteObjectiveId::ReachRelayDoor,
                ReplayRouteObjectiveState::Pending,
                ReplayRouteObjectiveState::Active,
                0,
            )?;
            self.stage = SessionStage::Door;
            self.broadcast(actor_update);
            for message in messages {
                self.broadcast(message);
            }
            self.broadcast_route_state("route stabilizer completed")?;
            return Ok(());
        }
        if self.stage != SessionStage::Door || position != [6, 0, 0] {
            self.broadcast(actor_update);
            return Ok(());
        }
        if self
            .cooperation
            .as_ref()
            .is_some_and(|runtime| runtime.state.phase() != DomainCooperationPhase::EncounterActive)
        {
            self.broadcast(actor_update);
            return Ok(());
        }
        let baseline_locked = if let Some(route) = self.route.as_mut() {
            if route.operation.phase() == revenant_operations::RoutePhase::ChoiceOpen {
                route.operation.lock_baseline()?;
                true
            } else {
                false
            }
        } else {
            false
        };
        let (messages, boss_archetype) =
            activity_messages(self.activity.apply_trigger(&WorldTrigger::AreaReached {
                area_id: "relay_door".to_owned(),
            }));
        let archetype =
            boss_archetype.ok_or_else(|| io::Error::other("activity did not request a boss"))?;
        let boss_base_health = scaled_enemy_health(WARDEN_BASE_HEALTH, self.participants.len())?;
        let selected_effect = self
            .route
            .as_ref()
            .and_then(|route| route.operation.selection())
            .map(|selection| selection.effect);
        let boss_health = selected_effect.map_or(Ok(boss_base_health), |effect| {
            apply_warden_health_effect(boss_base_health, effect)
        })?;
        let boss = self
            .actors
            .spawn(ActorKind::Enemy, &archetype, [8, 0, 0], boss_health);
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::BossSpawned,
            &owner,
            Some(boss.id),
            &format!("boss spawned: {archetype}"),
        )?;
        self.enemy_id = Some(boss.id);
        let pressure = selected_effect.map_or(WARDEN_PRESSURE_PROFILE, |effect| PressureProfile {
            damage: effect.warden_counter_damage,
            ..WARDEN_PRESSURE_PROFILE
        });
        self.enemy_ai = Some(AiController::new(pressure));
        self.stage = SessionStage::Boss;
        if let Some(route) = self.route.as_mut() {
            if route.operation.selection().is_some() {
                push_route_transition(
                    &mut route.transitions,
                    DomainRouteObjectiveId::ReachRelayDoor,
                    ReplayRouteObjectiveState::Active,
                    ReplayRouteObjectiveState::Completed,
                    1,
                )?;
                push_route_transition(
                    &mut route.transitions,
                    DomainRouteObjectiveId::DefeatWarden,
                    ReplayRouteObjectiveState::Pending,
                    ReplayRouteObjectiveState::Active,
                    0,
                )?;
                route.encounter.warden_spawned = true;
                route.encounter.warden_base_health = boss_base_health;
                route.encounter.warden_effective_health = boss_health;
                route.encounter.warden_remaining_health = boss_health;
            }
        }
        self.combat = CombatRuntime::default();
        self.combat_started = Instant::now();
        self.broadcast(actor_update);
        for message in messages {
            self.broadcast(message);
        }
        self.broadcast(actor_spawn_message(&boss));
        self.activate_enemy_ai(boss.id, player_id)?;
        if baseline_locked {
            self.broadcast_route_state("route choice locked to compatibility baseline")?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn complete_activity(
        &mut self,
        owner: &str,
    ) -> Result<(Vec<TargetedMessage>, Option<ServerMessage>), Box<dyn std::error::Error>> {
        if self
            .cooperation
            .as_ref()
            .is_some_and(|runtime| runtime.state.phase() == DomainCooperationPhase::EncounterActive)
        {
            return self.complete_cooperation_activity();
        }
        if self
            .route
            .as_ref()
            .and_then(|route| route.operation.selection())
            .is_some()
        {
            return self.complete_routed_activity();
        }
        let activity_id = self.activity.id().to_owned();
        let reward = self.activity.reward().clone();
        let experience_reward = self.activity.experience_reward();
        let quantity = i32::try_from(reward.quantity)?;
        let participants = self
            .participants
            .iter()
            .map(|participant| {
                (
                    participant.account_id.clone(),
                    participant.character_id.clone(),
                    participant.actor.id,
                    participant.protocol_generation,
                    participant.outbound.clone(),
                )
            })
            .collect::<Vec<_>>();
        let completion_inputs = participants
            .iter()
            .map(|(account_id, character_id, actor_id, _, _)| {
                Ok(ActivityParticipantCompletion {
                    completion: ActivityCompletion {
                        session_id: &self.session_id,
                        account_id,
                        character_id,
                        activity_id: &activity_id,
                        item_id: &reward.item_id,
                        item_quantity: quantity,
                        experience_reward,
                    },
                    actor_id: i64::try_from(*actor_id)?,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let completion_kind = ReplayEventKind::ActivityCompleted.to_string();
        let reward_results = self.persistence.complete_activity_with_rewards_and_replay(
            &NewReplayEvent {
                event_type: &completion_kind,
                session_id: &self.session_id,
                account_id: owner,
                activity_id: Some(&activity_id),
                actor_id: None,
                payload: "activity completed",
            },
            &completion_inputs,
        )?;
        self.stage = SessionStage::Complete;
        self.enemy_id = None;
        let mut messages = Vec::new();
        for ((_, _, actor_id, generation, outbound), rewards) in
            participants.into_iter().zip(reward_results)
        {
            let Some(rewards) = rewards else {
                continue;
            };
            if reward.item_id == RELAY_CORE_FRAGMENT {
                let resulting_fragments = u32::try_from(rewards.item_quantity)?;
                let participant = self
                    .participants
                    .iter_mut()
                    .find(|participant| participant.actor.id == actor_id)
                    .ok_or_else(|| io::Error::other("rewarded module participant disappeared"))?;
                participant.module_state.fragments = resulting_fragments;
            }
            if generation == ProtocolGeneration::CurrentV2 {
                messages.push((
                    outbound.clone(),
                    ServerMessage::LootGranted(LootGranted {
                        activity_id: activity_id.clone(),
                        item_id: reward.item_id.clone(),
                        quantity: reward.quantity,
                        resulting_quantity: u32::try_from(rewards.item_quantity)?,
                    }),
                ));
                let experience = u64::try_from(rewards.experience)?;
                messages.push((
                    outbound,
                    ServerMessage::ProgressionGranted(ProgressionGranted {
                        activity_id: activity_id.clone(),
                        experience_granted: u64::try_from(rewards.experience_granted)?,
                        experience,
                        previous_level: u32::try_from(rewards.previous_level)?,
                        level: u32::try_from(rewards.level)?,
                        experience_to_next_level: experience_to_next_level(experience),
                    }),
                ));
            }
        }
        Ok((messages, None))
    }

    fn complete_cooperation_activity(
        &mut self,
    ) -> Result<(Vec<TargetedMessage>, Option<ServerMessage>), Box<dyn std::error::Error>> {
        let elapsed_ms = self.cooperation_elapsed_ms()?;
        self.complete_cooperation_activity_at(elapsed_ms)
    }

    #[allow(clippy::too_many_lines)]
    fn complete_cooperation_activity_at(
        &mut self,
        elapsed_ms: u64,
    ) -> Result<(Vec<TargetedMessage>, Option<ServerMessage>), Box<dyn std::error::Error>> {
        let previous_phase = self
            .cooperation
            .as_ref()
            .map(|runtime| runtime.state.phase())
            .ok_or_else(|| io::Error::other("cooperation runtime disappeared"))?;
        if previous_phase != DomainCooperationPhase::EncounterActive {
            return Err(io::Error::other("cooperation Warden is not completable").into());
        }
        let result = self
            .persistence
            .complete_cooperation_operation_with_replay(&self.session_id, elapsed_ms)?;
        let receipt = result.map_err(io::Error::other)?;
        if receipt.state.terminal() != Some(DomainCooperationTerminalOutcome::Succeeded) {
            return Err(io::Error::other("durable cooperation completion did not succeed").into());
        }

        let participant_projection_inputs = self
            .participants
            .iter()
            .enumerate()
            .map(|(index, participant)| {
                (
                    index,
                    participant.character_id.clone(),
                    participant.actor.id,
                    participant.protocol_generation,
                    participant.outbound.clone(),
                )
            })
            .collect::<Vec<_>>();
        let mut projections = Vec::with_capacity(participant_projection_inputs.len());
        for (index, character_id, actor_id, generation, outbound) in participant_projection_inputs {
            let module_state = self.persistence.module_state_for(&character_id)?;
            let progression = self.persistence.progression_for(&character_id)?;
            projections.push((
                index,
                actor_id,
                generation,
                outbound,
                module_state,
                progression,
            ));
        }

        let mut messages = Vec::new();
        for (index, _actor_id, generation, outbound, module_state, progression) in projections {
            let experience = u64::try_from(progression.experience)?;
            let current = DomainProgression::from_experience(experience);
            if i64::from(current.level) != i64::from(progression.level) {
                return Err(io::Error::other(
                    "persisted cooperation progression level disagrees with experience",
                )
                .into());
            }
            let previous_experience = experience
                .checked_sub(REWARD_EXPERIENCE_PER_PARTICIPANT)
                .ok_or_else(|| {
                    io::Error::other("cooperation progression grant exceeds total XP")
                })?;
            let previous = DomainProgression::from_experience(previous_experience);
            let resulting_fragments = module_state.fragments;
            self.participants[index].module_state = module_state;
            if generation == ProtocolGeneration::CurrentV2 {
                messages.push((
                    outbound.clone(),
                    ServerMessage::LootGranted(LootGranted {
                        activity_id: self.activity.id().to_owned(),
                        item_id: RELAY_CORE_FRAGMENT.to_owned(),
                        quantity: REWARD_FRAGMENTS_PER_PARTICIPANT,
                        resulting_quantity: resulting_fragments,
                    }),
                ));
                messages.push((
                    outbound,
                    ServerMessage::ProgressionGranted(ProgressionGranted {
                        activity_id: self.activity.id().to_owned(),
                        experience_granted: REWARD_EXPERIENCE_PER_PARTICIPANT,
                        experience,
                        previous_level: previous.level,
                        level: current.level,
                        experience_to_next_level: experience_to_next_level(experience),
                    }),
                ));
            }
        }

        let runtime = self
            .cooperation
            .as_mut()
            .ok_or_else(|| io::Error::other("cooperation runtime disappeared after completion"))?;
        runtime.last_nonterminal_phase = previous_phase;
        runtime.state = receipt.state;
        runtime.last_observed_elapsed_ms = elapsed_ms;
        runtime.terminal_elapsed_ms = Some(elapsed_ms);
        runtime.subject_role = None;
        if let Some(ping) = runtime.ping.as_mut() {
            ping.active = false;
        }
        self.stage = SessionStage::Complete;
        self.enemy_id = None;
        let summary = self.wire_cooperation_summary()?;
        Ok((
            messages,
            Some(ServerMessage::CooperationOperationSummary(summary)),
        ))
    }

    fn complete_routed_activity(
        &mut self,
    ) -> Result<(Vec<TargetedMessage>, Option<ServerMessage>), Box<dyn std::error::Error>> {
        let started_at = self
            .route
            .as_ref()
            .and_then(|route| route.started_at)
            .ok_or_else(|| io::Error::other("routed operation has no monotonic start"))?;
        let elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.complete_routed_activity_at(elapsed_ms)
    }

    #[allow(clippy::too_many_lines)]
    fn complete_routed_activity_at(
        &mut self,
        elapsed_ms: u64,
    ) -> Result<(Vec<TargetedMessage>, Option<ServerMessage>), Box<dyn std::error::Error>> {
        let (completed_operation, summary, transitions, selection_replay, encounter) = {
            let route = self
                .route
                .as_ref()
                .ok_or_else(|| io::Error::other("routed operation disappeared"))?;
            let mut completed_operation = route.operation.clone();
            let receipt = completed_operation.complete(elapsed_ms)?;
            let mut transitions = route.transitions.clone();
            push_route_transition(
                &mut transitions,
                DomainRouteObjectiveId::DefeatWarden,
                ReplayRouteObjectiveState::Active,
                ReplayRouteObjectiveState::Completed,
                1,
            )?;
            let selection_replay = route
                .selection_replay
                .clone()
                .ok_or_else(|| io::Error::other("routed operation has no replay selection"))?;
            (
                completed_operation,
                receipt.outcome,
                transitions,
                selection_replay,
                route.encounter.clone(),
            )
        };
        let reward = summary
            .reward
            .ok_or_else(|| io::Error::other("successful route has no reward"))?;
        let grants = selection_replay
            .participants
            .iter()
            .map(|participant| ReplayRouteGrantEvidence {
                participant_id: participant.participant_id.clone(),
                character_id: participant.character_id.clone(),
                actor_id: participant.actor_id,
                item_id: RELAY_CORE_FRAGMENT.to_owned(),
                item_quantity: reward.fragments,
                experience: reward.experience,
            })
            .collect::<Vec<_>>();
        let terminal = route_terminal_replay(
            &selection_replay,
            &summary,
            transitions.clone(),
            &encounter,
            grants,
        )?;
        let persisted = self.persistence.complete_route_operation_with_replay(
            &self.session_id,
            elapsed_ms,
            &terminal,
        )?;
        if persisted.outcome != summary {
            return Err(io::Error::other(
                "durable route completion disagrees with authoritative runtime",
            )
            .into());
        }

        let participant_projection_inputs = self
            .participants
            .iter()
            .enumerate()
            .map(|(index, participant)| {
                (
                    index,
                    participant.character_id.clone(),
                    participant.actor.id,
                    participant.protocol_generation,
                    participant.outbound.clone(),
                )
            })
            .collect::<Vec<_>>();
        let mut projections = Vec::with_capacity(participant_projection_inputs.len());
        for (index, character_id, actor_id, generation, outbound) in participant_projection_inputs {
            let module_state = self.persistence.module_state_for(&character_id)?;
            let progression = self.persistence.progression_for(&character_id)?;
            projections.push((
                index,
                actor_id,
                generation,
                outbound,
                module_state,
                progression,
            ));
        }

        let mut messages = Vec::new();
        for (index, _actor_id, generation, outbound, module_state, progression) in projections {
            let experience = u64::try_from(progression.experience)?;
            let current = DomainProgression::from_experience(experience);
            if i64::from(current.level) != i64::from(progression.level) {
                return Err(io::Error::other(
                    "persisted route progression level disagrees with experience",
                )
                .into());
            }
            let previous_experience = experience
                .checked_sub(reward.experience)
                .ok_or_else(|| io::Error::other("route progression grant exceeds total XP"))?;
            let previous = DomainProgression::from_experience(previous_experience);
            let resulting_fragments = module_state.fragments;
            self.participants[index].module_state = module_state;
            if generation == ProtocolGeneration::CurrentV2 {
                messages.push((
                    outbound.clone(),
                    ServerMessage::LootGranted(LootGranted {
                        activity_id: self.activity.id().to_owned(),
                        item_id: RELAY_CORE_FRAGMENT.to_owned(),
                        quantity: reward.fragments,
                        resulting_quantity: resulting_fragments,
                    }),
                ));
                messages.push((
                    outbound,
                    ServerMessage::ProgressionGranted(ProgressionGranted {
                        activity_id: self.activity.id().to_owned(),
                        experience_granted: reward.experience,
                        experience,
                        previous_level: previous.level,
                        level: current.level,
                        experience_to_next_level: experience_to_next_level(experience),
                    }),
                ));
            }
        }
        let wire_summary =
            self.wire_route_summary(&persisted.outcome, &transitions, &terminal.grants)?;
        let route = self
            .route
            .as_mut()
            .ok_or_else(|| io::Error::other("route operation disappeared after completion"))?;
        route.operation = completed_operation;
        route.transitions = transitions;
        route.started_at = None;
        route.last_observed_elapsed_ms = elapsed_ms;
        self.stage = SessionStage::Complete;
        self.enemy_id = None;
        Ok((
            messages,
            Some(ServerMessage::RouteOperationSummary(wire_summary)),
        ))
    }

    fn append_event(
        &mut self,
        kind: ReplayEventKind,
        account_id: &str,
        actor_id: Option<u64>,
        payload: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self
            .challenge
            .as_ref()
            .is_some_and(|r| r.contract == revenant_challenges::ContractId::RelayGauntlet)
            && matches!(
                kind,
                ReplayEventKind::EnemySpawned
                    | ReplayEventKind::BossSpawned
                    | ReplayEventKind::EnemyDied
                    | ReplayEventKind::FieldActivity
            )
        {
            return self.record_gauntlet_combat(kind, account_id, actor_id, payload);
        }
        let activity_id = self
            .challenge_activity_id()
            .unwrap_or_else(|| self.activity.id())
            .to_owned();
        self.persistence.append_replay_event(&NewReplayEvent {
            event_type: &kind.to_string(),
            session_id: &self.session_id,
            account_id,
            activity_id: Some(&activity_id),
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
        for participant in &self.participants {
            let _ = participant.outbound.send(message.clone());
        }
    }

    fn disconnect(&mut self, player_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let elapsed_ms = self.cooperation.as_ref().map_or(0, |runtime| {
            u64::try_from(runtime.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
        self.disconnect_at(player_id, elapsed_ms)
    }

    fn disconnect_at(
        &mut self,
        player_id: u64,
        cooperation_elapsed_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.interrupt_challenge(player_id)?;
        let cooperation_role = self
            .participants
            .iter()
            .position(|participant| participant.actor.id == player_id)
            .and_then(|index| DomainCooperationRole::ALL.get(index).copied());
        let cooperation_nonterminal = self
            .cooperation
            .as_ref()
            .is_some_and(|runtime| !runtime.state.phase().is_terminal());
        if let (true, Some(role)) = (cooperation_nonterminal, cooperation_role) {
            let deadline_finished =
                self.observe_cooperation_clock_at(cooperation_elapsed_ms, None)?;
            let still_nonterminal = self
                .cooperation
                .as_ref()
                .is_some_and(|runtime| !runtime.state.phase().is_terminal());
            if !deadline_finished && still_nonterminal {
                let result = self.persistence.abandon_cooperation_operation_with_replay(
                    &self.session_id,
                    role,
                    cooperation_elapsed_ms,
                )?;
                let receipt = result.map_err(io::Error::other)?;
                self.finish_cooperation_terminal(receipt, cooperation_elapsed_ms, Some(role))?;
            }
        }
        let participant_count = self.participants.len();
        self.participants
            .retain(|participant| participant.actor.id != player_id);
        self.actors.destroy(player_id);
        let participant_removed = self.participants.len() < participant_count;
        if self.participants.is_empty()
            && (participant_removed || self.stage != SessionStage::Waiting)
        {
            self.reset()?;
        }
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        self.participants.clear();
        self.activity = load_activity(&self.activity_script)?;
        self.campaign = None;
        self.challenge = None;
        self.actors = ActorRegistry::default();
        self.combat = CombatRuntime::default();
        self.combat_started = Instant::now();
        self.enemy_ai = None;
        self.enemy_id = None;
        self.stage = SessionStage::Waiting;
        self.route = None;
        self.cooperation = None;
        self.signal = None;
        self.coolant = None;
        self.meridian = None;
        self.lancer = None;
        self.bulwark = None;
        self.support = None;
        self.elite = None;
        self.prism = None;
        self.session_id = new_session_id()?;
        println!("{{\"event\":\"session_reset\"}}");
        Ok(())
    }
}

fn load_activity(path: &str) -> io::Result<ScriptedActivity> {
    ScriptedActivity::load(path)
        .map_err(|error| io::Error::other(format!("activity load failed: {error}")))
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

fn objective_state_message(
    objective_id: &str,
    objective_type: &str,
    state: &str,
    progress: u32,
) -> ServerMessage {
    ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
        objective_id: objective_id.to_owned(),
        objective_type: objective_type.to_owned(),
        state: state.to_owned(),
        progress,
        target: 1,
    })
}

fn os_route_seed() -> io::Result<RouteSeed> {
    let mut bytes = [0_u8; std::mem::size_of::<u64>()];
    getrandom::fill(&mut bytes)
        .map_err(|error| io::Error::other(format!("route seed generation failed: {error}")))?;
    let value = u64::from_le_bytes(bytes) & i64::MAX.unsigned_abs();
    RouteSeed::new(value).map_err(|error| io::Error::other(error.to_string()))
}

fn bounded_route_message(message: &str) -> String {
    if message.len() <= MAX_ROUTE_MESSAGE_BYTES {
        return message.to_owned();
    }
    let mut end = MAX_ROUTE_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_owned()
}

fn bounded_route_operation_id(operation_id: &str) -> String {
    let valid = !operation_id.is_empty()
        && operation_id.len() <= revenant_protocol::MAX_ROUTE_OPERATION_ID_BYTES
        && operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if valid {
        operation_id.to_owned()
    } else {
        String::new()
    }
}

fn bounded_cooperation_message(message: &str) -> String {
    if message.len() <= MAX_COOPERATION_MESSAGE_BYTES {
        return message.to_owned();
    }
    let mut end = MAX_COOPERATION_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_owned()
}

fn validate_cooperation_operation_id(operation_id: &str) -> Result<(), CooperationError> {
    revenant_cooperation::validate_operation_id(operation_id)
}

fn bounded_cooperation_operation_id(operation_id: &str) -> String {
    if validate_cooperation_operation_id(operation_id).is_ok()
        && operation_id.len() <= MAX_COOPERATION_OPERATION_ID_BYTES
    {
        operation_id.to_owned()
    } else {
        String::new()
    }
}

fn checked_squared_distance(left: [i32; 3], right: [i32; 3]) -> Option<i64> {
    left.into_iter()
        .zip(right)
        .try_fold(0_i64, |sum, (left, right)| {
            let delta = i64::from(left).checked_sub(i64::from(right))?;
            sum.checked_add(delta.checked_mul(delta)?)
        })
}

const fn wire_cooperation_domain_phase(phase: DomainCooperationPhase) -> WireCooperationPhase {
    match phase {
        DomainCooperationPhase::AwaitingAnchor => WireCooperationPhase::AwaitingAnchor,
        DomainCooperationPhase::AwaitingPing => WireCooperationPhase::AwaitingPing,
        DomainCooperationPhase::AwaitingRunner => WireCooperationPhase::AwaitingRunner,
        DomainCooperationPhase::RunnerDowned => WireCooperationPhase::RunnerDowned,
        DomainCooperationPhase::ReviveChannel => WireCooperationPhase::ReviveChannel,
        DomainCooperationPhase::EncounterActive => WireCooperationPhase::EncounterActive,
        DomainCooperationPhase::Succeeded => WireCooperationPhase::Succeeded,
        DomainCooperationPhase::Failed => WireCooperationPhase::Failed,
    }
}

const fn wire_cooperation_role(role: DomainCooperationRole) -> WireCooperationRole {
    match role {
        DomainCooperationRole::Anchor => WireCooperationRole::Anchor,
        DomainCooperationRole::Runner => WireCooperationRole::Runner,
    }
}

const fn wire_cooperation_life(life: DomainCooperationLife) -> WireCooperationLife {
    match life {
        DomainCooperationLife::Active => WireCooperationLife::Active,
        DomainCooperationLife::Downed => WireCooperationLife::Downed,
        DomainCooperationLife::Defeated => WireCooperationLife::Defeated,
    }
}

const fn wire_cooperation_terminal(
    outcome: DomainCooperationTerminalOutcome,
) -> WireCooperationTerminalOutcome {
    match outcome {
        DomainCooperationTerminalOutcome::Succeeded => WireCooperationTerminalOutcome::Succeeded,
        DomainCooperationTerminalOutcome::FailedPingTimeout => {
            WireCooperationTerminalOutcome::FailedPingTimeout
        }
        DomainCooperationTerminalOutcome::FailedReviveTimeout => {
            WireCooperationTerminalOutcome::FailedReviveTimeout
        }
        DomainCooperationTerminalOutcome::FailedOperationTimeout => {
            WireCooperationTerminalOutcome::FailedOperationTimeout
        }
        DomainCooperationTerminalOutcome::FailedParticipantDefeated => {
            WireCooperationTerminalOutcome::FailedParticipantDefeated
        }
        DomainCooperationTerminalOutcome::AbandonedDisconnect => {
            WireCooperationTerminalOutcome::AbandonedDisconnect
        }
    }
}

const fn wire_cooperation_contributions(
    contributions: DomainCooperationContributions,
    revive_count: u8,
) -> CooperationContributions {
    CooperationContributions {
        anchor_arrived: contributions.anchor_arrived,
        pinged: contributions.pinged,
        runner_arrived: contributions.runner_arrived,
        revived: contributions.revived,
        warden_completed: contributions.warden_completed,
        revive_count,
    }
}

fn cooperation_wire_reward() -> CooperationReward {
    CooperationReward {
        item_id: RELAY_CORE_FRAGMENT.to_owned(),
        item_quantity: REWARD_FRAGMENTS_PER_PARTICIPANT,
        experience: REWARD_EXPERIENCE_PER_PARTICIPANT,
    }
}

const fn cooperation_no_reward_reason(
    outcome: DomainCooperationTerminalOutcome,
) -> Option<CooperationNoRewardReason> {
    match outcome {
        DomainCooperationTerminalOutcome::Succeeded => None,
        DomainCooperationTerminalOutcome::FailedPingTimeout => {
            Some(CooperationNoRewardReason::PingTimeout)
        }
        DomainCooperationTerminalOutcome::FailedReviveTimeout => {
            Some(CooperationNoRewardReason::ReviveTimeout)
        }
        DomainCooperationTerminalOutcome::FailedOperationTimeout => {
            Some(CooperationNoRewardReason::OperationTimeout)
        }
        DomainCooperationTerminalOutcome::FailedParticipantDefeated => {
            Some(CooperationNoRewardReason::ParticipantDefeated)
        }
        DomainCooperationTerminalOutcome::AbandonedDisconnect => {
            Some(CooperationNoRewardReason::ParticipantDisconnected)
        }
    }
}

fn wire_route_options() -> Vec<RouteOption> {
    RouteId::ALL
        .into_iter()
        .map(|route_id| {
            let definition = route_definition(route_id);
            RouteOption {
                route_id: route_id.to_string(),
                objective_path: definition
                    .objective_path
                    .iter()
                    .copied()
                    .map(wire_route_objective_id)
                    .collect(),
                duration_budget_ms: definition.duration_ms,
                reward: wire_route_reward(definition.reward),
                events: definition
                    .events
                    .into_iter()
                    .map(|event_id| {
                        let event = event_definition(event_id);
                        RouteEventCandidate {
                            event_id: event_id.to_string(),
                            effect: wire_route_effect(event.effect),
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}

const fn wire_route_effect(effect: revenant_operations::RouteEffect) -> WireRouteEffect {
    WireRouteEffect {
        warden_health_basis_points: effect.warden_health_basis_points,
        warden_counter_damage: effect.warden_counter_damage,
    }
}

fn wire_route_reward(reward: revenant_operations::RouteReward) -> WireRouteReward {
    WireRouteReward {
        item_id: RELAY_CORE_FRAGMENT.to_owned(),
        item_quantity: reward.fragments,
        experience: reward.experience,
    }
}

const fn wire_route_objective_id(objective_id: DomainRouteObjectiveId) -> WireRouteObjectiveId {
    match objective_id {
        DomainRouteObjectiveId::ClearDroneGroup => WireRouteObjectiveId::ClearDroneGroup,
        DomainRouteObjectiveId::ReachRelayStabilizer => WireRouteObjectiveId::ReachRelayStabilizer,
        DomainRouteObjectiveId::ReachRelayDoor => WireRouteObjectiveId::ReachRelayDoor,
        DomainRouteObjectiveId::DefeatWarden => WireRouteObjectiveId::DefeatWarden,
    }
}

const fn wire_route_objective_state(state: ReplayRouteObjectiveState) -> WireRouteObjectiveState {
    match state {
        ReplayRouteObjectiveState::Pending => WireRouteObjectiveState::Pending,
        ReplayRouteObjectiveState::Active => WireRouteObjectiveState::Active,
        ReplayRouteObjectiveState::Completed => WireRouteObjectiveState::Completed,
        ReplayRouteObjectiveState::Failed => WireRouteObjectiveState::Failed,
    }
}

fn wire_route_transition(transition: &ReplayObjectiveTransition) -> WireRouteObjectiveTransition {
    WireRouteObjectiveTransition {
        ordinal: transition.ordinal,
        objective_id: wire_route_objective_id(transition.objective_id),
        from: wire_route_objective_state(transition.from),
        to: wire_route_objective_state(transition.to),
        progress: transition.progress,
        target: transition.target,
    }
}

const fn replay_route_objective_kind(
    objective_id: DomainRouteObjectiveId,
) -> ReplayRouteObjectiveKind {
    match objective_id {
        DomainRouteObjectiveId::ClearDroneGroup => ReplayRouteObjectiveKind::KillActors,
        DomainRouteObjectiveId::ReachRelayStabilizer | DomainRouteObjectiveId::ReachRelayDoor => {
            ReplayRouteObjectiveKind::ReachArea
        }
        DomainRouteObjectiveId::DefeatWarden => ReplayRouteObjectiveKind::Boss,
    }
}

const fn wire_objective_kind(objective_id: DomainRouteObjectiveId) -> &'static str {
    match objective_id {
        DomainRouteObjectiveId::ClearDroneGroup => "KillActors",
        DomainRouteObjectiveId::ReachRelayStabilizer | DomainRouteObjectiveId::ReachRelayDoor => {
            "ReachArea"
        }
        DomainRouteObjectiveId::DefeatWarden => "Boss",
    }
}

fn initial_route_transitions(
    objective_path: &[DomainRouteObjectiveId],
) -> Result<Vec<ReplayObjectiveTransition>, Box<dyn std::error::Error>> {
    if objective_path.first() != Some(&DomainRouteObjectiveId::ClearDroneGroup) {
        return Err(io::Error::other("route path does not begin with the drone objective").into());
    }
    let next = objective_path
        .get(1)
        .copied()
        .ok_or_else(|| io::Error::other("route path has no post-drone objective"))?;
    let mut transitions = Vec::with_capacity(8);
    push_route_transition(
        &mut transitions,
        DomainRouteObjectiveId::ClearDroneGroup,
        ReplayRouteObjectiveState::Active,
        ReplayRouteObjectiveState::Completed,
        1,
    )?;
    push_route_transition(
        &mut transitions,
        next,
        ReplayRouteObjectiveState::Pending,
        ReplayRouteObjectiveState::Active,
        0,
    )?;
    Ok(transitions)
}

fn push_route_transition(
    transitions: &mut Vec<ReplayObjectiveTransition>,
    objective_id: DomainRouteObjectiveId,
    from: ReplayRouteObjectiveState,
    to: ReplayRouteObjectiveState,
    progress: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    if transitions.len() >= revenant_protocol::MAX_ROUTE_TRANSITIONS {
        return Err(io::Error::other("route transition limit was exceeded").into());
    }
    transitions.push(ReplayObjectiveTransition {
        ordinal: u8::try_from(transitions.len())?,
        objective_id,
        from,
        to,
        progress,
        target: 1,
    });
    Ok(())
}

fn active_route_objective(
    objective_path: &[DomainRouteObjectiveId],
    transitions: &[ReplayObjectiveTransition],
) -> Result<DomainRouteObjectiveId, Box<dyn std::error::Error>> {
    let mut states = objective_path
        .iter()
        .enumerate()
        .map(|(index, objective_id)| {
            (
                *objective_id,
                if index == 0 {
                    ReplayRouteObjectiveState::Active
                } else {
                    ReplayRouteObjectiveState::Pending
                },
            )
        })
        .collect::<Vec<_>>();
    for (index, transition) in transitions.iter().enumerate() {
        if usize::from(transition.ordinal) != index {
            return Err(io::Error::other("route transition ordinal is not consecutive").into());
        }
        let state = states
            .iter_mut()
            .find(|(objective_id, _)| *objective_id == transition.objective_id)
            .ok_or_else(|| io::Error::other("route transition names an unknown objective"))?;
        if state.1 != transition.from {
            return Err(io::Error::other("route transition source state is inconsistent").into());
        }
        state.1 = transition.to;
    }
    let mut active = states.into_iter().filter_map(|(objective_id, state)| {
        (state == ReplayRouteObjectiveState::Active).then_some(objective_id)
    });
    let objective_id = active
        .next()
        .ok_or_else(|| io::Error::other("route operation has no active objective"))?;
    if active.next().is_some() {
        return Err(io::Error::other("route operation has multiple active objectives").into());
    }
    Ok(objective_id)
}

fn route_terminal_replay(
    selection: &RouteSelectedPayloadV1,
    summary: &DomainRouteOperationSummary,
    transitions: Vec<ReplayObjectiveTransition>,
    encounter: &RouteEncounter,
    grants: Vec<ReplayRouteGrantEvidence>,
) -> Result<RouteTerminalPayloadV1, Box<dyn std::error::Error>> {
    Ok(RouteTerminalPayloadV1 {
        schema_version: ROUTE_REPLAY_SCHEMA_VERSION,
        authoring_revision: selection.authoring_revision.clone(),
        catalog_revision: selection.catalog_revision.clone(),
        resolver_revision: selection.resolver_revision.clone(),
        operation_id: selection.operation_id.clone(),
        route_id: selection.route_id,
        event_id: selection.event_id,
        transitions,
        encounter: ReplayRouteEncounterEvidence {
            participant_count: u8::try_from(selection.participants.len())?,
            drone_health: encounter.drone_health,
            drone_damage: encounter.drone_damage,
            drone_defeated: encounter.drone_defeated,
            warden_spawned: encounter.warden_spawned,
            warden_base_health: encounter.warden_base_health,
            warden_effective_health: encounter.warden_effective_health,
            warden_remaining_health: encounter.warden_remaining_health,
            accepted_player_hits: encounter.accepted_player_hits,
            warden_counter_count: encounter.warden_counter_count,
            warden_counter_damage: selection.effect.warden_counter_damage,
            total_hostile_damage: encounter.total_hostile_damage,
        },
        summary: summary.clone(),
        grants,
    })
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

struct LoadedCharacterState {
    inventory: Vec<ItemStack>,
    progression: DomainProgression,
    equipped_weapon_item_id: String,
    module_state: PersistedModuleState,
}

fn load_character_state(
    persistence: &mut Persistence,
    character_id: &str,
) -> Result<LoadedCharacterState, Box<dyn std::error::Error>> {
    let inventory = persistence
        .inventory_for(character_id)?
        .into_iter()
        .map(|entry| {
            Ok(ItemStack {
                item_id: entry.item_id,
                quantity: u32::try_from(entry.quantity)?,
            })
        })
        .collect::<Result<Vec<_>, std::num::TryFromIntError>>()?;
    let progression = persistence
        .progression_for(character_id)?
        .ok_or_else(|| io::Error::other("character progression is missing"))?;
    let equipped_weapon_item_id = persistence
        .equipped_weapon_for(character_id)?
        .ok_or_else(|| io::Error::other("character equipment loadout is missing"))?;
    let module_state = persistence.module_state_for(character_id)?;
    Ok(LoadedCharacterState {
        inventory,
        progression: DomainProgression::from_experience(u64::try_from(progression.experience)?),
        equipped_weapon_item_id,
        module_state,
    })
}

fn resolved_arsenal_projection(
    state: &PersistedModuleState,
    modules_active: bool,
    content_capable: bool,
    arsenal_catalog: Option<&str>,
) -> Result<ResolvedModuleProjection, Box<dyn std::error::Error>> {
    let modules = if modules_active {
        state.loadout.as_slice()
    } else {
        &[]
    };
    let mut max_health = None;
    let revision = arsenal_catalog.unwrap_or(if content_capable {
        CONTENT_CATALOG_REVISION
    } else {
        CATALOG_REVISION
    });
    let mut profiles = vec![PULSE_RIFLE_PROFILE, ARC_SIDEARM_PROFILE];
    if content_capable {
        profiles.push(COIL_LANCE_PROFILE);
    }
    if arsenal_catalog.is_some() {
        profiles.extend([
            revenant_inventory::SCATTER_CASTER_PROFILE,
            revenant_inventory::RAIL_DRIVER_PROFILE,
        ]);
    }
    let weapons = profiles
        .into_iter()
        .map(|base| {
            let resolved = resolve_build(
                revision,
                BaseCombatProfile {
                    damage: base.damage,
                    range: base.range,
                    cooldown_ms: base.cooldown_ms,
                    max_health: 100,
                },
                modules,
            )?;
            if let Some(expected) = max_health {
                if resolved.profile.max_health != expected {
                    return Err(io::Error::other(
                        "module maximum health disagrees between weapon projections",
                    )
                    .into());
                }
            } else {
                max_health = Some(resolved.profile.max_health);
            }
            Ok(ModuleWeaponProfile {
                item_id: base.item_id.to_owned(),
                base_damage: base.damage,
                base_range: base.range,
                base_cooldown_ms: base.cooldown_ms,
                effective_damage: resolved.profile.damage,
                effective_range: resolved.profile.range,
                effective_cooldown_ms: resolved.profile.cooldown_ms,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    Ok(ResolvedModuleProjection {
        weapons,
        max_health: max_health.ok_or_else(|| io::Error::other("module weapon catalog is empty"))?,
    })
}

fn arsenal_module_snapshot(
    state: &PersistedModuleState,
    content_capable: bool,
    arsenal_catalog: Option<&str>,
) -> Result<ModuleSnapshot, Box<dyn std::error::Error>> {
    let projection = resolved_arsenal_projection(state, true, content_capable, arsenal_catalog)?;
    let mut owned_modules = state.owned_modules.clone();
    owned_modules.sort_unstable();
    if !content_capable {
        owned_modules.retain(|id| ModuleId::LEGACY.contains(id));
    }
    if arsenal_catalog != Some(revenant_modules::BUILD_CATALOG_REVISION) {
        owned_modules.retain(|id| ModuleId::CONTENT.contains(id));
    }
    Ok(ModuleSnapshot {
        catalog_revision: arsenal_catalog
            .unwrap_or(if content_capable {
                CONTENT_CATALOG_REVISION
            } else {
                CATALOG_REVISION
            })
            .to_owned(),
        fragments: state.fragments,
        owned_modules: owned_modules
            .into_iter()
            .map(|module_id| module_id.to_string())
            .collect(),
        loadout_revision: state.revision,
        equipped_modules: state.loadout.iter().map(ToString::to_string).collect(),
        maximum_slots: u8::try_from(MAX_EQUIPPED_MODULES)?,
        catalog: (if arsenal_catalog == Some(revenant_modules::BUILD_CATALOG_REVISION) {
            revenant_modules::BUILD_MODULE_CATALOG.as_slice()
        } else if content_capable {
            CONTENT_MODULE_CATALOG.as_slice()
        } else {
            MODULE_CATALOG.as_slice()
        })
        .iter()
        .map(|definition| ModuleCatalogEntry {
            module_id: definition.module_id.to_string(),
            family: definition.family.to_string(),
            recipe_fragments: definition.recipe_fragments,
            damage_basis_points: definition.damage_basis_points,
            cooldown_basis_points: definition.cooldown_basis_points,
            range_delta: definition.range_delta,
            max_health_delta: definition.max_health_delta,
        })
        .collect(),
        weapons: projection.weapons,
        max_health: projection.max_health,
    })
}

fn equipment_weapon_profile(profile: &ModuleWeaponProfile) -> WireWeaponProfile {
    WireWeaponProfile {
        item_id: profile.item_id.clone(),
        damage: profile.effective_damage,
        range: profile.effective_range,
        cooldown_ms: profile.effective_cooldown_ms,
    }
}

fn parse_wire_modules(
    modules: &[String],
    content_capable: bool,
    build_capable: bool,
) -> Result<Vec<ModuleId>, Box<dyn std::error::Error>> {
    if !build_capable
        && modules.iter().any(|id| {
            id.parse::<ModuleId>()
                .is_ok_and(|id| !ModuleId::CONTENT.contains(&id))
        })
    {
        return Err(io::Error::other("module requires build negotiation").into());
    }
    if !content_capable
        && modules.iter().any(|id| {
            id.parse::<ModuleId>()
                .is_ok_and(|id| !ModuleId::LEGACY.contains(&id))
        })
    {
        return Err(io::Error::other("module requires content negotiation").into());
    }

    if modules.len() > MAX_MODULE_LOADOUT_ENTRIES {
        return Err(io::Error::other("module request exceeds three entries").into());
    }
    let parsed = modules
        .iter()
        .map(|module_id| {
            if module_id.is_empty() || module_id.len() > MAX_MODULE_ID_BYTES {
                return Err(io::Error::other(
                    "module identifier must contain 1-32 bytes",
                ));
            }
            module_id
                .parse::<ModuleId>()
                .map_err(|error| io::Error::other(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(canonicalize_loadout(&parsed)?)
}

fn validate_wire_operation_id(operation_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    if operation_id.len() > MAX_MODULE_OPERATION_ID_BYTES {
        return Err(io::Error::other("operation identifier exceeds 32 bytes").into());
    }
    Ok(validate_operation_id(operation_id)?)
}

fn progression_message(progression: DomainProgression) -> ProgressionSnapshot {
    ProgressionSnapshot {
        level: progression.level,
        experience: progression.experience,
        experience_to_next_level: experience_to_next_level(progression.experience),
    }
}

fn experience_to_next_level(experience: u64) -> u64 {
    EXPERIENCE_PER_LEVEL - experience % EXPERIENCE_PER_LEVEL
}

fn new_session_id() -> io::Result<String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::other(format!("system clock error: {error}")))?
        .as_nanos();
    Ok(format!("session-{timestamp}"))
}

fn valid_movement_target(position: [i32; 3]) -> bool {
    position[1] == 0 && (-12..=12).contains(&position[0]) && (-12..=12).contains(&position[2])
}

fn scaled_enemy_health(base_health: u32, participant_count: usize) -> io::Result<u32> {
    if participant_count == 0 {
        return Err(io::Error::other(
            "enemy health scaling requires at least one participant",
        ));
    }
    let additional_participants = u32::try_from(participant_count - 1)
        .map_err(|_| io::Error::other("participant count exceeds enemy health scaling"))?;
    let scale_numerator = 2_u32
        .checked_add(additional_participants)
        .ok_or_else(|| io::Error::other("enemy health scale overflowed"))?;
    base_health
        .checked_mul(scale_numerator)
        .map(|scaled| scaled / 2)
        .ok_or_else(|| io::Error::other("scaled enemy health overflowed"))
}

fn session_accepts_join(stage: SessionStage, participants: usize, expected_players: usize) -> bool {
    stage == SessionStage::Waiting && participants < expected_players
}

fn unexpected_message(expected: &str) -> Box<dyn std::error::Error> {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expected {expected} in the current connection state"),
    )
    .into()
}
#[cfg(test)]
fn resolved_module_projection(
    state: &PersistedModuleState,
    active: bool,
    content: bool,
) -> Result<ResolvedModuleProjection, Box<dyn std::error::Error>> {
    resolved_arsenal_projection(state, active, content, None)
}

#[cfg(test)]
fn module_snapshot(
    state: &PersistedModuleState,
    content: bool,
) -> Result<ModuleSnapshot, Box<dyn std::error::Error>> {
    arsenal_module_snapshot(state, content, None)
}

#[cfg(test)]
mod tests {
    mod acquisition;
    mod arsenal;
    mod bulwark;
    mod campaign_breach;
    mod campaign_story;
    mod elite;
    mod lancer;
    mod prism;
    mod support;
    use std::io::{self, Cursor};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::mpsc::{self, Receiver, TryRecvError};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{
        campaign, configured_inspector_origin, database_startup_mode, deadline_expired,
        handle_http_request, inspector_request_path, inspector_route, module_snapshot,
        read_http_request, read_http_request_line, resolved_module_projection, scaled_enemy_health,
        session_accepts_join, valid_movement_target, ActivityComplete, AiController,
        CombinationOutcome, CooperationDisposition, CooperationMutationResult,
        CooperationOperationInput, CooperationReceipt, CorrelationDigest, DatabaseStartupMode,
        DomainCooperationRole, DomainCooperationState, FrameRateExceeded, FrameRateLimiter,
        HttpRequest, HttpResponse, InspectorRoute, LoadoutOutcome, ModuleActivityPhase,
        ModuleCombineIntent, ModuleId, ModuleJoinReplayContext, ModuleLoadoutIntent,
        ModuleMutationReplayContext, ModulePreviewRequest, OperationReceipt, OutboundSender,
        Participant, PersistedModuleState, ProtocolGeneration, RecoveringSessionPersistence,
        RouteChoiceIntent, RouteDisposition, RouteOperationSelection, RouteOperationState,
        RouteReceipt, RouteSeed, RouteSelectedPayloadV1, RouteSelectionRequest,
        RouteTerminalOutcome, RouteTerminalPayloadV1, ServerMessage, SessionCommand,
        SessionPersistence, SessionPersistenceConnection, SessionPersistenceConnector,
        SessionPersistenceFailure, SessionStage, SharedSession, DEFAULT_BIND_ADDR,
        DEFAULT_GAME_ADDR, DEFAULT_INSPECTOR_ORIGIN, FRAME_BURST_LIMIT, FRAME_MINUTE_LIMIT,
        GAMEPLAY_IDLE_TIMEOUT, HANDSHAKE_TIMEOUT, MAX_HTTP_HEADER_BYTES, MAX_HTTP_HEADER_LINE,
        MAX_HTTP_HEADER_LINES, MAX_HTTP_REQUEST_LINE, OUTBOUND_QUEUE_CAPACITY,
        WARDEN_PRESSURE_PROFILE,
    };
    use revenant_activities::ScriptedActivity;
    use revenant_actors::{Actor, ActorKind, ActorRegistry};
    use revenant_objectives::WorldTrigger;
    use revenant_operations::{
        apply_warden_health_effect, resolve_event, route_definition, RouteEventId, RouteId,
        RoutePhase,
    };
    use revenant_persistence::{
        ActivityParticipantCompletion, CompletionRewards, NewReplayEvent,
        Progression as PersistedProgression,
    };

    struct FixturePersistence {
        campaign_state: Option<revenant_campaign::CampaignState>,
        fail_operation: Option<String>,
        calls: Arc<Mutex<Vec<String>>>,
        completion_rewards: Vec<Option<CompletionRewards>>,
        module_state: revenant_modules::ModuleState,
        route_operation: Option<RouteOperationState>,
        route_selection_replay: Option<RouteSelectedPayloadV1>,
        route_fragments: u32,
        route_experience: u64,
        cooperation_state: Option<DomainCooperationState>,
        acquisition_state: revenant_persistence::PersistedAcquisitionState,
    }

    type ActiveCooperationFixture = (
        SharedSession,
        Receiver<ServerMessage>,
        Receiver<ServerMessage>,
        Arc<Mutex<Vec<String>>>,
    );

    impl FixturePersistence {
        fn new(
            fail_operation: Option<&str>,
            calls: Arc<Mutex<Vec<String>>>,
            completion_rewards: Vec<Option<CompletionRewards>>,
        ) -> Self {
            Self {
                campaign_state: None,
                fail_operation: fail_operation.map(str::to_owned),
                calls,
                completion_rewards,
                module_state: revenant_modules::ModuleState::new(0, &[], &[], 0)
                    .expect("empty fixture module state should be valid"),
                route_operation: None,
                route_selection_replay: None,
                route_fragments: 0,
                route_experience: 0,
                cooperation_state: None,
                acquisition_state: revenant_persistence::PersistedAcquisitionState::default(),
            }
        }

        fn with_module_state(
            mut self,
            fragments: u32,
            owned_modules: &[revenant_modules::ModuleId],
            loadout: &[revenant_modules::ModuleId],
            revision: u64,
        ) -> Self {
            self.module_state =
                revenant_modules::ModuleState::new(fragments, owned_modules, loadout, revision)
                    .expect("fixture module state should be valid");
            self
        }

        fn persisted_module_state(&self) -> PersistedModuleState {
            PersistedModuleState {
                fragments: self
                    .module_state
                    .fragments()
                    .checked_add(self.route_fragments)
                    .expect("fixture route fragment total should remain bounded"),
                owned_modules: self.module_state.owned_modules(),
                loadout: self.module_state.loadout().to_vec(),
                revision: self.module_state.revision(),
                combination_operations: self.module_state.combination_operation_count(),
                loadout_operations: self.module_state.loadout_operation_count(),
            }
        }

        fn record(&mut self, operation: &str) -> Result<(), Box<dyn std::error::Error>> {
            self.calls
                .lock()
                .expect("fixture call ledger should lock")
                .push(operation.to_owned());
            if self.fail_operation.as_deref() == Some(operation) {
                self.fail_operation = None;
                return Err(
                    SessionPersistenceFailure(format!("fixture rejected {operation}")).into(),
                );
            }
            Ok(())
        }
    }

    impl SessionPersistence for FixturePersistence {
        fn apply_campaign_command(
            &mut self,
            _character_id: &str,
            request: &revenant_persistence::CampaignRequest<'_>,
            _context: ModuleMutationReplayContext<'_>,
        ) -> campaign::Result<revenant_persistence::CampaignReceipt> {
            self.record("campaign_checkpoint")?;
            let transition = self
                .campaign_state
                .as_ref()
                .ok_or_else(|| io::Error::other("campaign fixture missing"))?
                .propose(request.expected_revision, request.command.clone())?;
            if transition.first_clear.is_some() {
                return Err(
                    io::Error::other("first-clear fixture requires real PostgreSQL").into(),
                );
            }
            self.campaign_state = Some(transition.after.clone());
            Ok(revenant_persistence::CampaignReceipt {
                replayed: false,
                state: transition.after.clone(),
                transition,
                rewards: None,
            })
        }
        fn refresh_acquisition_progress(
            &mut self,
            _account_id: &str,
            _character_id: &str,
        ) -> super::AcquisitionStateResult {
            self.record("acquisition_state")?;
            Ok(self.acquisition_state.clone())
        }
        fn claim_acquisition_with_replay(
            &mut self,
            _character_id: &str,
            arc_id: revenant_modules::acquisition::ArcId,
            _context: ModuleMutationReplayContext<'_>,
        ) -> super::AcquisitionMutationResult {
            self.record("acquisition_claimed")?;
            let mut domain = self.acquisition_state.domain()?;
            match domain.claim(arc_id, self.persisted_module_state().fragments) {
                Ok(grant) => {
                    if !grant.replayed {
                        self.acquisition_state.claimed.push(arc_id);
                        let state = self.persisted_module_state();
                        self.module_state = revenant_modules::ModuleState::new(
                            grant.resulting_fragments,
                            &state.owned_modules,
                            &state.loadout,
                            state.revision,
                        )?;
                    }
                    Ok(Ok(grant))
                }
                Err(error) => Ok(Err(error.to_string())),
            }
        }

        fn append_player_joined_with_module_snapshot(
            &mut self,
            _character_id: &str,
            _context: ModuleJoinReplayContext<'_>,
        ) -> Result<PersistedModuleState, Box<dyn std::error::Error>> {
            self.record("player_joined")?;
            Ok(self.persisted_module_state())
        }

        fn module_state_for(
            &mut self,
            _character_id: &str,
        ) -> Result<PersistedModuleState, Box<dyn std::error::Error>> {
            self.record("module_state_loaded")?;
            Ok(self.persisted_module_state())
        }

        fn combine_module_with_replay(
            &mut self,
            _character_id: &str,
            phase: ModuleActivityPhase,
            operation_id: &str,
            module_id: ModuleId,
            _replay: ModuleMutationReplayContext<'_>,
        ) -> Result<Result<OperationReceipt<CombinationOutcome>, String>, Box<dyn std::error::Error>>
        {
            self.record("module_combined")?;
            Ok(self
                .module_state
                .combine(
                    phase,
                    revenant_modules::CombinationRequest {
                        operation_id,
                        module_id,
                    },
                )
                .map_err(|error| error.to_string()))
        }

        fn set_module_loadout_with_replay(
            &mut self,
            _character_id: &str,
            phase: ModuleActivityPhase,
            operation_id: &str,
            expected_revision: u64,
            modules: &[ModuleId],
            _replay: ModuleMutationReplayContext<'_>,
        ) -> Result<Result<OperationReceipt<LoadoutOutcome>, String>, Box<dyn std::error::Error>>
        {
            self.record("module_loadout_changed")?;
            Ok(self
                .module_state
                .set_loadout(
                    phase,
                    revenant_modules::LoadoutRequest {
                        operation_id,
                        expected_revision,
                        modules,
                    },
                )
                .map_err(|error| error.to_string()))
        }

        fn equip_weapon_with_replay(
            &mut self,
            _character_id: &str,
            _item_id: &str,
            _replay_event: &NewReplayEvent<'_>,
        ) -> Result<bool, Box<dyn std::error::Error>> {
            self.record("equipment_changed")?;
            Ok(true)
        }

        fn complete_activity_with_rewards_and_replay(
            &mut self,
            _activity_completed: &NewReplayEvent<'_>,
            participants: &[ActivityParticipantCompletion<'_>],
        ) -> Result<Vec<Option<CompletionRewards>>, Box<dyn std::error::Error>> {
            self.record("completion_transaction")?;
            if self.completion_rewards.is_empty() {
                Ok(vec![None; participants.len()])
            } else {
                assert_eq!(self.completion_rewards.len(), participants.len());
                Ok(self.completion_rewards.clone())
            }
        }

        fn select_route_operation_with_replay(
            &mut self,
            selection: &RouteOperationSelection<'_>,
            replay: &RouteSelectedPayloadV1,
        ) -> Result<
            Result<RouteReceipt<super::RouteSelectionOutcome>, String>,
            Box<dyn std::error::Error>,
        > {
            self.record("route_selected")?;
            revenant_replay::encode_route_selected(replay)?;
            if self.route_operation.is_none() {
                let participant_ids = selection
                    .participants
                    .iter()
                    .map(|participant| participant.account_id)
                    .collect::<Vec<_>>();
                let mut operation = RouteOperationState::new(&participant_ids)?;
                for participant_id in &participant_ids {
                    operation.mark_capable(participant_id)?;
                }
                operation.open_choice()?;
                self.route_operation = Some(operation);
            }
            let receipt = self
                .route_operation
                .as_mut()
                .expect("fixture route operation should exist")
                .select(
                    selection.requester_account_id,
                    RouteSelectionRequest {
                        operation_id: selection.operation_id,
                        route_id: selection.route_id,
                    },
                    selection.candidate_seed,
                )
                .map_err(|error| error.to_string());
            if receipt
                .as_ref()
                .is_ok_and(|receipt| receipt.disposition == RouteDisposition::Applied)
            {
                self.route_selection_replay = Some(replay.clone());
            }
            Ok(receipt)
        }

        fn complete_route_operation_with_replay(
            &mut self,
            _session_id: &str,
            elapsed_ms: u64,
            replay: &RouteTerminalPayloadV1,
        ) -> Result<RouteReceipt<super::DomainRouteOperationSummary>, Box<dyn std::error::Error>>
        {
            self.record("route_completed")?;
            let selection = self
                .route_selection_replay
                .as_ref()
                .ok_or_else(|| std::io::Error::other("fixture route selection is missing"))?;
            revenant_replay::encode_route_operation_succeeded(replay, selection)?;
            let receipt = self
                .route_operation
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture route operation is missing"))?
                .complete(elapsed_ms)?;
            if receipt.disposition == RouteDisposition::Applied {
                let reward = receipt
                    .outcome
                    .reward
                    .expect("successful fixture route should have a reward");
                self.route_fragments = self
                    .route_fragments
                    .checked_add(reward.fragments)
                    .ok_or_else(|| std::io::Error::other("fixture fragment total overflowed"))?;
                self.route_experience = self
                    .route_experience
                    .checked_add(reward.experience)
                    .ok_or_else(|| std::io::Error::other("fixture XP total overflowed"))?;
            }
            Ok(receipt)
        }

        fn fail_route_operation_timeout_with_replay(
            &mut self,
            _session_id: &str,
            elapsed_ms: u64,
            replay: &RouteTerminalPayloadV1,
        ) -> Result<RouteReceipt<super::DomainRouteOperationSummary>, Box<dyn std::error::Error>>
        {
            self.record("route_failed")?;
            let selection = self
                .route_selection_replay
                .as_ref()
                .ok_or_else(|| std::io::Error::other("fixture route selection is missing"))?;
            revenant_replay::encode_route_operation_failed(replay, selection)?;
            Ok(self
                .route_operation
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture route operation is missing"))?
                .fail_timeout(elapsed_ms)?)
        }

        fn start_cooperation_operation_with_replay(
            &mut self,
            input: &CooperationOperationInput<'_>,
        ) -> CooperationMutationResult {
            self.record("cooperation_started")?;
            let participants: [&revenant_persistence::CooperationParticipant<'_>; 2] = input
                .participants
                .iter()
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| std::io::Error::other("fixture cooperation requires two players"))?;
            let profiles = [participants[0].profile, participants[1].profile];
            let health = [
                participants[0].current_health,
                participants[1].current_health,
            ];
            let result = if let Some(state) = self.cooperation_state.as_ref() {
                state
                    .retry_start(input.operation_id, profiles, health)
                    .map(|disposition| CooperationReceipt {
                        disposition,
                        state: state.clone(),
                    })
            } else {
                DomainCooperationState::start_with_health(input.operation_id, profiles, health).map(
                    |state| {
                        self.cooperation_state = Some(state.clone());
                        CooperationReceipt {
                            disposition: CooperationDisposition::Applied,
                            state,
                        }
                    },
                )
            };
            Ok(result.map_err(|error| error.to_string()))
        }

        fn arrive_cooperation_anchor(
            &mut self,
            _session_id: &str,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_anchor_arrived")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.arrive_anchor(
                DomainCooperationRole::Anchor,
                revenant_cooperation::Target::RelayAnchor,
                elapsed_ms,
            );
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn ping_cooperation_operation_with_replay(
            &mut self,
            _session_id: &str,
            operation_id: &str,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_pinged")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.ping(
                DomainCooperationRole::Anchor,
                revenant_cooperation::Target::RelayConsole,
                operation_id,
                elapsed_ms,
            );
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn down_cooperation_runner_with_replay(
            &mut self,
            _session_id: &str,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_runner_downed")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.arrive_runner(
                DomainCooperationRole::Runner,
                revenant_cooperation::Target::RelayConsole,
                elapsed_ms,
            );
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn start_cooperation_revive(
            &mut self,
            _session_id: &str,
            operation_id: &str,
            distance_squared: i64,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_revive_started")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.start_revive(
                DomainCooperationRole::Anchor,
                DomainCooperationRole::Runner,
                distance_squared,
                operation_id,
                elapsed_ms,
            );
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn observe_cooperation_revive_with_replay(
            &mut self,
            _session_id: &str,
            operation_id: &str,
            distance_squared: i64,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_revive_observed")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.observe_revive(
                DomainCooperationRole::Anchor,
                DomainCooperationRole::Runner,
                distance_squared,
                operation_id,
                elapsed_ms,
            );
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn complete_cooperation_operation_with_replay(
            &mut self,
            _session_id: &str,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_completed")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.complete_warden(elapsed_ms);
            if result
                .as_ref()
                .is_ok_and(|disposition| *disposition == CooperationDisposition::Applied)
            {
                self.route_fragments = self
                    .route_fragments
                    .checked_add(super::REWARD_FRAGMENTS_PER_PARTICIPANT)
                    .ok_or_else(|| std::io::Error::other("fixture fragment total overflowed"))?;
                self.route_experience = self
                    .route_experience
                    .checked_add(super::REWARD_EXPERIENCE_PER_PARTICIPANT)
                    .ok_or_else(|| std::io::Error::other("fixture XP total overflowed"))?;
            }
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn timeout_cooperation_operation_with_replay(
            &mut self,
            _session_id: &str,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_timed_out")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = match state.observe_deadlines(elapsed_ms) {
                Err(revenant_cooperation::CooperationError::DeadlineExpired(_)) => {
                    Ok(CooperationDisposition::Applied)
                }
                other => other,
            };
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn defeat_cooperation_participant_with_replay(
            &mut self,
            _session_id: &str,
            role: DomainCooperationRole,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_participant_defeated")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.defeat_participant(role, elapsed_ms);
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn abandon_cooperation_operation_with_replay(
            &mut self,
            _session_id: &str,
            role: DomainCooperationRole,
            elapsed_ms: u64,
        ) -> CooperationMutationResult {
            self.record("cooperation_abandoned")?;
            let state = self
                .cooperation_state
                .as_mut()
                .ok_or_else(|| std::io::Error::other("fixture cooperation is missing"))?;
            let result = state.disconnect(role, elapsed_ms);
            Ok(result
                .map(|disposition| CooperationReceipt {
                    disposition,
                    state: state.clone(),
                })
                .map_err(|error| error.to_string()))
        }

        fn progression_for(
            &mut self,
            _character_id: &str,
        ) -> Result<PersistedProgression, Box<dyn std::error::Error>> {
            self.record("progression_loaded")?;
            let progression =
                revenant_progression::Progression::from_experience(self.route_experience);
            Ok(PersistedProgression {
                level: i32::try_from(progression.level)?,
                experience: i64::try_from(progression.experience)?,
            })
        }

        fn append_replay_event(
            &mut self,
            event: &NewReplayEvent<'_>,
        ) -> Result<i64, Box<dyn std::error::Error>> {
            self.record(event.event_type)?;
            Ok(1)
        }
    }

    pub(super) fn fixture_participant(
        account_id: &str,
        actor_id: u64,
        owned_items: &[&str],
    ) -> (Participant, Receiver<ServerMessage>) {
        let correlation = CorrelationDigest::fixture("00000000000000000000000000000000");
        let (outbound, receiver) = OutboundSender::channel(correlation.clone());
        let module_state = PersistedModuleState {
            fragments: 0,
            owned_modules: Vec::new(),
            loadout: Vec::new(),
            revision: 0,
            combination_operations: 0,
            loadout_operations: 0,
        };
        let admitted = resolved_module_projection(&module_state, true, false)
            .expect("empty fixture module projection should resolve");
        (
            Participant {
                account_id: account_id.to_owned(),
                character_id: format!("{account_id}:operator"),
                correlation,
                actor: Actor {
                    id: actor_id,
                    kind: ActorKind::Player,
                    archetype: "operator".to_owned(),
                    position: [0, 0, 0],
                    health: 100,
                    max_health: 100,
                },
                outbound,
                protocol_generation: ProtocolGeneration::CurrentV2,
                content_capable: false,
                exploration_capable: false,
                encounter_capable: false,
                bulwark_capable: false,
                support_capable: false,
                elite_capable: false,
                prism_capable: false,
                arsenal_capable: false,
                build_capable: false,
                acquisition_capable: false,
                campaign_entry: None,
                challenge_entry: None,
                owned_items: owned_items.iter().map(|item| (*item).to_owned()).collect(),
                equipped_weapon_item_id: "pulse_rifle".to_owned(),
                module_state,
                admitted_module_weapons: admitted.weapons,
                admitted_max_health: admitted.max_health,
                route_capable: false,
                cooperation_capable: false,
            },
            receiver,
        )
    }

    fn fixture_activity(stage: SessionStage) -> ScriptedActivity {
        let mut activity = ScriptedActivity::load("../../scripts/activities/relay_awakening.lua")
            .expect("fixture activity should load");
        if stage != SessionStage::Waiting {
            let _ = activity.start();
        }
        if matches!(
            stage,
            SessionStage::Door | SessionStage::Boss | SessionStage::Complete
        ) {
            activity.apply_trigger(&WorldTrigger::ActorGroupDead {
                group_id: "relay_drones".to_owned(),
            });
        }
        if matches!(stage, SessionStage::Boss | SessionStage::Complete) {
            activity.apply_trigger(&WorldTrigger::AreaReached {
                area_id: "relay_door".to_owned(),
            });
        }
        activity
    }

    fn fixture_session(
        expected_players: usize,
        participants: Vec<Participant>,
        stage: SessionStage,
        persistence: FixturePersistence,
    ) -> SharedSession {
        let mut actors = ActorRegistry::default();
        for participant in &participants {
            actors.insert(participant.actor.clone());
        }
        SharedSession {
            expected_players,
            participants,
            actors,
            activity: fixture_activity(stage),
            campaign: None,
            challenge: None,
            combat: super::CombatRuntime::default(),
            combat_started: Instant::now(),
            enemy_ai: None,
            enemy_id: None,
            stage,
            persistence: Box::new(persistence),
            session_id: "session-fixture".to_owned(),
            activity_script: "../../scripts/activities/relay_awakening.lua".to_owned(),
            route: None,
            cooperation: None,
            route_seed_source: Box::new(|| {
                RouteSeed::new(0).map_err(|error| std::io::Error::other(error.to_string()))
            }),
            signal: None,
            coolant: None,
            meridian: None,
            lancer: None,
            bulwark: None,
            support: None,
            elite: None,
            prism: None,
        }
    }

    fn fatal_damage(source_actor_id: u64, target_actor_id: u64) -> ServerMessage {
        ServerMessage::DamageApplied(super::DamageApplied {
            source_actor_id,
            target_actor_id,
            damage: 100,
            remaining_health: 0,
            killed: true,
        })
    }

    fn message_kind(message: &ServerMessage) -> &'static str {
        match message {
            ServerMessage::DamageApplied(_) => "DamageApplied",
            ServerMessage::ActorDestroy(_) => "ActorDestroy",
            ServerMessage::ObjectiveUpdate(_) => "ObjectiveUpdate",
            ServerMessage::LootGranted(_) => "LootGranted",
            ServerMessage::ProgressionGranted(_) => "ProgressionGranted",
            ServerMessage::ActivityComplete(_) => "ActivityComplete",
            ServerMessage::EquipmentChanged(_) => "EquipmentChanged",
            ServerMessage::RouteState(_) => "RouteState",
            ServerMessage::RouteChoiceResult(_) => "RouteChoiceResult",
            ServerMessage::RouteOperationSummary(_) => "RouteOperationSummary",
            _ => "Other",
        }
    }

    fn drain_messages(receiver: &Receiver<ServerMessage>) -> Vec<ServerMessage> {
        receiver.try_iter().collect()
    }

    fn start_and_clear_drone(session: &mut SharedSession, player_id: u64) {
        session
            .start_activity()
            .expect("route fixture activity should start");
        let enemy_id = session
            .enemy_id
            .expect("route fixture should spawn the relay drone");
        let mut elapsed_ms = 0;
        while session.stage == SessionStage::Drone {
            session
                .attack_at(player_id, enemy_id, elapsed_ms)
                .expect("route fixture drone hit should be accepted");
            elapsed_ms += 250;
        }
        assert_eq!(session.stage, SessionStage::Door);
        assert_eq!(
            session
                .route
                .as_ref()
                .expect("route fixture should exist")
                .operation
                .phase(),
            RoutePhase::ChoiceOpen
        );
    }

    fn start_fixture_cooperation(
        session: &mut SharedSession,
        anchor_id: u64,
        runner_id: u64,
        operation_id: &str,
    ) {
        session
            .request_cooperation_state(anchor_id)
            .expect("anchor cooperation capability should be accepted");
        session
            .request_cooperation_state(runner_id)
            .expect("runner cooperation capability should be accepted");
        start_and_clear_drone(session, anchor_id);
        session
            .start_cooperation(
                anchor_id,
                &super::CooperationStartIntent {
                    operation_id: operation_id.to_owned(),
                },
            )
            .expect("fixture cooperation should start");
        assert_eq!(
            session
                .cooperation
                .as_ref()
                .expect("fixture cooperation runtime should exist")
                .state
                .phase(),
            revenant_cooperation::Phase::AwaitingAnchor
        );
    }

    fn drive_fixture_cooperation_to_encounter(
        session: &mut SharedSession,
        anchor_id: u64,
        runner_id: u64,
    ) {
        session
            .move_player_at(anchor_id, [3, 0, 3], 100)
            .expect("anchor should commit its fixed target");
        session
            .ping_cooperation_at(
                anchor_id,
                &super::CooperationPingIntent {
                    operation_id: "coop-ping".to_owned(),
                },
                200,
            )
            .expect("anchor ping should commit");
        session
            .move_player_at(runner_id, [4, 0, 3], 300)
            .expect("runner should commit its fixed target and downing");
        session
            .revive_cooperation_at(
                anchor_id,
                &super::CooperationReviveIntent {
                    operation_id: "coop-revive".to_owned(),
                },
                400,
            )
            .expect("anchor should start the fixed revive");
        assert!(!session
            .observe_cooperation_clock_at(2_399, None)
            .expect("pre-boundary revive observation should remain pending"));
        assert!(!session
            .observe_cooperation_clock_at(2_400, None)
            .expect("exact revive boundary should complete without terminal"));
        assert_eq!(
            session
                .cooperation
                .as_ref()
                .expect("fixture cooperation should remain active")
                .state
                .phase(),
            revenant_cooperation::Phase::EncounterActive
        );
        assert_eq!(session.actors.get(runner_id).unwrap().health, 50);
    }

    fn active_cooperation_fixture(
        fail_operation: Option<&str>,
        anchor_id: u64,
        runner_id: u64,
    ) -> ActiveCooperationFixture {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (anchor, anchor_rx) = fixture_participant(
            &format!("local:anchor-{anchor_id}"),
            anchor_id,
            &["pulse_rifle"],
        );
        let (runner, runner_rx) = fixture_participant(
            &format!("local:runner-{runner_id}"),
            runner_id,
            &["pulse_rifle"],
        );
        let persistence = FixturePersistence::new(fail_operation, Arc::clone(&calls), Vec::new());
        let mut session =
            fixture_session(2, vec![anchor, runner], SessionStage::Waiting, persistence);
        start_fixture_cooperation(&mut session, anchor_id, runner_id, "coop-fixture");
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        (session, anchor_rx, runner_rx, calls)
    }

    fn received_cooperation_summary(
        receiver: &Receiver<ServerMessage>,
    ) -> super::WireCooperationOperationSummary {
        drain_messages(receiver)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::CooperationOperationSummary(summary) => Some(summary),
                _ => None,
            })
            .expect("fixture participant should receive cooperation summary")
    }

    fn seed_for_event(route_id: RouteId, event_id: RouteEventId) -> RouteSeed {
        (0..1_024)
            .map(|value| RouteSeed::new(value).expect("fixture seed should validate"))
            .find(|seed| resolve_event(route_id, *seed).event_id == event_id)
            .expect("bounded fixture vector should reach each route event")
    }

    fn set_fixture_seed(session: &mut SharedSession, seed: RouteSeed) {
        session.route_seed_source = Box::new(move || Ok(seed));
    }

    fn drive_route_to_warden(session: &mut SharedSession, player_id: u64, route_id: RouteId) {
        if route_id == RouteId::Stabilize {
            session
                .move_player(player_id, [3, 0, 3])
                .expect("stabilize route should accept its area trigger");
            assert_eq!(session.stage, SessionStage::Door);
        }
        session
            .move_player(player_id, [6, 0, 0])
            .expect("selected route should open the relay door");
        assert_eq!(session.stage, SessionStage::Boss);
    }

    fn defeat_active_warden(session: &mut SharedSession, player_id: u64) {
        let warden_id = session
            .enemy_id
            .expect("route fixture should have an active Warden");
        let mut elapsed_ms = 0;
        while session.stage == SessionStage::Boss {
            session
                .attack_at(player_id, warden_id, elapsed_ms)
                .expect("route fixture Warden hit should be accepted");
            elapsed_ms += 250;
        }
        assert_eq!(session.stage, SessionStage::Complete);
    }

    fn record_successful_warden_evidence(session: &mut SharedSession) {
        let maximum_damage = session
            .participants
            .iter()
            .filter_map(Participant::admitted_weapon)
            .map(|weapon| weapon.effective_damage)
            .max()
            .expect("route fixture should have an admitted weapon");
        let route = session.route.as_mut().expect("selected route should exist");
        let counter_damage = route
            .operation
            .selection()
            .expect("selected route should expose its effect")
            .effect
            .warden_counter_damage;
        route.encounter.warden_remaining_health = 0;
        route.encounter.accepted_player_hits = route
            .encounter
            .warden_effective_health
            .div_ceil(maximum_damage);
        route.encounter.warden_counter_count = 2;
        route.encounter.total_hostile_damage = 10 + 2 * counter_damage;
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn cooperation_capability_roles_retry_and_route_exclusion_are_authoritative() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (anchor, anchor_rx) =
            fixture_participant("local:cooperation-anchor", 2_100, &["pulse_rifle"]);
        let (runner, runner_rx) =
            fixture_participant("local:cooperation-runner", 2_101, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session =
            fixture_session(2, vec![anchor, runner], SessionStage::Waiting, persistence);

        session
            .request_cooperation_state(2_100)
            .expect("first independent capability should be accepted");
        let first = drain_messages(&anchor_rx)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::CooperationState(state) => Some(state),
                _ => None,
            })
            .expect("anchor should receive its capability state");
        assert_eq!(first.phase, super::WireCooperationPhase::Waiting);
        assert_eq!(first.capable_actor_ids, [2_100]);
        assert!(!first.all_capable);
        assert!(first.operation.is_none());
        assert_eq!(runner_rx.try_recv(), Err(TryRecvError::Empty));

        start_and_clear_drone(&mut session, 2_100);
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        session
            .start_cooperation(
                2_100,
                &super::CooperationStartIntent {
                    operation_id: "coop-authority".to_owned(),
                },
            )
            .expect("incomplete capability should be a targeted rejection");
        let ServerMessage::CooperationStartResult(incomplete) = anchor_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("anchor should receive incomplete capability rejection")
        else {
            panic!("anchor should receive CooperationStartResult");
        };
        assert!(!incomplete.accepted);
        assert!(incomplete.operation.is_none());
        assert!(calls
            .lock()
            .expect("fixture ledger should lock")
            .iter()
            .all(|call| call != "cooperation_started"));

        session
            .request_cooperation_state(2_101)
            .expect("second independent capability should be accepted");
        session
            .request_route_state(2_100)
            .expect("anchor route capability should be accepted before selection");
        session
            .request_route_state(2_101)
            .expect("runner route capability should be accepted before selection");
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);

        session
            .start_cooperation(
                2_101,
                &super::CooperationStartIntent {
                    operation_id: "coop-authority".to_owned(),
                },
            )
            .expect("non-leader start should reject without mutation");
        let ServerMessage::CooperationStartResult(non_leader) = runner_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("runner should receive authority rejection")
        else {
            panic!("runner should receive CooperationStartResult");
        };
        assert!(!non_leader.accepted);
        assert!(non_leader.operation.is_none());

        session
            .start_cooperation(
                2_100,
                &super::CooperationStartIntent {
                    operation_id: "bad_id".to_owned(),
                },
            )
            .expect("invalid identifier should be a bounded rejection");
        let ServerMessage::CooperationStartResult(invalid) = anchor_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("anchor should receive identifier rejection")
        else {
            panic!("anchor should receive CooperationStartResult");
        };
        assert!(!invalid.accepted);
        assert!(invalid.operation_id.is_empty());

        session
            .start_cooperation(
                2_100,
                &super::CooperationStartIntent {
                    operation_id: "coop-authority".to_owned(),
                },
            )
            .expect("leader cooperation start should commit");
        let anchor_messages = drain_messages(&anchor_rx);
        let runner_messages = drain_messages(&runner_rx);
        let anchor_start = anchor_messages
            .iter()
            .find_map(|message| match message {
                ServerMessage::CooperationStartResult(result) if result.accepted => {
                    Some(result.clone())
                }
                _ => None,
            })
            .expect("anchor should receive shared accepted start");
        let runner_start = runner_messages
            .iter()
            .find_map(|message| match message {
                ServerMessage::CooperationStartResult(result) if result.accepted => {
                    Some(result.clone())
                }
                _ => None,
            })
            .expect("runner should receive shared accepted start");
        assert_eq!(anchor_start, runner_start);
        let operation = anchor_start
            .operation
            .expect("committed start should expose immutable operation");
        assert_eq!(operation.participants.len(), 2);
        assert_eq!(operation.participants[0].actor_id, 2_100);
        assert_eq!(
            operation.participants[0].role,
            super::WireCooperationRole::Anchor
        );
        assert_eq!(operation.participants[1].actor_id, 2_101);
        assert_eq!(
            operation.participants[1].role,
            super::WireCooperationRole::Runner
        );
        assert_eq!(
            session
                .route
                .as_ref()
                .expect("route coordinator should remain present")
                .operation
                .phase(),
            RoutePhase::BaselineLocked
        );

        session
            .start_cooperation(
                2_100,
                &super::CooperationStartIntent {
                    operation_id: "coop-authority".to_owned(),
                },
            )
            .expect("exact start retry should replay");
        let replay = drain_messages(&anchor_rx)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::CooperationStartResult(result) => Some(result),
                _ => None,
            })
            .expect("anchor should receive targeted replay");
        assert!(replay.accepted && replay.replayed);
        assert!(drain_messages(&runner_rx).is_empty());

        session
            .choose_route(
                2_100,
                &RouteChoiceIntent {
                    operation_id: "route-after-cooperation".to_owned(),
                    route_id: "breach".to_owned(),
                },
            )
            .expect("cooperation-first route request should reject");
        let route_rejection = drain_messages(&anchor_rx)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::RouteChoiceResult(result) => Some(result),
                _ => None,
            })
            .expect("anchor should receive route rejection");
        assert!(!route_rejection.accepted);
        assert!(calls
            .lock()
            .expect("fixture ledger should lock")
            .iter()
            .all(|call| call != "route_selected"));

        let second_calls = Arc::new(Mutex::new(Vec::new()));
        let (second_anchor, second_anchor_rx) =
            fixture_participant("local:route-first-anchor", 2_110, &["pulse_rifle"]);
        let (second_runner, second_runner_rx) =
            fixture_participant("local:route-first-runner", 2_111, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&second_calls), Vec::new());
        let mut route_first = fixture_session(
            2,
            vec![second_anchor, second_runner],
            SessionStage::Waiting,
            persistence,
        );
        route_first.request_cooperation_state(2_110).unwrap();
        for actor_id in [2_110, 2_111] {
            route_first.request_route_state(actor_id).unwrap();
        }
        start_and_clear_drone(&mut route_first, 2_110);
        drain_messages(&second_anchor_rx);
        drain_messages(&second_runner_rx);
        route_first
            .choose_route(
                2_110,
                &RouteChoiceIntent {
                    operation_id: "route-first".to_owned(),
                    route_id: "breach".to_owned(),
                },
            )
            .expect("route-first selection should commit");
        route_first
            .request_cooperation_state(2_111)
            .expect("late cooperation capability should reject without mutation");
        let late_state = drain_messages(&second_runner_rx)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::CooperationState(state) => Some(state),
                _ => None,
            })
            .expect("late requester should receive current bounded capability truth");
        assert!(!late_state.accepted);
        assert_eq!(late_state.phase, super::WireCooperationPhase::Unavailable);
        assert_eq!(late_state.capable_actor_ids, [2_110]);
        assert!(!late_state.all_capable);
        assert!(late_state.operation.is_none());
        route_first
            .start_cooperation(
                2_110,
                &super::CooperationStartIntent {
                    operation_id: "coop-after-route".to_owned(),
                },
            )
            .expect("route-first cooperation request should reject");
        assert!(route_first.cooperation.is_none());
        assert_eq!(
            second_calls
                .lock()
                .expect("second fixture ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "route_selected")
                .count(),
            1
        );
        assert!(second_calls
            .lock()
            .expect("second fixture ledger should lock")
            .iter()
            .all(|call| call != "cooperation_started"));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn cooperation_lifecycle_is_shared_and_success_rewards_are_exact() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (anchor, anchor_rx) =
            fixture_participant("local:lifecycle-anchor", 2_200, &["pulse_rifle"]);
        let (runner, runner_rx) =
            fixture_participant("local:lifecycle-runner", 2_201, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session =
            fixture_session(2, vec![anchor, runner], SessionStage::Waiting, persistence);
        start_fixture_cooperation(&mut session, 2_200, 2_201, "coop-lifecycle");
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);

        session
            .move_player_at(2_200, [3, 0, 3], 100)
            .expect("anchor target should commit");
        session
            .ping_cooperation_at(
                2_200,
                &super::CooperationPingIntent {
                    operation_id: "coop-ping".to_owned(),
                },
                200,
            )
            .expect("ping should commit");
        session
            .move_player_at(2_201, [4, 0, 3], 300)
            .expect("runner target should commit its downing");
        assert!(session.move_player_at(2_201, [4, 0, 2], 301).is_err());
        session
            .equip_weapon(2_201, "pulse_rifle")
            .expect("active cooperation equipment should reject in-band");
        session
            .revive_cooperation_at(
                2_201,
                &super::CooperationReviveIntent {
                    operation_id: "runner-cannot-revive".to_owned(),
                },
                302,
            )
            .expect("runner revive should reject in-band");
        assert_eq!(session.actors.get(2_201).unwrap().health, 0);
        session
            .revive_cooperation_at(
                2_200,
                &super::CooperationReviveIntent {
                    operation_id: "coop-revive".to_owned(),
                },
                400,
            )
            .expect("anchor revive should start");
        assert!(!session
            .observe_cooperation_clock_at(2_399, None)
            .expect("revive should remain pending one millisecond before its boundary"));
        assert_eq!(session.actors.get(2_201).unwrap().health, 0);
        assert!(!session
            .observe_cooperation_clock_at(2_400, None)
            .expect("revive should complete exactly at its boundary"));
        assert_eq!(session.actors.get(2_201).unwrap().health, 50);

        let anchor_shared = drain_messages(&anchor_rx);
        let runner_shared = drain_messages(&runner_rx);
        let cooperation_messages = |messages: Vec<ServerMessage>| {
            messages
                .into_iter()
                .filter(|message| {
                    matches!(
                        message,
                        ServerMessage::CooperationState(_)
                            | ServerMessage::CooperationPingResult(_)
                            | ServerMessage::CooperationLifeState(_)
                            | ServerMessage::CooperationReviveResult(_)
                    )
                })
                .collect::<Vec<_>>()
        };
        let anchor_cooperation = cooperation_messages(anchor_shared);
        let runner_cooperation = cooperation_messages(runner_shared);
        let anchor_life = anchor_cooperation
            .iter()
            .filter_map(|message| match message {
                ServerMessage::CooperationLifeState(life) => Some(life),
                _ => None,
            })
            .collect::<Vec<_>>();
        let runner_life = runner_cooperation
            .iter()
            .filter_map(|message| match message {
                ServerMessage::CooperationLifeState(life) => Some(life),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(anchor_life, runner_life);
        assert_eq!(anchor_life.len(), 2);
        assert_eq!(anchor_life[0].health_before, 100);
        assert_eq!(anchor_life[0].health_after, 0);
        assert_eq!(anchor_life[1].health_before, 0);
        assert_eq!(anchor_life[1].health_after, 50);

        session
            .move_player_at(2_200, [6, 0, 0], 2_500)
            .expect("successful revive should unlock the baseline Warden");
        assert_eq!(session.stage, SessionStage::Boss);
        session
            .cooperation
            .as_mut()
            .expect("cooperation runtime should remain active")
            .started_at = Instant::now()
            .checked_sub(Duration::from_secs(3))
            .expect("three-second fixture offset should be representable");
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        defeat_active_warden(&mut session, 2_200);

        let assert_success = |messages: Vec<ServerMessage>| {
            let loot = messages
                .iter()
                .position(|message| matches!(message, ServerMessage::LootGranted(_)))
                .expect("cooperation success should publish loot");
            let progression = messages
                .iter()
                .position(|message| matches!(message, ServerMessage::ProgressionGranted(_)))
                .expect("cooperation success should publish progression");
            let complete = messages
                .iter()
                .position(|message| matches!(message, ServerMessage::ActivityComplete(_)))
                .expect("cooperation success should publish activity completion");
            let summary_index = messages
                .iter()
                .position(|message| {
                    matches!(message, ServerMessage::CooperationOperationSummary(_))
                })
                .expect("cooperation success should publish its summary");
            assert!(loot < progression && progression < complete && complete < summary_index);
            let ServerMessage::CooperationOperationSummary(summary) = &messages[summary_index]
            else {
                unreachable!();
            };
            assert_eq!(
                summary.outcome,
                super::WireCooperationTerminalOutcome::Succeeded
            );
            assert_eq!(
                summary.last_nonterminal_phase,
                super::WireCooperationPhase::EncounterActive
            );
            assert!(summary.subject_role.is_none());
            assert!(summary.no_reward_reason.is_none());
            assert_eq!(summary.grants.len(), 2);
            assert_eq!(summary.grants[0].actor_id, 2_200);
            assert_eq!(summary.grants[1].actor_id, 2_201);
            assert!(summary.grants.iter().all(|grant| {
                grant.item_id == super::RELAY_CORE_FRAGMENT
                    && grant.item_quantity == super::REWARD_FRAGMENTS_PER_PARTICIPANT
                    && grant.experience == super::REWARD_EXPERIENCE_PER_PARTICIPANT
            }));
            assert!(summary.contributions.anchor_arrived);
            assert!(summary.contributions.pinged);
            assert!(summary.contributions.runner_arrived);
            assert!(summary.contributions.revived);
            assert!(summary.contributions.warden_completed);
            assert_eq!(summary.contributions.revive_count, 1);
        };
        assert_success(drain_messages(&anchor_rx));
        assert_success(drain_messages(&runner_rx));
        assert_eq!(session.stage, SessionStage::Complete);
        assert_eq!(session.participants[0].module_state.fragments, 2);
        assert_eq!(session.participants[1].module_state.fragments, 2);
        assert_eq!(
            calls
                .lock()
                .expect("fixture ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "cooperation_completed")
                .count(),
            1
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn cooperation_clock_boundaries_cancellation_and_disconnect_are_exact() {
        let (mut ping, _anchor_rx, runner_rx, _) = active_cooperation_fixture(None, 2_300, 2_301);
        ping.move_player_at(2_300, [3, 0, 3], 100).unwrap();
        ping.ping_cooperation_at(
            2_300,
            &super::CooperationPingIntent {
                operation_id: "ping-boundary".to_owned(),
            },
            200,
        )
        .unwrap();
        assert!(!ping.observe_cooperation_clock_at(5_200, None).unwrap());
        assert!(ping.observe_cooperation_clock_at(5_201, None).unwrap());
        let summary = received_cooperation_summary(&runner_rx);
        assert_eq!(
            summary.outcome,
            super::WireCooperationTerminalOutcome::FailedPingTimeout
        );
        assert_eq!(
            summary.no_reward_reason,
            Some(super::CooperationNoRewardReason::PingTimeout)
        );
        assert!(summary.reward.is_none() && summary.grants.is_empty());

        let (mut revive_window, _anchor_rx, runner_rx, _) =
            active_cooperation_fixture(None, 2_310, 2_311);
        revive_window.move_player_at(2_310, [3, 0, 3], 100).unwrap();
        revive_window
            .ping_cooperation_at(
                2_310,
                &super::CooperationPingIntent {
                    operation_id: "revive-window-ping".to_owned(),
                },
                200,
            )
            .unwrap();
        revive_window.move_player_at(2_311, [4, 0, 3], 300).unwrap();
        assert!(!revive_window
            .observe_cooperation_clock_at(15_300, None)
            .unwrap());
        assert!(revive_window
            .observe_cooperation_clock_at(15_301, None)
            .unwrap());
        let summary = received_cooperation_summary(&runner_rx);
        assert_eq!(
            summary.outcome,
            super::WireCooperationTerminalOutcome::FailedReviveTimeout
        );
        assert_eq!(
            summary.no_reward_reason,
            Some(super::CooperationNoRewardReason::ReviveTimeout)
        );

        let (mut overall, _anchor_rx, runner_rx, _) =
            active_cooperation_fixture(None, 2_320, 2_321);
        assert!(!overall.observe_cooperation_clock_at(60_000, None).unwrap());
        assert!(overall.observe_cooperation_clock_at(60_001, None).unwrap());
        let summary = received_cooperation_summary(&runner_rx);
        assert_eq!(
            summary.outcome,
            super::WireCooperationTerminalOutcome::FailedOperationTimeout
        );
        assert_eq!(
            summary.no_reward_reason,
            Some(super::CooperationNoRewardReason::OperationTimeout)
        );

        let (mut precedence, _anchor_rx, runner_rx, _) =
            active_cooperation_fixture(None, 2_330, 2_331);
        precedence.move_player_at(2_330, [3, 0, 3], 100).unwrap();
        precedence
            .ping_cooperation_at(
                2_330,
                &super::CooperationPingIntent {
                    operation_id: "precedence-ping".to_owned(),
                },
                200,
            )
            .unwrap();
        assert!(precedence
            .observe_cooperation_clock_at(60_001, None)
            .unwrap());
        assert_eq!(
            received_cooperation_summary(&runner_rx).outcome,
            super::WireCooperationTerminalOutcome::FailedPingTimeout
        );

        let (mut cancellation, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(None, 2_340, 2_341);
        cancellation.move_player_at(2_340, [3, 0, 3], 100).unwrap();
        cancellation
            .ping_cooperation_at(
                2_340,
                &super::CooperationPingIntent {
                    operation_id: "cancel-ping".to_owned(),
                },
                200,
            )
            .unwrap();
        cancellation.move_player_at(2_341, [4, 0, 3], 300).unwrap();
        cancellation
            .revive_cooperation_at(
                2_340,
                &super::CooperationReviveIntent {
                    operation_id: "cancel-revive".to_owned(),
                },
                400,
            )
            .unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        cancellation
            .move_player_at(2_340, [0, 0, 0], 500)
            .expect("out-of-range anchor move should cancel before publication");
        let cancellation_messages = drain_messages(&runner_rx);
        let cancel_index = cancellation_messages
            .iter()
            .position(|message| {
                matches!(
                    message,
                    ServerMessage::CooperationReviveResult(result)
                        if result.status == super::CooperationReviveStatus::Cancelled
                )
            })
            .expect("peer should receive cancelled revive");
        let move_index = cancellation_messages
            .iter()
            .position(|message| {
                matches!(
                    message,
                    ServerMessage::ActorUpdate(update) if update.actor_id == 2_340
                )
            })
            .expect("peer should receive the candidate move after cancellation");
        assert!(cancel_index < move_index);
        assert_eq!(
            cancellation.cooperation.as_ref().unwrap().state.phase(),
            revenant_cooperation::Phase::RunnerDowned
        );
        cancellation.move_player_at(2_340, [3, 0, 3], 600).unwrap();
        cancellation
            .revive_cooperation_at(
                2_340,
                &super::CooperationReviveIntent {
                    operation_id: "replacement-revive".to_owned(),
                },
                700,
            )
            .unwrap();
        assert!(!cancellation
            .observe_cooperation_clock_at(2_699, None)
            .unwrap());
        assert_eq!(cancellation.actors.get(2_341).unwrap().health, 0);
        assert!(!cancellation
            .observe_cooperation_clock_at(2_700, None)
            .unwrap());
        assert_eq!(cancellation.actors.get(2_341).unwrap().health, 50);

        let (mut abandoned, _anchor_rx, runner_rx, calls) =
            active_cooperation_fixture(None, 2_350, 2_351);
        abandoned
            .disconnect_at(2_350, 100)
            .expect("disconnect should commit abandonment before removal");
        let summary = received_cooperation_summary(&runner_rx);
        assert_eq!(
            summary.outcome,
            super::WireCooperationTerminalOutcome::AbandonedDisconnect
        );
        assert_eq!(
            summary.subject_role,
            Some(super::WireCooperationRole::Anchor)
        );
        assert_eq!(
            summary.no_reward_reason,
            Some(super::CooperationNoRewardReason::ParticipantDisconnected)
        );
        assert_eq!(abandoned.participants.len(), 1);
        abandoned
            .disconnect_at(2_351, 101)
            .expect("later terminal disconnect should only reset");
        assert_eq!(
            calls
                .lock()
                .expect("fixture ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "cooperation_abandoned")
                .count(),
            1
        );
        assert_eq!(abandoned.stage, SessionStage::Waiting);
        assert!(abandoned.cooperation.is_none());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn cooperation_persistence_faults_publish_no_optimistic_truth() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (anchor, anchor_rx) =
            fixture_participant("local:start-fault-anchor", 2_400, &["pulse_rifle"]);
        let (runner, runner_rx) =
            fixture_participant("local:start-fault-runner", 2_401, &["pulse_rifle"]);
        let persistence =
            FixturePersistence::new(Some("cooperation_started"), Arc::clone(&calls), Vec::new());
        let mut start_fault =
            fixture_session(2, vec![anchor, runner], SessionStage::Waiting, persistence);
        for actor_id in [2_400, 2_401] {
            start_fault.request_cooperation_state(actor_id).unwrap();
        }
        start_and_clear_drone(&mut start_fault, 2_400);
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        assert!(start_fault
            .start_cooperation(
                2_400,
                &super::CooperationStartIntent {
                    operation_id: "start-fault".to_owned(),
                },
            )
            .is_err());
        assert!(start_fault.cooperation.is_none());
        assert_eq!(
            start_fault.route.as_ref().unwrap().operation.phase(),
            RoutePhase::ChoiceOpen
        );
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());

        let (mut ping_fault, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_pinged"), 2_410, 2_411);
        ping_fault.move_player_at(2_410, [3, 0, 3], 100).unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        assert!(ping_fault
            .ping_cooperation_at(
                2_410,
                &super::CooperationPingIntent {
                    operation_id: "ping-fault".to_owned(),
                },
                200,
            )
            .is_err());
        let ping_runtime = ping_fault.cooperation.as_ref().unwrap();
        assert_eq!(
            ping_runtime.state.phase(),
            revenant_cooperation::Phase::AwaitingPing
        );
        assert!(ping_runtime.ping.is_none());
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());

        let (mut down_fault, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_runner_downed"), 2_420, 2_421);
        down_fault.move_player_at(2_420, [3, 0, 3], 100).unwrap();
        down_fault
            .ping_cooperation_at(
                2_420,
                &super::CooperationPingIntent {
                    operation_id: "down-fault-ping".to_owned(),
                },
                200,
            )
            .unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        assert!(down_fault.move_player_at(2_421, [4, 0, 3], 300).is_err());
        assert_eq!(down_fault.actors.get(2_421).unwrap().position, [0, 0, 0]);
        assert_eq!(down_fault.actors.get(2_421).unwrap().health, 100);
        assert_eq!(
            down_fault.cooperation.as_ref().unwrap().state.phase(),
            revenant_cooperation::Phase::AwaitingRunner
        );
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());

        let (mut revive_fault, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_revive_started"), 2_430, 2_431);
        revive_fault.move_player_at(2_430, [3, 0, 3], 100).unwrap();
        revive_fault
            .ping_cooperation_at(
                2_430,
                &super::CooperationPingIntent {
                    operation_id: "revive-fault-ping".to_owned(),
                },
                200,
            )
            .unwrap();
        revive_fault.move_player_at(2_431, [4, 0, 3], 300).unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        assert!(revive_fault
            .revive_cooperation_at(
                2_430,
                &super::CooperationReviveIntent {
                    operation_id: "revive-fault".to_owned(),
                },
                400,
            )
            .is_err());
        assert_eq!(
            revive_fault.cooperation.as_ref().unwrap().state.phase(),
            revenant_cooperation::Phase::RunnerDowned
        );
        assert!(revive_fault.cooperation.as_ref().unwrap().revive.is_none());
        assert_eq!(revive_fault.actors.get(2_431).unwrap().health, 0);
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());

        let (mut observation_fault, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_revive_observed"), 2_440, 2_441);
        observation_fault
            .move_player_at(2_440, [3, 0, 3], 100)
            .unwrap();
        observation_fault
            .ping_cooperation_at(
                2_440,
                &super::CooperationPingIntent {
                    operation_id: "observation-fault-ping".to_owned(),
                },
                200,
            )
            .unwrap();
        observation_fault
            .move_player_at(2_441, [4, 0, 3], 300)
            .unwrap();
        observation_fault
            .revive_cooperation_at(
                2_440,
                &super::CooperationReviveIntent {
                    operation_id: "observation-fault".to_owned(),
                },
                400,
            )
            .unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        assert!(observation_fault
            .observe_cooperation_clock_at(2_400, None)
            .is_err());
        assert_eq!(
            observation_fault
                .cooperation
                .as_ref()
                .unwrap()
                .state
                .phase(),
            revenant_cooperation::Phase::ReviveChannel
        );
        assert_eq!(observation_fault.actors.get(2_441).unwrap().health, 0);
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());

        let (mut completion_fault, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_completed"), 2_450, 2_451);
        drive_fixture_cooperation_to_encounter(&mut completion_fault, 2_450, 2_451);
        completion_fault
            .move_player_at(2_450, [6, 0, 0], 2_500)
            .unwrap();
        let warden_id = completion_fault.enemy_id;
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        assert!(completion_fault
            .complete_cooperation_activity_at(3_000)
            .is_err());
        assert_eq!(completion_fault.stage, SessionStage::Boss);
        assert_eq!(completion_fault.enemy_id, warden_id);
        assert_eq!(
            completion_fault.cooperation.as_ref().unwrap().state.phase(),
            revenant_cooperation::Phase::EncounterActive
        );
        assert_eq!(completion_fault.participants[0].module_state.fragments, 0);
        assert_eq!(completion_fault.participants[1].module_state.fragments, 0);
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());

        let (mut timeout_fault, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_timed_out"), 2_460, 2_461);
        assert!(timeout_fault
            .observe_cooperation_clock_at(60_001, None)
            .is_err());
        assert_eq!(timeout_fault.stage, SessionStage::Door);
        assert_eq!(
            timeout_fault.cooperation.as_ref().unwrap().state.phase(),
            revenant_cooperation::Phase::AwaitingAnchor
        );
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());

        let (mut abandon_fault, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_abandoned"), 2_470, 2_471);
        assert!(abandon_fault.disconnect_at(2_470, 100).is_err());
        assert_eq!(abandon_fault.participants.len(), 2);
        assert_eq!(
            abandon_fault.cooperation.as_ref().unwrap().state.phase(),
            revenant_cooperation::Phase::AwaitingAnchor
        );
        assert!(drain_messages(&anchor_rx).is_empty());
        assert!(drain_messages(&runner_rx).is_empty());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn cooperation_participant_defeat_commits_before_life_and_summary() {
        let (mut defeated, anchor_rx, runner_rx, calls) =
            active_cooperation_fixture(None, 2_500, 2_501);
        drive_fixture_cooperation_to_encounter(&mut defeated, 2_500, 2_501);
        defeated.move_player_at(2_500, [6, 0, 0], 2_500).unwrap();
        defeated.cooperation.as_mut().unwrap().started_at = Instant::now()
            .checked_sub(Duration::from_secs(3))
            .expect("three-second fixture offset should be representable");
        let mut vulnerable = defeated.actors.get(2_500).unwrap().clone();
        vulnerable.health = 15;
        defeated.actors.insert(vulnerable);
        let warden_id = defeated.enemy_id.unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        defeated.attack_at(2_500, warden_id, 0).unwrap();
        defeated.attack_at(2_500, warden_id, 1_000).unwrap();
        assert_eq!(defeated.stage, SessionStage::Failed);
        assert_eq!(defeated.actors.get(2_500).unwrap().health, 0);
        let anchor_messages = drain_messages(&anchor_rx);
        let runner_messages = drain_messages(&runner_rx);
        let terminal_order = |messages: &[ServerMessage]| {
            let fatal = messages
                .iter()
                .position(|message| {
                    matches!(
                        message,
                        ServerMessage::DamageApplied(damage)
                            if damage.source_actor_id == warden_id && damage.killed
                    )
                })
                .expect("participant should receive fatal Warden damage");
            let life = messages
                .iter()
                .position(|message| {
                    matches!(
                        message,
                        ServerMessage::CooperationLifeState(state)
                            if state.life_after == super::WireCooperationLife::Defeated
                    )
                })
                .expect("participant should receive defeated life truth");
            let summary_index = messages
                .iter()
                .position(|message| {
                    matches!(message, ServerMessage::CooperationOperationSummary(_))
                })
                .expect("participant should receive failure summary");
            assert!(fatal < life && life < summary_index);
            assert!(messages.iter().all(|message| !matches!(
                message,
                ServerMessage::LootGranted(_)
                    | ServerMessage::ProgressionGranted(_)
                    | ServerMessage::ActivityComplete(_)
            )));
            let ServerMessage::CooperationOperationSummary(summary) = &messages[summary_index]
            else {
                unreachable!();
            };
            summary.clone()
        };
        let anchor_summary = terminal_order(&anchor_messages);
        let runner_summary = terminal_order(&runner_messages);
        assert_eq!(anchor_summary, runner_summary);
        assert_eq!(
            anchor_summary.outcome,
            super::WireCooperationTerminalOutcome::FailedParticipantDefeated
        );
        assert_eq!(
            anchor_summary.subject_role,
            Some(super::WireCooperationRole::Anchor)
        );
        assert_eq!(
            anchor_summary.no_reward_reason,
            Some(super::CooperationNoRewardReason::ParticipantDefeated)
        );
        assert_eq!(anchor_summary.participants[0].current_health, 0);
        assert_eq!(
            anchor_summary.participants[0].life,
            super::WireCooperationLife::Defeated
        );
        assert_eq!(
            calls
                .lock()
                .expect("fixture ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "cooperation_participant_defeated")
                .count(),
            1
        );

        let (mut failed_commit, anchor_rx, runner_rx, _) =
            active_cooperation_fixture(Some("cooperation_participant_defeated"), 2_510, 2_511);
        drive_fixture_cooperation_to_encounter(&mut failed_commit, 2_510, 2_511);
        failed_commit
            .move_player_at(2_510, [6, 0, 0], 2_500)
            .unwrap();
        failed_commit.cooperation.as_mut().unwrap().started_at = Instant::now()
            .checked_sub(Duration::from_secs(3))
            .expect("three-second fixture offset should be representable");
        let mut vulnerable = failed_commit.actors.get(2_510).unwrap().clone();
        vulnerable.health = 15;
        failed_commit.actors.insert(vulnerable);
        let warden_id = failed_commit.enemy_id.unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        failed_commit.attack_at(2_510, warden_id, 0).unwrap();
        drain_messages(&anchor_rx);
        drain_messages(&runner_rx);
        assert!(failed_commit.attack_at(2_510, warden_id, 1_000).is_err());
        assert_eq!(failed_commit.stage, SessionStage::Boss);
        assert_eq!(failed_commit.actors.get(2_510).unwrap().health, 15);
        assert_eq!(
            failed_commit.cooperation.as_ref().unwrap().state.phase(),
            revenant_cooperation::Phase::EncounterActive
        );
        for message in drain_messages(&anchor_rx)
            .into_iter()
            .chain(drain_messages(&runner_rx))
        {
            assert!(!matches!(
                message,
                ServerMessage::CooperationLifeState(_)
                    | ServerMessage::CooperationOperationSummary(_)
            ));
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn route_capability_leader_choice_retry_and_conflict_are_authoritative() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (leader, leader_rx) =
            fixture_participant("local:route-leader", 1_100, &["pulse_rifle"]);
        let (peer, peer_rx) = fixture_participant("local:route-peer", 1_101, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session =
            fixture_session(2, vec![leader, peer], SessionStage::Waiting, persistence);
        let generated = Arc::new(AtomicUsize::new(0));
        let generated_by_source = Arc::clone(&generated);
        session.route_seed_source = Box::new(move || {
            generated_by_source.fetch_add(1, AtomicOrdering::SeqCst);
            RouteSeed::new(7).map_err(|error| std::io::Error::other(error.to_string()))
        });

        session
            .request_route_state(1_100)
            .expect("leader capability should be accepted while waiting");
        let leader_state = drain_messages(&leader_rx)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::RouteState(state) => Some(state),
                _ => None,
            })
            .expect("leader should receive waiting route state");
        assert_eq!(leader_state.phase, super::WireRoutePhase::Waiting);
        assert_eq!(leader_state.capable_actor_ids, [1_100]);
        assert!(!leader_state.all_capable);
        assert_eq!(peer_rx.try_recv(), Err(TryRecvError::Empty));

        start_and_clear_drone(&mut session, 1_100);
        drain_messages(&leader_rx);
        drain_messages(&peer_rx);
        session
            .choose_route(
                1_100,
                &RouteChoiceIntent {
                    operation_id: "route-shared".to_owned(),
                    route_id: "breach".to_owned(),
                },
            )
            .expect("incomplete capability should be a targeted rejection");
        let ServerMessage::RouteChoiceResult(incomplete) = leader_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("leader should receive capability rejection")
        else {
            panic!("leader should receive RouteChoiceResult");
        };
        assert!(!incomplete.accepted);
        assert!(incomplete.selection.is_none());
        assert_eq!(generated.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(peer_rx.try_recv(), Err(TryRecvError::Empty));

        session
            .request_route_state(1_101)
            .expect("peer capability should complete opt-in");
        let peer_state = drain_messages(&peer_rx)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::RouteState(state) => Some(state),
                _ => None,
            })
            .expect("peer should receive shared route state");
        assert_eq!(peer_state.phase, super::WireRoutePhase::ChoiceOpen);
        assert_eq!(peer_state.capable_actor_ids, [1_100, 1_101]);
        assert!(peer_state.all_capable);
        drain_messages(&leader_rx);

        session
            .choose_route(
                1_101,
                &RouteChoiceIntent {
                    operation_id: "route-shared".to_owned(),
                    route_id: "breach".to_owned(),
                },
            )
            .expect("non-leader should be rejected without failing the session");
        let ServerMessage::RouteChoiceResult(non_leader) = peer_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("peer should receive leader rejection")
        else {
            panic!("peer should receive RouteChoiceResult");
        };
        assert!(!non_leader.accepted);
        assert_eq!(generated.load(AtomicOrdering::SeqCst), 0);

        for intent in [
            RouteChoiceIntent {
                operation_id: "x".repeat(33),
                route_id: "breach".to_owned(),
            },
            RouteChoiceIntent {
                operation_id: "route-shared".to_owned(),
                route_id: "third-route".to_owned(),
            },
        ] {
            session
                .choose_route(1_100, &intent)
                .expect("malformed route input should be a targeted rejection");
            let ServerMessage::RouteChoiceResult(rejected) = leader_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("leader should receive malformed-input rejection")
            else {
                panic!("leader should receive RouteChoiceResult");
            };
            assert!(!rejected.accepted);
            assert!(rejected.selection.is_none());
        }
        assert_eq!(generated.load(AtomicOrdering::SeqCst), 0);

        let accepted_intent = RouteChoiceIntent {
            operation_id: "route-shared".to_owned(),
            route_id: "breach".to_owned(),
        };
        session
            .choose_route(1_100, &accepted_intent)
            .expect("leader route should commit");
        let leader_messages = drain_messages(&leader_rx);
        let peer_messages = drain_messages(&peer_rx);
        let accepted_leader = leader_messages
            .iter()
            .find_map(|message| match message {
                ServerMessage::RouteChoiceResult(result) if result.accepted => Some(result),
                _ => None,
            })
            .expect("leader should receive accepted selection");
        let accepted_peer = peer_messages
            .iter()
            .find_map(|message| match message {
                ServerMessage::RouteChoiceResult(result) if result.accepted => Some(result),
                _ => None,
            })
            .expect("peer should receive identical accepted selection");
        assert_eq!(accepted_leader, accepted_peer);
        assert!(!accepted_leader.replayed);
        assert_eq!(generated.load(AtomicOrdering::SeqCst), 1);

        session
            .choose_route(1_100, &accepted_intent)
            .expect("same route operation should replay");
        let ServerMessage::RouteChoiceResult(retry) = leader_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("leader should receive route retry")
        else {
            panic!("leader should receive RouteChoiceResult");
        };
        assert!(retry.accepted && retry.replayed);
        assert_eq!(retry.selection, accepted_leader.selection);
        assert_eq!(generated.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(peer_rx.try_recv(), Err(TryRecvError::Empty));

        session
            .choose_route(
                1_100,
                &RouteChoiceIntent {
                    operation_id: "route-shared".to_owned(),
                    route_id: "stabilize".to_owned(),
                },
            )
            .expect("conflict should remain a targeted rejection");
        let ServerMessage::RouteChoiceResult(conflict) = leader_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("leader should receive route conflict")
        else {
            panic!("leader should receive RouteChoiceResult");
        };
        assert!(!conflict.accepted && conflict.selection.is_none());
        assert_eq!(generated.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(
            calls
                .lock()
                .expect("fixture call ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "route_selected")
                .count(),
            2
        );
    }

    #[test]
    fn ordinary_v2_baseline_receives_no_route_or_cooperation_variant() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:ordinary-v2", 1_110, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);

        start_and_clear_drone(&mut session, 1_110);
        session
            .move_player(1_110, [6, 0, 0])
            .expect("ordinary V2 should retain the baseline door path");
        assert_eq!(
            session
                .route
                .as_ref()
                .expect("baseline should retain inert route state")
                .operation
                .phase(),
            RoutePhase::BaselineLocked
        );
        defeat_active_warden(&mut session, 1_110);
        let messages = drain_messages(&receiver);
        assert!(messages.iter().all(|message| !matches!(
            message,
            ServerMessage::RouteState(_)
                | ServerMessage::RouteChoiceResult(_)
                | ServerMessage::RouteOperationSummary(_)
                | ServerMessage::CooperationState(_)
                | ServerMessage::CooperationStartResult(_)
                | ServerMessage::CooperationPingResult(_)
                | ServerMessage::CooperationLifeState(_)
                | ServerMessage::CooperationReviveResult(_)
                | ServerMessage::CooperationOperationSummary(_)
        )));
        assert!(calls
            .lock()
            .expect("fixture call ledger should lock")
            .iter()
            .all(|call| !call.starts_with("route_")));
    }

    #[test]
    fn signal_excursion_requires_cover_flanking_and_recovery_before_the_warden() {
        use revenant_ai::sentinel::{SHOT_INTERVAL_MS, SIGNAL_APPROACH, SIGNAL_TERMINAL};
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) = fixture_participant("local:signal", 1_170, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, calls, Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
        start_and_clear_drone(&mut session, 1_170);
        session.move_player(1_170, SIGNAL_APPROACH).unwrap();
        let sentinel = session.enemy_id.unwrap();
        assert_eq!(
            session.actors.get(sentinel).unwrap().archetype,
            "signal-sentinel"
        );
        drain_messages(&receiver);
        session.tick_signal_at(SHOT_INTERVAL_MS);
        assert_eq!(session.actors.get(1_170).unwrap().health, 90);
        assert!(session.attack_at(1_170, sentinel, 0).is_err());
        session.move_player(1_170, [-4, 0, 0]).unwrap();
        assert_eq!(session.actors.get(1_170).unwrap().position, SIGNAL_APPROACH);
        session.move_player(1_170, [-7, 0, 4]).unwrap();
        session.move_player(1_170, [-7, 0, 0]).unwrap();
        session.tick_signal_at(2 * SHOT_INTERVAL_MS);
        assert_eq!(session.actors.get(1_170).unwrap().health, 78);
        session.move_player(1_170, SIGNAL_TERMINAL).unwrap();
        session.move_player(1_170, [6, 0, 0]).unwrap();
        assert_eq!(session.enemy_id, Some(sentinel));
        session.move_player(1_170, [-7, 0, 0]).unwrap();
        for elapsed in [0, 250, 500, 750, 1000] {
            session.attack_at(1_170, sentinel, elapsed).unwrap();
        }
        assert_eq!(session.enemy_id, None);
        assert_eq!(session.stage, SessionStage::Door);
        let messages = drain_messages(&receiver);
        assert!(!messages.iter().any(|message| matches!(
            message,
            ServerMessage::LootGranted(_) | ServerMessage::ActivityComplete(_)
        )));
        assert!(messages.iter().any(|message| matches!(message, ServerMessage::ObjectiveUpdate(o) if o.objective_id == "recover_lost_signal" && o.progress == 1 && o.state == "Active")));
        session.move_player(1_170, SIGNAL_TERMINAL).unwrap();
        let messages = drain_messages(&receiver);
        assert!(messages.iter().any(|message| matches!(message, ServerMessage::ObjectiveUpdate(o) if o.objective_id == "recover_lost_signal" && o.state == "Completed")));
        session.move_player(1_170, [6, 0, 0]).unwrap();
        assert_eq!(session.stage, SessionStage::Boss);
        defeat_active_warden(&mut session, 1_170);
        assert_eq!(session.stage, SessionStage::Complete);
        session.disconnect(1_170).unwrap();
        assert!(session.signal.is_none());
    }

    #[test]
    fn meridian_preserves_old_clients_and_optional_skip_to_the_warden() {
        for capable in [false, true] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let (mut player, receiver) =
                fixture_participant("local:meridian-skip", 1_191, &["pulse_rifle"]);
            player.exploration_capable = capable;
            let persistence = FixturePersistence::new(None, calls.clone(), Vec::new());
            let mut session = fixture_session(1, vec![player], SessionStage::Waiting, persistence);
            start_and_clear_drone(&mut session, 1_191);
            session.move_player(1_191, [-12, 0, 0]).unwrap();
            assert_eq!(session.meridian.is_some(), capable);
            session.move_player(1_191, [-13, 0, 0]).unwrap();
            assert_eq!(
                session.actors.get(1_191).unwrap().position[0],
                if capable { -13 } else { -12 }
            );
            session.move_player(1_191, [-12, 0, 0]).unwrap();
            // Both activities can be skipped without blocking the main mission.
            session.move_player(1_191, [6, 0, 0]).unwrap();
            assert_eq!(session.stage, SessionStage::Boss);
            drain_messages(&receiver);
            defeat_active_warden(&mut session, 1_191);
            assert_eq!(session.stage, SessionStage::Complete);
            let messages = drain_messages(&receiver);
            assert_eq!(
                messages
                    .iter()
                    .filter(|m| matches!(m, ServerMessage::ActivityComplete(_)))
                    .count(),
                1
            );
            assert_eq!(
                calls
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|c| *c == "completion_transaction")
                    .count(),
                1
            );
            session.disconnect(1_191).unwrap();
            assert!(session.meridian.is_none());
        }
    }

    #[test]
    fn meridian_persistence_failure_withholds_position_and_objectives_then_allows_retry() {
        for starting in [true, false] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let (mut player, receiver) =
                fixture_participant("local:meridian-fault", 1_192, &["pulse_rifle"]);
            player.exploration_capable = true;
            let persistence = FixturePersistence::new(Some("field_activity"), calls, Vec::new());
            let mut session = fixture_session(1, vec![player], SessionStage::Waiting, persistence);
            start_and_clear_drone(&mut session, 1_192);
            let (before, target) = if starting {
                ([-11, 0, 0], [-12, 0, 0])
            } else {
                session.meridian =
                    Some(revenant_activities::meridian::Expedition::start().unwrap());
                session
                    .route
                    .as_mut()
                    .unwrap()
                    .operation
                    .lock_baseline()
                    .unwrap();
                ([-15, 0, 0], [-16, 0, 0])
            };
            session.actors.update_position(1_192, before);
            drain_messages(&receiver);
            assert!(session.move_player(1_192, target).is_err());
            assert_eq!(session.actors.get(1_192).unwrap().position, before);
            assert!(drain_messages(&receiver).is_empty());
            session.move_player(1_192, target).unwrap();
            assert_eq!(session.actors.get(1_192).unwrap().position, target);
            assert!(!drain_messages(&receiver).is_empty());
            session.move_player(1_192, target).unwrap();
            assert!(!drain_messages(&receiver)
                .iter()
                .any(|m| matches!(m, ServerMessage::ObjectiveUpdate(_))));
            // Neither a teleport across the annex nor an off-floor step is accepted.
            session.move_player(1_192, [-28, 0, -7]).unwrap();
            assert_eq!(session.actors.get(1_192).unwrap().position, target);
        }
    }

    #[test]
    fn coolant_expiration_retry_and_delivery_preserve_the_main_reward_boundary() {
        use revenant_activities::coolant::{DEADLINE_MS, DELIVERY, DWELL_MS, INTAKE, TRANSFER};
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (mut player, receiver) = fixture_participant("local:coolant", 1_190, &["pulse_rifle"]);
        player.content_capable = true;
        let persistence = FixturePersistence::new(None, calls, Vec::new());
        let mut session = fixture_session(1, vec![player], SessionStage::Waiting, persistence);
        start_and_clear_drone(&mut session, 1_190);
        session.move_player(1_190, INTAKE).unwrap();
        session.move_player(1_190, [6, 0, 0]).unwrap();
        assert_eq!(session.stage, SessionStage::Door);
        session.tick_coolant_at(DEADLINE_MS).unwrap();
        assert!(drain_messages(&receiver).iter().any(|message| matches!(message,
            ServerMessage::ObjectiveUpdate(objective) if objective.objective_id == "coolant_transfer" && objective.state == "Failed")));
        session.move_player(1_190, INTAKE).unwrap();
        session.actors.update_position(1_190, TRANSFER);
        session.tick_coolant_at(0).unwrap();
        session.tick_coolant_at(DWELL_MS).unwrap();
        session.actors.update_position(1_190, DELIVERY);
        session.tick_coolant_at(DWELL_MS + 1).unwrap();
        session.tick_coolant_at(2 * DWELL_MS + 1).unwrap();
        let messages = drain_messages(&receiver);
        assert!(messages.iter().any(|message| matches!(message,
            ServerMessage::ObjectiveUpdate(objective) if objective.objective_id == "coolant_delivery" && objective.state == "Completed")));
        assert!(!messages.iter().any(|message| matches!(
            message,
            ServerMessage::LootGranted(_) | ServerMessage::ActivityComplete(_)
        )));
        session.move_player(1_190, [6, 0, 0]).unwrap();
        assert_eq!(session.stage, SessionStage::Boss);
        defeat_active_warden(&mut session, 1_190);
        assert_eq!(session.stage, SessionStage::Complete);
        session.disconnect(1_190).unwrap();
        assert!(session.coolant.is_none());
    }

    #[test]
    fn signal_excursion_keeps_v1_unchanged_and_death_grants_nothing() {
        use revenant_ai::sentinel::{SHOT_INTERVAL_MS, SIGNAL_APPROACH};
        for frozen in [true, false] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let (mut participant, receiver) =
                fixture_participant("local:signal-limit", 1_171, &["pulse_rifle"]);
            if frozen {
                participant.protocol_generation = ProtocolGeneration::FrozenV1;
            }
            let persistence = FixturePersistence::new(None, calls, Vec::new());
            let mut session =
                fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
            start_and_clear_drone(&mut session, 1_171);
            session.move_player(1_171, SIGNAL_APPROACH).unwrap();
            if frozen {
                assert!(session.signal.is_none());
                continue;
            }
            session.move_player(1_171, [-7, 0, 4]).unwrap();
            session.move_player(1_171, [-7, 0, 0]).unwrap();
            session.actors.apply_damage(1_171, 89);
            drain_messages(&receiver);
            session.tick_signal_at(SHOT_INTERVAL_MS);
            assert_eq!(session.stage, SessionStage::Failed);
            let messages = drain_messages(&receiver);
            assert!(messages.iter().any(|message| matches!(message, ServerMessage::ObjectiveUpdate(o) if o.objective_id == "recover_lost_signal" && o.state == "Failed")));
            assert!(!messages.iter().any(|message| matches!(
                message,
                ServerMessage::LootGranted(_) | ServerMessage::ActivityComplete(_)
            )));
        }
    }

    #[test]
    fn arc_warden_movement_is_shared_and_requires_a_nonlethal_accepted_hit() {
        for event_id in [RouteEventId::ArcSurge, RouteEventId::OverchargedArmor] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let (leader, leader_rx) =
                fixture_participant("local:arc-leader", 1_150, &["pulse_rifle"]);
            let (peer, peer_rx) = fixture_participant("local:arc-peer", 1_151, &["pulse_rifle"]);
            let persistence = FixturePersistence::new(None, calls, Vec::new());
            let mut session =
                fixture_session(2, vec![leader, peer], SessionStage::Waiting, persistence);
            for player_id in [1_150, 1_151] {
                session.request_route_state(player_id).unwrap();
            }
            start_and_clear_drone(&mut session, 1_150);
            set_fixture_seed(&mut session, seed_for_event(RouteId::Breach, event_id));
            session
                .choose_route(
                    1_150,
                    &RouteChoiceIntent {
                        operation_id: "arc-encounter".to_owned(),
                        route_id: "breach".to_owned(),
                    },
                )
                .unwrap();
            drive_route_to_warden(&mut session, 1_150, RouteId::Breach);
            let enemy_id = session.enemy_id.unwrap();
            drain_messages(&leader_rx);
            drain_messages(&peer_rx);

            session.attack_at(1_150, enemy_id, 0).unwrap();
            let first_position = session.actors.get(enemy_id).unwrap().position;
            assert_eq!(
                first_position,
                if event_id == RouteEventId::ArcSurge {
                    [8, 0, -3]
                } else {
                    [8, 0, 0]
                }
            );
            let leader_events = drain_messages(&leader_rx);
            assert_eq!(leader_events, drain_messages(&peer_rx));
            assert!(matches!(
                leader_events.first(),
                Some(ServerMessage::DamageApplied(_))
            ));
            assert_eq!(leader_events.iter().any(|message| matches!(message, ServerMessage::ActorUpdate(update) if update.actor_id == enemy_id)), event_id == RouteEventId::ArcSurge);

            // Neither a cooldown rejection nor an out-of-range shot advances the pattern.
            assert!(session.attack_at(1_150, enemy_id, 0).is_err());
            session.move_player(1_150, [-12, 0, -12]).unwrap();
            assert!(session.attack_at(1_150, enemy_id, 250).is_err());
            assert_eq!(
                session.actors.get(enemy_id).unwrap().position,
                first_position
            );
            session.move_player(1_150, [6, 0, 0]).unwrap();
            session.attack_at(1_150, enemy_id, 250).unwrap();
            assert_eq!(
                session.actors.get(enemy_id).unwrap().position,
                if event_id == RouteEventId::ArcSurge {
                    [10, 0, 3]
                } else {
                    [8, 0, 0]
                }
            );

            let mut elapsed = 500;
            while session.actors.get(enemy_id).unwrap().health > 40 {
                session.attack_at(1_150, enemy_id, elapsed).unwrap();
                elapsed += 250;
            }
            drain_messages(&leader_rx);
            session.attack_at(1_150, enemy_id, elapsed).unwrap();
            assert_eq!(session.stage, SessionStage::Complete);
            let final_events = drain_messages(&leader_rx);
            assert!(!final_events.iter().any(|message| matches!(message, ServerMessage::ActorUpdate(update) if update.actor_id == enemy_id)));
            assert!(final_events
                .iter()
                .any(|message| matches!(message, ServerMessage::ActivityComplete(_))));
        }
    }

    #[test]
    fn both_routes_and_all_events_complete_with_exact_shared_wire_truth() {
        for route_id in RouteId::ALL {
            for event_id in route_definition(route_id).events {
                let calls = Arc::new(Mutex::new(Vec::new()));
                let account = format!("local:{route_id}-{event_id}");
                let (participant, receiver) =
                    fixture_participant(&account, 1_120, &["pulse_rifle"]);
                let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
                let mut session =
                    fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
                session
                    .request_route_state(1_120)
                    .expect("solo route capability should be accepted");
                start_and_clear_drone(&mut session, 1_120);
                drain_messages(&receiver);
                set_fixture_seed(&mut session, seed_for_event(route_id, event_id));
                session
                    .choose_route(
                        1_120,
                        &RouteChoiceIntent {
                            operation_id: "route-event-fixture".to_owned(),
                            route_id: route_id.to_string(),
                        },
                    )
                    .expect("fixture route should be selected");
                let selection = drain_messages(&receiver)
                    .into_iter()
                    .find_map(|message| match message {
                        ServerMessage::RouteChoiceResult(result) if result.accepted => {
                            result.selection
                        }
                        _ => None,
                    })
                    .expect("route selection should be broadcast");
                assert_eq!(selection.event_id, event_id.to_string());
                assert_eq!(selection.route_id, route_id.to_string());

                drive_route_to_warden(&mut session, 1_120, route_id);
                let warden = session
                    .actors
                    .get(session.enemy_id.expect("Warden should be active"))
                    .expect("Warden actor should exist");
                assert_eq!(
                    warden.max_health,
                    apply_warden_health_effect(
                        240,
                        resolve_event(route_id, seed_for_event(route_id, event_id)).effect,
                    )
                    .expect("event health effect should apply")
                );
                defeat_active_warden(&mut session, 1_120);
                let messages = drain_messages(&receiver);
                let summary = messages
                    .iter()
                    .find_map(|message| match message {
                        ServerMessage::RouteOperationSummary(summary) => Some(summary),
                        _ => None,
                    })
                    .expect("successful route should emit its terminal summary");
                let definition = route_definition(route_id);
                assert_eq!(summary.outcome, RouteTerminalOutcome::Succeeded);
                assert_eq!(summary.event_id, event_id.to_string());
                assert_eq!(summary.objective_path, selection.objective_path);
                assert_eq!(
                    summary.transitions.len(),
                    definition.objective_path.len() * 2 - 1
                );
                assert_eq!(summary.grants.len(), 1);
                assert_eq!(
                    summary
                        .reward
                        .as_ref()
                        .map(|reward| (reward.item_quantity, reward.experience,)),
                    Some((definition.reward.fragments, definition.reward.experience))
                );
                assert!(summary.no_reward_reason.is_none());
                let completion_index = messages
                    .iter()
                    .position(|message| matches!(message, ServerMessage::ActivityComplete(_)))
                    .expect("generic activity completion should remain present");
                let summary_index = messages
                    .iter()
                    .position(|message| matches!(message, ServerMessage::RouteOperationSummary(_)))
                    .expect("route summary should remain present");
                assert!(completion_index < summary_index);
                assert!(calls
                    .lock()
                    .expect("fixture call ledger should lock")
                    .iter()
                    .any(|call| call == "route_completed"));
            }
        }
    }

    #[test]
    fn route_deadline_boundary_timeout_and_reset_are_monotonic() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:route-timeout", 1_130, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
        session
            .request_route_state(1_130)
            .expect("timeout fixture should opt in");
        start_and_clear_drone(&mut session, 1_130);
        drain_messages(&receiver);
        set_fixture_seed(
            &mut session,
            RouteSeed::new(3).expect("fixture seed should validate"),
        );
        session
            .choose_route(
                1_130,
                &RouteChoiceIntent {
                    operation_id: "route-timeout".to_owned(),
                    route_id: "stabilize".to_owned(),
                },
            )
            .expect("timeout fixture route should select");
        drain_messages(&receiver);

        assert!(!session
            .observe_route_deadline_at(90_000)
            .expect("deadline boundary should remain active"));
        assert_eq!(session.stage, SessionStage::Stabilizer);
        assert!(session
            .observe_route_deadline_at(90_001)
            .expect("first observation after deadline should fail"));
        assert_eq!(session.stage, SessionStage::Failed);
        let messages = drain_messages(&receiver);
        let failed_objective = messages
            .iter()
            .find_map(|message| match message {
                ServerMessage::ObjectiveUpdate(objective) if objective.state == "Failed" => {
                    Some(objective)
                }
                _ => None,
            })
            .expect("timeout should emit one authoritative failed objective");
        assert_eq!(failed_objective.objective_id, "reach_relay_stabilizer");
        let summary = messages
            .iter()
            .find_map(|message| match message {
                ServerMessage::RouteOperationSummary(summary) => Some(summary),
                _ => None,
            })
            .expect("timeout should emit a route summary");
        assert_eq!(summary.outcome, RouteTerminalOutcome::FailedTimeout);
        assert_eq!(summary.elapsed_ms, 90_001);
        assert!(summary.reward.is_none() && summary.grants.is_empty());
        assert_eq!(
            summary.no_reward_reason.as_deref(),
            Some("deadline_exceeded")
        );
        assert!(session.equip_weapon(1_130, "pulse_rifle").is_ok());
        let ServerMessage::EquipmentChanged(rejected) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("post-timeout equipment should return a rejection")
        else {
            panic!("post-timeout equipment should return EquipmentChanged");
        };
        assert!(!rejected.accepted);

        session
            .disconnect(1_130)
            .expect("last disconnect should reset failed route state");
        assert_eq!(session.stage, SessionStage::Waiting);
        assert!(session.route.is_none());
        assert_eq!(
            calls
                .lock()
                .expect("fixture call ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "route_failed")
                .count(),
            1
        );
    }

    #[test]
    fn route_success_at_exact_deadline_is_committed_with_rewards() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:route-boundary", 1_131, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
        session
            .request_route_state(1_131)
            .expect("boundary fixture should opt in");
        start_and_clear_drone(&mut session, 1_131);
        drain_messages(&receiver);
        session
            .choose_route(
                1_131,
                &RouteChoiceIntent {
                    operation_id: "route-boundary".to_owned(),
                    route_id: "breach".to_owned(),
                },
            )
            .expect("boundary fixture should select a route");
        drain_messages(&receiver);
        drive_route_to_warden(&mut session, 1_131, RouteId::Breach);
        record_successful_warden_evidence(&mut session);

        let (reward_messages, summary) = session
            .complete_routed_activity_at(90_000)
            .expect("the exact deadline must still succeed");
        let Some(ServerMessage::RouteOperationSummary(summary)) = summary else {
            panic!("deadline completion should return a route summary");
        };
        assert_eq!(session.stage, SessionStage::Complete);
        assert_eq!(summary.outcome, RouteTerminalOutcome::Succeeded);
        assert_eq!(summary.elapsed_ms, 90_000);
        assert!(summary.reward.is_some() && summary.grants.len() == 1);
        assert_eq!(reward_messages.len(), 2);
        assert!(!session
            .observe_route_deadline_at(90_001)
            .expect("completed route must not fail later"));
        assert!(calls
            .lock()
            .expect("fixture call ledger should lock")
            .iter()
            .any(|call| call == "route_completed"));
    }

    #[test]
    fn route_timeout_persistence_failure_exposes_no_terminal_projection() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:route-timeout-failure", 1_132, &["pulse_rifle"]);
        let persistence =
            FixturePersistence::new(Some("route_failed"), Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
        session
            .request_route_state(1_132)
            .expect("timeout failure fixture should opt in");
        start_and_clear_drone(&mut session, 1_132);
        drain_messages(&receiver);
        session
            .choose_route(
                1_132,
                &RouteChoiceIntent {
                    operation_id: "route-timeout-failure".to_owned(),
                    route_id: "stabilize".to_owned(),
                },
            )
            .expect("timeout failure fixture should select a route");
        drain_messages(&receiver);

        assert!(session.observe_route_deadline_at(90_001).is_err());
        assert_eq!(session.stage, SessionStage::Stabilizer);
        assert_eq!(
            session
                .route
                .as_ref()
                .expect("route state should remain")
                .operation
                .phase(),
            RoutePhase::Routed
        );
        assert!(drain_messages(&receiver).iter().all(|message| !matches!(
            message,
            ServerMessage::ObjectiveUpdate(objective) if objective.state == "Failed"
        ) && !matches!(
            message,
            ServerMessage::RouteOperationSummary(_)
        )));
        assert_eq!(
            calls
                .lock()
                .expect("fixture call ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "route_failed")
                .count(),
            1
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn routed_multiplayer_terminal_truth_and_rewards_are_shared() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (leader, leader_rx) =
            fixture_participant("local:route-shared-leader", 1_133, &["pulse_rifle"]);
        let (peer, peer_rx) =
            fixture_participant("local:route-shared-peer", 1_134, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session =
            fixture_session(2, vec![leader, peer], SessionStage::Waiting, persistence);
        for actor_id in [1_133, 1_134] {
            session
                .request_route_state(actor_id)
                .expect("each multiplayer participant should opt in");
        }
        start_and_clear_drone(&mut session, 1_133);
        drain_messages(&leader_rx);
        drain_messages(&peer_rx);
        session
            .choose_route(
                1_133,
                &RouteChoiceIntent {
                    operation_id: "route-shared-terminal".to_owned(),
                    route_id: "breach".to_owned(),
                },
            )
            .expect("leader should select the shared route");
        drain_messages(&leader_rx);
        drain_messages(&peer_rx);
        drive_route_to_warden(&mut session, 1_133, RouteId::Breach);
        defeat_active_warden(&mut session, 1_133);

        let leader_messages = drain_messages(&leader_rx);
        let peer_messages = drain_messages(&peer_rx);
        let route_summary = |messages: &[ServerMessage]| {
            messages.iter().find_map(|message| match message {
                ServerMessage::RouteOperationSummary(summary) => Some(summary.clone()),
                _ => None,
            })
        };
        let leader_summary = route_summary(&leader_messages)
            .expect("leader should receive the terminal route summary");
        let peer_summary =
            route_summary(&peer_messages).expect("peer should receive the terminal route summary");
        assert_eq!(leader_summary, peer_summary);
        assert_eq!(leader_summary.participant_actor_ids, [1_133, 1_134]);
        assert_eq!(leader_summary.grants.len(), 2);
        assert!(leader_messages
            .iter()
            .any(|message| matches!(message, ServerMessage::LootGranted(_))));
        assert!(peer_messages
            .iter()
            .any(|message| matches!(message, ServerMessage::LootGranted(_))));
        assert!(leader_messages
            .iter()
            .any(|message| matches!(message, ServerMessage::ProgressionGranted(_))));
        assert!(peer_messages
            .iter()
            .any(|message| matches!(message, ServerMessage::ProgressionGranted(_))));
        assert_eq!(
            calls
                .lock()
                .expect("fixture call ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "route_completed")
                .count(),
            1
        );
    }

    #[test]
    fn route_selection_and_terminal_persistence_failures_expose_no_success() {
        for failure in ["route_selected", "route_completed"] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let (participant, receiver) =
                fixture_participant("local:route-failure", 1_140, &["pulse_rifle"]);
            let persistence =
                FixturePersistence::new(Some(failure), Arc::clone(&calls), Vec::new());
            let mut session =
                fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
            session
                .request_route_state(1_140)
                .expect("failure fixture should opt in");
            start_and_clear_drone(&mut session, 1_140);
            drain_messages(&receiver);
            set_fixture_seed(
                &mut session,
                RouteSeed::new(5).expect("fixture seed should validate"),
            );
            let choice = session.choose_route(
                1_140,
                &RouteChoiceIntent {
                    operation_id: "route-failure".to_owned(),
                    route_id: "breach".to_owned(),
                },
            );
            if failure == "route_selected" {
                assert!(choice.is_err());
                assert_eq!(
                    session
                        .route
                        .as_ref()
                        .expect("route state should remain")
                        .operation
                        .phase(),
                    RoutePhase::ChoiceOpen
                );
                assert!(drain_messages(&receiver).iter().all(|message| !matches!(
                    message,
                    ServerMessage::RouteChoiceResult(result) if result.accepted
                )));
                continue;
            }

            choice.expect("selection should precede terminal failure");
            drain_messages(&receiver);
            drive_route_to_warden(&mut session, 1_140, RouteId::Breach);
            record_successful_warden_evidence(&mut session);
            assert!(session.complete_routed_activity_at(90_000).is_err());
            assert_eq!(session.stage, SessionStage::Boss);
            assert_eq!(
                session
                    .route
                    .as_ref()
                    .expect("route state should remain")
                    .operation
                    .phase(),
                RoutePhase::Routed
            );
            assert!(drain_messages(&receiver).iter().all(|message| !matches!(
                message,
                ServerMessage::LootGranted(_)
                    | ServerMessage::ProgressionGranted(_)
                    | ServerMessage::ActivityComplete(_)
                    | ServerMessage::RouteOperationSummary(_)
            )));
        }
    }

    #[test]
    fn replay_fixture_rejects_one_named_event_without_fallback() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut fixture =
            FixturePersistence::new(Some("activity_completed"), Arc::clone(&calls), Vec::new());
        let event = NewReplayEvent {
            event_type: "activity_completed",
            session_id: "session-fixture",
            account_id: "local:fixture",
            activity_id: Some("relay_awakening"),
            actor_id: None,
            payload: "fixture",
        };
        assert!(fixture.append_replay_event(&event).is_err());
        assert!(fixture.append_replay_event(&event).is_ok());
        assert_eq!(
            *calls.lock().expect("fixture call ledger should lock"),
            ["activity_completed", "activity_completed"]
        );
    }

    #[test]
    fn discarded_connection_is_renewed_only_on_the_next_operation() {
        let connect_count = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let connector_count = Arc::clone(&connect_count);
        let connector_calls = Arc::clone(&calls);
        let connector: SessionPersistenceConnector = Box::new(move || {
            let attempt = connector_count.fetch_add(1, AtomicOrdering::SeqCst);
            let failure = (attempt == 0).then_some("equipment_changed");
            let connection: SessionPersistenceConnection = Box::new(FixturePersistence::new(
                failure,
                Arc::clone(&connector_calls),
                Vec::new(),
            ));
            Ok(connection)
        });
        let mut persistence = RecoveringSessionPersistence::with_connector(connector)
            .expect("initial fixture connection should open");
        let event = NewReplayEvent {
            event_type: "equipment_changed",
            session_id: "session-fixture",
            account_id: "local:fixture",
            activity_id: Some("relay_awakening"),
            actor_id: Some(1),
            payload: "weapon equipped: arc_sidearm",
        };
        assert!(persistence
            .equip_weapon_with_replay("character", "arc_sidearm", &event)
            .is_err());
        assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 1);
        assert!(persistence
            .equip_weapon_with_replay("character", "arc_sidearm", &event)
            .expect("next operation should use a replacement connection"));
        assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 2);
        assert_eq!(
            *calls.lock().expect("fixture call ledger should lock"),
            ["equipment_changed", "equipment_changed"]
        );
    }

    #[test]
    fn activity_start_and_enemy_spawn_failures_withhold_all_projection() {
        for (failure, expected_calls) in [
            ("activity_started", &["activity_started"][..]),
            ("enemy_spawned", &["activity_started", "enemy_spawned"][..]),
        ] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let (participant, receiver) =
                fixture_participant("local:start-fixture", 900, &["pulse_rifle"]);
            let persistence =
                FixturePersistence::new(Some(failure), Arc::clone(&calls), Vec::new());
            let mut session =
                fixture_session(1, vec![participant], SessionStage::Waiting, persistence);

            assert!(session.start_activity().is_err());
            assert_eq!(session.stage, SessionStage::Waiting);
            assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
            assert_eq!(
                calls
                    .lock()
                    .expect("fixture call ledger should lock")
                    .as_slice(),
                expected_calls
            );
        }
    }

    #[test]
    fn boss_spawn_failure_withholds_door_objectives_and_actor_projection() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:boss-fixture", 910, &["pulse_rifle"]);
        let persistence =
            FixturePersistence::new(Some("boss_spawned"), Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Door, persistence);

        assert!(session.move_player(910, [6, 0, 0]).is_err());
        assert_eq!(session.stage, SessionStage::Door);
        assert_eq!(session.enemy_id, None);
        assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(
            *calls.lock().expect("fixture call ledger should lock"),
            ["boss_spawned"]
        );
    }

    #[test]
    fn enemy_death_fixture_withholds_projection_when_replay_fails() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:enemy-fixture", 920, &["pulse_rifle"]);
        let enemy = Actor {
            id: 921,
            kind: ActorKind::Enemy,
            archetype: "relay-drone".to_owned(),
            position: [4, 0, 2],
            health: 0,
            max_health: 100,
        };
        let persistence =
            FixturePersistence::new(Some("enemy_died"), Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Drone, persistence);
        session.actors.insert(enemy.clone());
        session.enemy_id = Some(enemy.id);
        assert!(session
            .enemy_defeated(enemy.id, fatal_damage(920, enemy.id))
            .is_err());
        assert!(session.actors.get(enemy.id).is_some());
        assert_eq!(session.stage, SessionStage::Drone);
        assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn equipment_failure_changes_neither_memory_nor_projection() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) = fixture_participant(
            "local:equipment-fixture",
            930,
            &["pulse_rifle", "arc_sidearm"],
        );
        let persistence =
            FixturePersistence::new(Some("equipment_changed"), Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Drone, persistence);

        assert!(session.equip_weapon(930, "arc_sidearm").is_err());
        assert_eq!(
            session.participants[0].equipped_weapon_item_id,
            "pulse_rifle"
        );
        assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn completion_failure_preserves_boss_state_and_withholds_projection() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:completion-fixture", 940, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(
            Some("completion_transaction"),
            Arc::clone(&calls),
            Vec::new(),
        );
        let mut session = fixture_session(1, vec![participant], SessionStage::Boss, persistence);

        assert!(session
            .complete_activity("local:completion-fixture")
            .is_err());
        assert_eq!(session.stage, SessionStage::Boss);
        assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn successful_multiplayer_completion_preserves_message_order_and_rewards() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant_one, receiver_one) =
            fixture_participant("local:reward-one", 950, &["pulse_rifle"]);
        let (participant_two, receiver_two) =
            fixture_participant("local:reward-two", 951, &["pulse_rifle"]);
        let rewards = vec![
            Some(CompletionRewards {
                item_quantity: 1,
                experience_granted: 100,
                previous_level: 1,
                level: 2,
                experience: 100,
            }),
            Some(CompletionRewards {
                item_quantity: 4,
                experience_granted: 100,
                previous_level: 2,
                level: 3,
                experience: 200,
            }),
        ];
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), rewards);
        let mut session = fixture_session(
            2,
            vec![participant_one, participant_two],
            SessionStage::Boss,
            persistence,
        );
        let boss = Actor {
            id: 952,
            kind: ActorKind::Enemy,
            archetype: "warden".to_owned(),
            position: [8, 0, 0],
            health: 0,
            max_health: 120,
        };
        session.actors.insert(boss.clone());
        session.enemy_id = Some(boss.id);

        session
            .enemy_defeated(boss.id, fatal_damage(950, boss.id))
            .expect("atomic completion should succeed");
        assert_eq!(session.stage, SessionStage::Complete);
        let messages_one = receiver_one.try_iter().collect::<Vec<_>>();
        let messages_two = receiver_two.try_iter().collect::<Vec<_>>();
        let expected_order = [
            "DamageApplied",
            "ActorDestroy",
            "ObjectiveUpdate",
            "LootGranted",
            "ProgressionGranted",
            "ActivityComplete",
        ];
        assert_eq!(
            messages_one.iter().map(message_kind).collect::<Vec<_>>(),
            expected_order
        );
        assert_eq!(
            messages_two.iter().map(message_kind).collect::<Vec<_>>(),
            expected_order
        );
        let ServerMessage::LootGranted(loot_one) = &messages_one[3] else {
            panic!("participant one should receive loot");
        };
        let ServerMessage::ProgressionGranted(progression_one) = &messages_one[4] else {
            panic!("participant one should receive progression");
        };
        let ServerMessage::LootGranted(loot_two) = &messages_two[3] else {
            panic!("participant two should receive loot");
        };
        let ServerMessage::ProgressionGranted(progression_two) = &messages_two[4] else {
            panic!("participant two should receive progression");
        };
        assert_eq!(
            (loot_one.resulting_quantity, progression_one.experience),
            (1, 100)
        );
        assert_eq!(
            (loot_two.resulting_quantity, progression_two.experience),
            (4, 200)
        );
        assert_eq!(session.participants[0].module_state.fragments, 1);
        assert_eq!(session.participants[1].module_state.fragments, 4);
        assert_eq!(
            *calls.lock().expect("fixture call ledger should lock"),
            ["enemy_died", "completion_transaction"]
        );
    }

    #[test]
    fn warden_runtime_orders_and_bounds_authoritative_counterattacks() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) = fixture_participant("local:pressure", 955, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Boss, persistence);
        let boss = session
            .actors
            .spawn(ActorKind::Enemy, "warden", [2, 0, 0], 240);
        session.enemy_id = Some(boss.id);
        session.enemy_ai = Some(AiController::new(WARDEN_PRESSURE_PROFILE));
        session
            .activate_enemy_ai(boss.id, 955)
            .expect("Warden should enter its pressure-ready state");

        for (index, now_ms) in [0, 250, 500, 750, 1_000, 1_250].into_iter().enumerate() {
            session
                .attack_at(955, boss.id, now_ms)
                .unwrap_or_else(|error| panic!("accepted hit {} failed: {error}", index + 1));
        }

        assert_eq!(session.stage, SessionStage::Complete);
        assert_eq!(session.enemy_id, None);
        assert!(session.enemy_ai.is_none());
        assert_eq!(session.actors.get(955).unwrap().health, 70);
        let damage = receiver
            .try_iter()
            .filter_map(|message| match message {
                ServerMessage::DamageApplied(damage) => Some((
                    damage.source_actor_id,
                    damage.target_actor_id,
                    damage.damage,
                    damage.remaining_health,
                    damage.killed,
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            damage,
            [
                (955, boss.id, 40, 200, false),
                (955, boss.id, 40, 160, false),
                (boss.id, 955, 15, 85, false),
                (955, boss.id, 40, 120, false),
                (955, boss.id, 40, 80, false),
                (boss.id, 955, 15, 70, false),
                (955, boss.id, 40, 40, false),
                (955, boss.id, 40, 0, true),
            ]
        );
        assert_eq!(
            *calls.lock().expect("fixture call ledger should lock"),
            ["enemy_died", "completion_transaction"]
        );
    }

    #[test]
    fn lethal_pressure_prevents_a_defeated_player_from_attacking_again() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (mut participant, receiver) =
            fixture_participant("local:lethal-pressure", 956, &["pulse_rifle"]);
        participant.actor.health = 15;
        participant.actor.max_health = 15;
        let persistence = FixturePersistence::new(None, calls, Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Boss, persistence);
        let boss = session
            .actors
            .spawn(ActorKind::Enemy, "warden", [2, 0, 0], 240);
        session.enemy_id = Some(boss.id);
        session.enemy_ai = Some(AiController::new(WARDEN_PRESSURE_PROFILE));
        session
            .activate_enemy_ai(boss.id, 956)
            .expect("Warden should enter its pressure-ready state");

        session
            .attack_at(956, boss.id, 0)
            .expect("first hit should be accepted");
        session
            .attack_at(956, boss.id, 250)
            .expect("second hit should be accepted before the counterattack");
        assert_eq!(session.actors.get(956).unwrap().health, 0);
        assert_eq!(session.actors.get(boss.id).unwrap().health, 160);
        assert!(session.attack_at(956, boss.id, 500).is_err());
        assert_eq!(session.actors.get(boss.id).unwrap().health, 160);
        let damage = receiver
            .try_iter()
            .filter(|message| matches!(message, ServerMessage::DamageApplied(_)))
            .count();
        assert_eq!(damage, 3);
    }

    #[test]
    fn normal_combat_rejection_does_not_abort_the_command_loop() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:normal-error", 960, &["pulse_rifle", "arc_sidearm"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Drone, persistence);
        let enemy = session
            .actors
            .spawn(ActorKind::Enemy, "relay-drone", [4, 0, 2], 100);
        session.enemy_id = Some(enemy.id);
        let (command_tx, command_rx) = mpsc::channel();
        let run_thread = thread::spawn(move || session.run(command_rx));

        command_tx
            .send(SessionCommand::Attack {
                player_id: 960,
                target_id: enemy.id + 1,
            })
            .expect("invalid combat command should reach the coordinator");
        command_tx
            .send(SessionCommand::Equip {
                player_id: 960,
                item_id: "arc_sidearm".to_owned(),
            })
            .expect("follow-up command should reach the coordinator");
        let message = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("normal rejection must leave the participant connected");
        assert_eq!(message_kind(&message), "EquipmentChanged");
        command_tx
            .send(SessionCommand::Disconnect(960))
            .expect("disconnect should reach the coordinator");
        drop(command_tx);
        run_thread.join().expect("coordinator should stop cleanly");
    }

    #[test]
    fn persistence_failure_aborts_participants_and_accepts_a_fresh_session() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) = fixture_participant("local:aborted", 970, &["pulse_rifle"]);
        let persistence =
            FixturePersistence::new(Some("activity_started"), Arc::clone(&calls), Vec::new());
        let session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
        let (command_tx, command_rx) = mpsc::channel();
        let run_thread = thread::spawn(move || session.run(command_rx));

        command_tx
            .send(SessionCommand::Start(970))
            .expect("start should reach the coordinator");
        assert!(matches!(
            receiver.recv_timeout(Duration::from_secs(1)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ));
        let (replacement, _replacement_receiver) =
            fixture_participant("local:replacement", 971, &["pulse_rifle"]);
        let (join_result_tx, join_result_rx) = mpsc::channel();
        command_tx
            .send(SessionCommand::Join {
                participant: Box::new(replacement),
                result: join_result_tx,
            })
            .expect("replacement join should reach the coordinator");
        assert_eq!(
            join_result_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("replacement join should return"),
            Ok([0, 0, 0])
        );
        command_tx
            .send(SessionCommand::Disconnect(971))
            .expect("replacement disconnect should reach the coordinator");
        drop(command_tx);
        run_thread.join().expect("coordinator should stop cleanly");
        assert_eq!(
            *calls.lock().expect("fixture call ledger should lock"),
            ["activity_started", "player_joined"]
        );
    }

    #[test]
    fn late_disconnect_resets_after_failed_broadcast_only_once() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (participant, receiver) =
            fixture_participant("local:failed-broadcast", 980, &["pulse_rifle"]);
        drop(receiver);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let mut session = fixture_session(1, vec![participant], SessionStage::Drone, persistence);
        session.enemy_ai = Some(AiController::default());
        let failed_session_id = session.session_id.clone();

        session.broadcast(fatal_damage(980, 981));
        assert_eq!(session.participants.len(), 1);
        assert_eq!(session.stage, SessionStage::Drone);

        session
            .disconnect(980)
            .expect("late disconnect should remove the retained participant and reset");
        assert_eq!(session.stage, SessionStage::Waiting);
        assert!(session.enemy_ai.is_none());
        assert_ne!(session.session_id, failed_session_id);
        let reset_session_id = session.session_id.clone();

        session
            .disconnect(980)
            .expect("duplicate disconnect should remain idempotent");
        assert_eq!(session.session_id, reset_session_id);
    }

    #[test]
    fn cooperation_failed_broadcast_retains_role_until_abandonment() {
        let (mut session, anchor_rx, runner_rx, calls) =
            active_cooperation_fixture(None, 2_800, 2_801);
        drop(anchor_rx);

        session.broadcast(fatal_damage(2_800, 2_801));
        assert_eq!(session.participants.len(), 2);
        session
            .disconnect_at(2_800, 100)
            .expect("late disconnect should commit abandonment with the retained role");

        let summary = received_cooperation_summary(&runner_rx);
        assert_eq!(
            summary.outcome,
            super::WireCooperationTerminalOutcome::AbandonedDisconnect
        );
        assert_eq!(
            summary.subject_role,
            Some(super::WireCooperationRole::Anchor)
        );
        assert_eq!(
            summary.no_reward_reason,
            Some(super::CooperationNoRewardReason::ParticipantDisconnected)
        );
        assert_eq!(session.participants.len(), 1);
        assert_eq!(
            calls
                .lock()
                .expect("fixture call ledger should lock")
                .iter()
                .filter(|call| call.as_str() == "cooperation_abandoned")
                .count(),
            1
        );
    }

    #[test]
    fn default_address_is_local_only() {
        assert_eq!(DEFAULT_BIND_ADDR, "127.0.0.1:8080");
        assert_eq!(DEFAULT_GAME_ADDR, "127.0.0.1:7000");
        assert_eq!(
            database_startup_mode(None).unwrap(),
            DatabaseStartupMode::Migrate
        );
        assert_eq!(
            database_startup_mode(Some("existing")).unwrap(),
            DatabaseStartupMode::Existing
        );
        assert!(database_startup_mode(Some("unsafe")).is_err());
    }

    #[test]
    fn multiplayer_session_stages_are_explicit() {
        assert_ne!(SessionStage::Waiting, SessionStage::Drone);
        assert_ne!(SessionStage::Door, SessionStage::Boss);
        assert_ne!(SessionStage::Boss, SessionStage::Complete);
    }

    #[test]
    fn manual_movement_stays_inside_relay_hub() {
        assert!(valid_movement_target([6, 0, 0]));
        assert!(valid_movement_target([-12, 0, 12]));
        assert!(!valid_movement_target([13, 0, 0]));
        assert!(!valid_movement_target([0, 1, 0]));
        assert!(!valid_movement_target([i32::MIN, 0, 0]));
        assert!(!valid_movement_target([0, 0, i32::MIN]));
        assert!(!valid_movement_target([i32::MAX, 0, i32::MAX]));
    }

    #[test]
    fn enemy_health_scales_by_half_for_each_additional_participant() {
        assert_eq!(scaled_enemy_health(140, 1).unwrap(), 140);
        assert_eq!(scaled_enemy_health(140, 2).unwrap(), 210);
        assert_eq!(scaled_enemy_health(240, 1).unwrap(), 240);
        assert_eq!(scaled_enemy_health(240, 2).unwrap(), 360);
        assert!(scaled_enemy_health(140, 0).is_err());
        assert!(scaled_enemy_health(u32::MAX, 2).is_err());
    }

    #[test]
    fn active_or_full_sessions_reject_late_players() {
        assert!(session_accepts_join(SessionStage::Waiting, 0, 1));
        assert!(!session_accepts_join(SessionStage::Waiting, 1, 1));
        assert!(!session_accepts_join(SessionStage::Drone, 0, 1));
        assert!(!session_accepts_join(SessionStage::Complete, 0, 1));
    }

    #[test]
    fn module_projection_is_exact_and_frozen_v1_remains_base() {
        let state = PersistedModuleState {
            fragments: 7,
            owned_modules: vec![ModuleId::WardCapacitor, ModuleId::ForceMatrix],
            loadout: vec![ModuleId::ForceMatrix, ModuleId::WardCapacitor],
            revision: 3,
            combination_operations: 2,
            loadout_operations: 1,
        };
        let active = resolved_module_projection(&state, true, false)
            .expect("current V2 projection should resolve");
        assert_eq!(active.max_health, 120);
        assert_eq!(
            active
                .weapons
                .iter()
                .map(|profile| (
                    profile.item_id.as_str(),
                    profile.effective_damage,
                    profile.effective_range,
                    profile.effective_cooldown_ms,
                ))
                .collect::<Vec<_>>(),
            [("pulse_rifle", 48, 6, 325), ("arc_sidearm", 30, 8, 195)]
        );
        let frozen = resolved_module_projection(&state, false, false)
            .expect("frozen V1 projection should resolve");
        assert_eq!(frozen.max_health, 100);
        assert_eq!(
            frozen
                .weapons
                .iter()
                .map(|profile| (
                    profile.item_id.as_str(),
                    profile.effective_damage,
                    profile.effective_range,
                    profile.effective_cooldown_ms,
                ))
                .collect::<Vec<_>>(),
            [("pulse_rifle", 40, 6, 250), ("arc_sidearm", 25, 8, 150)]
        );
        let snapshot = module_snapshot(&state, false).expect("module snapshot should resolve");
        assert_eq!(snapshot.catalog.len(), 4);
        assert_eq!(snapshot.weapons.len(), 2);
        assert_eq!(snapshot.maximum_slots, 3);
        assert_eq!(snapshot.max_health, 120);
    }

    #[test]
    fn lance_requires_content_negotiation_and_preserves_cover_and_cooldown() {
        for capable in [false, true] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let (mut player, receiver) =
                fixture_participant("local:lance", 1_180, &["pulse_rifle", "coil_lance"]);
            player.content_capable = capable;
            player.admitted_module_weapons =
                resolved_module_projection(&player.module_state, true, capable)
                    .unwrap()
                    .weapons;
            let persistence = FixturePersistence::new(None, calls, Vec::new());
            let mut session = fixture_session(1, vec![player], SessionStage::Waiting, persistence);
            start_and_clear_drone(&mut session, 1_180);
            session.equip_weapon(1_180, "coil_lance").unwrap();
            assert_eq!(
                session.participants[0].equipped_weapon_item_id,
                if capable { "coil_lance" } else { "pulse_rifle" }
            );
            if !capable {
                assert!(drain_messages(&receiver).iter().any(|message| matches!(message, ServerMessage::EquipmentChanged(result) if !result.accepted)));
                continue;
            }
            session
                .move_player(1_180, revenant_ai::sentinel::SIGNAL_APPROACH)
                .unwrap();
            let sentinel = session.enemy_id.unwrap();
            assert!(session.attack_at(1_180, sentinel, 0).is_err());
            session.move_player(1_180, [-7, 0, 4]).unwrap();
            assert!(session.attack_at(1_180, sentinel, 0).is_err());
            session.move_player(1_180, [-8, 0, 4]).unwrap();
            session.attack_at(1_180, sentinel, 0).unwrap();
            assert_eq!(session.actors.get(sentinel).unwrap().health, 152);
            assert!(session.attack_at(1_180, sentinel, 349).is_err());
            session.attack_at(1_180, sentinel, 350).unwrap();
            assert_eq!(session.actors.get(sentinel).unwrap().health, 104);
        }
    }

    #[test]
    fn preview_is_targeted_non_mutating_and_rejects_invalid_shapes() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (mut requester, requester_rx) =
            fixture_participant("local:preview", 1_001, &["pulse_rifle"]);
        requester.module_state.fragments = 3;
        let original_state = requester.module_state.clone();
        let original_profiles = requester.admitted_module_weapons.clone();
        let (observer, observer_rx) =
            fixture_participant("local:preview-observer", 1_002, &["pulse_rifle"]);
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new());
        let session = fixture_session(
            2,
            vec![requester, observer],
            SessionStage::Complete,
            persistence,
        );

        session
            .preview_modules(
                1_001,
                &ModulePreviewRequest {
                    modules: vec![
                        ModuleId::WardCapacitor.to_string(),
                        ModuleId::ForceMatrix.to_string(),
                    ],
                },
            )
            .expect("valid unowned preview should resolve");
        let ServerMessage::ModulePreview(preview) = requester_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("requester should receive preview")
        else {
            panic!("requester should receive ModulePreview");
        };
        assert!(preview.accepted);
        assert_eq!(preview.max_health, 120);
        assert_eq!(preview.requested_modules.len(), 2);
        assert_eq!(preview.weapons[0].effective_damage, 48);
        assert_eq!(preview.weapons[0].effective_cooldown_ms, 325);
        assert_eq!(observer_rx.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(session.participants[0].module_state, original_state);
        assert_eq!(
            session.participants[0].admitted_module_weapons,
            original_profiles
        );

        for modules in [
            vec!["unknown_module".to_owned()],
            vec![
                ModuleId::ForceMatrix.to_string(),
                ModuleId::ForceMatrix.to_string(),
            ],
            vec![
                ModuleId::ForceMatrix.to_string(),
                ModuleId::TempoRegulator.to_string(),
                ModuleId::ReachLattice.to_string(),
                ModuleId::WardCapacitor.to_string(),
            ],
        ] {
            session
                .preview_modules(1_001, &ModulePreviewRequest { modules })
                .expect("invalid preview should be projected as a rejection");
            let ServerMessage::ModulePreview(rejected) = requester_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("requester should receive preview rejection")
            else {
                panic!("requester should receive rejected ModulePreview");
            };
            assert!(!rejected.accepted);
            assert!(rejected.requested_modules.is_empty());
            assert!(rejected.weapons.is_empty());
            assert_eq!(rejected.max_health, 0);
        }
        assert!(calls
            .lock()
            .expect("fixture call ledger should lock")
            .is_empty());
    }

    #[test]
    fn active_module_mutations_reject_without_persistence_or_state_change() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (mut participant, receiver) =
            fixture_participant("local:active-modules", 1_010, &["pulse_rifle"]);
        participant.module_state.fragments = 4;
        let original = participant.module_state.clone();
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new())
            .with_module_state(4, &[], &[], 0);
        let mut session = fixture_session(1, vec![participant], SessionStage::Drone, persistence);

        session
            .combine_module(
                1_010,
                &ModuleCombineIntent {
                    operation_id: "active-combine".to_owned(),
                    module_id: ModuleId::ForceMatrix.to_string(),
                },
            )
            .expect("active combination should return a domain rejection");
        let ServerMessage::ModuleCombined(combined) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("combination rejection should be targeted")
        else {
            panic!("expected ModuleCombined rejection");
        };
        assert!(!combined.accepted);
        assert_eq!(combined.state.fragments, 4);

        session
            .set_module_loadout(
                1_010,
                &ModuleLoadoutIntent {
                    operation_id: "active-loadout".to_owned(),
                    expected_revision: 0,
                    modules: vec![ModuleId::ForceMatrix.to_string()],
                },
            )
            .expect("active loadout should return a domain rejection");
        let ServerMessage::ModuleLoadoutChanged(loadout) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("loadout rejection should be targeted")
        else {
            panic!("expected ModuleLoadoutChanged rejection");
        };
        assert!(!loadout.accepted);
        assert_eq!(loadout.state.fragments, 4);
        assert_eq!(session.participants[0].module_state, original);
        assert!(calls
            .lock()
            .expect("fixture call ledger should lock")
            .is_empty());
    }

    fn apply_and_retry_fixture_modules(
        session: &mut SharedSession,
        receiver: &Receiver<ServerMessage>,
    ) {
        for (operation_id, module_id) in [
            ("combine-force", ModuleId::ForceMatrix),
            ("combine-force", ModuleId::ForceMatrix),
            ("combine-ward", ModuleId::WardCapacitor),
        ] {
            session
                .combine_module(
                    1_020,
                    &ModuleCombineIntent {
                        operation_id: operation_id.to_owned(),
                        module_id: module_id.to_string(),
                    },
                )
                .expect("completed combination should resolve");
        }
        let ServerMessage::ModuleCombined(first) =
            receiver.recv().expect("first combination should respond")
        else {
            panic!("expected first ModuleCombined");
        };
        let ServerMessage::ModuleCombined(retry) =
            receiver.recv().expect("combination retry should respond")
        else {
            panic!("expected retried ModuleCombined");
        };
        let ServerMessage::ModuleCombined(second) =
            receiver.recv().expect("second combination should respond")
        else {
            panic!("expected second ModuleCombined");
        };
        assert!(first.accepted && !first.replayed);
        assert!(retry.accepted && retry.replayed);
        assert_eq!(first.state.fragments, 2);
        assert_eq!(retry.state.fragments, 2);
        assert_eq!(second.state.fragments, 0);

        for _ in 0..2 {
            session
                .set_module_loadout(
                    1_020,
                    &ModuleLoadoutIntent {
                        operation_id: "loadout-force-ward".to_owned(),
                        expected_revision: 0,
                        modules: vec![
                            ModuleId::WardCapacitor.to_string(),
                            ModuleId::ForceMatrix.to_string(),
                        ],
                    },
                )
                .expect("completed loadout should resolve");
        }
        let ServerMessage::ModuleLoadoutChanged(applied) =
            receiver.recv().expect("loadout should respond")
        else {
            panic!("expected applied ModuleLoadoutChanged");
        };
        let ServerMessage::ModuleLoadoutChanged(replayed) =
            receiver.recv().expect("loadout retry should respond")
        else {
            panic!("expected replayed ModuleLoadoutChanged");
        };
        assert!(applied.accepted && !applied.replayed);
        assert!(replayed.accepted && replayed.replayed);
        assert_eq!(applied.state.loadout_revision, 1);
        assert_eq!(replayed.state.loadout_revision, 1);
        assert_eq!(applied.state.max_health, 120);
    }

    #[test]
    fn completed_mutations_are_idempotent_and_activate_only_on_next_admission() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (mut participant, receiver) =
            fixture_participant("local:next-admission", 1_020, &["pulse_rifle"]);
        participant.module_state.fragments = 4;
        let persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new())
            .with_module_state(4, &[], &[], 0);
        let mut session =
            fixture_session(1, vec![participant], SessionStage::Complete, persistence);

        apply_and_retry_fixture_modules(&mut session, &receiver);

        session
            .equip_weapon(1_020, "pulse_rifle")
            .expect("post-completion equipment should reject cleanly");
        let ServerMessage::EquipmentChanged(locked) =
            receiver.recv().expect("equipment rejection should respond")
        else {
            panic!("expected EquipmentChanged rejection");
        };
        assert!(!locked.accepted);
        assert_eq!((locked.damage, locked.cooldown_ms), (40, 250));
        assert_eq!(session.participants[0].admitted_max_health, 100);
        assert_eq!(session.actors.get(1_020).unwrap().max_health, 100);

        session
            .disconnect(1_020)
            .expect("completed participant should disconnect and reset");
        let (replacement, _replacement_rx) =
            fixture_participant("local:next-admission", 1_021, &["pulse_rifle"]);
        session
            .join(replacement)
            .expect("next admission should load persisted modules");
        let admitted = &session.participants[0];
        assert_eq!(admitted.admitted_max_health, 120);
        assert_eq!(admitted.actor.max_health, 120);
        let rifle = admitted
            .admitted_weapon()
            .expect("rifle should be admitted");
        assert_eq!(
            (rifle.effective_damage, rifle.effective_cooldown_ms),
            (48, 325)
        );
        assert_eq!(
            *calls.lock().expect("fixture call ledger should lock"),
            [
                "module_combined",
                "module_state_loaded",
                "module_combined",
                "module_state_loaded",
                "module_combined",
                "module_state_loaded",
                "module_loadout_changed",
                "module_state_loaded",
                "module_loadout_changed",
                "module_state_loaded",
                "player_joined",
            ]
        );
    }

    #[test]
    fn multiplayer_attacks_use_each_participants_distinct_admitted_build() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (mut force, force_rx) =
            fixture_participant("local:force-player", 1_030, &["pulse_rifle"]);
        force.module_state.owned_modules = vec![ModuleId::ForceMatrix];
        force.module_state.loadout = vec![ModuleId::ForceMatrix];
        let force_projection =
            resolved_module_projection(&force.module_state, true, false).unwrap();
        force.admitted_module_weapons = force_projection.weapons;
        force.admitted_max_health = force_projection.max_health;
        let (mut ward, ward_rx) = fixture_participant("local:ward-player", 1_031, &["arc_sidearm"]);
        ward.equipped_weapon_item_id = "arc_sidearm".to_owned();
        ward.module_state.owned_modules = vec![ModuleId::WardCapacitor];
        ward.module_state.loadout = vec![ModuleId::WardCapacitor];
        let ward_projection = resolved_module_projection(&ward.module_state, true, false).unwrap();
        ward.admitted_module_weapons = ward_projection.weapons;
        ward.admitted_max_health = ward_projection.max_health;
        ward.actor.health = ward_projection.max_health;
        ward.actor.max_health = ward_projection.max_health;
        let persistence = FixturePersistence::new(None, calls, Vec::new());
        let mut session = fixture_session(2, vec![force, ward], SessionStage::Boss, persistence);
        let enemy = session
            .actors
            .spawn(ActorKind::Enemy, "warden", [2, 0, 0], 500);
        session.enemy_id = Some(enemy.id);

        session
            .attack_at(1_030, enemy.id, 0)
            .expect("force participant should attack");
        session
            .attack_at(1_031, enemy.id, 0)
            .expect("ward participant should attack");
        let damage = force_rx
            .try_iter()
            .filter_map(|message| match message {
                ServerMessage::DamageApplied(damage)
                    if matches!(damage.source_actor_id, 1_030 | 1_031) =>
                {
                    Some((damage.source_actor_id, damage.damage))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(damage, [(1_030, 48), (1_031, 25)]);
        assert_eq!(session.actors.get(1_031).unwrap().max_health, 120);
        assert_eq!(ward_rx.try_iter().count(), 2);
    }

    #[test]
    fn http_request_line_is_bounded_and_complete() {
        let line = read_http_request_line(Cursor::new(b"GET /health HTTP/1.1\r\n"))
            .expect("complete request line should parse");
        assert_eq!(line, "GET /health HTTP/1.1\r\n");
        assert_eq!(
            inspector_request_path(&line).expect("GET request should parse"),
            Some("/health")
        );
        assert_eq!(
            inspector_request_path("POST /health HTTP/1.1\r\n")
                .expect("known non-GET method should parse"),
            None
        );
        assert!(inspector_request_path("GET /health HTTP/2 extra\r\n").is_err());
        assert!(read_http_request_line(Cursor::new(b"GET /health HTTP/1.1")).is_err());
        let oversized = vec![b'a'; MAX_HTTP_REQUEST_LINE + 1];
        assert!(read_http_request_line(Cursor::new(oversized)).is_err());
    }

    #[test]
    fn inspector_origin_configuration_is_literal_loopback_only() {
        assert_eq!(
            configured_inspector_origin(None).unwrap(),
            DEFAULT_INSPECTOR_ORIGIN
        );
        assert_eq!(
            configured_inspector_origin(Some("http://127.0.0.1:18080")).unwrap(),
            "http://127.0.0.1:18080"
        );
        for rejected in [
            "https://127.0.0.1:4173",
            "http://localhost:4173",
            "http://[::1]:4173",
            "http://127.0.0.1:0",
            "http://127.0.0.1:00",
            "http://127.0.0.1:65536",
            "http://127.0.0.1:4173/path",
            "http://127.0.0.1:4173?query",
            "http://127.0.0.1:4173#fragment",
            "http://user@127.0.0.1:4173",
            "http://192.168.1.5:4173",
        ] {
            assert!(
                configured_inspector_origin(Some(rejected)).is_err(),
                "unexpectedly accepted {rejected}"
            );
        }
    }

    #[test]
    fn correlation_digests_are_fresh_lowercase_random_128_bit_values() {
        let first = CorrelationDigest::generate().expect("first digest should be available");
        let second = CorrelationDigest::generate().expect("second digest should be available");
        assert_ne!(first, second);
        for digest in [first, second] {
            assert_eq!(digest.as_str().len(), 32);
            assert!(digest
                .as_str()
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        }
    }

    #[test]
    fn handshake_and_idle_deadlines_are_inclusive() {
        for deadline in [HANDSHAKE_TIMEOUT, GAMEPLAY_IDLE_TIMEOUT] {
            assert!(!deadline_expired(deadline, deadline));
            assert!(deadline_expired(
                deadline + Duration::from_nanos(1),
                deadline
            ));
        }
    }

    #[test]
    fn frame_deadline_does_not_restart_when_bytes_trickle() {
        use std::io::{Read, Write};
        use std::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let started = Instant::now();
        let mut reader = super::DeadlineReader {
            stream: &server,
            started,
            budget: Duration::from_millis(120),
        };
        peer.write_all(&[0]).unwrap();
        let mut prefix = [0_u8; 4];
        reader.read_exact(&mut prefix[..1]).unwrap();
        thread::sleep(Duration::from_millis(70));
        peer.write_all(&[0]).unwrap();
        reader.read_exact(&mut prefix[1..2]).unwrap();
        thread::sleep(Duration::from_millis(70));
        peer.write_all(&[0, 0]).unwrap();
        assert_eq!(
            reader.read_exact(&mut prefix[2..]).unwrap_err().kind(),
            std::io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn participant_exit_and_writer_lifetime_release_each_slot_once() {
        let (command_tx, commands) = mpsc::sync_channel(1);
        let participant = super::ParticipantConnectionGuard {
            command_tx,
            player_id: 123,
        };
        drop(participant);
        assert!(matches!(
            commands.recv().unwrap(),
            SessionCommand::Disconnect(123)
        ));
        assert!(commands.try_recv().is_err());

        let counter = Arc::new(AtomicUsize::new(1));
        let handler = Arc::new(super::CounterGuard::new(Arc::clone(&counter)));
        let writer = Arc::clone(&handler);
        drop(handler);
        assert_eq!(counter.load(AtomicOrdering::Relaxed), 1);
        drop(writer);
        assert_eq!(counter.load(AtomicOrdering::Relaxed), 0);
    }

    #[test]
    fn rolling_frame_limiter_enforces_exact_second_and_minute_boundaries() {
        let mut burst = FrameRateLimiter::default();
        for _ in 0..FRAME_BURST_LIMIT {
            burst
                .observe_at(Duration::ZERO)
                .expect("first 32 burst frames should pass");
        }
        assert_eq!(
            burst.observe_at(Duration::ZERO),
            Err(FrameRateExceeded::Second)
        );
        burst
            .observe_at(Duration::from_secs(1))
            .expect("exactly expired burst timestamp should open capacity");

        let mut minute = FrameRateLimiter::default();
        for index in 0..FRAME_MINUTE_LIMIT {
            minute
                .observe_at(Duration::from_millis(index as u64 * 100))
                .expect("first 600 minute frames should pass");
        }
        assert_eq!(
            minute.observe_at(Duration::from_millis(59_950)),
            Err(FrameRateExceeded::Minute)
        );
        minute
            .observe_at(Duration::from_mins(1))
            .expect("exactly expired minute timestamp should open capacity");
        assert_eq!(
            minute.observe_at(Duration::from_millis(59_999)),
            Err(FrameRateExceeded::ClockReversed)
        );
    }

    #[test]
    fn outbound_queue_closes_on_first_excess_without_affecting_peer() {
        let correlation = CorrelationDigest::fixture("11111111111111111111111111111111");
        let (outbound, receiver) = OutboundSender::channel(correlation);
        let message = ServerMessage::ActivityComplete(ActivityComplete {
            activity_id: "fixture".to_owned(),
        });
        for _ in 0..OUTBOUND_QUEUE_CAPACITY {
            outbound
                .send(message.clone())
                .expect("the exact queue capacity should be accepted");
        }
        assert!(outbound.send(message.clone()).is_err());
        assert!(outbound.send(message.clone()).is_err());
        assert_eq!(receiver.try_iter().count(), OUTBOUND_QUEUE_CAPACITY);
        assert_eq!(receiver.try_recv(), Err(TryRecvError::Disconnected));

        let peer_correlation = CorrelationDigest::fixture("22222222222222222222222222222222");
        let (peer, peer_receiver) = OutboundSender::channel(peer_correlation);
        peer.send(message.clone())
            .expect("independent peer queue should remain usable");
        assert_eq!(peer_receiver.recv().unwrap(), message);
    }

    #[test]
    fn inspector_http_headers_are_bounded_and_origin_is_not_reflected() {
        let exact = format!(
            "GET /api/inspector/not-found HTTP/1.1\r\nOrigin: {DEFAULT_INSPECTOR_ORIGIN}\r\n\r\n"
        );
        let request = read_http_request(Cursor::new(exact)).expect("exact origin should parse");
        assert_eq!(request.origin.as_deref(), Some(DEFAULT_INSPECTOR_ORIGIN));
        let response = handle_http_request(&request, "unused", DEFAULT_INSPECTOR_ORIGIN)
            .expect("not-found route should not touch persistence")
            .render();
        assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
        assert!(response.contains(&format!(
            "Access-Control-Allow-Origin: {DEFAULT_INSPECTOR_ORIGIN}\r\n"
        )));
        assert!(response.contains("Vary: Origin\r\n"));
        assert!(response.contains("Cache-Control: no-store\r\n"));
        assert!(!response.contains("Access-Control-Allow-Origin: *"));

        let missing = HttpRequest {
            method: "GET".to_owned(),
            path: "/api/inspector/not-found".to_owned(),
            origin: None,
        };
        let forbidden = handle_http_request(&missing, "unused", DEFAULT_INSPECTOR_ORIGIN)
            .unwrap()
            .render();
        assert!(forbidden.starts_with("HTTP/1.1 403 Forbidden\r\n"));
        assert!(!forbidden.contains("Access-Control-Allow-Origin"));
        assert!(forbidden.contains("Cache-Control: no-store\r\n"));

        let duplicate = format!(
            "GET /health HTTP/1.1\r\nOrigin: {DEFAULT_INSPECTOR_ORIGIN}\r\norigin: {DEFAULT_INSPECTOR_ORIGIN}\r\n\r\n"
        );
        assert!(read_http_request(Cursor::new(duplicate)).is_err());

        let too_many = format!(
            "GET /health HTTP/1.1\r\n{}\r\n",
            "X: y\r\n".repeat(MAX_HTTP_HEADER_LINES + 1)
        );
        assert!(read_http_request(Cursor::new(too_many)).is_err());
        let exact_lines = format!(
            "GET /health HTTP/1.1\r\n{}\r\n",
            "X: y\r\n".repeat(MAX_HTTP_HEADER_LINES)
        );
        assert!(read_http_request(Cursor::new(exact_lines)).is_ok());
        let full_line = format!("X:{}\r\n", "a".repeat(MAX_HTTP_HEADER_LINE - 4));
        let shorter_line = format!("X:{}\r\n", "a".repeat(MAX_HTTP_HEADER_LINE - 5));
        let exact_headers = format!("{}{}\r\n", full_line.repeat(2), shorter_line.repeat(2));
        assert_eq!(exact_headers.len(), MAX_HTTP_HEADER_BYTES);
        assert!(read_http_request(Cursor::new(format!(
            "GET /health HTTP/1.1\r\n{exact_headers}"
        )))
        .is_ok());
        let excess_headers = format!("{}{shorter_line}\r\n", full_line.repeat(3));
        assert_eq!(excess_headers.len(), MAX_HTTP_HEADER_BYTES + 1);
        assert!(read_http_request(Cursor::new(format!(
            "GET /health HTTP/1.1\r\n{excess_headers}"
        )))
        .is_err());
        let oversized_line = format!(
            "GET /health HTTP/1.1\r\nX: {}\r\n\r\n",
            "a".repeat(MAX_HTTP_HEADER_LINE)
        );
        assert!(read_http_request(Cursor::new(oversized_line)).is_err());
        let large_header = format!("X: {}\r\n", "a".repeat(4090));
        let oversized_total = format!("GET /health HTTP/1.1\r\n{}\r\n", large_header.repeat(5));
        assert!(oversized_total.len() > MAX_HTTP_HEADER_BYTES);
        assert!(read_http_request(Cursor::new(oversized_total)).is_err());

        let post = HttpRequest {
            method: "POST".to_owned(),
            path: "/api/inspector/sessions".to_owned(),
            origin: Some(DEFAULT_INSPECTOR_ORIGIN.to_owned()),
        };
        let post_response = handle_http_request(&post, "unused", DEFAULT_INSPECTOR_ORIGIN)
            .unwrap()
            .render();
        assert!(post_response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"));
        assert!(!post_response.contains("Access-Control-Allow-Origin"));

        let generic = HttpResponse::json("500 Internal Server Error", "{}", None).render();
        assert!(generic.contains("Cache-Control: no-store\r\n"));
    }

    #[test]
    fn inspector_database_failure_is_generic_with_exact_origin_and_no_store() {
        let request = HttpRequest {
            method: "GET".to_owned(),
            path: "/api/inspector/sessions".to_owned(),
            origin: Some(DEFAULT_INSPECTOR_ORIGIN.to_owned()),
        };
        let response = super::http_response(
            &request,
            "invalid-database-canary",
            DEFAULT_INSPECTOR_ORIGIN,
            None,
        );
        assert_eq!(response.status, "500 Internal Server Error");
        assert_eq!(response.body, "{\"error\":\"internal_error\"}\n");
        let rendered = response.render();
        assert!(!rendered.contains("canary"));
        assert!(rendered.contains("Cache-Control: no-store\r\n"));
        assert!(rendered.contains(&format!(
            "Access-Control-Allow-Origin: {DEFAULT_INSPECTOR_ORIGIN}\r\n"
        )));
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
            inspector_route("/api/inspector/sessions/session-123/summary"),
            InspectorRoute::Summary("session-123")
        );
        assert_eq!(
            inspector_route("/api/inspector/sessions/../events"),
            InspectorRoute::NotFound
        );
        assert_eq!(
            inspector_route("/api/inspector/sessions/../summary"),
            InspectorRoute::NotFound
        );
    }
}
