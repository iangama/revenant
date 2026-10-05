use crate::SharedSession;
use revenant_challenges::{
    gauntlet::{GauntletRun, Phase, Stage},
    ContractId,
};
use revenant_persistence::ModuleMutationReplayContext;
use revenant_protocol::{ActorUpdate, ChallengeGauntletState, ServerMessage};
use revenant_replay::{
    EliteStep, EliteTransition, GauntletAction, GauntletEffect, GauntletEvidence, PrismStep,
    PrismTransition, ReplayEventKind, SupportTransition,
};
use std::{error::Error, io, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone)]
pub(super) struct GauntletRuntime {
    pub(super) world: GauntletRun,
    sequence: u64,
    started_at: Instant,
    pub(super) last_elapsed_ms: u64,
    encounter_started_ms: u64,
    pending_spawn: bool,
    last_stage: Stage,
    recovered_health: u32,
}

impl GauntletRuntime {
    fn elapsed(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(self.last_elapsed_ms)
    }
}

impl SharedSession {
    pub(crate) fn start_gauntlet(&mut self) -> Result<()> {
        let run = self
            .challenge
            .as_ref()
            .unwrap()
            .state
            .active
            .as_ref()
            .unwrap();
        let world = GauntletRun::with_preset(&run.equipment, run.rules.preset)?;
        let first = run.rules.preset.gauntlet_stages()[0];
        self.challenge.as_mut().unwrap().gauntlet = Some(GauntletRuntime {
            world,
            sequence: 0,
            started_at: Instant::now(),
            last_elapsed_ms: 0,
            encounter_started_ms: 0,
            pending_spawn: true,
            recovered_health: 0,
            last_stage: first,
        });
        self.spawn_gauntlet_stage()?;
        self.project_challenge();
        Ok(())
    }

    fn spawn_gauntlet_stage(&mut self) -> Result<()> {
        let runtime = self.challenge.as_ref().unwrap().gauntlet.as_ref().unwrap();
        if !runtime.pending_spawn {
            return Ok(());
        }
        let Phase::Combat(stage) = runtime.world.phase() else {
            return Ok(());
        };
        let player = self.participants[0].actor.id;
        match stage {
            Stage::Support => self.spawn_support_encounter(player)?,
            Stage::Bastion => self
                .spawn_elite_encounter(player, revenant_ai::elite::EliteComposition::BastionLink)?,
            Stage::Prism => self.spawn_prism_encounter(player)?,
        }
        self.challenge
            .as_mut()
            .unwrap()
            .gauntlet
            .as_mut()
            .unwrap()
            .pending_spawn = false;
        Ok(())
    }

    pub(crate) fn record_gauntlet_combat(
        &mut self,
        kind: ReplayEventKind,
        owner: &str,
        actor_id: Option<u64>,
        payload: &str,
    ) -> Result<()> {
        if self
            .participants
            .first()
            .is_none_or(|p| p.account_id != owner)
        {
            return Err(io::Error::other("gauntlet evidence owner differs").into());
        }
        let action = GauntletAction::from_combat_payload(payload)?;
        let mut proposed = self
            .challenge
            .as_ref()
            .unwrap()
            .gauntlet
            .as_ref()
            .ok_or_else(|| io::Error::other("gauntlet runtime absent"))?
            .clone();
        let Phase::Combat(stage) = proposed.world.phase() else {
            return Err(io::Error::other("gauntlet combat is inactive").into());
        };
        let nested_time = match &action {
            GauntletAction::Support(e) => e.elapsed_ms,
            GauntletAction::Elite(e) => e.elapsed_ms,
            GauntletAction::Prism(e) => e.elapsed_ms,
            _ => unreachable!(),
        };
        let started = matches!(
            kind,
            ReplayEventKind::EnemySpawned | ReplayEventKind::BossSpawned
        );
        let elapsed = proposed
            .elapsed()
            .max(proposed.encounter_started_ms.saturating_add(nested_time));
        if started {
            proposed.encounter_started_ms = elapsed;
        }
        let player = self.actors.get(self.participants[0].actor.id).unwrap();
        let health = match &action {
            GauntletAction::Support(e) => match e.transition {
                SupportTransition::Charged { health_after, .. } => health_after,
                _ => player.health,
            },
            GauntletAction::Elite(e) => match e.transition {
                EliteTransition::Step {
                    step: EliteStep::Slam { health_after, .. },
                    ..
                } => health_after,
                _ => player.health,
            },
            GauntletAction::Prism(e) => match e.transition {
                PrismTransition::Step {
                    step: PrismStep::Resolved { health_after, .. },
                    ..
                } => health_after,
                _ => player.health,
            },
            _ => unreachable!(),
        };
        let cleared =
            kind == ReplayEventKind::EnemyDied && self.gauntlet_last_enemy(stage, actor_id);
        let effect = GauntletEffect {
            cleared: cleared.then_some(stage),
            defeated: health == 0,
            ..GauntletEffect::default()
        };
        if cleared || health == 0 {
            proposed.world = proposed
                .world
                .finish_encounter(elapsed, stage, health)
                .map_err(|_| io::Error::other("invalid gauntlet terminal"))?;
        }
        let proof = self.gauntlet_proof(
            &proposed,
            elapsed,
            actor_id.ok_or_else(|| io::Error::other("gauntlet actor absent"))?,
            action,
            effect,
        )?;
        if kind != proof.kind() {
            return Err(io::Error::other("gauntlet event kind differs").into());
        }
        self.persist_gauntlet(&proof, proposed)
    }

