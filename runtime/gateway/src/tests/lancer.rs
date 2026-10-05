use super::*;
use revenant_ai::lancer::{ENTRANCE, HEALTH};

const PLAYER: u64 = 1_934;
type Fixture = (
    SharedSession,
    Receiver<ServerMessage>,
    Arc<Mutex<Vec<String>>>,
);

fn setup(capable: bool) -> Fixture {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (mut participant, receiver) =
        fixture_participant("local:lancer-test", PLAYER, &["pulse_rifle"]);
    participant.encounter_capable = capable;
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
fn lancer_dodge_recovery_defeat_and_old_client_skip_preserve_the_normal_reward() {
    for capable in [false, true] {
        let (mut session, receiver, calls) = setup(capable);
        session.move_player(PLAYER, ENTRANCE).unwrap();
        assert_eq!(session.lancer_fighting(), capable);
        if capable {
            let enemy = session.enemy_id.unwrap();
            let health = session.actors.get(PLAYER).unwrap().health;
            session.tick_lancer_at(700).unwrap();
            assert!(drain_messages(&receiver).iter().any(|message| matches!(message,
                ServerMessage::ActorUpdate(update) if update.charge.as_ref().is_some_and(|cue| cue.winding_up && cue.target == ENTRANCE))));
            session.move_player(PLAYER, [4, 0, 9]).unwrap();
            session.tick_lancer_at(2_500).unwrap();
            assert_eq!(session.actors.get(PLAYER).unwrap().health, health);
            assert_eq!(session.actors.get(enemy).unwrap().position, ENTRANCE);
            session.tick_lancer_at(3_900).unwrap();
            session.tick_lancer_at(5_700).unwrap();
            assert_eq!(session.actors.get(PLAYER).unwrap().health, health - 18);
            for hit in 0..5 {
                session.attack_at(PLAYER, enemy, hit * 250).unwrap();
            }
            assert!(!session.lancer_fighting());
            assert_eq!(session.enemy_id, None);
            drain_messages(&receiver);
            session.tick_lancer_at(99_000).unwrap();
            assert!(drain_messages(&receiver).is_empty());
        }
        session.move_player(PLAYER, [6, 0, 0]).unwrap();
        assert_eq!(session.stage, SessionStage::Boss);
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
        assert!(session.lancer.is_none());
    }
}

#[test]
fn lancer_failed_spawn_warning_and_retreat_are_retryable_without_projection() {
    let (mut session, receiver, calls) = setup(true);
    let initial = session.actors.get(PLAYER).unwrap().position;
    fail_once(&mut session, &calls, "enemy_spawned");
    assert!(session.move_player(PLAYER, ENTRANCE).is_err());
    assert_eq!(session.actors.get(PLAYER).unwrap().position, initial);
    assert!(session.lancer.is_none());
    assert!(drain_messages(&receiver).is_empty());
    session.move_player(PLAYER, ENTRANCE).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_lancer_at(700).is_err());
    assert!(drain_messages(&receiver).is_empty());
    session.tick_lancer_at(700).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.move_player(PLAYER, [4, 0, 4]).is_err());
    assert_eq!(session.actors.get(PLAYER).unwrap().position, ENTRANCE);
    assert!(session.lancer_fighting());
    assert!(drain_messages(&receiver).is_empty());
    session.move_player(PLAYER, [4, 0, 4]).unwrap();
    assert!(!session.lancer_fighting());
    assert_eq!(session.enemy_id, None);
    session.move_player(PLAYER, [6, 0, 0]).unwrap();
    assert_eq!(session.stage, SessionStage::Boss);
}

#[test]
fn lancer_failed_damage_and_fatal_hit_do_not_advance_health_or_cooldown() {
    let (mut session, receiver, calls) = setup(true);
    session.move_player(PLAYER, ENTRANCE).unwrap();
    let enemy = session.enemy_id.unwrap();
    session.tick_lancer_at(700).unwrap();
    drain_messages(&receiver);
    let health = session.actors.get(PLAYER).unwrap().health;
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_lancer_at(2_500).is_err());
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health);
    assert!(drain_messages(&receiver).is_empty());
    session.tick_lancer_at(2_500).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health - 18);
    for hit in 0..4 {
        session.attack_at(PLAYER, enemy, hit * 250).unwrap();
    }
    assert_eq!(session.actors.get(enemy).unwrap().health, HEALTH - 160);
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "enemy_died");
    assert!(session.attack_at(PLAYER, enemy, 1_000).is_err());
    assert_eq!(session.actors.get(enemy).unwrap().health, 40);
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, enemy, 1_000).unwrap();
    assert!(!session.lancer_fighting());
}
