use std::collections::{BTreeMap, BTreeSet};

use revenant_modules::acquisition::{ArcId, Milestone, MAX_FRAGMENTS, REVISION, REWARD_FRAGMENTS};
use revenant_operations::RouteId;
use serde::{Deserialize, Serialize};

use crate::{
    invalid_module, matching_participant, reconstruct, ReconstructedModuleParticipant,
    ReconstructedRouteState, ReconstructedSession, ReplayError, ReplayEvent, ReplayEventKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionMilestoneProof {
    pub milestone: Milestone,
    pub session_id: String,
    /// Last append included in the validated proof; later workshop events do
    /// not recursively become part of the commission's own evidence.
    pub through_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionClaimedPayloadV1 {
    pub schema_version: u8,
    pub revision: String,
    pub character_id: String,
    pub arc_id: ArcId,
    pub proofs: Vec<AcquisitionMilestoneProof>,
    pub previous_fragments: u32,
    pub granted_fragments: u32,
    pub resulting_fragments: u32,
}

/// Encodes one accepted commission grant, distinct from mission loot.
///
/// # Errors
/// Rejects invalid policy, proof identifiers or fragment arithmetic.
pub fn encode_acquisition_claimed(
    payload: &AcquisitionClaimedPayloadV1,
) -> Result<String, ReplayError> {
    validate_claim(payload)?;
    serde_json::to_string(payload).map_err(|e| invalid_module(e.to_string()))
}

/// Decodes the bounded, versioned commission grant evidence.
///
/// # Errors
/// Rejects malformed or inconsistent evidence.
pub fn decode_acquisition_claimed(
    payload: &str,
) -> Result<AcquisitionClaimedPayloadV1, ReplayError> {
    if payload.len() > 4096 {
        return Err(invalid_module(
            "acquisition claim payload exceeds its bound",
        ));
    }
    let payload = serde_json::from_str(payload).map_err(|e| invalid_module(e.to_string()))?;
    validate_claim(&payload)?;
    Ok(payload)
}

fn validate_claim(payload: &AcquisitionClaimedPayloadV1) -> Result<(), ReplayError> {
    let required: BTreeSet<_> = payload.arc_id.requirements().iter().copied().collect();
    let supplied: BTreeSet<_> = payload.proofs.iter().map(|p| p.milestone).collect();
    if payload.schema_version != 1
        || payload.revision != REVISION
        || payload.character_id.is_empty()
        || payload.character_id.len() > 256
        || supplied != required
        || supplied.len() != payload.proofs.len()
        || payload.proofs.iter().any(|p| {
            p.through_event_id <= 0
                || p.session_id.is_empty()
                || p.session_id.len() > 128
                || !p
                    .session_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        || payload.granted_fragments != REWARD_FRAGMENTS
        || payload.previous_fragments.checked_add(REWARD_FRAGMENTS)
            != Some(payload.resulting_fragments)
        || payload.resulting_fragments > MAX_FRAGMENTS
    {
        return Err(invalid_module(
            "invalid acquisition claim policy, proofs or balance",
        ));
    }
    Ok(())
}

pub(super) fn apply_claim(
    event: &ReplayEvent,
    session: &mut ReconstructedSession,
    participants: &mut BTreeMap<String, ReconstructedModuleParticipant>,
) -> Result<(), ReplayError> {
    let payload = decode_acquisition_claimed(&event.payload)?;
    if !session.completed
        || payload
            .proofs
            .iter()
            .any(|p| p.through_event_id >= event.id)
        || session
            .acquisition_claims
            .iter()
            .any(|p| p.character_id == payload.character_id && p.arc_id == payload.arc_id)
    {
        return Err(invalid_module(
            "acquisition claim is premature or duplicated",
        ));
    }
    let participant = matching_participant(event, &payload.character_id, participants)?;
    if participant.state.fragments != payload.previous_fragments {
        return Err(invalid_module(
            "acquisition balance differs from reconstructed inventory",
        ));
    }
    participant.state.fragments = payload.resulting_fragments;
    session.acquisition_claims.push(payload);
    Ok(())
}

/// Extracts commission progress from a validated completed session belonging to
/// the requested character. Client-supplied objectives are never accepted here.
///
/// # Errors
/// Rejects malformed or inconsistent authoritative replay evidence.
pub fn acquisition_milestones(
    events: &[ReplayEvent],
    character_id: &str,
) -> Result<Vec<Milestone>, ReplayError> {
    let state = reconstruct(events)?;
    let Some(participant) = state
        .module_participants
        .iter()
        .find(|p| p.character_id == character_id)
    else {
        return Ok(Vec::new());
    };
    if !state.completed
        || !events.iter().any(|e| {
            e.kind == ReplayEventKind::LootGranted && e.actor_id == Some(participant.actor_id)
        })
    {
        return Ok(Vec::new());
    }
    let owns_field = |prefix: &str| {
        events
            .iter()
            .filter(|e| e.kind == ReplayEventKind::FieldActivity && e.payload.starts_with(prefix))
            .all(|e| e.actor_id == Some(participant.actor_id))
    };
    let completed = |id: &str| {
        state
            .field_objectives
            .get(id)
            .is_some_and(|status| status == "Completed")
    };
    let mut milestones = Vec::new();
    if owns_field("meridian-v1:") && completed("meridian_gallery") && completed("meridian_return") {
        milestones.push(Milestone::MeridianRecovered);
    }
    if owns_field("prism-v1:") && completed("prism_warden") {
        milestones.push(Milestone::PrismCompleted);
    }
    if state.route.state == ReconstructedRouteState::Succeeded {
        if let Some(selection) = state.route.selection {
            if selection
                .participants
                .iter()
                .any(|p| p.character_id == character_id)
            {
                milestones.push(match selection.route_id {
                    RouteId::Breach => Milestone::BreachCompleted,
                    RouteId::Stabilize => Milestone::StabilizeCompleted,
                });
            }
        }
    }
    Ok(milestones)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encode_module_state_snapshot, module_state_evidence_for_catalog,
        ModuleStateSnapshotPayloadV1, ReplayProtocolGeneration, MODULE_REPLAY_SCHEMA_VERSION,
    };

    fn event(id: i64, kind: ReplayEventKind, payload: &str) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            timestamp: format!("2026-09-19 00:00:{id:02}+00"),
            session_id: "acquisition-proof".to_owned(),
            account_id: "account".to_owned(),
            activity_id: Some("relay_awakening".to_owned()),
            actor_id: Some(10),
            payload: payload.to_owned(),
        }
    }

    #[test]
    fn meridian_requires_both_paths_full_mission_and_the_actual_character() {
        let snapshot = encode_module_state_snapshot(&ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: "character".to_owned(),
            protocol_generation: ReplayProtocolGeneration::V2,
            state: module_state_evidence_for_catalog(
                "m35-v2",
                0,
                &[],
                &[],
                0,
                ReplayProtocolGeneration::V2,
            )
            .unwrap(),
        })
        .unwrap();
        let mut events = vec![
            event(1, ReplayEventKind::PlayerJoined, "player joined"),
            event(2, ReplayEventKind::ModuleStateSnapshot, &snapshot),
        ];
        for (i, payload) in [
            "started",
            "meridian_arrival",
            "meridian_lens",
            "meridian_gallery",
            "meridian_log",
            "meridian_return",
        ]
        .iter()
        .enumerate()
        {
            events.push(event(
                i64::try_from(i).unwrap() + 3,
                ReplayEventKind::FieldActivity,
                &format!("meridian-v1:{payload}"),
            ));
        }
        assert!(acquisition_milestones(&events, "character")
            .unwrap()
            .is_empty());
        events.push(event(
            9,
            ReplayEventKind::ActivityCompleted,
            "activity completed",
        ));
        assert!(acquisition_milestones(&events, "character")
            .unwrap()
            .is_empty());
        events.push(event(
            10,
            ReplayEventKind::LootGranted,
            "loot granted: relay_core_fragment x1 (total 1)",
        ));
        assert_eq!(
            acquisition_milestones(&events, "character").unwrap(),
            [Milestone::MeridianRecovered]
        );
        assert_claim_replay(&events);
        assert!(acquisition_milestones(&events, "another-character")
            .unwrap()
            .is_empty());
        let mut partial = events.clone();
        partial.remove(7); // Omit recovery completion; survey alone does not qualify.
        assert!(acquisition_milestones(&partial, "character")
            .unwrap()
            .is_empty());
        let mut foreign = events.clone();
        for e in &mut foreign {
            if e.kind == ReplayEventKind::FieldActivity {
                e.actor_id = Some(99);
            }
        }
        assert!(acquisition_milestones(&foreign, "character")
            .unwrap()
            .is_empty());
        events[3].payload = "meridian-v1:meridian_gallery".to_owned();
        assert!(acquisition_milestones(&events, "character").is_err());
    }

    fn assert_claim_replay(events: &[ReplayEvent]) {
        let claim = AcquisitionClaimedPayloadV1 {
            schema_version: 1,
            revision: REVISION.to_owned(),
            character_id: "character".to_owned(),
            arc_id: ArcId::Meridian,
            proofs: vec![AcquisitionMilestoneProof {
                milestone: Milestone::MeridianRecovered,
                session_id: "acquisition-proof".to_owned(),
                through_event_id: 10,
            }],
            previous_fragments: 1,
            granted_fragments: 2,
            resulting_fragments: 3,
        };
        let mut claimed = events.to_vec();
        claimed.push(event(
            11,
            ReplayEventKind::AcquisitionClaimed,
            &encode_acquisition_claimed(&claim).unwrap(),
        ));
        let state = reconstruct(&claimed).unwrap();
        assert_eq!(state.loot_grants, 1);
        assert_eq!(state.acquisition_claims.len(), 1);
        assert_eq!(state.module_participants[0].state.fragments, 3);
        let mut invalid = claim.clone();
        invalid.resulting_fragments = 4;
        assert!(encode_acquisition_claimed(&invalid).is_err());
        invalid = claim.clone();
        invalid.arc_id = ArcId::Routes;
        assert!(encode_acquisition_claimed(&invalid).is_err());
        invalid = claim.clone();
        invalid.previous_fragments = 2;
        invalid.resulting_fragments = 4;
        claimed[10].payload = encode_acquisition_claimed(&invalid).unwrap();
        assert!(reconstruct(&claimed).is_err());
        claimed[10].payload = encode_acquisition_claimed(&claim).unwrap();
        claimed[10].actor_id = Some(99);
        assert!(reconstruct(&claimed).is_err());
        claimed[10].actor_id = Some(10);
        claimed.push(event(
            12,
            ReplayEventKind::AcquisitionClaimed,
            &encode_acquisition_claimed(&claim).unwrap(),
        ));
        assert!(reconstruct(&claimed).is_err());
    }
}
