//! Recovery timing is reconstructed from the server's complete accepted move
//! stream. A duration, station position or client completion claim alone is not
//! enough: every hold must follow arrival without an intervening departure.
use revenant_challenges::{
    recovery::{Action, Checkpoint, RecoveryRun},
    ChallengeRun, Command, ContractId, Objective,
};
use serde::{Deserialize, Serialize};

use crate::{decode_challenge_payload, invalid_module, ReplayError, ReplayEvent, ReplayEventKind};

pub const PREFIX: &str = "challenge-recovery-v1:";
const MAX_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEvidence {
    pub run_id: u64,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub action: Action,
    pub checkpoint: Option<Checkpoint>,
}

impl RecoveryEvidence {
    #[must_use]
    pub fn objective(&self) -> Option<Objective> {
        self.checkpoint.map(|checkpoint| match checkpoint {
            Checkpoint::Retrieved => Objective::CoolantRetrieved,
            Checkpoint::Transferred => Objective::CoolantTransferred,
            Checkpoint::Delivered => Objective::CoolantDelivered,
        })
    }

    /// # Errors
    /// Rejects invalid attempt/sequence identities and oversized evidence.
    pub fn encode(&self) -> Result<String, ReplayError> {
        if self.run_id == 0 || self.sequence == 0 {
            return Err(invalid_module("recovery proof has no attempt or sequence"));
        }
        let encoded = format!(
            "{PREFIX}{}",
            serde_json::to_string(self).map_err(|e| invalid_module(e.to_string()))?
        );
        if encoded.len() > MAX_BYTES {
            return Err(invalid_module("recovery proof exceeds its bound"));
        }
        Ok(encoded)
    }

    /// # Errors
    /// Rejects unversioned, malformed, unbounded or unidentified evidence.
    pub fn decode(encoded: &str) -> Result<Self, ReplayError> {
        if encoded.len() > MAX_BYTES {
            return Err(invalid_module("recovery proof exceeds its bound"));
        }
        let body = encoded
            .strip_prefix(PREFIX)
            .ok_or_else(|| invalid_module("missing recovery proof revision"))?;
        let proof: Self = serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))?;
        proof.encode()?;
        Ok(proof)
    }
}

pub(super) fn validate(events: &[ReplayEvent], run: &ChallengeRun) -> Result<(), ReplayError> {
    let invalid =
        || invalid_module("recovery stream differs from its admitted attempt or continuous hold");
    let anchor = events.first().ok_or_else(invalid)?;
    if run.rules.contract != ContractId::CoolantRecovery
        || anchor.kind != ReplayEventKind::ChallengeCheckpoint
    {
        return Err(invalid());
    }
    let mut recovery = RecoveryRun::default();
    let mut sequence = 0;
    let mut progress = 0;
    for (index, event) in events.iter().enumerate().skip(1) {
        if event.kind == ReplayEventKind::ChallengeCheckpoint {
            continue;
        }
        if event.kind != ReplayEventKind::FieldActivity || event.actor_id != anchor.actor_id {
            return Err(invalid());
        }
        let proof = RecoveryEvidence::decode(&event.payload)?;
        sequence += 1;
        if proof.run_id != run.run_id || proof.sequence != sequence {
            return Err(invalid());
        }
        let next = recovery
            .apply(proof.elapsed_ms, proof.action)
            .map_err(|_| invalid())?;
        if next.checkpoint != proof.checkpoint {
            return Err(invalid());
        }
        recovery = next.run;
        if let Some(objective) = proof.objective() {
            let checkpoint = events
                .get(index + 1)
                .filter(|e| e.kind == ReplayEventKind::ChallengeCheckpoint)
                .ok_or_else(invalid)?;
            let entry = decode_challenge_payload(&checkpoint.payload)?;
            if entry.proof_event_id != Some(event.id)
                || entry.command
                    != (Command::Confirm {
                        run_id: run.run_id,
                        objective,
                    })
            {
                return Err(invalid());
            }
            progress = match proof.checkpoint {
                Some(Checkpoint::Retrieved) => 1,
                Some(Checkpoint::Transferred) => 3,
                Some(Checkpoint::Delivered) => 7,
                None => unreachable!(),
            };
        }
    }
    if progress != run.objectives {
        return Err(invalid());
    }
    Ok(())
}
