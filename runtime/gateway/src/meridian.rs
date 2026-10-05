use std::error::Error;

use revenant_activities::{
    meridian::{Expedition, ENTRANCE, REVISION},
    ActivityEvent,
};
use revenant_operations::RoutePhase;

use super::{activity_messages, ReplayEventKind, SessionStage, SharedSession};

impl SharedSession {
    pub(super) fn meridian_traversal_available(&self) -> bool {
        self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].exploration_capable
            && self.signal.is_none()
            && self.coolant.is_none()
            && self.cooperation.is_none()
            && (self.meridian.is_some()
                || self
                    .route
                    .as_ref()
                    .is_some_and(|route| route.operation.phase() == RoutePhase::ChoiceOpen))
    }

    pub(super) fn handle_meridian_movement(
        &mut self,
        position: [i32; 3],
    ) -> Result<(), Box<dyn Error>> {
        if !self.meridian_traversal_available() {
            return Ok(());
        }
        let owner = self.owner_account()?.to_owned();
        let player_id = self.participants[0].actor.id;
        if self.meridian.is_none() {
            // Enter deliberately through the marked west threshold, never by teleport.
            if position != ENTRANCE {
                return Ok(());
            }
            let expedition = Expedition::start()?;
            self.append_event(
                ReplayEventKind::FieldActivity,
                &owner,
                Some(player_id),
                &format!("{REVISION}:started"),
            )?;
            self.route
                .as_mut()
                .expect("eligible route exists")
                .operation
                .lock_baseline()?;
            let events = expedition.objectives();
            self.meridian = Some(expedition);
            self.broadcast_meridian_objectives(events);
            self.broadcast_route_state("Meridian annex selected; standard mission rewards")?;
            return Ok(());
        }
        let mut proposed = self.meridian.as_ref().expect("expedition exists").clone();
        if let Some((payload, events)) = proposed.visit(position) {
            self.append_event(
                ReplayEventKind::FieldActivity,
                &owner,
                Some(player_id),
                &payload,
            )?;
            self.meridian = Some(proposed);
            self.broadcast_meridian_objectives(events);
        }
        Ok(())
    }

    fn broadcast_meridian_objectives(&mut self, events: Vec<ActivityEvent>) {
        let (messages, _) = activity_messages(events);
        for message in messages {
            self.broadcast(message);
        }
    }
}
