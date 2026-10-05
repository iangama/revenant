use super::*;

fn equipment() -> Equipment {
    Equipment {
        loadout_revision: 4,
        catalog_revision: BUILD_CATALOG_REVISION.to_owned(),
        weapon_id: "scatter_caster".to_owned(),
        modules: vec![ModuleId::FocusLens, ModuleId::BreachShunt],
    }
}

fn step(state: &mut ChallengeState, command: &Command) -> Option<ContractId> {
    let transition = apply(REVISION, state, state.revision, command).unwrap();
    *state = transition.state;
    transition.first_completion
}

fn start(state: &mut ChallengeState, contract: ContractId) -> u64 {
    step(
        state,
        &Command::Start {
            contract,
            equipment: equipment(),
        },
    );
    state.active.as_ref().unwrap().run_id
}

fn confirm(state: &mut ChallengeState, run_id: u64, objective: Objective) -> Option<ContractId> {
    step(state, &Command::Confirm { run_id, objective })
}

fn rejected(state: &ChallengeState, command: &Command, expected: ChallengeError) {
    assert_eq!(
        apply(REVISION, state, state.revision, command),
        Err(expected)
    );
}

#[test]
fn combat_requires_each_distinct_target_and_repeats_cannot_farm_records() {
    let mut state = ChallengeState::default();
    let first = start(&mut state, ContractId::CloseQuarters);
    assert_eq!(confirm(&mut state, first, Objective::MenderCleared), None);
    rejected(
        &state,
        &Command::Confirm {
            run_id: first,
            objective: Objective::MenderCleared,
        },
        ChallengeError::WrongObjective,
    );
    rejected(
        &state,
        &Command::Confirm {
            run_id: first,
            objective: Objective::LensRead,
        },
        ChallengeError::WrongObjective,
    );
    assert_eq!(
        confirm(&mut state, first, Objective::LancerCleared),
        Some(ContractId::CloseQuarters)
    );
    let record = state.records[0].clone();
    assert!(state.active.is_none());
    assert_eq!(
        state.last_result.as_ref().unwrap().outcome,
        Outcome::Completed
    );
    rejected(
        &state,
        &Command::Confirm {
            run_id: first,
            objective: Objective::LancerCleared,
        },
        ChallengeError::WrongRun,
    );
    step(
        &mut state,
        &Command::Retry {
            run_id: first,
            equipment: equipment(),
        },
    );
    let second = state.active.as_ref().unwrap().run_id;
    assert!(second > first);
    confirm(&mut state, second, Objective::LancerCleared);
    assert_eq!(confirm(&mut state, second, Objective::MenderCleared), None);
    assert_eq!(state.records, vec![record]);
}

#[test]
fn exploration_accepts_route_planning_but_return_requires_all_three_readings() {
    for order in [
        [
            Objective::LensRead,
            Objective::LogRead,
            Objective::GalleryRead,
        ],
        [
            Objective::GalleryRead,
            Objective::LogRead,
            Objective::LensRead,
        ],
    ] {
        let mut state = ChallengeState::default();
        let run_id = start(&mut state, ContractId::MeridianCircuit);
        for objective in order {
            rejected(
                &state,
                &Command::Confirm {
                    run_id,
                    objective: Objective::Returned,
                },
                ChallengeError::WrongObjective,
            );
            assert_eq!(confirm(&mut state, run_id, objective), None);
        }
        assert!(state.records.is_empty());
        assert_eq!(
            confirm(&mut state, run_id, Objective::Returned),
            Some(ContractId::MeridianCircuit)
        );
        assert_eq!(state.records[0].equipment, equipment());
    }
}

