use std::{error::Error, io, time::Instant};

use revenant_activities::ScriptedActivity;
use revenant_ai::prism::{self, PrismAction, PrismMode, PrismPattern, PrismPhase, PrismWarden};
use revenant_combat::AttackProfile;
use revenant_objectives::WorldTrigger;
use revenant_operations::RoutePhase;
use revenant_protocol::{
    PrismMode as WireMode, PrismPattern as WirePattern, PrismPhase as WirePhase, PrismState,
};
use revenant_replay::{encode_prism_evidence, PrismEvidence, PrismStep, PrismTransition};

use super::{
    activity_messages, actor_spawn_message, ActorDestroy, ActorKind, AiEvent, CombatRuntime,
    DamageApplied, ObjectiveUpdate, ReplayEventKind, ServerMessage, SessionStage, SharedSession,
};

pub(super) struct PrismEncounter {
    boss_id: u64,
    player_id: u64,
    active: bool,
    started_at: Instant,
    confirmed_ms: u64,
    brain: PrismWarden,
    before_activity: ScriptedActivity,
    pending_fatal: Option<ServerMessage>,
    pending_campaign_proof: Option<i64>,
}

impl SharedSession {
    pub(super) fn prism_fighting(&self) -> bool {
        self.prism.as_ref().is_some_and(|e| e.active)
    }

