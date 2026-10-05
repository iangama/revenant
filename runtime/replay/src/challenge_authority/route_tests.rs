use super::*;
use crate::RouteEvidence;
use revenant_challenges::{modifiers::Preset, MODIFIER_REVISION};
use std::collections::{BTreeMap, VecDeque};

fn path(start: [i32; 3], target: [i32; 3]) -> Vec<[i32; 3]> {
    let mut previous = BTreeMap::from([(start, start)]);
    let mut queue = VecDeque::from([start]);
    while let Some(p) = queue.pop_front() {
        if p == target {
            break;
        }
        for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
            let next = [p[0] + dx, 0, p[2] + dz];
            if !previous.contains_key(&next) && revenant_activities::meridian::walkable(next) {
                previous.insert(next, p);
                queue.push_back(next);
            }
        }
    }
    let mut reverse = vec![target];
    while *reverse.last().unwrap() != start {
        reverse.push(previous[reverse.last().unwrap()]);
    }
    reverse.pop();
    reverse.reverse();
    reverse
}

pub(super) fn fixture(preset: Preset) -> (Vec<ReplayEvent>, ChallengeState) {
    let (mut events, _) = admission(
        ContractId::MeridianCircuit,
        &ChallengeState::default(),
        false,
    );
    events.pop();
    let mut state = checkpoint(
        &mut events,
        &ChallengeState::default(),
        Command::StartModified {
            contract: ContractId::MeridianCircuit,
            preset,
            equipment: equipment(),
        },
        Some(2),
    );
    let mut position = preset.meridian_entrance().unwrap();
    let mut sequence = 0;
    for target in [[-28, 0, -7], [-30, 0, 0], [-28, 0, 7], position] {
        for next in path(position, target) {
            sequence += 1;
            let run = state.active.as_ref().unwrap();
            let proof = RouteEvidence {
                run_id: run.run_id,
                actor_id: 1,
                sequence,
                elapsed_ms: 70_000 + sequence * 100,
                position: next,
                checkpoint: run.rules.visit_objective(run.objectives, next),
            };
            let command = proof.command_for(&run.rules);
            let id = event(&mut events, Kind::FieldActivity, 1, proof.encode().unwrap());
            if let Some(command) = command {
                state = checkpoint(&mut events, &state, command, Some(id));
            }
            position = next;
        }
    }
    (events, state)
}

#[test]
fn challenge_modified_routes_validate_the_whole_path_and_allow_late_completion() {
    for preset in [Preset::WestApproach, Preset::Pace, Preset::WestApproachPace] {
        let (events, state) = fixture(preset);
        let result = reconstruct(&events).unwrap();
        assert_eq!(result.challenge.unwrap().state, state);
        assert_eq!((result.loot_grants, result.progression_grants), (0, 0));
        let end = state.last_result.unwrap();
        assert_eq!(end.outcome, Outcome::Completed);
        assert_eq!(end.run.rules.preset, preset);
        assert_eq!(
            end.run.elapsed_ms.is_some(),
            preset.time_goal_ms(ContractId::MeridianCircuit).is_some()
        );
        assert!(end.run.elapsed_ms.is_none_or(|t| t > 60_000));
    }
}

#[test]
fn challenge_modified_route_rejects_teleports_missing_steps_foreign_actors_and_forged_time() {
    let (events, _) = fixture(Preset::WestApproachPace);
    let movement = events
        .iter()
        .enumerate()
        .filter(|(_, e)| RouteEvidence::decode(&e.payload).is_ok())
        .nth(1)
        .unwrap()
        .0;
    for variant in 0..5 {
        let mut bad = events.clone();
        let mut proof = RouteEvidence::decode(&bad[movement].payload).unwrap();
        match variant {
            0 => proof.position = [-28, 0, 7],
            1 => proof.sequence += 1,
            2 => proof.elapsed_ms = 0,
            3 => proof.actor_id += 1,
            _ => proof.checkpoint = Some(Objective::Returned),
        }
        bad[movement].payload = proof.encode().unwrap();
        assert!(
            reconstruct(&bad).is_err(),
            "accepted route forgery {variant}"
        );
    }
    let mut bad = events;
    let end = bad.last_mut().unwrap();
    let mut payload = decode_challenge_payload(&end.payload).unwrap();
    let Command::ConfirmTimed { elapsed_ms, .. } = &mut payload.command else {
        unreachable!()
    };
    *elapsed_ms = 10;
    payload.after = apply(
        MODIFIER_REVISION,
        &payload.before,
        payload.before.revision,
        &payload.command,
    )
    .unwrap()
    .state;
    end.payload = encode_challenge_payload(&payload).unwrap();
    assert!(reconstruct(&bad).is_err());
}
