//! Recovery actors and continuous station holds share one persisted move stream.
use std::{error::Error, io, time::Instant};

use revenant_challenges::recovery::{Action, Error as RecoveryError, RecoveryRun};
use revenant_persistence::ModuleMutationReplayContext;
use revenant_protocol::{ActorUpdate, ServerMessage};
use revenant_replay::RecoveryEvidence;

use crate::SharedSession;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) struct RecoveryRuntime {
    run: RecoveryRun,
    sequence: u64,
    started_at: Instant,
}

impl RecoveryRuntime {
    pub(super) fn new() -> Self {
        Self {
            run: RecoveryRun::default(),
            sequence: 0,
            started_at: Instant::now(),
        }
    }
}

impl SharedSession {
    pub(crate) fn tick_challenge_recovery(&mut self) -> Result<()> {
        let Some(runtime) = self
            .challenge
            .as_ref()
            .filter(|runtime| runtime.state.active.is_some())
        else {
            return Ok(());
        };
        let Some(recovery) = &runtime.recovery else {
            return Ok(());
        };
        let Some(player) = self.participants.first() else {
            return Ok(());
        };
        self.apply_recovery_action_at(
            player.actor.id,
            u64::try_from(recovery.started_at.elapsed().as_millis())?,
            Action::Observe,
        )
    }

    pub(crate) fn move_challenge_recovery(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<()> {
        let Some(recovery) = self
            .challenge
            .as_ref()
            .and_then(|runtime| runtime.recovery.as_ref())
        else {
            return Ok(());
        };
        self.apply_recovery_action_at(
            player_id,
            u64::try_from(recovery.started_at.elapsed().as_millis())?,
            Action::Move { position },
        )
    }

    pub(crate) fn apply_recovery_action_at(
        &mut self,
        player_id: u64,
        elapsed_ms: u64,
        action: Action,
    ) -> Result<()> {
        let runtime = self
            .challenge
            .as_ref()
            .ok_or_else(|| io::Error::other("recovery challenge missing"))?;
        let Some(run) = &runtime.state.active else {
            return Ok(());
        };
        let recovery = runtime
            .recovery
            .as_ref()
            .ok_or_else(|| io::Error::other("recovery world missing"))?;
        let actor = self
            .actors
            .get(player_id)
            .ok_or_else(|| io::Error::other("recovery actor missing"))?;
        let player = self
            .participants
            .first()
            .filter(|player| player.actor.id == player_id)
            .ok_or_else(|| io::Error::other("recovery participant missing"))?;
        if actor.health == 0 || actor.position != recovery.run.position() {
            return Err(io::Error::other("recovery actor differs from accepted movement").into());
        }
        let proposed = match recovery.run.apply(elapsed_ms, action) {
            Ok(next) => next,
            Err(RecoveryError::InvalidMovement) => return Ok(()),
            Err(_) => return Err(io::Error::other("recovery clock or phase changed").into()),
        };
        if action == Action::Observe && proposed.checkpoint.is_none() {
            return Ok(());
        }
        let proof = RecoveryEvidence {
            run_id: run.run_id,
            sequence: recovery
                .sequence
                .checked_add(1)
                .ok_or_else(|| io::Error::other("recovery sequence exhausted"))?,
            elapsed_ms,
            action,
            checkpoint: proposed.checkpoint,
        };
        let receipt = self.persistence.apply_challenge_recovery_step(
            &player.character_id,
            &proof,
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: runtime.contract.activity_id(),
                actor_id: i64::try_from(player_id)?,
            },
        )?;
        let position = proposed.run.position();
        let runtime = self.challenge.as_mut().unwrap();
        runtime.state = receipt.state;
        let recovery = runtime.recovery.as_mut().unwrap();
        recovery.run = proposed.run;
        recovery.sequence = proof.sequence;
        if matches!(action, Action::Move { .. }) {
            self.actors.update_position(player_id, position);
            self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
                actor_id: player_id,
                position,
                charge: None,
                defense: None,
            }));
        }
        self.project_challenge();
        Ok(())
    }
}
