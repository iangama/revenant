use super::*;
use revenant_modules::{ModuleId, BUILD_CATALOG_REVISION};

fn equipment(modules: Vec<ModuleId>) -> Equipment {
    Equipment {
        loadout_revision: 1,
        catalog_revision: BUILD_CATALOG_REVISION.to_owned(),
        weapon_id: "pulse_rifle".to_owned(),
        modules,
    }
}

fn fresh() -> SurvivalRun {
    SurvivalRun::new(&equipment(vec![])).unwrap()
}

fn step(run: &mut SurvivalRun, now: u64, action: Action) -> Effect {
    let update = run.apply(now, action).unwrap();
    *run = update.run;
    update.effect
}

fn walk(run: &mut SurvivalRun, now: &mut u64, target: [i32; 3]) -> Vec<Checkpoint> {
    let mut checkpoints = vec![];
    while run.position() != target {
        let mut position = run.position();
        let axis = if position[0] == target[0] { 2 } else { 0 };
        position[axis] += (target[axis] - position[axis]).signum();
        *now += 200;
        let effect = step(run, *now, Action::Move { position });
        assert!(!effect.defeated);
        checkpoints.extend(effect.checkpoint);
    }
    checkpoints
}

fn hold(run: &mut SurvivalRun, now: &mut u64, duration: u64) -> Vec<Checkpoint> {
    let mut checkpoints = vec![];
    let end = *now + duration;
    while *now < end {
        *now = (*now + 100).min(end);
        let effect = step(run, *now, Action::Observe);
        assert!(!effect.defeated);
        checkpoints.extend(effect.checkpoint);
    }
    checkpoints
}

#[test]
fn ordered_exposed_holds_then_return_complete_without_killing_or_grants() {
    let mut run = fresh();
    let mut now = 100_000; // Waiting in cover never expires the contract.
    step(&mut run, now, Action::Observe);
    walk(&mut run, &mut now, [-8, 0, 4]);
    walk(&mut run, &mut now, WEST_RELAY);
    assert!(hold(&mut run, &mut now, CHARGE_MS - 1).is_empty());
    assert_eq!(hold(&mut run, &mut now, 1), [Checkpoint::WestCharged]);
    // A safe southern detour trades travel for preserving life.
    walk(&mut run, &mut now, [-8, 0, 4]);
    walk(&mut run, &mut now, RESERVE_PAD);
    hold(&mut run, &mut now, RECOVERY_MS);
    assert!(run.reserve_used());
    walk(&mut run, &mut now, [0, 0, 5]);
    walk(&mut run, &mut now, EAST_RELAY);
    assert_eq!(
        hold(&mut run, &mut now, CHARGE_MS),
        [Checkpoint::EastCharged]
    );
    assert!(run.health() > 0);
    walk(&mut run, &mut now, [0, 0, 4]);
    assert_eq!(walk(&mut run, &mut now, SPAWN), [Checkpoint::Returned]);
    assert_eq!(run.phase(), Phase::Completed);
    assert_eq!(run.apply(now + 1, Action::Observe), Err(Error::Terminal));
}

#[test]
fn wrong_station_and_interrupted_hold_cannot_advance() {
    let mut run = fresh();
    let mut now = 0;
    walk(&mut run, &mut now, [0, 0, 4]);
    walk(&mut run, &mut now, EAST_RELAY);
    assert!(hold(&mut run, &mut now, CHARGE_MS).is_empty());
    walk(&mut run, &mut now, WEST_RELAY);
    hold(&mut run, &mut now, 1_000);
    walk(&mut run, &mut now, [-9, 0, 0]);
    walk(&mut run, &mut now, WEST_RELAY);
    assert!(hold(&mut run, &mut now, CHARGE_MS - 1).is_empty());
    assert_eq!(hold(&mut run, &mut now, 1), [Checkpoint::WestCharged]);
}

#[test]
fn reserve_hold_resets_caps_at_build_health_and_cannot_be_reused() {
    let mut run = SurvivalRun::new(&equipment(vec![ModuleId::WardCapacitor])).unwrap();
    assert_eq!(run.max_health(), 120);
    let mut now = 0;
    walk(&mut run, &mut now, RESERVE_PAD);
    hold(&mut run, &mut now, RECOVERY_MS);
    assert!(!run.reserve_used()); // Full health does not waste the reserve.
    run.health = 110;
    step(&mut run, now, Action::Observe);
    hold(&mut run, &mut now, RECOVERY_MS - 1);
    walk(&mut run, &mut now, SPAWN);
    walk(&mut run, &mut now, RESERVE_PAD);
    hold(&mut run, &mut now, RECOVERY_MS - 1);
    assert_eq!(run.health(), 110);
    now += 1;
    assert_eq!(step(&mut run, now, Action::Observe).recovered_health, 10);
    assert_eq!(run.health(), 120);
    run.health = 50;
    hold(&mut run, &mut now, RECOVERY_MS * 2);
    assert_eq!(run.health(), 50);
    let retry = SurvivalRun::new(&equipment(vec![ModuleId::WardCapacitor])).unwrap();
    assert_eq!(retry.health(), 120);
    assert!(!retry.reserve_used());
    assert_eq!(retry.position(), SPAWN);
    assert_eq!(retry.phase(), Phase::West);
    assert_eq!(retry.next_observation_ms(), SHOT_INTERVAL_MS);
}

