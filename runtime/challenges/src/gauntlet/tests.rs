use super::*;

fn equipment() -> Equipment {
    Equipment {
        catalog_revision: "m35-v2".into(),
        loadout_revision: 0,
        weapon_id: "pulse_rifle".into(),
        modules: vec![],
    }
}

fn transfer(run: &GauntletRun, at: u64) -> TransferUpdate {
    run.observe_transfer(at, true)
        .unwrap()
        .run
        .observe_transfer(at + TRANSFER_HOLD_MS, true)
        .unwrap()
}

#[test]
fn ordered_encounters_carry_health_with_two_bounded_transfers() {
    let start = GauntletRun::new(&equipment()).unwrap();
    let first = start.finish_encounter(100, Stage::Support, 90).unwrap();
    assert_eq!(first.phase(), Phase::Transfer(Stage::Support));
    assert_eq!(first.health(), 90);
    let recovered = transfer(&first, 200);
    assert_eq!(recovered.recovered_health, 10);
    assert_eq!(recovered.next_stage, Some(Stage::Bastion));
    assert_eq!(recovered.run.health(), 100);
    assert_eq!(
        recovered.run.observe_transfer(1_500, true),
        Err(Error::WrongPhase)
    );
    let second = recovered
        .run
        .finish_encounter(2_000, Stage::Bastion, 40)
        .unwrap();
    let recovered = transfer(&second, 2_100);
    assert_eq!(recovered.recovered_health, 24);
    assert_eq!(recovered.run.health(), 64);
    assert_eq!(recovered.next_stage, Some(Stage::Prism));
    let end = recovered
        .run
        .finish_encounter(4_000, Stage::Prism, 1)
        .unwrap();
    assert_eq!(end.phase(), Phase::Completed);
    assert_eq!(end.health(), 1);
    assert_eq!(end.observe_transfer(5_000, true), Err(Error::WrongPhase));
    assert_eq!(start.health(), 100); // Pure plans do not mutate accepted state.
}

#[test]
fn exit_hold_resets_and_has_no_overall_deadline() {
    let run = GauntletRun::new(&equipment())
        .unwrap()
        .finish_encounter(100, Stage::Support, 50)
        .unwrap();
    let run = run.observe_transfer(200, true).unwrap().run;
    let run = run.observe_transfer(1_300, false).unwrap().run;
    let run = run.observe_transfer(90_000, true).unwrap().run;
    let early = run.observe_transfer(91_199, true).unwrap();
    assert_eq!(early.next_stage, None);
    assert_eq!(early.recovered_health, 0);
    assert_eq!(early.run.health(), 50);
    let ready = early.run.observe_transfer(91_200, true).unwrap();
    assert_eq!(ready.next_stage, Some(Stage::Bastion));
    assert_eq!(ready.run.health(), 74);
}

#[test]
fn invalid_or_terminal_outcomes_cannot_skip_heal_or_resume() {
    let start = GauntletRun::new(&equipment()).unwrap();
    assert_eq!(
        start.finish_encounter(100, Stage::Bastion, 100),
        Err(Error::WrongPhase)
    );
    assert_eq!(
        start.finish_encounter(100, Stage::Support, 101),
        Err(Error::InvalidHealth)
    );
    let dead = start.finish_encounter(100, Stage::Support, 0).unwrap();
    assert_eq!(dead.phase(), Phase::Defeated);
    assert_eq!(dead.observe_transfer(2_000, true), Err(Error::WrongPhase));
    assert_eq!(
        dead.finish_encounter(2_000, Stage::Support, 100),
        Err(Error::WrongPhase)
    );
    let clear = start.finish_encounter(100, Stage::Support, 20).unwrap();
    assert_eq!(
        clear.finish_encounter(100, Stage::Support, 20),
        Err(Error::WrongPhase)
    );
    assert_eq!(clear.observe_transfer(99, true), Err(Error::ClockReversed));
    let fresh = GauntletRun::new(&equipment()).unwrap();
    assert_eq!(fresh, start);
}

#[test]
fn recovery_respects_admitted_build_maximum() {
    let mut equipment = equipment();
    equipment.modules = vec![revenant_modules::ModuleId::WardCapacitor];
    let run = GauntletRun::new(&equipment).unwrap();
    assert_eq!(run.max_health(), 120);
    let clear = run.finish_encounter(100, Stage::Support, 115).unwrap();
    let recovered = transfer(&clear, 200);
    assert_eq!(recovered.run.health(), 120);
    assert_eq!(recovered.recovered_health, 5);
}

