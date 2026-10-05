//! Authenticated board and finite challenge sessions, separate from campaign saves.
use revenant_challenges::modifiers::Preset;
use std::io;

#[cfg(test)]
use revenant_challenges::REVISION;
use revenant_challenges::{ChallengeRun, ChallengeState, ContractId, Outcome};
use revenant_protocol::{
    ChallengeContract, ChallengeEquipment, ChallengeResult, ChallengeSnapshot,
    ChallengeStateRequest,
};

use super::Persistence;

mod elite;
mod gauntlet;
mod prism;
mod recovery;
mod route;
mod runtime;
mod signal;
mod survival;
pub(super) use runtime::{Entry, Runtime};

pub(super) fn entry(
    request: &revenant_protocol::ChallengeJoinRequest,
    policy: &'static str,
) -> Result<Entry, io::Error> {
    let contract = ContractId::ALL
        .into_iter()
        .find(|c| c.available_in(policy) && c.as_str() == request.contract_id)
        .ok_or_else(|| io::Error::other("unknown challenge contract"))?;
    let preset = Preset::ALL
        .into_iter()
        .find(|p| p.as_str() == request.preset_id.as_deref().unwrap_or("baseline"))
        .filter(|p| {
            p.available_for(contract)
                && (p.is_baseline() || policy == revenant_challenges::MODIFIER_REVISION)
        })
        .ok_or_else(|| io::Error::other("unsupported challenge modifiers"))?;
    Ok(Entry {
        preset,
        policy,
        operation_id: request.operation_id.clone(),
        expected_revision: request.expected_revision,
        contract,
        retry_run_id: request.retry_run_id,
    })
}

pub(super) fn board(
    persistence: &mut Persistence,
    account: &str,
    owned_characters: &[String],
    current_content: bool,
    policy: &str,
    mastery_capable: bool,
    request: &ChallengeStateRequest,
) -> Result<ChallengeSnapshot, Box<dyn std::error::Error>> {
    validate_query(owned_characters, current_content, request)?;
    let (state, archive) = if mastery_capable {
        let (state, archive) = persistence
            .challenge_archive_for(account, &request.character_id)
            .map_err(|e| e as Box<dyn std::error::Error>)?;
        (state, Some(archive))
    } else {
        (
            persistence
                .challenge_state_for(account, &request.character_id)
                .map_err(|e| e as Box<dyn std::error::Error>)?,
            None,
        )
    };
    if !state.supports_policy(policy) {
        return Err(io::Error::other("challenge save requires expanded challenge content").into());
    }
    let mut board = snapshot(&request.character_id, &state, policy);
    board.mastery = archive
        .as_ref()
        .map(|archive| Box::new(mastery_snapshot(archive)));
    Ok(board)
}

fn mastery_snapshot(
    archive: &revenant_challenges::mastery::Archive,
) -> revenant_protocol::ChallengeMasteryArchive {
    use revenant_protocol::{
        ChallengeBadgeRecord, ChallengeMasteryArchive, ChallengeMasteryAssessment,
        ChallengeMasteryAttempt, ChallengeMasteryRecord,
    };
    ChallengeMasteryArchive {
        revision: revenant_challenges::mastery::REVISION.into(),
        records: archive
            .records
            .iter()
            .map(|record| ChallengeMasteryRecord {
                goal: record.goal.as_str().into(),
                run_id: record.run_id,
            })
            .collect(),
        badges: archive
            .badges()
            .iter()
            .map(|record| ChallengeBadgeRecord {
                badge: record.badge.as_str().into(),
                run_id: record.run_id,
            })
            .collect(),
        last_attempt: archive
            .last_attempt
            .as_ref()
            .map(|attempt| ChallengeMasteryAttempt {
                run_id: attempt.run_id,
                assessments: attempt
                    .assessments
                    .iter()
                    .map(|assessment| ChallengeMasteryAssessment {
                        goal: assessment.goal.as_str().into(),
                        reason: assessment.reason.as_str().into(),
                    })
                    .collect(),
                route_moves: attempt.route_moves,
                prism_damage: attempt.prism_damage,
            }),
    }
}

fn validate_query(
    owned_characters: &[String],
    current_content: bool,
    request: &ChallengeStateRequest,
) -> Result<(), io::Error> {
    if !current_content || !owned_characters.contains(&request.character_id) {
        return Err(io::Error::other(
            "challenge query requires current capability and owned character",
        ));
    }
    Ok(())
}

fn contract(id: ContractId) -> ChallengeContract {
    let rules = id.rules();
    ChallengeContract {
        contract_id: id.as_str().to_owned(),
        contract_revision: rules.contract_revision,
        gameplay_revision: rules.gameplay_revision,
        seed: rules.seed,
        preset_id: None,
        objective_count: match id {
            ContractId::CloseQuarters | ContractId::BastionLink => 2,
            ContractId::MeridianCircuit => 4,
            ContractId::CoolantRecovery | ContractId::LastReserve | ContractId::RelayGauntlet => 3,
            ContractId::PrismDiscipline | ContractId::DistantSignal => 1,
        },
    }
}

