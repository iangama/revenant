use super::{database, total_events, unique, Fixture};
use revenant_challenges::{
    survival::{
        Action, Effect, Phase, SurvivalRun, CHARGE_MS, EAST_RELAY, RECOVERY_MS, RESERVE_PAD, SPAWN,
        WEST_RELAY,
    },
    Command, ContractId, Objective, Outcome,
};
use revenant_persistence::{ChallengeRequest, Persistence};
use revenant_replay::SurvivalEvidence;
use std::{
    sync::{Arc, Barrier},
    thread,
};

struct Route {
    world: SurvivalRun,
    proof: SurvivalEvidence,
    steps: Vec<SurvivalEvidence>,
}

impl Route {
    fn step(&mut self, after_ms: u64, action: Action) {
        self.proof.sequence += 1;
        self.proof.elapsed_ms += after_ms;
        self.proof.action = Some(action);
        let next = self.world.apply(self.proof.elapsed_ms, action).unwrap();
        self.proof.effect = next.effect;
        self.world = next.run;
        self.steps.push(self.proof.clone());
    }

    fn walk(&mut self, target: [i32; 3]) {
        while self.world.position() != target {
            let mut position = self.world.position();
            let axis = if position[0] == target[0] { 2 } else { 0 };
            position[axis] += (target[axis] - position[axis]).signum();
            self.step(200, Action::Move { position });
        }
    }

    fn hold(&mut self, duration: u64) {
        for _ in 0..duration / 100 {
            self.step(100, Action::Observe);
        }
    }
}

fn prefix(f: &Fixture, db: &mut Persistence, win: bool) -> SurvivalEvidence {
    let run = f.state(db).active.unwrap();
    let proof = SurvivalEvidence {
        run_id: run.run_id,
        sequence: 1,
        elapsed_ms: 0,
        enemy_id: 42,
        action: None,
        effect: Effect::default(),
    };
    let mut route = Route {
        world: SurvivalRun::new(&run.equipment).unwrap(),
        proof: proof.clone(),
        steps: vec![proof],
    };
    route.walk([-8, 0, 4]);
    route.walk(WEST_RELAY);
    if win {
        route.hold(CHARGE_MS);
        route.walk([-8, 0, 4]);
        route.walk(RESERVE_PAD);
        route.hold(RECOVERY_MS);
        route.walk([0, 0, 5]);
        route.walk(EAST_RELAY);
        route.hold(CHARGE_MS);
        route.walk([0, 0, 4]);
        route.walk(SPAWN);
        assert_eq!(route.world.phase(), Phase::Completed);
    } else {
        while route.world.health() > 0 {
            route.step(1800, Action::Observe);
        }
    }
    let terminal = route.steps.pop().unwrap();
    for proof in route.steps {
        if proof.effect.recovered_health > 0 {
            let count = total_events(f);
            let mut forged = proof.clone();
            forged.effect.recovered_health += 1;
            assert!(db
                .apply_challenge_survival_step(&f.character, &forged, f.context())
                .is_err());
            assert_eq!(total_events(f), count);
        }
        db.apply_challenge_survival_step(&f.character, &proof, f.context())
            .unwrap();
        if proof.effect.recovered_health > 0 {
            let count = total_events(f);
            assert!(
                db.apply_challenge_survival_step(&f.character, &proof, f.context())
                    .unwrap()
                    .replayed
            );
            assert_eq!(total_events(f), count);
        }
    }
    terminal
}

#[test]
fn survival_atomic_terminal_rejects_forgery_retries_concurrently_and_preserves_old_contracts() {
    let Some(mut f) = Fixture::new(ContractId::LastReserve) else {
        return;
    };
    let mut db = database(&f.url);
    let inventory = db.inventory_for(&f.character).unwrap();
    let progression = db.progression_for(&f.character).unwrap();
    let campaign = db.campaign_state_for(&f.account, &f.character).unwrap();
    f.join(&mut db);
    f.start(&mut db, "survival-start");
    let proof = prefix(&f, &mut db, true);
    let count = total_events(&f);
    let mut forged = proof.clone();
    forged.effect.checkpoint = None;
    assert!(db
        .apply_challenge_survival_step(&f.character, &forged, f.context())
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
                db.apply_challenge_survival_step(&f.character, &proof, f.context())
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
        .apply_challenge_survival_step(&f.character, &proof, context)
        .is_err());
    forged = proof.clone();
    forged.elapsed_ms += 1;
    assert!(db
        .apply_challenge_survival_step(&f.character, &forged, f.context())
        .is_err());
    f.session = format!("meridian-after-survival-{}", unique());
    f.contract = ContractId::MeridianCircuit;
    f.join(&mut db);
    f.start(&mut db, "meridian-after-survival");
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
fn survival_defeat_and_fresh_retry_reject_old_movement_and_reset_both_actors() {
    let Some(mut f) = Fixture::new(ContractId::LastReserve) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.start(&mut db, "survival-defeat");
    let proof = prefix(&f, &mut db, false);
    let count = total_events(&f);
    let mut forged = proof.clone();
    forged.elapsed_ms -= 1;
    assert!(db
        .apply_challenge_survival_step(&f.character, &forged, f.context())
        .is_err());
    assert_eq!(total_events(&f), count);
    let receipt = db
        .apply_challenge_survival_step(&f.character, &proof, f.context())
        .unwrap();
    assert_eq!(
        receipt.state.last_result.as_ref().unwrap().outcome,
        Outcome::Defeated
    );
    assert!(receipt.state.records.is_empty());
    assert!(
        db.apply_challenge_survival_step(&f.character, &proof, f.context())
            .unwrap()
            .replayed
    );
    f.session = format!("survival-retry-{}", unique());
    f.join(&mut db);
    let retry = ChallengeRequest {
        operation_id: "survival-retry",
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
        .apply_challenge_survival_step(&f.character, &proof, f.context())
        .is_err());
    let terminal = prefix(&f, &mut db, true);
    let done = db
        .apply_challenge_survival_step(&f.character, &terminal, f.context())
        .unwrap();
    assert_eq!(done.state.records.len(), 1);
}
