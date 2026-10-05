//! Challenge world adapter using admitted actors, support AI and atomic persistence.
use revenant_challenges::modifiers::Preset;
use std::{error::Error, io};

use revenant_challenges::{
    ChallengeState, Command, ContractId, EndReason, Equipment, Objective, Outcome,
};
use revenant_persistence::{ChallengeRequest, ModuleMutationReplayContext};
use revenant_protocol::{ActivityStart, ActorUpdate, ServerMessage};
use revenant_replay::{SupportEvidence, SupportTransition};

use crate::{actor_spawn_message, Participant, ProtocolGeneration, SessionStage, SharedSession};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(crate) struct Entry {
    pub policy: &'static str,
    pub operation_id: String,
    pub expected_revision: u64,
    pub contract: ContractId,
    pub retry_run_id: Option<u64>,
    pub preset: Preset,
}

pub(crate) struct Runtime {
    pub policy: &'static str,
    pub contract: ContractId,
    pub state: ChallengeState,
    pub(super) route: Option<super::route::RouteRuntime>,
    pub(super) gauntlet: Option<super::gauntlet::GauntletRuntime>,
    pub(super) survival: Option<super::survival::SurvivalRuntime>,
    pub(super) signal: Option<super::signal::SignalRuntime>,
    pub(super) recovery: Option<super::recovery::RecoveryRuntime>,
}

