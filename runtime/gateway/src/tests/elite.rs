use super::*;
use revenant_ai::elite::EliteComposition;

const PLAYER: u64 = 4034;
type Fixture = (
    SharedSession,
    Receiver<ServerMessage>,
    Arc<Mutex<Vec<String>>>,
);

fn setup(capable: bool) -> Fixture {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (mut participant, receiver) =
        fixture_participant("local:elite-test", PLAYER, &["pulse_rifle"]);
    participant.elite_capable = capable;
    participant.support_capable = true;
    participant.bulwark_capable = true;
    participant.encounter_capable = true;
    let persistence = FixturePersistence::new(None, calls.clone(), Vec::new());
    let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
    start_and_clear_drone(&mut session, PLAYER);
    drain_messages(&receiver);
    (session, receiver, calls)
}

fn begin(
    session: &mut SharedSession,
    receiver: &Receiver<ServerMessage>,
    composition: EliteComposition,
) -> (u64, u64) {
    session.move_player(PLAYER, composition.entrance()).unwrap();
    let guard = session.enemy_id.unwrap();
    let partner = drain_messages(receiver)
        .iter()
        .find_map(|message| match message {
            ServerMessage::ActorSpawn(actor) if actor.actor_id != guard => Some(actor.actor_id),
            _ => None,
        })
        .unwrap();
    session.move_player(PLAYER, [7, 0, -6]).unwrap();
    drain_messages(receiver);
    (guard, partner)
}

fn fail_once(session: &mut SharedSession, calls: &Arc<Mutex<Vec<String>>>, kind: &str) {
    session.persistence = Box::new(FixturePersistence::new(
        Some(kind),
        calls.clone(),
        Vec::new(),
    ));
}

#[test]
fn elite_pairs_keep_both_targets_and_one_standard_reward_in_either_kill_order() {
    for composition in [
        EliteComposition::BastionLink,
        EliteComposition::CrossedGuard,
    ] {
        for guard_first in [false, true] {
            let (mut session, receiver, calls) = setup(true);
            let (guard, partner) = begin(&mut session, &receiver, composition);
            assert!(session.attack_at(PLAYER, PLAYER, 0).is_err());
            session.attack_at(PLAYER, guard, 0).unwrap();
            assert!(session.attack_at(PLAYER, partner, 100).is_err());
            session.tick_elite_at(900).unwrap();
            assert_eq!(
                session.actors.get(guard).unwrap().health,
                if composition == EliteComposition::BastionLink {
                    208
                } else {
                    200
                }
            );
            let mut time = 1000;
            for target in if guard_first {
                [guard, partner]
            } else {
                [partner, guard]
            } {
                while session.actors.get(target).is_some() {
                    session.attack_at(PLAYER, target, time).unwrap();
                    time += 250;
                }
                session.tick_elite_at(time).unwrap();
            }
            assert!(!session.elite_fighting());
            assert!(drain_messages(&receiver).iter().any(|m| matches!(m, ServerMessage::ObjectiveUpdate(o) if o.objective_id == composition.id() && o.state == "Completed" && o.progress == 2)));
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
            assert!(session.elite.is_none());
        }
    }
}

#[test]
fn elite_failed_spawn_repair_block_and_fatal_hit_leave_state_retryable() {
    let (mut session, receiver, calls) = setup(true);
    fail_once(&mut session, &calls, "enemy_spawned");
    assert!(session
        .move_player(PLAYER, EliteComposition::BastionLink.entrance())
        .is_err());
    assert!(session.elite.is_none());
    assert!(drain_messages(&receiver).is_empty());
    let (guard, partner) = begin(&mut session, &receiver, EliteComposition::BastionLink);
    session.move_player(PLAYER, [6, 0, -8]).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.attack_at(PLAYER, guard, 0).is_err());
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, guard, 0).unwrap();
    assert_eq!(session.actors.get(guard).unwrap().health, 240);
    session.move_player(PLAYER, [7, 0, -6]).unwrap();
    session.attack_at(PLAYER, guard, 250).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_elite_at(900).is_err());
    assert_eq!(session.actors.get(guard).unwrap().health, 200);
    assert!(drain_messages(&receiver).is_empty());
    session.tick_elite_at(900).unwrap();
    assert_eq!(session.actors.get(guard).unwrap().health, 208);
    session.attack_at(PLAYER, partner, 1000).unwrap();
    session.attack_at(PLAYER, partner, 1250).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "enemy_died");
    assert!(session.attack_at(PLAYER, partner, 1500).is_err());
    assert_eq!(session.actors.get(partner).unwrap().health, 40);
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, partner, 1500).unwrap();
    session.tick_elite_at(2500).unwrap();
    assert_eq!(session.actors.get(guard).unwrap().health, 208);
}

