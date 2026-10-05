use std::{
    collections::{BTreeMap, VecDeque},
    env,
    net::{Shutdown, TcpListener, TcpStream},
    sync::{atomic::AtomicUsize, mpsc, Arc},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use revenant_challenges::ContractId;
use revenant_persistence::Persistence;
use revenant_protocol::{
    read_message, write_message, AttackIntent, AuthRequest, ChallengeAbandonIntent,
    ChallengeJoinRequest, ChallengeSnapshot, ChallengeStateRequest, CharacterListRequest,
    ClientHello, ClientMessage, MoveIntent, ServerMessage, CHALLENGE_CONTENT_REVISION,
};

use crate::{CorrelationDigest, CounterGuard, SharedSession};

struct Peer {
    stream: TcpStream,
    handler: Option<JoinHandle<()>>,
    world: Option<JoinHandle<()>>,
    player: u64,
    position: [i32; 3],
    enemies: BTreeMap<String, u64>,
    state: Option<ChallengeSnapshot>,
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
        if let Some(handle) = self.handler.take() {
            handle.join().unwrap();
        }
        if let Some(handle) = self.world.take() {
            handle.join().unwrap();
        }
    }
}

impl Peer {
    fn enter(url: &str, username: &str, contract: ContractId, retry: Option<u64>) -> Self {
        let session =
            SharedSession::new(url, "../../scripts/activities/relay_awakening.lua", 1).unwrap();
        let (tx, rx) = mpsc::sync_channel(16);
        let world = thread::spawn(move || session.run(rx));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        stream.set_nodelay(true).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let (server, _) = listener.accept().unwrap();
        server.set_nodelay(true).unwrap();
        let url = url.to_owned();
        let handler = thread::spawn(move || {
            let unauthenticated = CounterGuard::new(Arc::new(AtomicUsize::new(1)));
            let connection = Arc::new(CounterGuard::new(Arc::new(AtomicUsize::new(1))));
            let _ = crate::handle_game_connection(
                server,
                &url,
                &tx,
                unauthenticated,
                &CorrelationDigest::generate().unwrap(),
                &connection,
            );
        });
        let mut peer = Self {
            stream,
            handler: Some(handler),
            world: Some(world),
            player: 0,
            position: [0, 0, 0],
            enemies: BTreeMap::new(),
            state: None,
        };
        let (content, count) = match contract {
            ContractId::RelayGauntlet => {
                (revenant_protocol::GAUNTLET_CHALLENGE_CONTENT_REVISION, 8)
            }
            ContractId::LastReserve => (revenant_protocol::SURVIVAL_CHALLENGE_CONTENT_REVISION, 7),
            ContractId::BastionLink => (revenant_protocol::ELITE_CHALLENGE_CONTENT_REVISION, 4),
            ContractId::CoolantRecovery => (revenant_protocol::RECOVERY_CONTENT_REVISION, 3),
            _ => (CHALLENGE_CONTENT_REVISION, 2),
        };
        peer.send(&ClientMessage::ClientHello(ClientHello {
            protocol_version: 2,
            client_name: "challenge-tcp".into(),
            client_build: "m37-test".into(),
            content_revision: Some(content.into()),
        }));
        assert!(
            matches!(peer.read(),ServerMessage::ServerHello(h) if h.content_revision.as_deref()==Some(content))
        );
        peer.send(&ClientMessage::AuthRequest(AuthRequest {
            username: username.into(),
        }));
        assert!(matches!(peer.read(),ServerMessage::AuthResponse(a) if a.authenticated));
        peer.send(&ClientMessage::CharacterListRequest(
            CharacterListRequest {},
        ));
        let ServerMessage::CharacterListResponse(list) = peer.read() else {
            panic!("missing character list");
        };
        let character = list.characters[0].character_id.clone();
        peer.send(&ClientMessage::ChallengeStateRequest(
            ChallengeStateRequest {
                character_id: character.clone(),
            },
        ));
        let ServerMessage::ChallengeSnapshot(board) = peer.read() else {
            panic!("missing board");
        };
        assert_eq!(board.contracts.len(), count);
        peer.send(&ClientMessage::ChallengeJoinRequest(ChallengeJoinRequest {
            preset_id: None,
            character_id: character,
            operation_id: format!("join-{}", board.state_revision),
            expected_revision: board.state_revision,
            contract_id: contract.as_str().into(),
            retry_run_id: retry,
        }));
        let ServerMessage::WorldJoinResponse(join) = peer.read() else {
            panic!("missing world join");
        };
        assert!(join.accepted, "{}", join.message);
        peer.player = join.player_actor_id;
        peer.position = join.spawn_position;
        peer.until(|m| matches!(m,ServerMessage::ChallengeSnapshot(s) if s.active.is_some()));
        peer
    }

