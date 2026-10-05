use revenant_replay::{
    EliteEvidence, EliteStep, EliteTransition, PrismEvidence, PrismStep, PrismTransition,
    SupportEvidence, SupportTransition,
};

use super::{invalid, ChallengePersistenceResult};

/// Combat adapters share the character lock, retry check and atomic writer.
#[derive(Clone, Copy)]
pub(super) enum Proof<'a> {
    Support(&'a SupportEvidence),
    Elite(&'a EliteEvidence),
    Prism {
        boss_id: u64,
        evidence: &'a PrismEvidence,
    },
}

impl Proof<'_> {
    pub(super) fn encode(self) -> ChallengePersistenceResult<String> {
        Ok(match self {
            Self::Support(evidence) => revenant_replay::encode_support_evidence(evidence)?,
            Self::Elite(evidence) => revenant_replay::encode_elite_evidence(evidence)?,
            Self::Prism { evidence, .. } => revenant_replay::encode_prism_evidence(evidence)?,
        })
    }

    pub(super) fn identity(self, player: i64) -> ChallengePersistenceResult<(&'static str, i64)> {
        match self {
            Self::Prism {
                boss_id,
                evidence:
                    PrismEvidence {
                        transition:
                            PrismTransition::Attack {
                                health_after: 0,
                                damage: 1..,
                                ..
                            },
                        ..
                    },
            } => Ok(("enemy_died", i64::try_from(boss_id)?)),
            Self::Support(SupportEvidence {
                transition:
                    SupportTransition::Attack {
                        target_id,
                        health_after: 0,
                        ..
                    },
                ..
            })
            | Self::Elite(EliteEvidence {
                transition:
                    EliteTransition::Attack {
                        target_id,
                        health_after: 0,
                        damage: 1..,
                        ..
                    },
                ..
            }) => Ok(("enemy_died", i64::try_from(*target_id)?)),
            Self::Prism {
                evidence:
                    PrismEvidence {
                        transition:
                            PrismTransition::Step {
                                step:
                                    PrismStep::Resolved {
                                        health_after: 0,
                                        damage: 1..,
                                        ..
                                    },
                                ..
                            },
                        ..
                    },
                ..
            }
            | Self::Support(SupportEvidence {
                transition:
                    SupportTransition::Charged {
                        health_after: 0,
                        damage: 1..,
                        ..
                    },
                ..
            })
            | Self::Elite(EliteEvidence {
                transition:
                    EliteTransition::Step {
                        step:
                            EliteStep::Slam {
                                health_after: 0,
                                damage: 1..,
                                ..
                            },
                        ..
                    },
                ..
            }) => Ok(("field_activity", player)),
            _ => Err(invalid("challenge atomic combat requires a fatal event")),
        }
    }
}
