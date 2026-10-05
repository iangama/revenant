use super::*;
use revenant_ai::support::ENTRANCE;

const PLAYER: u64 = 3034;
type Fixture = (
    SharedSession,
    Receiver<ServerMessage>,
    Arc<Mutex<Vec<String>>>,
);

fn setup(capable: bool) -> Fixture {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (mut participant, receiver) =
        fixture_participant("local:support-test", PLAYER, &["pulse_rifle"]);
    participant.support_capable = capable;
    participant.encounter_capable = true;
    participant.bulwark_capable = true;
    let persistence = FixturePersistence::new(None, calls.clone(), Vec::new());
    let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
    start_and_clear_drone(&mut session, PLAYER);
    drain_messages(&receiver);
    (session, receiver, calls)
}

fn begin(session: &mut SharedSession, receiver: &Receiver<ServerMessage>) -> (u64, u64) {
    session.move_player(PLAYER, ENTRANCE).unwrap();
    let messages = drain_messages(receiver);
    let id = |name| {
        messages
            .iter()
            .find_map(|m| match m {
                ServerMessage::ActorSpawn(a) if a.archetype == name => Some(a.actor_id),
                _ => None,
            })
            .unwrap()
    };
    session.move_player(PLAYER, [5, 0, 7]).unwrap();
    drain_messages(receiver);
    (id("glass-lancer"), id("relay-mender"))
}

fn fail_once(session: &mut SharedSession, calls: &Arc<Mutex<Vec<String>>>, kind: &str) {
    session.persistence = Box::new(FixturePersistence::new(
        Some(kind),
        calls.clone(),
        Vec::new(),
    ));
}

#[test]
fn support_repair_and_both_kill_orders_preserve_one_standard_reward() {
    for mender_first in [true, false] {
        let (mut session, receiver, calls) = setup(true);
        let (lancer, mender) = begin(&mut session, &receiver);
        assert!(session.attack_at(PLAYER, PLAYER, 0).is_err());
        session.attack_at(PLAYER, lancer, 0).unwrap();
        assert!(
            session.attack_at(PLAYER, mender, 100).is_err(),
            "switching targets cannot bypass cooldown"
        );
        session.tick_support_at(700).unwrap();
        session.tick_support_at(900).unwrap();
        assert_eq!(session.actors.get(lancer).unwrap().health, 168);
        assert!(drain_messages(&receiver).iter().any(|m| matches!(m, ServerMessage::RepairApplied(r) if r.amount == 8 && r.target_actor_id == lancer && r.source_actor_id == mender)));
        let first = if mender_first { mender } else { lancer };
        let second = if mender_first { lancer } else { mender };
        let mut time = 1000;
        for target in [first, second] {
            while session.actors.get(target).is_some() {
                session.attack_at(PLAYER, target, time).unwrap();
                time += 250;
            }
            session.tick_support_at(time).unwrap();
        }
        assert!(!session.support_fighting());
        assert_eq!(session.enemy_id, None);
        assert!(drain_messages(&receiver).iter().any(|m| matches!(m, ServerMessage::ObjectiveUpdate(o) if o.objective_id == "relay_mender" && o.state == "Completed" && o.progress == 2)));
        assert_eq!(session.stage, SessionStage::Door);
        session.move_player(PLAYER, [6, 0, 0]).unwrap();
        defeat_active_warden(&mut session, PLAYER);
        assert_eq!(session.stage, SessionStage::Complete);
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| *c == "completion_transaction")
                .count(),
            1
        );
        session.disconnect(PLAYER).unwrap();
        assert!(session.support.is_none());
    }
}

#[test]
fn support_failed_spawn_hit_repair_and_death_are_retryable_without_projection() {
    let (mut session, receiver, calls) = setup(true);
    fail_once(&mut session, &calls, "enemy_spawned");
    assert!(session.move_player(PLAYER, ENTRANCE).is_err());
    assert!(session.support.is_none());
    assert!(drain_messages(&receiver).is_empty());
    let (lancer, mender) = begin(&mut session, &receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.attack_at(PLAYER, lancer, 0).is_err());
    assert_eq!(session.actors.get(lancer).unwrap().health, 200);
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, lancer, 0).unwrap();
    session.tick_support_at(700).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_support_at(900).is_err());
    assert_eq!(session.actors.get(lancer).unwrap().health, 160);
    assert!(drain_messages(&receiver).is_empty());
    session.tick_support_at(900).unwrap();
    assert_eq!(session.actors.get(lancer).unwrap().health, 168);
    session.attack_at(PLAYER, mender, 1000).unwrap();
    session.attack_at(PLAYER, mender, 1250).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "enemy_died");
    assert!(session.attack_at(PLAYER, mender, 1500).is_err());
    assert_eq!(session.actors.get(mender).unwrap().health, 40);
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, mender, 1500).unwrap();
    drain_messages(&receiver);
    session.tick_support_at(2500).unwrap();
    assert_eq!(session.actors.get(lancer).unwrap().health, 168);
    assert!(!drain_messages(&receiver)
        .iter()
        .any(|m| matches!(m, ServerMessage::RepairApplied(_))));
}

#[test]
fn support_old_client_skip_withdrawal_and_lethal_charge_stop_the_pair() {
    for capable in [false, true] {
        let (mut session, receiver, calls) = setup(capable);
        session.move_player(PLAYER, ENTRANCE).unwrap();
        assert_eq!(session.support_fighting(), capable);
        if capable {
            drain_messages(&receiver);
            fail_once(&mut session, &calls, "field_activity");
            assert!(session.move_player(PLAYER, [1, 0, 6]).is_err());
            assert!(session.support_fighting());
            assert!(drain_messages(&receiver).is_empty());
            session.move_player(PLAYER, [1, 0, 6]).unwrap();
            assert!(!session.support_fighting());
            assert_eq!(
                drain_messages(&receiver)
                    .iter()
                    .filter(|m| matches!(m, ServerMessage::ActorDestroy(_)))
                    .count(),
                2
            );
        }
        session.move_player(PLAYER, [6, 0, 0]).unwrap();
        assert_eq!(session.stage, SessionStage::Boss);
    }
    let (mut session, receiver, calls) = setup(true);
    begin(&mut session, &receiver);
    session.move_player(PLAYER, [5, 0, 8]).unwrap();
    let health = session.actors.get(PLAYER).unwrap().health;
    session.actors.apply_damage(PLAYER, health - 1);
    session.tick_support_at(700).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_support_at(2500).is_err());
    assert_eq!(session.actors.get(PLAYER).unwrap().health, 1);
    assert!(drain_messages(&receiver).is_empty());
    session.tick_support_at(2500).unwrap();
    assert_eq!(session.stage, SessionStage::Failed);
    assert!(!session.support_fighting());
    drain_messages(&receiver);
    session.tick_support_at(9000).unwrap();
    assert!(drain_messages(&receiver).is_empty());
}
