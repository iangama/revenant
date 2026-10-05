//! Plan, persist, then publish each survival action.
use crate::{actor_spawn_message, ActorKind, SharedSession};
use revenant_ai::sentinel::{SENTINEL_HEALTH, SENTINEL_POSITION};
use revenant_challenges::{
    survival::{Action, Effect, Error as SurvivalError, Phase, SurvivalRun},
    ContractId,
};
use revenant_persistence::ModuleMutationReplayContext;
use revenant_protocol::{ActorUpdate, ChallengeSurvivalState, DamageApplied, ServerMessage};
use revenant_replay::SurvivalEvidence;
use std::{error::Error, io, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) struct SurvivalRuntime {
    world: SurvivalRun,
    sequence: u64,
    enemy_id: u64,
    started_at: Instant,
}

impl SharedSession {
    pub(crate) fn spawn_challenge_survival(&mut self) -> Result<()> {
        let run = self
            .challenge
            .as_ref()
            .and_then(|r| r.state.active.as_ref())
            .ok_or_else(|| io::Error::other("survival admission missing"))?;
        let world = SurvivalRun::new(&run.equipment)?;
        let mut actors = self.actors.clone();
        let enemy = actors.spawn(
            ActorKind::Enemy,
            "signal-sentinel",
            SENTINEL_POSITION,
            SENTINEL_HEALTH,
        );
        let proof = SurvivalEvidence {
            run_id: run.run_id,
            sequence: 1,
            elapsed_ms: 0,
            enemy_id: enemy.id,
            action: None,
            effect: Effect::default(),
        };
        self.persist_survival_proof(&proof)?;
        let started_at = Instant::now();
        self.challenge.as_mut().unwrap().survival = Some(SurvivalRuntime {
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
        self.project_challenge();
        let world = self
            .challenge
            .as_ref()
            .unwrap()
            .survival
            .as_ref()
            .unwrap()
            .world
            .clone();
        self.project_survival_state(self.participants[0].actor.id, &proof, &world);
        Ok(())
    }

    pub(crate) fn tick_challenge_survival(&mut self) -> Result<()> {
        let Some(runtime) = self.challenge.as_ref().filter(|r| r.state.active.is_some()) else {
            return Ok(());
        };
        let Some(survival) = &runtime.survival else {
            return Ok(());
        };
        let elapsed = u64::try_from(survival.started_at.elapsed().as_millis())?;
        if elapsed < survival.world.next_observation_ms() {
            return Ok(());
        }
        self.apply_survival_action_at(self.participants[0].actor.id, elapsed, Action::Observe)
    }

    pub(crate) fn move_challenge_survival(
        &mut self,
        player: u64,
        position: [i32; 3],
    ) -> Result<()> {
        let survival = self
            .challenge
            .as_ref()
            .and_then(|r| r.survival.as_ref())
            .ok_or_else(|| io::Error::other("survival world missing"))?;
        self.apply_survival_action_at(
            player,
            u64::try_from(survival.started_at.elapsed().as_millis())?,
            Action::Move { position },
        )
    }

    pub(crate) fn apply_survival_action_at(
        &mut self,
        player: u64,
        elapsed: u64,
        action: Action,
    ) -> Result<()> {
        let runtime = self
            .challenge
            .as_ref()
            .filter(|r| r.contract == ContractId::LastReserve)
            .ok_or_else(|| io::Error::other("survival challenge missing"))?;
        let Some(run) = &runtime.state.active else {
            return Ok(());
        };
        let survival = runtime
            .survival
            .as_ref()
            .ok_or_else(|| io::Error::other("survival world missing"))?;
        let actor = self
            .actors
            .get(player)
            .ok_or_else(|| io::Error::other("survival player missing"))?;
        if self
            .participants
            .first()
            .is_none_or(|p| p.actor.id != player)
            || actor.position != survival.world.position()
            || actor.health != survival.world.health()
            || actor.max_health != survival.world.max_health()
            || self
                .actors
                .get(survival.enemy_id)
                .is_none_or(|a| a.health != SENTINEL_HEALTH)
        {
            return Err(io::Error::other("survival actors differ from accepted world").into());
        }
        let proposed = match survival.world.apply(elapsed, action) {
            Ok(next) => next,
            Err(SurvivalError::InvalidMovement) => return Ok(()),
            Err(_) => return Err(io::Error::other("survival clock or terminal changed").into()),
        };
        let proof = SurvivalEvidence {
            run_id: run.run_id,
            sequence: survival
                .sequence
                .checked_add(1)
                .ok_or_else(|| io::Error::other("survival sequence exhausted"))?,
            elapsed_ms: elapsed,
            enemy_id: survival.enemy_id,
            action: Some(action),
            effect: proposed.effect,
        };
        self.persist_survival_proof(&proof)?;
        self.project_survival_action(player, &proof, &proposed.run);
        let survival = self.challenge.as_mut().unwrap().survival.as_mut().unwrap();
        survival.world = proposed.run;
        survival.sequence = proof.sequence;
        if proof.command().is_some() {
            self.project_challenge();
        }
        Ok(())
    }

    fn persist_survival_proof(&mut self, proof: &SurvivalEvidence) -> Result<()> {
        let player = self
            .participants
            .first()
            .ok_or_else(|| io::Error::other("survival participant missing"))?;
        let receipt = self.persistence.apply_challenge_survival_step(
            &player.character_id,
            proof,
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: ContractId::LastReserve.activity_id(),
                actor_id: i64::try_from(player.actor.id)?,
            },
        )?;
        self.challenge.as_mut().unwrap().state = receipt.state;
        Ok(())
    }

    fn project_survival_action(
        &mut self,
        player: u64,
        proof: &SurvivalEvidence,
        next: &SurvivalRun,
    ) {
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
        if proof.effect.recovered_health > 0 {
            let mut healed = self.actors.get(player).unwrap().clone();
            healed.health = next.health();
            self.actors.insert(healed);
        }
        self.project_survival_state(player, proof, next);
    }

    fn project_survival_state(
        &mut self,
        player: u64,
        proof: &SurvivalEvidence,
        next: &SurvivalRun,
    ) {
        let phase = if next.health() == 0 {
            "defeated"
        } else {
            match next.phase() {
                Phase::West => "west",
                Phase::East => "east",
                Phase::Return => "return",
                Phase::Completed => "completed",
            }
        };
        self.broadcast(ServerMessage::ChallengeSurvivalState(
            ChallengeSurvivalState {
                run_id: proof.run_id,
                sequence: proof.sequence,
                player_actor_id: player,
                phase: phase.into(),
                health: next.health(),
                max_health: next.max_health(),
                reserve_used: next.reserve_used(),
                recovered_health: proof.effect.recovered_health,
                charge_remaining_ms: next.charge_remaining_ms(),
                recovery_remaining_ms: next.recovery_remaining_ms(),
            },
        ));
    }
}
