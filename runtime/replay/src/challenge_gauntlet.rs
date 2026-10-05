//! One ordered attempt wrapping the retained encounter evidence and transfers.
use revenant_challenges::{gauntlet::Stage, Command, EndReason, Objective};
use serde::{Deserialize, Serialize};

use crate::{
    invalid_module, EliteEvidence, EliteTransition, PrismEvidence, PrismTransition, ReplayError,
    ReplayEvent, ReplayEventKind as Kind, SupportEvidence, SupportTransition,
};

mod validate;
pub(super) use validate::validate;

pub const PREFIX: &str = "challenge-gauntlet-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum GauntletAction {
    Support(SupportEvidence),
    Elite(EliteEvidence),
    Prism(PrismEvidence),
    Move { position: [i32; 3] },
    Observe,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GauntletEffect {
    pub cleared: Option<Stage>,
    pub defeated: bool,
    pub recovered_health: u32,
    pub next_stage: Option<Stage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GauntletEvidence {
    pub run_id: u64,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub actor_id: u64,
    pub action: GauntletAction,
    pub effect: GauntletEffect,
}

impl GauntletEvidence {
    #[must_use]
    pub fn command(&self) -> Option<Command> {
        if self.effect.defeated {
            return Some(Command::End {
                run_id: self.run_id,
                reason: EndReason::Defeated,
            });
        }
        self.effect.cleared.map(|stage| Command::Confirm {
            run_id: self.run_id,
            objective: match stage {
                Stage::Support => Objective::GauntletSupportCleared,
                Stage::Bastion => Objective::GauntletBastionCleared,
                Stage::Prism => Objective::GauntletPrismCleared,
            },
        })
    }

    #[must_use]
    pub fn command_for(&self, rules: &revenant_challenges::Rules) -> Option<Command> {
        match self.command() {
            Some(Command::Confirm {
                run_id,
                objective: Objective::GauntletPrismCleared,
            }) if rules.preset.time_goal_ms(rules.contract).is_some() => {
                Some(Command::ConfirmTimed {
                    run_id,
                    objective: Objective::GauntletPrismCleared,
                    elapsed_ms: self.elapsed_ms,
                })
            }
            other => other,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> Kind {
        match &self.action {
            GauntletAction::Support(SupportEvidence {
                transition: SupportTransition::Started { .. },
                ..
            })
            | GauntletAction::Elite(EliteEvidence {
                transition: EliteTransition::Started { .. },
                ..
            }) => Kind::EnemySpawned,
            GauntletAction::Prism(PrismEvidence {
                transition: PrismTransition::Started { .. },
                ..
            }) => Kind::BossSpawned,
            GauntletAction::Support(SupportEvidence {
                transition:
                    SupportTransition::Attack {
                        health_after: 0, ..
                    },
                ..
            })
            | GauntletAction::Elite(EliteEvidence {
                transition:
                    EliteTransition::Attack {
                        health_after: 0, ..
                    },
                ..
            })
            | GauntletAction::Prism(PrismEvidence {
                transition:
                    PrismTransition::Attack {
                        health_after: 0, ..
                    },
                ..
            }) => Kind::EnemyDied,
            _ => Kind::FieldActivity,
        }
    }

    /// # Errors
    /// Rejects absent attempt/actor/sequence identities or oversized evidence.
    pub fn encode(&self) -> Result<String, ReplayError> {
        if self.run_id == 0 || self.sequence == 0 || self.actor_id == 0 {
            return Err(invalid_module("gauntlet evidence identity missing"));
        }
        let encoded = format!(
            "{PREFIX}{}",
            serde_json::to_string(self).map_err(|e| invalid_module(e.to_string()))?
        );
        if encoded.len() > 4096 {
            return Err(invalid_module("gauntlet evidence exceeds bound"));
        }
        Ok(encoded)
    }

    /// # Errors
    /// Rejects unversioned, malformed or oversized evidence.
    pub fn decode(encoded: &str) -> Result<Self, ReplayError> {
        if encoded.len() > 4096 {
            return Err(invalid_module("gauntlet evidence exceeds bound"));
        }
        let body = encoded
            .strip_prefix(PREFIX)
            .ok_or_else(|| invalid_module("gauntlet evidence revision missing"))?;
        let proof: Self = serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))?;
        proof.encode()?;
        Ok(proof)
    }
}

impl GauntletAction {
    /// Retained encounter adapters still generate their original typed evidence.
    /// # Errors
    /// Rejects unrelated or malformed payloads; never wraps arbitrary world rows.
    pub fn from_combat_payload(payload: &str) -> Result<Self, ReplayError> {
        if payload.len() <= 2048 {
            if let Some(body) = payload.strip_prefix(crate::support::PREFIX) {
                return serde_json::from_str(body)
                    .map(Self::Support)
                    .map_err(|e| invalid_module(e.to_string()));
            }
            if let Some(body) = payload.strip_prefix(crate::elite::PREFIX) {
                return serde_json::from_str(body)
                    .map(Self::Elite)
                    .map_err(|e| invalid_module(e.to_string()));
            }
            if let Some(body) = payload.strip_prefix(crate::prism::PREFIX) {
                return serde_json::from_str(body)
                    .map(Self::Prism)
                    .map_err(|e| invalid_module(e.to_string()));
            }
        }
        Err(invalid_module(
            "gauntlet requires retained encounter evidence",
        ))
    }

    fn payload(&self) -> Result<String, ReplayError> {
        match self {
            Self::Support(e) => crate::encode_support_evidence(e),
            Self::Elite(e) => crate::encode_elite_evidence(e),
            Self::Prism(e) => crate::encode_prism_evidence(e),
            _ => return Err(invalid_module("gauntlet action is not combat")),
        }
        .map_err(|e| invalid_module(e.to_string()))
    }
}

pub(super) fn verify_terminal(
    prefix: &[ReplayEvent],
    proof: &ReplayEvent,
    command: &Command,
) -> Result<(), ReplayError> {
    let evidence = GauntletEvidence::decode(&proof.payload)?;
    let admission = prefix
        .iter()
        .find(|e| e.kind == Kind::ChallengeCheckpoint)
        .ok_or_else(|| invalid_module("gauntlet admission missing"))?;
    let payload = crate::decode_challenge_payload(&admission.payload)?;
    let rules = &payload
        .after
        .active
        .as_ref()
        .ok_or_else(|| invalid_module("gauntlet run missing"))?
        .rules;
    if prefix.last() != Some(proof)
        || evidence.command_for(rules).as_ref() != Some(command)
        || proof.kind != evidence.kind()
        || proof.actor_id != Some(evidence.actor_id)
    {
        return Err(invalid_module(
            "gauntlet checkpoint lacks its adjacent encounter terminal",
        ));
    }
    Ok(())
}
