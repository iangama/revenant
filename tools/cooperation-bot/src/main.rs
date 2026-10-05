use std::collections::VecDeque;
use std::env;
use std::io;
use std::net::{Shutdown, TcpStream};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use revenant_protocol::{
    read_message, write_message, AttackIntent, AuthRequest, CharacterListRequest, ClientHello,
    ClientMessage, CooperationLife, CooperationLifeCause, CooperationNoRewardReason,
    CooperationOperationState, CooperationOperationSummary, CooperationPhase,
    CooperationPingIntent, CooperationPingResult, CooperationReviveIntent, CooperationReviveResult,
    CooperationReviveStatus, CooperationRole, CooperationStartIntent, CooperationStartResult,
    CooperationState, CooperationStateRequest, CooperationTerminalOutcome, MoveIntent,
    RouteChoiceIntent, ServerMessage, WorldJoinRequest, PROTOCOL_VERSION,
};

const IO_TIMEOUT: Duration = Duration::from_secs(15);
const EVENT_TIMEOUT: Duration = Duration::from_secs(20);
const ATTACK_INTERVAL: Duration = Duration::from_millis(275);
const PING_TIMEOUT_OBSERVATION: Duration = Duration::from_millis(5_100);
const REVIVE_TIMEOUT_OBSERVATION: Duration = Duration::from_millis(15_100);
const OPERATION_TIMEOUT_OBSERVATION: Duration = Duration::from_millis(60_100);
const REVIVE_COMPLETION_OBSERVATION: Duration = Duration::from_millis(2_100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Anchor,
    Runner,
}

impl Side {
    const ALL: [Self; 2] = [Self::Anchor, Self::Runner];

    const fn index(self) -> usize {
        match self {
            Self::Anchor => 0,
            Self::Runner => 1,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Anchor => "anchor",
            Self::Runner => "runner",
        }
    }

    const fn role(self) -> CooperationRole {
        match self {
            Self::Anchor => CooperationRole::Anchor,
            Self::Runner => CooperationRole::Runner,
        }
    }

    const fn peer(self) -> Self {
        match self {
            Self::Anchor => Self::Runner,
            Self::Runner => Self::Anchor,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisconnectPhase {
    AwaitingAnchor,
    AwaitingPing,
    AwaitingRunner,
    RunnerDowned,
    ReviveChannel,
    EncounterActive,
}

impl DisconnectPhase {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "awaiting-anchor" => Some(Self::AwaitingAnchor),
            "awaiting-ping" => Some(Self::AwaitingPing),
            "awaiting-runner" => Some(Self::AwaitingRunner),
            "runner-downed" => Some(Self::RunnerDowned),
            "revive-channel" => Some(Self::ReviveChannel),
            "encounter-active" => Some(Self::EncounterActive),
            _ => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::AwaitingAnchor => "awaiting-anchor",
            Self::AwaitingPing => "awaiting-ping",
            Self::AwaitingRunner => "awaiting-runner",
            Self::RunnerDowned => "runner-downed",
            Self::ReviveChannel => "revive-channel",
            Self::EncounterActive => "encounter-active",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Success,
    PingTimeout,
    ReviveTimeout,
    OperationTimeout,
    Disconnect { side: Side, phase: DisconnectPhase },
}

impl Scenario {
    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            "" | "success" => Ok(Self::Success),
            "ping-timeout" => Ok(Self::PingTimeout),
            "revive-timeout" => Ok(Self::ReviveTimeout),
            "operation-timeout" => Ok(Self::OperationTimeout),
            _ => {
                let Some(rest) = value.strip_prefix("disconnect-") else {
                    return Err(format!("unknown cooperation bot scenario: {value}").into());
                };
                let (side, phase) = if let Some(phase) = rest.strip_prefix("anchor-") {
                    (Side::Anchor, phase)
                } else if let Some(phase) = rest.strip_prefix("runner-") {
                    (Side::Runner, phase)
                } else {
                    return Err("disconnect scenario must name anchor or runner".into());
                };
                let phase = DisconnectPhase::parse(phase)
                    .ok_or("disconnect scenario has an unknown operation phase")?;
                Ok(Self::Disconnect { side, phase })
            }
        }
    }

    fn label(self) -> String {
        match self {
            Self::Success => "success".to_owned(),
            Self::PingTimeout => "ping-timeout".to_owned(),
            Self::ReviveTimeout => "revive-timeout".to_owned(),
            Self::OperationTimeout => "operation-timeout".to_owned(),
            Self::Disconnect { side, phase } => {
                format!("disconnect-{}-{}", side.label(), phase.label())
            }
        }
    }
}

struct Client {
    stream: TcpStream,
    actor_id: u64,
    account_id: String,
}

struct PairEvent {
    side: Side,
    message: ServerMessage,
}

struct ClientPair {
    clients: [Client; 2],
    events: Receiver<PairEvent>,
    pending: VecDeque<PairEvent>,
}

impl ClientPair {
    fn connect(game_addr: &str, account_prefix: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let anchor = connect_client(game_addr, &format!("{account_prefix}-a"))?;
        let runner = connect_client(game_addr, &format!("{account_prefix}-r"))?;
        let (event_tx, events) = mpsc::channel();
        spawn_reader(Side::Anchor, &anchor.stream, event_tx.clone())?;
        spawn_reader(Side::Runner, &runner.stream, event_tx)?;
        Ok(Self {
            clients: [anchor, runner],
            events,
            pending: VecDeque::new(),
        })
    }