#[test]
fn retry_of_interrupted_attempt_resets_all_progress_and_rejects_old_evidence() {
    let mut state = ChallengeState::default();
    let old = start(&mut state, ContractId::MeridianCircuit);
    confirm(&mut state, old, Objective::LensRead);
    let mut restored: ChallengeState =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    step(
        &mut restored,
        &Command::Retry {
            run_id: old,
            equipment: equipment(),
        },
    );
    let run = restored.active.as_ref().unwrap();
    assert_eq!(run.objectives, 0);
    assert!(run.run_id > old);
    assert_eq!(run.equipment, equipment());
    assert_eq!(
        restored.last_result.as_ref().unwrap().outcome,
        Outcome::Interrupted
    );
    rejected(
        &restored,
        &Command::Confirm {
            run_id: old,
            objective: Objective::GalleryRead,
        },
        ChallengeError::WrongRun,
    );
    assert!(restored.records.is_empty());
}

#[test]
fn abandon_and_death_preserve_equipment_without_completion() {
    for reason in [
        EndReason::Abandoned,
        EndReason::Defeated,
        EndReason::Interrupted,
    ] {
        let mut state = ChallengeState::default();
        let id = start(&mut state, ContractId::CloseQuarters);
        confirm(&mut state, id, Objective::LancerCleared);
        step(&mut state, &Command::End { run_id: id, reason });
        assert!(state.active.is_none());
        assert!(state.records.is_empty());
        assert_eq!(
            state.last_result.as_ref().unwrap().run.equipment,
            equipment()
        );
        let next = start(&mut state, ContractId::MeridianCircuit);
        assert!(next > id);
        rejected(
            &state,
            &Command::Retry {
                run_id: id,
                equipment: equipment(),
            },
            ChallengeError::WrongRun,
        );
    }
}

#[test]
fn stale_unknown_and_overlapping_commands_leave_original_state_untouched() {
    let mut state = ChallengeState::default();
    let id = start(&mut state, ContractId::CloseQuarters);
    let original = state.clone();
    let command = Command::Confirm {
        run_id: id,
        objective: Objective::LancerCleared,
    };
    assert_eq!(
        apply("m37-challenges-v0", &state, state.revision, &command),
        Err(ChallengeError::UnsupportedRevision)
    );
    assert_eq!(
        apply(REVISION, &state, 0, &command),
        Err(ChallengeError::StaleRevision)
    );
    rejected(
        &state,
        &Command::Start {
            contract: ContractId::MeridianCircuit,
            equipment: equipment(),
        },
        ChallengeError::ActiveRun,
    );
    assert_eq!(state, original);
    state.revision = MAX_REVISION;
    assert_eq!(
        apply(REVISION, &state, MAX_REVISION, &command),
        Err(ChallengeError::RevisionOverflow)
    );
}

#[test]
fn invalid_or_noncanonical_equipment_cannot_enter_or_replace_a_valid_attempt() {
    let mut invalid = Vec::new();
    let mut gear = equipment();
    gear.modules.push(ModuleId::FocusLens);
    invalid.push(gear);
    let mut gear = equipment();
    gear.modules.reverse();
    invalid.push(gear);
    let mut gear = equipment();
    gear.weapon_id = "invented_weapon".into();
    invalid.push(gear);
    let mut gear = equipment();
    gear.catalog_revision = "m35-v1".into();
    invalid.push(gear);
    let mut state = ChallengeState::default();
    let run_id = start(&mut state, ContractId::CloseQuarters);
    for equipment in invalid {
        rejected(
            &ChallengeState::default(),
            &Command::Start {
                contract: ContractId::CloseQuarters,
                equipment: equipment.clone(),
            },
            ChallengeError::InvalidEquipment,
        );
        rejected(
            &state,
            &Command::Retry { run_id, equipment },
            ChallengeError::InvalidEquipment,
        );
    }
    assert_eq!(state.active.unwrap().equipment, equipment());
}

