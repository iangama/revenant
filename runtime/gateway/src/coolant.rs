use std::{error::Error, time::Instant};

use revenant_activities::{
    coolant::{CoolantRun, Phase, INTAKE},
    ActivityEvent,
};
use revenant_operations::RoutePhase;

use super::{activity_messages, ReplayEventKind, SessionStage, SharedSession};

pub(super) struct CoolantEncounter {
    player_id: u64,
    started_at: Instant,
    run: CoolantRun,
}

impl SharedSession {
    fn coolant_available(&self) -> bool {
        self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].content_capable
            && self.signal.is_none()
            && self.cooperation.is_none()
            && self
                .route
                .as_ref()
                .is_some_and(|route| route.operation.phase() == RoutePhase::ChoiceOpen)
    }

    pub(super) fn handle_coolant_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn Error>> {
        let retry = self.stage == SessionStage::Door
            && self
                .coolant
                .as_ref()
                .is_some_and(|encounter| encounter.run.phase() == Phase::Expired);
        if position == INTAKE && (retry || self.coolant_available()) {
            let (run, events) = CoolantRun::start()?;
            let owner = self.owner_account()?.to_owned();
            self.append_event(
                ReplayEventKind::FieldActivity,
                &owner,
                Some(player_id),
                "coolant_run:started",
            )?;
            if !retry {
                self.route
                    .as_mut()
                    .expect("eligible route exists")
                    .operation
                    .lock_baseline()?;
            }
            self.coolant = Some(CoolantEncounter {
                player_id,
                started_at: Instant::now(),
                run,
            });
            self.broadcast_coolant_events(events);
            self.broadcast_route_state("coolant run selected; standard mission rewards")?;
            return Ok(true);
        }
        self.tick_coolant()?;
        Ok(self.coolant.as_ref().is_some_and(|encounter| {
            matches!(encounter.run.phase(), Phase::Transfer | Phase::Delivery)
        }))
    }

    pub(super) fn tick_coolant(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(encounter) = &self.coolant else {
            return Ok(());
        };
        let elapsed = u64::try_from(encounter.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.tick_coolant_at(elapsed)
    }

    pub(super) fn tick_coolant_at(&mut self, elapsed: u64) -> Result<(), Box<dyn Error>> {
        let Some(encounter) = &self.coolant else {
            return Ok(());
        };
        let Some(player) = self.actors.get(encounter.player_id) else {
            return Ok(());
        };
        let mut proposed = encounter.run.clone();
        let events = proposed.update(player.position, elapsed);
        if proposed.phase() != encounter.run.phase() {
            let payload = match proposed.phase() {
                Phase::Delivery => "coolant_run:transfer",
                Phase::Completed => "coolant_run:completed",
                Phase::Expired => "coolant_run:expired",
                Phase::Transfer => unreachable!(),
            };
            let player_id = encounter.player_id;
            let owner = self.owner_account()?.to_owned();
            self.append_event(
                ReplayEventKind::FieldActivity,
                &owner,
                Some(player_id),
                payload,
            )?;
        }
        self.coolant.as_mut().expect("encounter exists").run = proposed;
        self.broadcast_coolant_events(events);
        Ok(())
    }

    fn broadcast_coolant_events(&mut self, events: Vec<ActivityEvent>) {
        // Field objectives never grant the enclosing mission's reward.
        let objectives = events
            .into_iter()
            .filter(|event| matches!(event, ActivityEvent::ObjectiveUpdated(_)))
            .collect();
        let (messages, _) = activity_messages(objectives);
        for message in messages {
            self.broadcast(message);
        }
    }
}