    pub(super) fn handle_prism_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn Error>> {
        if self.prism_fighting() {
            if self.prism.as_ref().unwrap().pending_fatal.is_some()
                || prism::arena_contains(position)
            {
                return Ok(true);
            }
            self.record_prism(self.prism_elapsed(), PrismTransition::Abandoned, false)?;
            let e = self.prism.as_mut().unwrap();
            let boss_id = e.boss_id;
            e.active = false;
            self.activity = e.before_activity.clone();
            self.stage = SessionStage::Door;
            self.enemy_id = None;
            self.actors.destroy(boss_id);
            self.broadcast_prism_objective("Failed");
            self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
                actor_id: boss_id,
            }));
            self.broadcast(ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
                objective_id: "defeat_warden".to_owned(),
                objective_type: "Boss".to_owned(),
                state: "Pending".to_owned(),
                progress: 0,
                target: 1,
            }));
            self.broadcast(ServerMessage::DoorState(super::DoorState {
                door_id: "relay_core".to_owned(),
                open: false,
            }));
            for message in activity_messages(self.activity.start()).0 {
                if matches!(message, ServerMessage::ObjectiveUpdate(_)) {
                    self.broadcast(message);
                }
            }
            return Ok(false);
        }
        let available = self.prism.is_none()
            && self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].prism_capable
            && self
                .route
                .as_ref()
                .is_some_and(|r| r.operation.phase() == RoutePhase::ChoiceOpen);
        if !available || position != prism::ENTRANCE {
            return Ok(false);
        }
        self.spawn_prism_encounter(player_id)?;
        self.broadcast_route_state("Prism Warden selected; standard mission rewards")?;
        Ok(true)
    }

    /// The caller owns admission and completion policy. The shared encounter
    /// retains the same actor, timing, evidence and two-phase combat mechanics.
    pub(super) fn spawn_prism_encounter(&mut self, player_id: u64) -> Result<(), Box<dyn Error>> {
        let mut actors = self.actors.clone();
        let boss = actors.spawn(
            ActorKind::Enemy,
            "prism-warden",
            prism::SPAWN,
            prism::HEALTH,
        );
        let before_activity = self.activity.clone();
        let mut activity = before_activity.clone();
        let messages = if self.challenge.is_some() {
            Vec::new()
        } else {
            activity_messages(activity.apply_trigger(&WorldTrigger::AreaReached {
                area_id: "relay_door".to_owned(),
            }))
            .0
        };
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::BossSpawned,
            &owner,
            Some(boss.id),
            &encode_prism_evidence(&PrismEvidence {
                elapsed_ms: 0,
                previous_confirmed_ms: 0,
                transition: PrismTransition::Started {
                    boss_id: boss.id,
                    player_health: actors.get(player_id).unwrap().health,
                },
            })?,
        )?;
        if let Some(route) = self.route.as_mut() {
            route.operation.lock_baseline()?;
        }
        self.actors = actors;
        self.activity = activity;
        let started_at = Instant::now();
        self.prism = Some(PrismEncounter {
            boss_id: boss.id,
            player_id,
            active: true,
            started_at,
            confirmed_ms: 0,
            brain: PrismWarden::default(),
            before_activity,
            pending_fatal: None,
            pending_campaign_proof: None,
        });
        self.stage = SessionStage::Boss;
        self.enemy_id = Some(boss.id);
        self.enemy_ai = None;
        self.combat = CombatRuntime::default();
        self.combat_started = started_at;
        for message in messages {
            self.broadcast(message);
        }
        self.broadcast(actor_spawn_message(&boss));
        self.broadcast_prism_objective("Active");
        self.broadcast_prism_state();
        Ok(())
    }

    fn prism_elapsed(&self) -> u64 {
        self.prism.as_ref().map_or(0, |e| {
            u64::try_from(e.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
        })
    }

    fn record_prism(
        &mut self,
        time: u64,
        transition: PrismTransition,
        death: bool,
    ) -> Result<(), Box<dyn Error>> {
        let e = self.prism.as_ref().unwrap();
        let actor_id = if death { e.boss_id } else { e.player_id };
        let evidence = PrismEvidence {
            elapsed_ms: time,
            previous_confirmed_ms: e.confirmed_ms,
            transition,
        };
        if !self.record_challenge_prism_fatal(e.boss_id, &evidence)? {
            let owner = self.owner_account()?.to_owned();
            self.append_event(
                if death {
                    ReplayEventKind::EnemyDied
                } else {
                    ReplayEventKind::FieldActivity
                },
                &owner,
                Some(actor_id),
                &encode_prism_evidence(&evidence)?,
            )?;
        }
        self.prism.as_mut().unwrap().confirmed_ms = self.prism_elapsed().max(time);
        Ok(())
    }

    fn broadcast_prism_objective(&mut self, state: &str) {
        self.broadcast(ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
            objective_id: "prism_warden".to_owned(),
            objective_type: "KillActors".to_owned(),
            state: state.to_owned(),
            progress: u32::from(state == "Completed"),
            target: 1,
        }));
    }

    fn broadcast_prism_state(&mut self) {
        let e = self.prism.as_ref().unwrap();
        let (mode, pattern, interval) = match e.brain.mode() {
            PrismMode::Opening => (WireMode::Opening, None, prism::OPENING_MS),
            PrismMode::Warning(p) => (
                WireMode::Warning,
                Some(match p {
                    PrismPattern::AcrossX { z } => WirePattern::AcrossX { z },
                    PrismPattern::AcrossZ { x } => WirePattern::AcrossZ { x },
                    PrismPattern::Center => WirePattern::Center,
                    PrismPattern::Perimeter => WirePattern::Perimeter,
                }),
                p.warning_ms(),
            ),
            PrismMode::PulseGap => (WireMode::PulseGap, None, prism::PULSE_GAP_MS),
            PrismMode::Recovery => (WireMode::Recovery, None, prism::RECOVERY_MS),
            PrismMode::Shifting => (WireMode::Shifting, None, prism::SHIFT_MS),
            PrismMode::Defeated => (WireMode::Defeated, None, 0),
        };
        self.broadcast(ServerMessage::PrismState(PrismState {
            actor_id: e.boss_id,
            phase: match e.brain.phase() {
                PrismPhase::Lanes => WirePhase::Lanes,
                PrismPhase::Pulses => WirePhase::Pulses,
            },
            mode,
            pattern,
            interval_ms: u32::try_from(interval).unwrap(),
        }));
    }

    pub(super) fn tick_prism(&mut self) -> Result<(), Box<dyn Error>> {
        self.tick_prism_at(self.prism_elapsed())
    }

    pub(super) fn tick_prism_at(&mut self, time: u64) -> Result<(), Box<dyn Error>> {
        if !self.prism_fighting() || self.stage == SessionStage::Failed {
            return Ok(());
        }
        let e = self.prism.as_ref().unwrap();
        if e.pending_fatal.is_some() {
            return self.finish_prism();
        }
        let (boss_id, player_id) = (e.boss_id, e.player_id);
        let player = self.actors.get(player_id).unwrap();
        let (position, health) = (player.position, player.health);
        if health == 0 {
            return Ok(());
        }
        let mut brain = e.brain.clone();
        let Some(action) = brain.tick(time, self.actors.get(boss_id).unwrap().health, position)
        else {
            return Ok(());
        };
        self.record_prism(
            time,
            PrismTransition::Step {
                player_position: position,
                step: PrismStep::from_action(action, health),
            },
            false,
        )?;
        brain.confirm_at(self.prism.as_ref().unwrap().confirmed_ms);
        self.prism.as_mut().unwrap().brain = brain;
        if let PrismAction::Resolved { damage, .. } = action {
            if damage > 0 {
                let victim = self.actors.apply_damage(player_id, damage).unwrap();
                self.apply_ai_event(&AiEvent::Attacked {
                    source_actor_id: boss_id,
                    target_actor_id: player_id,
                    damage,
                    remaining_health: victim.health,
                    killed: victim.health == 0,
                });
                if victim.health == 0 {
                    self.stage = SessionStage::Failed;
                    self.prism.as_mut().unwrap().active = false;
                    self.broadcast_prism_objective("Failed");
                    self.project_challenge();
                }
            }
        }
        self.broadcast_prism_state();
        Ok(())
    }

    pub(super) fn attack_prism(
        &mut self,
        player_id: u64,
        target_id: u64,
        time: u64,
    ) -> Result<(), Box<dyn Error>> {
        let e = self.prism.as_ref().unwrap();
        if e.player_id != player_id || e.boss_id != target_id {
            return Err(io::Error::other("invalid Prism target").into());
        }
        if e.pending_fatal.is_some() {
            return self.finish_prism();
        }
        let position = self.actors.get(player_id).unwrap().position;
        if !prism::arena_contains(position) {
            return Err(io::Error::other("outside Prism arena").into());
        }
        let before = self.actors.get(target_id).unwrap().health;
        let profile = self.positioned_attack_profile(player_id, target_id)?;
        let mut brain = e.brain.clone();
        let hit = brain
            .hit(time, before, profile.damage)
            .ok_or_else(|| io::Error::other("Prism is not attackable"))?;
        let mut actors = self.actors.clone();
        let mut combat = self.combat.clone();
        let result = combat.attack(
            &mut actors,
            player_id,
            target_id,
            time,
            AttackProfile {
                damage: hit.damage,
                range: profile.range,
                cooldown_ms: profile.cooldown_ms,
            },
        )?;
        self.record_prism(
            time,
            PrismTransition::Attack {
                player_position: position,
                damage: result.damage,
                health_before: before,
                health_after: result.remaining_health,
                phase_changed: hit.phase_changed,
            },
            result.killed,
        )?;
        if hit.phase_changed {
            brain.confirm_at(self.prism.as_ref().unwrap().confirmed_ms);
        }
        self.prism.as_mut().unwrap().brain = brain;
        self.actors = actors;
        self.combat = combat;
        let message = ServerMessage::DamageApplied(DamageApplied {
            source_actor_id: player_id,
            target_actor_id: target_id,
            damage: result.damage,
            remaining_health: result.remaining_health,
            killed: result.killed,
        });
        if result.killed {
            self.prism.as_mut().unwrap().pending_fatal = Some(message);
            self.finish_prism()?;
        } else {
            self.broadcast(message);
            if hit.phase_changed {
                self.broadcast_prism_state();
            }
        }
        Ok(())
    }

    /// Death is already durable. Retry only completion after failure; never append
    /// the fatal hit twice or ask the player to shoot an internally dead actor.
    fn finish_prism(&mut self) -> Result<(), Box<dyn Error>> {
        if self.challenge.as_ref().is_some_and(|r| {
            matches!(
                r.contract,
                revenant_challenges::ContractId::PrismDiscipline
                    | revenant_challenges::ContractId::RelayGauntlet
            )
        }) {
            self.finish_challenge_prism();
            return Ok(());
        }
        if self
            .campaign
            .as_ref()
            .and_then(|s| s.active.as_ref())
            .is_some_and(|r| r.chapter == revenant_campaign::ChapterId::PrismCore)
        {
            return self.finish_campaign_prism();
        }
        let owner = self.owner_account()?.to_owned();
        let (rewards, _) = self.complete_activity(&owner)?;
        let e = self.prism.as_mut().unwrap();
        let fatal = e.pending_fatal.take().unwrap();
        let boss_id = e.boss_id;
        e.active = false;
        let messages =
            activity_messages(self.activity.apply_trigger(&WorldTrigger::ActorGroupDead {
                group_id: "warden".to_owned(),
            }))
            .0;
        self.actors.destroy(boss_id);
        self.broadcast(fatal);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: boss_id,
        }));
        self.broadcast_prism_objective("Completed");
        for message in &messages {
            if !matches!(message, ServerMessage::ActivityComplete(_)) {
                self.broadcast(message.clone());
            }
        }
        for (outbound, message) in rewards {
            let _ = outbound.send(message);
        }
        for message in messages {
            if matches!(message, ServerMessage::ActivityComplete(_)) {
                self.broadcast(message);
            }
        }
        Ok(())
    }

    fn finish_challenge_prism(&mut self) {
        let encounter = self.prism.as_mut().unwrap();
        let fatal = encounter.pending_fatal.take().unwrap();
        let boss_id = encounter.boss_id;
        encounter.active = false;
        self.enemy_id = None;
        self.actors.destroy(boss_id);
        self.broadcast(fatal);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: boss_id,
        }));
        self.broadcast_prism_objective("Completed");
        self.project_challenge();
    }

    fn finish_campaign_prism(&mut self) -> Result<(), Box<dyn Error>> {
        let milestone = revenant_campaign::Milestone::PrismWardenCleared;
        let proof = if let Some(proof) = self.prism.as_ref().unwrap().pending_campaign_proof {
            proof
        } else {
            let proof = self.campaign_event(
                ReplayEventKind::CampaignObjective,
                Some(self.participants[0].actor.id),
                revenant_campaign::prism_objective(milestone).expect("Prism clear boundary"),
            )?;
            self.prism.as_mut().unwrap().pending_campaign_proof = Some(proof);
            proof
        };
        let receipt = self.confirm_campaign(milestone, proof)?;
        let encounter = self.prism.as_mut().unwrap();
        let fatal = encounter.pending_fatal.take().unwrap();
        let boss_id = encounter.boss_id;
        encounter.active = false;
        self.enemy_id = None;
        self.stage = SessionStage::Door;
        self.actors.destroy(boss_id);
        self.broadcast(fatal);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: boss_id,
        }));
        self.broadcast_prism_objective("Completed");
        self.broadcast_campaign_prism_trigger(milestone)?;
        self.publish_campaign_receipt(&receipt)
    }
}
