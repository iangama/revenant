use super::{
    arsenal_module_snapshot, ModuleMutationReplayContext, ServerMessage, SessionStage,
    SharedSession,
};
use revenant_modules::acquisition::{ArcId, ArcStatus, Grant, REVISION, REWARD_FRAGMENTS};
use revenant_persistence::PersistedAcquisitionState;
use revenant_protocol::{
    AcquisitionArc, AcquisitionClaimIntent, AcquisitionClaimed, AcquisitionRequirement,
    AcquisitionSnapshot,
};
use std::{error::Error, io};

pub(super) type AcquisitionStateResult = Result<PersistedAcquisitionState, Box<dyn Error>>;
pub(super) type AcquisitionMutationResult = Result<Result<Grant, String>, Box<dyn Error>>;

fn snapshot(
    state: &PersistedAcquisitionState,
    can_claim: bool,
) -> Result<AcquisitionSnapshot, Box<dyn Error>> {
    let domain = state.domain()?;
    Ok(AcquisitionSnapshot {
        revision: REVISION.to_owned(),
        can_claim,
        arcs: ArcId::ALL
            .into_iter()
            .map(|arc| AcquisitionArc {
                arc_id: arc.as_str().to_owned(),
                status: match domain.status(arc) {
                    ArcStatus::Locked => "locked",
                    ArcStatus::Ready => "ready",
                    ArcStatus::Claimed => "claimed",
                }
                .to_owned(),
                reward_fragments: REWARD_FRAGMENTS,
                requirements: arc
                    .requirements()
                    .iter()
                    .map(|m| AcquisitionRequirement {
                        milestone: m.as_str().to_owned(),
                        completed: state.proofs.iter().any(|p| p.milestone == *m),
                    })
                    .collect(),
            })
            .collect(),
    })
}

impl SharedSession {
    fn acquisition_participant(&self, player_id: u64) -> Result<usize, Box<dyn Error>> {
        if self.challenge.is_some() {
            return Err(io::Error::other("commissions are unavailable during a challenge").into());
        }
        let index = self.module_participant_index(player_id)?;
        if !self.participants[index].acquisition_capable {
            return Err(
                io::Error::other("acquisition requires negotiated m35-v3 capability").into(),
            );
        }
        Ok(index)
    }

    pub(super) fn send_acquisition_state(&mut self, player_id: u64) -> Result<(), Box<dyn Error>> {
        let index = self.acquisition_participant(player_id)?;
        let player = &self.participants[index];
        let state = self
            .persistence
            .refresh_acquisition_progress(&player.account_id, &player.character_id)?;
        let _ = player
            .outbound
            .send(ServerMessage::AcquisitionSnapshot(snapshot(
                &state,
                self.stage == SessionStage::Complete,
            )?));
        Ok(())
    }

    pub(super) fn claim_acquisition(
        &mut self,
        player_id: u64,
        intent: &AcquisitionClaimIntent,
    ) -> Result<(), Box<dyn Error>> {
        let index = self.acquisition_participant(player_id)?;
        let player = &self.participants[index];
        let character_id = player.character_id.clone();
        let account_id = player.account_id.clone();
        let arc = ArcId::ALL
            .into_iter()
            .find(|arc| arc.as_str() == intent.arc_id);
        let result = if self.stage != SessionStage::Complete {
            Err("commission claim is locked until activity completion".to_owned())
        } else if let Some(arc) = arc {
            // Refresh here too: a reconnect or missed state request must not
            // make the claim depend on the client's cached objectives.
            self.persistence
                .refresh_acquisition_progress(&account_id, &character_id)?;
            self.persistence.claim_acquisition_with_replay(
                &character_id,
                arc,
                ModuleMutationReplayContext {
                    catalog_revision: player.arsenal_catalog(),
                    session_id: &self.session_id,
                    account_id: &account_id,
                    activity_id: self.activity.id(),
                    actor_id: i64::try_from(player.actor.id)?,
                },
            )?
        } else {
            Err("unknown commission".to_owned())
        };
        // Project the durable balance, including a retry after spending.
        let state = self
            .persistence
            .refresh_acquisition_progress(&account_id, &character_id)?;
        self.participants[index].module_state = self.persistence.module_state_for(&character_id)?;
        let player = &self.participants[index];
        let (accepted, replayed, granted_fragments, message) = match result {
            Ok(grant) => (
                true,
                grant.replayed,
                grant.granted_fragments,
                if grant.replayed {
                    "commission already claimed"
                } else {
                    "commission claimed"
                }
                .to_owned(),
            ),
            Err(message) => (false, false, 0, message),
        };
        let _ = player
            .outbound
            .send(ServerMessage::AcquisitionClaimed(AcquisitionClaimed {
                accepted,
                replayed,
                message,
                arc_id: if arc.is_some() {
                    intent.arc_id.clone()
                } else {
                    String::new()
                },
                granted_fragments,
                state: snapshot(&state, self.stage == SessionStage::Complete)?,
                modules: arsenal_module_snapshot(
                    &player.module_state,
                    player.content_capable,
                    player.arsenal_catalog(),
                )?,
            }));
        Ok(())
    }
}
