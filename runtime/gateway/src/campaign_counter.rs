//! Counter-signal links two retained encounters to an investigation and isolation.
use std::io;

use revenant_ai::{bulwark, elite::EliteComposition, lancer};
use revenant_campaign::{
    Milestone, COUNTER_APPROACH, COUNTER_BASTION, COUNTER_EMITTER, COUNTER_TRANSMISSION,
};

use super::campaign::Result;
use super::{activity_messages, ReplayEventKind, ServerMessage, SharedSession, WorldTrigger};

pub(super) fn spawn_position(checkpoint: u8) -> Result<[i32; 3]> {
    match checkpoint {
        0 => Ok([0, 0, 0]),
        1 | 2 => Ok(COUNTER_APPROACH),
        3 => Ok(COUNTER_TRANSMISSION),
        4 | 5 => Ok(COUNTER_BASTION),
        _ => Err(io::Error::other("unsupported counter-signal checkpoint").into()),
    }
}

pub(super) fn trigger(milestone: Milestone) -> Result<WorldTrigger> {
    let (id, combat) = match milestone {
        Milestone::CounterApproachReached => ("counter_approach", false),
        Milestone::CounterSupportCleared => ("counter_support", true),
        Milestone::CounterSignalDecoded => ("counter_transmission", false),
        Milestone::CounterBastionReached => ("counter_bastion", false),
        Milestone::CounterBastionCleared => ("counter_guard", true),
        Milestone::CounterSignalIsolated => ("counter_emitter", false),
        _ => return Err(io::Error::other("unsupported counter-signal milestone").into()),
    };
    Ok(if combat {
        WorldTrigger::ActorGroupDead {
            group_id: id.to_owned(),
        }
    } else {
        WorldTrigger::AreaReached {
            area_id: id.to_owned(),
        }
    })
}

impl SharedSession {
    pub(super) fn start_counter_encounter(&mut self, checkpoint: u8) -> Result<()> {
        let player_id = self.participants[0].actor.id;
        match checkpoint {
            1 => self.spawn_support_encounter(player_id),
            4 => self.spawn_elite_encounter(player_id, EliteComposition::BastionLink),
            _ => Ok(()),
        }
    }

    pub(super) fn counter_movement_allowed(&self, position: [i32; 3]) -> bool {
        (!self.support_fighting() || lancer::arena_contains(position))
            && (!self.elite_fighting() || bulwark::arena_contains(position))
    }

    pub(super) fn visit_campaign_counter(
        &mut self,
        position: [i32; 3],
        checkpoint: u8,
    ) -> Result<()> {
        let milestone = match (checkpoint, position) {
            (0, COUNTER_APPROACH) => Milestone::CounterApproachReached,
            (2, COUNTER_TRANSMISSION) => Milestone::CounterSignalDecoded,
            (3, COUNTER_BASTION) => Milestone::CounterBastionReached,
            (5, COUNTER_EMITTER) => Milestone::CounterSignalIsolated,
            _ => return Ok(()),
        };
        self.complete_counter_boundary(milestone)?;
        self.start_counter_encounter(checkpoint + 1)
    }

    pub(super) fn complete_counter_boundary(&mut self, milestone: Milestone) -> Result<()> {
        let proof = self.campaign_event(
            ReplayEventKind::CampaignObjective,
            Some(self.participants[0].actor.id),
            revenant_campaign::counter_objective(milestone)
                .ok_or_else(|| io::Error::other("unsupported counter-signal proof"))?,
        )?;
        let receipt = self.confirm_campaign(milestone, proof)?;
        for message in activity_messages(self.activity.apply_trigger(&trigger(milestone)?)).0 {
            if !matches!(message, ServerMessage::ActivityComplete(_)) {
                self.broadcast(message);
            }
        }
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
    fn counter_resume_restores_all_six_objective_boundaries() {
        for checkpoint in 0..6 {
            let mut activity = ScriptedActivity::from_source(include_str!(
                "../../../scripts/activities/campaign_counter.lua"
            ))
            .unwrap();
            for milestone in ChapterId::CounterSignal
                .milestones()
                .iter()
                .take(checkpoint)
            {
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
            assert_eq!(states.len(), 6);
            assert!(states[..checkpoint]
                .iter()
                .all(|s| *s == ObjectiveState::Completed));
            assert_eq!(states[checkpoint], ObjectiveState::Active);
            assert!(states[checkpoint + 1..]
                .iter()
                .all(|s| *s == ObjectiveState::Pending));
        }
        assert_eq!(spawn_position(1).unwrap(), revenant_ai::support::ENTRANCE);
        assert_eq!(
            spawn_position(4).unwrap(),
            EliteComposition::BastionLink.entrance()
        );
    }
}