#[test]
fn gauntlet_catalog_keeps_v6_frozen_and_requires_ordered_clears() {
    use crate::{
        apply, ChallengeState, Command, ContractId, Objective, GAUNTLET_REVISION, SURVIVAL_REVISION,
    };
    assert_eq!(
        ContractId::ALL
            .iter()
            .copied()
            .filter(|c| c.available_in(SURVIVAL_REVISION))
            .collect::<Vec<_>>(),
        ContractId::SURVIVAL
    );
    let start = Command::Start {
        contract: ContractId::RelayGauntlet,
        equipment: equipment(),
    };
    assert!(apply(SURVIVAL_REVISION, &ChallengeState::default(), 0, &start).is_err());
    let mut state = apply(GAUNTLET_REVISION, &ChallengeState::default(), 0, &start)
        .unwrap()
        .state;
    for objective in [
        Objective::GauntletBastionCleared,
        Objective::GauntletPrismCleared,
        Objective::MenderCleared,
    ] {
        assert!(apply(
            GAUNTLET_REVISION,
            &state,
            1,
            &Command::Confirm {
                run_id: 1,
                objective
            }
        )
        .is_err());
    }
    for objective in [
        Objective::GauntletSupportCleared,
        Objective::GauntletBastionCleared,
        Objective::GauntletPrismCleared,
    ] {
        state = apply(
            GAUNTLET_REVISION,
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
    assert_eq!(state.records[0].rules.seed, 37008);
    assert!(!state.supports_policy(SURVIVAL_REVISION));
    let fresh = apply(
        GAUNTLET_REVISION,
        &state,
        state.revision,
        &Command::Retry {
            run_id: 1,
            equipment: equipment(),
        },
    )
    .unwrap()
    .state;
    assert_eq!(fresh.active.unwrap().objectives, 0);
    assert_eq!(fresh.records, state.records);
}

#[test]
fn bastion_first_changes_stage_order_and_keeps_one_admitted_health_pool() {
    let run = GauntletRun::with_preset(&equipment(), Preset::BastionFirst).unwrap();
    assert_eq!(run.phase(), Phase::Combat(Stage::Bastion));
    assert_eq!(
        run.finish_encounter(100, Stage::Support, 100),
        Err(Error::WrongPhase)
    );
    let run = run.finish_encounter(100, Stage::Bastion, 50).unwrap();
    let next = transfer(&run, 200);
    assert_eq!(next.next_stage, Some(Stage::Support));
    assert_eq!(next.run.health(), 74);
    let run = next
        .run
        .finish_encounter(2_000, Stage::Support, 50)
        .unwrap();
    let next = transfer(&run, 2_100);
    assert_eq!(next.next_stage, Some(Stage::Prism));
    assert_eq!(
        next.run
            .finish_encounter(4_000, Stage::Prism, 25)
            .unwrap()
            .phase(),
        Phase::Completed
    );
    assert_eq!(
        GauntletRun::with_preset(&equipment(), Preset::WestApproach),
        Err(ChallengeError::InvalidModifiers)
    );
}

fn reserve(run: &GauntletRun, at: u64) -> TransferUpdate {
    run.observe_reserve(at, true)
        .unwrap()
        .run
        .observe_reserve(at + TRANSFER_HOLD_MS, true)
        .unwrap()
}

#[test]
fn reserve_can_be_spent_after_either_stage_but_never_twice() {
    for use_after_first in [false, true] {
        let initial = GauntletRun::with_preset(&equipment(), Preset::SingleReserve).unwrap();
        let mut clear = initial.finish_encounter(100, Stage::Support, 50).unwrap();
        if use_after_first {
            let healed = reserve(&clear, 200);
            assert_eq!(healed.recovered_health, 24);
            assert_eq!(healed.next_stage, None);
            assert_eq!(healed.run.phase(), Phase::Transfer(Stage::Support));
            clear = healed.run;
        }
        let next = transfer(&clear, 2_000);
        assert_eq!(next.recovered_health, 0);
        assert_eq!(next.run.health(), if use_after_first { 74 } else { 50 });
        let clear = next
            .run
            .finish_encounter(4_000, Stage::Bastion, 20)
            .unwrap();
        let healed = reserve(&clear, 4_100);
        assert_eq!(
            healed.recovered_health,
            if use_after_first { 0 } else { 24 }
        );
        assert!(healed.run.reserve_used());
        let repeated = reserve(&healed.run, 5_500);
        assert_eq!(repeated.recovered_health, 0);
        let final_stage = transfer(&repeated.run, 7_000);
        assert_eq!(final_stage.recovered_health, 0);
        let end = final_stage
            .run
            .finish_encounter(9_000, Stage::Prism, 1)
            .unwrap();
        assert_eq!(end.observe_reserve(10_000, true), Err(Error::WrongPhase));
        let retry = GauntletRun::with_preset(&equipment(), Preset::SingleReserve).unwrap();
        assert_eq!(retry, initial);
        assert!(!retry.reserve_used());
    }
}

#[test]
fn reserve_holds_reset_between_pads_and_reject_reversed_clocks() {
    let clear = GauntletRun::with_preset(&equipment(), Preset::BastionFirstSingleReserve)
        .unwrap()
        .finish_encounter(100, Stage::Bastion, 50)
        .unwrap();
    let waiting = clear.observe_reserve(200, true).unwrap().run;
    assert_eq!(waiting.reserve_remaining_ms(), Some(1_200));
    assert_eq!(
        waiting.observe_reserve(199, true),
        Err(Error::ClockReversed)
    );
    let exit = waiting.observe_transfer(1_399, true).unwrap().run;
    assert_eq!(exit.reserve_remaining_ms(), None);
    let back = exit.observe_reserve(1_500, true).unwrap().run;
    assert_eq!(back.transfer_remaining_ms(), None);
    let early = back.observe_reserve(2_699, true).unwrap();
    assert_eq!(early.recovered_health, 0);
    let healed = early.run.observe_reserve(2_700, true).unwrap();
    assert_eq!(healed.recovered_health, 24);
    let next = transfer(&healed.run, 3_000);
    assert_eq!(next.next_stage, Some(Stage::Support));
    assert_eq!(
        next.run.observe_reserve(4_300, true),
        Err(Error::WrongPhase)
    );
    for stage in [Stage::Support, Stage::Bastion] {
        let pad = stage.reserve_position().unwrap();
        assert!(stage.walkable(pad));
        assert_ne!(pad, stage.entrance());
    }
}

#[test]
fn reserve_does_not_waste_a_charge_at_full_health_and_caps_at_the_build_maximum() {
    let mut gear = equipment();
    gear.modules = vec![revenant_modules::ModuleId::WardCapacitor];
    let first = GauntletRun::with_preset(&gear, Preset::SingleReserve)
        .unwrap()
        .finish_encounter(100, Stage::Support, 120)
        .unwrap();
    let full = reserve(&first, 200);
    assert_eq!(full.recovered_health, 0);
    assert!(!full.run.reserve_used());
    let next = transfer(&full.run, 1_500);
    let second = next
        .run
        .finish_encounter(3_000, Stage::Bastion, 115)
        .unwrap();
    let healed = reserve(&second, 3_100);
    assert_eq!(healed.recovered_health, 5);
    assert_eq!(healed.run.health(), 120);
    assert!(healed.run.reserve_used());
    let baseline = GauntletRun::new(&gear)
        .unwrap()
        .finish_encounter(100, Stage::Support, 100)
        .unwrap();
    assert_eq!(baseline.observe_reserve(200, true), Err(Error::WrongPhase));
}

#[test]
fn pace_is_an_optional_distinction_and_never_invalidates_a_late_clear() {
    for preset in [Preset::Baseline, Preset::Pace, Preset::SingleReservePace] {
        for elapsed in [180_000, 180_001] {
            let first = GauntletRun::with_preset(&equipment(), preset).unwrap();
            assert_eq!(first.pace_met(), None);
            let dead = first.finish_encounter(elapsed, Stage::Support, 0).unwrap();
            assert_eq!(dead.pace_met(), None);
            let first = first.finish_encounter(100, Stage::Support, 100).unwrap();
            let second = transfer(&first, 200)
                .run
                .finish_encounter(2_000, Stage::Bastion, 100)
                .unwrap();
            let third = transfer(&second, 2_100).run;
            let complete = third.finish_encounter(elapsed, Stage::Prism, 100).unwrap();
            assert_eq!(complete.phase(), Phase::Completed);
            assert_eq!(
                complete.pace_met(),
                (preset != Preset::Baseline).then_some(elapsed <= 180_000)
            );
            assert_eq!(
                complete.observe_transfer(elapsed + 1, true),
                Err(Error::WrongPhase)
            );
        }
    }
}
