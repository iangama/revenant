use super::*;
use crate::{
    apply, ChallengeState, Command, Equipment, Objective, GAUNTLET_REVISION, MODIFIER_REVISION,
};

fn equipment() -> Equipment {
    Equipment {
        catalog_revision: "m35-v2".into(),
        loadout_revision: 0,
        weapon_id: "pulse_rifle".into(),
        modules: vec![],
    }
}

fn step(state: &mut ChallengeState, command: &Command) -> Option<ContractId> {
    let timed;
    let command = if let Command::Confirm {
        run_id,
        objective: objective @ (Objective::Returned | Objective::GauntletPrismCleared),
    } = command
    {
        if state
            .active
            .as_ref()
            .is_some_and(|r| r.rules.preset.time_goal_ms(r.rules.contract).is_some())
        {
            timed = Command::ConfirmTimed {
                run_id: *run_id,
                objective: *objective,
                elapsed_ms: 100_000,
            };
            &timed
        } else {
            command
        }
    } else {
        command
    };
    let next = apply(MODIFIER_REVISION, state, state.revision, command).unwrap();
    *state = next.state;
    next.first_completion
}

fn modified(contract: ContractId, preset: Preset) -> Command {
    Command::StartModified {
        contract,
        preset,
        equipment: equipment(),
    }
}

fn clear(state: &mut ChallengeState, objectives: &[Objective]) {
    let run_id = state.active.as_ref().unwrap().run_id;
    for objective in objectives {
        step(
            state,
            &Command::Confirm {
                run_id,
                objective: *objective,
            },
        );
    }
}

#[test]
fn baseline_json_stays_identical_and_modified_runs_require_v8() {
    for contract in ContractId::ALL {
        let command = Command::Start {
            contract,
            equipment: equipment(),
        };
        let before = ChallengeState::default();
        let old = apply(GAUNTLET_REVISION, &before, 0, &command).unwrap();
        let new = apply(MODIFIER_REVISION, &before, 0, &command).unwrap();
        assert_eq!(
            serde_json::to_string(&old.state).unwrap(),
            serde_json::to_string(&new.state).unwrap()
        );
        let rule = serde_json::to_value(contract.rules()).unwrap();
        assert!(rule.get("preset").is_none());
        assert_eq!(
            serde_json::from_value::<crate::Rules>(rule).unwrap(),
            contract.rules()
        );
    }
    let command = modified(ContractId::MeridianCircuit, Preset::WestApproach);
    for policy in [
        crate::REVISION,
        crate::EXPANDED_REVISION,
        crate::ELITE_REVISION,
        crate::PRISM_REVISION,
        crate::SIGNAL_REVISION,
        crate::SURVIVAL_REVISION,
        GAUNTLET_REVISION,
    ] {
        assert_eq!(
            apply(policy, &ChallengeState::default(), 0, &command),
            Err(ChallengeError::UnsupportedRevision)
        );
    }
    let mut state = ChallengeState::default();
    step(&mut state, &command);
    assert!(!state.supports_policy(GAUNTLET_REVISION));
    assert_eq!(
        state.active.as_ref().unwrap().rules.preset,
        Preset::WestApproach
    );
    assert_eq!(
        apply(
            MODIFIER_REVISION,
            &ChallengeState::default(),
            0,
            &modified(ContractId::MeridianCircuit, Preset::SingleReserve)
        ),
        Err(ChallengeError::InvalidModifiers)
    );
    assert_eq!(
        apply(
            MODIFIER_REVISION,
            &ChallengeState::default(),
            0,
            &modified(ContractId::RelayGauntlet, Preset::Baseline)
        ),
        Err(ChallengeError::InvalidModifiers)
    );
}

