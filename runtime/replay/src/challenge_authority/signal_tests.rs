use super::{
    admission, checkpoint, event, reconstruct, ChallengeState, ContractId, Kind, ReplayEvent,
};
use crate::SignalEvidence;
use revenant_challenges::signal::{Action, Effect, SignalRun};

fn fixture(win: bool) -> (Vec<ReplayEvent>, ChallengeState) {
    let (mut events, mut state) =
        admission(ContractId::DistantSignal, &ChallengeState::default(), false);
    let mut run = SignalRun::new(&state.active.as_ref().unwrap().equipment).unwrap();
    let mut proof = SignalEvidence {
        run_id: 1,
        sequence: 1,
        elapsed_ms: 0,
        enemy_id: 2,
        action: None,
        effect: Effect::default(),
    };
    event(&mut events, Kind::EnemySpawned, 2, proof.encode().unwrap());
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
        proof.sequence += 1;
        proof.elapsed_ms = u64::try_from(i).unwrap() * 100;
        proof.action = Some(Action::Move { position });
        let next = run.apply(proof.elapsed_ms, proof.action.unwrap()).unwrap();
        proof.effect = next.effect;
        run = next.run;
        event(
            &mut events,
            proof.kind(),
            proof.actor(1),
            proof.encode().unwrap(),
        );
    }
    for i in 1..=9 {
        proof.sequence += 1;
        proof.elapsed_ms = if win { 600 + (i - 1) * 250 } else { i * 1800 };
        proof.action = Some(if win { Action::Attack } else { Action::Observe });
        let next = run.apply(proof.elapsed_ms, proof.action.unwrap()).unwrap();
        proof.effect = next.effect;
        run = next.run;
        let id = event(
            &mut events,
            proof.kind(),
            proof.actor(1),
            proof.encode().unwrap(),
        );
        if let Some(command) = proof.command() {
            state = checkpoint(&mut events, &state, command, Some(id));
            return (events, state);
        }
    }
    panic!("signal fixture failed to terminate");
}

#[test]
fn challenge_signal_reconstructs_full_victory_and_defeat_without_rewards() {
    for win in [true, false] {
        let (events, state) = fixture(win);
        let replay = reconstruct(&events).unwrap();
        assert_eq!(replay.challenge.unwrap().state, state);
        assert_eq!(replay.loot_grants, 0);
        assert_eq!(replay.progression_grants, 0);
        assert_eq!(state.records.len(), usize::from(win));
    }
}

#[test]
fn challenge_signal_rejects_forged_identity_damage_time_movement_and_missing_terminal() {
    let (events, _) = fixture(true);
    let spawn = events
        .iter()
        .position(|e| e.kind == Kind::EnemySpawned)
        .unwrap();
    let attack = events
        .iter()
        .position(|e| {
            SignalEvidence::decode(&e.payload).is_ok_and(|p| p.action == Some(Action::Attack))
        })
        .unwrap();
    for change in 0..8 {
        let mut forged = events.clone();
        let index = if change < 4 { spawn + 1 } else { attack };
        let mut proof = SignalEvidence::decode(&forged[index].payload).unwrap();
        match change {
            0 => proof.sequence += 1,
            1 => proof.run_id += 1,
            2 => proof.enemy_id = 99,
            3 => {
                proof.action = Some(Action::Move {
                    position: [-7, 0, 1],
                });
            }
            4 => proof.effect.outgoing_damage += 1,
            5 => proof.elapsed_ms = 0,
            6 => proof.effect.incoming_damage = 12,
            7 => forged[index].actor_id = Some(99),
            _ => unreachable!(),
        }
        forged[index].payload = proof.encode().unwrap();
        assert!(reconstruct(&forged).is_err(), "corruption {change}");
    }
    let mut split = events.clone();
    split.pop();
    assert!(reconstruct(&split).is_err());
    let mut missing = events.clone();
    missing.remove(spawn + 1);
    assert!(reconstruct(&missing).is_err());
    let mut repeated = events.clone();
    let tail = repeated[attack].clone();
    event(
        &mut repeated,
        tail.kind,
        tail.actor_id.unwrap(),
        tail.payload,
    );
    assert!(reconstruct(&repeated).is_err());
}

#[test]
fn challenge_signal_rejects_wrong_build_and_early_enemy_fire() {
    let (events, _) = fixture(false);
    let index = events
        .iter()
        .position(|e| {
            SignalEvidence::decode(&e.payload).is_ok_and(|p| p.effect.incoming_damage > 0)
        })
        .unwrap();
    let mut forged = events.clone();
    let mut proof = SignalEvidence::decode(&forged[index].payload).unwrap();
    proof.elapsed_ms = 1799;
    forged[index].payload = proof.encode().unwrap();
    assert!(reconstruct(&forged).is_err());
    let mut foreign = events.clone();
    for e in &mut foreign {
        e.activity_id = Some(ContractId::PrismDiscipline.activity_id().into());
    }
    assert!(reconstruct(&foreign).is_err());
}
