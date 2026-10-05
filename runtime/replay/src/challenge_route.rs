//! Modified Meridian routes bind position and terminal time to one admitted,
//! ordered movement stream. Baseline visit proofs retain their frozen format.
use crate::{
    decode_challenge_payload, invalid_module, ReplayError, ReplayEvent, ReplayEventKind as Kind,
};
use revenant_challenges::{ChallengeRun, Command, ContractId, Objective, Rules};
use serde::{Deserialize, Serialize};

pub const PREFIX: &str = "challenge-route-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteEvidence {
    pub run_id: u64,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub actor_id: u64,
    pub position: [i32; 3],
    pub checkpoint: Option<Objective>,
}

impl RouteEvidence {
    #[must_use]
    pub const fn kind(&self) -> Kind {
        Kind::FieldActivity
    }

    #[must_use]
    pub fn command(&self) -> Option<Command> {
        self.checkpoint.map(|objective| Command::Confirm {
            run_id: self.run_id,
            objective,
        })
    }

    #[must_use]
    pub fn command_for(&self, rules: &Rules) -> Option<Command> {
        match self.checkpoint {
            Some(Objective::Returned) if rules.preset.time_goal_ms(rules.contract).is_some() => {
                Some(Command::ConfirmTimed {
                    run_id: self.run_id,
                    objective: Objective::Returned,
                    elapsed_ms: self.elapsed_ms,
                })
            }
            _ => self.command(),
        }
    }

    /// # Errors
    /// Rejects absent identity, unbounded clocks and oversized proof data.
    pub fn encode(&self) -> Result<String, ReplayError> {
        if self.run_id == 0
            || self.sequence == 0
            || self.actor_id == 0
            || self.elapsed_ms > revenant_challenges::MAX_REVISION
        {
            return Err(invalid_module("route evidence identity or clock invalid"));
        }
        let encoded = format!(
            "{PREFIX}{}",
            serde_json::to_string(self).map_err(|e| invalid_module(e.to_string()))?
        );
        if encoded.len() > 1024 {
            return Err(invalid_module("route evidence exceeds bound"));
        }
        Ok(encoded)
    }

    /// # Errors
    /// Rejects malformed, unversioned or oversized evidence.
    pub fn decode(encoded: &str) -> Result<Self, ReplayError> {
        if encoded.len() > 1024 {
            return Err(invalid_module("route evidence exceeds bound"));
        }
        let body = encoded
            .strip_prefix(PREFIX)
            .ok_or_else(|| invalid_module("route evidence version missing"))?;
        let value: Self = serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))?;
        value.encode()?;
        Ok(value)
    }
}

pub(super) fn validate(events: &[ReplayEvent], run: &ChallengeRun) -> Result<(), ReplayError> {
    let invalid = || invalid_module("route stream differs from admitted path, objective or time");
    let anchor = events.first().ok_or_else(invalid)?;
    if run.rules.contract != ContractId::MeridianCircuit
        || run.rules.preset.is_baseline()
        || anchor.kind != Kind::ChallengeCheckpoint
    {
        return Err(invalid());
    }
    let mut position = run.rules.preset.meridian_entrance().ok_or_else(invalid)?;
    let mut objectives = 0;
    let mut elapsed = 0;
    let mut sequence = 0;
    for (index, event) in events.iter().enumerate().skip(1) {
        if event.kind == Kind::ChallengeCheckpoint {
            continue;
        }
        let proof = RouteEvidence::decode(&event.payload)?;
        sequence += 1;
        if event.kind != Kind::FieldActivity
            || event.actor_id != anchor.actor_id
            || event.actor_id != Some(proof.actor_id)
            || proof.run_id != run.run_id
            || proof.sequence != sequence
            || proof.elapsed_ms < elapsed
            || proof.position == position
            || !revenant_activities::meridian::walkable(proof.position)
            || !revenant_activities::meridian::valid_step(position, proof.position)
            || proof.checkpoint != run.rules.visit_objective(objectives, proof.position)
        {
            return Err(invalid());
        }
        position = proof.position;
        elapsed = proof.elapsed_ms;
        if let Some(objective) = proof.checkpoint {
            let checkpoint = events
                .get(index + 1)
                .filter(|e| e.kind == Kind::ChallengeCheckpoint)
                .ok_or_else(invalid)?;
            let payload = decode_challenge_payload(&checkpoint.payload)?;
            if payload.proof_event_id != Some(event.id)
                || Some(payload.command) != proof.command_for(&run.rules)
            {
                return Err(invalid());
            }
            objectives |= run.rules.objective_bit(objective).ok_or_else(invalid)?;
        }
    }
    if objectives != run.objectives {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn verify_terminal(
    prefix: &[ReplayEvent],
    proof: &ReplayEvent,
    command: &Command,
    run: &ChallengeRun,
) -> Result<(), ReplayError> {
    let evidence = RouteEvidence::decode(&proof.payload)?;
    if prefix.last() != Some(proof)
        || proof.kind != Kind::FieldActivity
        || evidence.command_for(&run.rules).as_ref() != Some(command)
        || evidence.run_id != run.run_id
        || proof.actor_id != Some(evidence.actor_id)
    {
        return Err(invalid_module(
            "route objective lacks its adjacent movement proof",
        ));
    }
    Ok(())
}