    fn actor_id(&self, side: Side) -> u64 {
        self.clients[side.index()].actor_id
    }

    fn account_id(&self, side: Side) -> &str {
        &self.clients[side.index()].account_id
    }

    #[allow(clippy::needless_pass_by_value)]
    fn send(
        &mut self,
        side: Side,
        message: ClientMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        write_message(&mut self.clients[side.index()].stream, &message)?;
        Ok(())
    }

    fn disconnect(&mut self, side: Side) -> io::Result<()> {
        self.clients[side.index()].stream.shutdown(Shutdown::Both)
    }

    fn wait_for<F>(
        &mut self,
        side: Side,
        label: &str,
        predicate: F,
    ) -> Result<ServerMessage, Box<dyn std::error::Error>>
    where
        F: Fn(&ServerMessage) -> bool,
    {
        let deadline = Instant::now() + EVENT_TIMEOUT;
        loop {
            if let Some(index) = self
                .pending
                .iter()
                .position(|event| event.side == side && predicate(&event.message))
            {
                return Ok(self
                    .pending
                    .remove(index)
                    .ok_or("pending cooperation event disappeared")?
                    .message);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!("timed out waiting for {label} on {}", side.label()).into());
            }
            match self.events.recv_timeout(remaining) {
                Ok(event) if event.side == side && predicate(&event.message) => {
                    return Ok(event.message);
                }
                Ok(event) => self.pending.push_back(event),
                Err(RecvTimeoutError::Timeout) => {
                    return Err(format!("timed out waiting for {label} on {}", side.label()).into());
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("cooperation reader threads stopped".into());
                }
            }
        }
    }

    fn wait_state(
        &mut self,
        side: Side,
        phase: CooperationPhase,
    ) -> Result<CooperationState, Box<dyn std::error::Error>> {
        let message = self.wait_for(side, "cooperation state", |message| {
            matches!(message, ServerMessage::CooperationState(state) if state.phase == phase)
        })?;
        let ServerMessage::CooperationState(state) = message else {
            unreachable!();
        };
        Ok(state)
    }

    fn wait_start(
        &mut self,
        side: Side,
        operation_id: &str,
    ) -> Result<CooperationStartResult, Box<dyn std::error::Error>> {
        let message = self.wait_for(side, "cooperation start result", |message| {
            matches!(message, ServerMessage::CooperationStartResult(result) if result.operation_id == operation_id || result.operation_id.is_empty())
        })?;
        let ServerMessage::CooperationStartResult(result) = message else {
            unreachable!();
        };
        Ok(result)
    }

    fn wait_ping(
        &mut self,
        side: Side,
        operation_id: &str,
    ) -> Result<CooperationPingResult, Box<dyn std::error::Error>> {
        let message = self.wait_for(side, "cooperation ping result", |message| {
            matches!(message, ServerMessage::CooperationPingResult(result) if result.operation_id == operation_id || result.operation_id.is_empty())
        })?;
        let ServerMessage::CooperationPingResult(result) = message else {
            unreachable!();
        };
        Ok(result)
    }

    fn wait_revive(
        &mut self,
        side: Side,
        operation_id: &str,
        status: CooperationReviveStatus,
    ) -> Result<CooperationReviveResult, Box<dyn std::error::Error>> {
        let message = self.wait_for(side, "cooperation revive result", |message| {
            matches!(message, ServerMessage::CooperationReviveResult(result) if (result.operation_id == operation_id || result.operation_id.is_empty()) && result.status == status)
        })?;
        let ServerMessage::CooperationReviveResult(result) = message else {
            unreachable!();
        };
        Ok(result)
    }

