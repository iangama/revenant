use std::collections::{BTreeMap, BTreeSet};

use revenant_campaign::{CampaignState, ChapterId, Command, Milestone, Transition};
use serde::{Deserialize, Serialize};

use crate::{
    invalid_module, ReconstructedModuleParticipant, ReplayError, ReplayEvent, ReplayEventKind,
    ReplayProtocolGeneration,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignReplayPayload {
    pub schema_version: u8,
    pub character_id: String,
    pub operation_id: String,
    /// The first checkpoint event carries its inherited before-state inside the
    /// transition. Persistence verifies that state against the character journal.
    pub transition: Transition,
    /// A confirm cites an earlier authoritative world event in this session.
    /// Entering a chapter records a choice and has no world-completion proof.
    pub proof_event_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignResumePayload {
    pub schema_version: u8,
    pub character_id: String,
    /// Exact previously committed checkpoint; persistence verifies this link.
    pub checkpoint_event_id: i64,
    pub state: CampaignState,
}

/// Encodes admission at a saved boundary without advancing progress.
///
/// # Errors
/// Rejects malformed state, unsupported schemas or a missing checkpoint identity.
pub fn encode_campaign_resume(payload: &CampaignResumePayload) -> Result<String, ReplayError> {
    validate_resume(payload)?;
    serde_json::to_string(payload).map_err(|e| invalid_module(e.to_string()))
}

/// # Errors
/// Rejects oversized or malformed saved-boundary evidence.
pub fn decode_campaign_resume(encoded: &str) -> Result<CampaignResumePayload, ReplayError> {
    if encoded.len() > 8192 {
        return Err(invalid_module("campaign resume exceeds its bound"));
    }
    let payload = serde_json::from_str(encoded).map_err(|e| invalid_module(e.to_string()))?;
    validate_resume(&payload)?;
    Ok(payload)
}

fn validate_resume(payload: &CampaignResumePayload) -> Result<(), ReplayError> {
    payload
        .state
        .validate()
        .map_err(|e| invalid_module(e.to_string()))?;
    if payload.schema_version != 1
        || payload.character_id.is_empty()
        || payload.character_id.len() > 256
        || payload.checkpoint_event_id <= 0
        || payload.state.active.is_none()
        || payload.state.revision == 0
    {
        return Err(invalid_module("invalid campaign resume identity or state"));
    }
    Ok(())
}

pub(super) fn resume_snapshot(
    events: &[ReplayEvent],
) -> Result<Option<(&ReplayEvent, CampaignResumePayload)>, ReplayError> {
    let mut admissions = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == ReplayEventKind::CampaignResumed);
    let Some((index, event)) = admissions.next() else {
        return Ok(None);
    };
    if admissions.next().is_some() {
        return Err(invalid_module("duplicate campaign admission"));
    }
    let payload = decode_campaign_resume(&event.payload)?;
    let chapter = payload
        .state
        .active
        .as_ref()
        .ok_or_else(|| invalid_module("resume has no active run"))?
        .chapter;
    let snapshots = events[..index]
        .iter()
        .filter(|e| e.kind == ReplayEventKind::ModuleStateSnapshot)
        .collect::<Vec<_>>();
    if snapshots.len() != 1
        || event.actor_id.is_none()
        || event.activity_id.as_deref() != Some(chapter.as_str())
        || payload.checkpoint_event_id >= event.id
        || events[..index].iter().any(|e| {
            !matches!(
                e.kind,
                ReplayEventKind::PlayerJoined
                    | ReplayEventKind::ModuleStateSnapshot
                    | ReplayEventKind::ActivityStarted
            )
        })
    {
        return Err(invalid_module(
            "campaign resume is not a fresh admitted session",
        ));
    }
    let snapshot = snapshots[0];
    let Some(crate::DecodedModuleReplayPayload::ModuleStateSnapshot(module)) =
        crate::decode_module_payload(snapshot.kind, &snapshot.payload)?
    else {
        return Err(invalid_module("campaign resume lacks module admission"));
    };
    if module.character_id != payload.character_id
        || module.protocol_generation != ReplayProtocolGeneration::V2
        || snapshot.actor_id != event.actor_id
        || snapshot.account_id != event.account_id
        || snapshot.activity_id != event.activity_id
    {
        return Err(invalid_module(
            "campaign resume belongs to another participant",
        ));
    }
    Ok(Some((event, payload)))
}

pub(super) fn restored_meridian(
    payload: &CampaignResumePayload,
) -> Result<Option<revenant_activities::meridian::Expedition>, ReplayError> {
    use revenant_activities::meridian::{Checkpoint, Expedition};
    let run = payload
        .state
        .active
        .as_ref()
        .ok_or_else(|| invalid_module("resume has no active run"))?;
    if run.chapter != ChapterId::MeridianReadings {
        return Ok(None);
    }
    let checkpoint = match run.checkpoint {
        0 => Checkpoint::Entrance,
        1 => Checkpoint::Arrival,
        2 => Checkpoint::SurveyComplete,
        _ => return Err(invalid_module("unsupported Meridian boundary")),
    };
    Expedition::restore(checkpoint)
        .map(Some)
        .map_err(|e| invalid_module(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedCampaign {
    pub character_id: String,
    pub state: CampaignState,
    pub checkpoints: usize,
    pub first_clears: usize,
}

/// Encodes a bounded, deterministic checkpoint transition.
///
/// # Errors
/// Rejects invalid identifiers, transitions or proof shape.
pub fn encode_campaign_payload(payload: &CampaignReplayPayload) -> Result<String, ReplayError> {
    validate_payload(payload)?;
    let encoded = serde_json::to_string(payload).map_err(|e| invalid_module(e.to_string()))?;
    if encoded.len() > 8192 {
        return Err(invalid_module("campaign payload exceeds its bound"));
    }
    Ok(encoded)
}

/// Decodes a checkpoint without accepting its embedded reward as authority.
///
/// # Errors
/// Rejects unsupported schemas, malformed JSON or invalid transition arithmetic.
pub fn decode_campaign_payload(payload: &str) -> Result<CampaignReplayPayload, ReplayError> {
    if payload.len() > 8192 {
        return Err(invalid_module("campaign payload exceeds its bound"));
    }
    let payload = serde_json::from_str(payload).map_err(|e| invalid_module(e.to_string()))?;
    validate_payload(&payload)?;
    Ok(payload)
}

fn validate_payload(payload: &CampaignReplayPayload) -> Result<(), ReplayError> {
    payload
        .transition
        .validate()
        .map_err(|e| invalid_module(e.to_string()))?;
    if payload.schema_version != 1
        || payload.character_id.is_empty()
        || payload.character_id.len() > 256
        || payload.operation_id.is_empty()
        || payload.operation_id.len() > 96
        || !payload
            .operation_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || match payload.transition.command {
            Command::Enter { .. } => payload.proof_event_id.is_some(),
            Command::Confirm { .. } | Command::Story { .. } => {
                payload.proof_event_id.is_none_or(|id| id <= 0)
            }
        }
    {
        return Err(invalid_module(
            "invalid campaign identity or proof reference",
        ));
    }
    Ok(())
}

/// Verifies one milestone against an immutable prefix of a solo session.
///
/// # Errors
/// Rejects foreign actors, missing prerequisites and mismatched world events.
pub fn verify_campaign_milestone(
    events: &[ReplayEvent],
    account_id: &str,
    actor_id: u64,
    milestone: Milestone,
    proof_event_id: i64,
) -> Result<(), ReplayError> {
    let proof = events
        .iter()
        .find(|e| e.id == proof_event_id)
        .ok_or_else(|| invalid_module("campaign proof event is absent"))?;
    if proof.account_id != account_id {
        return Err(invalid_module("campaign proof belongs to another account"));
    }
    let enemy_died = |archetype: &str| {
        proof.kind == ReplayEventKind::EnemyDied
            && proof.payload == format!("enemy died: {archetype}")
            && proof.actor_id.is_some_and(|enemy| {
                events.iter().any(|e| {
                    e.id < proof.id
                        && e.actor_id == Some(enemy)
                        && e.account_id == account_id
                        && ((e.kind == ReplayEventKind::EnemySpawned
                            && e.payload == format!("enemy spawned: {archetype}"))
                            || (e.kind == ReplayEventKind::BossSpawned
                                && e.payload == format!("boss spawned: {archetype}")))
                })
            })
    };
    let valid = match milestone {
        Milestone::RelayGuardCleared => enemy_died("relay-drone"),
        Milestone::ReturnSignalRecovered | Milestone::BreachGuardCleared => enemy_died("warden"),
        Milestone::SupplyGuardCleared => enemy_died("signal-sentinel"),
        Milestone::SupplyRouteReached
        | Milestone::SupplyCellRecovered
        | Milestone::SupplyDelivered
        | Milestone::BreachDoorReached
        | Milestone::BreachCircuitStabilized
        | Milestone::BreachCoreReached
        | Milestone::PrismChamberReached
        | Milestone::PrismSourceStopped
        | Milestone::PrismReturnCompleted => {
            proof.kind == ReplayEventKind::CampaignObjective
                && proof.actor_id == Some(actor_id)
                && objective_at_boundary(events, proof, milestone)?
                    .is_some_and(|expected| proof.payload == expected)
        }
        Milestone::PrismWardenCleared => {
            proof.kind == ReplayEventKind::CampaignObjective
                && proof.actor_id == Some(actor_id)
                && revenant_campaign::prism_objective(milestone) == Some(proof.payload.as_str())
                && verify_prism_encounter(events, proof)?
        }
        Milestone::CounterApproachReached
        | Milestone::CounterSupportCleared
        | Milestone::CounterSignalDecoded
        | Milestone::CounterBastionReached
        | Milestone::CounterBastionCleared
        | Milestone::CounterSignalIsolated => {
            proof.kind == ReplayEventKind::CampaignObjective
                && proof.actor_id == Some(actor_id)
                && revenant_campaign::counter_objective(milestone) == Some(proof.payload.as_str())
                && verify_counter_encounter(events, proof, milestone)?
        }
        Milestone::MeridianReached | Milestone::SurveyLinked | Milestone::RecoveryDelivered => {
            let expected = match milestone {
                Milestone::MeridianReached => "meridian_arrival",
                Milestone::SurveyLinked => "meridian_gallery",
                _ => "meridian_return",
            };
            let prefix = events
                .iter()
                .filter(|e| e.id <= proof.id)
                .cloned()
                .collect::<Vec<_>>();
            let objectives = crate::validate_field_activity_events(&prefix)?;
            proof.kind == ReplayEventKind::FieldActivity
                && proof.actor_id == Some(actor_id)
                && proof.payload == format!("meridian-v1:{expected}")
                && objectives
                    .get(expected)
                    .is_some_and(|status| status == "Completed")
                && prefix
                    .iter()
                    .filter(|e| e.kind == ReplayEventKind::FieldActivity)
                    .all(|e| e.account_id == account_id && e.actor_id == Some(actor_id))
        }
    };
    if !valid {
        return Err(invalid_module(
            "campaign milestone lacks matching world evidence",
        ));
    }
    Ok(())
}

fn authored_objective(milestone: Milestone) -> Option<&'static str> {
    match milestone {
        Milestone::SupplyRouteReached => Some("supply-v1:route:-4,0,4"),
        Milestone::SupplyCellRecovered => Some("supply-v1:cell:-4,0,-3"),
        Milestone::SupplyDelivered => Some("supply-v1:delivery:-1,0,-5"),
        _ => revenant_campaign::counter_objective(milestone)
            .or_else(|| revenant_campaign::breach_objective(milestone))
            .or_else(|| revenant_campaign::prism_objective(milestone)),
    }
}

fn authored_for_state(state: &CampaignState, milestone: Milestone) -> Option<&'static str> {
    if milestone == Milestone::SupplyRouteReached
        && state.story.supply_approach() == revenant_campaign::story::SupplyApproach::Service
    {
        Some(revenant_campaign::story::SERVICE_OBJECTIVE)
    } else {
        authored_objective(milestone)
    }
}

fn objective_at_boundary(
    events: &[ReplayEvent],
    proof: &ReplayEvent,
    milestone: Milestone,
) -> Result<Option<&'static str>, ReplayError> {
    if milestone != Milestone::SupplyRouteReached {
        return Ok(authored_objective(milestone));
    }
    let boundary = events
        .iter()
        .rev()
        .find(|e| {
            e.id < proof.id
                && matches!(
                    e.kind,
                    ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
                )
        })
        .ok_or_else(|| invalid_module("supply approach lacks its admitted choice"))?;
    let state = if boundary.kind == ReplayEventKind::CampaignCheckpoint {
        decode_campaign_payload(&boundary.payload)?.transition.after
    } else {
        decode_campaign_resume(&boundary.payload)?.state
    };
    Ok(authored_for_state(&state, milestone))
}

fn verify_prism_encounter(
    events: &[ReplayEvent],
    proof: &ReplayEvent,
) -> Result<bool, ReplayError> {
    let prefix = events
        .iter()
        .filter(|e| e.id < proof.id)
        .cloned()
        .collect::<Vec<_>>();
    let Some(boundary) = prefix.iter().rev().find(|e| {
        matches!(
            e.kind,
            ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
        )
    }) else {
        return Ok(false);
    };
    if !prefix.iter().any(|e| {
        e.id > boundary.id
            && e.kind == ReplayEventKind::BossSpawned
            && e.payload.starts_with(crate::prism::PREFIX)
            && e.account_id == proof.account_id
            && e.activity_id == proof.activity_id
    }) {
        return Ok(false);
    }
    Ok(crate::prism::validate(&prefix)? == Some("Completed"))
}

pub(super) fn prism_admission(
    events: &[ReplayEvent],
    spawn: &ReplayEvent,
) -> Result<bool, ReplayError> {
    if spawn.activity_id.as_deref() != Some("prism_core") {
        return Ok(false);
    }
    let boundary = events
        .iter()
        .rev()
        .find(|e| {
            e.id < spawn.id
                && matches!(
                    e.kind,
                    ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
                )
        })
        .ok_or_else(|| invalid_module("Prism core spawn has no campaign admission"))?;
    let state = if boundary.kind == ReplayEventKind::CampaignCheckpoint {
        decode_campaign_payload(&boundary.payload)?.transition.after
    } else {
        decode_campaign_resume(&boundary.payload)?.state
    };
    if state.active.as_ref().is_none_or(|r| {
        r.chapter != ChapterId::PrismCore
            || r.next_milestone() != Some(Milestone::PrismWardenCleared)
    }) || boundary.actor_id.is_none()
        || boundary.account_id != spawn.account_id
        || boundary.activity_id != spawn.activity_id
        || !events.iter().any(|e| {
            e.id < boundary.id
                && e.kind == ReplayEventKind::PlayerJoined
                && e.actor_id == boundary.actor_id
                && e.account_id == spawn.account_id
        })
    {
        return Err(invalid_module(
            "Prism core spawn violates its campaign boundary",
        ));
    }
    Ok(true)
}

fn verify_counter_encounter(
    events: &[ReplayEvent],
    proof: &ReplayEvent,
    milestone: Milestone,
) -> Result<bool, ReplayError> {
    let prefix_name = match milestone {
        Milestone::CounterSupportCleared => crate::support::PREFIX,
        Milestone::CounterBastionCleared => crate::elite::PREFIX,
        _ => return Ok(true),
    };
    let prefix = events
        .iter()
        .filter(|e| e.id < proof.id)
        .cloned()
        .collect::<Vec<_>>();
    let Some(boundary) = prefix.iter().rev().find(|e| {
        matches!(
            e.kind,
            ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
        )
    }) else {
        return Ok(false);
    };
    // The interrupted pair must restart after this session's admitted boundary;
    // a completed pair from an earlier checkpoint cannot prove another encounter.
    if !prefix.iter().any(|e| {
        e.id > boundary.id
            && e.kind == ReplayEventKind::EnemySpawned
            && e.payload.starts_with(prefix_name)
            && e.account_id == proof.account_id
            && e.activity_id == proof.activity_id
    }) {
        return Ok(false);
    }
    match milestone {
        Milestone::CounterSupportCleared => {
            Ok(crate::support::validate(&prefix)? == Some("Completed"))
        }
        Milestone::CounterBastionCleared => Ok(crate::elite::validate(&prefix)?
            == Some((
                revenant_ai::elite::EliteComposition::BastionLink,
                "Completed",
            ))),
        _ => unreachable!(),
    }
}

// Objective evidence is legal only after the matching admitted checkpoint.
// A partial journal ending at the world event is valid, but cannot skip ahead,
// repeat a visit or use another character's admission.
fn validate_authored_objectives(events: &[ReplayEvent]) -> Result<(), ReplayError> {
    for (index, event) in events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == ReplayEventKind::CampaignObjective)
    {
        let (boundary_index, boundary) = events[..index]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, e)| {
                matches!(
                    e.kind,
                    ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
                )
            })
            .ok_or_else(|| invalid_module("authored objective has no campaign boundary"))?;
        let state = if boundary.kind == ReplayEventKind::CampaignCheckpoint {
            decode_campaign_payload(&boundary.payload)?.transition.after
        } else {
            decode_campaign_resume(&boundary.payload)?.state
        };
        let run = state
            .active
            .as_ref()
            .ok_or_else(|| invalid_module("authored objective after campaign terminal"))?;
        if !matches!(
            run.chapter,
            ChapterId::BrokenSupplyLine
                | ChapterId::CounterSignal
                | ChapterId::TheBreach
                | ChapterId::PrismCore
        ) || state
            .next_milestone()
            .and_then(|m| authored_for_state(&state, m))
            != Some(event.payload.as_str())
            || event.actor_id.is_none()
            || event.actor_id != boundary.actor_id
            || event.account_id != boundary.account_id
            || event.activity_id.as_deref() != Some(run.chapter.as_str())
            || events[boundary_index + 1..index].iter().any(|e| {
                matches!(
                    e.kind,
                    ReplayEventKind::CampaignObjective | ReplayEventKind::ActivityCompleted
                )
            })
        {
            return Err(invalid_module(
                "authored objective violates its admitted boundary",
            ));
        }
    }
    Ok(())
}

