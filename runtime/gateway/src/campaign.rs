use std::{error::Error, io};

use revenant_activities::meridian::{Checkpoint, Expedition};
use revenant_campaign::{CampaignState, ChapterId, Command, Milestone, RunMode};
use revenant_persistence::{CampaignReceipt, CampaignRequest, NewReplayEvent};
use revenant_protocol::{CampaignEntryMode, CampaignRun, CampaignSnapshot};

use super::{
    activity_messages, actor_spawn_message, experience_to_next_level, new_session_id,
    valid_movement_target, ActivityComplete, ActorDestroy, ActorKind, ActorUpdate, AiController,
    LootGranted, ModuleMutationReplayContext, Participant, ProgressionGranted, ReplayEventKind,
    ScriptedActivity, ServerMessage, SessionStage, SharedSession, RELAY_CORE_FRAGMENT,
    RELAY_DRONE_BASE_HEALTH, RELAY_DRONE_PRESSURE_PROFILE, WARDEN_BASE_HEALTH,
    WARDEN_PRESSURE_PROFILE,
};

pub(super) type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) struct PreparedEntry {
    state: CampaignState,
    command: Option<Command>,
}

pub(super) fn snapshot(state: &CampaignState) -> CampaignSnapshot {
    CampaignSnapshot {
        revision: revenant_campaign::REVISION.to_owned(),
        state_revision: state.revision,
        cleared_chapters: state.cleared_chapters,
        available_chapters: revenant_campaign::AVAILABLE_CHAPTERS,
        total_chapters: revenant_campaign::TOTAL_CHAPTERS,
        active: state.active.as_ref().map(|run| CampaignRun {
            run_id: run.run_id.clone(),
            chapter_id: run.chapter.as_str().to_owned(),
            practice: run.mode == RunMode::Practice,
            checkpoint: run.checkpoint,
        }),
        first_clear_fragments: revenant_campaign::FIRST_CLEAR_FRAGMENTS,
        first_clear_experience: revenant_campaign::FIRST_CLEAR_EXPERIENCE,
        story: super::campaign_story::snapshot(&state.story),
    }
}

fn meridian_checkpoint(checkpoint: u8) -> Result<Checkpoint> {
    match checkpoint {
        0 => Ok(Checkpoint::Entrance),
        1 => Ok(Checkpoint::Arrival),
        2 => Ok(Checkpoint::SurveyComplete),
        _ => Err(io::Error::other("unsupported Meridian checkpoint").into()),
    }
}

impl SharedSession {
    pub(super) fn prepare_campaign_entry(
        &mut self,
        player: &Participant,
    ) -> Result<Option<PreparedEntry>> {
        let Some(request) = &player.campaign_entry else {
            return Ok(None);
        };
        if self.expected_players != 1
            || !self.participants.is_empty()
            || player.protocol_generation != super::ProtocolGeneration::CurrentV2
        {
            return Err(io::Error::other("campaign requires a solo V2 session").into());
        }
        let chapter = ChapterId::ALL
            .into_iter()
            .find(|c| c.as_str() == request.chapter_id)
            .ok_or_else(|| io::Error::other("unknown campaign chapter"))?;
        if chapter == ChapterId::PrismCore && !player.prism_capable {
            return Err(
                io::Error::other("Prism core requires the Prism encounter capability").into(),
            );
        }
        let state = self
            .persistence
            .campaign_state_for(&player.account_id, &player.character_id)?;
        if state.revision != request.expected_revision {
            return Err(io::Error::other("campaign save changed; refresh before entering").into());
        }
        let command = match request.mode {
            CampaignEntryMode::Resume => {
                if state
                    .active
                    .as_ref()
                    .is_none_or(|run| run.chapter != chapter)
                {
                    return Err(io::Error::other("this campaign chapter has no active save").into());
                }
                None
            }
            CampaignEntryMode::Progress | CampaignEntryMode::Practice => {
                let command = Command::Enter {
                    run_id: new_session_id()?,
                    chapter,
                    mode: if request.mode == CampaignEntryMode::Practice {
                        RunMode::Practice
                    } else {
                        RunMode::Progress
                    },
                };
                state.propose(request.expected_revision, command.clone())?;
                Some(command)
            }
        };
        let source = match chapter {
            ChapterId::ReturnSignal => {
                include_str!("../../../scripts/activities/campaign_return.lua")
            }
            ChapterId::MeridianReadings => {
                include_str!("../../../scripts/activities/campaign_meridian.lua")
            }
            ChapterId::BrokenSupplyLine => {
                include_str!("../../../scripts/activities/campaign_supply.lua")
            }
            ChapterId::CounterSignal => {
                include_str!("../../../scripts/activities/campaign_counter.lua")
            }
            ChapterId::TheBreach
                if state.story.core_approach()
                    == revenant_campaign::story::CoreApproach::Direct =>
            {
                include_str!("../../../scripts/activities/campaign_breach_direct.lua")
            }
            ChapterId::TheBreach => include_str!("../../../scripts/activities/campaign_breach.lua"),
            ChapterId::PrismCore => include_str!("../../../scripts/activities/campaign_prism.lua"),
        };
        self.activity = ScriptedActivity::from_source(source)?;
        Ok(Some(PreparedEntry { state, command }))
    }