    fn wait_summary(
        &mut self,
        side: Side,
    ) -> Result<CooperationOperationSummary, Box<dyn std::error::Error>> {
        let message = self.wait_for(side, "cooperation terminal summary", |message| {
            matches!(message, ServerMessage::CooperationOperationSummary(_))
        })?;
        let ServerMessage::CooperationOperationSummary(summary) = message else {
            unreachable!();
        };
        Ok(summary)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("revenant cooperation bot failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let game_addr = env::var("REVENANT_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7000".to_owned());
    let account_prefix = env::var("REVENANT_COOPERATION_ACCOUNT_PREFIX")
        .unwrap_or_else(|_| "m28-cooperation-bot".to_owned());
    if account_prefix.is_empty()
        || account_prefix.len() > 24
        || !account_prefix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(
            "cooperation account prefix must be 1-24 ASCII alphanumeric/hyphen bytes".into(),
        );
    }
    let standalone_role = env::var("REVENANT_COOPERATION_STANDALONE_ROLE").unwrap_or_default();
    if !standalone_role.is_empty() {
        if standalone_role != "runner" {
            return Err("standalone cooperation bot role must be runner".into());
        }
        return run_standalone_runner(&game_addr, &format!("{account_prefix}-r"));
    }
    let scenario = Scenario::parse(
        &env::var("REVENANT_COOPERATION_SCENARIO").unwrap_or_else(|_| "success".to_owned()),
    )?;
    let mut pair = ClientPair::connect(&game_addr, &account_prefix)?;
    opt_in_and_clear_drone(&mut pair)?;
    let operation = start_operation(&mut pair, scenario == Scenario::Success)?;
    validate_operation(&pair, &operation)?;

    let summary = match scenario {
        Scenario::Success => run_success(&mut pair)?,
        Scenario::PingTimeout => run_ping_timeout(&mut pair)?,
        Scenario::ReviveTimeout => run_revive_timeout(&mut pair)?,
        Scenario::OperationTimeout => run_operation_timeout(&mut pair)?,
        Scenario::Disconnect { side, phase } => run_disconnect(&mut pair, side, phase)?,
    };
    validate_summary(&pair, scenario, &summary)?;
    println!(
        "M28_COOPERATION_BOT {}",
        serde_json::json!({
            "schema": "RevenantM28CooperationBotV1",
            "scenario": scenario.label(),
            "session_id": summary.session_id,
            "start_operation_id": summary.start_operation_id,
            "outcome": summary.outcome,
            "subject_role": summary.subject_role,
            "terminal_elapsed_ms": summary.terminal_elapsed_ms,
            "anchor_account_id": pair.account_id(Side::Anchor),
            "runner_account_id": pair.account_id(Side::Runner),
            "contributions": summary.contributions,
            "grant_count": summary.grants.len(),
        })
    );
    Ok(())
}

fn run_standalone_runner(
    game_addr: &str,
    username: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = connect_client(game_addr, username)?;
    write_message(
        &mut client.stream,
        &ClientMessage::CooperationStateRequest(CooperationStateRequest {}),
    )?;
    let mut ping_observed = false;
    let mut down_observed = false;
    loop {
        match read_message(&mut client.stream)? {
            ServerMessage::CooperationPingResult(result)
                if result.accepted && !result.replayed && !ping_observed =>
            {
                let ping = result
                    .ping
                    .ok_or("accepted standalone ping omitted truth")?;
                if ping.source_actor_id == client.actor_id || !ping.active {
                    return Err(format!("standalone runner received invalid ping: {ping:?}").into());
                }
                ping_observed = true;
                write_message(
                    &mut client.stream,
                    &ClientMessage::MoveIntent(MoveIntent {
                        position: [4, 0, 3],
                    }),
                )?;
            }
            ServerMessage::CooperationLifeState(life)
                if life.actor_id == client.actor_id
                    && life.life_after == CooperationLife::Downed =>
            {
                if !ping_observed
                    || life.role != CooperationRole::Runner
                    || life.cause != CooperationLifeCause::RelayFeedback
                    || life.health_before == 0
                    || life.health_after != 0
                {
                    return Err(format!("standalone runner downing diverged: {life:?}").into());
                }
                down_observed = true;
            }
            ServerMessage::CooperationOperationSummary(summary) => {
                if !ping_observed
                    || !down_observed
                    || summary.outcome != CooperationTerminalOutcome::Succeeded
                    || summary.participants.len() != 2
                    || summary.participants[Side::Runner.index()].actor_id != client.actor_id
                    || summary.participants[Side::Runner.index()].role != CooperationRole::Runner
                    || summary.grants.len() != 2
                    || summary.grants[Side::Runner.index()].actor_id != client.actor_id
                    || summary.grants[Side::Runner.index()].item_quantity != 2
                    || summary.grants[Side::Runner.index()].experience != 125
                {
                    return Err(
                        format!("standalone runner terminal truth diverged: {summary:?}").into(),
                    );
                }
                println!(
                    "M28_STANDALONE_RUNNER {}",
                    serde_json::json!({
                        "schema": "RevenantM28StandaloneRunnerV1",
                        "session_id": summary.session_id,
                        "actor_id": client.actor_id,
                        "role": "runner",
                        "ping_observed": ping_observed,
                        "down_observed": down_observed,
                        "outcome": summary.outcome,
                        "grant_count": summary.grants.len(),
                    })
                );
                return Ok(());
            }
            _ => {}
        }
    }
}

fn connect_client(game_addr: &str, username: &str) -> Result<Client, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(game_addr)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    write_message(
        &mut stream,
        &ClientMessage::ClientHello(ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "revenant-cooperation-bot".to_owned(),
            client_build: env!("CARGO_PKG_VERSION").to_owned(),
            content_revision: None,
        }),
    )?;
    let ServerMessage::ServerHello(hello) = read_message(&mut stream)? else {
        return Err("expected ServerHello".into());
    };
    if !hello.accepted || hello.protocol_version != PROTOCOL_VERSION {
        return Err(format!("current V2 handshake rejected: {}", hello.message).into());
    }
    write_message(
        &mut stream,
        &ClientMessage::AuthRequest(AuthRequest {
            username: username.to_owned(),
        }),
    )?;
    let ServerMessage::AuthResponse(auth) = read_message(&mut stream)? else {
        return Err("expected AuthResponse".into());
    };
    if !auth.authenticated {
        return Err(format!("local authentication rejected: {}", auth.message).into());
    }
    write_message(
        &mut stream,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )?;
    let ServerMessage::CharacterListResponse(characters) = read_message(&mut stream)? else {
        return Err("expected CharacterListResponse".into());
    };
    let character_id = characters
        .characters
        .first()
        .ok_or("local account has no character")?
        .character_id
        .clone();
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
    for expected in [
        "InventorySnapshot",
        "ProgressionSnapshot",
        "EquipmentSnapshot",
    ] {
        let message: ServerMessage = read_message(&mut stream)?;
        let valid = matches!(
            (expected, message),
            ("InventorySnapshot", ServerMessage::InventorySnapshot(_))
                | ("ProgressionSnapshot", ServerMessage::ProgressionSnapshot(_))
                | ("EquipmentSnapshot", ServerMessage::EquipmentSnapshot(_))
        );
        if !valid {
            return Err(format!("expected initial {expected}").into());
        }
    }
    stream.set_read_timeout(None)?;
    Ok(Client {
        stream,
        actor_id: world.player_actor_id,
        account_id: auth.account_id,
    })
}

