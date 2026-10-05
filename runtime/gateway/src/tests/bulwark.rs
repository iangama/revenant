use super::*;
use revenant_ai::bulwark::{ENTRANCE, HEALTH};

const PLAYER: u64 = 2_034;
type Fixture = (
    SharedSession,
    Receiver<ServerMessage>,
    Arc<Mutex<Vec<String>>>,
);

fn setup(capable: bool) -> Fixture {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (mut participant, receiver) =
        fixture_participant("local:bulwark-test", PLAYER, &["pulse_rifle"]);
    participant.encounter_capable = true;
    participant.bulwark_capable = capable;
    let persistence = FixturePersistence::new(None, calls.clone(), Vec::new());
    let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
    start_and_clear_drone(&mut session, PLAYER);
    drain_messages(&receiver);
    (session, receiver, calls)
}

fn fail_once(session: &mut SharedSession, calls: &Arc<Mutex<Vec<String>>>, event: &str) {
    session.persistence = Box::new(FixturePersistence::new(
        Some(event),
        calls.clone(),
        Vec::new(),
    ));
}

#[test]
fn bulwark_blocks_front_allows_flank_exposes_recovery_and_retains_mission_rewards() {
    let (mut session, receiver, calls) = setup(true);
    session.move_player(PLAYER, ENTRANCE).unwrap();
    let enemy = session.enemy_id.unwrap();
    assert!(drain_messages(&receiver).iter().any(|message| matches!(message, ServerMessage::ActorUpdate(update) if update.defense.as_ref().is_some_and(|cue| cue.braced && cue.facing == [-1, 0]))));
    session.attack_at(PLAYER, enemy, 0).unwrap();
    assert_eq!(session.actors.get(enemy).unwrap().health, HEALTH);
    assert!(session.attack_at(PLAYER, enemy, 100).is_err());
    session.move_player(PLAYER, [8, 0, -6]).unwrap();
    session.attack_at(PLAYER, enemy, 250).unwrap();
    assert_eq!(session.actors.get(enemy).unwrap().health, HEALTH - 40);
    let health = session.actors.get(PLAYER).unwrap().health;
    session.tick_bulwark_at(1600).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health);
    session.move_player(PLAYER, ENTRANCE).unwrap();
    session.attack_at(PLAYER, enemy, 1850).unwrap();
    assert_eq!(session.actors.get(enemy).unwrap().health, HEALTH - 80);
    session.tick_bulwark_at(3400).unwrap();
    session.tick_bulwark_at(5000).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health - 14);
    for hit in 0..4 {
        session.attack_at(PLAYER, enemy, 5100 + hit * 250).unwrap();
    }
    assert!(!session.bulwark_fighting());
    session.move_player(PLAYER, [6, 0, 0]).unwrap();
    defeat_active_warden(&mut session, PLAYER);
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| *call == "completion_transaction")
            .count(),
        1
    );
    session.disconnect(PLAYER).unwrap();
    assert!(session.bulwark.is_none());
}

#[test]
fn bulwark_failed_block_slam_and_death_leave_health_and_cooldown_retryable() {
    let (mut session, receiver, calls) = setup(true);
    fail_once(&mut session, &calls, "enemy_spawned");
    assert!(session.move_player(PLAYER, ENTRANCE).is_err());
    assert!(session.bulwark.is_none());
    session.move_player(PLAYER, ENTRANCE).unwrap();
    let enemy = session.enemy_id.unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.attack_at(PLAYER, enemy, 0).is_err());
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, enemy, 0).unwrap();
    assert_eq!(session.actors.get(enemy).unwrap().health, HEALTH);
    let health = session.actors.get(PLAYER).unwrap().health;
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_bulwark_at(1600).is_err());
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health);
    session.tick_bulwark_at(1600).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health - 14);
    for hit in 0..5 {
        session.attack_at(PLAYER, enemy, 1700 + hit * 250).unwrap();
    }
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "enemy_died");
    assert!(session.attack_at(PLAYER, enemy, 2950).is_err());
    assert_eq!(session.actors.get(enemy).unwrap().health, 40);
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, enemy, 2950).unwrap();
    assert_eq!(session.enemy_id, None);
}

#[test]
fn bulwark_retreat_and_older_content_clients_preserve_the_direct_route() {
    for capable in [false, true] {
        let (mut session, receiver, calls) = setup(capable);
        session.move_player(PLAYER, ENTRANCE).unwrap();
        assert_eq!(session.bulwark_fighting(), capable);
        if capable {
            drain_messages(&receiver);
            fail_once(&mut session, &calls, "field_activity");
            assert!(session.move_player(PLAYER, [4, 0, -8]).is_err());
            assert!(drain_messages(&receiver).is_empty());
            assert!(session.bulwark_fighting());
            session.move_player(PLAYER, [4, 0, -8]).unwrap();
            assert!(!session.bulwark_fighting());
        }
        session.move_player(PLAYER, [6, 0, 0]).unwrap();
        assert_eq!(session.stage, SessionStage::Boss);
    }
}
