use std::{
    env,
    net::{TcpListener, TcpStream},
    sync::{atomic::AtomicUsize, mpsc, Arc},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use revenant_challenges::{Command, ContractId, Equipment, Objective};
use revenant_persistence::{
    ChallengeRequest, ModuleJoinReplayContext, ModuleMutationReplayContext, Persistence,
};
use revenant_protocol::{
    read_message, write_message, AuthRequest, ChallengeSnapshot, ChallengeStateRequest,
    CharacterListRequest, ClientHello, ClientMessage, ServerMessage, CAMPAIGN_CONTENT_REVISION,
};
use revenant_replay::ReplayProtocolGeneration;

fn saved_character(url: &str, username: &str) -> String {
    let mut db = Persistence::connect(url).unwrap();
    let account = format!("local:{username}");
    let character = format!("{account}:operator");
    let session = format!("board-{username}");
    db.ensure_local_account(&account, username).unwrap();
    db.append_player_joined_with_module_snapshot(
        &character,
        ModuleJoinReplayContext {
            catalog_revision: Some("m35-v2"),
            session_id: &session,
            account_id: &account,
            activity_id: ContractId::MeridianCircuit.activity_id(),
            actor_id: 1,
            player_joined_payload: "player joined",
            protocol_generation: ReplayProtocolGeneration::V2,
        },
    )
    .unwrap();
    let context = ModuleMutationReplayContext {
        catalog_revision: Some("m35-v2"),
        session_id: &session,
        account_id: &account,
        activity_id: ContractId::MeridianCircuit.activity_id(),
        actor_id: 1,
    };
    db.apply_challenge_command(
        &character,
        &ChallengeRequest {
            operation_id: "start",
            expected_revision: 0,
            command: Command::Start {
                contract: ContractId::MeridianCircuit,
                equipment: Equipment {
                    catalog_revision: "m35-v2".into(),
                    loadout_revision: 0,
                    weapon_id: "pulse_rifle".into(),
                    modules: vec![],
                },
            },
            proof_event_id: None,
            position: None,
        },
        context,
    )
    .unwrap();
    for (index, objective) in [
        Objective::LensRead,
        Objective::GalleryRead,
        Objective::LogRead,
        Objective::Returned,
    ]
    .into_iter()
    .enumerate()
    {
        db.apply_challenge_command(
            &character,
            &ChallengeRequest {
                operation_id: &format!("visit-{index}"),
                expected_revision: u64::try_from(index).unwrap() + 1,
                command: Command::Confirm {
                    run_id: 1,
                    objective,
                },
                proof_event_id: None,
                position: objective.position(),
            },
            context,
        )
        .unwrap();
    }
    character
}

fn request_board(
    url: &str,
    username: &str,
    character: &str,
    version: u16,
    content: &str,
    forged: bool,
) -> Option<ChallengeSnapshot> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let (server, _) = listener.accept().unwrap();
    let url = url.to_owned();
    let (tx, commands) = mpsc::sync_channel(1);
    let handle = thread::spawn(move || {
        let counter = Arc::new(AtomicUsize::new(1));
        let unauthenticated = crate::CounterGuard::new(counter);
        let connection = Arc::new(crate::CounterGuard::new(Arc::new(AtomicUsize::new(1))));
        let correlation = crate::CorrelationDigest::generate().unwrap();
        crate::handle_game_connection(
            server,
            &url,
            &tx,
            unauthenticated,
            &correlation,
            &connection,
        )
        .is_err()
    });
    write_message(
        &mut peer,
        &ClientMessage::ClientHello(ClientHello {
            protocol_version: version,
            client_name: "challenge-board-test".into(),
            client_build: "m37-authority-test".into(),
            content_revision: Some(content.into()),
        }),
    )
    .unwrap();
    assert!(matches!(
        read_message::<_, ServerMessage>(&mut peer).unwrap(),
        ServerMessage::ServerHello(_)
    ));
    write_message(
        &mut peer,
        &ClientMessage::AuthRequest(AuthRequest {
            username: username.into(),
        }),
    )
    .unwrap();
    assert!(
        matches!(read_message::<_,ServerMessage>(&mut peer).unwrap(), ServerMessage::AuthResponse(r) if r.authenticated)
    );
    write_message(
        &mut peer,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )
    .unwrap();
    assert!(matches!(
        read_message::<_, ServerMessage>(&mut peer).unwrap(),
        ServerMessage::CharacterListResponse(_)
    ));
    if forged {
        write_message(
            &mut peer,
            &serde_json::json!({"type": "ChallengeStateRequest", "character_id": character, "completed": true}),
        )
        .unwrap();
    } else {
        write_message(
            &mut peer,
            &ClientMessage::ChallengeStateRequest(ChallengeStateRequest {
                character_id: character.into(),
            }),
        )
        .unwrap();
    }
    let response = read_message::<_, ServerMessage>(&mut peer).ok();
    drop(peer);
    assert!(handle.join().unwrap()); // Query-only clients close before world entry.
    assert!(commands.try_recv().is_err());
    match response {
        Some(ServerMessage::ChallengeSnapshot(board)) => Some(board),
        _ => None,
    }
}