fn spawn_reader(
    side: Side,
    stream: &TcpStream,
    sender: Sender<PairEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = stream.try_clone()?;
    thread::spawn(move || {
        while let Ok(message) = read_message(&mut reader) {
            if sender.send(PairEvent { side, message }).is_err() {
                break;
            }
        }
    });
    Ok(())
}

fn opt_in_and_clear_drone(pair: &mut ClientPair) -> Result<(), Box<dyn std::error::Error>> {
    for side in Side::ALL {
        pair.send(
            side,
            ClientMessage::CooperationStateRequest(CooperationStateRequest {}),
        )?;
    }
    for side in Side::ALL {
        let message = pair.wait_for(side, "both cooperation capabilities", |message| {
            matches!(message, ServerMessage::CooperationState(state) if state.all_capable)
        })?;
        let ServerMessage::CooperationState(state) = message else {
            unreachable!();
        };
        if !state.accepted
            || state.capable_actor_ids != [pair.actor_id(Side::Anchor), pair.actor_id(Side::Runner)]
            || state.operation.is_some()
        {
            return Err(format!("capability truth diverged: {state:?}").into());
        }
    }
    let message = pair.wait_for(Side::Anchor, "Relay Drone spawn", |message| {
        matches!(message, ServerMessage::ActorSpawn(actor) if actor.archetype == "relay-drone")
    })?;
    let ServerMessage::ActorSpawn(drone) = message else {
        unreachable!();
    };
    defeat_enemy(pair, drone.actor_id)?;
    for side in Side::ALL {
        let state = pair.wait_state(side, CooperationPhase::Eligible)?;
        if !state.accepted || !state.all_capable || state.operation.is_some() {
            return Err(format!("eligible cooperation state diverged: {state:?}").into());
        }
    }
    Ok(())
}

fn defeat_enemy(pair: &mut ClientPair, enemy_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let anchor_actor_id = pair.actor_id(Side::Anchor);
    loop {
        pair.send(
            Side::Anchor,
            ClientMessage::AttackIntent(AttackIntent {
                target_actor_id: enemy_id,
            }),
        )?;
        let message = pair.wait_for(Side::Anchor, "accepted anchor hit", |message| {
            matches!(message, ServerMessage::DamageApplied(damage) if damage.source_actor_id == anchor_actor_id && damage.target_actor_id == enemy_id)
        })?;
        let ServerMessage::DamageApplied(damage) = message else {
            unreachable!();
        };
        if damage.killed {
            break;
        }
        thread::sleep(ATTACK_INTERVAL);
    }
    for side in Side::ALL {
        let _ = pair.wait_for(side, "enemy destruction", |message| {
            matches!(message, ServerMessage::ActorDestroy(actor) if actor.actor_id == enemy_id)
        })?;
    }
    Ok(())
}

fn start_operation(
    pair: &mut ClientPair,
    probes: bool,
) -> Result<CooperationOperationState, Box<dyn std::error::Error>> {
    if probes {
        pair.send(
            Side::Anchor,
            ClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id: "bad_id".to_owned(),
            }),
        )?;
        let invalid = pair.wait_start(Side::Anchor, "")?;
        if invalid.accepted || !invalid.operation_id.is_empty() || invalid.operation.is_some() {
            return Err(format!("invalid start was not bounded: {invalid:?}").into());
        }
        pair.send(
            Side::Runner,
            ClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id: "runner-start".to_owned(),
            }),
        )?;
        let non_leader = pair.wait_start(Side::Runner, "runner-start")?;
        if non_leader.accepted || non_leader.operation.is_some() {
            return Err(format!("runner start was not rejected: {non_leader:?}").into());
        }
    }
    pair.send(
        Side::Anchor,
        ClientMessage::CooperationStartIntent(CooperationStartIntent {
            operation_id: "bot-start".to_owned(),
        }),
    )?;
    let anchor = pair.wait_start(Side::Anchor, "bot-start")?;
    let runner = pair.wait_start(Side::Runner, "bot-start")?;
    if !anchor.accepted || anchor.replayed || anchor != runner {
        return Err(format!("shared cooperation start diverged: {anchor:?} / {runner:?}").into());
    }
    let operation = anchor
        .operation
        .clone()
        .ok_or("accepted start omitted operation truth")?;
    if probes {
        pair.send(
            Side::Anchor,
            ClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id: "bot-start".to_owned(),
            }),
        )?;
        let replay = pair.wait_start(Side::Anchor, "bot-start")?;
        let mut replay_operation = replay
            .operation
            .clone()
            .ok_or("start retry omitted operation truth")?;
        if replay_operation.observed_elapsed_ms < operation.observed_elapsed_ms {
            return Err(format!("start retry moved elapsed time backwards: {replay:?}").into());
        }
        replay_operation.observed_elapsed_ms = operation.observed_elapsed_ms;
        if !replay.accepted || !replay.replayed || replay_operation != operation {
            return Err(format!("start retry diverged: {replay:?}").into());
        }
        pair.send(
            Side::Anchor,
            ClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id: "second-start".to_owned(),
            }),
        )?;
        let conflict = pair.wait_start(Side::Anchor, "second-start")?;
        if conflict.accepted || conflict.operation.is_some() {
            return Err(format!("second start exposed state: {conflict:?}").into());
        }
    }
    Ok(operation)
}

