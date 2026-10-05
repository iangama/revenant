//! Final campaign boundaries surround the retained two-phase Prism encounter.
use std::io;

use revenant_campaign::{Milestone, PRISM_APPROACH, PRISM_RETURN, PRISM_SOURCE};

use super::campaign::Result;
use super::{activity_messages, ReplayEventKind, ServerMessage, SharedSession, WorldTrigger};

pub(super) fn spawn_position(checkpoint: u8) -> Result<[i32; 3]> {
    match checkpoint {
        0 => Ok(PRISM_RETURN),
        1 => Ok(PRISM_APPROACH),
        2 => Ok([7, 0, 0]),
        3 => Ok(PRISM_SOURCE),
        _ => Err(io::Error::other("unsupported Prism core checkpoint").into()),
    }
}

pub(super) fn trigger(milestone: Milestone) -> Result<WorldTrigger> {
    let area_id = match milestone {
        Milestone::PrismChamberReached => "prism_arrival",
        Milestone::PrismSourceStopped => "prism_shutdown",
        Milestone::PrismReturnCompleted => "prism_return",
        Milestone::PrismWardenCleared => {
            return Ok(WorldTrigger::ActorGroupDead {
                group_id: "prism_guard".to_owned(),
            })
        }
        _ => return Err(io::Error::other("unsupported Prism core milestone").into()),
    };
    Ok(WorldTrigger::AreaReached {
        area_id: area_id.to_owned(),
    })
}

impl SharedSession {
    pub(super) fn broadcast_campaign_prism_trigger(&mut self, milestone: Milestone) -> Result<()> {
        for message in activity_messages(self.activity.apply_trigger(&trigger(milestone)?)).0 {
            if !matches!(message, ServerMessage::ActivityComplete(_)) {
                self.broadcast(message);
            }
        }
        Ok(())
    }

    pub(super) fn visit_campaign_prism(
        &mut self,
        position: [i32; 3],
        checkpoint: u8,
    ) -> Result<()> {
        let milestone = match (checkpoint, position) {
            (0, PRISM_APPROACH) => Milestone::PrismChamberReached,
            (2, PRISM_SOURCE) => Milestone::PrismSourceStopped,
            (3, PRISM_RETURN) => Milestone::PrismReturnCompleted,
            _ => return Ok(()),
        };
        let proof = self.campaign_event(
            ReplayEventKind::CampaignObjective,
            Some(self.participants[0].actor.id),
            revenant_campaign::prism_objective(milestone).expect("authored Prism boundary"),
        )?;
        let receipt = self.confirm_campaign(milestone, proof)?;
        self.broadcast_campaign_prism_trigger(milestone)?;
        self.publish_campaign_receipt(&receipt)?;
        if milestone == Milestone::PrismChamberReached {
            self.spawn_prism_encounter(self.participants[0].actor.id)?;
        }
        Ok(())
    }
}