    pub(super) fn commit_campaign_entry(
        &mut self,
        player: &mut Participant,
        prepared: Option<PreparedEntry>,
    ) -> Result<()> {
        let Some(prepared) = prepared else {
            return Ok(());
        };
        let context = ModuleMutationReplayContext {
            catalog_revision: player.arsenal_catalog(),
            session_id: &self.session_id,
            account_id: &player.account_id,
            activity_id: self.activity.id(),
            actor_id: i64::try_from(player.actor.id)?,
        };
        let state = if let Some(command) = prepared.command {
            self.persistence
                .apply_campaign_command(
                    &player.character_id,
                    &CampaignRequest {
                        operation_id: &format!("entry-{}", self.session_id),
                        expected_revision: prepared.state.revision,
                        command,
                        proof_event_id: None,
                    },
                    context,
                )?
                .state
        } else {
            self.persistence.resume_campaign_with_replay(
                &player.character_id,
                prepared.state.revision,
                context,
            )?
        };
        let run = state
            .active
            .as_ref()
            .ok_or_else(|| io::Error::other("campaign entry has no active chapter"))?;
        player.actor.position = match run.chapter {
            ChapterId::ReturnSignal if run.checkpoint > 0 => [4, 0, 0],
            ChapterId::MeridianReadings => meridian_checkpoint(run.checkpoint)?.position(),
            ChapterId::BrokenSupplyLine
                if run.checkpoint == 1
                    && state.story.supply_approach()
                        == revenant_campaign::story::SupplyApproach::Service =>
            {
                revenant_campaign::story::SERVICE_APPROACH
            }
            ChapterId::BrokenSupplyLine => super::campaign_supply::spawn_position(run.checkpoint)?,
            ChapterId::CounterSignal => super::campaign_counter::spawn_position(run.checkpoint)?,
            ChapterId::TheBreach
                if state.story.core_approach()
                    == revenant_campaign::story::CoreApproach::Direct =>
            {
                super::campaign_breach::direct_spawn_position(run.checkpoint)?
            }
            ChapterId::TheBreach => super::campaign_breach::spawn_position(run.checkpoint)?,
            ChapterId::PrismCore => super::campaign_prism::spawn_position(run.checkpoint)?,
            ChapterId::ReturnSignal => [0, 0, 0],
        };
        self.campaign = Some(state);
        Ok(())
    }

    pub(super) fn campaign_event(
        &mut self,
        kind: ReplayEventKind,
        actor_id: Option<u64>,
        payload: &str,
    ) -> Result<i64> {
        self.persistence.append_replay_event(&NewReplayEvent {
            event_type: &kind.to_string(),
            session_id: &self.session_id,
            account_id: &self.participants[0].account_id,
            activity_id: Some(self.activity.id()),
            actor_id: actor_id.map(i64::try_from).transpose()?,
            payload,
        })
    }

