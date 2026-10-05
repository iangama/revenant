//! Closed, provisional modifier presets for M37-C. These are not admitted by
//! a published challenge policy until replay and gateway integration is ready.
use serde::{Deserialize, Serialize};

use crate::{gauntlet::Stage, ChallengeError, ContractId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    AlternateApproach,
    ReinforcementOrder,
    Recovery,
    TimePressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    WestApproach,
    BastionFirst,
    SingleReserve,
    Pace,
}

impl Modifier {
    #[must_use]
    pub const fn family(self) -> Family {
        match self {
            Self::WestApproach => Family::AlternateApproach,
            Self::BastionFirst => Family::ReinforcementOrder,
            Self::SingleReserve => Family::Recovery,
            Self::Pace => Family::TimePressure,
        }
    }
}

/// An explicit allowed-pair list, not arbitrary combinations. IDs will also
/// distinguish completion records when the new policy is published.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    #[default]
    Baseline,
    WestApproach,
    BastionFirst,
    SingleReserve,
    Pace,
    WestApproachPace,
    BastionFirstSingleReserve,
    SingleReservePace,
}

impl Preset {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::WestApproach => "west_approach",
            Self::BastionFirst => "bastion_first",
            Self::SingleReserve => "single_reserve",
            Self::Pace => "pace",
            Self::WestApproachPace => "west_approach_pace",
            Self::BastionFirstSingleReserve => "bastion_first_single_reserve",
            Self::SingleReservePace => "single_reserve_pace",
        }
    }

    #[must_use]
    pub const fn is_baseline(&self) -> bool {
        matches!(self, Self::Baseline)
    }

    pub const ALL: [Self; 8] = [
        Self::Baseline,
        Self::WestApproach,
        Self::BastionFirst,
        Self::SingleReserve,
        Self::Pace,
        Self::WestApproachPace,
        Self::BastionFirstSingleReserve,
        Self::SingleReservePace,
    ];

    #[must_use]
    pub const fn modifiers(self) -> &'static [Modifier] {
        use Modifier::{BastionFirst, Pace, SingleReserve, WestApproach};
        match self {
            Self::Baseline => &[],
            Self::WestApproach => &[WestApproach],
            Self::BastionFirst => &[BastionFirst],
            Self::SingleReserve => &[SingleReserve],
            Self::Pace => &[Pace],
            Self::WestApproachPace => &[WestApproach, Pace],
            Self::BastionFirstSingleReserve => &[BastionFirst, SingleReserve],
            Self::SingleReservePace => &[SingleReserve, Pace],
        }
    }

    #[must_use]
    pub const fn available_for(self, contract: ContractId) -> bool {
        match self {
            Self::Baseline => true,
            Self::WestApproach | Self::WestApproachPace => {
                matches!(contract, ContractId::MeridianCircuit)
            }
            Self::BastionFirst
            | Self::SingleReserve
            | Self::BastionFirstSingleReserve
            | Self::SingleReservePace => matches!(contract, ContractId::RelayGauntlet),
            Self::Pace => matches!(
                contract,
                ContractId::MeridianCircuit | ContractId::RelayGauntlet
            ),
        }
    }

    /// Input ordering has no rule significance. Duplicate modifiers, unsupported
    /// contracts, unlisted pairs and triples cannot produce a preset.
    /// # Errors
    /// Rejects any selection outside the authored catalog.
    pub fn select(contract: ContractId, modifiers: &[Modifier]) -> Result<Self, ChallengeError> {
        Self::ALL
            .into_iter()
            .find(|p| {
                p.available_for(contract)
                    && p.modifiers().len() == modifiers.len()
                    && p.modifiers().iter().all(|m| modifiers.contains(m))
            })
            .ok_or(ChallengeError::InvalidModifiers)
    }

    #[must_use]
    pub fn has(self, modifier: Modifier) -> bool {
        self.modifiers().contains(&modifier)
    }

    /// Optional distinction only: no controller uses this as a defeat deadline.
    /// Targets remain provisional until the rendered trial.
    #[must_use]
    pub fn time_goal_ms(self, contract: ContractId) -> Option<u64> {
        if !self.available_for(contract) || !self.has(Modifier::Pace) {
            return None;
        }
        match contract {
            ContractId::MeridianCircuit => Some(60_000),
            ContractId::RelayGauntlet => Some(180_000),
            _ => None,
        }
    }

    #[must_use]
    pub fn gauntlet_stages(self) -> [Stage; 3] {
        if self.has(Modifier::BastionFirst) {
            [Stage::Bastion, Stage::Support, Stage::Prism]
        } else {
            [Stage::Support, Stage::Bastion, Stage::Prism]
        }
    }

    /// The alternate route uses a western landing near the log, with both
    /// galleries still required. The arrival is also the return point.
    #[must_use]
    pub fn meridian_entrance(self) -> Option<[i32; 3]> {
        self.available_for(ContractId::MeridianCircuit).then(|| {
            if self.has(Modifier::WestApproach) {
                [-31, 0, 0]
            } else {
                [-16, 0, 0]
            }
        })
    }
}

#[cfg(test)]
mod records_tests;
#[cfg(test)]
mod tests;