// The chapter deliberately sequences two historically exclusive optional pairs.
// Scope only their unrelated field steps; retain every admission/death row so
// actor reuse and terminal checks remain visible to the frozen validators.
pub(super) fn counter_encounter_events(
    events: &[ReplayEvent],
    prefix: &str,
) -> Result<Option<Vec<ReplayEvent>>, ReplayError> {
    if !events.iter().any(|e| {
        e.activity_id.as_deref() == Some("counter_signal")
            && matches!(
                e.kind,
                ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
            )
    }) {
        return Ok(None);
    }
    for (index, event) in events.iter().enumerate().filter(|(_, e)| {
        e.kind == ReplayEventKind::EnemySpawned
            && (e.payload.starts_with(crate::support::PREFIX)
                || e.payload.starts_with(crate::elite::PREFIX))
    }) {
        let boundary = events[..index]
            .iter()
            .rev()
            .find(|e| {
                matches!(
                    e.kind,
                    ReplayEventKind::CampaignCheckpoint | ReplayEventKind::CampaignResumed
                )
            })
            .ok_or_else(|| invalid_module("counter encounter lacks admission"))?;
        let state = if boundary.kind == ReplayEventKind::CampaignCheckpoint {
            decode_campaign_payload(&boundary.payload)?.transition.after
        } else {
            decode_campaign_resume(&boundary.payload)?.state
        };
        let expected = if event.payload.starts_with(crate::support::PREFIX) {
            Milestone::CounterSupportCleared
        } else {
            Milestone::CounterBastionCleared
        };
        if state.active.as_ref().is_none_or(|r| {
            r.chapter != ChapterId::CounterSignal || r.next_milestone() != Some(expected)
        }) || event.account_id != boundary.account_id
            || event.activity_id != boundary.activity_id
        {
            return Err(invalid_module(
                "counter encounter violates its admitted boundary",
            ));
        }
    }
    let other = if prefix == crate::support::PREFIX {
        crate::elite::PREFIX
    } else {
        crate::support::PREFIX
    };
    Ok(Some(
        events
            .iter()
            .filter(|e| !(e.kind == ReplayEventKind::FieldActivity && e.payload.starts_with(other)))
            .cloned()
            .collect(),
    ))
}