    fn gauntlet_last_enemy(&self, stage: Stage, target: Option<u64>) -> bool {
        let ids = match stage {
            Stage::Support => {
                let e = self.support.as_ref().unwrap();
                vec![e.lancer_id, e.mender_id]
            }
            Stage::Bastion => {
                let e = self.elite.as_ref().unwrap();
                vec![e.bulwark_id, e.partner_id]
            }
            Stage::Prism => return true,
        };
        ids.into_iter()
            .filter(|id| Some(*id) != target)
            .all(|id| self.actors.get(id).is_none())
    }

    fn gauntlet_proof(
        &self,
        proposed: &GauntletRuntime,
        elapsed_ms: u64,
        actor_id: u64,
        action: GauntletAction,
        effect: GauntletEffect,
    ) -> Result<GauntletEvidence> {
        let run = self
            .challenge
            .as_ref()
            .unwrap()
            .state
            .active
            .as_ref()
            .ok_or_else(|| io::Error::other("gauntlet attempt ended"))?;
        Ok(GauntletEvidence {
            run_id: run.run_id,
            sequence: proposed
                .sequence
                .checked_add(1)
                .ok_or_else(|| io::Error::other("gauntlet sequence exhausted"))?,
            elapsed_ms,
            actor_id,
            action,
            effect,
        })
    }

