use crate::SharedSession;
use revenant_challenges::{Command, ContractId, EndReason, Objective};
use revenant_persistence::{ChallengeRequest, ModuleMutationReplayContext};
use revenant_replay::{PrismEvidence, PrismStep, PrismTransition};
use std::{error::Error, io};

impl SharedSession {
    pub(crate) fn record_challenge_prism_fatal(
        &mut self,
        boss_id: u64,
        evidence: &PrismEvidence,
    ) -> Result<bool, Box<dyn Error>> {
        let Some(runtime) = self
            .challenge
            .as_ref()
            .filter(|r| r.contract == ContractId::PrismDiscipline)
        else {
            return Ok(false);
        };
        let run = runtime
            .state
            .active
            .as_ref()
            .ok_or_else(|| io::Error::other("Prism challenge already ended"))?;
        let command = match evidence.transition {
            PrismTransition::Attack {
                health_after: 0, ..
            } => Command::Confirm {
                run_id: run.run_id,
                objective: Objective::PrismCleared,
            },
            PrismTransition::Step {
                step:
                    PrismStep::Resolved {
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
            .ok_or_else(|| io::Error::other("Prism challenge player missing"))?;
        let operation = format!("world-{}-{}", self.session_id, runtime.state.revision);
        let receipt = self.persistence.apply_challenge_prism_command(
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
            boss_id,
            evidence,
        )?;
        self.challenge.as_mut().unwrap().state = receipt.state;
        Ok(true)
    }
}
