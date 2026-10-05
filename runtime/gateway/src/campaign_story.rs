use revenant_campaign::{story, Command};
use revenant_persistence::CampaignRequest;
use revenant_protocol::{
    CampaignStory, CampaignStoryAction, CampaignStoryIntent, CampaignStoryResult,
};

use super::campaign::Result;
use super::{ModuleMutationReplayContext, ServerMessage, SessionStage, SharedSession};

pub(super) fn snapshot(state: &story::StoryState) -> CampaignStory {
    let arc = |value| {
        match value {
            story::ArcProgress::Unseen => "unseen",
            story::ArcProgress::Found => "found",
            story::ArcProgress::Returned => "returned",
        }
        .to_owned()
    };
    CampaignStory {
        revision: story::REVISION.to_owned(),
        empty_seat: arc(state.empty_seat),
        held_connection: arc(state.held_connection),
        supply: state.supply.map(|value| {
            match value {
                story::SupplyApproach::Covered => "covered",
                story::SupplyApproach::Service => "service",
            }
            .to_owned()
        }),
        core: state.core.map(|value| {
            match value {
                story::CoreApproach::Grounded => "grounded",
                story::CoreApproach::Direct => "direct",
            }
            .to_owned()
        }),
        epilogue: match state.epilogue() {
            story::Epilogue::SignalSilent => "signal_silent",
            story::Epilogue::RouteLit => "route_lit",
            story::Epilogue::OpenPassage => "open_passage",
        }
        .to_owned(),
    }
}

fn action(value: CampaignStoryAction) -> story::Command {
    use story::{Command as C, Discovery as D};
    use CampaignStoryAction as A;
    match value {
        A::EmptySeatMemory => C::Discover {
            discovery: D::EmptySeatMemory,
        },
        A::EmptySeatReturned => C::Discover {
            discovery: D::EmptySeatReturned,
        },
        A::HeldConnectionNote => C::Discover {
            discovery: D::HeldConnectionNote,
        },
        A::HeldConnectionReturned => C::Discover {
            discovery: D::HeldConnectionReturned,
        },
        A::SupplyCovered => C::ChooseSupply {
            approach: story::SupplyApproach::Covered,
        },
        A::SupplyService => C::ChooseSupply {
            approach: story::SupplyApproach::Service,
        },
        A::CoreGrounded => C::ChooseCore {
            approach: story::CoreApproach::Grounded,
        },
        A::CoreDirect => C::ChooseCore {
            approach: story::CoreApproach::Direct,
        },
    }
}

impl SharedSession {
    pub(super) fn interact_campaign_story(
        &mut self,
        player_id: u64,
        intent: &CampaignStoryIntent,
    ) -> Result<()> {
        let index = self.module_participant_index(player_id)?;
        let status = self.save_campaign_story(index, intent);
        let _ = self.participants[index]
            .outbound
            .send(ServerMessage::CampaignStoryResult(CampaignStoryResult {
                operation_id: intent.operation_id.clone(),
                status: status.to_owned(),
            }));
        if let Some(state) = &self.campaign {
            let _ = self.participants[index]
                .outbound
                .send(ServerMessage::CampaignSnapshot(super::campaign::snapshot(
                    state,
                )));
        }
        Ok(())
    }

    fn save_campaign_story(&mut self, index: usize, intent: &CampaignStoryIntent) -> &'static str {
        let player = &self.participants[index];
        let Some(state) = &self.campaign else {
            return "rejected";
        };
        let Some(actor) = self.actors.get(player.actor.id) else {
            return "rejected";
        };
        if self.expected_players != 1
            || self.participants.len() != 1
            || player.campaign_entry.is_none()
            || player.protocol_generation != super::ProtocolGeneration::CurrentV2
            || matches!(self.stage, SessionStage::Complete | SessionStage::Failed)
            || actor.health == 0
            || intent.operation_id.is_empty()
            || intent.operation_id.len() > 64
            || !intent
                .operation_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return "rejected";
        }
        let command = Command::Story {
            run_id: intent.run_id.clone(),
            position: actor.position,
            action: action(intent.action),
        };
        // Validate the present physical boundary even for a retry. Persistence
        // resolves the original operation before checking expected_revision.
        if state.propose(state.revision, command.clone()).is_err() {
            return "rejected";
        }
        let Ok(actor_id) = i64::try_from(player.actor.id) else {
            return "rejected";
        };
        let result = self.persistence.apply_campaign_command(
            &player.character_id,
            &CampaignRequest {
                operation_id: &format!("story-{}", intent.operation_id),
                expected_revision: intent.expected_revision,
                command,
                proof_event_id: None,
            },
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: self.activity.id(),
                actor_id,
            },
        );
        match result {
            Ok(receipt) => {
                self.campaign = Some(receipt.state);
                "accepted"
            }
            Err(_) => "unconfirmed",
        }
    }
}
