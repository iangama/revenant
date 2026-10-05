use super::{ContractId, Fixture, Outcome, ServerMessage, SessionStage, SharedSession};
use revenant_challenges::signal::{Action, SPAWN};

fn firing_lane(session: &mut SharedSession) {
    for (i, position) in [
        [-5, 0, 4],
        [-6, 0, 4],
        [-7, 0, 4],
        [-7, 0, 3],
        [-7, 0, 2],
        [-7, 0, 1],
    ]
    .into_iter()
    .enumerate()
    {
        session
            .apply_signal_action_at(
                41,
                u64::try_from(i).unwrap() * 100,
                Action::Move { position },
            )
            .unwrap();
    }
}

#[test]
fn challenge_world_signal_respects_cover_and_completes_without_rewards() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::DistantSignal, None);
    let enemy = session.enemy_id.unwrap();
    assert!(messages
        .try_iter()
        .any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.contracts.len() == 6)));
    session.attack_at(41, enemy, 0).unwrap();
    assert_eq!(session.actors.get(enemy).unwrap().health, 200);
    session
        .apply_signal_action_at(
            41,
            0,
            Action::Move {
                position: [-4, 0, 0],
            },
        )
        .unwrap();
    assert_eq!(session.actors.get(41).unwrap().position, SPAWN);
    firing_lane(&mut session);
    for time in [600, 850, 1100, 1350, 1600] {
        session.attack_at(41, enemy, time).unwrap();
    }
    assert!(session.actors.get(enemy).is_none());
    assert_eq!(session.actors.get(41).unwrap().health, 100);
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(f.state().records[0].rules.seed, 37006);
    let output: Vec<_> = messages.try_iter().collect();
    assert!(output.iter().any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.last_result.as_ref().is_some_and(|r| r.outcome == "completed"))));
    assert!(!output.iter().any(|m| matches!(
        m,
        ServerMessage::LootGranted(_)
            | ServerMessage::ProgressionGranted(_)
            | ServerMessage::ActivityComplete(_)
    )));
}

#[test]
fn challenge_world_signal_storage_failure_withholds_movement_and_fatal_damage() {
    for fatal in [false, true] {
        let Some(f) = Fixture::new() else {
            return;
        };
        let (mut session, messages) = f.enter(ContractId::DistantSignal, None);
        let enemy = session.enemy_id.unwrap();
        if fatal {
            firing_lane(&mut session);
            for time in [600, 850, 1100, 1350] {
                session.attack_at(41, enemy, time).unwrap();
            }
        }
        let position = session.actors.get(41).unwrap().position;
        let health = session.actors.get(enemy).unwrap().health;
        for _ in messages.try_iter() {}
        f.end_externally(&session);
        let result = if fatal {
            session.attack_at(41, enemy, 1600)
        } else {
            session.apply_signal_action_at(
                41,
                0,
                Action::Move {
                    position: [-5, 0, 4],
                },
            )
        };
        assert!(result.is_err());
        assert_eq!(session.actors.get(41).unwrap().position, position);
        assert_eq!(session.actors.get(enemy).unwrap().health, health);
        assert!(messages.try_recv().is_err());
        assert!(f.state().records.is_empty());
        let (mut retry, _) = f.enter(ContractId::DistantSignal, Some(1));
        assert_eq!(retry.actors.get(41).unwrap().position, SPAWN);
        assert_eq!(retry.actors.get(41).unwrap().health, 100);
        assert_eq!(
            retry.actors.get(retry.enemy_id.unwrap()).unwrap().health,
            200
        );
        retry.disconnect(41).unwrap();
        assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Interrupted);
    }
}

#[test]
fn challenge_world_signal_confirms_lethal_enemy_shot_before_defeat() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::DistantSignal, None);
    firing_lane(&mut session);
    for i in 1..=9 {
        session
            .apply_signal_action_at(41, i * 1800, Action::Observe)
            .unwrap();
    }
    assert_eq!(session.actors.get(41).unwrap().health, 0);
    assert_eq!(session.stage, SessionStage::Failed);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Defeated);
    assert!(f.state().records.is_empty());
    assert!(messages.try_iter().any(|m| matches!(m, ServerMessage::ChallengeSnapshot(s) if s.last_result.as_ref().is_some_and(|r| r.outcome == "defeated"))));
}