#[test]
fn crossed_guard_does_not_overlap_warnings_and_retries_a_failed_charge() {
    let (mut session, receiver, calls) = setup(true);
    let (guard, partner) = begin(&mut session, &receiver, EliteComposition::CrossedGuard);
    session.move_player(PLAYER, [6, 0, -8]).unwrap();
    let health = session.actors.get(PLAYER).unwrap().health;
    drain_messages(&receiver);
    session.tick_elite_at(700).unwrap();
    assert!(
        drain_messages(&receiver).is_empty(),
        "Lancer waits for the initial slam"
    );
    session.tick_elite_at(1600).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health - 14);
    drain_messages(&receiver);
    session.tick_elite_at(2199).unwrap();
    assert!(drain_messages(&receiver).is_empty());
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_elite_at(2200).is_err());
    assert!(drain_messages(&receiver).is_empty());
    session.tick_elite_at(2200).unwrap();
    assert!(drain_messages(&receiver).iter().any(|m| matches!(m, ServerMessage::ActorUpdate(a) if a.actor_id == partner && a.charge.as_ref().is_some_and(|c| c.winding_up))));
    session.tick_elite_at(3400).unwrap();
    assert!(
        drain_messages(&receiver).is_empty(),
        "Bulwark keeps shield down during the charge warning"
    );
    session.move_player(PLAYER, [6, 0, -9]).unwrap();
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_elite_at(4000).is_err());
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health - 14);
    assert!(drain_messages(&receiver).is_empty());
    session.tick_elite_at(4000).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, health - 14 - 18);
    assert_eq!(session.actors.get(partner).unwrap().position, [6, 0, -9]);
    drain_messages(&receiver);
    session.tick_elite_at(4599).unwrap();
    assert!(drain_messages(&receiver).is_empty());
    session.tick_elite_at(4600).unwrap();
    assert!(drain_messages(&receiver).iter().any(|m| matches!(m, ServerMessage::ActorUpdate(a) if a.actor_id == guard && a.defense.as_ref().is_some_and(|c| c.braced))));
}

#[test]
fn elite_old_clients_retreat_and_lethal_damage_preserve_session_boundaries() {
    for composition in [
        EliteComposition::BastionLink,
        EliteComposition::CrossedGuard,
    ] {
        let (mut session, receiver, _) = setup(false);
        session.move_player(PLAYER, composition.entrance()).unwrap();
        assert!(!session.elite_fighting());
        assert!(!drain_messages(&receiver)
            .iter()
            .any(|m| matches!(m, ServerMessage::ActorSpawn(_))));
        let (mut session, receiver, calls) = setup(true);
        let (guard, partner) = begin(&mut session, &receiver, composition);
        fail_once(&mut session, &calls, "field_activity");
        assert!(session.move_player(PLAYER, [4, 0, -6]).is_err());
        assert!(session.elite_fighting());
        assert!(drain_messages(&receiver).is_empty());
        session.move_player(PLAYER, [4, 0, -6]).unwrap();
        assert!(!session.elite_fighting());
        assert!(session.actors.get(guard).is_none() && session.actors.get(partner).is_none());
        session.move_player(PLAYER, [6, 0, 0]).unwrap();
        assert_eq!(session.stage, SessionStage::Boss);
    }
    let (mut session, receiver, _) = setup(true);
    begin(&mut session, &receiver, EliteComposition::CrossedGuard);
    session.move_player(PLAYER, [6, 0, -8]).unwrap();
    let health = session.actors.get(PLAYER).unwrap().health;
    session.actors.apply_damage(PLAYER, health - 1);
    session.tick_elite_at(1600).unwrap();
    assert_eq!(session.stage, SessionStage::Failed);
    assert!(!session.elite_fighting());
    drain_messages(&receiver);
    session.tick_elite_at(10000).unwrap();
    assert!(drain_messages(&receiver).is_empty());
}