#[test]
fn records_reject_other_revisions_seeds_and_impossible_terminal_states() {
    let mut state = ChallengeState::default();
    let id = start(&mut state, ContractId::CloseQuarters);
    confirm(&mut state, id, Objective::LancerCleared);
    confirm(&mut state, id, Objective::MenderCleared);
    let mut changed = state.clone();
    changed.records[0].rules.seed += 1;
    assert_eq!(changed.validate(), Err(ChallengeError::InvalidState));
    let mut changed = state.clone();
    changed.records[0].rules.gameplay_revision = "unknown".into();
    assert_eq!(changed.validate(), Err(ChallengeError::InvalidState));
    let mut changed = state.clone();
    changed.records.push(changed.records[0].clone());
    assert_eq!(changed.validate(), Err(ChallengeError::InvalidState));
    let mut changed = state.clone();
    changed.last_result.as_mut().unwrap().outcome = Outcome::Defeated;
    assert_eq!(changed.validate(), Err(ChallengeError::InvalidState));
    let mut changed = state.clone();
    changed.records.clear();
    assert_eq!(changed.validate(), Err(ChallengeError::InvalidState));
    let mut changed = state.clone();
    changed
        .last_result
        .as_mut()
        .unwrap()
        .run
        .equipment
        .weapon_id = "pulse_rifle".into();
    assert_eq!(changed.validate(), Err(ChallengeError::InvalidState));
    let mut wire = serde_json::to_value(state).unwrap();
    wire["campaign_checkpoint"] = 3.into();
    assert!(serde_json::from_value::<ChallengeState>(wire).is_err());
}

#[test]
fn later_contract_and_equipment_changes_leave_the_first_record_intact() {
    let mut state = ChallengeState::default();
    let id = start(&mut state, ContractId::CloseQuarters);
    confirm(&mut state, id, Objective::MenderCleared);
    confirm(&mut state, id, Objective::LancerCleared);
    let first = state.records[0].clone();
    let mut next_equipment = equipment();
    next_equipment.loadout_revision += 1;
    next_equipment.weapon_id = "pulse_rifle".into();
    next_equipment.modules = vec![];
    step(
        &mut state,
        &Command::Start {
            contract: ContractId::MeridianCircuit,
            equipment: next_equipment.clone(),
        },
    );
    let id = state.active.as_ref().unwrap().run_id;
    for objective in [
        Objective::GalleryRead,
        Objective::LensRead,
        Objective::LogRead,
        Objective::Returned,
    ] {
        confirm(&mut state, id, objective);
    }
    assert_eq!(state.records.len(), 2);
    assert_eq!(state.records[0], first);
    assert_eq!(state.records[1].equipment, next_equipment);
    assert_eq!(state.records[1].rules, ContractId::MeridianCircuit.rules());
    state.validate().unwrap();
}

#[test]
fn expanded_policy_adds_recovery_without_changing_initial_records_or_admitting_it_in_v1() {
    let mut state = ChallengeState::default();
    let combat = start(&mut state, ContractId::CloseQuarters);
    confirm(&mut state, combat, Objective::LancerCleared);
    confirm(&mut state, combat, Objective::MenderCleared);
    let initial_records = state.records.clone();
    let request = Command::Start {
        contract: ContractId::CoolantRecovery,
        equipment: equipment(),
    };
    rejected(&state, &request, ChallengeError::UnsupportedRevision);
    state = apply(EXPANDED_REVISION, &state, state.revision, &request)
        .unwrap()
        .state;
    let run_id = state.active.as_ref().unwrap().run_id;
    let delivery = Command::Confirm {
        run_id,
        objective: Objective::CoolantDelivered,
    };
    assert_eq!(
        apply(EXPANDED_REVISION, &state, state.revision, &delivery),
        Err(ChallengeError::WrongObjective)
    );
    rejected(&state, &delivery, ChallengeError::UnsupportedRevision);
    for objective in [
        Objective::CoolantRetrieved,
        Objective::CoolantTransferred,
        Objective::CoolantDelivered,
    ] {
        state = apply(
            EXPANDED_REVISION,
            &state,
            state.revision,
            &Command::Confirm { run_id, objective },
        )
        .unwrap()
        .state;
    }
    assert_eq!(&state.records[..1], &initial_records);
    assert_eq!(state.records[1].rules.contract_revision, recovery::REVISION);
    assert_eq!(state.records[1].rules.seed, 37_003);
    assert_eq!(state.records[1].equipment, equipment());
    assert_eq!(state.last_result.unwrap().outcome, Outcome::Completed);
}