    pub(super) fn start_campaign(&mut self) -> Result<()> {
        let run = self
            .campaign
            .as_ref()
            .and_then(|s| s.active.clone())
            .ok_or_else(|| io::Error::other("campaign run missing"))?;
        self.campaign_event(ReplayEventKind::ActivityStarted, None, "activity started")?;
        self.stage = SessionStage::Door;
        if run.chapter == ChapterId::ReturnSignal && run.checkpoint == 1 {
            self.activity
                .apply_trigger(&super::WorldTrigger::ActorGroupDead {
                    group_id: "relay_drones".to_owned(),
                });
        }
        self.restore_authored_campaign_objectives(&run)?;
        for message in activity_messages(self.activity.start()).0 {
            self.broadcast(message);
        }
        self.broadcast(actor_spawn_message(&self.participants[0].actor));
        self.broadcast(ServerMessage::CampaignSnapshot(snapshot(
            self.campaign.as_ref().expect("campaign exists"),
        )));
        match run.chapter {
            ChapterId::ReturnSignal if run.checkpoint == 0 => self.spawn_campaign_enemy(false)?,
            ChapterId::ReturnSignal => {}
            ChapterId::MeridianReadings => {
                self.meridian = Some(Expedition::restore(meridian_checkpoint(run.checkpoint)?)?);
                if self.participants[0]
                    .campaign_entry
                    .as_ref()
                    .is_some_and(|r| r.mode != CampaignEntryMode::Resume)
                {
                    self.campaign_event(
                        ReplayEventKind::FieldActivity,
                        Some(self.participants[0].actor.id),
                        "meridian-v1:started",
                    )?;
                }
                self.broadcast_campaign_meridian_objectives();
            }
            ChapterId::BrokenSupplyLine => {
                for message in activity_messages(self.activity.objective_snapshot()).0 {
                    self.broadcast(message);
                }
                if run.checkpoint == 1 {
                    self.spawn_signal_enemy(self.participants[0].actor.id)?;
                }
            }
            ChapterId::CounterSignal => {
                for message in activity_messages(self.activity.objective_snapshot()).0 {
                    self.broadcast(message);
                }
                self.start_counter_encounter(run.checkpoint)?;
            }
            ChapterId::TheBreach => {
                self.broadcast_breach_door(run.checkpoint > 0);
                for message in activity_messages(self.activity.objective_snapshot()).0 {
                    self.broadcast(message);
                }
                if run.checkpoint == 1 || (run.checkpoint == 2 && !self.direct_breach()) {
                    self.spawn_campaign_enemy(true)?;
                }
            }
            ChapterId::PrismCore => {
                self.broadcast_breach_door(true);
                for message in activity_messages(self.activity.objective_snapshot()).0 {
                    self.broadcast(message);
                }
                if run.checkpoint == 1 {
                    self.spawn_prism_encounter(self.participants[0].actor.id)?;
                }
            }
        }
        Ok(())
    }

    fn restore_authored_campaign_objectives(
        &mut self,
        run: &revenant_campaign::ChapterRun,
    ) -> Result<()> {
        for milestone in self
            .campaign
            .as_ref()
            .expect("campaign exists")
            .milestones(run.chapter)
            .iter()
            .take(usize::from(run.checkpoint))
        {
            let trigger = match run.chapter {
                ChapterId::BrokenSupplyLine => super::campaign_supply::trigger(*milestone)?,
                ChapterId::CounterSignal => super::campaign_counter::trigger(*milestone)?,
                ChapterId::TheBreach => super::campaign_breach::trigger(*milestone)?,
                ChapterId::PrismCore => super::campaign_prism::trigger(*milestone)?,
                _ => return Ok(()),
            };
            self.activity.apply_trigger(&trigger);
        }
        Ok(())
    }

    pub(super) fn spawn_campaign_enemy(&mut self, boss: bool) -> Result<()> {
        let (archetype, position, health, kind, pressure) = if boss {
            (
                "warden",
                [8, 0, 0],
                WARDEN_BASE_HEALTH,
                ReplayEventKind::BossSpawned,
                WARDEN_PRESSURE_PROFILE,
            )
        } else {
            (
                "relay-drone",
                [4, 0, 2],
                RELAY_DRONE_BASE_HEALTH,
                ReplayEventKind::EnemySpawned,
                RELAY_DRONE_PRESSURE_PROFILE,
            )
        };
        let enemy = self
            .actors
            .spawn(ActorKind::Enemy, archetype, position, health);
        let prefix = if boss { "boss" } else { "enemy" };
        self.campaign_event(
            kind,
            Some(enemy.id),
            &format!("{prefix} spawned: {archetype}"),
        )?;
        self.enemy_id = Some(enemy.id);
        self.enemy_ai = Some(AiController::new(pressure));
        self.stage = if boss {
            SessionStage::Boss
        } else {
            SessionStage::Drone
        };
        self.combat = super::CombatRuntime::default();
        self.combat_started = super::Instant::now();
        self.broadcast(actor_spawn_message(&enemy));
        self.activate_enemy_ai(enemy.id, self.participants[0].actor.id)
    }

