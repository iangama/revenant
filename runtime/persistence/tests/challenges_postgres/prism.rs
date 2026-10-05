use super::{database, total_events, unique, Fixture};
use revenant_challenges::{Command, ContractId, Objective};
use revenant_persistence::{ChallengeRequest, NewReplayEvent, Persistence};
use revenant_replay::{
    encode_prism_evidence, PrismEvidence, PrismPatternEvidence as Pattern, PrismStep,
    PrismTransition,
};
use std::{
    sync::{Arc, Barrier},
    thread,
};

fn append(
    f: &Fixture,
    db: &mut Persistence,
    confirmed: &mut u64,
    time: u64,
    transition: PrismTransition,
) {
    let proof = PrismEvidence {
        elapsed_ms: time,
        previous_confirmed_ms: *confirmed,
        transition,
    };
    let spawn = matches!(proof.transition, PrismTransition::Started { .. });
    db.append_replay_event(&NewReplayEvent {
        event_type: if spawn {
            "boss_spawned"
        } else {
            "field_activity"
        },
        session_id: &f.session,
        account_id: &f.account,
        activity_id: Some(f.contract.activity_id()),
        actor_id: Some(if spawn { 42 } else { 41 }),
        payload: &encode_prism_evidence(&proof).unwrap(),
    })
    .unwrap();
    *confirmed = time;
}

fn prefix(f: &Fixture, db: &mut Persistence) -> PrismEvidence {
    let mut confirmed = 0;
    append(
        f,
        db,
        &mut confirmed,
        0,
        PrismTransition::Started {
            boss_id: 42,
            player_health: 100,
        },
    );
    for (time, position, step) in [
        (
            700,
            [5, 0, 2],
            PrismStep::Warning {
                pattern: Pattern::AcrossX { z: 2 },
            },
        ),
        (
            2500,
            [5, 0, 1],
            PrismStep::Resolved {
                pattern: Pattern::AcrossX { z: 2 },
                damage: 0,
                health_after: 100,
            },
        ),
    ] {
        append(
            f,
            db,
            &mut confirmed,
            time,
            PrismTransition::Step {
                player_position: position,
                step,
            },
        );
    }
    for i in 0..4u32 {
        append(
            f,
            db,
            &mut confirmed,
            2500 + u64::from(i) * 250,
            attack(320 - i * 40),
        );
    }
    phase_two(f, db, &mut confirmed);
    for i in 0..3u32 {
        append(
            f,
            db,
            &mut confirmed,
            10650 + u64::from(i) * 250,
            attack(160 - i * 40),
        );
    }
    PrismEvidence {
        elapsed_ms: 11400,
        previous_confirmed_ms: confirmed,
        transition: attack(40),
    }
}

fn phase_two(f: &Fixture, db: &mut Persistence, confirmed: &mut u64) {
    for (time, position, step) in [
        (
            5650,
            [6, 0, 0],
            PrismStep::Warning {
                pattern: Pattern::Center,
            },
        ),
        (
            7850,
            [6, 0, 0],
            PrismStep::Resolved {
                pattern: Pattern::Center,
                damage: 0,
                health_after: 100,
            },
        ),
        (
            8450,
            [6, 0, 0],
            PrismStep::Warning {
                pattern: Pattern::Perimeter,
            },
        ),
        (
            10650,
            [8, 0, 0],
            PrismStep::Resolved {
                pattern: Pattern::Perimeter,
                damage: 0,
                health_after: 100,
            },
        ),
    ] {
        append(
            f,
            db,
            confirmed,
            time,
            PrismTransition::Step {
                player_position: position,
                step,
            },
        );
    }
}

fn attack(health: u32) -> PrismTransition {
    PrismTransition::Attack {
        player_position: [7, 0, 1],
        damage: 40,
        health_before: health,
        health_after: health - 40,
        phase_changed: health == 200,
    }
}

fn request() -> ChallengeRequest<'static> {
    ChallengeRequest {
        operation_id: "prism-terminal",
        expected_revision: 1,
        command: Command::Confirm {
            run_id: 1,
            objective: Objective::PrismCleared,
        },
        proof_event_id: None,
        position: None,
    }
}

#[test]
fn prism_atomic_terminal_rolls_back_foreign_target_and_resolves_concurrent_exact_retry() {
    let Some(mut f) = Fixture::new(ContractId::PrismDiscipline) else {
        return;
    };
    let mut db = database(&f.url);
    let inventory = db.inventory_for(&f.character).unwrap();
    let progression = db.progression_for(&f.character).unwrap();
    let campaign = db.campaign_state_for(&f.account, &f.character).unwrap();
    f.join(&mut db);
    f.start(&mut db, "prism-start");
    let proof = prefix(&f, &mut db);
    let count = total_events(&f);
    assert!(db
        .apply_challenge_prism_command(&f.character, &request(), f.context(), 99, &proof)
        .is_err());
    assert_eq!(total_events(&f), count);
    assert_eq!(f.state(&mut db).active.unwrap().objectives, 0);
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let f = f.clone();
            let proof = proof.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut db = database(&f.url);
                barrier.wait();
                db.apply_challenge_prism_command(&f.character, &request(), f.context(), 42, &proof)
                    .unwrap()
            })
        })
        .collect();
    let receipts: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(receipts.iter().filter(|r| r.replayed).count(), 1);
    assert_eq!(receipts[0].transition, receipts[1].transition);
    assert_eq!(total_events(&f), count + 2);
    let record = f.state(&mut db).records[0].clone();
    let mut changed = proof.clone();
    changed.elapsed_ms += 1;
    assert!(db
        .apply_challenge_prism_command(&f.character, &request(), f.context(), 42, &changed)
        .is_err());
    assert!(db
        .apply_challenge_prism_command(&f.character, &request(), f.context(), 99, &proof)
        .is_err());
    f.session = format!("meridian-after-prism-{}", unique());
    f.contract = ContractId::MeridianCircuit;
    f.join(&mut db);
    f.start(&mut db, "meridian-after-prism");
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
