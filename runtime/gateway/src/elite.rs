use std::{error::Error, time::Instant};

use revenant_ai::{
    bulwark::{self, BulwarkAction},
    elite::{EliteAction, EliteBrain, EliteComposition},
    lancer::{self, ChargeAction},
};
use revenant_combat::AttackProfile;
use revenant_operations::RoutePhase;
use revenant_protocol::{ChargeTelegraph, DefenseTelegraph, RepairApplied};
use revenant_replay::{encode_elite_evidence, EliteEvidence, EliteStep, EliteTransition};

use super::{
    actor_spawn_message, ActorDestroy, ActorKind, ActorUpdate, AiEvent, CombatRuntime,
    DamageApplied, ObjectiveUpdate, ReplayEventKind, ServerMessage, SessionStage, SharedSession,
};

pub(super) struct EliteEncounter {
    composition: EliteComposition,
    pub(super) bulwark_id: u64,
    pub(super) partner_id: u64,
    player_id: u64,
    active: bool,
    started_at: Instant,
    confirmed_ms: u64,
    brain: EliteBrain,
}

impl SharedSession {
    pub(super) fn elite_fighting(&self) -> bool {
        self.elite.as_ref().is_some_and(|e| e.active)
    }

    pub(super) fn handle_elite_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn Error>> {
        if self.elite_fighting() {
            if bulwark::arena_contains(position) {
                return Ok(true);
            }
            self.record_elite(self.elite_elapsed(), EliteTransition::Abandoned, None)?;
            self.finish_elite("Failed");
            return Ok(false);
        }
        let available = self.elite.is_none()
            && self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].elite_capable
            && self
                .route
                .as_ref()
                .is_some_and(|route| route.operation.phase() == RoutePhase::ChoiceOpen);
        let Some(composition) = [
            EliteComposition::BastionLink,
            EliteComposition::CrossedGuard,
        ]
        .into_iter()
        .find(|composition| composition.entrance() == position) else {
            return Ok(false);
        };
        if !available {
            return Ok(false);
        }
        self.spawn_elite_encounter(player_id, composition)?;
        self.broadcast_route_state("Elite encounter selected; standard mission rewards")?;
        Ok(true)
    }

    pub(super) fn spawn_elite_encounter(
        &mut self,
        player_id: u64,
        composition: EliteComposition,
    ) -> Result<(), Box<dyn Error>> {
        let mut actors = self.actors.clone();
        let guard = actors.spawn(
            ActorKind::Enemy,
            "steel-bulwark",
            bulwark::SPAWN,
            bulwark::HEALTH,
        );
        let (archetype, spawn, health) = composition.partner();
        let partner = actors.spawn(ActorKind::Enemy, archetype, spawn, health);
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::EnemySpawned,
            &owner,
            Some(guard.id),
            &encode_elite_evidence(&EliteEvidence {
                elapsed_ms: 0,
                previous_confirmed_ms: 0,
                transition: EliteTransition::Started {
                    composition: composition.id().to_owned(),
                    bulwark_id: guard.id,
                    partner_id: partner.id,
                    player_health: actors.get(player_id).unwrap().health,
                },
            })?,
        )?;
        if let Some(route) = self.route.as_mut() {
            route.operation.lock_baseline()?;
        }
        self.actors = actors;
        let started_at = Instant::now();
        self.elite = Some(EliteEncounter {
            composition,
            bulwark_id: guard.id,
            partner_id: partner.id,
            player_id,
            active: true,
            started_at,
            confirmed_ms: 0,
            brain: EliteBrain::new(composition, guard.id, partner.id).unwrap(),
        });
        self.enemy_id = Some(guard.id);
        self.enemy_ai = None;
        self.combat = CombatRuntime::default();
        self.combat_started = started_at;
        self.broadcast(actor_spawn_message(&guard));
        self.broadcast(actor_spawn_message(&partner));
        self.broadcast_elite_objective("Active");
        self.broadcast_elite_defense();
        Ok(())
    }

    fn elite_elapsed(&self) -> u64 {
        self.elite.as_ref().map_or(0, |e| {
            u64::try_from(e.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        })
    }

    fn record_elite(
        &mut self,
        elapsed_ms: u64,
        transition: EliteTransition,
        death: Option<u64>,
    ) -> Result<(), Box<dyn Error>> {
        let e = self.elite.as_ref().unwrap();
        let actor_id = death.unwrap_or(e.player_id);
        let evidence = EliteEvidence {
            elapsed_ms,
            previous_confirmed_ms: e.confirmed_ms,
            transition,
        };
        let owner = self.owner_account()?.to_owned();
        if !self.record_challenge_elite_fatal(&evidence)? {
            self.append_event(
                if death.is_some() {
                    ReplayEventKind::EnemyDied
                } else {
                    ReplayEventKind::FieldActivity
                },
                &owner,
                Some(actor_id),
                &encode_elite_evidence(&evidence)?,
            )?;
        }
        self.elite.as_mut().unwrap().confirmed_ms = self.elite_elapsed().max(elapsed_ms);
        Ok(())
    }

    fn broadcast_elite_objective(&mut self, state: &str) {
        let e = self.elite.as_ref().unwrap();
        let progress = [e.bulwark_id, e.partner_id]
            .iter()
            .filter(|id| self.actors.get(**id).is_none())
            .count();
        self.broadcast(ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
            objective_id: e.composition.id().to_owned(),
            objective_type: "KillActors".to_owned(),
            state: state.to_owned(),
            progress: u32::try_from(progress).unwrap(),
            target: 2,
        }));
    }

    fn finish_elite(&mut self, state: &str) {
        self.broadcast_elite_objective(state);
        let e = self.elite.as_mut().unwrap();
        e.active = false;
        let ids = [e.bulwark_id, e.partner_id];
        for actor_id in ids {
            if self.actors.destroy(actor_id).is_some() {
                self.broadcast(ServerMessage::ActorDestroy(ActorDestroy { actor_id }));
            }
        }
        self.enemy_id = None;
    }

    fn broadcast_elite_defense(&mut self) {
        let e = self.elite.as_ref().unwrap();
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: e.bulwark_id,
            position: bulwark::SPAWN,
            charge: None,
            defense: Some(DefenseTelegraph {
                facing: e.brain.defense().facing().vector(),
                braced: e.brain.defense().braced(),
                warning_ms: u32::try_from(bulwark::WINDUP_MS).unwrap(),
            }),
        }));
    }

    pub(super) fn tick_elite(&mut self) -> Result<(), Box<dyn Error>> {
        self.tick_elite_at(self.elite_elapsed())
    }

    pub(super) fn tick_elite_at(&mut self, elapsed_ms: u64) -> Result<(), Box<dyn Error>> {
        if !self.elite_fighting() || self.stage == SessionStage::Failed {
            return Ok(());
        }
        let e = self.elite.as_ref().unwrap();
        let (guard_id, partner_id, player_id) = (e.bulwark_id, e.partner_id, e.player_id);
        let Some(player) = self.actors.get(player_id) else {
            return Ok(());
        };
        let mut proposed = e.brain.clone();
        let Some(action) = proposed.tick(
            elapsed_ms,
            self.actors.get(guard_id),
            self.actors.get(partner_id),
            player,
        ) else {
            return Ok(());
        };
        self.record_elite(
            elapsed_ms,
            EliteTransition::Step {
                player_position: player.position,
                step: EliteStep::from_action(action, player.health),
            },
            None,
        )?;
        proposed.confirm_at(action, self.elite.as_ref().unwrap().confirmed_ms);
        self.elite.as_mut().unwrap().brain = proposed;
        match action {
            EliteAction::Defense(defense) => {
                if let BulwarkAction::Slam { damage, .. } = defense {
                    self.apply_elite_damage(guard_id, player_id, damage);
                }
                self.broadcast_elite_defense();
            }
            EliteAction::Charge(charge) => {
                let (position, target, winding_up) = match charge {
                    ChargeAction::Windup { origin, target } => (origin, target, true),
                    ChargeAction::Resolved { target, damage, .. } => {
                        self.actors.update_position(partner_id, target);
                        self.apply_elite_damage(partner_id, player_id, damage);
                        (target, target, false)
                    }
                };
                self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
                    actor_id: partner_id,
                    position,
                    defense: None,
                    charge: Some(ChargeTelegraph {
                        target,
                        winding_up,
                        warning_ms: u32::try_from(lancer::WINDUP_MS)?,
                    }),
                }));
            }
            EliteAction::Repair(repair) => {
                let mut healed = self.actors.get(repair.target_id).unwrap().clone();
                healed.health = repair.health_after;
                self.actors.insert(healed);
                self.broadcast(ServerMessage::RepairApplied(RepairApplied {
                    source_actor_id: partner_id,
                    target_actor_id: repair.target_id,
                    amount: repair.amount,
                    remaining_health: repair.health_after,
                }));
            }
        }
        Ok(())
    }

    fn apply_elite_damage(&mut self, source_id: u64, player_id: u64, damage: u32) {
        if damage == 0 {
            return;
        }
        let victim = self.actors.apply_damage(player_id, damage).unwrap();
        self.apply_ai_event(&AiEvent::Attacked {
            source_actor_id: source_id,
            target_actor_id: player_id,
            damage,
            remaining_health: victim.health,
            killed: victim.health == 0,
        });
        if victim.health == 0 {
            self.stage = SessionStage::Failed;
            self.elite.as_mut().unwrap().active = false;
            self.broadcast_elite_objective("Failed");
            self.project_challenge();
        }
    }

    pub(super) fn attack_elite(
        &mut self,
        player_id: u64,
        target_id: u64,
        elapsed_ms: u64,
    ) -> Result<(), Box<dyn Error>> {
        let e = self.elite.as_ref().unwrap();
        if e.player_id != player_id || ![e.bulwark_id, e.partner_id].contains(&target_id) {
            return Err(std::io::Error::other("invalid elite target").into());
        }
        let player_position = self.actors.get(player_id).unwrap().position;
        let blocked =
            target_id == e.bulwark_id && e.brain.defense().blocks(bulwark::SPAWN, player_position);
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
        self.record_elite(
            elapsed_ms,
            EliteTransition::Attack {
                target_id,
                player_position,
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
            let e = self.elite.as_mut().unwrap();
            e.brain.retire(target_id, e.confirmed_ms);
            self.actors.destroy(target_id);
            self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
                actor_id: target_id,
            }));
            let e = self.elite.as_ref().unwrap();
            self.enemy_id = [e.bulwark_id, e.partner_id]
                .into_iter()
                .find(|id| self.actors.get(*id).is_some());
            if self.enemy_id.is_none() {
                self.finish_elite("Completed");
                if self.campaign.is_some() {
                    self.complete_counter_boundary(
                        revenant_campaign::Milestone::CounterBastionCleared,
                    )?;
                }
            } else {
                self.broadcast_elite_objective("Active");
            }
        }
        self.project_challenge();
        Ok(())
    }
}
