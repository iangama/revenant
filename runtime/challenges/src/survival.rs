//! Last Reserve: charge two exposed relays, then return to cover alive.
//! The retained sentinel cannot be disabled here. A separate sheltered pad
//! restores a bounded amount of run-local health once; it never grants items.
use revenant_ai::sentinel::{
    cover_blocks_segment, SENTINEL_POSITION, SHOT_DAMAGE, SHOT_INTERVAL_MS,
};
use revenant_inventory::weapon_profile;
use revenant_modules::{resolve_build, BaseCombatProfile};
use serde::{Deserialize, Serialize};

use crate::{signal, ChallengeError, Equipment};

pub const REVISION: &str = "last-reserve-v1";
pub const SPAWN: [i32; 3] = signal::SPAWN;
pub const WEST_RELAY: [i32; 3] = [-8, 0, 0];
pub const EAST_RELAY: [i32; 3] = [0, 0, 0];
pub const RESERVE_PAD: [i32; 3] = [-4, 0, 5];
pub const CHARGE_MS: u64 = 3_600;
pub const RECOVERY_MS: u64 = 1_200;
pub const RECOVERY_HEALTH: u32 = 36;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Move { position: [i32; 3] },
    Observe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Checkpoint {
    WestCharged,
    EastCharged,
    Returned,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub incoming_damage: u32,
    pub recovered_health: u32,
    pub checkpoint: Option<Checkpoint>,
    pub defeated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    West,
    East,
    Return,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurvivalRun {
    position: [i32; 3],
    health: u32,
    max_health: u32,
    phase: Phase,
    reserve_used: bool,
    last_elapsed_ms: u64,
    next_shot_ms: u64,
    charge_started_ms: Option<u64>,
    recovery_started_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub run: SurvivalRun,
    pub effect: Effect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    ClockReversed,
    InvalidMovement,
    Terminal,
}

impl SurvivalRun {
    /// # Errors
    /// Rejects equipment outside the admitted M35 catalog. Weapon range is not
    /// an entry requirement: the objective is to survive, not kill the sentinel.
    pub fn new(equipment: &Equipment) -> Result<Self, ChallengeError> {
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
            position: SPAWN,
            health: build.profile.max_health,
            max_health: build.profile.max_health,
            phase: Phase::West,
            reserve_used: false,
            last_elapsed_ms: 0,
            next_shot_ms: SHOT_INTERVAL_MS,
            charge_started_ms: None,
            recovery_started_ms: None,
        })
    }

    #[must_use]
    pub const fn position(&self) -> [i32; 3] {
        self.position
    }
    #[must_use]
    pub const fn health(&self) -> u32 {
        self.health
    }
    #[must_use]
    pub const fn max_health(&self) -> u32 {
        self.max_health
    }
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    #[must_use]
    pub const fn reserve_used(&self) -> bool {
        self.reserve_used
    }

    #[must_use]
    pub fn charge_remaining_ms(&self) -> Option<u64> {
        (self.health > 0 && self.phase != Phase::Completed)
            .then_some(self.charge_started_ms)
            .flatten()
            .map(|t| CHARGE_MS.saturating_sub(self.last_elapsed_ms - t))
    }

    #[must_use]
    pub fn recovery_remaining_ms(&self) -> Option<u64> {
        (self.health > 0)
            .then_some(self.recovery_started_ms)
            .flatten()
            .map(|t| RECOVERY_MS.saturating_sub(self.last_elapsed_ms - t))
    }

    /// Next meaningful observation; avoids writing a journal entry every frame.
    #[must_use]
    pub fn next_observation_ms(&self) -> u64 {
        [
            Some(self.next_shot_ms),
            self.charge_started_ms.map(|t| t.saturating_add(CHARGE_MS)),
            self.recovery_started_ms
                .map(|t| t.saturating_add(RECOVERY_MS)),
        ]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(self.next_shot_ms)
    }

    /// A due shot resolves at the previous accepted position, before moving,
    /// charging or recovering. Late observations resolve one shot, not a burst.
    /// Leaving either pad resets its hold. No overall completion deadline.
    ///
    /// # Errors
    /// Rejects reversed time, teleports, cover crossings and terminal actions.
    pub fn apply(&self, elapsed_ms: u64, action: Action) -> Result<Update, Error> {
        if self.health == 0 || self.phase == Phase::Completed {
            return Err(Error::Terminal);
        }
        if elapsed_ms < self.last_elapsed_ms {
            return Err(Error::ClockReversed);
        }
        if let Action::Move { position } = action {
            let distance = (i64::from(position[0]) - i64::from(self.position[0])).abs()
                + (i64::from(position[2]) - i64::from(self.position[2])).abs();
            if !signal::walkable(position)
                || distance != 1
                || cover_blocks_segment(self.position, position)
            {
                return Err(Error::InvalidMovement);
            }
        }
        let mut next = self.clone();
        let mut effect = Effect::default();
        next.last_elapsed_ms = elapsed_ms;
        if elapsed_ms >= self.next_shot_ms {
            next.next_shot_ms = elapsed_ms.saturating_add(SHOT_INTERVAL_MS);
            // Every walkable point in this bounded lane is within sentinel range.
            if !cover_blocks_segment(self.position, SENTINEL_POSITION) {
                effect.incoming_damage = SHOT_DAMAGE;
                next.health = next.health.saturating_sub(SHOT_DAMAGE);
            }
        }
        if next.health == 0 {
            effect.defeated = true;
        } else {
            if let Action::Move { position } = action {
                next.position = position;
            }
            next.recover(elapsed_ms, &mut effect);
            next.charge(elapsed_ms, &mut effect);
        }
        Ok(Update { run: next, effect })
    }

    fn recover(&mut self, elapsed_ms: u64, effect: &mut Effect) {
        if self.reserve_used || self.health == self.max_health || self.position != RESERVE_PAD {
            self.recovery_started_ms = None;
            return;
        }
        let started = *self.recovery_started_ms.get_or_insert(elapsed_ms);
        if elapsed_ms - started >= RECOVERY_MS {
            effect.recovered_health = RECOVERY_HEALTH.min(self.max_health - self.health);
            self.health += effect.recovered_health;
            self.reserve_used = true;
            self.recovery_started_ms = None;
        }
    }

    fn charge(&mut self, elapsed_ms: u64, effect: &mut Effect) {
        let station = match self.phase {
            Phase::West => WEST_RELAY,
            Phase::East => EAST_RELAY,
            Phase::Return => SPAWN,
            Phase::Completed => return,
        };
        if self.position != station {
            self.charge_started_ms = None;
            return;
        }
        let started = *self.charge_started_ms.get_or_insert(elapsed_ms);
        if self.phase != Phase::Return && elapsed_ms - started < CHARGE_MS {
            return;
        }
        let (phase, checkpoint) = match self.phase {
            Phase::West => (Phase::East, Checkpoint::WestCharged),
            Phase::East => (Phase::Return, Checkpoint::EastCharged),
            Phase::Return => (Phase::Completed, Checkpoint::Returned),
            Phase::Completed => return,
        };
        self.phase = phase;
        self.charge_started_ms = None;
        effect.checkpoint = Some(checkpoint);
    }
}

#[cfg(test)]
mod tests;