fn validate_operation(
    pair: &ClientPair,
    operation: &CooperationOperationState,
) -> Result<(), Box<dyn std::error::Error>> {
    if operation.catalog_revision != "m28-v1"
        || operation.start_operation_id != "bot-start"
        || operation.phase != CooperationPhase::AwaitingAnchor
        || operation.participants.len() != 2
        || operation.participants[0].actor_id != pair.actor_id(Side::Anchor)
        || operation.participants[0].role != CooperationRole::Anchor
        || operation.participants[1].actor_id != pair.actor_id(Side::Runner)
        || operation.participants[1].role != CooperationRole::Runner
        || operation.targets.len() != 2
        || operation.timing.operation_duration_ms != 60_000
        || operation.timing.ping_ttl_ms != 5_000
        || operation.timing.revive_window_ms != 15_000
        || operation.timing.revive_channel_ms != 2_000
        || operation.timing.revive_health != 50
        || operation.timing.maximum_distance_squared != 4
        || operation.reward.item_id != "relay_core_fragment"
        || operation.reward.item_quantity != 2
        || operation.reward.experience != 125
        || operation.contributions.anchor_arrived
        || operation.contributions.pinged
        || operation.contributions.runner_arrived
        || operation.contributions.revived
        || operation.contributions.warden_completed
        || operation.contributions.revive_count != 0
    {
        return Err(format!("immutable operation truth diverged: {operation:?}").into());
    }
    Ok(())
}

fn move_player(
    pair: &mut ClientPair,
    side: Side,
    position: [i32; 3],
) -> Result<(), Box<dyn std::error::Error>> {
    let actor_id = pair.actor_id(side);
    pair.send(side, ClientMessage::MoveIntent(MoveIntent { position }))?;
    for recipient in Side::ALL {
        let _ = pair.wait_for(recipient, "shared actor movement", |message| {
            matches!(message, ServerMessage::ActorUpdate(update) if update.actor_id == actor_id && update.position == position)
        })?;
    }
    Ok(())
}

fn arrive_anchor(pair: &mut ClientPair) -> Result<(), Box<dyn std::error::Error>> {
    move_player(pair, Side::Anchor, [3, 0, 3])?;
    for side in Side::ALL {
        let state = pair.wait_state(side, CooperationPhase::AwaitingPing)?;
        if !state
            .operation
            .as_ref()
            .is_some_and(|operation| operation.contributions.anchor_arrived)
        {
            return Err("anchor arrival was not shared authoritatively".into());
        }
    }
    Ok(())
}

fn accept_ping(pair: &mut ClientPair, probes: bool) -> Result<(), Box<dyn std::error::Error>> {
    if probes {
        pair.send(
            Side::Runner,
            ClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id: "runner-ping".to_owned(),
            }),
        )?;
        let wrong_role = pair.wait_ping(Side::Runner, "runner-ping")?;
        if wrong_role.accepted || wrong_role.ping.is_some() {
            return Err(format!("runner ping exposed state: {wrong_role:?}").into());
        }
    }
    pair.send(
        Side::Anchor,
        ClientMessage::CooperationPingIntent(CooperationPingIntent {
            operation_id: "bot-ping".to_owned(),
        }),
    )?;
    let anchor = pair.wait_ping(Side::Anchor, "bot-ping")?;
    let runner = pair.wait_ping(Side::Runner, "bot-ping")?;
    if !anchor.accepted || anchor.replayed || anchor != runner {
        return Err(format!("shared ping truth diverged: {anchor:?} / {runner:?}").into());
    }
    let ping = anchor.ping.clone().ok_or("accepted ping omitted truth")?;
    if ping.source_actor_id != pair.actor_id(Side::Anchor)
        || ping.expires_elapsed_ms != ping.accepted_elapsed_ms + 5_000
        || !ping.active
    {
        return Err(format!("fixed ping truth diverged: {ping:?}").into());
    }
    if probes {
        pair.send(
            Side::Anchor,
            ClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id: "bot-ping".to_owned(),
            }),
        )?;
        let replay = pair.wait_ping(Side::Anchor, "bot-ping")?;
        if !replay.accepted || !replay.replayed || replay.ping.as_ref() != Some(&ping) {
            return Err(format!("ping retry diverged: {replay:?}").into());
        }
        pair.send(
            Side::Anchor,
            ClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id: "second-ping".to_owned(),
            }),
        )?;
        let conflict = pair.wait_ping(Side::Anchor, "second-ping")?;
        if conflict.accepted || conflict.ping.is_some() {
            return Err(format!("second ping exposed state: {conflict:?}").into());
        }
    }
    Ok(())
}

