use super::{database, total_events, Fixture};
use revenant_challenges::{modifiers::Preset, Command, ContractId, Outcome};
use revenant_persistence::ChallengeRequest;
use revenant_replay::RouteEvidence;

fn path(mut start: [i32; 3], target: [i32; 3]) -> Vec<[i32; 3]> {
    let mut points = vec![];
    while start != target {
        let axis = if start[0] == target[0] { 2 } else { 0 };
        start[axis] += (target[axis] - start[axis]).signum();
        points.push(start);
    }
    points
}

#[test]
fn modified_route_steps_are_atomic_idempotent_and_keep_a_proved_late_record() {
    let Some(f) = Fixture::new(ContractId::MeridianCircuit) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    let inventory = db.inventory_for(&f.character).unwrap();
    let progression = db.progression_for(&f.character).unwrap();
    let request = ChallengeRequest {
        operation_id: "modified-route",
        expected_revision: 0,
        command: Command::StartModified {
            contract: f.contract,
            preset: Preset::WestApproachPace,
            equipment: f.equipment(&mut db),
        },
        proof_event_id: None,
        position: None,
    };
    let mut state = db
        .apply_challenge_command(&f.character, &request, f.context())
        .unwrap()
        .state;
    let mut position = [-31, 0, 0];
    let mut sequence = 0;
    for target in [
        [-30, 0, 0],
        [-30, 0, -7],
        [-28, 0, -7],
        [-30, 0, -7],
        [-30, 0, 7],
        [-28, 0, 7],
        [-30, 0, 7],
        [-30, 0, 0],
        position,
    ] {
        for next in path(position, target) {
            sequence += 1;
            let run = state.active.as_ref().unwrap();
            let proof = RouteEvidence {
                run_id: 1,
                actor_id: 41,
                sequence,
                elapsed_ms: 70_000 + sequence * 100,
                position: next,
                checkpoint: run.rules.visit_objective(run.objectives, next),
            };
            if sequence == 2 {
                let count = total_events(&f);
                let mut bad = proof.clone();
                bad.position = [-28, 0, 7];
                assert!(db
                    .apply_challenge_route_step(&f.character, &bad, f.context())
                    .is_err());
                assert_eq!(total_events(&f), count);
                assert_eq!(f.state(&mut db), state);
            }
            state = db
                .apply_challenge_route_step(&f.character, &proof, f.context())
                .unwrap()
                .state;
            let count = total_events(&f);
            if sequence == 2 || state.active.is_none() {
                assert!(
                    db.apply_challenge_route_step(&f.character, &proof, f.context())
                        .unwrap()
                        .replayed
                );
                let mut changed = proof;
                changed.elapsed_ms += 1;
                assert!(db
                    .apply_challenge_route_step(&f.character, &changed, f.context())
                    .is_err());
                assert_eq!(total_events(&f), count);
            }
            position = next;
        }
    }
    drop(db);
    let mut db = database(&f.url);
    assert_eq!(f.state(&mut db), state);
    assert_eq!(state.last_result.unwrap().outcome, Outcome::Completed);
    assert!(state.records[0].elapsed_ms.unwrap() > 60_000);
    assert_eq!(db.inventory_for(&f.character).unwrap(), inventory);
    assert_eq!(db.progression_for(&f.character).unwrap(), progression);
}
