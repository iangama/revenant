use std::{error::Error, time::Instant};

use revenant_ai::{
    lancer::{self, ChargeAction, GlassLancer},
    support::{self, RelayMender},
};
use revenant_combat::AttackProfile;
use revenant_operations::RoutePhase;
use revenant_protocol::{ChargeTelegraph, RepairApplied};
use revenant_replay::{encode_support_evidence, SupportEvidence, SupportTransition};

use super::{
    actor_spawn_message, ActorDestroy, ActorKind, ActorUpdate, AiEvent, CombatRuntime,
    DamageApplied, ObjectiveUpdate, ReplayEventKind, ServerMessage, SessionStage, SharedSession,
};

pub(super) struct SupportEncounter {
    pub(super) lancer_id: u64,
    pub(super) mender_id: u64,
    player_id: u64,
    active: bool,
    started_at: Instant,
    charge_ai: GlassLancer,
    repair_ai: RelayMender,
}

impl SharedSession {
    pub(super) fn support_fighting(&self) -> bool {
        self.support
            .as_ref()
            .is_some_and(|encounter| encounter.active)
    }

    pub(super) fn handle_support_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn Error>> {
        if self.support_fighting() {
            if lancer::arena_contains(position) {
                return Ok(true);
            }
            let elapsed = self.support_elapsed();
            self.record_support(elapsed, SupportTransition::Abandoned, None)?;
            self.finish_support("Failed");
            return Ok(false);
        }
        let available = self.support.is_none()
            && self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].support_capable
            && self
                .route
                .as_ref()
                .is_some_and(|route| route.operation.phase() == RoutePhase::ChoiceOpen);
        if !available || position != support::ENTRANCE {
            return Ok(false);
        }
        self.spawn_support_encounter(player_id)?;
        self.broadcast_route_state("Relay Mender pair selected; standard mission rewards")?;
        Ok(true)
    }

    pub(super) fn spawn_support_encounter(&mut self, player_id: u64) -> Result<(), Box<dyn Error>> {
        let mut actors = self.actors.clone();
        let lancer = actors.spawn(
            ActorKind::Enemy,
            "glass-lancer",
            lancer::SPAWN,
            lancer::HEALTH,
        );
        let mender = actors.spawn(
            ActorKind::Enemy,
            "relay-mender",
            support::SPAWN,
            support::HEALTH,
        );
        let owner = self.owner_account()?.to_owned();
        // Both actors appear from one durable row, so a failed write cannot leave half a pair.
        self.append_event(
            ReplayEventKind::EnemySpawned,
            &owner,
            Some(lancer.id),
            &encode_support_evidence(&SupportEvidence {
                elapsed_ms: 0,
                transition: SupportTransition::Started {
                    lancer_id: lancer.id,
                    mender_id: mender.id,
                    player_health: actors.get(player_id).unwrap().health,
                },
            })?,
        )?;
        if let Some(route) = self.route.as_mut() {
            route.operation.lock_baseline()?;
        }
        self.actors = actors;
        let started_at = Instant::now();
        self.support = Some(SupportEncounter {
            lancer_id: lancer.id,
            mender_id: mender.id,
            player_id,
            active: true,
            started_at,
            charge_ai: GlassLancer::default(),
            repair_ai: RelayMender::default(),
        });
        self.enemy_id = Some(lancer.id);
        self.enemy_ai = None;
        self.combat = CombatRuntime::default();
        self.combat_started = started_at;
        self.broadcast(actor_spawn_message(&lancer));
        self.broadcast(actor_spawn_message(&mender));
        self.broadcast_support_objective("Active");
        Ok(())
    }

    fn support_elapsed(&self) -> u64 {
        self.support.as_ref().map_or(0, |e| {
            u64::try_from(e.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        })
    }

    fn record_support(
        &mut self,
        elapsed_ms: u64,
        transition: SupportTransition,
        death: Option<u64>,
    ) -> Result<(), Box<dyn Error>> {
        let evidence = SupportEvidence {
            elapsed_ms,
            transition,
        };
        if self.record_challenge_fatal(&evidence)? {
            return Ok(());
        }
        let player_id = self.support.as_ref().unwrap().player_id;
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            if death.is_some() {
                ReplayEventKind::EnemyDied
            } else {
                ReplayEventKind::FieldActivity
            },
            &owner,
            Some(death.unwrap_or(player_id)),
            &encode_support_evidence(&evidence)?,
        )
    }

    fn broadcast_support_objective(&mut self, state: &str) {
        let e = self.support.as_ref().unwrap();
        let progress = [e.lancer_id, e.mender_id]
            .iter()
            .filter(|id| self.actors.get(**id).is_none())
            .count();
        self.broadcast(ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
            objective_id: "relay_mender".to_owned(),
            objective_type: "KillActors".to_owned(),
            state: state.to_owned(),
            progress: u32::try_from(progress).unwrap(),
            target: 2,
        }));
    }

    fn finish_support(&mut self, state: &str) {
        self.broadcast_support_objective(state);
        let e = self.support.as_mut().unwrap();
        e.active = false;
        let ids = [e.lancer_id, e.mender_id];
        for actor_id in ids {
            if self.actors.destroy(actor_id).is_some() {
                self.broadcast(ServerMessage::ActorDestroy(ActorDestroy { actor_id }));
            }
        }
        self.enemy_id = None;
    }

    pub(super) fn tick_support(&mut self) -> Result<(), Box<dyn Error>> {
        self.tick_support_at(self.support_elapsed())
    }

    pub(super) fn tick_support_at(&mut self, elapsed_ms: u64) -> Result<(), Box<dyn Error>> {
        if !self.support_fighting() || self.stage == SessionStage::Failed {
            return Ok(());
        }
        self.tick_support_charge(elapsed_ms)?;
        if !self.support_fighting() {
            return Ok(());
        }
        let e = self.support.as_ref().unwrap();
        let (Some(source), Some(target)) =
            (self.actors.get(e.mender_id), self.actors.get(e.lancer_id))
        else {
            return Ok(());
        };
        let mut proposed = e.repair_ai.clone();
        let Some(action) = proposed.tick(elapsed_ms, source, std::slice::from_ref(target)) else {
            return Ok(());
        };
        let source_id = source.id;
        let mut healed = target.clone();
        self.record_support(
            elapsed_ms,
            SupportTransition::Repaired {
                target_id: action.target_id,
                amount: action.amount,
                health_before: action.health_before,
                health_after: action.health_after,
            },
            None,
        )?;
        healed.health = action.health_after;
        self.actors.insert(healed);
        proposed.confirm_at(self.support_elapsed().max(elapsed_ms));
        self.support.as_mut().unwrap().repair_ai = proposed;
        self.broadcast(ServerMessage::RepairApplied(RepairApplied {
            source_actor_id: source_id,
            target_actor_id: action.target_id,
            amount: action.amount,
            remaining_health: action.health_after,
        }));
        Ok(())
    }

    fn tick_support_charge(&mut self, elapsed_ms: u64) -> Result<(), Box<dyn Error>> {
        let e = self.support.as_ref().unwrap();
        let (enemy_id, player_id) = (e.lancer_id, e.player_id);
        let (Some(enemy), Some(player)) = (self.actors.get(enemy_id), self.actors.get(player_id))
        else {
            return Ok(());
        };
        if enemy.health == 0 || player.health == 0 {
            return Ok(());
        }
        let player_position = player.position;
        let health_before = player.health;
        let mut proposed = e.charge_ai.clone();
        let Some(action) = proposed.tick(elapsed_ms, enemy.position, player_position) else {
            return Ok(());
        };
        let (position, target, winding_up) = match action {
            ChargeAction::Windup { origin, target } => {
                self.record_support(
                    elapsed_ms,
                    SupportTransition::Windup {
                        player_position,
                        origin,
                        target,
                    },
                    None,
                )?;
                (origin, target, true)
            }
            ChargeAction::Resolved {
                origin,
                target,
                damage,
            } => {
                self.record_support(
                    elapsed_ms,
                    SupportTransition::Charged {
                        player_position,
                        origin,
                        target,
                        damage,
                        health_after: health_before.saturating_sub(damage),
                    },
                    None,
                )?;
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
                        self.support.as_mut().unwrap().active = false;
                        self.broadcast_support_objective("Failed");
                        self.project_challenge();
                    }
                }
                (target, target, false)
            }
        };
        proposed.confirm_at(self.support_elapsed().max(elapsed_ms));
        self.support.as_mut().unwrap().charge_ai = proposed;
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: enemy_id,
            position,
            defense: None,
            charge: Some(ChargeTelegraph {
                target,
                winding_up,
                warning_ms: u32::try_from(lancer::WINDUP_MS)?,
            }),
        }));
        Ok(())
    }

    pub(super) fn attack_support(
        &mut self,
        player_id: u64,
        target_id: u64,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn Error>> {
        let e = self.support.as_ref().unwrap();
        if e.player_id != player_id || ![e.lancer_id, e.mender_id].contains(&target_id) {
            return Err(std::io::Error::other("invalid support encounter target").into());
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
        self.record_support(
            elapsed_ms,
            SupportTransition::Attack {
                target_id,
                player_position: self.actors.get(player_id).unwrap().position,
                damage: result.damage,
                health_before: self.actors.get(target_id).unwrap().health,
                health_after: result.remaining_health,
            },
            result.killed.then_some(target_id),
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
            self.actors.destroy(target_id);
            self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
                actor_id: target_id,
            }));
            let e = self.support.as_ref().unwrap();
            self.enemy_id = [e.lancer_id, e.mender_id]
                .into_iter()
                .find(|id| self.actors.get(*id).is_some());
            if self.enemy_id.is_none() {
                self.finish_support("Completed");
                if self.campaign.is_some() {
                    self.complete_counter_boundary(
                        revenant_campaign::Milestone::CounterSupportCleared,
                    )?;
                }
            } else {
                self.broadcast_support_objective("Active");
            }
            self.project_challenge();
        }
        Ok(())
    }
}