fn initial_campaigns(
    events: &[ReplayEvent],
    participants: &[ReconstructedModuleParticipant],
) -> Result<BTreeMap<String, ReconstructedCampaign>, ReplayError> {
    let mut result = BTreeMap::<String, ReconstructedCampaign>::new();
    if let Some((event, payload)) = resume_snapshot(events)? {
        if participants.len() != 1
            || participants[0].character_id != payload.character_id
            || Some(participants[0].actor_id) != event.actor_id
        {
            return Err(invalid_module("campaign resume is not solo"));
        }
        result.insert(
            payload.character_id.clone(),
            ReconstructedCampaign {
                character_id: payload.character_id,
                state: payload.state,
                checkpoints: 0,
                first_clears: 0,
            },
        );
    }
    Ok(result)
}

pub(super) fn reconstruct(
    events: &[ReplayEvent],
    participants: &[ReconstructedModuleParticipant],
) -> Result<Vec<ReconstructedCampaign>, ReplayError> {
    validate_authored_objectives(events)?;
    super::campaign_story::validate_proofs(events)?;
    let mut result = initial_campaigns(events, participants)?;
    let mut operations = BTreeSet::new();
    for (index, event) in events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == ReplayEventKind::CampaignCheckpoint)
    {
        let payload = decode_campaign_payload(&event.payload)?;
        let participant = participants
            .iter()
            .find(|p| p.character_id == payload.character_id)
            .ok_or_else(|| invalid_module("campaign character has no admission snapshot"))?;
        let chapter = match &payload.transition.command {
            Command::Enter { chapter, .. } => *chapter,
            Command::Confirm { .. } | Command::Story { .. } => {
                payload
                    .transition
                    .before
                    .active
                    .as_ref()
                    .ok_or_else(|| invalid_module("campaign confirmation has no active chapter"))?
                    .chapter
            }
        };
        if participants.len() != 1
            || participant.protocol_generation != ReplayProtocolGeneration::V2
            || event.actor_id != Some(participant.actor_id)
            || event.activity_id.as_deref() != Some(chapter.as_str())
            || !operations.insert(payload.operation_id.clone())
            || !events[..index].iter().any(|e| {
                e.kind == ReplayEventKind::ModuleStateSnapshot
                    && e.actor_id == event.actor_id
                    && e.account_id == event.account_id
                    && e.activity_id == event.activity_id
            })
        {
            return Err(invalid_module(
                "campaign is not a unique, admitted solo V2 transition",
            ));
        }
        if let Command::Confirm { milestone, .. } = payload.transition.command {
            let proof_id = payload
                .proof_event_id
                .ok_or_else(|| invalid_module("campaign confirmation has no proof"))?;
            let boundary = events[..index]
                .iter()
                .rev()
                .find(|e| {
                    matches!(
                        e.kind,
                        ReplayEventKind::CampaignCheckpoint
                            | ReplayEventKind::CampaignResumed
                            | ReplayEventKind::ModuleStateSnapshot
                    )
                })
                .ok_or_else(|| invalid_module("campaign confirmation has no session boundary"))?;
            if proof_id <= boundary.id
                || !events[..index]
                    .iter()
                    .any(|e| e.id == proof_id && e.activity_id == event.activity_id)
            {
                return Err(invalid_module(
                    "campaign proof predates its checkpoint or uses a different chapter",
                ));
            }
            verify_campaign_milestone(
                &events[..index],
                &event.account_id,
                participant.actor_id,
                milestone,
                proof_id,
            )?;
        }
        if matches!(payload.transition.command, Command::Story { .. }) {
            super::campaign_story::verify(&events[..index], event, &payload)?;
            if !result.contains_key(&payload.character_id) {
                return Err(invalid_module(
                    "story interaction has no admitted campaign state",
                ));
            }
        }
        let record = result
            .entry(payload.character_id.clone())
            .or_insert_with(|| ReconstructedCampaign {
                character_id: payload.character_id,
                state: payload.transition.before.clone(),
                checkpoints: 0,
                first_clears: 0,
            });
        if record.state != payload.transition.before {
            return Err(invalid_module("campaign checkpoint chain is discontinuous"));
        }
        record.state = payload.transition.after;
        record.checkpoints += 1;
        record.first_clears += usize::from(payload.transition.first_clear.is_some());
    }
    Ok(result.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encode_module_state_snapshot, module_state_evidence_for_catalog,
        ModuleStateSnapshotPayloadV1,
    };
    use revenant_campaign::{ChapterId, RunMode};

    fn event(
        id: i64,
        kind: ReplayEventKind,
        actor_id: Option<u64>,
        payload: String,
    ) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            timestamp: "2026-09-20 16:00:00+00".to_owned(),
            session_id: "campaign-test".to_owned(),
            account_id: "account".to_owned(),
            activity_id: Some("return_signal".to_owned()),
            actor_id,
            payload,
        }
    }

    fn admission() -> Vec<ReplayEvent> {
        vec![
            event(
                1,
                ReplayEventKind::PlayerJoined,
                Some(10),
                "player joined".to_owned(),
            ),
            event(
                2,
                ReplayEventKind::ModuleStateSnapshot,
                Some(10),
                encode_module_state_snapshot(&ModuleStateSnapshotPayloadV1 {
                    schema_version: 1,
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
                .unwrap(),
            ),
        ]
    }

    #[test]
    fn story_proof_binds_character_run_revision_position_and_atomic_checkpoint() {
        use revenant_campaign::{story, ChapterRun};
        let state = CampaignState {
            revision: 5,
            cleared_chapters: 1,
            active: Some(ChapterRun {
                run_id: "story-run".to_owned(),
                chapter: ChapterId::MeridianReadings,
                mode: RunMode::Progress,
                checkpoint: 2,
            }),
            ..CampaignState::default()
        };
        let mut events = admission();
        events.push(event(
            3,
            ReplayEventKind::CampaignResumed,
            Some(10),
            encode_campaign_resume(&CampaignResumePayload {
                schema_version: 1,
                character_id: "character".to_owned(),
                checkpoint_event_id: 1,
                state: state.clone(),
            })
            .unwrap(),
        ));
        let command = Command::Story {
            run_id: "story-run".to_owned(),
            position: story::DEPARTURE_BOARD,
            action: story::Command::ChooseSupply {
                approach: story::SupplyApproach::Service,
            },
        };
        let proof = crate::CampaignStoryProof::new("character", &state, &command).unwrap();
        events.push(event(
            4,
            ReplayEventKind::CampaignStory,
            Some(10),
            proof.encode().unwrap(),
        ));
        let transition = state.propose(5, command).unwrap();
        append(&mut events, transition.clone(), Some(4));
        for e in &mut events {
            e.activity_id = Some("meridian_readings".to_owned());
        }
        let accepted = crate::reconstruct(&events).unwrap();
        assert_eq!(accepted.campaign[0].state, transition.after);
        assert_eq!((accepted.loot_grants, accepted.progression_grants), (0, 0));
        for field in [
            "character_id",
            "run_id",
            "state_revision",
            "position",
            "checkpoint",
            "revision",
        ] {
            let mut invalid = events.clone();
            let mut value: serde_json::Value = serde_json::from_str(&invalid[3].payload).unwrap();
            value[field] = match field {
                "state_revision" | "checkpoint" => serde_json::json!(0),
                "position" => serde_json::json!([0, 0, 0]),
                _ => serde_json::json!("foreign"),
            };
            invalid[3].payload = value.to_string();
            assert!(crate::reconstruct(&invalid).is_err(), "{field}");
        }
        let mut foreign = events.clone();
        foreign[3].actor_id = Some(11);
        assert!(crate::reconstruct(&foreign).is_err());
        assert!(crate::reconstruct(&events[..4]).is_err());
        let mut reused = events.clone();
        let mut again = reused[4].clone();
        again.id = 6;
        reused.push(again);
        assert!(crate::reconstruct(&reused).is_err());
    }

    #[test]
    fn prism_core_draft_cannot_borrow_standalone_or_another_chapter_admission() {
        let mut events = admission();
        let entry = CampaignState::default()
            .propose(
                0,
                Command::Enter {
                    run_id: "return-before-prism".to_owned(),
                    chapter: ChapterId::ReturnSignal,
                    mode: RunMode::Progress,
                },
            )
            .unwrap();
        append(&mut events, entry, None);
        events.push(event(
            4,
            ReplayEventKind::EnemyDied,
            Some(21),
            "enemy died: relay-drone".to_owned(),
        ));
        let mut spawn = event(
            5,
            ReplayEventKind::BossSpawned,
            Some(20),
            crate::encode_prism_evidence(&crate::PrismEvidence {
                elapsed_ms: 0,
                previous_confirmed_ms: 0,
                transition: crate::PrismTransition::Started {
                    boss_id: 20,
                    player_health: 100,
                },
            })
            .unwrap(),
        );
        events.push(spawn.clone());
        assert_eq!(crate::prism::validate(&events).unwrap(), Some("Active"));
        spawn.activity_id = Some("prism_core".to_owned());
        *events.last_mut().unwrap() = spawn.clone();
        // Even a real standalone prerequisite cannot replace the campaign gate.
        assert!(crate::prism::validate(&events).is_err());
        events.retain(|e| e.kind != ReplayEventKind::CampaignCheckpoint);
        assert!(prism_admission(&events, &spawn).is_err());
    }

    fn append(events: &mut Vec<ReplayEvent>, transition: Transition, proof_event_id: Option<i64>) {
        let id = events.last().unwrap().id + 1;
        events.push(event(
            id,
            ReplayEventKind::CampaignCheckpoint,
            Some(10),
            encode_campaign_payload(&CampaignReplayPayload {
                schema_version: 1,
                character_id: "character".to_owned(),
                operation_id: format!("operation-{id}"),
                transition,
                proof_event_id,
            })
            .unwrap(),
        ));
    }

    fn prism_combat(events: &mut Vec<ReplayEvent>) {
        for source in crate::prism::tests::completed_fixture()
            .iter()
            .filter(|e| e.payload.starts_with(crate::prism::PREFIX))
        {
            let mut evidence: crate::PrismEvidence =
                serde_json::from_str(&source.payload[crate::prism::PREFIX.len()..]).unwrap();
            if let crate::PrismTransition::Started { boss_id, .. } = &mut evidence.transition {
                *boss_id = 20;
            }
            let mut row = event(
                events.last().unwrap().id + 1,
                source.kind,
                Some(if source.actor_id == Some(1) { 10 } else { 20 }),
                crate::encode_prism_evidence(&evidence).unwrap(),
            );
            row.activity_id = Some("prism_core".to_owned());
            events.push(row);
        }
    }

    #[test]
    fn prism_campaign_replays_both_phases_and_all_four_boundaries_without_drone() {
        let mut state = CampaignState {
            story: revenant_campaign::story::StoryState::default(),
            revision: 25,
            cleared_chapters: 5,
            active: None,
        };
        let entered = state
            .propose(
                state.revision,
                Command::Enter {
                    run_id: "prism-campaign".to_owned(),
                    chapter: ChapterId::PrismCore,
                    mode: RunMode::Progress,
                },
            )
            .unwrap();
        let mut events = admission();
        append(&mut events, entered.clone(), None);
        for row in &mut events {
            row.activity_id = Some("prism_core".to_owned());
        }
        state = entered.after;
        for milestone in ChapterId::PrismCore.milestones() {
            if *milestone == Milestone::PrismWardenCleared {
                prism_combat(&mut events);
            }
            let proof_id = events.last().unwrap().id + 1;
            let mut proof = event(
                proof_id,
                ReplayEventKind::CampaignObjective,
                Some(10),
                revenant_campaign::prism_objective(*milestone)
                    .unwrap()
                    .to_owned(),
            );
            proof.activity_id = Some("prism_core".to_owned());
            events.push(proof);
            let next = state
                .propose(
                    state.revision,
                    Command::Confirm {
                        run_id: "prism-campaign".to_owned(),
                        milestone: *milestone,
                    },
                )
                .unwrap();
            append(&mut events, next.clone(), Some(proof_id));
            events.last_mut().unwrap().activity_id = Some("prism_core".to_owned());
            assert_eq!(
                crate::reconstruct(&events).unwrap().campaign[0].state,
                next.after
            );
            state = next.after;
        }
        assert_eq!(state.cleared_chapters, 6);
        let mut no_death = events.clone();
        no_death.retain(|e| e.kind != ReplayEventKind::EnemyDied);
        assert!(crate::reconstruct(&no_death).is_err());
        let mut premature_return = events.clone();
        premature_return
            .iter_mut()
            .find(|e| e.kind == ReplayEventKind::CampaignObjective)
            .unwrap()
            .payload = revenant_campaign::prism_objective(Milestone::PrismReturnCompleted)
            .unwrap()
            .to_owned();
        assert!(crate::reconstruct(&premature_return).is_err());
        let mut foreign_spawn = events.clone();
        foreign_spawn
            .iter_mut()
            .find(|e| e.kind == ReplayEventKind::BossSpawned)
            .unwrap()
            .account_id = "foreign".to_owned();
        assert!(crate::reconstruct(&foreign_spawn).is_err());
    }

    #[test]
    fn prism_resume_admits_only_a_fresh_encounter_at_the_guard_boundary() {
        let state = CampaignState {
            story: revenant_campaign::story::StoryState::default(),
            revision: 27,
            cleared_chapters: 5,
            active: Some(revenant_campaign::ChapterRun {
                run_id: "prism-resume".to_owned(),
                chapter: ChapterId::PrismCore,
                mode: RunMode::Progress,
                checkpoint: 1,
            }),
        };
        let mut events = admission();
        for row in &mut events {
            row.id += 100;
            row.activity_id = Some("prism_core".to_owned());
        }
        let mut resume = event(
            103,
            ReplayEventKind::CampaignResumed,
            Some(10),
            encode_campaign_resume(&CampaignResumePayload {
                schema_version: 1,
                character_id: "character".to_owned(),
                checkpoint_event_id: 50,
                state: state.clone(),
            })
            .unwrap(),
        );
        resume.activity_id = Some("prism_core".to_owned());
        events.push(resume);
        prism_combat(&mut events);
        assert!(crate::reconstruct(&events).is_ok());
        let spawn = events
            .iter()
            .find(|e| e.kind == ReplayEventKind::BossSpawned)
            .unwrap()
            .clone();
        assert!(prism_admission(&events, &spawn).unwrap());
        for checkpoint in [0, 2, 3] {
            let mut bad = events.clone();
            let mut invalid = state.clone();
            invalid.active.as_mut().unwrap().checkpoint = checkpoint;
            bad[2].payload = encode_campaign_resume(&CampaignResumePayload {
                schema_version: 1,
                character_id: "character".to_owned(),
                checkpoint_event_id: 50,
                state: invalid,
            })
            .unwrap();
            assert!(prism_admission(&bad, &spawn).is_err());
        }
        events[2].actor_id = Some(11);
        assert!(prism_admission(&events, &spawn).is_err());
    }

    fn counter_event(
        events: &mut Vec<ReplayEvent>,
        kind: ReplayEventKind,
        actor: u64,
        payload: String,
    ) -> i64 {
        let id = events.last().unwrap().id + 1;
        let mut row = event(id, kind, Some(actor), payload);
        row.activity_id = Some("counter_signal".to_owned());
        events.push(row);
        id
    }

    fn counter_pair(events: &mut Vec<ReplayEvent>, elite: bool) {
        use crate::{
            encode_elite_evidence, encode_support_evidence, EliteEvidence, EliteTransition,
            SupportEvidence, SupportTransition,
        };
        let guard = if elite { 22 } else { 20 };
        let partner = guard + 1;
        let start = if elite {
            encode_elite_evidence(&EliteEvidence {
                elapsed_ms: 0,
                previous_confirmed_ms: 0,
                transition: EliteTransition::Started {
                    composition: "bastion_link".to_owned(),
                    bulwark_id: guard,
                    partner_id: partner,
                    player_health: 100,
                },
            })
            .unwrap()
        } else {
            encode_support_evidence(&SupportEvidence {
                elapsed_ms: 0,
                transition: SupportTransition::Started {
                    lancer_id: guard,
                    mender_id: partner,
                    player_health: 100,
                },
            })
            .unwrap()
        };
        counter_event(events, ReplayEventKind::EnemySpawned, guard, start);
        let mut time = 0;
        for (target, health) in [(partner, 120), (guard, if elite { 240 } else { 200 })] {
            for health_before in (1..=health / 40).rev().map(|n| n * 40) {
                time += 400;
                let health_after = health_before - 40;
                let payload = if elite {
                    encode_elite_evidence(&EliteEvidence {
                        elapsed_ms: time,
                        previous_confirmed_ms: time - 400,
                        transition: EliteTransition::Attack {
                            target_id: target,
                            player_position: [8, 0, -6],
                            damage: 40,
                            health_before,
                            health_after,
                        },
                    })
                    .unwrap()
                } else {
                    encode_support_evidence(&SupportEvidence {
                        elapsed_ms: time,
                        transition: SupportTransition::Attack {
                            target_id: target,
                            player_position: [5, 0, 7],
                            damage: 40,
                            health_before,
                            health_after,
                        },
                    })
                    .unwrap()
                };
                counter_event(
                    events,
                    if health_after == 0 {
                        ReplayEventKind::EnemyDied
                    } else {
                        ReplayEventKind::FieldActivity
                    },
                    if health_after == 0 { target } else { 10 },
                    payload,
                );
            }
        }
    }

    #[test]
    fn counter_sequence_requires_both_pairs_and_rejects_forged_or_early_proofs() {
        let initial = CampaignState {
            story: revenant_campaign::story::StoryState::default(),
            revision: 12,
            cleared_chapters: 3,
            active: None,
        };
        let entry = initial
            .propose(
                12,
                Command::Enter {
                    run_id: "counter-run".to_owned(),
                    chapter: ChapterId::CounterSignal,
                    mode: RunMode::Progress,
                },
            )
            .unwrap();
        let mut state = entry.after.clone();
        let mut events = admission();
        append(&mut events, entry, None);
        for e in &mut events {
            e.activity_id = Some("counter_signal".to_owned());
        }
        // An elite pair cannot start at the approach boundary.
        let mut early = events.clone();
        counter_pair(&mut early, true);
        assert!(crate::reconstruct(&early).is_err());
        for milestone in ChapterId::CounterSignal.milestones() {
            let mut forged = events.clone();
            let payload = revenant_campaign::counter_objective(*milestone)
                .unwrap()
                .to_owned();
            if matches!(
                milestone,
                Milestone::CounterSupportCleared | Milestone::CounterBastionCleared
            ) {
                let id = counter_event(
                    &mut forged,
                    ReplayEventKind::CampaignObjective,
                    10,
                    payload.clone(),
                );
                assert!(verify_campaign_milestone(&forged, "account", 10, *milestone, id).is_err());
                counter_pair(&mut events, *milestone == Milestone::CounterBastionCleared);
                // A single remaining member cannot satisfy the chapter boundary.
                let mut partial = events.clone();
                partial.pop();
                let id = counter_event(
                    &mut partial,
                    ReplayEventKind::CampaignObjective,
                    10,
                    payload.clone(),
                );
                assert!(
                    verify_campaign_milestone(&partial, "account", 10, *milestone, id).is_err()
                );
            }
            let proof = counter_event(&mut events, ReplayEventKind::CampaignObjective, 10, payload);
            for tamper in 0..4 {
                let mut bad = events.clone();
                let row = bad.last_mut().unwrap();
                match tamper {
                    0 => row.actor_id = Some(11),
                    1 => row.account_id = "foreign".to_owned(),
                    2 => row.payload.push_str(":wrong-position"),
                    _ => row.activity_id = Some("broken_supply_line".to_owned()),
                }
                assert!(crate::reconstruct(&bad).is_err());
            }
            let next = state
                .propose(
                    state.revision,
                    Command::Confirm {
                        run_id: "counter-run".to_owned(),
                        milestone: *milestone,
                    },
                )
                .unwrap();
            append(&mut events, next.clone(), Some(proof));
            events.last_mut().unwrap().activity_id = Some("counter_signal".to_owned());
            state = next.after;
            assert_eq!(
                crate::reconstruct(&events).unwrap().campaign[0].state,
                state
            );
        }
        assert_eq!(state.cleared_chapters, 4);
        assert!(state.active.is_none());
        let mut duplicate = events[3].clone();
        duplicate.id = events.last().unwrap().id + 1;
        events.push(duplicate);
        assert!(crate::reconstruct(&events).is_err());
    }

    #[test]
    fn breach_replay_requires_the_stabilizer_and_a_spawned_guard_before_core_access() {
        let initial = CampaignState {
            story: revenant_campaign::story::StoryState::default(),
            revision: 20,
            cleared_chapters: 4,
            active: None,
        };
        let entry = initial
            .propose(
                20,
                Command::Enter {
                    run_id: "breach-run".to_owned(),
                    chapter: ChapterId::TheBreach,
                    mode: RunMode::Progress,
                },
            )
            .unwrap();
        let mut state = entry.after.clone();
        let mut events = admission();
        append(&mut events, entry, None);
        for e in &mut events {
            e.activity_id = Some("the_breach".to_owned());
        }
        for milestone in ChapterId::TheBreach.milestones() {
            let id = events.last().unwrap().id + 1;
            let (kind, actor, payload) = if *milestone == Milestone::BreachGuardCleared {
                (
                    ReplayEventKind::EnemyDied,
                    20,
                    "enemy died: warden".to_owned(),
                )
            } else {
                (
                    ReplayEventKind::CampaignObjective,
                    10,
                    revenant_campaign::breach_objective(*milestone)
                        .unwrap()
                        .to_owned(),
                )
            };
            let mut proof = event(id, kind, Some(actor), payload);
            proof.activity_id = Some("the_breach".to_owned());
            events.push(proof);
            if kind == ReplayEventKind::CampaignObjective {
                let mut invalid = events.clone();
                invalid.last_mut().unwrap().payload = "breach-v1:core:10,0,1".to_owned();
                assert!(crate::reconstruct(&invalid).is_err());
            } else {
                let missing_spawn = events
                    .iter()
                    .filter(|e| e.kind != ReplayEventKind::BossSpawned)
                    .cloned()
                    .collect::<Vec<_>>();
                assert!(
                    verify_campaign_milestone(&missing_spawn, "account", 10, *milestone, id)
                        .is_err()
                );
            }
            let next = state
                .propose(
                    state.revision,
                    Command::Confirm {
                        run_id: "breach-run".to_owned(),
                        milestone: *milestone,
                    },
                )
                .unwrap();
            append(&mut events, next.clone(), Some(id));
            events.last_mut().unwrap().activity_id = Some("the_breach".to_owned());
            state = next.after;
            assert_eq!(
                crate::reconstruct(&events).unwrap().campaign[0].state,
                state
            );
            if *milestone == Milestone::BreachDoorReached {
                let mut spawn = event(
                    events.last().unwrap().id + 1,
                    ReplayEventKind::BossSpawned,
                    Some(20),
                    "boss spawned: warden".to_owned(),
                );
                spawn.activity_id = Some("the_breach".to_owned());
                events.push(spawn);
                let mut skipped = events.clone();
                let mut illegal = event(
                    events.last().unwrap().id + 1,
                    ReplayEventKind::CampaignObjective,
                    Some(10),
                    "breach-v1:core:10,0,0".to_owned(),
                );
                illegal.activity_id = Some("the_breach".to_owned());
                skipped.push(illegal);
                assert!(crate::reconstruct(&skipped).is_err());
            }
        }
        assert_eq!(state.cleared_chapters, 5);
    }

    #[test]
    fn supply_objectives_require_current_boundary_owner_and_exact_location() {
        let state = CampaignState {
            story: revenant_campaign::story::StoryState::default(),
            revision: 7,
            cleared_chapters: 2,
            active: None,
        };
        let entry = state
            .propose(
                7,
                Command::Enter {
                    run_id: "supply-one".to_owned(),
                    chapter: ChapterId::BrokenSupplyLine,
                    mode: RunMode::Progress,
                },
            )
            .unwrap();
        let mut events = admission();
        append(&mut events, entry.clone(), None);
        for event in &mut events {
            event.activity_id = Some("broken_supply_line".to_owned());
        }
        events.push(event(
            4,
            ReplayEventKind::CampaignObjective,
            Some(10),
            "supply-v1:route:-4,0,4".to_owned(),
        ));
        events.last_mut().unwrap().activity_id = Some("broken_supply_line".to_owned());
        assert!(crate::reconstruct(&events).is_ok());
        for change in 0..5 {
            let mut bad = events.clone();
            let proof = bad.last_mut().unwrap();
            match change {
                0 => proof.actor_id = Some(11),
                1 => proof.account_id = "foreign".to_owned(),
                2 => proof.payload = "supply-v1:delivery:-1,0,-5".to_owned(),
                3 => proof.payload = "supply-v1:route:-4,0,3".to_owned(),
                _ => proof.activity_id = Some("return_signal".to_owned()),
            }
            assert!(crate::reconstruct(&bad).is_err());
        }
        let mut duplicate = events.last().unwrap().clone();
        duplicate.id = 5;
        events.push(duplicate);
        assert!(crate::reconstruct(&events).is_err());
        events.pop();
        let next = entry
            .after
            .propose(
                entry.after.revision,
                Command::Confirm {
                    run_id: "supply-one".to_owned(),
                    milestone: Milestone::SupplyRouteReached,
                },
            )
            .unwrap();
        append(&mut events, next.clone(), Some(4));
        events.last_mut().unwrap().activity_id = Some("broken_supply_line".to_owned());
        assert_eq!(
            crate::reconstruct(&events).unwrap().campaign[0].state,
            next.after
        );
        // Reaching the cell while the guard is still alive cannot advance.
        events.push(event(
            6,
            ReplayEventKind::CampaignObjective,
            Some(10),
            "supply-v1:cell:-4,0,-3".to_owned(),
        ));
        events.last_mut().unwrap().activity_id = Some("broken_supply_line".to_owned());
        assert!(crate::reconstruct(&events).is_err());
    }

    #[test]
    fn campaign_checkpoint_proofs_and_resumed_run_reconstruct_without_client_objectives() {
        let mut events = admission();
        let enter = CampaignState::default()
            .propose(
                0,
                Command::Enter {
                    run_id: "run-one".to_owned(),
                    chapter: ChapterId::ReturnSignal,
                    mode: RunMode::Progress,
                },
            )
            .unwrap();
        append(&mut events, enter.clone(), None);
        events.push(event(
            4,
            ReplayEventKind::EnemySpawned,
            Some(20),
            "enemy spawned: relay-drone".to_owned(),
        ));
        events.push(event(
            5,
            ReplayEventKind::EnemyDied,
            Some(20),
            "enemy died: relay-drone".to_owned(),
        ));
        let checkpoint = enter
            .after
            .propose(
                1,
                Command::Confirm {
                    run_id: "run-one".to_owned(),
                    milestone: Milestone::RelayGuardCleared,
                },
            )
            .unwrap();
        append(&mut events, checkpoint.clone(), Some(5));
        let state = crate::reconstruct(&events).unwrap();
        assert_eq!(state.campaign[0].state, checkpoint.after);
        assert_eq!(state.campaign[0].first_clears, 0);
        let mut invalid = events.clone();
        invalid[4].account_id = "foreign-account".to_owned();
        assert!(crate::reconstruct(&invalid).is_err());
        invalid = events;
        invalid[4].actor_id = Some(99);
        assert!(crate::reconstruct(&invalid).is_err());

        // A new session starts directly at the accepted safe boundary. No
        // duplicate drone kill or second chapter entry is required to resume.
        let mut resumed = admission();
        resumed.push(event(
            3,
            ReplayEventKind::BossSpawned,
            Some(30),
            "boss spawned: warden".to_owned(),
        ));
        resumed.push(event(
            4,
            ReplayEventKind::EnemyDied,
            Some(30),
            "enemy died: warden".to_owned(),
        ));
        let finish = checkpoint
            .after
            .propose(
                2,
                Command::Confirm {
                    run_id: "run-one".to_owned(),
                    milestone: Milestone::ReturnSignalRecovered,
                },
            )
            .unwrap();
        append(&mut resumed, finish.clone(), Some(4));
        let state = crate::reconstruct(&resumed).unwrap();
        assert_eq!(state.campaign[0].state.cleared_chapters, 1);
        assert_eq!(state.campaign[0].first_clears, 1);
        assert_eq!(state.loot_grants, 0); // Policy evidence alone does not grant inventory.
        append(&mut resumed, finish, Some(4));
        assert!(crate::reconstruct(&resumed).is_err());
    }

    #[test]
    fn meridian_proof_requires_completed_authored_chain_and_owner() {
        let mut events = Vec::new();
        for (index, step) in [
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
                i64::try_from(index).unwrap() + 1,
                ReplayEventKind::FieldActivity,
                Some(10),
                format!("meridian-v1:{step}"),
            ));
        }
        for (milestone, proof) in [
            (Milestone::MeridianReached, 2),
            (Milestone::SurveyLinked, 4),
            (Milestone::RecoveryDelivered, 6),
        ] {
            verify_campaign_milestone(&events, "account", 10, milestone, proof).unwrap();
            assert!(verify_campaign_milestone(&events, "account", 99, milestone, proof).is_err());
            assert!(
                verify_campaign_milestone(&events, "account", 10, milestone, proof - 1).is_err()
            );
        }
        events.remove(2);
        assert!(
            verify_campaign_milestone(&events, "account", 10, Milestone::SurveyLinked, 4).is_err()
        );
    }

    #[test]
    fn meridian_admission_restores_both_boundaries_without_fabricated_visits() {
        use revenant_campaign::ChapterRun;
        for (checkpoint, steps, milestone) in [
            (
                1,
                ["meridian_lens", "meridian_gallery"],
                Milestone::SurveyLinked,
            ),
            (
                2,
                ["meridian_log", "meridian_return"],
                Milestone::RecoveryDelivered,
            ),
        ] {
            let state = CampaignState {
                story: revenant_campaign::story::StoryState::default(),
                revision: 4 + u64::from(checkpoint),
                cleared_chapters: 1,
                active: Some(ChapterRun {
                    run_id: "meridian-run".to_owned(),
                    chapter: ChapterId::MeridianReadings,
                    mode: RunMode::Progress,
                    checkpoint,
                }),
            };
            let mut events = admission();
            for e in &mut events {
                e.id += 100;
                e.activity_id = Some("meridian_readings".to_owned());
            }
            let mut resumed = event(
                103,
                ReplayEventKind::CampaignResumed,
                Some(10),
                encode_campaign_resume(&CampaignResumePayload {
                    schema_version: 1,
                    character_id: "character".to_owned(),
                    checkpoint_event_id: 90,
                    state: state.clone(),
                })
                .unwrap(),
            );
            resumed.activity_id = Some("meridian_readings".to_owned());
            events.push(resumed);
            let initial = crate::reconstruct(&events).unwrap();
            assert_eq!(initial.campaign[0].state, state);
            assert_eq!(initial.campaign[0].checkpoints, 0);
            assert_eq!(initial.field_objectives["meridian_arrival"], "Completed");
            for step in steps {
                let mut visited = event(
                    events.last().unwrap().id + 1,
                    ReplayEventKind::FieldActivity,
                    Some(10),
                    format!("meridian-v1:{step}"),
                );
                visited.activity_id = Some("meridian_readings".to_owned());
                events.push(visited);
            }
            let proof = events.last().unwrap().id;
            let transition = state
                .propose(
                    state.revision,
                    Command::Confirm {
                        run_id: "meridian-run".to_owned(),
                        milestone,
                    },
                )
                .unwrap();
            append(&mut events, transition.clone(), Some(proof));
            events.last_mut().unwrap().activity_id = Some("meridian_readings".to_owned());
            let reconstructed = crate::reconstruct(&events).unwrap();
            assert_eq!(reconstructed.campaign[0].state, transition.after);
            assert_eq!(reconstructed.loot_grants, 0);
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.kind == ReplayEventKind::FieldActivity)
                    .count(),
                2
            );

            let mut bad = events.clone();
            bad[2].account_id = "foreign".to_owned();
            assert!(crate::reconstruct(&bad).is_err());
            bad = events.clone();
            bad[3].payload = "meridian-v1:started".to_owned();
            assert!(crate::reconstruct(&bad).is_err());
            bad = events.clone();
            bad[3].payload = "meridian-v1:meridian_arrival".to_owned();
            assert!(crate::reconstruct(&bad).is_err());
            bad = events;
            let mut duplicate = bad[2].clone();
            duplicate.id = bad.last().unwrap().id + 1;
            bad.push(duplicate);
            assert!(crate::reconstruct(&bad).is_err());
        }
    }
}
