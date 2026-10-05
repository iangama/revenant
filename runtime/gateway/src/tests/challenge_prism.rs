use super::{ContractId, Fixture, Outcome, ServerMessage, SessionStage, SharedSession};

fn move_to(session: &mut SharedSession, target: [i32; 3]) {
    while session.actors.get(41).unwrap().position != target {
        let mut next = session.actors.get(41).unwrap().position;
        let axis = if next[0] == target[0] { 2 } else { 0 };
        next[axis] += (target[axis] - next[axis]).signum();
        session.move_player(41, next).unwrap();
        assert_eq!(session.actors.get(41).unwrap().position, next);
    }
}

fn reach_last_hit(session: &mut SharedSession, boss: u64) {
    session.attack_at(41, boss, 0).unwrap();
    assert_eq!(session.actors.get(boss).unwrap().health, 320); // Closed shell.
    session.tick_prism_at(700).unwrap();
    move_to(session, [5, 0, 1]);
    session.tick_prism_at(2500).unwrap();
    for time in [2500, 2750, 3000, 3250] {
        session.attack_at(41, boss, time).unwrap();
    }
    assert_eq!(session.actors.get(boss).unwrap().health, 160);
    session.attack_at(41, boss, 3500).unwrap();
    assert_eq!(session.actors.get(boss).unwrap().health, 160); // Phase two cannot be skipped.
    move_to(session, [6, 0, 0]);
    session.tick_prism_at(5650).unwrap();
    session.tick_prism_at(7850).unwrap();
    session.tick_prism_at(8450).unwrap();
    move_to(session, [8, 0, 0]);
    session.tick_prism_at(10650).unwrap();
    for time in [10650, 10900, 11150] {
        session.attack_at(41, boss, time).unwrap();
    }
    assert_eq!(session.actors.get(boss).unwrap().health, 40);
}

#[test]
fn challenge_world_prism_preserves_both_phases_and_completes_without_rewards() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::PrismDiscipline, None);
    let boss = session.enemy_id.unwrap();
    assert!(messages
        .try_iter()
        .any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.contracts.len()==5)));
    reach_last_hit(&mut session, boss);
    session.attack_at(41, boss, 11400).unwrap();
    assert!(session.actors.get(boss).is_none());
    assert_eq!(session.actors.get(41).unwrap().health, 100);
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Completed);
    assert_eq!(f.state().records[0].rules.seed, 37005);
    let output: Vec<_> = messages.try_iter().collect();
    assert!(output.iter().any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.active.is_none() && s.records.len()==1)));
    assert!(!output.iter().any(|m| matches!(
        m,
        ServerMessage::LootGranted(_)
            | ServerMessage::ProgressionGranted(_)
            | ServerMessage::ActivityComplete(_)
    )));
}

#[test]
fn challenge_world_prism_storage_failure_withholds_fatal_hit_and_retry_restarts_boss() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::PrismDiscipline, None);
    let boss = session.enemy_id.unwrap();
    reach_last_hit(&mut session, boss);
    for _ in messages.try_iter() {}
    f.end_externally(&session);
    assert!(session.attack_at(41, boss, 11400).is_err());
    assert_eq!(session.actors.get(boss).unwrap().health, 40);
    assert!(messages.try_recv().is_err());
    assert!(f.state().records.is_empty());
    let (mut retry, _) = f.enter(ContractId::PrismDiscipline, Some(1));
    assert_eq!(
        retry.actors.get(retry.enemy_id.unwrap()).unwrap().health,
        320
    );
    assert_eq!(
        retry.actors.get(41).unwrap().position,
        revenant_ai::prism::ENTRANCE
    );
    assert_eq!(f.state().active.unwrap().objectives, 0);
    retry.disconnect(41).unwrap();
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Interrupted);
}

#[test]
fn challenge_world_prism_lethal_pattern_commits_defeat_before_projection() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::PrismDiscipline, None);
    for time in (700..40_000).step_by(100) {
        session.tick_prism_at(time).unwrap();
        if session.actors.get(41).unwrap().health == 0 {
            break;
        }
    }
    assert_eq!(session.actors.get(41).unwrap().health, 0);
    assert_eq!(session.stage, SessionStage::Failed);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Defeated);
    assert!(f.state().records.is_empty());
    assert!(messages.try_iter().any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.last_result.as_ref().is_some_and(|r| r.outcome=="defeated"))));
}