    fn send(&mut self, message: &ClientMessage) {
        // Match a human client's input cadence without bypassing frame limits.
        thread::sleep(Duration::from_millis(70));
        write_message(&mut self.stream, message).unwrap();
    }

    fn read(&mut self) -> ServerMessage {
        let message: ServerMessage = read_message(&mut self.stream).unwrap();
        assert!(!matches!(
            message,
            ServerMessage::LootGranted(_) | ServerMessage::ProgressionGranted(_)
        ));
        match &message {
            ServerMessage::ActorSpawn(actor) if actor.actor_kind == "enemy" => {
                self.enemies.insert(actor.archetype.clone(), actor.actor_id);
            }
            ServerMessage::ChallengeSnapshot(state) => self.state = Some(state.clone()),
            ServerMessage::ActorUpdate(actor) if actor.actor_id == self.player => {
                self.position = actor.position;
            }
            _ => {}
        }
        message
    }

    fn until(&mut self, predicate: impl Fn(&ServerMessage) -> bool) -> ServerMessage {
        for _ in 0..512 {
            let message = self.read();
            if predicate(&message) {
                return message;
            }
        }
        panic!("expected challenge message was not received");
    }

    fn step(&mut self, position: [i32; 3]) {
        self.send(&ClientMessage::MoveIntent(MoveIntent { position }));
        let player = self.player;
        self.until(|m|matches!(m,ServerMessage::ActorUpdate(a) if a.actor_id==player && a.position==position));
    }

    fn walk(&mut self, target: [i32; 3]) {
        let start = self.position;
        let mut queue = VecDeque::from([start]);
        let mut previous = BTreeMap::from([(start, start)]);
        while let Some(at) = queue.pop_front() {
            if at == target {
                break;
            }
            for delta in [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]] {
                let next = [at[0] + delta[0], 0, at[2] + delta[2]];
                if !previous.contains_key(&next)
                    && revenant_activities::meridian::walkable(next)
                    && revenant_activities::meridian::valid_step(at, next)
                {
                    previous.insert(next, at);
                    queue.push_back(next);
                }
            }
        }
        let mut at = target;
        let mut steps = vec![];
        while at != start {
            steps.push(at);
            at = previous[&at];
        }
        for point in steps.into_iter().rev() {
            self.step(point);
        }
    }
}

#[test]
fn challenge_tcp_completes_both_contracts_and_abandon_retry_preserves_identity() {
    let Ok(url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; challenge TCP test skipped");
        return;
    };
    Persistence::connect(&url).unwrap();
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let username = format!("m37-tcp-{id}");
    let mut peer = Peer::enter(&url, &username, ContractId::MeridianCircuit, None);
    peer.walk([-28, 0, -7]);
    peer.until(|m| matches!(m,ServerMessage::ChallengeSnapshot(s) if s.state_revision==2));
    let intent = ChallengeAbandonIntent {
        operation_id: "abandon".into(),
        expected_revision: 2,
        run_id: 1,
    };
    peer.send(&ClientMessage::ChallengeAbandonIntent(intent.clone()));
    peer.until(|m| matches!(m,ServerMessage::ChallengeActionResult(r) if r.status=="accepted"));
    peer.send(&ClientMessage::ChallengeAbandonIntent(intent));
    peer.until(|m| matches!(m,ServerMessage::ChallengeActionResult(r) if r.status=="replayed"));
    drop(peer);
    let mut peer = Peer::enter(&url, &username, ContractId::MeridianCircuit, Some(1));
    assert_eq!(
        peer.state
            .as_ref()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .objectives,
        0
    );
    for target in [[-28, 0, 7], [-30, 0, 0], [-28, 0, -7], [-16, 0, 0]] {
        peer.walk(target);
    }
    peer.until(|m| matches!(m,ServerMessage::ChallengeSnapshot(s) if s.active.is_none()));
    assert_eq!(peer.state.as_ref().unwrap().records.len(), 1);
    drop(peer);
    let mut peer = Peer::enter(&url, &username, ContractId::CloseQuarters, None);
    for point in [[3, 0, 6], [4, 0, 6], [5, 0, 6], [5, 0, 7], [5, 0, 8]] {
        peer.step(point);
    }
    for archetype in ["relay-mender", "glass-lancer"] {
        let target = peer.enemies[archetype];
        loop {
            peer.send(&ClientMessage::AttackIntent(AttackIntent {
                target_actor_id: target,
            }));
            let message = peer.until(
                |m| matches!(m,ServerMessage::DamageApplied(d) if d.target_actor_id==target),
            );
            if matches!(message,ServerMessage::DamageApplied(d) if d.killed) {
                break;
            }
            thread::sleep(Duration::from_millis(270));
        }
        thread::sleep(Duration::from_millis(270));
    }
    peer.until(|m| matches!(m,ServerMessage::ChallengeSnapshot(s) if s.active.is_none()));
    assert_eq!(peer.state.as_ref().unwrap().records.len(), 2);
    drop(peer);
    let mut db = Persistence::connect_existing(&url).unwrap();
    let character = format!("local:{username}:operator");
    assert_eq!(
        db.progression_for(&character).unwrap().unwrap().experience,
        0
    );
}

