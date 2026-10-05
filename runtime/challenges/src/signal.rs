//! A bounded firing lane around the retained Signal Sentinel barrier. Every
//! accepted move, shot and sentinel observation is replayed before projection.
use revenant_ai::sentinel::{
    cover_blocks_segment, SENTINEL_HEALTH, SENTINEL_POSITION, SHOT_DAMAGE, SHOT_INTERVAL_MS,
    SHOT_RANGE_SQUARED,
};
use revenant_inventory::{positioned_damage, weapon_profile};
use revenant_modules::{resolve_build, BaseCombatProfile, EffectiveCombatProfile};
use serde::{Deserialize, Serialize};

use crate::{ChallengeError, Equipment};

pub const SPAWN: [i32; 3] = [-4, 0, 4];
pub const REVISION: &str = "distant-signal-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Move { position: [i32; 3] },
    Attack,
    Observe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terminal {
    Completed,
    Defeated,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub outgoing_damage: u32,
    pub incoming_damage: u32,
    pub terminal: Option<Terminal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalRun {
    position: [i32; 3],
    health: u32,
    enemy_health: u32,
    weapon: String,
    profile: EffectiveCombatProfile,
    last_elapsed_ms: u64,
    next_shot_ms: u64,
    attack_ready_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub run: SignalRun,
    pub effect: Effect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    ClockReversed,
    InvalidMovement,
    BlockedShot,
    Cooldown,
    OutOfRange,
    Terminal,
}

impl SignalRun {
    /// # Errors
    /// Rejects equipment outside the admitted M35 build catalog.
    pub fn new(equipment: &Equipment) -> Result<Self, ChallengeError> {
        equipment.validate()?;
        let base = weapon_profile(&equipment.weapon_id).ok_or(ChallengeError::InvalidEquipment)?;
        let build = resolve_build(
            &equipment.catalog_revision,
            BaseCombatProfile {
                damage: base.damage,
                range: base.range,
                cooldown_ms: base.cooldown_ms,
                max_health: 100,
            },
            &equipment.modules,
        )
        .map_err(|_| ChallengeError::InvalidEquipment)?;
        if build.profile.range < 4 {
            return Err(ChallengeError::InvalidEquipment);
        }
        Ok(Self {
            position: SPAWN,
            health: build.profile.max_health,
            enemy_health: SENTINEL_HEALTH,
            weapon: equipment.weapon_id.clone(),
            profile: build.profile,
            last_elapsed_ms: 0,
            next_shot_ms: SHOT_INTERVAL_MS,
            attack_ready_ms: 0,
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
    pub const fn enemy_health(&self) -> u32 {
        self.enemy_health
    }
    #[must_use]
    pub const fn next_shot_ms(&self) -> u64 {
        self.next_shot_ms
    }

    /// Plans without changing accepted state. A due sentinel shot resolves at
    /// the previous accepted position before movement or the player's attack;
    /// a delayed tick never accumulates a burst. Cover observations consume the
    /// same cadence as the retained sentinel, even when no damage is dealt.
    ///
    /// # Errors
    /// Rejects terminal actions, reversed time, teleports, barrier crossings,
    /// blocked/out-of-range shots and shared weapon cooldown violations.
    pub fn apply(&self, elapsed_ms: u64, action: Action) -> Result<Update, Error> {
        if self.health == 0 || self.enemy_health == 0 {
            return Err(Error::Terminal);
        }
        if elapsed_ms < self.last_elapsed_ms {
            return Err(Error::ClockReversed);
        }
        let requested = self.validate_action(elapsed_ms, action)?;
        let mut next = self.clone();
        let mut effect = Effect::default();
        next.last_elapsed_ms = elapsed_ms;
        if elapsed_ms >= next.next_shot_ms {
            next.next_shot_ms = elapsed_ms.saturating_add(SHOT_INTERVAL_MS);
            if self.distance_squared() <= SHOT_RANGE_SQUARED
                && !cover_blocks_segment(self.position, SENTINEL_POSITION)
            {
                effect.incoming_damage = SHOT_DAMAGE;
                next.health = next.health.saturating_sub(effect.incoming_damage);
            }
        }
        if next.health == 0 {
            effect.terminal = Some(Terminal::Defeated);
        } else {
            match action {
                Action::Move { position } => next.position = position,
                Action::Attack => {
                    effect.outgoing_damage = requested;
                    next.enemy_health = next.enemy_health.saturating_sub(requested);
                    next.attack_ready_ms = elapsed_ms.saturating_add(self.profile.cooldown_ms);
                    if next.enemy_health == 0 {
                        effect.terminal = Some(Terminal::Completed);
                    }
                }
                Action::Observe => {}
            }
        }
        Ok(Update { run: next, effect })
    }

    fn validate_action(&self, elapsed_ms: u64, action: Action) -> Result<u32, Error> {
        match action {
            Action::Move { position } => {
                let distance = (i64::from(position[0]) - i64::from(self.position[0])).abs()
                    + (i64::from(position[2]) - i64::from(self.position[2])).abs();
                if !walkable(position)
                    || distance != 1
                    || cover_blocks_segment(self.position, position)
                {
                    return Err(Error::InvalidMovement);
                }
                Ok(0)
            }
            Action::Attack => {
                if cover_blocks_segment(self.position, SENTINEL_POSITION) {
                    return Err(Error::BlockedShot);
                }
                if elapsed_ms < self.attack_ready_ms {
                    return Err(Error::Cooldown);
                }
                let distance = self.distance_squared();
                if distance > i64::from(self.profile.range).pow(2) {
                    return Err(Error::OutOfRange);
                }
                positioned_damage(&self.weapon, self.profile.damage, distance)
                    .ok_or(Error::OutOfRange)
            }
            Action::Observe => Ok(0),
        }
    }

    fn distance_squared(&self) -> i64 {
        self.position
            .iter()
            .zip(SENTINEL_POSITION)
            .map(|(a, b)| (i64::from(*a) - i64::from(b)).pow(2))
            .sum()
    }
}

#[must_use]
pub fn walkable(position: [i32; 3]) -> bool {
    position[1] == 0
        && (-9..=1).contains(&position[0])
        && (0..=5).contains(&position[2])
        && !cover_blocks_segment(position, position)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equipment(weapon: &str) -> Equipment {
        Equipment {
            loadout_revision: 0,
            catalog_revision: revenant_modules::BUILD_CATALOG_REVISION.into(),
            weapon_id: weapon.into(),
            modules: vec![],
        }
    }

    fn left_lane(weapon: &str) -> SignalRun {
        let mut run = SignalRun::new(&equipment(weapon)).unwrap();
        for (time, position) in [[-5, 0, 4], [-6, 0, 4], [-7, 0, 4], [-7, 0, 3], [-7, 0, 2]]
            .into_iter()
            .enumerate()
        {
            run = run
                .apply(
                    u64::try_from(time).unwrap() * 100,
                    Action::Move { position },
                )
                .unwrap()
                .run;
        }
        run
    }

    #[test]
    fn signal_cover_flanks_and_weapon_ranges_create_distinct_firing_choices() {
        let start = SignalRun::new(&equipment("pulse_rifle")).unwrap();
        assert_eq!(start.apply(0, Action::Attack), Err(Error::BlockedShot));
        let covered = start.apply(20_000, Action::Observe).unwrap();
        assert_eq!(covered.run.health(), 100);
        assert_eq!(covered.run.next_shot_ms(), 21_800);
        let pulse = left_lane("pulse_rifle");
        assert_eq!(pulse.apply(500, Action::Attack), Err(Error::OutOfRange));
        let rail = left_lane("rail_driver").apply(500, Action::Attack).unwrap();
        assert_eq!(rail.effect.outgoing_damage, 56);
        assert_eq!(rail.run.enemy_health(), 144);
        let close = pulse
            .apply(
                500,
                Action::Move {
                    position: [-7, 0, 1],
                },
            )
            .unwrap()
            .run;
        assert_eq!(
            close
                .apply(600, Action::Attack)
                .unwrap()
                .effect
                .outgoing_damage,
            40
        );
        let scatter = left_lane("scatter_caster")
            .apply(
                500,
                Action::Move {
                    position: [-7, 0, 1],
                },
            )
            .unwrap()
            .run;
        assert_eq!(
            scatter
                .apply(600, Action::Attack)
                .unwrap()
                .effect
                .outgoing_damage,
            28
        );
    }

    #[test]
    fn signal_requires_adjacent_bounded_moves_and_cannot_cross_cover() {
        let run = SignalRun::new(&equipment("pulse_rifle")).unwrap();
        for position in [
            SENTINEL_POSITION,
            [-4, 1, 4],
            [i32::MAX, 0, i32::MIN],
            SPAWN,
        ] {
            assert_eq!(
                run.apply(100, Action::Move { position }),
                Err(Error::InvalidMovement)
            );
        }
        let run = run
            .apply(
                100,
                Action::Move {
                    position: [-4, 0, 3],
                },
            )
            .unwrap()
            .run
            .apply(
                200,
                Action::Move {
                    position: [-4, 0, 2],
                },
            )
            .unwrap()
            .run;
        assert_eq!(
            run.apply(
                300,
                Action::Move {
                    position: [-4, 0, 1]
                }
            ),
            Err(Error::InvalidMovement)
        );
        assert_eq!(run.apply(199, Action::Observe), Err(Error::ClockReversed));
        assert_eq!(run.position(), [-4, 0, 2]);
    }

    #[test]
    fn signal_cadence_cooldown_terminal_and_fresh_retry_are_authoritative() {
        let mut run = left_lane("rail_driver");
        for time in [500, 900, 1300, 1700] {
            let update = run.apply(time, Action::Attack).unwrap();
            run = update.run;
            if time == 500 {
                assert_eq!(run.apply(899, Action::Attack), Err(Error::Cooldown));
            }
            if time == 1700 {
                assert_eq!(update.effect.terminal, Some(Terminal::Completed));
            }
        }
        assert_eq!(run.apply(1800, Action::Observe), Err(Error::Terminal));
        let mut run = left_lane("rail_driver");
        for index in 1..=9 {
            let update = run
                .apply(index * SHOT_INTERVAL_MS, Action::Observe)
                .unwrap();
            assert_eq!(update.effect.incoming_damage, 12);
            run = update.run;
            if index == 9 {
                assert_eq!(update.effect.terminal, Some(Terminal::Defeated));
            }
        }
        assert_eq!(run.apply(20_000, Action::Attack), Err(Error::Terminal));
        let retry = SignalRun::new(&equipment("rail_driver")).unwrap();
        assert_eq!(
            (retry.health(), retry.enemy_health(), retry.position()),
            (100, 200, SPAWN)
        );
        let delayed = left_lane("rail_driver")
            .apply(100_000, Action::Observe)
            .unwrap();
        assert_eq!(delayed.run.health(), 88);
        assert_eq!(delayed.run.next_shot_ms(), 101_800);
    }
}
