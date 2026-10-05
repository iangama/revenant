use std::{error::Error, time::Instant};

use revenant_ai::lancer::{
    arena_contains, ChargeAction, GlassLancer, ENTRANCE, HEALTH, SPAWN, WINDUP_MS,
};
use revenant_combat::AttackProfile;
use revenant_operations::RoutePhase;
use revenant_protocol::ChargeTelegraph;
use revenant_replay::{encode_lancer_evidence, LancerEvidence};

use super::{
    actor_spawn_message, ActorDestroy, ActorKind, ActorUpdate, AiEvent, CombatRuntime,
    DamageApplied, ObjectiveUpdate, ReplayEventKind, ServerMessage, SessionStage, SharedSession,
};

pub(super) struct LancerEncounter {
    enemy_id: u64,
    player_id: u64,
    active: bool,
    started_at: Instant,
    ai: GlassLancer,
}

impl SharedSession {
    pub(super) fn lancer_fighting(&self) -> bool {
        self.lancer
            .as_ref()
            .is_some_and(|encounter| encounter.active)
    }

    pub(super) fn handle_lancer_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn Error>> {
        if self.lancer_fighting() {
            if arena_contains(position) {
                return Ok(true);
            }
            let encounter = self.lancer.as_ref().unwrap();
            let enemy_id = encounter.enemy_id;
            let elapsed_ms =
                u64::try_from(encounter.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
            self.record_lancer(&LancerEvidence::Abandoned { elapsed_ms })?;
            self.lancer.as_mut().unwrap().active = false;
            self.actors.destroy(enemy_id);
            self.enemy_id = None;
            self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
                actor_id: enemy_id,
            }));
            self.broadcast(lancer_objective("Failed"));
            return Ok(false);
        }
        let available = self.lancer.is_none()
            && self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].encounter_capable
            && self
                .route
                .as_ref()
                .is_some_and(|route| route.operation.phase() == RoutePhase::ChoiceOpen);
        if !available || position != ENTRANCE {
            return Ok(false);
        }
        let mut actors = self.actors.clone();
        let enemy = actors.spawn(ActorKind::Enemy, "glass-lancer", SPAWN, HEALTH);
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::EnemySpawned,
            &owner,
            Some(enemy.id),
            "enemy spawned: glass-lancer",
        )?;
        self.route.as_mut().unwrap().operation.lock_baseline()?;
        self.actors = actors;
        self.lancer = Some(LancerEncounter {
            enemy_id: enemy.id,
            player_id,
            active: true,
            started_at: Instant::now(),
            ai: GlassLancer::default(),
        });
        self.enemy_id = Some(enemy.id);
        self.enemy_ai = None;
        self.combat = CombatRuntime::default();
        self.combat_started = Instant::now();
        self.broadcast(actor_spawn_message(&enemy));
        self.broadcast(lancer_objective("Active"));
        self.broadcast_route_state("Glass Lancer selected; standard mission rewards")?;
        Ok(true)
    }

    pub(super) fn tick_lancer(&mut self) -> Result<(), Box<dyn Error>> {
        let elapsed = self.lancer.as_ref().map_or(0, |encounter| {
            u64::try_from(encounter.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
        self.tick_lancer_at(elapsed)
    }

    pub(super) fn tick_lancer_at(&mut self, elapsed_ms: u64) -> Result<(), Box<dyn Error>> {
        if !self.lancer_fighting() || self.stage == SessionStage::Failed {
            return Ok(());
        }
        let encounter = self.lancer.as_ref().unwrap();
        let (enemy_id, player_id) = (encounter.enemy_id, encounter.player_id);
        let Some(enemy) = self.actors.get(enemy_id) else {
            return Ok(());
        };
        let Some(player) = self.actors.get(player_id) else {
            return Ok(());
        };
        if enemy.health == 0 || player.health == 0 {
            return Ok(());
        }
        let player_position = player.position;
        let health_before = player.health;
        let mut proposed = encounter.ai.clone();
        let Some(action) = proposed.tick(elapsed_ms, enemy.position, player.position) else {
            return Ok(());
        };
        let (position, target, winding_up) = match action {
            ChargeAction::Windup { origin, target } => {
                self.record_lancer(&LancerEvidence::Windup {
                    origin,
                    target,
                    elapsed_ms,
                })?;
                (origin, target, true)
            }
            ChargeAction::Resolved {
                origin,
                target,
                damage,
            } => {
                self.record_lancer(&LancerEvidence::Resolved {
                    origin,
                    target,
                    player_position,
                    elapsed_ms,
                    damage,
                    health_before,
                    health_after: health_before.saturating_sub(damage),
                })?;
                self.actors.update_position(enemy_id, target);
                if damage > 0 {
                    let victim = self.actors.apply_damage(player_id, damage).unwrap();
                    self.apply_ai_event(&AiEvent::Attacked {
                        source_actor_id: enemy_id,
                        target_actor_id: player_id,
                        damage,
                        remaining_health: victim.health,
                        killed: victim.health == 0,
                    });
                    if victim.health == 0 {
                        self.stage = SessionStage::Failed;
                        self.lancer.as_mut().unwrap().active = false;
                        self.broadcast(lancer_objective("Failed"));
                    }
                }
                (target, target, false)
            }
        };
        let encounter = self.lancer.as_mut().unwrap();
        let confirmed_ms =
            u64::try_from(encounter.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        proposed.confirm_at(confirmed_ms.max(elapsed_ms));
        encounter.ai = proposed;
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: enemy_id,
            position,
            defense: None,
            charge: Some(ChargeTelegraph {
                target,
                winding_up,
                warning_ms: u32::try_from(WINDUP_MS)?,
            }),
        }));
        Ok(())
    }

    fn record_lancer(&mut self, evidence: &LancerEvidence) -> Result<(), Box<dyn Error>> {
        let owner = self.owner_account()?.to_owned();
        let player_id = self.lancer.as_ref().unwrap().player_id;
        self.append_event(
            ReplayEventKind::FieldActivity,
            &owner,
            Some(player_id),
            &encode_lancer_evidence(evidence)?,
        )
    }

    pub(super) fn attack_lancer(
        &mut self,
        player_id: u64,
        target_id: u64,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn Error>> {
        let encounter = self.lancer.as_ref().unwrap();
        if encounter.player_id != player_id || encounter.enemy_id != target_id {
            return Err(std::io::Error::other("invalid Lancer target").into());
        }
        let profile = self.positioned_attack_profile(player_id, target_id)?;
        let mut actors = self.actors.clone();
        let mut combat = self.combat.clone();
        let result = combat.attack(
            &mut actors,
            player_id,
            target_id,
            elapsed_ms,
            AttackProfile {
                damage: profile.damage,
                range: profile.range,
                cooldown_ms: profile.cooldown_ms,
            },
        )?;
        if result.killed {
            let owner = self.owner_account()?.to_owned();
            self.append_event(
                ReplayEventKind::EnemyDied,
                &owner,
                Some(target_id),
                "enemy died: glass-lancer",
            )?;
        }
        self.actors = actors;
        self.combat = combat;
        self.broadcast(ServerMessage::DamageApplied(DamageApplied {
            source_actor_id: player_id,
            target_actor_id: target_id,
            damage: result.damage,
            remaining_health: result.remaining_health,
            killed: result.killed,
        }));
        if result.killed {
            self.lancer.as_mut().unwrap().active = false;
            self.actors.destroy(target_id);
            self.enemy_id = None;
            self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
                actor_id: target_id,
            }));
            self.broadcast(lancer_objective("Completed"));
        }
        Ok(())
    }
}

fn lancer_objective(state: &str) -> ServerMessage {
    ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
        objective_id: "glass_lancer".to_owned(),
        objective_type: "KillActors".to_owned(),
        state: state.to_owned(),
        progress: u32::from(state == "Completed"),
        target: 1,
    })
}
