//! Plan, persist, then publish each distant firing-lane action.
use crate::{actor_spawn_message, ActorKind, SharedSession};
use revenant_ai::sentinel::{SENTINEL_HEALTH, SENTINEL_POSITION};
use revenant_challenges::{
    signal::{Action, Effect, Error as SignalError, SignalRun, Terminal},
    ContractId,
};
use revenant_persistence::ModuleMutationReplayContext;
use revenant_protocol::{ActorDestroy, ActorUpdate, DamageApplied, ServerMessage};
use revenant_replay::SignalEvidence;
use std::{error::Error, io, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) struct SignalRuntime {
    world: SignalRun,
    sequence: u64,
    enemy_id: u64,
    started_at: Instant,
}

impl SharedSession {
    pub(crate) fn spawn_challenge_signal(&mut self) -> Result<()> {
        let run = self
            .challenge
            .as_ref()
            .and_then(|r| r.state.active.as_ref())
            .ok_or_else(|| io::Error::other("signal admission missing"))?;
        let world = SignalRun::new(&run.equipment)?;
        let mut actors = self.actors.clone();
        let enemy = actors.spawn(
            ActorKind::Enemy,
            "signal-sentinel",
            SENTINEL_POSITION,
            SENTINEL_HEALTH,
        );
        let proof = SignalEvidence {
            run_id: run.run_id,
            sequence: 1,
            elapsed_ms: 0,
            enemy_id: enemy.id,
            action: None,
            effect: Effect::default(),
        };
        self.persist_signal_proof(&proof)?;
        let started_at = Instant::now();
        self.challenge.as_mut().unwrap().signal = Some(SignalRuntime {
            world,
            sequence: 1,
            enemy_id: enemy.id,
            started_at,
        });
        self.combat_started = started_at;
        self.actors = actors;
        self.enemy_id = Some(enemy.id);
        self.enemy_ai = None;
        self.broadcast(actor_spawn_message(&enemy));
        Ok(())
    }

    pub(crate) fn tick_challenge_signal(&mut self) -> Result<()> {
        let Some(runtime) = self.challenge.as_ref().filter(|r| r.state.active.is_some()) else {
            return Ok(());
        };
        let Some(signal) = &runtime.signal else {
            return Ok(());
        };
        let elapsed = u64::try_from(signal.started_at.elapsed().as_millis())?;
        if elapsed < signal.world.next_shot_ms() {
            return Ok(());
        }
        self.apply_signal_action_at(self.participants[0].actor.id, elapsed, Action::Observe)
    }

    pub(crate) fn move_challenge_signal(&mut self, player: u64, position: [i32; 3]) -> Result<()> {
        let signal = self
            .challenge
            .as_ref()
            .and_then(|r| r.signal.as_ref())
            .ok_or_else(|| io::Error::other("signal world missing"))?;
        self.apply_signal_action_at(
            player,
            u64::try_from(signal.started_at.elapsed().as_millis())?,
            Action::Move { position },
        )
    }

    pub(crate) fn attack_challenge_signal(
        &mut self,
        player: u64,
        target: u64,
        elapsed: u64,
    ) -> Result<()> {
        if self.enemy_id != Some(target) {
            return Err(io::Error::other("foreign signal target").into());
        }
        self.apply_signal_action_at(player, elapsed, Action::Attack)
    }

    pub(crate) fn apply_signal_action_at(
        &mut self,
        player: u64,
        elapsed: u64,
        action: Action,
    ) -> Result<()> {
        let runtime = self
            .challenge
            .as_ref()
            .filter(|r| r.contract == ContractId::DistantSignal)
            .ok_or_else(|| io::Error::other("signal challenge missing"))?;
        let Some(run) = &runtime.state.active else {
            return Ok(());
        };
        let signal = runtime
            .signal
            .as_ref()
            .ok_or_else(|| io::Error::other("signal world missing"))?;
        let actor = self
            .actors
            .get(player)
            .ok_or_else(|| io::Error::other("signal player missing"))?;
        if self
            .participants
            .first()
            .is_none_or(|p| p.actor.id != player)
            || actor.position != signal.world.position()
            || actor.health != signal.world.health()
            || self
                .actors
                .get(signal.enemy_id)
                .is_none_or(|a| a.health != signal.world.enemy_health())
        {
            return Err(io::Error::other("signal actors differ from accepted world").into());
        }
        let proposed = match signal.world.apply(elapsed, action) {
            Ok(next) => next,
            Err(
                SignalError::InvalidMovement
                | SignalError::BlockedShot
                | SignalError::Cooldown
                | SignalError::OutOfRange,
            ) => return Ok(()),
            Err(_) => return Err(io::Error::other("signal clock or terminal changed").into()),
        };
        let proof = SignalEvidence {
            run_id: run.run_id,
            sequence: signal
                .sequence
                .checked_add(1)
                .ok_or_else(|| io::Error::other("signal sequence exhausted"))?,
            elapsed_ms: elapsed,
            enemy_id: signal.enemy_id,
            action: Some(action),
            effect: proposed.effect,
        };
        self.persist_signal_proof(&proof)?;
        self.project_signal_action(player, &proof, &proposed.run);
        let signal = self.challenge.as_mut().unwrap().signal.as_mut().unwrap();
        signal.world = proposed.run;
        signal.sequence = proof.sequence;
        if proof.command().is_some() {
            self.project_challenge();
        }
        Ok(())
    }

    fn persist_signal_proof(&mut self, proof: &SignalEvidence) -> Result<()> {
        let player = self
            .participants
            .first()
            .ok_or_else(|| io::Error::other("signal participant missing"))?;
        let receipt = self.persistence.apply_challenge_signal_step(
            &player.character_id,
            proof,
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: ContractId::DistantSignal.activity_id(),
                actor_id: i64::try_from(player.actor.id)?,
            },
        )?;
        self.challenge.as_mut().unwrap().state = receipt.state;
        Ok(())
    }

    fn project_signal_action(&mut self, player: u64, proof: &SignalEvidence, next: &SignalRun) {
        if proof.effect.incoming_damage > 0 {
            self.actors
                .apply_damage(player, proof.effect.incoming_damage);
            self.broadcast(ServerMessage::DamageApplied(DamageApplied {
                source_actor_id: proof.enemy_id,
                target_actor_id: player,
                damage: proof.effect.incoming_damage,
                remaining_health: next.health(),
                killed: next.health() == 0,
            }));
        }
        if matches!(proof.action, Some(Action::Move { .. })) && next.health() > 0 {
            self.actors.update_position(player, next.position());
            self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
                actor_id: player,
                position: next.position(),
                charge: None,
                defense: None,
            }));
        }
        if proof.effect.outgoing_damage > 0 {
            self.actors
                .apply_damage(proof.enemy_id, proof.effect.outgoing_damage);
            self.broadcast(ServerMessage::DamageApplied(DamageApplied {
                source_actor_id: player,
                target_actor_id: proof.enemy_id,
                damage: proof.effect.outgoing_damage,
                remaining_health: next.enemy_health(),
                killed: next.enemy_health() == 0,
            }));
        }
        if proof.effect.terminal == Some(Terminal::Completed) {
            self.actors.destroy(proof.enemy_id);
            self.enemy_id = None;
            self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
                actor_id: proof.enemy_id,
            }));
        }
    }
}
