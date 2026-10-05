//! The Warden pressures the relay approach; its powered shield prevents a skip.
use std::io;

use revenant_campaign::{ChapterId, Milestone, BREACH_CORE, BREACH_DOOR, BREACH_STABILIZER};

use super::campaign::Result;
use super::{
    activity_messages, ActorDestroy, ReplayEventKind, ServerMessage, SessionStage, SharedSession,
    WorldTrigger,
};

pub(super) fn spawn_position(checkpoint: u8) -> Result<[i32; 3]> {
    match checkpoint {
        0 => Ok([0, 0, 0]),
        1 => Ok(BREACH_DOOR),
        2 => Ok(BREACH_STABILIZER),
        3 => Ok([8, 0, 0]),
        _ => Err(io::Error::other("unsupported breach checkpoint").into()),
    }
}

pub(super) fn direct_spawn_position(checkpoint: u8) -> Result<[i32; 3]> {
    match checkpoint {
        2 => Ok([8, 0, 0]),
        3 => Ok(BREACH_STABILIZER),
        _ => spawn_position(checkpoint),
    }
}

pub(super) fn trigger(milestone: Milestone) -> Result<WorldTrigger> {
    let area_id = match milestone {
        Milestone::BreachDoorReached => "breach_door",
        Milestone::BreachCircuitStabilized => "breach_stabilizer",
        Milestone::BreachCoreReached => "breach_core",
        Milestone::BreachGuardCleared => {
            return Ok(WorldTrigger::ActorGroupDead {
                group_id: "breach_guard".to_owned(),
            })
        }
        _ => return Err(io::Error::other("unsupported breach milestone").into()),
    };
    Ok(WorldTrigger::AreaReached {
        area_id: area_id.to_owned(),
    })
}

impl SharedSession {
    pub(super) fn direct_breach(&self) -> bool {
        self.campaign.as_ref().is_some_and(|s| {
            s.story.core_approach() == revenant_campaign::story::CoreApproach::Direct
        })
    }
    pub(super) fn broadcast_breach_door(&mut self, open: bool) {
        self.broadcast(ServerMessage::DoorState(revenant_protocol::DoorState {
            door_id: "relay_core".to_owned(),
            open,
        }));
    }

    pub(super) fn breach_shield_active(&self) -> bool {
        !self.direct_breach()
            && self
                .campaign
                .as_ref()
                .and_then(|s| s.active.as_ref())
                .is_some_and(|r| r.chapter == ChapterId::TheBreach && r.checkpoint == 1)
    }

    fn broadcast_breach_trigger(&mut self, milestone: Milestone) -> Result<()> {
        for message in activity_messages(self.activity.apply_trigger(&trigger(milestone)?)).0 {
            if !matches!(message, ServerMessage::ActivityComplete(_)) {
                self.broadcast(message);
            }
        }
        Ok(())
    }

    pub(super) fn visit_campaign_breach(
        &mut self,
        position: [i32; 3],
        checkpoint: u8,
    ) -> Result<()> {
        let milestone = match (checkpoint, position) {
            (0, BREACH_DOOR) => Milestone::BreachDoorReached,
            (1, BREACH_STABILIZER) if !self.direct_breach() => Milestone::BreachCircuitStabilized,
            (2, BREACH_STABILIZER) if self.direct_breach() => Milestone::BreachCircuitStabilized,
            (3, BREACH_CORE) => Milestone::BreachCoreReached,
            _ => return Ok(()),
        };
        let proof = self.campaign_event(
            ReplayEventKind::CampaignObjective,
            Some(self.participants[0].actor.id),
            revenant_campaign::breach_objective(milestone).expect("authored spatial boundary"),
        )?;
        let receipt = self.confirm_campaign(milestone, proof)?;
        self.broadcast_breach_trigger(milestone)?;
        self.publish_campaign_receipt(&receipt)?;
        if milestone == Milestone::BreachDoorReached {
            self.broadcast_breach_door(true);
            self.spawn_campaign_enemy(true)?;
        }
        Ok(())
    }

    pub(super) fn campaign_breach_guard_defeated(
        &mut self,
        target_id: u64,
        fatal_damage: ServerMessage,
    ) -> Result<()> {
        if self.enemy_id != Some(target_id) || self.breach_shield_active() {
            return Err(io::Error::other("breach guard is not vulnerable").into());
        }
        let proof = self.campaign_event(
            ReplayEventKind::EnemyDied,
            Some(target_id),
            "enemy died: warden",
        )?;
        let receipt = self.confirm_campaign(Milestone::BreachGuardCleared, proof)?;
        self.enemy_id = None;
        self.enemy_ai = None;
        self.stage = SessionStage::Door;
        self.actors.destroy(target_id);
        self.broadcast(fatal_damage);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: target_id,
        }));
        self.broadcast_breach_trigger(Milestone::BreachGuardCleared)?;
        self.publish_campaign_receipt(&receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revenant_activities::{ActivityEvent, ScriptedActivity};
    use revenant_objectives::ObjectiveState;

    #[test]
    fn breach_restores_powered_drained_and_cleared_boundaries() {
        for checkpoint in 0..4 {
            let mut activity = ScriptedActivity::from_source(include_str!(
                "../../../scripts/activities/campaign_breach.lua"
            ))
            .unwrap();
            for milestone in ChapterId::TheBreach.milestones().iter().take(checkpoint) {
                activity.apply_trigger(&trigger(*milestone).unwrap());
            }
            let states = activity
                .objective_snapshot()
                .into_iter()
                .filter_map(|e| match e {
                    ActivityEvent::ObjectiveUpdated(o) => Some(o.state),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(states.len(), 4);
            assert!(states[..checkpoint]
                .iter()
                .all(|s| *s == ObjectiveState::Completed));
            assert_eq!(states[checkpoint], ObjectiveState::Active);
            assert!(states[checkpoint + 1..]
                .iter()
                .all(|s| *s == ObjectiveState::Pending));
        }
    }
}