    pub(super) fn confirm_campaign(
        &mut self,
        milestone: Milestone,
        proof: i64,
    ) -> Result<CampaignReceipt> {
        let state = self
            .campaign
            .as_ref()
            .ok_or_else(|| io::Error::other("campaign state missing"))?;
        let run = state
            .active
            .as_ref()
            .ok_or_else(|| io::Error::other("campaign run missing"))?;
        let player = &self.participants[0];
        let receipt = self.persistence.apply_campaign_command(
            &player.character_id,
            &CampaignRequest {
                operation_id: &format!("checkpoint-{proof}"),
                expected_revision: state.revision,
                command: Command::Confirm {
                    run_id: run.run_id.clone(),
                    milestone,
                },
                proof_event_id: Some(proof),
            },
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: self.activity.id(),
                actor_id: i64::try_from(player.actor.id)?,
            },
        )?;
        self.campaign = Some(receipt.state.clone());
        Ok(receipt)
    }

    pub(super) fn publish_campaign_receipt(&mut self, receipt: &CampaignReceipt) -> Result<()> {
        if let Some(rewards) = &receipt.rewards {
            self.participants[0].module_state.fragments = u32::try_from(rewards.item_quantity)?;
            self.broadcast(ServerMessage::LootGranted(LootGranted {
                activity_id: self.activity.id().to_owned(),
                item_id: RELAY_CORE_FRAGMENT.to_owned(),
                quantity: revenant_campaign::FIRST_CLEAR_FRAGMENTS,
                resulting_quantity: u32::try_from(rewards.item_quantity)?,
            }));
            let experience = u64::try_from(rewards.experience)?;
            self.broadcast(ServerMessage::ProgressionGranted(ProgressionGranted {
                activity_id: self.activity.id().to_owned(),
                experience_granted: u64::try_from(rewards.experience_granted)?,
                experience,
                previous_level: u32::try_from(rewards.previous_level)?,
                level: u32::try_from(rewards.level)?,
                experience_to_next_level: experience_to_next_level(experience),
            }));
        }
        if receipt.state.active.is_none() {
            self.stage = SessionStage::Complete;
            self.enemy_id = None;
            self.enemy_ai = None;
            self.broadcast(ServerMessage::ActivityComplete(ActivityComplete {
                activity_id: self.activity.id().to_owned(),
            }));
        }
        self.broadcast(ServerMessage::CampaignSnapshot(snapshot(&receipt.state)));
        Ok(())
    }

    pub(super) fn campaign_enemy_defeated(
        &mut self,
        target_id: u64,
        fatal_damage: ServerMessage,
    ) -> Result<()> {
        if self
            .campaign
            .as_ref()
            .and_then(|s| s.active.as_ref())
            .is_some_and(|r| r.chapter == ChapterId::BrokenSupplyLine)
        {
            return self.campaign_supply_guard_defeated(target_id, fatal_damage);
        }
        if self
            .campaign
            .as_ref()
            .and_then(|s| s.active.as_ref())
            .is_some_and(|r| r.chapter == ChapterId::TheBreach)
        {
            return self.campaign_breach_guard_defeated(target_id, fatal_damage);
        }
        let defeated = self
            .actors
            .get(target_id)
            .cloned()
            .ok_or_else(|| io::Error::other("campaign enemy missing"))?;
        let drone = self.stage == SessionStage::Drone;
        let proof = self.campaign_event(
            ReplayEventKind::EnemyDied,
            Some(target_id),
            &format!("enemy died: {}", defeated.archetype),
        )?;
        let receipt = self.confirm_campaign(
            if drone {
                Milestone::RelayGuardCleared
            } else {
                Milestone::ReturnSignalRecovered
            },
            proof,
        )?;
        let messages = activity_messages(self.activity.apply_trigger(
            &super::WorldTrigger::ActorGroupDead {
                group_id: if drone { "relay_drones" } else { "warden" }.to_owned(),
            },
        ))
        .0;
        self.stage = if drone {
            SessionStage::Door
        } else {
            SessionStage::Complete
        };
        self.enemy_id = None;
        self.enemy_ai = None;
        self.actors.destroy(target_id);
        self.broadcast(fatal_damage);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: target_id,
        }));
        for message in messages {
            if !matches!(message, ServerMessage::ActivityComplete(_)) {
                self.broadcast(message);
            }
        }
        self.publish_campaign_receipt(&receipt)
    }

    pub(super) fn move_campaign(&mut self, player_id: u64, position: [i32; 3]) -> Result<()> {
        if matches!(self.stage, SessionStage::Complete | SessionStage::Failed) {
            return Ok(());
        }
        let player = self
            .actors
            .get(player_id)
            .ok_or_else(|| io::Error::other("campaign player missing"))?;
        if player.health == 0 {
            return Ok(());
        }
        let run = self
            .campaign
            .as_ref()
            .and_then(|s| s.active.clone())
            .ok_or_else(|| io::Error::other("campaign run missing"))?;
        if run.chapter == ChapterId::MeridianReadings {
            if !revenant_activities::meridian::walkable(position)
                || !revenant_activities::meridian::valid_step(player.position, position)
            {
                return Ok(());
            }
            self.visit_campaign_meridian(position, run.checkpoint)?;
        } else if run.chapter == ChapterId::BrokenSupplyLine {
            if !valid_movement_target(position)
                || !revenant_activities::meridian::valid_step(player.position, position)
                || revenant_ai::sentinel::cover_blocks_segment(player.position, position)
            {
                return Ok(());
            }
            self.visit_campaign_supply(position, run.checkpoint)?;
        } else if run.chapter == ChapterId::CounterSignal {
            if !valid_movement_target(position)
                || !revenant_activities::meridian::valid_step(player.position, position)
                || !self.counter_movement_allowed(position)
            {
                return Ok(());
            }
            self.visit_campaign_counter(position, run.checkpoint)?;
        } else if run.chapter == ChapterId::TheBreach {
            if !valid_movement_target(position)
                || !revenant_activities::meridian::valid_step(player.position, position)
                || (run.checkpoint < 3 && position[0] > 8)
            {
                return Ok(());
            }
            self.visit_campaign_breach(position, run.checkpoint)?;
        } else if run.chapter == ChapterId::PrismCore {
            if !valid_movement_target(position)
                || !revenant_activities::meridian::valid_step(player.position, position)
                || (run.checkpoint == 1 && !revenant_ai::prism::arena_contains(position))
            {
                return Ok(());
            }
            self.visit_campaign_prism(position, run.checkpoint)?;
        } else if !valid_movement_target(position) {
            return Ok(());
        }
        self.actors.update_position(player_id, position);
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: player_id,
            position,
            charge: None,
            defense: None,
        }));
        if run.chapter == ChapterId::ReturnSignal
            && self.stage == SessionStage::Door
            && position == [6, 0, 0]
        {
            let messages = activity_messages(self.activity.apply_trigger(
                &super::WorldTrigger::AreaReached {
                    area_id: "relay_door".to_owned(),
                },
            ))
            .0;
            for message in messages {
                self.broadcast(message);
            }
            self.spawn_campaign_enemy(true)?;
        }
        Ok(())
    }

    fn visit_campaign_meridian(&mut self, position: [i32; 3], checkpoint: u8) -> Result<()> {
        // Campaign recovery starts only after linking the survey. Standalone
        // Meridian keeps its independent optional activities unchanged.
        if checkpoint < 2 && position == [-30, 0, 0] {
            return Ok(());
        }
        let mut proposed = self
            .meridian
            .as_ref()
            .ok_or_else(|| io::Error::other("campaign expedition missing"))?
            .clone();
        let Some((payload, _)) = proposed.visit(position) else {
            return Ok(());
        };
        let proof = self.campaign_event(
            ReplayEventKind::FieldActivity,
            Some(self.participants[0].actor.id),
            &payload,
        )?;
        let milestone = match payload.as_str() {
            "meridian-v1:meridian_arrival" => Some(Milestone::MeridianReached),
            "meridian-v1:meridian_gallery" => Some(Milestone::SurveyLinked),
            "meridian-v1:meridian_return" => Some(Milestone::RecoveryDelivered),
            _ => None,
        };
        let receipt = milestone
            .map(|milestone| self.confirm_campaign(milestone, proof))
            .transpose()?;
        self.meridian = Some(proposed);
        self.broadcast_campaign_meridian_objectives();
        if let Some(receipt) = receipt {
            if receipt.state.active.is_none() {
                for message in activity_messages(self.activity.apply_trigger(
                    &super::WorldTrigger::AreaReached {
                        area_id: "campaign_meridian".to_owned(),
                    },
                ))
                .0
                {
                    if !matches!(message, ServerMessage::ActivityComplete(_)) {
                        self.broadcast(message);
                    }
                }
            }
            self.publish_campaign_receipt(&receipt)?;
        }
        Ok(())
    }

    fn broadcast_campaign_meridian_objectives(&mut self) {
        let Some(expedition) = &self.meridian else {
            return;
        };
        let recovery_locked = self
            .campaign
            .as_ref()
            .and_then(|s| s.active.as_ref())
            .is_some_and(|run| run.checkpoint < 2);
        let mut events = expedition.objectives();
        if recovery_locked {
            for event in &mut events {
                if let super::ActivityEvent::ObjectiveUpdated(objective) = event {
                    if objective.id == "meridian_log" {
                        objective.state = revenant_objectives::ObjectiveState::Pending;
                    }
                }
            }
        }
        for message in activity_messages(events).0 {
            self.broadcast(message);
        }
    }
}
