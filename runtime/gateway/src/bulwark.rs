use std::{error::Error, time::Instant};

use revenant_ai::bulwark::{
    arena_contains, BulwarkAction, SteelBulwark, ENTRANCE, HEALTH, SPAWN, WINDUP_MS,
};
use revenant_combat::AttackProfile;
use revenant_operations::RoutePhase;
use revenant_protocol::DefenseTelegraph;
use revenant_replay::{encode_bulwark_evidence, BulwarkEvidence};

use super::{
    actor_spawn_message, ActorDestroy, ActorKind, ActorUpdate, AiEvent, CombatRuntime,
    DamageApplied, ObjectiveUpdate, ReplayEventKind, ServerMessage, SessionStage, SharedSession,
};

pub(super) struct BulwarkEncounter {
    enemy_id: u64,
    player_id: u64,
    active: bool,
    started_at: Instant,
    ai: SteelBulwark,
}

impl SharedSession {
    pub(super) fn bulwark_fighting(&self) -> bool {
        self.bulwark
            .as_ref()
            .is_some_and(|encounter| encounter.active)
    }

    pub(super) fn handle_bulwark_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn Error>> {
        if self.bulwark_fighting() {
            if arena_contains(position) {
                return Ok(true);
            }
            let encounter = self.bulwark.as_ref().unwrap();
            let elapsed_ms =
                u64::try_from(encounter.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
            self.record_bulwark(&BulwarkEvidence::Abandoned { elapsed_ms }, false)?;
            self.finish_bulwark("Failed");
            return Ok(false);
        }
        let available = self.bulwark.is_none()
            && self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].bulwark_capable
            && self
                .route
                .as_ref()
                .is_some_and(|route| route.operation.phase() == RoutePhase::ChoiceOpen);
        if !available || position != ENTRANCE {
            return Ok(false);
        }
        let mut actors = self.actors.clone();
        let enemy = actors.spawn(ActorKind::Enemy, "steel-bulwark", SPAWN, HEALTH);
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::EnemySpawned,
            &owner,
            Some(enemy.id),
            "enemy spawned: steel-bulwark",
        )?;
        self.route.as_mut().unwrap().operation.lock_baseline()?;
        self.actors = actors;
        let started_at = Instant::now();
        self.bulwark = Some(BulwarkEncounter {
            enemy_id: enemy.id,
            player_id,
            active: true,
            started_at,
            ai: SteelBulwark::default(),
        });
        self.enemy_id = Some(enemy.id);
        self.enemy_ai = None;
        self.combat = CombatRuntime::default();
        self.combat_started = started_at;
        self.broadcast(actor_spawn_message(&enemy));
        self.broadcast(bulwark_objective("Active"));
        self.broadcast_bulwark_stance();
        self.broadcast_route_state("Steel Bulwark selected; standard mission rewards")?;
        Ok(true)
    }

    fn finish_bulwark(&mut self, state: &str) {
        let encounter = self.bulwark.as_mut().unwrap();
        encounter.active = false;
        let enemy_id = encounter.enemy_id;
        self.actors.destroy(enemy_id);
        self.enemy_id = None;
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: enemy_id,
        }));
        self.broadcast(bulwark_objective(state));
    }

    fn broadcast_bulwark_stance(&mut self) {
        let encounter = self.bulwark.as_ref().unwrap();
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: encounter.enemy_id,
            position: SPAWN,
            charge: None,
            defense: Some(DefenseTelegraph {
                facing: encounter.ai.facing().vector(),
                braced: encounter.ai.braced(),
                warning_ms: u32::try_from(WINDUP_MS).unwrap(),
            }),
        }));
    }

    pub(super) fn tick_bulwark(&mut self) -> Result<(), Box<dyn Error>> {
        let elapsed_ms = self.bulwark.as_ref().map_or(0, |encounter| {
            u64::try_from(encounter.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
        self.tick_bulwark_at(elapsed_ms)
    }

    pub(super) fn tick_bulwark_at(&mut self, elapsed_ms: u64) -> Result<(), Box<dyn Error>> {
        if !self.bulwark_fighting() || self.stage == SessionStage::Failed {
            return Ok(());
        }
        let encounter = self.bulwark.as_ref().unwrap();
        let (enemy_id, player_id) = (encounter.enemy_id, encounter.player_id);
        let Some(player) = self.actors.get(player_id) else {
            return Ok(());
        };
        if player.health == 0 {
            return Ok(());
        }
        let player_position = player.position;
        let health_before = player.health;
        let mut proposed = encounter.ai.clone();
        let Some(action) = proposed.tick(elapsed_ms, SPAWN, player_position) else {
            return Ok(());
        };
        match action {
            BulwarkAction::Brace { facing } => {
                self.record_bulwark(
                    &BulwarkEvidence::Braced {
                        facing: facing.vector(),
                        player_position,
                        elapsed_ms,
                    },
                    false,
                )?;
            }
            BulwarkAction::Slam { facing, damage } => {
                self.record_bulwark(
                    &BulwarkEvidence::Slam {
                        facing: facing.vector(),
                        player_position,
                        elapsed_ms,
                        damage,
                        health_before,
                        health_after: health_before.saturating_sub(damage),
                    },
                    false,
                )?;
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
                        self.bulwark.as_mut().unwrap().active = false;
                        self.broadcast(bulwark_objective("Failed"));
                    }
                }
            }
        }
        let encounter = self.bulwark.as_mut().unwrap();
        let confirmed_ms =
            u64::try_from(encounter.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        proposed.confirm_at(confirmed_ms.max(elapsed_ms));
        encounter.ai = proposed;
        self.broadcast_bulwark_stance();
        Ok(())
    }

    fn record_bulwark(
        &mut self,
        evidence: &BulwarkEvidence,
        killed: bool,
    ) -> Result<(), Box<dyn Error>> {
        let encounter = self.bulwark.as_ref().unwrap();
        let (kind, actor) = if killed {
            (ReplayEventKind::EnemyDied, encounter.enemy_id)
        } else {
            (ReplayEventKind::FieldActivity, encounter.player_id)
        };
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            kind,
            &owner,
            Some(actor),
            &encode_bulwark_evidence(evidence)?,
        )
    }

    pub(super) fn attack_bulwark(
        &mut self,
        player_id: u64,
        target_id: u64,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn Error>> {
        let encounter = self.bulwark.as_ref().unwrap();
        if encounter.player_id != player_id || encounter.enemy_id != target_id {
            return Err(std::io::Error::other("invalid Bulwark target").into());
        }
        let player_position = self.actors.get(player_id).unwrap().position;
        let health_before = self.actors.get(target_id).unwrap().health;
        let blocked = encounter.ai.blocks(SPAWN, player_position);
        let profile = self.positioned_attack_profile(player_id, target_id)?;
        let mut actors = self.actors.clone();
        let mut combat = self.combat.clone();
        let result = combat.attack(
            &mut actors,
            player_id,
            target_id,
            elapsed_ms,
            AttackProfile {
                damage: if blocked { 0 } else { profile.damage },
                range: profile.range,
                cooldown_ms: profile.cooldown_ms,
            },
        )?;
        // A fatal hit and its death share one durable row; retries cannot split them.
        self.record_bulwark(
            &BulwarkEvidence::Attack {
                player_position,
                elapsed_ms,
                damage: result.damage,
                health_before,
                health_after: result.remaining_health,
            },
            result.killed,
        )?;
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
            self.finish_bulwark("Completed");
        }
        Ok(())
    }
}

fn bulwark_objective(state: &str) -> ServerMessage {
    ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
        objective_id: "steel_bulwark".to_owned(),
        objective_type: "KillActors".to_owned(),
        state: state.to_owned(),
        progress: u32::from(state == "Completed"),
        target: 1,
    })
}