impl SharedSession {
    pub(crate) fn abandon_challenge(
        &mut self,
        player_id: u64,
        intent: &revenant_protocol::ChallengeAbandonIntent,
    ) -> Result<()> {
        let runtime = self
            .challenge
            .as_ref()
            .ok_or_else(|| io::Error::other("challenge runtime missing"))?;
        let player = self
            .participants
            .first()
            .filter(|p| p.actor.id == player_id)
            .ok_or_else(|| io::Error::other("challenge participant missing"))?;
        let receipt = self.persistence.apply_challenge_command(
            &player.character_id,
            &ChallengeRequest {
                operation_id: &intent.operation_id,
                expected_revision: intent.expected_revision,
                command: Command::End {
                    run_id: intent.run_id,
                    reason: EndReason::Abandoned,
                },
                proof_event_id: None,
                position: None,
            },
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: runtime.contract.activity_id(),
                actor_id: i64::try_from(player_id)?,
            },
        );
        let (status, message) = match &receipt {
            Ok(r) if r.replayed => ("replayed", "challenge already abandoned"),
            Ok(_) => ("accepted", "challenge abandoned"),
            Err(_) => (
                "unconfirmed",
                "challenge abandonment could not be confirmed",
            ),
        };
        let _ = player.outbound.send(ServerMessage::ChallengeActionResult(
            revenant_protocol::ChallengeActionResult {
                operation_id: intent.operation_id.clone(),
                status: status.into(),
                message: message.into(),
            },
        ));
        self.challenge.as_mut().unwrap().state = receipt?.state;
        self.project_challenge();
        Ok(())
    }

    pub(crate) fn prepare_challenge_entry(&mut self, player: &Participant) -> Result<()> {
        let Some(entry) = &player.challenge_entry else {
            return Ok(());
        };
        if !entry.contract.available_in(entry.policy)
            || (matches!(
                entry.contract,
                ContractId::PrismDiscipline | ContractId::RelayGauntlet
            ) && !player.prism_capable)
            || (matches!(
                entry.contract,
                ContractId::BastionLink | ContractId::RelayGauntlet
            ) && !player.elite_capable)
            || self.expected_players != 1
            || !self.participants.is_empty()
            || player.campaign_entry.is_some()
            || player.protocol_generation != ProtocolGeneration::CurrentV2
            || !player.build_capable
            || !player.support_capable
            || !player.exploration_capable
        {
            return Err(io::Error::other("challenge entry requires current solo content").into());
        }
        let state = self
            .persistence
            .challenge_state_for(&player.account_id, &player.character_id)?;
        if !state.supports_policy(entry.policy) || state.revision != entry.expected_revision {
            return Err(io::Error::other("challenge save changed; refresh the board").into());
        }
        if let Some(run_id) = entry.retry_run_id {
            let previous = state
                .active
                .as_ref()
                .or_else(|| state.last_result.as_ref().map(|r| &r.run));
            if previous.is_none_or(|r| {
                r.run_id != run_id
                    || r.rules.contract != entry.contract
                    || r.rules.preset != entry.preset
            }) {
                return Err(io::Error::other("challenge retry refers to another attempt").into());
            }
        }
        self.challenge = Some(Runtime {
            policy: entry.policy,
            contract: entry.contract,
            signal: None,
            survival: None,
            gauntlet: None,
            route: None,
            recovery: (entry.contract == ContractId::CoolantRecovery)
                .then(super::recovery::RecoveryRuntime::new),
            state,
        });
        Ok(())
    }

    pub(crate) fn commit_challenge_entry(&mut self, player: &mut Participant) -> Result<()> {
        let Some(entry) = &player.challenge_entry else {
            return Ok(());
        };
        let equipment = Equipment {
            catalog_revision: revenant_modules::BUILD_CATALOG_REVISION.into(),
            loadout_revision: player.module_state.revision,
            weapon_id: player.equipped_weapon_item_id.clone(),
            modules: player.module_state.loadout.clone(),
        };
        let command = entry.retry_run_id.map_or_else(
            || {
                if entry.preset.is_baseline() {
                    Command::Start {
                        contract: entry.contract,
                        equipment: equipment.clone(),
                    }
                } else {
                    Command::StartModified {
                        contract: entry.contract,
                        preset: entry.preset,
                        equipment: equipment.clone(),
                    }
                }
            },
            |run_id| Command::Retry {
                run_id,
                equipment: equipment.clone(),
            },
        );
        let state = self
            .persistence
            .apply_challenge_command(
                &player.character_id,
                &ChallengeRequest {
                    operation_id: &entry.operation_id,
                    expected_revision: entry.expected_revision,
                    command,
                    proof_event_id: None,
                    position: None,
                },
                ModuleMutationReplayContext {
                    catalog_revision: player.arsenal_catalog(),
                    session_id: &self.session_id,
                    account_id: &player.account_id,
                    activity_id: entry.contract.activity_id(),
                    actor_id: i64::try_from(player.actor.id)?,
                },
            )?
            .state;
        player.actor.position = match entry.contract {
            ContractId::CloseQuarters => revenant_ai::support::ENTRANCE,
            ContractId::RelayGauntlet => entry.preset.gauntlet_stages()[0].entrance(),
            ContractId::MeridianCircuit => entry.preset.meridian_entrance().unwrap(),
            ContractId::CoolantRecovery => revenant_challenges::recovery::SPAWN,
            ContractId::DistantSignal => revenant_challenges::signal::SPAWN,
            ContractId::LastReserve => revenant_challenges::survival::SPAWN,
            ContractId::PrismDiscipline => revenant_ai::prism::ENTRANCE,
            ContractId::BastionLink => revenant_ai::elite::EliteComposition::BastionLink.entrance(),
        };
        self.challenge
            .as_mut()
            .ok_or_else(|| io::Error::other("challenge preparation missing"))?
            .state = state;
        Ok(())
    }

    pub(crate) fn start_challenge(&mut self) -> Result<()> {
        let runtime = self
            .challenge
            .as_ref()
            .ok_or_else(|| io::Error::other("challenge run missing"))?;
        let contract = runtime.contract;
        let modified_route = contract == ContractId::MeridianCircuit
            && !runtime
                .state
                .active
                .as_ref()
                .unwrap()
                .rules
                .preset
                .is_baseline();
        self.stage = SessionStage::Door;
        self.broadcast(ServerMessage::ActivityStart(ActivityStart {
            activity_id: contract.activity_id().into(),
        }));
        self.broadcast(actor_spawn_message(&self.participants[0].actor));
        if modified_route {
            self.challenge.as_mut().unwrap().route = Some(super::route::RouteRuntime::new());
        }
        if contract == ContractId::RelayGauntlet {
            return self.start_gauntlet();
        }
        if contract == ContractId::CloseQuarters {
            self.spawn_support_encounter(self.participants[0].actor.id)?;
        }
        if contract == ContractId::BastionLink {
            self.spawn_elite_encounter(
                self.participants[0].actor.id,
                revenant_ai::elite::EliteComposition::BastionLink,
            )?;
        }
        if contract == ContractId::PrismDiscipline {
            self.spawn_prism_encounter(self.participants[0].actor.id)?;
        }
        if contract == ContractId::DistantSignal {
            self.spawn_challenge_signal()?;
        }
        if contract == ContractId::LastReserve {
            self.spawn_challenge_survival()?;
            return Ok(());
        }
        self.project_challenge();
        Ok(())
    }

    pub(crate) fn challenge_activity_id(&self) -> Option<&'static str> {
        self.challenge.as_ref().map(|r| r.contract.activity_id())
    }

    pub(crate) fn save_challenge_command(
        &mut self,
        operation: &str,
        command: Command,
        position: Option<[i32; 3]>,
        combat: Option<&SupportEvidence>,
    ) -> Result<()> {
        let runtime = self
            .challenge
            .as_ref()
            .ok_or_else(|| io::Error::other("challenge runtime missing"))?;
        let player = self
            .participants
            .first()
            .ok_or_else(|| io::Error::other("challenge player missing"))?;
        let request = ChallengeRequest {
            operation_id: operation,
            expected_revision: runtime.state.revision,
            command,
            proof_event_id: None,
            position,
        };
        let context = ModuleMutationReplayContext {
            catalog_revision: player.arsenal_catalog(),
            session_id: &self.session_id,
            account_id: &player.account_id,
            activity_id: runtime.contract.activity_id(),
            actor_id: i64::try_from(player.actor.id)?,
        };
        let receipt = if let Some(evidence) = combat {
            self.persistence.apply_challenge_combat_command(
                &player.character_id,
                &request,
                context,
                evidence,
            )?
        } else {
            self.persistence
                .apply_challenge_command(&player.character_id, &request, context)?
        };
        self.challenge.as_mut().unwrap().state = receipt.state;
        Ok(())
    }

    pub(crate) fn record_challenge_fatal(&mut self, evidence: &SupportEvidence) -> Result<bool> {
        let Some(runtime) = self
            .challenge
            .as_ref()
            .filter(|r| r.contract == ContractId::CloseQuarters)
        else {
            return Ok(false);
        };
        let run = runtime
            .state
            .active
            .as_ref()
            .ok_or_else(|| io::Error::other("challenge attempt already ended"))?;
        let command = match evidence.transition {
            SupportTransition::Attack {
                target_id,
                health_after: 0,
                ..
            } => {
                let pair = self
                    .support
                    .as_ref()
                    .ok_or_else(|| io::Error::other("challenge combat missing"))?;
                let objective = if target_id == pair.lancer_id {
                    Objective::LancerCleared
                } else if target_id == pair.mender_id {
                    Objective::MenderCleared
                } else {
                    return Err(io::Error::other("challenge target is foreign").into());
                };
                Command::Confirm {
                    run_id: run.run_id,
                    objective,
                }
            }
            SupportTransition::Charged {
                health_after: 0, ..
            } => Command::End {
                run_id: run.run_id,
                reason: EndReason::Defeated,
            },
            _ => return Ok(false),
        };
        let operation = format!("world-{}-{}", self.session_id, runtime.state.revision);
        self.save_challenge_command(&operation, command, None, Some(evidence))?;
        Ok(true)
    }

    pub(crate) fn move_challenge(&mut self, player_id: u64, position: [i32; 3]) -> Result<()> {
        if self.challenge.as_ref().is_some_and(|r| r.route.is_some()) {
            return self.move_challenge_route(player_id, position);
        }
        match self.challenge.as_ref().map(|runtime| runtime.contract) {
            Some(ContractId::RelayGauntlet) => return self.move_gauntlet(player_id, position),
            Some(ContractId::LastReserve) => {
                return self.move_challenge_survival(player_id, position);
            }
            Some(ContractId::DistantSignal) => {
                return self.move_challenge_signal(player_id, position);
            }
            Some(ContractId::CoolantRecovery) => {
                return self.move_challenge_recovery(player_id, position);
            }
            _ => {}
        }
        let runtime = self
            .challenge
            .as_ref()
            .ok_or_else(|| io::Error::other("challenge runtime missing"))?;
        let Some(run) = &runtime.state.active else {
            return Ok(());
        };
        let player = self
            .actors
            .get(player_id)
            .ok_or_else(|| io::Error::other("challenge actor missing"))?;
        if self.participants[0].actor.id != player_id
            || player.health == 0
            || !revenant_activities::meridian::valid_step(player.position, position)
        {
            return Ok(());
        }
        let walkable = match runtime.contract {
            ContractId::CloseQuarters => revenant_ai::lancer::arena_contains(position),
            ContractId::MeridianCircuit => revenant_activities::meridian::walkable(position),
            ContractId::CoolantRecovery => revenant_challenges::recovery::walkable(position),
            ContractId::DistantSignal => revenant_challenges::signal::walkable(position),
            ContractId::LastReserve | ContractId::RelayGauntlet => false,
            ContractId::PrismDiscipline => revenant_ai::prism::arena_contains(position),
            ContractId::BastionLink => revenant_ai::bulwark::arena_contains(position),
        };
        if !walkable {
            return Ok(());
        }
        let mut proposed = self.actors.clone();
        proposed.update_position(player_id, position);
        let accepted_position = proposed.get(player_id).unwrap().position;
        if runtime.contract == ContractId::MeridianCircuit {
            let objective = [
                Objective::LensRead,
                Objective::GalleryRead,
                Objective::LogRead,
                Objective::Returned,
            ]
            .into_iter()
            .find(|o| o.position() == Some(accepted_position));
            if let Some(objective) = objective {
                let command = Command::Confirm {
                    run_id: run.run_id,
                    objective,
                };
                if revenant_challenges::apply(
                    runtime.policy,
                    &runtime.state,
                    runtime.state.revision,
                    &command,
                )
                .is_ok()
                {
                    let operation = format!("visit-{}-{}", self.session_id, runtime.state.revision);
                    self.save_challenge_command(
                        &operation,
                        command,
                        Some(accepted_position),
                        None,
                    )?;
                }
            }
        }
        self.actors = proposed;
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: player_id,
            position: accepted_position,
            charge: None,
            defense: None,
        }));
        self.project_challenge();
        Ok(())
    }

    pub(crate) fn interrupt_challenge(&mut self, player_id: u64) -> Result<()> {
        if self
            .participants
            .first()
            .is_none_or(|p| p.actor.id != player_id)
        {
            return Ok(());
        }
        let Some(runtime) = &self.challenge else {
            return Ok(());
        };
        let Some(run) = &runtime.state.active else {
            return Ok(());
        };
        let command = Command::End {
            run_id: run.run_id,
            reason: EndReason::Interrupted,
        };
        let operation = format!("disconnect-{}-{}", self.session_id, runtime.state.revision);
        self.save_challenge_command(&operation, command, None, None)
    }

    pub(crate) fn project_challenge(&mut self) {
        let Some(runtime) = &self.challenge else {
            return;
        };
        if runtime.state.active.is_none() {
            self.stage = if runtime
                .state
                .last_result
                .as_ref()
                .is_some_and(|r| r.outcome == Outcome::Completed)
            {
                SessionStage::Complete
            } else {
                SessionStage::Failed
            };
        }
        let mut snapshot = super::snapshot(
            &self.participants[0].character_id,
            &runtime.state,
            runtime.policy,
        );
        snapshot.elapsed_ms = runtime.route.as_ref().map(|r| r.elapsed_ms);
        self.broadcast(ServerMessage::ChallengeSnapshot(snapshot));
        self.project_gauntlet();
    }
}
