use super::{database, total_events, Fixture};
use revenant_challenges::{gauntlet::Stage, ContractId};
use revenant_persistence::Persistence;
use revenant_replay::{
    GauntletAction, GauntletEffect, GauntletEvidence, SupportEvidence, SupportTransition,
};

struct Route {
    proof: GauntletEvidence,
    position: [i32; 3],
}

impl Route {
    fn append(
        &mut self,
        f: &Fixture,
        db: &mut Persistence,
        actor: u64,
        action: GauntletAction,
        effect: GauntletEffect,
    ) {
        self.proof.sequence += 1;
        self.proof.elapsed_ms += 250;
        self.proof.actor_id = actor;
        self.proof.action = action;
        self.proof.effect = effect;
        db.apply_challenge_gauntlet_step(&f.character, &self.proof, f.context())
            .unwrap();
    }

    fn walk(&mut self, f: &Fixture, db: &mut Persistence, target: [i32; 3]) {
        while self.position != target {
            let axis = if self.position[0] == target[0] { 2 } else { 0 };
            self.position[axis] += (target[axis] - self.position[axis]).signum();
            self.append(
                f,
                db,
                41,
                GauntletAction::Move {
                    position: self.position,
                },
                GauntletEffect::default(),
            );
        }
    }
}

fn first_stage(f: &Fixture, db: &mut Persistence) -> Route {
    f.join(db);
    let state = f.start(db, "gauntlet-start");
    let proof = GauntletEvidence {
        run_id: state.active.unwrap().run_id,
        sequence: 1,
        elapsed_ms: 0,
        actor_id: 42,
        action: GauntletAction::Support(SupportEvidence {
            elapsed_ms: 0,
            transition: SupportTransition::Started {
                lancer_id: 42,
                mender_id: 43,
                player_health: 100,
            },
        }),
        effect: GauntletEffect::default(),
    };
    db.apply_challenge_gauntlet_step(&f.character, &proof, f.context())
        .unwrap();
    let mut route = Route {
        proof,
        position: Stage::Support.entrance(),
    };
    route.walk(f, db, [5, 0, 8]);
    let mut time = 0;
    for (target, initial) in [(43, 120u32), (42, 200)] {
        let mut health = initial;
        while health > 0 {
            let next = health.saturating_sub(40);
            route.append(
                f,
                db,
                if next == 0 { target } else { 41 },
                GauntletAction::Support(SupportEvidence {
                    elapsed_ms: time,
                    transition: SupportTransition::Attack {
                        target_id: target,
                        player_position: route.position,
                        damage: 40,
                        health_before: health,
                        health_after: next,
                    },
                }),
                GauntletEffect {
                    cleared: (target == 42 && next == 0).then_some(Stage::Support),
                    ..GauntletEffect::default()
                },
            );
            time += 250;
            health = next;
        }
    }
    route
}

#[test]
fn gauntlet_transfer_rejects_forged_healing_rolls_back_and_replays_once() {
    let Some(f) = Fixture::new(ContractId::RelayGauntlet) else {
        return;
    };
    let mut db = database(&f.url);
    let mut route = first_stage(&f, &mut db);
    assert_eq!(f.state(&mut db).active.unwrap().objectives, 1);
    route.walk(&f, &mut db, Stage::Support.entrance());
    let mut transfer = route.proof.clone();
    transfer.sequence += 1;
    transfer.elapsed_ms += 1200;
    transfer.actor_id = 41;
    transfer.action = GauntletAction::Observe;
    transfer.effect.next_stage = Some(Stage::Bastion);
    let before = total_events(&f);
    let mut bad = transfer.clone();
    bad.effect.recovered_health = 24;
    assert!(db
        .apply_challenge_gauntlet_step(&f.character, &bad, f.context())
        .is_err());
    bad = transfer.clone();
    bad.elapsed_ms -= 1;
    assert!(db
        .apply_challenge_gauntlet_step(&f.character, &bad, f.context())
        .is_err());
    assert_eq!(total_events(&f), before);
    assert!(
        !db.apply_challenge_gauntlet_step(&f.character, &transfer, f.context())
            .unwrap()
            .replayed
    );
    assert!(
        db.apply_challenge_gauntlet_step(&f.character, &transfer, f.context())
            .unwrap()
            .replayed
    );
    assert_eq!(total_events(&f), before + 1);
    bad = transfer;
    bad.effect.recovered_health = 1;
    assert!(db
        .apply_challenge_gauntlet_step(&f.character, &bad, f.context())
        .is_err());
    assert_eq!(total_events(&f), before + 1);
    assert!(f.state(&mut db).records.is_empty());
    assert_eq!(
        db.progression_for(&f.character)
            .unwrap()
            .unwrap()
            .experience,
        0
    );
}

#[test]
fn gauntlet_checkpoint_is_atomic_and_foreign_actor_cannot_append() {
    let Some(f) = Fixture::new(ContractId::RelayGauntlet) else {
        return;
    };
    let mut db = database(&f.url);
    let route = first_stage(&f, &mut db);
    let before = total_events(&f);
    assert!(
        db.apply_challenge_gauntlet_step(&f.character, &route.proof, f.context())
            .unwrap()
            .replayed
    );
    assert_eq!(total_events(&f), before);
    let mut bad = route.proof.clone();
    bad.effect.cleared = None;
    assert!(db
        .apply_challenge_gauntlet_step(&f.character, &bad, f.context())
        .is_err());
    bad = route.proof.clone();
    bad.sequence += 1;
    bad.actor_id = 99;
    bad.action = GauntletAction::Move {
        position: [4, 0, 8],
    };
    bad.effect = GauntletEffect::default();
    assert!(db
        .apply_challenge_gauntlet_step(&f.character, &bad, f.context())
        .is_err());
    assert_eq!(total_events(&f), before);
    assert_eq!(f.state(&mut db).active.unwrap().objectives, 1);
}
