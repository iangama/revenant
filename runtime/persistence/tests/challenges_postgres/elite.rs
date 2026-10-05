use super::{database, total_events, unique, Fixture, RecoveryDriver};
use postgres::{Client, NoTls};
use revenant_challenges::recovery::{Action, DELIVERY, DWELL_MS, INTAKE, TRANSFER};
use revenant_challenges::{Command, ContractId, Objective};
use revenant_persistence::{ChallengeRequest, NewReplayEvent, Persistence};
use revenant_replay::{encode_elite_evidence, EliteEvidence, EliteTransition};
use std::{
    sync::{Arc, Barrier},
    thread,
};

fn append(f: &Fixture, db: &mut Persistence, proof: &EliteEvidence) {
    let (kind, actor) = match proof.transition {
        EliteTransition::Started { bulwark_id, .. } => {
            ("enemy_spawned", i64::try_from(bulwark_id).unwrap())
        }
        _ => ("field_activity", 41),
    };
    db.append_replay_event(&NewReplayEvent {
        event_type: kind,
        session_id: &f.session,
        account_id: &f.account,
        activity_id: Some(f.contract.activity_id()),
        actor_id: Some(actor),
        payload: &encode_elite_evidence(proof).unwrap(),
    })
    .unwrap();
}

fn attack(time: u64, target: u64, health: u32) -> EliteEvidence {
    EliteEvidence {
        elapsed_ms: time,
        previous_confirmed_ms: time.saturating_sub(250),
        transition: EliteTransition::Attack {
            target_id: target,
            player_position: [10, 0, -8],
            damage: 40,
            health_before: health,
            health_after: health.saturating_sub(40),
        },
    }
}