#[test]
fn both_policies_keep_the_original_contract_transitions_byte_equivalent() {
    for contract in ContractId::INITIAL {
        let before = ChallengeState::default();
        let request = Command::Start {
            contract,
            equipment: equipment(),
        };
        let old = apply(REVISION, &before, 0, &request).unwrap();
        let new = apply(EXPANDED_REVISION, &before, 0, &request).unwrap();
        assert_eq!(
            serde_json::to_string(&old.state).unwrap(),
            serde_json::to_string(&new.state).unwrap()
        );
        assert_eq!(
            old.state.active.unwrap().rules.gameplay_revision,
            GAMEPLAY_REVISION
        );
    }
    assert_eq!(
        ContractId::ALL
            .iter()
            .filter(|c| c.available_in(REVISION))
            .count(),
        2
    );
    assert_eq!(
        ContractId::ALL
            .iter()
            .filter(|c| c.available_in(EXPANDED_REVISION))
            .count(),
        3
    );
}

#[test]
fn elite_policy_preserves_published_catalogs_and_requires_distinct_targets() {
    let command = Command::Start {
        contract: ContractId::BastionLink,
        equipment: equipment(),
    };
    for policy in [REVISION, EXPANDED_REVISION] {
        assert_eq!(
            apply(policy, &ChallengeState::default(), 0, &command),
            Err(ChallengeError::UnsupportedRevision)
        );
    }
    for order in [
        [Objective::MenderCleared, Objective::BulwarkCleared],
        [Objective::BulwarkCleared, Objective::MenderCleared],
    ] {
        let mut state = apply(ELITE_REVISION, &ChallengeState::default(), 0, &command)
            .unwrap()
            .state;
        let run = state.active.as_ref().unwrap().clone();
        assert_eq!(run.rules.seed, 37_004);
        assert_eq!(run.rules.gameplay_revision, "m37-elite-v1");
        assert!(!state.supports_policy(EXPANDED_REVISION));
        let wrong = Command::Confirm {
            run_id: run.run_id,
            objective: Objective::LancerCleared,
        };
        assert_eq!(
            apply(ELITE_REVISION, &state, state.revision, &wrong),
            Err(ChallengeError::WrongObjective)
        );
        let first = Command::Confirm {
            run_id: run.run_id,
            objective: order[0],
        };
        state = apply(ELITE_REVISION, &state, state.revision, &first)
            .unwrap()
            .state;
        assert_eq!(
            apply(ELITE_REVISION, &state, state.revision, &first),
            Err(ChallengeError::WrongObjective)
        );
        let terminal = apply(
            ELITE_REVISION,
            &state,
            state.revision,
            &Command::Confirm {
                run_id: run.run_id,
                objective: order[1],
            },
        )
        .unwrap();
        assert_eq!(terminal.first_completion, Some(ContractId::BastionLink));
        assert_eq!(terminal.state.records[0].equipment, equipment());
        assert_eq!(terminal.state.records[0].objectives, 3);
        assert_eq!(
            terminal.state.last_result.as_ref().unwrap().outcome,
            Outcome::Completed
        );
        assert!(!terminal.state.supports_policy(REVISION));
    }
}

