//! Stage and recovery policy for Relay Gauntlet.
//! Encounter outcomes and exit occupancy must come from the authoritative
//! retained encounter validators; this controller does not prove a kill.
use revenant_inventory::weapon_profile;
use revenant_modules::{resolve_build, BaseCombatProfile};

use crate::modifiers::{Modifier, Preset};
use crate::{ChallengeError, Equipment};
use serde::{Deserialize, Serialize};

pub const REVISION: &str = "relay-gauntlet-v1";

pub const TRANSFER_HOLD_MS: u64 = 1_200;
pub const TRANSFER_HEALTH: u32 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Support,
    Bastion,
    Prism,
}

impl Stage {
    #[must_use]
    pub const fn entrance(self) -> [i32; 3] {
        match self {
            Self::Support => revenant_ai::support::ENTRANCE,
            Self::Bastion => revenant_ai::elite::EliteComposition::BastionLink.entrance(),
            Self::Prism => revenant_ai::prism::ENTRANCE,
        }
    }

    #[must_use]
    pub fn walkable(self, position: [i32; 3]) -> bool {
        match self {
            Self::Support => revenant_ai::lancer::arena_contains(position),
            Self::Bastion => revenant_ai::bulwark::arena_contains(position),
            Self::Prism => revenant_ai::prism::arena_contains(position),
        }
    }

    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Support => Some(Self::Bastion),
            Self::Bastion => Some(Self::Prism),
            Self::Prism => None,
        }
    }

    /// Optional reserve is adjacent to, but distinct from, the transfer exit.
    #[must_use]
    pub const fn reserve_position(self) -> Option<[i32; 3]> {
        match self {
            Self::Support => Some([3, 0, 6]),
            Self::Bastion => Some([6, 0, -6]),
            Self::Prism => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Combat(Stage),
    Transfer(Stage),
    Completed,
    Defeated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GauntletRun {
    preset: Preset,
    phase: Phase,
    health: u32,
    max_health: u32,
    last_elapsed_ms: u64,
    transfer_started_ms: Option<u64>,
    reserve_started_ms: Option<u64>,
    reserve_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferUpdate {
    pub run: GauntletRun,
    pub recovered_health: u32,
    pub next_stage: Option<Stage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    ClockReversed,
    WrongPhase,
    InvalidHealth,
}

impl GauntletRun {
    /// # Errors
    /// Rejects equipment outside the admitted catalog or invalid builds.
    pub fn new(equipment: &Equipment) -> Result<Self, ChallengeError> {
        Self::with_preset(equipment, Preset::Baseline)
    }

    /// Pure M37-C controller; live admissions continue to use `new` until the
    /// versioned evidence and client projection support modified runs.
    /// # Errors
    /// Rejects unsupported presets or invalid equipment/builds.
    pub fn with_preset(equipment: &Equipment, preset: Preset) -> Result<Self, ChallengeError> {
        if !preset.available_for(crate::ContractId::RelayGauntlet) {
            return Err(ChallengeError::InvalidModifiers);
        }
        equipment.validate()?;
        let weapon =
            weapon_profile(&equipment.weapon_id).ok_or(ChallengeError::InvalidEquipment)?;
        let build = resolve_build(
            &equipment.catalog_revision,
            BaseCombatProfile {
                damage: weapon.damage,
                range: weapon.range,
                cooldown_ms: weapon.cooldown_ms,
                max_health: 100,
            },
            &equipment.modules,
        )
        .map_err(|_| ChallengeError::InvalidEquipment)?;
        Ok(Self {
            preset,
            phase: Phase::Combat(preset.gauntlet_stages()[0]),
            health: build.profile.max_health,
            max_health: build.profile.max_health,
            last_elapsed_ms: 0,
            transfer_started_ms: None,
            reserve_started_ms: None,
            reserve_used: false,
        })
    }

    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    /// Health supplied to the current encounter, or its confirmed terminal
    /// health during transfer. Live damage remains in the encounter controller.
    #[must_use]
    pub const fn health(&self) -> u32 {
        self.health
    }
    #[must_use]
    pub const fn max_health(&self) -> u32 {
        self.max_health
    }

    #[must_use]
    pub const fn reserve_used(&self) -> bool {
        self.reserve_used
    }

    #[must_use]
    pub fn reserve_remaining_ms(&self) -> Option<u64> {
        self.reserve_started_ms.map(|t| {
            t.saturating_add(TRANSFER_HOLD_MS)
                .saturating_sub(self.last_elapsed_ms)
        })
    }

    /// None for untimed or unfinished runs; a late clear still completes.
    #[must_use]
    pub fn pace_met(&self) -> Option<bool> {
        if self.phase != Phase::Completed {
            return None;
        }
        self.preset
            .time_goal_ms(crate::ContractId::RelayGauntlet)
            .map(|goal| self.last_elapsed_ms <= goal)
    }

    fn next_stage(&self, current: Stage) -> Option<Stage> {
        self.preset
            .gauntlet_stages()
            .windows(2)
            .find_map(|pair| (pair[0] == current).then_some(pair[1]))
    }

    #[must_use]
    pub fn transfer_deadline_ms(&self) -> Option<u64> {
        self.transfer_started_ms
            .map(|t| t.saturating_add(TRANSFER_HOLD_MS))
    }

    #[must_use]
    pub fn transfer_remaining_ms(&self) -> Option<u64> {
        self.transfer_deadline_ms()
            .map(|t| t.saturating_sub(self.last_elapsed_ms))
    }

    /// Accept only a proved clear with positive health, or proved player defeat
    /// with zero health. Finishing an encounter never performs recovery.
    /// # Errors
    /// Rejects stale/out-of-order stages, reversed time and impossible healing.
    pub fn finish_encounter(
        &self,
        elapsed_ms: u64,
        stage: Stage,
        health: u32,
    ) -> Result<Self, Error> {
        if elapsed_ms < self.last_elapsed_ms {
            return Err(Error::ClockReversed);
        }
        if self.phase != Phase::Combat(stage) {
            return Err(Error::WrongPhase);
        }
        if health > self.health {
            return Err(Error::InvalidHealth);
        }
        let mut next = self.clone();
        next.last_elapsed_ms = elapsed_ms;
        next.health = health;
        next.phase = if health == 0 {
            Phase::Defeated
        } else if self.next_stage(stage).is_some() {
            Phase::Transfer(stage)
        } else {
            Phase::Completed
        };
        Ok(next)
    }

    /// Shared position-derived recovery choice used by replay and the gateway.
    /// # Errors
    /// Rejects observations outside a transfer or with reversed time.
    pub fn observe_at(&self, elapsed_ms: u64, position: [i32; 3]) -> Result<TransferUpdate, Error> {
        let Phase::Transfer(stage) = self.phase else {
            return Err(Error::WrongPhase);
        };
        if self.preset.has(Modifier::SingleReserve) && stage.reserve_position() == Some(position) {
            self.observe_reserve(elapsed_ms, true)
        } else {
            self.observe_transfer(elapsed_ms, position == stage.entrance())
        }
    }

    #[must_use]
    pub fn observation_deadline_ms(&self) -> Option<u64> {
        self.transfer_deadline_ms().or_else(|| {
            self.reserve_started_ms
                .map(|t| t.saturating_add(TRANSFER_HOLD_MS))
        })
    }

    /// Observe the accepted player position at a cleared arena's exit.
    /// Leaving resets the hold. Baseline transfers restore health; the single
    /// reserve preset instead requires a separate, optional reserve visit.
    /// # Errors
    /// Rejects observations outside a pending transfer or with reversed time.
    pub fn observe_transfer(
        &self,
        elapsed_ms: u64,
        at_exit: bool,
    ) -> Result<TransferUpdate, Error> {
        if elapsed_ms < self.last_elapsed_ms {
            return Err(Error::ClockReversed);
        }
        let Phase::Transfer(cleared) = self.phase else {
            return Err(Error::WrongPhase);
        };
        let next_stage = self.next_stage(cleared).ok_or(Error::WrongPhase)?;
        let mut update = TransferUpdate {
            run: self.clone(),
            recovered_health: 0,
            next_stage: None,
        };
        update.run.last_elapsed_ms = elapsed_ms;
        update.run.reserve_started_ms = None;
        if at_exit {
            let started = *update.run.transfer_started_ms.get_or_insert(elapsed_ms);
            if elapsed_ms - started >= TRANSFER_HOLD_MS {
                if !self.preset.has(Modifier::SingleReserve) {
                    update.recovered_health = TRANSFER_HEALTH.min(self.max_health - self.health);
                }
                update.run.health += update.recovered_health;
                update.run.phase = Phase::Combat(next_stage);
                update.run.transfer_started_ms = None;
                update.next_stage = Some(next_stage);
            }
        } else {
            update.run.transfer_started_ms = None;
        }
        Ok(update)
    }

    /// Hold the reserve pad for 1.2 seconds after either early-stage clear.
    /// A positive heal spends the single reserve; full health does not waste it.
    /// Movement to the exit cancels this hold and starts its separate transfer.
    /// # Errors
    /// Rejects reversed clocks, combat/terminal phases and presets without a reserve.
    pub fn observe_reserve(
        &self,
        elapsed_ms: u64,
        at_reserve: bool,
    ) -> Result<TransferUpdate, Error> {
        if elapsed_ms < self.last_elapsed_ms {
            return Err(Error::ClockReversed);
        }
        if !matches!(self.phase, Phase::Transfer(_)) || !self.preset.has(Modifier::SingleReserve) {
            return Err(Error::WrongPhase);
        }
        let mut update = TransferUpdate {
            run: self.clone(),
            recovered_health: 0,
            next_stage: None,
        };
        update.run.last_elapsed_ms = elapsed_ms;
        update.run.transfer_started_ms = None;
        if at_reserve && !self.reserve_used && self.health < self.max_health {
            let started = *update.run.reserve_started_ms.get_or_insert(elapsed_ms);
            if elapsed_ms - started >= TRANSFER_HOLD_MS {
                update.recovered_health = TRANSFER_HEALTH.min(self.max_health - self.health);
                update.run.health += update.recovered_health;
                update.run.reserve_used = true;
                update.run.reserve_started_ms = None;
            }
        } else {
            update.run.reserve_started_ms = None;
        }
        Ok(update)
    }
}

#[cfg(test)]
mod tests;