fn down_runner(pair: &mut ClientPair) -> Result<(), Box<dyn std::error::Error>> {
    move_player(pair, Side::Runner, [4, 0, 3])?;
    let runner_actor_id = pair.actor_id(Side::Runner);
    for side in Side::ALL {
        let message = pair.wait_for(side, "shared runner downing", |message| {
            matches!(message, ServerMessage::CooperationLifeState(state) if state.actor_id == runner_actor_id && state.life_after == CooperationLife::Downed)
        })?;
        let ServerMessage::CooperationLifeState(life) = message else {
            unreachable!();
        };
        if life.role != CooperationRole::Runner
            || life.source_actor_id.is_some()
            || life.cause != CooperationLifeCause::RelayFeedback
            || life.health_before == 0
            || life.health_after != 0
            || life.life_before != CooperationLife::Active
        {
            return Err(format!("runner life transition diverged: {life:?}").into());
        }
        let state = pair.wait_state(side, CooperationPhase::RunnerDowned)?;
        if state.operation.as_ref().is_none_or(|operation| {
            operation.participants[Side::Runner.index()].life != CooperationLife::Downed
                || operation.participants[Side::Runner.index()].current_health != 0
        }) {
            return Err("downed runner state was not shared".into());
        }
    }
    Ok(())
}

fn start_revive(
    pair: &mut ClientPair,
    operation_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    pair.send(
        Side::Anchor,
        ClientMessage::CooperationReviveIntent(CooperationReviveIntent {
            operation_id: operation_id.to_owned(),
        }),
    )?;
    let anchor = pair.wait_revive(Side::Anchor, operation_id, CooperationReviveStatus::Started)?;
    let runner = pair.wait_revive(Side::Runner, operation_id, CooperationReviveStatus::Started)?;
    if !anchor.accepted || anchor.replayed || anchor != runner {
        return Err(format!("shared revive start diverged: {anchor:?} / {runner:?}").into());
    }
    let revive = anchor.revive.ok_or("revive start omitted channel truth")?;
    if revive.source_actor_id != pair.actor_id(Side::Anchor)
        || revive.target_actor_id != pair.actor_id(Side::Runner)
        || revive.required_duration_ms != 2_000
        || revive.maximum_distance_squared != 4
        || revive.distance_squared > 4
    {
        return Err(format!("revive channel truth diverged: {revive:?}").into());
    }
    Ok(())
}

fn complete_revive(pair: &mut ClientPair) -> Result<(), Box<dyn std::error::Error>> {
    thread::sleep(REVIVE_COMPLETION_OBSERVATION);
    pair.send(
        Side::Anchor,
        ClientMessage::CooperationStateRequest(CooperationStateRequest {}),
    )?;
    let runner_actor_id = pair.actor_id(Side::Runner);
    let anchor_actor_id = pair.actor_id(Side::Anchor);
    for side in Side::ALL {
        let message = pair.wait_for(side, "shared revive life transition", |message| {
            matches!(message, ServerMessage::CooperationLifeState(state) if state.actor_id == runner_actor_id && state.life_after == CooperationLife::Active)
        })?;
        let ServerMessage::CooperationLifeState(life) = message else {
            unreachable!();
        };
        if life.source_actor_id != Some(anchor_actor_id)
            || life.cause != CooperationLifeCause::Revive
            || life.health_before != 0
            || life.health_after != 50
            || life.life_before != CooperationLife::Downed
        {
            return Err(format!("revive life truth diverged: {life:?}").into());
        }
        let completed =
            pair.wait_revive(side, "bot-revive-two", CooperationReviveStatus::Completed)?;
        if !completed.accepted || completed.replayed {
            return Err(format!("revive completion diverged: {completed:?}").into());
        }
        let state = pair.wait_state(side, CooperationPhase::EncounterActive)?;
        if state.operation.as_ref().is_none_or(|operation| {
            !operation.contributions.revived || operation.contributions.revive_count != 1
        }) {
            return Err("revive contribution was not shared".into());
        }
    }
    Ok(())
}

fn drive_to_runner_downed(
    pair: &mut ClientPair,
    probes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if probes {
        pair.send(
            Side::Anchor,
            ClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id: "early-ping".to_owned(),
            }),
        )?;
        if pair.wait_ping(Side::Anchor, "early-ping")?.accepted {
            return Err("pre-anchor ping was accepted".into());
        }
        move_player(pair, Side::Runner, [4, 0, 3])?;
    }
    arrive_anchor(pair)?;
    accept_ping(pair, probes)?;
    down_runner(pair)
}

