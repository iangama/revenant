//! Complete server-owned movement and combat stream for the distant firing lane.
use revenant_challenges::{
    signal::{Action, Effect, SignalRun, Terminal},
    ChallengeRun, Command, ContractId, EndReason, Objective,
};
use serde::{Deserialize, Serialize};

use crate::{
    decode_challenge_payload, invalid_module, ReplayError, ReplayEvent, ReplayEventKind as Kind,
};

pub const PREFIX: &str = "challenge-signal-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalEvidence {
    pub run_id: u64,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub enemy_id: u64,
    /// None is the initial spawn; later entries always contain an accepted action.
    pub action: Option<Action>,
    pub effect: Effect,
}

impl SignalEvidence {
    #[must_use]
    pub fn command(&self) -> Option<Command> {
        self.effect.terminal.map(|terminal| match terminal {
            Terminal::Completed => Command::Confirm {
                run_id: self.run_id,
                objective: Objective::SentinelCleared,
            },
            Terminal::Defeated => Command::End {
                run_id: self.run_id,
                reason: EndReason::Defeated,
            },
        })
    }

    #[must_use]
    pub const fn kind(&self) -> Kind {
        if self.action.is_none() {
            Kind::EnemySpawned
        } else if matches!(self.effect.terminal, Some(Terminal::Completed)) {
            Kind::EnemyDied
        } else {
            Kind::FieldActivity
        }
    }

    #[must_use]
    pub const fn actor(&self, player: u64) -> u64 {
        if matches!(self.kind(), Kind::FieldActivity) {
            player
        } else {
            self.enemy_id
        }
    }

    /// # Errors
    /// Rejects missing identities and unbounded server evidence.
    pub fn encode(&self) -> Result<String, ReplayError> {
        if self.run_id == 0 || self.sequence == 0 || self.enemy_id == 0 {
            return Err(invalid_module(
                "signal proof has no attempt, sequence or enemy",
            ));
        }
        let encoded = format!(
            "{PREFIX}{}",
            serde_json::to_string(self).map_err(|e| invalid_module(e.to_string()))?
        );
        if encoded.len() > 1024 {
            return Err(invalid_module("signal proof exceeds its bound"));
        }
        Ok(encoded)
    }

    /// # Errors
    /// Rejects malformed, unversioned and oversized evidence.
    pub fn decode(encoded: &str) -> Result<Self, ReplayError> {
        if encoded.len() > 1024 {
            return Err(invalid_module("signal proof exceeds its bound"));
        }
        let body = encoded
            .strip_prefix(PREFIX)
            .ok_or_else(|| invalid_module("signal proof revision missing"))?;
        let proof: Self = serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))?;
        proof.encode()?;
        Ok(proof)
    }
}

pub(super) fn verify_terminal(
    prefix: &[ReplayEvent],
    proof: &ReplayEvent,
    command: &Command,
    player: Option<u64>,
) -> Result<(), ReplayError> {
    let evidence = SignalEvidence::decode(&proof.payload)?;
    if prefix.last() != Some(proof)
        || evidence.command().as_ref() != Some(command)
        || proof.kind != evidence.kind()
        || player.map(|id| evidence.actor(id)) != proof.actor_id
    {
        return Err(invalid_module(
            "signal terminal requires its adjacent fatal event",
        ));
    }
    Ok(())
}

pub(super) fn validate(events: &[ReplayEvent], run: &ChallengeRun) -> Result<(), ReplayError> {
    let invalid =
        || invalid_module("signal stream differs from its admitted build, movement or combat");
    let anchor = events.first().ok_or_else(invalid)?;
    if run.rules.contract != ContractId::DistantSignal || anchor.kind != Kind::ChallengeCheckpoint {
        return Err(invalid());
    }
    let player = anchor.actor_id.ok_or_else(invalid)?;
    let mut world = SignalRun::new(&run.equipment).map_err(|_| invalid())?;
    let mut sequence = 0;
    let mut enemy = None;
    let mut completed = false;
    for (index, event) in events.iter().enumerate().skip(1) {
        if event.kind == Kind::ChallengeCheckpoint {
            continue;
        }
        let proof = SignalEvidence::decode(&event.payload)?;
        sequence += 1;
        if proof.run_id != run.run_id
            || proof.sequence != sequence
            || proof.enemy_id == player
            || proof.kind() != event.kind
            || event.actor_id != Some(proof.actor(player))
        {
            return Err(invalid());
        }
        if sequence == 1 {
            if proof.action.is_some() || proof.elapsed_ms != 0 || proof.effect != Effect::default()
            {
                return Err(invalid());
            }
            enemy = Some(proof.enemy_id);
            continue;
        }
        if enemy != Some(proof.enemy_id) {
            return Err(invalid());
        }
        let next = world
            .apply(proof.elapsed_ms, proof.action.ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        if next.effect != proof.effect {
            return Err(invalid());
        }
        world = next.run;
        if let Some(command) = proof.command() {
            let checkpoint = events
                .get(index + 1)
                .filter(|e| e.kind == Kind::ChallengeCheckpoint)
                .ok_or_else(invalid)?;
            let entry = decode_challenge_payload(&checkpoint.payload)?;
            if entry.proof_event_id != Some(event.id) || entry.command != command {
                return Err(invalid());
            }
            completed = proof.effect.terminal == Some(Terminal::Completed);
        }
    }
    if run.objectives != u8::from(completed) {
        return Err(invalid());
    }
    Ok(())
}