    fn persist_gauntlet(
        &mut self,
        proof: &GauntletEvidence,
        mut proposed: GauntletRuntime,
    ) -> Result<()> {
        let player = &self.participants[0];
        let receipt = self.persistence.apply_challenge_gauntlet_step(
            &player.character_id,
            proof,
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: ContractId::RelayGauntlet.activity_id(),
                actor_id: i64::try_from(player.actor.id)?,
            },
        )?;
        proposed.sequence = proof.sequence;
        proposed.last_elapsed_ms = proof.elapsed_ms;
        proposed.recovered_health = proof.effect.recovered_health;
        let runtime = self.challenge.as_mut().unwrap();
        runtime.state = receipt.state;
        runtime.gauntlet = Some(proposed);
        Ok(())
    }

    pub(crate) fn gauntlet_transferring(&self) -> bool {
        self.challenge
            .as_ref()
            .and_then(|r| r.gauntlet.as_ref())
            .is_some_and(|g| !matches!(g.world.phase(), Phase::Combat(_)))
    }

    pub(crate) fn move_gauntlet(&mut self, player: u64, position: [i32; 3]) -> Result<()> {
        let time = self
            .challenge
            .as_ref()
            .unwrap()
            .gauntlet
            .as_ref()
            .unwrap()
            .elapsed();
        self.gauntlet_step_at(player, time, GauntletAction::Move { position })
    }

    pub(crate) fn tick_gauntlet(&mut self) -> Result<()> {
        let Some(runtime) = self
            .challenge
            .as_ref()
            .filter(|r| r.state.active.is_some())
            .and_then(|r| r.gauntlet.as_ref())
        else {
            return Ok(());
        };
        if runtime.pending_spawn {
            return self.spawn_gauntlet_stage();
        }
        let elapsed = runtime.elapsed();
        if runtime
            .world
            .observation_deadline_ms()
            .is_some_and(|t| elapsed >= t)
        {
            self.gauntlet_step_at(
                self.participants[0].actor.id,
                elapsed,
                GauntletAction::Observe,
            )?;
        }
        Ok(())
    }

    pub(crate) fn gauntlet_step_at(
        &mut self,
        player_id: u64,
        elapsed: u64,
        action: GauntletAction,
    ) -> Result<()> {
        let runtime = self.challenge.as_ref().unwrap();
        if runtime.state.active.is_none() {
            return Ok(());
        }
        let mut proposed = runtime.gauntlet.as_ref().unwrap().clone();
        let player = self
            .actors
            .get(player_id)
            .ok_or_else(|| io::Error::other("gauntlet player absent"))?;
        let (Phase::Combat(stage) | Phase::Transfer(stage)) = proposed.world.phase() else {
            return Ok(());
        };
        if self.participants[0].actor.id != player_id
            || player.health == 0
            || proposed.pending_spawn
        {
            return Ok(());
        }
        if player.max_health != proposed.world.max_health()
            || (matches!(proposed.world.phase(), Phase::Transfer(_))
                && player.health != proposed.world.health())
        {
            return Err(
                io::Error::other("gauntlet actor health differs from accepted stage").into(),
            );
        }
        let mut position = player.position;
        match action {
            GauntletAction::Move { position: next } => {
                let distance: i64 = position
                    .iter()
                    .zip(next)
                    .map(|(a, b)| (i64::from(*a) - i64::from(b)).abs())
                    .sum();
                if !stage.walkable(next) || distance != 1 {
                    return Ok(());
                }
                position = next;
            }
            GauntletAction::Observe if matches!(proposed.world.phase(), Phase::Transfer(_)) => {}
            _ => return Err(io::Error::other("invalid gauntlet spatial action").into()),
        }
        let mut effect = GauntletEffect::default();
        if matches!(proposed.world.phase(), Phase::Transfer(_)) {
            let update = proposed
                .world
                .observe_at(elapsed, position)
                .map_err(|_| io::Error::other("invalid gauntlet transfer clock"))?;
            effect.recovered_health = update.recovered_health;
            effect.next_stage = update.next_stage;
            proposed.world = update.run;
            if let Some(next) = update.next_stage {
                position = next.entrance();
                proposed.pending_spawn = true;
                proposed.last_stage = next;
            }
        }
        let proof = self.gauntlet_proof(&proposed, elapsed, player_id, action, effect)?;
        let mut actor = player.clone();
        actor.position = position;
        actor.health += effect.recovered_health;
        self.persist_gauntlet(&proof, proposed)?;
        self.actors.insert(actor);
        if matches!(proof.action, GauntletAction::Move { .. }) || effect.next_stage.is_some() {
            self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
                actor_id: player_id,
                position,
                charge: None,
                defense: None,
            }));
        }
        self.project_gauntlet();
        if effect.next_stage.is_some() {
            self.spawn_gauntlet_stage()?;
            self.project_challenge();
        }
        Ok(())
    }

    pub(crate) fn project_gauntlet(&mut self) {
        let Some(runtime) = self.challenge.as_ref() else {
            return;
        };
        let Some(g) = runtime.gauntlet.as_ref() else {
            return;
        };
        let Some(run) = runtime
            .state
            .active
            .as_ref()
            .or_else(|| runtime.state.last_result.as_ref().map(|r| &r.run))
        else {
            return;
        };
        let player_id = self.participants[0].actor.id;
        let Some(player) = self.actors.get(player_id) else {
            return;
        };
        let (stage, mut phase) = match g.world.phase() {
            Phase::Combat(s) => (s, "combat"),
            Phase::Transfer(s) => (s, "transfer"),
            Phase::Completed => (Stage::Prism, "completed"),
            Phase::Defeated => (g.last_stage, "defeated"),
        };
        if runtime.state.active.is_none() {
            phase = match runtime.state.last_result.as_ref().unwrap().outcome {
                revenant_challenges::Outcome::Abandoned => "abandoned",
                revenant_challenges::Outcome::Interrupted => "interrupted",
                _ => phase,
            };
        }
        self.broadcast(ServerMessage::ChallengeGauntletState(
            ChallengeGauntletState {
                run_id: run.run_id,
                sequence: g.sequence,
                player_actor_id: player_id,
                modifier: (!run.rules.preset.is_baseline()).then(|| {
                    revenant_protocol::ChallengeGauntletModifier {
                        preset_id: run.rules.preset.as_str().into(),
                        stage_index: u8::try_from(
                            run.rules
                                .preset
                                .gauntlet_stages()
                                .iter()
                                .position(|s| *s == stage)
                                .unwrap()
                                + 1,
                        )
                        .unwrap(),
                        reserve_used: g.world.reserve_used(),
                        reserve_remaining_ms: if runtime.state.active.is_some() {
                            g.world.reserve_remaining_ms()
                        } else {
                            None
                        },
                        elapsed_ms: g.last_elapsed_ms,
                    }
                }),
                stage: match stage {
                    Stage::Support => 1,
                    Stage::Bastion => 2,
                    Stage::Prism => 3,
                },
                phase: phase.into(),
                health: player.health,
                max_health: player.max_health,
                recovered_health: g.recovered_health,
                transfer_remaining_ms: if runtime.state.active.is_some() {
                    g.world.transfer_remaining_ms()
                } else {
                    None
                },
            },
        ));
    }
}