fn drive_to_encounter(
    pair: &mut ClientPair,
    probes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    drive_to_runner_downed(pair, probes)?;
    if probes {
        pair.send(
            Side::Runner,
            ClientMessage::CooperationReviveIntent(CooperationReviveIntent {
                operation_id: "runner-revive".to_owned(),
            }),
        )?;
        if pair
            .wait_revive(
                Side::Runner,
                "runner-revive",
                CooperationReviveStatus::Rejected,
            )?
            .accepted
        {
            return Err("runner revive was accepted".into());
        }
        start_revive(pair, "bot-revive-one")?;
        pair.send(
            Side::Anchor,
            ClientMessage::CooperationReviveIntent(CooperationReviveIntent {
                operation_id: "bot-revive-one".to_owned(),
            }),
        )?;
        let pending = pair.wait_revive(
            Side::Anchor,
            "bot-revive-one",
            CooperationReviveStatus::Pending,
        )?;
        if !pending.accepted || !pending.replayed {
            return Err(format!("pending revive retry diverged: {pending:?}").into());
        }
        pair.send(
            Side::Anchor,
            ClientMessage::MoveIntent(MoveIntent {
                position: [0, 0, 0],
            }),
        )?;
        let anchor_actor_id = pair.actor_id(Side::Anchor);
        for side in Side::ALL {
            let cancelled =
                pair.wait_revive(side, "bot-revive-one", CooperationReviveStatus::Cancelled)?;
            if !cancelled.accepted || cancelled.replayed {
                return Err(format!("cancelled revive diverged: {cancelled:?}").into());
            }
            let _ = pair.wait_for(side, "post-cancel anchor movement", |message| {
                matches!(message, ServerMessage::ActorUpdate(update) if update.actor_id == anchor_actor_id && update.position == [0, 0, 0])
            })?;
        }
        move_player(pair, Side::Anchor, [3, 0, 3])?;
    }
    start_revive(pair, "bot-revive-two")?;
    complete_revive(pair)
}

fn run_success(
    pair: &mut ClientPair,
) -> Result<CooperationOperationSummary, Box<dyn std::error::Error>> {
    drive_to_encounter(pair, true)?;
    pair.send(
        Side::Anchor,
        ClientMessage::CooperationReviveIntent(CooperationReviveIntent {
            operation_id: "second-revive".to_owned(),
        }),
    )?;
    if pair
        .wait_revive(
            Side::Anchor,
            "second-revive",
            CooperationReviveStatus::Rejected,
        )?
        .accepted
    {
        return Err("second revive was accepted".into());
    }
    pair.send(
        Side::Anchor,
        ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
            operation_id: "route-after-cooperation".to_owned(),
            route_id: "breach".to_owned(),
        }),
    )?;
    let route = pair.wait_for(Side::Anchor, "route exclusion", |message| {
        matches!(message, ServerMessage::RouteChoiceResult(_))
    })?;
    let ServerMessage::RouteChoiceResult(route) = route else {
        unreachable!();
    };
    if route.accepted || route.selection.is_some() {
        return Err(format!("route combined with cooperation: {route:?}").into());
    }
    move_player(pair, Side::Anchor, [6, 0, 0])?;
    let message = pair.wait_for(Side::Anchor, "Warden spawn", |message| {
        matches!(message, ServerMessage::ActorSpawn(actor) if actor.archetype == "warden")
    })?;
    let ServerMessage::ActorSpawn(warden) = message else {
        unreachable!();
    };
    defeat_enemy(pair, warden.actor_id)?;
    let anchor = pair.wait_summary(Side::Anchor)?;
    let runner = pair.wait_summary(Side::Runner)?;
    if anchor != runner {
        return Err("two clients received different success summaries".into());
    }
    for side in Side::ALL {
        let loot = pair.wait_for(side, "cooperation loot", |message| {
            matches!(message, ServerMessage::LootGranted(grant) if grant.item_id == "relay_core_fragment" && grant.quantity == 2)
        })?;
        let progression = pair.wait_for(side, "cooperation progression", |message| {
            matches!(message, ServerMessage::ProgressionGranted(grant) if grant.experience_granted == 125)
        })?;
        let completion = pair.wait_for(side, "generic cooperation completion", |message| {
            matches!(message, ServerMessage::ActivityComplete(activity) if activity.activity_id == "relay_awakening")
        })?;
        let _ = (loot, progression, completion);
    }
    Ok(anchor)
}

fn observe_timeout(
    pair: &mut ClientPair,
    duration: Duration,
) -> Result<CooperationOperationSummary, Box<dyn std::error::Error>> {
    thread::sleep(duration);
    pair.send(
        Side::Anchor,
        ClientMessage::CooperationStateRequest(CooperationStateRequest {}),
    )?;
    let anchor = pair.wait_summary(Side::Anchor)?;
    let runner = pair.wait_summary(Side::Runner)?;
    if anchor != runner {
        return Err("two clients received different timeout summaries".into());
    }
    Ok(anchor)
}

fn run_ping_timeout(
    pair: &mut ClientPair,
) -> Result<CooperationOperationSummary, Box<dyn std::error::Error>> {
    arrive_anchor(pair)?;
    accept_ping(pair, false)?;
    observe_timeout(pair, PING_TIMEOUT_OBSERVATION)
}

fn run_revive_timeout(
    pair: &mut ClientPair,
) -> Result<CooperationOperationSummary, Box<dyn std::error::Error>> {
    drive_to_runner_downed(pair, false)?;
    observe_timeout(pair, REVIVE_TIMEOUT_OBSERVATION)
}

fn run_operation_timeout(
    pair: &mut ClientPair,
) -> Result<CooperationOperationSummary, Box<dyn std::error::Error>> {
    observe_timeout(pair, OPERATION_TIMEOUT_OBSERVATION)
}

