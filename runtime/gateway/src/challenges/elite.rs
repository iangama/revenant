use std::{error::Error, io};

use revenant_challenges::{Command, ContractId, EndReason, Objective};
use revenant_persistence::{ChallengeRequest, ModuleMutationReplayContext};
use revenant_replay::{EliteEvidence, EliteStep, EliteTransition};

use crate::SharedSession;

impl SharedSession {
    /// Records the fatal elite event and checkpoint before actor/brain mutation.
    pub(crate) fn record_challenge_elite_fatal(
        &mut self,
        evidence: &EliteEvidence,
    ) -> Result<bool, Box<dyn Error>> {
        let Some(runtime) = self
            .challenge
            .as_ref()
            .filter(|r| r.contract == ContractId::BastionLink)
        else {
            return Ok(false);
        };
        let run = runtime
            .state
            .active
            .as_ref()
            .ok_or_else(|| io::Error::other("elite challenge already ended"))?;
        let command = match evidence.transition {
            EliteTransition::Attack {
                target_id,
                health_after: 0,
                ..
            } => {
                let pair = self
                    .elite
                    .as_ref()
                    .ok_or_else(|| io::Error::other("elite encounter missing"))?;
                let objective = if target_id == pair.bulwark_id {
                    Objective::BulwarkCleared
                } else if target_id == pair.partner_id {
                    Objective::MenderCleared
                } else {
                    return Err(io::Error::other("elite challenge target is foreign").into());
                };
                Command::Confirm {
                    run_id: run.run_id,
                    objective,
                }
            }
            EliteTransition::Step {
                step: EliteStep::Slam {
                    health_after: 0, ..
                },
                ..
            } => Command::End {
                run_id: run.run_id,
                reason: EndReason::Defeated,
            },
            _ => return Ok(false),
        };
        let player = self
            .participants
            .first()
            .ok_or_else(|| io::Error::other("elite challenge player missing"))?;
        let operation = format!("world-{}-{}", self.session_id, runtime.state.revision);
        let receipt = self.persistence.apply_challenge_elite_command(
            &player.character_id,
            &ChallengeRequest {
                operation_id: &operation,
                expected_revision: runtime.state.revision,
                command,
                proof_event_id: None,
                position: None,
            },
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: runtime.contract.activity_id(),
                actor_id: i64::try_from(player.actor.id)?,
            },
            evidence,
        )?;
        self.challenge.as_mut().unwrap().state = receipt.state;
        Ok(true)
    }
}
