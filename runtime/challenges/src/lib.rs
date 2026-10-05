//! Pure challenge transitions, separate from campaign saves and equipment
//! mutations. Only the authenticated gateway may supply equipment and confirmed
//! objectives; persistence and replay independently validate the linked evidence.
use std::fmt;

use revenant_inventory::weapon_profile;
use revenant_modules::{
    canonicalize_loadout, resolve_build, BaseCombatProfile, ModuleId, BUILD_CATALOG_REVISION,
};
use serde::{Deserialize, Serialize};

mod catalog;
pub mod gauntlet;
pub mod mastery;
pub mod modifiers;
pub mod recovery;
pub mod signal;
pub mod survival;
mod transition;
pub use catalog::{
    ContractId, Objective, Rules, ELITE_REVISION, EXPANDED_REVISION, GAMEPLAY_REVISION,
    GAUNTLET_REVISION, MODIFIER_REVISION, PRISM_REVISION, REVISION, SIGNAL_REVISION,
    SURVIVAL_REVISION,
};
pub use transition::{apply, Command, EndReason, Transition};

pub const MAX_REVISION: u64 = i64::MAX as u64;

/// Read from the character's accepted loadout under its persistence lock.
/// This is a record of equipment, never an inventory grant or a client request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Equipment {
    pub loadout_revision: u64,
    pub catalog_revision: String,
    pub weapon_id: String,
    pub modules: Vec<ModuleId>,
}

impl Equipment {
    /// # Errors
    /// Rejects unavailable catalogs, weapons, duplicate/noncanonical modules and
    /// invalid combat profiles. Ownership must be checked by persistence.
    pub fn validate(&self) -> Result<(), ChallengeError> {
        let invalid = || ChallengeError::InvalidEquipment;
        if self.loadout_revision > MAX_REVISION
            || self.catalog_revision != BUILD_CATALOG_REVISION
            || canonicalize_loadout(&self.modules).map_err(|_| invalid())? != self.modules
        {
            return Err(invalid());
        }
        let weapon = weapon_profile(&self.weapon_id).ok_or_else(invalid)?;
        resolve_build(
            &self.catalog_revision,
            BaseCombatProfile {
                damage: weapon.damage,
                range: weapon.range,
                cooldown_ms: weapon.cooldown_ms,
                max_health: 100,
            },
            &self.modules,
        )
        .map_err(|_| invalid())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeRun {
    /// Character-scoped, derived from the start transition's revision. Never
    /// supplied by the client or reused by retry, even across old terminals.
    pub run_id: u64,
    pub rules: Rules,
    pub equipment: Equipment,
    pub objectives: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

impl ChallengeRun {
    fn validate(&self, revision: u64, completed: bool) -> Result<(), ChallengeError> {
        let mask = self.rules.contract.complete_mask();
        if self.run_id == 0
            || self.run_id > revision
            || u64::from(self.objectives.count_ones()) > revision - self.run_id
            || self
                .rules
                .contract
                .rules_with_preset(self.rules.preset)
                .as_ref()
                != Ok(&self.rules)
            || self.objectives & !mask != 0
            || self.elapsed_ms.is_some_and(|t| t > MAX_REVISION)
            || self.elapsed_ms.is_some()
                != (completed
                    && self
                        .rules
                        .preset
                        .time_goal_ms(self.rules.contract)
                        .is_some())
            || (self.objectives == mask) != completed
            || (matches!(
                self.rules.contract,
                ContractId::CoolantRecovery | ContractId::LastReserve | ContractId::RelayGauntlet
            ) && ![0, 1, 3, 7].contains(&self.objectives))
            || (self.rules.contract == ContractId::MeridianCircuit
                && self.objectives & 8 != 0
                && !completed)
        {
            return Err(ChallengeError::InvalidState);
        }
        self.equipment.validate()?;
        if self.rules.contract == ContractId::DistantSignal {
            signal::SignalRun::new(&self.equipment)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Completed,
    Abandoned,
    Defeated,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptResult {
    pub run: ChallengeRun,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeState {
    pub revision: u64,
    pub active: Option<ChallengeRun>,
    pub last_result: Option<AttemptResult>,
    /// Bounded first-completion records per exact rule/preset identity. Optional
    /// performance distinctions need separate proofs; repeated clears do not farm.
    pub records: Vec<ChallengeRun>,
}

impl ChallengeState {
    /// Whether this save can be interpreted by the requested published policy.
    #[must_use]
    pub fn supports_policy(&self, policy: &str) -> bool {
        [
            REVISION,
            EXPANDED_REVISION,
            ELITE_REVISION,
            PRISM_REVISION,
            SIGNAL_REVISION,
            SURVIVAL_REVISION,
            GAUNTLET_REVISION,
            MODIFIER_REVISION,
        ]
        .contains(&policy)
            && self
                .active
                .iter()
                .chain(self.last_result.iter().map(|result| &result.run))
                .chain(self.records.iter())
                .all(|run| run.rules.available_in(policy))
    }

    /// # Errors
    /// Rejects malformed records, mismatched rule identities and impossible runs.
    pub fn validate(&self) -> Result<(), ChallengeError> {
        if self.revision > MAX_REVISION
            || self.records.len() > 16 // Eight baselines and eight authored variants.
            || (self.revision > 0 && self.active.is_none() && self.last_result.is_none())
        {
            return Err(ChallengeError::InvalidState);
        }
        if let Some(active) = &self.active {
            active.validate(self.revision, false)?;
        }
        if let Some(result) = &self.last_result {
            result
                .run
                .validate(self.revision, result.outcome == Outcome::Completed)?;
            if self
                .active
                .as_ref()
                .is_some_and(|r| r.run_id <= result.run.run_id)
                || (result.outcome == Outcome::Completed
                    && !self.records.iter().any(|r| r.rules == result.run.rules))
            {
                return Err(ChallengeError::InvalidState);
            }
        }
        for (index, record) in self.records.iter().enumerate() {
            record.validate(self.revision, true)?;
            if self.records[..index]
                .iter()
                .any(|r| r.rules == record.rules || r.run_id == record.run_id)
                || self
                    .active
                    .as_ref()
                    .is_some_and(|r| r.run_id <= record.run_id)
                || self
                    .last_result
                    .as_ref()
                    .is_none_or(|r| r.run.run_id < record.run_id)
                || self
                    .last_result
                    .as_ref()
                    .is_some_and(|r| r.run.run_id == record.run_id && r.run != *record)
            {
                return Err(ChallengeError::InvalidState);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeError {
    UnsupportedRevision,
    StaleRevision,
    RevisionOverflow,
    InvalidState,
    InvalidEquipment,
    InvalidModifiers,
    ActiveRun,
    WrongRun,
    WrongObjective,
}

impl fmt::Display for ChallengeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "challenge transition rejected: {self:?}")
    }
}

impl std::error::Error for ChallengeError {}

#[cfg(test)]
mod tests;