fn run_disconnect(
    pair: &mut ClientPair,
    disconnected: Side,
    phase: DisconnectPhase,
) -> Result<CooperationOperationSummary, Box<dyn std::error::Error>> {
    match phase {
        DisconnectPhase::AwaitingAnchor => {}
        DisconnectPhase::AwaitingPing => arrive_anchor(pair)?,
        DisconnectPhase::AwaitingRunner => {
            arrive_anchor(pair)?;
            accept_ping(pair, false)?;
        }
        DisconnectPhase::RunnerDowned => drive_to_runner_downed(pair, false)?,
        DisconnectPhase::ReviveChannel => {
            drive_to_runner_downed(pair, false)?;
            start_revive(pair, "bot-revive-two")?;
        }
        DisconnectPhase::EncounterActive => drive_to_encounter(pair, false)?,
    }
    pair.disconnect(disconnected)?;
    pair.wait_summary(disconnected.peer())
}

fn validate_summary(
    pair: &ClientPair,
    scenario: Scenario,
    summary: &CooperationOperationSummary,
) -> Result<(), Box<dyn std::error::Error>> {
    if summary.catalog_revision != "m28-v1"
        || summary.start_operation_id != "bot-start"
        || summary.participants.len() != 2
        || summary.participants[0].actor_id != pair.actor_id(Side::Anchor)
        || summary.participants[0].role != Side::Anchor.role()
        || summary.participants[1].actor_id != pair.actor_id(Side::Runner)
        || summary.participants[1].role != Side::Runner.role()
    {
        return Err(format!("terminal identity truth diverged: {summary:?}").into());
    }
    let (outcome, subject, reason) = match scenario {
        Scenario::Success => (CooperationTerminalOutcome::Succeeded, None, None),
        Scenario::PingTimeout => (
            CooperationTerminalOutcome::FailedPingTimeout,
            None,
            Some(CooperationNoRewardReason::PingTimeout),
        ),
        Scenario::ReviveTimeout => (
            CooperationTerminalOutcome::FailedReviveTimeout,
            None,
            Some(CooperationNoRewardReason::ReviveTimeout),
        ),
        Scenario::OperationTimeout => (
            CooperationTerminalOutcome::FailedOperationTimeout,
            None,
            Some(CooperationNoRewardReason::OperationTimeout),
        ),
        Scenario::Disconnect { side, .. } => (
            CooperationTerminalOutcome::AbandonedDisconnect,
            Some(side.role()),
            Some(CooperationNoRewardReason::ParticipantDisconnected),
        ),
    };
    if summary.outcome != outcome
        || summary.subject_role != subject
        || summary.no_reward_reason != reason
    {
        return Err(format!("terminal outcome truth diverged: {summary:?}").into());
    }
    if scenario == Scenario::Success {
        if summary.reward.as_ref().is_none_or(|reward| {
            reward.item_id != "relay_core_fragment"
                || reward.item_quantity != 2
                || reward.experience != 125
        }) || summary.grants.len() != 2
            || summary.grants.iter().enumerate().any(|(index, grant)| {
                grant.actor_id != pair.clients[index].actor_id
                    || grant.item_id != "relay_core_fragment"
                    || grant.item_quantity != 2
                    || grant.experience != 125
            })
            || !summary.contributions.anchor_arrived
            || !summary.contributions.pinged
            || !summary.contributions.runner_arrived
            || !summary.contributions.revived
            || !summary.contributions.warden_completed
            || summary.contributions.revive_count != 1
        {
            return Err(format!("success reward/contribution truth diverged: {summary:?}").into());
        }
    } else if summary.reward.is_some() || !summary.grants.is_empty() {
        return Err(format!("failure exposed a reward: {summary:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DisconnectPhase, Scenario, Side};

    #[test]
    fn scenario_parser_is_finite_and_exact() {
        assert_eq!(Scenario::parse("").unwrap(), Scenario::Success);
        assert_eq!(Scenario::parse("success").unwrap(), Scenario::Success);
        assert_eq!(
            Scenario::parse("ping-timeout").unwrap(),
            Scenario::PingTimeout
        );
        assert_eq!(
            Scenario::parse("revive-timeout").unwrap(),
            Scenario::ReviveTimeout
        );
        assert_eq!(
            Scenario::parse("operation-timeout").unwrap(),
            Scenario::OperationTimeout
        );
        for side in Side::ALL {
            for phase in [
                DisconnectPhase::AwaitingAnchor,
                DisconnectPhase::AwaitingPing,
                DisconnectPhase::AwaitingRunner,
                DisconnectPhase::RunnerDowned,
                DisconnectPhase::ReviveChannel,
                DisconnectPhase::EncounterActive,
            ] {
                let label = format!("disconnect-{}-{}", side.label(), phase.label());
                assert_eq!(
                    Scenario::parse(&label).unwrap(),
                    Scenario::Disconnect { side, phase }
                );
            }
        }
        for invalid in [
            "timeout",
            "disconnect-peer-awaiting-anchor",
            "disconnect-anchor-waiting",
            "disconnect-runner-succeeded",
        ] {
            assert!(Scenario::parse(invalid).is_err(), "{invalid}");
        }
    }
}