#[test]
fn challenge_tcp_recovery_confirms_real_server_holds_under_expanded_content() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    Persistence::connect(&url).unwrap();
    let username = format!(
        "m37-r-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut peer = Peer::enter(&url, &username, ContractId::CoolantRecovery, None);
    assert_eq!(
        peer.state.as_ref().unwrap().policy_revision,
        revenant_challenges::EXPANDED_REVISION
    );
    assert_eq!(peer.state.as_ref().unwrap().contracts.len(), 3);
    for position in [
        [-1, 0, -4],
        [-1, 0, -5],
        [0, 0, -5],
        [1, 0, -5],
        [2, 0, -5],
        [3, 0, -5],
        [4, 0, -5],
    ] {
        peer.step(position);
    }
    peer.until(|message| matches!(message, ServerMessage::ChallengeSnapshot(board) if board.active.as_ref().is_some_and(|run| run.objectives == 3)));
    for position in [[4, 0, -4], [4, 0, -3], [4, 0, -2], [4, 0, -1]] {
        peer.step(position);
    }
    peer.until(|message| matches!(message, ServerMessage::ChallengeSnapshot(board) if board.active.is_none()));
    assert_eq!(peer.state.as_ref().unwrap().records.len(), 1);
    assert_eq!(
        peer.state
            .as_ref()
            .unwrap()
            .last_result
            .as_ref()
            .unwrap()
            .outcome,
        "completed"
    );
    drop(peer);
    let mut db = Persistence::connect_existing(&url).unwrap();
    assert_eq!(
        db.progression_for(&format!("local:{username}:operator"))
            .unwrap()
            .unwrap()
            .experience,
        0
    );
}

#[test]
fn challenge_tcp_elite_content_completes_bastion_and_preserves_its_record() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    Persistence::connect(&url).unwrap();
    let username = format!(
        "m37-e-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut peer = Peer::enter(&url, &username, ContractId::BastionLink, None);
    assert_eq!(
        peer.state.as_ref().unwrap().policy_revision,
        revenant_challenges::ELITE_REVISION
    );
    for position in [
        [6, 0, -6],
        [7, 0, -6],
        [8, 0, -6],
        [9, 0, -6],
        [10, 0, -6],
        [10, 0, -7],
        [10, 0, -8],
    ] {
        peer.step(position);
    }
    for archetype in ["relay-mender", "steel-bulwark"] {
        let target = peer.enemies[archetype];
        let mut killed = false;
        for _ in 0..24 {
            peer.send(&ClientMessage::AttackIntent(AttackIntent {
                target_actor_id: target,
            }));
            let ServerMessage::DamageApplied(hit) = peer.until(
                |m| matches!(m, ServerMessage::DamageApplied(d) if d.target_actor_id == target),
            ) else {
                unreachable!()
            };
            if hit.killed {
                killed = true;
                break;
            }
            if hit.damage == 0 {
                let flank = if peer.position == [10, 0, -8] {
                    [8, 0, -6]
                } else {
                    [10, 0, -8]
                };
                while peer.position != flank {
                    let mut next = peer.position;
                    let axis = if next[0] == flank[0] { 2 } else { 0 };
                    next[axis] += (flank[axis] - next[axis]).signum();
                    peer.step(next);
                }
            }
            thread::sleep(Duration::from_millis(270));
        }
        assert!(killed, "elite target did not fall: {archetype}");
        thread::sleep(Duration::from_millis(270));
    }
    peer.until(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.active.is_none()));
    let first = peer.state.as_ref().unwrap().records[0].clone();
    assert_eq!(first.contract.contract_id, "bastion_link");
    assert_eq!(first.contract.seed, 37004);
    drop(peer);
    let peer = Peer::enter(&url, &username, ContractId::BastionLink, None);
    assert_eq!(peer.state.as_ref().unwrap().records[0], first);
    assert_eq!(
        peer.state
            .as_ref()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .objectives,
        0
    );
    drop(peer);
    let mut db = Persistence::connect_existing(&url).unwrap();
    assert_eq!(
        db.progression_for(&format!("local:{username}:operator"))
            .unwrap()
            .unwrap()
            .experience,
        0
    );
}

#[path = "challenge_survival_tcp.rs"]
mod survival;

#[path = "challenge_gauntlet_tcp.rs"]
mod gauntlet;
