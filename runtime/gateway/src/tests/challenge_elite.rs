use super::{ContractId, Fixture, Outcome, ServerMessage, SessionStage, SharedSession};
use std::sync::mpsc::Receiver;

fn advance(session: &mut SharedSession, messages: &Receiver<ServerMessage>, target: [i32; 3]) {
    while session.actors.get(41).unwrap().position != target {
        let mut position = session.actors.get(41).unwrap().position;
        let axis = if position[0] == target[0] { 2 } else { 0 };
        position[axis] += (target[axis] - position[axis]).signum();
        session.move_player(41, position).unwrap();
        assert_eq!(session.actors.get(41).unwrap().position, position);
        for message in messages.try_iter() {
            assert!(!matches!(
                message,
                ServerMessage::LootGranted(_) | ServerMessage::ProgressionGranted(_)
            ));
        }
    }
}

#[test]
fn challenge_world_elite_requires_flanking_and_repair_priority_without_rewards() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::BastionLink, None);
    let elite = session.elite.as_ref().unwrap();
    let (guard, mender) = (elite.bulwark_id, elite.partner_id);
    assert!(messages.try_iter().any(|m| matches!(m, ServerMessage::ChallengeSnapshot(board) if board.contracts.len() == 4 && board.active.as_ref().unwrap().contract.seed == 37004)));
    session.move_player(41, [10, 0, -8]).unwrap();
    assert_eq!(session.actors.get(41).unwrap().position, [5, 0, -6]);
    advance(&mut session, &messages, [6, 0, -8]);
    session.attack_at(41, guard, 0).unwrap();
    assert_eq!(session.actors.get(guard).unwrap().health, 240);
    advance(&mut session, &messages, [10, 0, -8]);
    session.attack_at(41, guard, 250).unwrap();
    assert_eq!(session.actors.get(guard).unwrap().health, 200);
    session.tick_elite_at(900).unwrap();
    assert_eq!(session.actors.get(guard).unwrap().health, 208);
    let mut time = 1000;
    for target in [mender, guard] {
        while session.actors.get(target).is_some() {
            session.attack_at(41, target, time).unwrap();
            time += 250;
        }
    }
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Completed);
    assert_eq!(f.state().records.len(), 1);
    let output: Vec<_> = messages.try_iter().collect();
    assert!(output.iter().any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.records.len() == 1 && s.active.is_none())));
    assert!(!output.iter().any(|m| matches!(
        m,
        ServerMessage::LootGranted(_) | ServerMessage::ProgressionGranted(_)
    )));
}

#[test]
fn challenge_world_elite_storage_failure_withholds_fatal_hit_and_retry_is_fresh() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::BastionLink, None);
    advance(&mut session, &messages, [10, 0, -8]);
    let target = session.elite.as_ref().unwrap().partner_id;
    session.attack_at(41, target, 0).unwrap();
    session.attack_at(41, target, 250).unwrap();
    for _ in messages.try_iter() {}
    f.end_externally(&session);
    assert!(session.attack_at(41, target, 500).is_err());
    assert_eq!(session.actors.get(target).unwrap().health, 40);
    assert!(messages.try_recv().is_err());
    assert!(f.state().records.is_empty());
    let (mut retry, _) = f.enter(ContractId::BastionLink, Some(1));
    assert_eq!(f.state().active.as_ref().unwrap().objectives, 0);
    assert!(f.state().active.as_ref().unwrap().run_id > 1);
    assert_eq!(retry.actors.get(41).unwrap().position, [5, 0, -6]);
    assert_eq!(
        retry
            .actors
            .get(retry.elite.as_ref().unwrap().partner_id)
            .unwrap()
            .health,
        120
    );
    retry.disconnect(41).unwrap();
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Interrupted);
}

#[test]
fn challenge_world_elite_lethal_slam_commits_defeat_before_projection() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::BastionLink, None);
    advance(&mut session, &messages, [6, 0, -8]);
    for time in (1600..50_000).step_by(100) {
        session.tick_elite_at(time).unwrap();
        if session.actors.get(41).unwrap().health == 0 {
            break;
        }
    }
    assert_eq!(session.actors.get(41).unwrap().health, 0);
    assert_eq!(session.stage, SessionStage::Failed);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Defeated);
    assert!(f.state().records.is_empty());
    assert!(messages.try_iter().any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.last_result.as_ref().is_some_and(|r| r.outcome == "defeated"))));
}