#[test]
fn retry_retains_the_preset_and_reversed_order_requires_bastion_first() {
    let mut state = ChallengeState::default();
    step(
        &mut state,
        &modified(ContractId::RelayGauntlet, Preset::BastionFirstSingleReserve),
    );
    assert_eq!(
        apply(
            MODIFIER_REVISION,
            &state,
            state.revision,
            &Command::Confirm {
                run_id: 1,
                objective: Objective::GauntletSupportCleared
            }
        ),
        Err(ChallengeError::WrongObjective)
    );
    clear(&mut state, &[Objective::GauntletBastionCleared]);
    step(
        &mut state,
        &Command::Retry {
            run_id: 1,
            equipment: equipment(),
        },
    );
    let fresh = state.active.as_ref().unwrap();
    assert_eq!(fresh.run_id, 3);
    assert_eq!(fresh.objectives, 0);
    assert_eq!(fresh.rules.preset, Preset::BastionFirstSingleReserve);
    assert_eq!(
        apply(
            MODIFIER_REVISION,
            &state,
            state.revision,
            &Command::Confirm {
                run_id: 1,
                objective: Objective::GauntletBastionCleared
            }
        ),
        Err(ChallengeError::WrongRun)
    );
    clear(
        &mut state,
        &[
            Objective::GauntletBastionCleared,
            Objective::GauntletSupportCleared,
            Objective::GauntletPrismCleared,
        ],
    );
    assert_eq!(state.records.len(), 1);
    assert!(!state.supports_policy(GAUNTLET_REVISION));
}

#[test]
fn first_completion_records_are_scoped_to_the_exact_preset() {
    let mut state = ChallengeState::default();
    let contract = ContractId::MeridianCircuit;
    let objectives = [
        Objective::LensRead,
        Objective::LogRead,
        Objective::GalleryRead,
        Objective::Returned,
    ];
    step(
        &mut state,
        &Command::Start {
            contract,
            equipment: equipment(),
        },
    );
    clear(&mut state, &objectives);
    let original = state.records[0].clone();
    step(&mut state, &modified(contract, Preset::WestApproachPace));
    clear(&mut state, &objectives);
    assert_eq!(state.records.len(), 2);
    assert_eq!(state.records[0], original);
    let records = state.records.clone();
    let run_id = state.last_result.as_ref().unwrap().run.run_id;
    step(
        &mut state,
        &Command::Retry {
            run_id,
            equipment: equipment(),
        },
    );
    clear(&mut state, &objectives[..3]);
    let run_id = state.active.as_ref().unwrap().run_id;
    assert_eq!(
        step(
            &mut state,
            &Command::Confirm {
                run_id,
                objective: Objective::Returned
            }
        ),
        None
    );
    assert_eq!(state.records, records);
    for mutate in [
        |s: &mut ChallengeState| s.records[1].rules.seed += 1,
        |s: &mut ChallengeState| s.records[1].rules.preset = Preset::SingleReserve,
        |s: &mut ChallengeState| s.records[1].rules.gameplay_revision = "m37-baseline-v1".into(),
    ] {
        let mut forged = state.clone();
        mutate(&mut forged);
        assert_eq!(forged.validate(), Err(ChallengeError::InvalidState));
    }
}

#[test]
fn timed_completion_keeps_the_fastest_record_and_accepts_a_missed_goal() {
    let mut state = ChallengeState::default();
    let contract = ContractId::MeridianCircuit;
    for (elapsed_ms, best) in [(70_000, 70_000), (50_000, 50_000), (60_000, 50_000)] {
        step(&mut state, &modified(contract, Preset::Pace));
        clear(
            &mut state,
            &[
                Objective::LensRead,
                Objective::GalleryRead,
                Objective::LogRead,
            ],
        );
        let run_id = state.active.as_ref().unwrap().run_id;
        let plain = Command::Confirm {
            run_id,
            objective: Objective::Returned,
        };
        assert_eq!(
            apply(MODIFIER_REVISION, &state, state.revision, &plain),
            Err(ChallengeError::WrongObjective)
        );
        step(
            &mut state,
            &Command::ConfirmTimed {
                run_id,
                objective: Objective::Returned,
                elapsed_ms,
            },
        );
        assert_eq!(state.records.len(), 1);
        assert_eq!(state.records[0].elapsed_ms, Some(best));
        let last = state.last_result.as_ref().unwrap();
        assert_eq!(last.outcome, crate::Outcome::Completed);
        assert_eq!(last.run.elapsed_ms, Some(elapsed_ms));
    }
}