fn run(value: &ChallengeRun) -> revenant_protocol::ChallengeRun {
    revenant_protocol::ChallengeRun {
        run_id: value.run_id,
        contract: ChallengeContract {
            contract_revision: value.rules.contract_revision.clone(),
            gameplay_revision: value.rules.gameplay_revision.clone(),
            preset_id: (!value.rules.preset.is_baseline())
                .then(|| value.rules.preset.as_str().into()),
            ..contract(value.rules.contract)
        },
        elapsed_ms: value.elapsed_ms,
        objectives: value.objectives,
        equipment: ChallengeEquipment {
            loadout_revision: value.equipment.loadout_revision,
            catalog_revision: value.equipment.catalog_revision.clone(),
            weapon_id: value.equipment.weapon_id.clone(),
            modules: value
                .equipment
                .modules
                .iter()
                .map(ToString::to_string)
                .collect(),
        },
    }
}

fn snapshot(character: &str, state: &ChallengeState, policy: &str) -> ChallengeSnapshot {
    ChallengeSnapshot {
        character_id: character.to_owned(),
        policy_revision: policy.to_owned(),
        state_revision: state.revision,
        contracts: ContractId::ALL
            .into_iter()
            .filter(|c| c.available_in(policy))
            .map(contract)
            .collect(),
        active: state.active.as_ref().map(run),
        last_result: state.last_result.as_ref().map(|result| ChallengeResult {
            run: run(&result.run),
            outcome: match result.outcome {
                Outcome::Completed => "completed",
                Outcome::Abandoned => "abandoned",
                Outcome::Defeated => "defeated",
                Outcome::Interrupted => "interrupted",
            }
            .to_owned(),
        }),
        records: state.records.iter().map(run).collect(),
        elapsed_ms: None,
        mastery: None,
        variants: if policy == revenant_challenges::MODIFIER_REVISION {
            ContractId::ALL
                .into_iter()
                .flat_map(|id| {
                    Preset::ALL
                        .into_iter()
                        .filter(move |p| !p.is_baseline() && p.available_for(id))
                        .map(move |p| revenant_protocol::ChallengeVariant {
                            contract_id: id.as_str().into(),
                            preset_id: p.as_str().into(),
                            time_goal_ms: p.time_goal_ms(id),
                        })
                })
                .collect()
        } else {
            vec![]
        },
    }
}

#[cfg(test)]
#[path = "tests/challenge_board.rs"]
mod board_tests;

#[cfg(test)]
#[path = "tests/challenge_runtime.rs"]
mod runtime_tests;

#[cfg(test)]
#[path = "tests/challenge_tcp.rs"]
mod tcp_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use revenant_challenges::{apply, Command, Equipment, Objective};

    #[test]
    fn challenge_board_requires_owned_character_and_current_content() {
        let owned = vec!["local:owner:operator".to_owned()];
        let mut query = ChallengeStateRequest {
            character_id: owned[0].clone(),
        };
        assert!(validate_query(&owned, true, &query).is_ok());
        assert!(validate_query(&owned, false, &query).is_err());
        query.character_id = "local:foreign:operator".into();
        assert!(validate_query(&owned, true, &query).is_err());
        assert!(validate_query(&[], true, &query).is_err());
    }

    #[test]
    fn challenge_board_projects_verified_results_and_wire_roundtrips() {
        let equipment = Equipment {
            catalog_revision: revenant_modules::BUILD_CATALOG_REVISION.into(),
            loadout_revision: 0,
            weapon_id: "pulse_rifle".into(),
            modules: vec![],
        };
        let mut state = apply(
            REVISION,
            &ChallengeState::default(),
            0,
            &Command::Start {
                contract: ContractId::CloseQuarters,
                equipment,
            },
        )
        .unwrap()
        .state;
        for objective in [Objective::MenderCleared, Objective::LancerCleared] {
            state = apply(
                REVISION,
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
        let board = snapshot("operator", &state, REVISION);
        assert_eq!(board.contracts.len(), 2);
        assert_eq!(board.records.len(), 1);
        assert_eq!(board.records[0].contract.seed, 37001);
        assert_eq!(board.last_result.as_ref().unwrap().outcome, "completed");
        assert_eq!(board.state_revision, 3);
        assert!(board.active.is_none());
        let message = revenant_protocol::ServerMessage::ChallengeSnapshot(board);
        let mut frame = vec![];
        revenant_protocol::write_message(&mut frame, &message).unwrap();
        let decoded: revenant_protocol::ServerMessage =
            revenant_protocol::read_message(&mut frame.as_slice()).unwrap();
        assert_eq!(decoded, message);
    }
}
