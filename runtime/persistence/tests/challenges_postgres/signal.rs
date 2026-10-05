use super::{database, total_events, unique, Fixture};
use revenant_challenges::{
    signal::{Action, Effect, SignalRun},
    Command, ContractId, Objective, Outcome,
};
use revenant_persistence::{ChallengeRequest, Persistence};
use revenant_replay::SignalEvidence;
use std::{
    sync::{Arc, Barrier},
    thread,
};

fn prefix(f: &Fixture, db: &mut Persistence, win: bool) -> SignalEvidence {
    let state = f.state(db);
    let run = state.active.unwrap();
    let mut world = SignalRun::new(&run.equipment).unwrap();
    let mut proof = SignalEvidence {
        run_id: run.run_id,
        sequence: 1,
        elapsed_ms: 0,
        enemy_id: 42,
        action: None,
        effect: Effect::default(),
    };
    db.apply_challenge_signal_step(&f.character, &proof, f.context())
        .unwrap();
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
        let next = world
            .apply(proof.elapsed_ms, proof.action.unwrap())
            .unwrap();
        proof.effect = next.effect;
        world = next.run;
        db.apply_challenge_signal_step(&f.character, &proof, f.context())
            .unwrap();
    }
    for i in 1..=9 {
        proof.sequence += 1;
        proof.elapsed_ms = if win { 600 + (i - 1) * 250 } else { i * 1800 };
        proof.action = Some(if win { Action::Attack } else { Action::Observe });
        let next = world
            .apply(proof.elapsed_ms, proof.action.unwrap())
            .unwrap();
        proof.effect = next.effect;
        world = next.run;
        if proof.command().is_some() {
            return proof;
        }
        db.apply_challenge_signal_step(&f.character, &proof, f.context())
            .unwrap();
    }
    panic!("signal fixture did not terminate");
}

#[test]
fn signal_atomic_terminal_rejects_forgery_retries_concurrently_and_preserves_old_contracts() {
    let Some(mut f) = Fixture::new(ContractId::DistantSignal) else {
        return;
    };
    let mut db = database(&f.url);
    let inventory = db.inventory_for(&f.character).unwrap();
    let progression = db.progression_for(&f.character).unwrap();
    let campaign = db.campaign_state_for(&f.account, &f.character).unwrap();
    f.join(&mut db);
    f.start(&mut db, "signal-start");
    let proof = prefix(&f, &mut db, true);
    let count = total_events(&f);
    let mut forged = proof.clone();
    forged.effect.outgoing_damage += 1;
    assert!(db
        .apply_challenge_signal_step(&f.character, &forged, f.context())
        .is_err());
    assert_eq!(total_events(&f), count);
    assert!(f.state(&mut db).active.is_some());
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let f = f.clone();
            let proof = proof.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut db = database(&f.url);
                barrier.wait();
                db.apply_challenge_signal_step(&f.character, &proof, f.context())
                    .unwrap()
            })
        })
        .collect();
    let receipts: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(receipts.iter().filter(|r| r.replayed).count(), 1);
    assert_eq!(receipts[0].transition, receipts[1].transition);
    assert_eq!(total_events(&f), count + 2);
    let record = f.state(&mut db).records[0].clone();
    let mut context = f.context();
    context.actor_id = 99;
    assert!(db
        .apply_challenge_signal_step(&f.character, &proof, context)
        .is_err());
    forged = proof.clone();
    forged.elapsed_ms += 1;
    assert!(db
        .apply_challenge_signal_step(&f.character, &forged, f.context())
        .is_err());
    f.session = format!("meridian-after-signal-{}", unique());
    f.contract = ContractId::MeridianCircuit;
    f.join(&mut db);
    f.start(&mut db, "meridian-after-signal");
    for objective in [
        Objective::LensRead,
        Objective::GalleryRead,
        Objective::LogRead,
        Objective::Returned,
    ] {
        f.visit(&mut db, objective);
    }
    let state = f.state(&mut db);
    assert_eq!(state.records.len(), 2);
    assert_eq!(state.records[0], record);
    assert_eq!(db.inventory_for(&f.character).unwrap(), inventory);
    assert_eq!(db.progression_for(&f.character).unwrap(), progression);
    assert_eq!(
        db.campaign_state_for(&f.account, &f.character).unwrap(),
        campaign
    );
}

#[test]
fn signal_defeat_and_fresh_retry_reject_old_movement_and_reset_both_actors() {
    let Some(mut f) = Fixture::new(ContractId::DistantSignal) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.start(&mut db, "signal-defeat");
    let proof = prefix(&f, &mut db, false);
    let count = total_events(&f);
    let mut forged = proof.clone();
    forged.elapsed_ms -= 1;
    assert!(db
        .apply_challenge_signal_step(&f.character, &forged, f.context())
        .is_err());
    assert_eq!(total_events(&f), count);
    let receipt = db
        .apply_challenge_signal_step(&f.character, &proof, f.context())
        .unwrap();
    assert_eq!(
        receipt.state.last_result.as_ref().unwrap().outcome,
        Outcome::Defeated
    );
    assert!(receipt.state.records.is_empty());
    assert!(
        db.apply_challenge_signal_step(&f.character, &proof, f.context())
            .unwrap()
            .replayed
    );
    f.session = format!("signal-retry-{}", unique());
    f.join(&mut db);
    let retry = ChallengeRequest {
        operation_id: "signal-retry",
        expected_revision: receipt.state.revision,
        command: Command::Retry {
            run_id: proof.run_id,
            equipment: f.equipment(&mut db),
        },
        proof_event_id: None,
        position: None,
    };
    let state = db
        .apply_challenge_command(&f.character, &retry, f.context())
        .unwrap()
        .state;
    assert!(state.active.as_ref().unwrap().run_id > proof.run_id);
    assert!(db
        .apply_challenge_signal_step(&f.character, &proof, f.context())
        .is_err());
    let terminal = prefix(&f, &mut db, true);
    let done = db
        .apply_challenge_signal_step(&f.character, &terminal, f.context())
        .unwrap();
    assert_eq!(done.state.records.len(), 1);
}