#[test]
fn elite_retry_is_fresh_and_a_repeat_preserves_the_first_record() {
    let mut state = apply(
        ELITE_REVISION,
        &ChallengeState::default(),
        0,
        &Command::Start {
            contract: ContractId::BastionLink,
            equipment: equipment(),
        },
    )
    .unwrap()
    .state;
    for objective in [Objective::MenderCleared, Objective::BulwarkCleared] {
        state = apply(
            ELITE_REVISION,
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
    let record = state.records[0].clone();
    state = apply(
        ELITE_REVISION,
        &state,
        state.revision,
        &Command::Retry {
            run_id: 1,
            equipment: equipment(),
        },
    )
    .unwrap()
    .state;
    let retry = state.active.as_ref().unwrap().run_id;
    assert!(retry > 1);
    assert_eq!(state.active.as_ref().unwrap().objectives, 0);
    assert_eq!(
        apply(
            ELITE_REVISION,
            &state,
            state.revision,
            &Command::Confirm {
                run_id: 1,
                objective: Objective::MenderCleared
            }
        ),
        Err(ChallengeError::WrongRun)
    );
    for objective in [Objective::BulwarkCleared, Objective::MenderCleared] {
        let transition = apply(
            ELITE_REVISION,
            &state,
            state.revision,
            &Command::Confirm {
                run_id: retry,
                objective,
            },
        )
        .unwrap();
        assert_eq!(transition.first_completion, None);
        state = transition.state;
    }
    assert_eq!(state.records, vec![record]);
}

#[test]
fn elite_policy_keeps_all_three_previous_rule_identities_and_catalog_counts() {
    for contract in ContractId::RECOVERY {
        let request = Command::Start {
            contract,
            equipment: equipment(),
        };
        let previous = apply(EXPANDED_REVISION, &ChallengeState::default(), 0, &request).unwrap();
        let expanded = apply(ELITE_REVISION, &ChallengeState::default(), 0, &request).unwrap();
        assert_eq!(
            serde_json::to_string(&previous.state).unwrap(),
            serde_json::to_string(&expanded.state).unwrap()
        );
    }
    for (policy, count) in [(REVISION, 2), (EXPANDED_REVISION, 3), (ELITE_REVISION, 4)] {
        assert_eq!(
            ContractId::ALL
                .iter()
                .filter(|c| c.available_in(policy))
                .count(),
            count
        );
    }
}

#[test]
fn prism_policy_preserves_published_catalogs() {
    let start = Command::Start {
        contract: ContractId::PrismDiscipline,
        equipment: equipment(),
    };
    for (policy, count) in [(REVISION, 2), (EXPANDED_REVISION, 3), (ELITE_REVISION, 4)] {
        assert_eq!(
            ContractId::ALL
                .iter()
                .filter(|c| c.available_in(policy))
                .count(),
            count
        );
        assert_eq!(
            apply(policy, &ChallengeState::default(), 0, &start),
            Err(ChallengeError::UnsupportedRevision)
        );
    }
    for contract in ContractId::ELITE {
        let request = Command::Start {
            contract,
            equipment: equipment(),
        };
        assert_eq!(
            apply(ELITE_REVISION, &ChallengeState::default(), 0, &request),
            apply(PRISM_REVISION, &ChallengeState::default(), 0, &request)
        );
    }
}

#[test]
fn prism_terminal_and_retry_preserve_first_record() {
    let start = Command::Start {
        contract: ContractId::PrismDiscipline,
        equipment: equipment(),
    };
    let state = apply(PRISM_REVISION, &ChallengeState::default(), 0, &start)
        .unwrap()
        .state;
    let rules = &state.active.as_ref().unwrap().rules;
    assert_eq!(rules.seed, 37005);
    assert_eq!(rules.gameplay_revision, "m37-prism-v1");
    assert!(!state.supports_policy(ELITE_REVISION));
    for objective in [
        Objective::BulwarkCleared,
        Objective::MenderCleared,
        Objective::Returned,
    ] {
        assert_eq!(
            apply(
                PRISM_REVISION,
                &state,
                1,
                &Command::Confirm {
                    run_id: 1,
                    objective
                }
            ),
            Err(ChallengeError::WrongObjective)
        );
    }
    let done = apply(
        PRISM_REVISION,
        &state,
        1,
        &Command::Confirm {
            run_id: 1,
            objective: Objective::PrismCleared,
        },
    )
    .unwrap();
    assert_eq!(done.first_completion, Some(ContractId::PrismDiscipline));
    assert_eq!(done.state.records[0].objectives, 1);
    let fresh = apply(
        PRISM_REVISION,
        &done.state,
        2,
        &Command::Retry {
            run_id: 1,
            equipment: equipment(),
        },
    )
    .unwrap()
    .state;
    assert_eq!(fresh.active.as_ref().unwrap().objectives, 0);
    assert_eq!(fresh.active.as_ref().unwrap().run_id, 3);
    assert_eq!(fresh.records, done.state.records);
    assert_eq!(
        apply(
            PRISM_REVISION,
            &fresh,
            3,
            &Command::Confirm {
                run_id: 1,
                objective: Objective::PrismCleared
            }
        ),
        Err(ChallengeError::WrongRun)
    );
    let repeat = apply(
        PRISM_REVISION,
        &fresh,
        3,
        &Command::Confirm {
            run_id: 3,
            objective: Objective::PrismCleared,
        },
    )
    .unwrap();
    assert!(repeat.first_completion.is_none());
    assert_eq!(repeat.state.records, done.state.records);
}

#[test]
fn signal_policy_freezes_prior_catalogs_and_only_accepts_its_sentinel_objective() {
    let start = Command::Start {
        contract: ContractId::DistantSignal,
        equipment: Equipment {
            weapon_id: "rail_driver".into(),
            ..equipment()
        },
    };
    for (policy, catalog) in [
        (REVISION, &ContractId::INITIAL[..]),
        (EXPANDED_REVISION, &ContractId::RECOVERY[..]),
        (ELITE_REVISION, &ContractId::ELITE[..]),
        (PRISM_REVISION, &ContractId::PRISM[..]),
    ] {
        let available: Vec<_> = ContractId::ALL
            .into_iter()
            .filter(|c| c.available_in(policy))
            .collect();
        assert_eq!(available, catalog);
        assert_eq!(
            apply(policy, &ChallengeState::default(), 0, &start),
            Err(ChallengeError::UnsupportedRevision)
        );
    }
    let state = apply(SIGNAL_REVISION, &ChallengeState::default(), 0, &start)
        .unwrap()
        .state;
    for objective in [
        Objective::PrismCleared,
        Objective::MenderCleared,
        Objective::Returned,
    ] {
        assert_eq!(
            apply(
                SIGNAL_REVISION,
                &state,
                1,
                &Command::Confirm {
                    run_id: 1,
                    objective
                }
            ),
            Err(ChallengeError::WrongObjective)
        );
    }
    let done = apply(
        SIGNAL_REVISION,
        &state,
        1,
        &Command::Confirm {
            run_id: 1,
            objective: Objective::SentinelCleared,
        },
    )
    .unwrap();
    assert_eq!(done.first_completion, Some(ContractId::DistantSignal));
    assert_eq!(done.state.records.len(), 1);
    assert!(!done.state.supports_policy(PRISM_REVISION));
    let retry = apply(
        SIGNAL_REVISION,
        &done.state,
        2,
        &Command::Retry {
            run_id: 1,
            equipment: Equipment {
                weapon_id: "rail_driver".into(),
                ..equipment()
            },
        },
    )
    .unwrap()
    .state;
    assert_eq!(retry.active.as_ref().unwrap().objectives, 0);
    assert_eq!(retry.active.as_ref().unwrap().run_id, 3);
    let repeated = apply(
        SIGNAL_REVISION,
        &retry,
        3,
        &Command::Confirm {
            run_id: 3,
            objective: Objective::SentinelCleared,
        },
    )
    .unwrap();
    assert_eq!(repeated.first_completion, None);
    assert_eq!(repeated.state.records, done.state.records);
}