#[test]
#[allow(clippy::too_many_lines)] // One atomic encounter, terminal race and subsequent policy upgrades.
fn elite_atomic_deaths_retry_and_later_recovery_preserve_character_history() {
    let Some(mut f) = Fixture::new(ContractId::BastionLink) else {
        return;
    };
    let mut db = database(&f.url);
    let inventory = db.inventory_for(&f.character).unwrap();
    let progression = db.progression_for(&f.character).unwrap();
    let campaign = db.campaign_state_for(&f.account, &f.character).unwrap();
    f.join(&mut db);
    f.start(&mut db, "elite-start");
    append(
        &f,
        &mut db,
        &EliteEvidence {
            elapsed_ms: 0,
            previous_confirmed_ms: 0,
            transition: EliteTransition::Started {
                composition: "bastion_link".into(),
                bulwark_id: 42,
                partner_id: 43,
                player_health: 100,
            },
        },
    );
    append(&f, &mut db, &attack(0, 43, 120));
    append(&f, &mut db, &attack(250, 43, 80));
    let proof = attack(500, 43, 40);
    let mut request = ChallengeRequest {
        operation_id: "elite-mender",
        expected_revision: 1,
        command: Command::Confirm {
            run_id: 1,
            objective: Objective::BulwarkCleared,
        },
        proof_event_id: None,
        position: None,
    };
    let count = total_events(&f);
    assert!(db
        .apply_challenge_elite_command(&f.character, &request, f.context(), &proof)
        .is_err());
    assert_eq!(total_events(&f), count); // Fatal row and wrong checkpoint both rolled back.
    assert_eq!(f.state(&mut db).active.unwrap().objectives, 0);
    request.command = Command::Confirm {
        run_id: 1,
        objective: Objective::MenderCleared,
    };
    let accepted = db
        .apply_challenge_elite_command(&f.character, &request, f.context(), &proof)
        .unwrap();
    assert_eq!(accepted.state.active.as_ref().unwrap().objectives, 2);
    assert!(db
        .challenge_archive_for(&f.account, &f.character)
        .unwrap()
        .1
        .records
        .is_empty());
    assert!(
        db.apply_challenge_elite_command(&f.character, &request, f.context(), &proof)
            .unwrap()
            .replayed
    );
    let mut changed = proof.clone();
    changed.elapsed_ms += 1;
    assert!(db
        .apply_challenge_elite_command(&f.character, &request, f.context(), &changed)
        .is_err());
    assert_eq!(total_events(&f), count + 2);
    for index in 0..5u32 {
        append(
            &f,
            &mut db,
            &attack(750 + u64::from(index) * 250, 42, 240 - index * 40),
        );
    }
    let proof = attack(2000, 42, 40);
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let f = f.clone();
            let proof = proof.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut db = database(&f.url);
                barrier.wait();
                db.apply_challenge_elite_command(
                    &f.character,
                    &ChallengeRequest {
                        operation_id: "elite-terminal",
                        expected_revision: 2,
                        command: Command::Confirm {
                            run_id: 1,
                            objective: Objective::BulwarkCleared,
                        },
                        proof_event_id: None,
                        position: None,
                    },
                    f.context(),
                    &proof,
                )
                .unwrap()
            })
        })
        .collect();
    let receipts: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(receipts.iter().filter(|r| r.replayed).count(), 1);
    assert_eq!(receipts[0].transition, receipts[1].transition);
    let record = f.state(&mut db).records[0].clone();
    let rows_before_archive = total_events(&f);
    let (archived_state, archive) = db.challenge_archive_for(&f.account, &f.character).unwrap();
    assert_eq!(archived_state, f.state(&mut db));
    assert_eq!(
        archive.records,
        vec![revenant_challenges::mastery::Record {
            goal: revenant_challenges::mastery::Goal::TargetPriority,
            run_id: 1,
        }]
    );
    assert!(archive.badges().is_empty());
    assert_eq!(
        database(&f.url)
            .challenge_archive_for(&f.account, &f.character)
            .unwrap()
            .1,
        archive
    );
    assert!(db
        .challenge_archive_for("local:foreign", &f.character)
        .is_err());
    assert_eq!(total_events(&f), rows_before_archive);
    let deaths: i64 = Client::connect(&f.url, NoTls).unwrap().query_one(
        "SELECT count(*) FROM replay_events WHERE session_id = $1 AND event_type = 'enemy_died'", &[&f.session]).unwrap().get(0);
    assert_eq!(deaths, 2);
    f.session = format!("recovery-after-elite-{}", unique());
    f.contract = ContractId::CoolantRecovery;
    f.join(&mut db);
    let state = f.start(&mut db, "recovery-after-elite");
    let mut recovery = RecoveryDriver::new(state.active.unwrap().run_id);
    recovery.walk(&f, &mut db, INTAKE);
    recovery.walk(&f, &mut db, TRANSFER);
    recovery.store(&f, &mut db, recovery.time + DWELL_MS, Action::Observe);
    recovery.walk(&f, &mut db, DELIVERY);
    recovery.store(&f, &mut db, recovery.time + DWELL_MS, Action::Observe);
    f.session = format!("meridian-after-elite-{}", unique());
    f.contract = ContractId::MeridianCircuit;
    f.join(&mut db);
    f.start(&mut db, "meridian-after-elite");
    for objective in [
        Objective::LensRead,
        Objective::GalleryRead,
        Objective::LogRead,
        Objective::Returned,
    ] {
        f.visit(&mut db, objective);
    }
    let state = f.state(&mut db);
    assert_eq!(state.records.len(), 3);
    assert_eq!(state.records[0], record);
    let after = db
        .challenge_archive_for(&f.account, &f.character)
        .unwrap()
        .1;
    assert_eq!(after.records, archive.records);
    assert_eq!(
        after.last_attempt.unwrap().assessments[0].reason,
        revenant_challenges::mastery::Reason::UseWestRoute
    );
    assert_eq!(db.inventory_for(&f.character).unwrap(), inventory);
    assert_eq!(db.progression_for(&f.character).unwrap(), progression);
    assert_eq!(
        db.campaign_state_for(&f.account, &f.character).unwrap(),
        campaign
    );
}
