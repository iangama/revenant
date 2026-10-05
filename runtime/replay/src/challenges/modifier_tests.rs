use super::*;
use revenant_challenges::{
    apply, modifiers::Preset, Equipment, Objective, MAX_REVISION, MODIFIER_REVISION,
};

fn equipment() -> Equipment {
    Equipment {
        loadout_revision: MAX_REVISION,
        catalog_revision: "m35-v2".into(),
        weapon_id: "scatter_caster".into(),
        modules: vec![
            revenant_modules::ModuleId::TempoRegulator,
            revenant_modules::ModuleId::AblativeShell,
        ],
    }
}

fn objectives(contract: ContractId, preset: Preset) -> Vec<Objective> {
    match contract {
        ContractId::CloseQuarters => vec![Objective::MenderCleared, Objective::LancerCleared],
        ContractId::MeridianCircuit => vec![
            Objective::LensRead,
            Objective::GalleryRead,
            Objective::LogRead,
            Objective::Returned,
        ],
        ContractId::CoolantRecovery => vec![
            Objective::CoolantRetrieved,
            Objective::CoolantTransferred,
            Objective::CoolantDelivered,
        ],
        ContractId::BastionLink => vec![Objective::MenderCleared, Objective::BulwarkCleared],
        ContractId::PrismDiscipline => vec![Objective::PrismCleared],
        ContractId::DistantSignal => vec![Objective::SentinelCleared],
        ContractId::LastReserve => vec![
            Objective::WestRelayCharged,
            Objective::EastRelayCharged,
            Objective::ReserveReturned,
        ],
        ContractId::RelayGauntlet => preset
            .gauntlet_stages()
            .map(|stage| match stage {
                revenant_challenges::gauntlet::Stage::Support => Objective::GauntletSupportCleared,
                revenant_challenges::gauntlet::Stage::Bastion => Objective::GauntletBastionCleared,
                revenant_challenges::gauntlet::Stage::Prism => Objective::GauntletPrismCleared,
            })
            .to_vec(),
    }
}

#[test]
fn challenge_modifier_full_record_catalog_roundtrips_with_maximum_identity_widths() {
    let mut state = ChallengeState::default();
    for contract in ContractId::ALL {
        for preset in Preset::ALL
            .into_iter()
            .filter(|p| p.available_for(contract))
        {
            // Signal requires range >=4; use the rifle for that admission.
            let mut gear = equipment();
            if contract == ContractId::DistantSignal {
                "pulse_rifle".clone_into(&mut gear.weapon_id);
            }
            let command = if preset.is_baseline() {
                Command::Start {
                    contract,
                    equipment: gear,
                }
            } else {
                Command::StartModified {
                    contract,
                    preset,
                    equipment: gear,
                }
            };
            state = apply(MODIFIER_REVISION, &state, state.revision, &command)
                .unwrap()
                .state;
            let run_id = state.active.as_ref().unwrap().run_id;
            for objective in objectives(contract, preset) {
                let command = if preset.time_goal_ms(contract).is_some()
                    && matches!(
                        objective,
                        Objective::Returned | Objective::GauntletPrismCleared
                    ) {
                    Command::ConfirmTimed {
                        run_id,
                        objective,
                        elapsed_ms: MAX_REVISION,
                    }
                } else {
                    Command::Confirm { run_id, objective }
                };
                state = apply(MODIFIER_REVISION, &state, state.revision, &command)
                    .unwrap()
                    .state;
            }
        }
    }
    assert_eq!(state.records.len(), 16);
    let pending = Command::StartModified {
        contract: ContractId::RelayGauntlet,
        preset: Preset::BastionFirstSingleReserve,
        equipment: equipment(),
    };
    state = apply(MODIFIER_REVISION, &state, state.revision, &pending)
        .unwrap()
        .state;
    let offset = MAX_REVISION - state.revision - 1;
    state.revision += offset;
    for run in state
        .records
        .iter_mut()
        .chain(state.active.iter_mut())
        .chain(state.last_result.iter_mut().map(|r| &mut r.run))
    {
        run.run_id += offset;
    }
    state.validate().unwrap();
    let command = Command::Retry {
        run_id: state.active.as_ref().unwrap().run_id,
        equipment: equipment(),
    };
    let result = apply(MODIFIER_REVISION, &state, state.revision, &command).unwrap();
    let payload = ChallengeReplayPayload {
        schema_version: 1,
        policy_revision: MODIFIER_REVISION.into(),
        character_id: "c".repeat(256),
        operation_id: "o".repeat(96),
        before: state,
        command,
        after: result.state,
        first_completion: None,
        proof_event_id: Some(i64::MAX),
    };
    let size = serde_json::to_string(&payload).unwrap().len();
    assert!(
        size <= MAX_PAYLOAD_BYTES,
        "maximum legal modifier payload is {size} bytes"
    );
    let encoded = encode_challenge_payload(&payload).unwrap();
    assert_eq!(decode_challenge_payload(&encoded).unwrap(), payload);
    let mut changed = payload;
    changed.after.active.as_mut().unwrap().rules.preset = Preset::Pace;
    assert!(encode_challenge_payload(&changed).is_err());
}