#[test]
fn fatal_due_shot_precedes_checkpoint_and_movement_to_cover() {
    let mut run = fresh();
    run.position = EAST_RELAY;
    run.health = SHOT_DAMAGE;
    run.phase = Phase::East;
    run.charge_started_ms = Some(0);
    let result = run.apply(CHARGE_MS, Action::Observe).unwrap();
    assert!(result.effect.defeated);
    assert_eq!(result.effect.checkpoint, None);
    assert_eq!(result.run.phase(), Phase::East);
    assert_eq!(
        result.run.apply(CHARGE_MS, Action::Observe),
        Err(Error::Terminal)
    );
    run.position = [-7, 0, 2];
    let result = run
        .apply(
            CHARGE_MS,
            Action::Move {
                position: [-6, 0, 2],
            },
        )
        .unwrap();
    assert!(result.effect.defeated);
    assert_eq!(result.run.position(), [-7, 0, 2]);
    assert_eq!(result.effect.recovered_health, 0);
}

#[test]
fn delayed_observations_resolve_one_shot_and_cover_is_symmetric() {
    let mut run = fresh();
    let safe = step(&mut run, 100_000, Action::Observe);
    assert_eq!(safe.incoming_damage, 0);
    run.position = WEST_RELAY;
    let hit = step(&mut run, 200_000, Action::Observe);
    assert_eq!(hit.incoming_damage, SHOT_DAMAGE);
    assert_eq!(run.health(), 88);
    assert_eq!(run.next_observation_ms(), 201_800);
    assert_eq!(step(&mut run, 200_001, Action::Observe).incoming_damage, 0);
}

#[test]
fn movement_and_clock_rejections_do_not_mutate_the_attempt() {
    let run = fresh().apply(50, Action::Observe).unwrap().run;
    for position in [SPAWN, WEST_RELAY, [-4, 1, 4], [-4, 0, 6], [i32::MAX, 0, 4]] {
        assert_eq!(
            run.apply(100, Action::Move { position }),
            Err(Error::InvalidMovement)
        );
    }
    assert_eq!(run.apply(49, Action::Observe), Err(Error::ClockReversed));
    let mut edge = run.clone();
    edge.position = [-4, 0, 2];
    assert_eq!(
        edge.apply(
            100,
            Action::Move {
                position: [-4, 0, 1]
            }
        ),
        Err(Error::InvalidMovement)
    );
    assert_eq!(run.position(), SPAWN);
    assert_eq!(run.health(), 100);
    assert_eq!(run.phase(), Phase::West);
}

#[test]
fn survival_policy_preserves_frozen_catalogs_and_requires_ordered_objectives() {
    use crate::{apply, ChallengeState, Command, ContractId, Objective};
    let start = Command::Start {
        contract: ContractId::LastReserve,
        equipment: equipment(vec![]),
    };
    for (index, policy) in [
        crate::REVISION,
        crate::EXPANDED_REVISION,
        crate::ELITE_REVISION,
        crate::PRISM_REVISION,
        crate::SIGNAL_REVISION,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            ContractId::ALL
                .into_iter()
                .filter(|c| c.available_in(policy))
                .count(),
            index + 2
        );
        assert_eq!(
            apply(policy, &ChallengeState::default(), 0, &start),
            Err(ChallengeError::UnsupportedRevision)
        );
    }
    let mut state = apply(
        crate::SURVIVAL_REVISION,
        &ChallengeState::default(),
        0,
        &start,
    )
    .unwrap()
    .state;
    for objective in [
        Objective::EastRelayCharged,
        Objective::ReserveReturned,
        Objective::SentinelCleared,
    ] {
        assert_eq!(
            apply(
                crate::SURVIVAL_REVISION,
                &state,
                state.revision,
                &Command::Confirm {
                    run_id: 1,
                    objective
                }
            ),
            Err(ChallengeError::WrongObjective)
        );
    }
    for objective in [
        Objective::WestRelayCharged,
        Objective::EastRelayCharged,
        Objective::ReserveReturned,
    ] {
        state = apply(
            crate::SURVIVAL_REVISION,
            &state,
            state.revision,
            &Command::Confirm {
                run_id: 1,
                objective,
            },
        )
        .unwrap()
        .state;
    }
    assert!(state.active.is_none());
    assert_eq!(state.records.len(), 1);
    assert_eq!(state.records[0].rules.seed, 37_007);
    assert_eq!(state.records[0].rules.gameplay_revision, "m37-survival-v1");
    assert!(!state.supports_policy(crate::SIGNAL_REVISION));
}
