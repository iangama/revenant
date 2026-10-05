//! Bounded challenge journal codec. This checks deterministic transition content;
//! `challenge_authority` checks the admitted session and world proofs separately.
use revenant_challenges::{ChallengeState, Command, ContractId, EndReason};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{invalid_module, ReplayError};

const MAX_PAYLOAD_BYTES: usize = 16_384;

#[cfg(test)]
#[path = "challenges/modifier_tests.rs"]
mod modifier_tests;

/// Checks a character's full journal from empty state. Callers must independently
/// verify each immutable row's account/session identity and world proof; a
/// coherent client-supplied chain is never sufficient authority.
///
/// # Errors
/// Rejects foreign characters, gaps, branches and reused operation identities.
pub fn reconstruct_challenge_journal(
    character_id: &str,
    entries: &[ChallengeReplayPayload],
) -> Result<ChallengeState, ReplayError> {
    if character_id.is_empty() || character_id.len() > 256 {
        return Err(invalid_module("invalid challenge journal character"));
    }
    let mut state = ChallengeState::default();
    let mut operations = BTreeSet::new();
    for entry in entries {
        validate(entry)?;
        if entry.character_id != character_id
            || entry.before != state
            || !operations.insert(&entry.operation_id)
        {
            return Err(invalid_module(
                "challenge journal is not a unique character transition chain",
            ));
        }
        state = entry.after.clone();
    }
    Ok(state)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeReplayPayload {
    pub schema_version: u8,
    pub policy_revision: String,
    pub character_id: String,
    pub operation_id: String,
    pub before: ChallengeState,
    pub command: Command,
    pub after: ChallengeState,
    pub first_completion: Option<ContractId>,
    /// Start/retry cite accepted equipment; Confirm cites world evidence;
    /// defeated cites the fatal hit. These links still require server-side
    /// identity, session, attempt and event-kind verification by the adapter.
    pub proof_event_id: Option<i64>,
}

/// # Errors
/// Rejects malformed identities, proof-reference shapes or forged transitions.
/// A valid encoding alone does not establish authenticated world completion.
pub fn encode_challenge_payload(payload: &ChallengeReplayPayload) -> Result<String, ReplayError> {
    validate(payload)?;
    let encoded = serde_json::to_string(payload).map_err(|e| invalid_module(e.to_string()))?;
    if encoded.len() > MAX_PAYLOAD_BYTES {
        return Err(invalid_module("challenge payload exceeds its bound"));
    }
    Ok(encoded)
}

/// # Errors
/// Rejects oversized, malformed or nondeterministic transition payloads.
pub fn decode_challenge_payload(encoded: &str) -> Result<ChallengeReplayPayload, ReplayError> {
    if encoded.len() > MAX_PAYLOAD_BYTES {
        return Err(invalid_module("challenge payload exceeds its bound"));
    }
    let payload = serde_json::from_str(encoded).map_err(|e| invalid_module(e.to_string()))?;
    validate(&payload)?;
    Ok(payload)
}

fn validate(payload: &ChallengeReplayPayload) -> Result<(), ReplayError> {
    let requires_proof = !matches!(
        payload.command,
        Command::End {
            reason: EndReason::Abandoned | EndReason::Interrupted,
            ..
        }
    );
    if payload.schema_version != 1
        || payload.character_id.is_empty()
        || payload.character_id.len() > 256
        || payload.operation_id.is_empty()
        || payload.operation_id.len() > 96
        || !payload
            .operation_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || (requires_proof && payload.proof_event_id.is_none_or(|id| id <= 0))
        || (!requires_proof && payload.proof_event_id.is_some())
    {
        return Err(invalid_module(
            "invalid challenge identity or proof reference",
        ));
    }
    let expected = revenant_challenges::apply(
        &payload.policy_revision,
        &payload.before,
        payload.before.revision,
        &payload.command,
    )
    .map_err(|e| invalid_module(e.to_string()))?;
    if expected.state != payload.after || expected.first_completion != payload.first_completion {
        return Err(invalid_module(
            "challenge journal differs from its deterministic transition",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use revenant_challenges::{Equipment, Objective, REVISION};

    fn start() -> ChallengeReplayPayload {
        let before = ChallengeState::default();
        let command = Command::Start {
            contract: ContractId::CloseQuarters,
            equipment: Equipment {
                loadout_revision: 0,
                catalog_revision: revenant_modules::BUILD_CATALOG_REVISION.to_owned(),
                weapon_id: "pulse_rifle".to_owned(),
                modules: vec![],
            },
        };
        let transition = revenant_challenges::apply(REVISION, &before, 0, &command).unwrap();
        ChallengeReplayPayload {
            schema_version: 1,
            policy_revision: REVISION.to_owned(),
            character_id: "local:challenge:operator".into(),
            operation_id: "start-1".into(),
            before,
            command,
            after: transition.state,
            first_completion: None,
            proof_event_id: Some(7),
        }
    }

    #[test]
    fn challenge_codec_rejects_forged_results_revisions_and_identity() {
        let payload = start();
        assert_eq!(
            decode_challenge_payload(&encode_challenge_payload(&payload).unwrap()).unwrap(),
            payload
        );
        let mut changed = payload.clone();
        changed.first_completion = Some(ContractId::CloseQuarters);
        assert!(encode_challenge_payload(&changed).is_err());
        let mut changed = payload.clone();
        changed.after.revision += 1;
        assert!(encode_challenge_payload(&changed).is_err());
        let mut changed = payload.clone();
        changed.policy_revision = "future".into();
        assert!(encode_challenge_payload(&changed).is_err());
        let mut changed = payload.clone();
        changed.character_id.clear();
        assert!(encode_challenge_payload(&changed).is_err());
        let mut changed = payload.clone();
        changed.operation_id = "foreign:operation".into();
        assert!(encode_challenge_payload(&changed).is_err());
        let mut wire = serde_json::to_value(&payload).unwrap();
        wire["reward_fragments"] = 1.into();
        assert!(decode_challenge_payload(&wire.to_string()).is_err());
        assert!(decode_challenge_payload(&" ".repeat(MAX_PAYLOAD_BYTES + 1)).is_err());
    }

    #[test]
    fn challenge_terminal_is_derived_from_the_last_objective_with_a_required_proof() {
        let mut payload = start();
        for (index, objective) in [Objective::MenderCleared, Objective::LancerCleared]
            .into_iter()
            .enumerate()
        {
            payload.before = payload.after.clone();
            payload.command = Command::Confirm {
                run_id: 1,
                objective,
            };
            payload.operation_id = format!("confirm-{index}");
            let transition = revenant_challenges::apply(
                REVISION,
                &payload.before,
                payload.before.revision,
                &payload.command,
            )
            .unwrap();
            payload.after = transition.state;
            payload.first_completion = transition.first_completion;
            assert!(encode_challenge_payload(&payload).is_ok());
            let mut no_proof = payload.clone();
            no_proof.proof_event_id = None;
            assert!(encode_challenge_payload(&no_proof).is_err());
        }
        assert_eq!(payload.first_completion, Some(ContractId::CloseQuarters));
        let mut changed = payload.clone();
        changed.first_completion = None;
        assert!(encode_challenge_payload(&changed).is_err());
        let mut changed = payload;
        changed.after.records.clear();
        assert!(encode_challenge_payload(&changed).is_err());
    }

    #[test]
    fn challenge_journal_rejects_gaps_foreign_characters_and_operation_reuse() {
        let first = start();
        let mut second = first.clone();
        second.before = first.after.clone();
        second.command = Command::End {
            run_id: 1,
            reason: EndReason::Abandoned,
        };
        second.operation_id = "abandon-1".into();
        second.proof_event_id = None;
        second.after = revenant_challenges::apply(REVISION, &second.before, 1, &second.command)
            .unwrap()
            .state;
        let entries = [first.clone(), second.clone()];
        assert_eq!(
            reconstruct_challenge_journal(&first.character_id, &entries).unwrap(),
            second.after
        );
        assert_eq!(
            reconstruct_challenge_journal(&first.character_id, &[]).unwrap(),
            ChallengeState::default()
        );
        assert!(reconstruct_challenge_journal("other-character", &entries).is_err());
        assert!(reconstruct_challenge_journal(&first.character_id, &[second.clone()]).is_err());
        let mut duplicate = second.clone();
        duplicate.operation_id.clone_from(&first.operation_id);
        assert!(
            reconstruct_challenge_journal(&first.character_id, &[first.clone(), duplicate])
                .is_err()
        );
        let mut with_proof = second;
        with_proof.proof_event_id = Some(8);
        assert!(encode_challenge_payload(&with_proof).is_err());
        assert!(reconstruct_challenge_journal(
            &first.character_id,
            &[first.clone(), first.clone()]
        )
        .is_err());
    }
}
