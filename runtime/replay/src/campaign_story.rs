//! Spatial interaction evidence, written atomically with a v6 story transition.
use revenant_campaign::{story, CampaignState, ChapterId, Command};
use serde::{Deserialize, Serialize};

use crate::{invalid_module, ReplayError, ReplayEvent, ReplayEventKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignStoryProof {
    pub revision: String,
    pub character_id: String,
    pub state_revision: u64,
    pub run_id: String,
    pub chapter: ChapterId,
    pub checkpoint: u8,
    pub position: [i32; 3],
    pub action: story::Command,
}

impl CampaignStoryProof {
    /// Builds evidence from an admitted save and a server-validated interaction.
    ///
    /// # Errors
    /// Rejects invalid story commands, characters or campaign boundaries.
    pub fn new(
        character: &str,
        state: &CampaignState,
        command: &Command,
    ) -> Result<Self, ReplayError> {
        state
            .propose(state.revision, command.clone())
            .map_err(|e| invalid_module(e.to_string()))?;
        let Command::Story {
            run_id,
            position,
            action,
        } = command
        else {
            return Err(invalid_module("story proof requires a story command"));
        };
        let run = state
            .active
            .as_ref()
            .ok_or_else(|| invalid_module("story proof lacks a run"))?;
        if character.is_empty() || character.len() > 256 {
            return Err(invalid_module("invalid story character"));
        }
        Ok(Self {
            revision: story::REVISION.to_owned(),
            character_id: character.to_owned(),
            state_revision: state.revision,
            run_id: run_id.clone(),
            chapter: run.chapter,
            checkpoint: run.checkpoint,
            position: *position,
            action: *action,
        })
    }

    /// # Errors
    /// Rejects unsupported proof encodings.
    pub fn encode(&self) -> Result<String, ReplayError> {
        serde_json::to_string(self).map_err(|e| invalid_module(e.to_string()))
    }
}

pub(super) fn verify(
    events: &[ReplayEvent],
    checkpoint: &ReplayEvent,
    payload: &super::campaign::CampaignReplayPayload,
) -> Result<(), ReplayError> {
    let proof = events
        .iter()
        .find(|e| Some(e.id) == payload.proof_event_id)
        .ok_or_else(|| invalid_module("story interaction lacks spatial evidence"))?;
    let boundary = events
        .iter()
        .rev()
        .find(|e| {
            matches!(
                e.kind,
                ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
            )
        })
        .ok_or_else(|| invalid_module("story interaction lacks campaign admission"))?;
    let expected = CampaignStoryProof::new(
        &payload.character_id,
        &payload.transition.before,
        &payload.transition.command,
    )?;
    let actual: CampaignStoryProof =
        serde_json::from_str(&proof.payload).map_err(|e| invalid_module(e.to_string()))?;
    if proof.kind != ReplayEventKind::CampaignStory
        || proof.id <= boundary.id
        || proof.actor_id != checkpoint.actor_id
        || proof.account_id != checkpoint.account_id
        || proof.activity_id != checkpoint.activity_id
        || proof.session_id != checkpoint.session_id
        || actual != expected
    {
        return Err(invalid_module(
            "story proof differs from the admitted interaction",
        ));
    }
    Ok(())
}

pub(super) fn validate_proofs(events: &[ReplayEvent]) -> Result<(), ReplayError> {
    for (index, proof) in events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == ReplayEventKind::CampaignStory)
    {
        if proof.payload.len() > 2048 {
            return Err(invalid_module("story proof exceeds its bound"));
        }
        let checkpoint = events
            .get(index + 1)
            .filter(|e| e.kind == ReplayEventKind::CampaignCheckpoint)
            .ok_or_else(|| invalid_module("story proof lacks its atomic checkpoint"))?;
        let payload = super::campaign::decode_campaign_payload(&checkpoint.payload)?;
        if payload.proof_event_id != Some(proof.id)
            || !matches!(payload.transition.command, Command::Story { .. })
        {
            return Err(invalid_module(
                "story proof does not belong to the next checkpoint",
            ));
        }
        verify(&events[..=index], checkpoint, &payload)?;
    }
    Ok(())
}
