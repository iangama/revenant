//! Chapter three stages the west obstacle and an untimed delivery. It does not
//! enter the optional signal mission or the timed Coolant activity.
use std::io;

use revenant_campaign::{Milestone, SUPPLY_APPROACH, SUPPLY_CELL, SUPPLY_DELIVERY};

use super::campaign::Result;
use super::{
    activity_messages, ActorDestroy, ReplayEventKind, ServerMessage, SharedSession, WorldTrigger,
};

pub(super) fn spawn_position(checkpoint: u8) -> Result<[i32; 3]> {
    match checkpoint {
        0 => Ok([0, 0, 0]),
        1 => Ok(SUPPLY_APPROACH),
        2 => Ok([-7, 0, -3]),
        3 => Ok(SUPPLY_CELL),
        _ => Err(io::Error::other("unsupported supply checkpoint").into()),
    }
}

pub(super) fn trigger(milestone: Milestone) -> Result<WorldTrigger> {
    Ok(match milestone {
        Milestone::SupplyRouteReached => WorldTrigger::AreaReached {
            area_id: "supply_route".to_owned(),
        },
        Milestone::SupplyGuardCleared => WorldTrigger::ActorGroupDead {
            group_id: "supply_guard".to_owned(),
        },
        Milestone::SupplyCellRecovered => WorldTrigger::AreaReached {
            area_id: "supply_cell".to_owned(),
        },
        Milestone::SupplyDelivered => WorldTrigger::AreaReached {
            area_id: "supply_delivery".to_owned(),
        },
        _ => return Err(io::Error::other("unsupported supply milestone").into()),
    })
}

impl SharedSession {
    fn broadcast_supply_trigger(&mut self, milestone: Milestone) -> Result<()> {
        for message in activity_messages(self.activity.apply_trigger(&trigger(milestone)?)).0 {
            if !matches!(message, ServerMessage::ActivityComplete(_)) {
                self.broadcast(message);
            }
        }
        Ok(())
    }

    pub(super) fn visit_campaign_supply(
        &mut self,
        position: [i32; 3],
        checkpoint: u8,
    ) -> Result<()> {
        let service = self.campaign.as_ref().is_some_and(|s| {
            s.story.supply_approach() == revenant_campaign::story::SupplyApproach::Service
        });
        let (milestone, payload) = match (checkpoint, position) {
            (0, SUPPLY_APPROACH) if !service => {
                (Milestone::SupplyRouteReached, "supply-v1:route:-4,0,4")
            }
            (0, revenant_campaign::story::SERVICE_APPROACH) if service => (
                Milestone::SupplyRouteReached,
                revenant_campaign::story::SERVICE_OBJECTIVE,
            ),
            (2, SUPPLY_CELL) => (Milestone::SupplyCellRecovered, "supply-v1:cell:-4,0,-3"),
            (3, SUPPLY_DELIVERY) => (Milestone::SupplyDelivered, "supply-v1:delivery:-1,0,-5"),
            _ => return Ok(()),
        };
        let proof = self.campaign_event(
            ReplayEventKind::CampaignObjective,
            Some(self.participants[0].actor.id),
            payload,
        )?;
        let receipt = self.confirm_campaign(milestone, proof)?;
        self.broadcast_supply_trigger(milestone)?;
        self.publish_campaign_receipt(&receipt)?;
        if milestone == Milestone::SupplyRouteReached {
            self.spawn_signal_enemy(self.participants[0].actor.id)?;
        }
        Ok(())
    }

    pub(super) fn campaign_supply_guard_defeated(
        &mut self,
        target_id: u64,
        fatal_damage: ServerMessage,
    ) -> Result<()> {
        if !self.signal_fighting() || self.enemy_id != Some(target_id) {
            return Err(io::Error::other("supply guard is not active").into());
        }
        let proof = self.campaign_event(
            ReplayEventKind::EnemyDied,
            Some(target_id),
            "enemy died: signal-sentinel",
        )?;
        let receipt = self.confirm_campaign(Milestone::SupplyGuardCleared, proof)?;
        self.signal = None;
        self.enemy_id = None;
        self.actors.destroy(target_id);
        self.broadcast(fatal_damage);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: target_id,
        }));
        self.broadcast_supply_trigger(Milestone::SupplyGuardCleared)?;
        self.publish_campaign_receipt(&receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revenant_activities::{ActivityEvent, ScriptedActivity};
    use revenant_campaign::ChapterId;
    use revenant_objectives::ObjectiveState;

    #[test]
    fn supply_resume_projects_completed_objectives_and_one_active_step() {
        for checkpoint in 0..4 {
            let mut activity = ScriptedActivity::from_source(include_str!(
                "../../../scripts/activities/campaign_supply.lua"
            ))
            .unwrap();
            for milestone in ChapterId::BrokenSupplyLine
                .milestones()
                .iter()
                .take(checkpoint)
            {
                activity.apply_trigger(&trigger(*milestone).unwrap());
            }
            let states = activity
                .objective_snapshot()
                .into_iter()
                .filter_map(|event| match event {
                    ActivityEvent::ObjectiveUpdated(objective) => Some(objective.state),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(states.len(), 4);
            assert_eq!(
                states
                    .iter()
                    .filter(|s| **s == ObjectiveState::Completed)
                    .count(),
                checkpoint
            );
            assert_eq!(states[checkpoint], ObjectiveState::Active);
            assert!(states[checkpoint + 1..]
                .iter()
                .all(|s| *s == ObjectiveState::Pending));
        }
    }
}