#[test]
fn challenge_board_tcp_uses_authenticated_save_and_rejects_foreign_legacy_and_forged_queries() {
    let Ok(url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL challenge board test skipped");
        return;
    };
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let username = format!("m37-board-{id}");
    let character = saved_character(&url, &username);
    let board = request_board(
        &url,
        &username,
        &character,
        2,
        CAMPAIGN_CONTENT_REVISION,
        false,
    )
    .unwrap();
    assert_eq!(board.character_id, character);
    assert_eq!(board.state_revision, 5);
    assert_eq!(board.contracts.len(), 2);
    assert_eq!(board.records.len(), 1);
    assert_eq!(board.records[0].contract.contract_id, "meridian_circuit");
    assert_eq!(board.last_result.unwrap().outcome, "completed");
    assert!(request_board(
        &url,
        &username,
        "local:foreign:operator",
        2,
        CAMPAIGN_CONTENT_REVISION,
        false
    )
    .is_none());
    assert!(request_board(&url, &username, &character, 2, "m35-v2", false).is_none());
    assert_mastery_archive(&url, &username, &character);
    for content in [
        revenant_protocol::RECOVERY_CONTENT_REVISION,
        revenant_protocol::ELITE_CHALLENGE_CONTENT_REVISION,
        revenant_protocol::PRISM_CHALLENGE_CONTENT_REVISION,
    ] {
        assert!(request_board(&url, &username, &character, 1, content, false).is_none());
    }
    for (content, policy, count) in [
        (
            revenant_protocol::PRISM_CHALLENGE_CONTENT_REVISION,
            revenant_challenges::PRISM_REVISION,
            5,
        ),
        (
            revenant_protocol::SIGNAL_CHALLENGE_CONTENT_REVISION,
            revenant_challenges::SIGNAL_REVISION,
            6,
        ),
        (
            revenant_protocol::SURVIVAL_CHALLENGE_CONTENT_REVISION,
            revenant_challenges::SURVIVAL_REVISION,
            7,
        ),
        (
            revenant_protocol::GAUNTLET_CHALLENGE_CONTENT_REVISION,
            revenant_challenges::GAUNTLET_REVISION,
            8,
        ),
    ] {
        let board = request_board(&url, &username, &character, 2, content, false).unwrap();
        assert_eq!(board.contracts.len(), count);
        assert_eq!(board.policy_revision, policy);
        assert!(request_board(&url, &username, &character, 1, content, false).is_none());
    }

    assert!(request_board(
        &url,
        &username,
        &character,
        1,
        CAMPAIGN_CONTENT_REVISION,
        false
    )
    .is_none());
    assert!(request_board(
        &url,
        &username,
        &character,
        2,
        CAMPAIGN_CONTENT_REVISION,
        true
    )
    .is_none());
}

fn assert_mastery_archive(url: &str, username: &str, character: &str) {
    let old = request_board(
        url,
        username,
        character,
        2,
        revenant_protocol::MODIFIER_CHALLENGE_CONTENT_REVISION,
        false,
    )
    .unwrap();
    assert!(old.mastery.is_none());
    let expanded = request_board(
        url,
        username,
        character,
        2,
        revenant_protocol::MASTERY_CHALLENGE_CONTENT_REVISION,
        false,
    )
    .unwrap();
    assert_eq!(expanded.policy_revision, old.policy_revision);
    assert_eq!(expanded.records, old.records);
    let archive = expanded.mastery.unwrap();
    assert_eq!(archive.revision, revenant_challenges::mastery::REVISION);
    assert!(archive.records.is_empty() && archive.badges.is_empty());
    assert_eq!(
        archive.last_attempt.unwrap().assessments[0].reason,
        "use_west_route"
    );
    assert!(request_board(
        url,
        username,
        character,
        1,
        revenant_protocol::MASTERY_CHALLENGE_CONTENT_REVISION,
        false
    )
    .is_none());
}
