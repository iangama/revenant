use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use revenant_actors::{ActorKind, ActorRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackProfile {
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageResult {
    pub damage: u32,
    pub remaining_health: u32,
    pub killed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatError {
    ActorNotFound,
    InvalidTarget,
    OutOfRange,
    CooldownActive,
}

impl Display for CombatError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ActorNotFound => "attacker or target does not exist",
            Self::InvalidTarget => "attacker cannot attack this target",
            Self::OutOfRange => "target is outside basic attack range",
            Self::CooldownActive => "basic attack cooldown is active",
        };
        formatter.write_str(message)
    }
}

impl Error for CombatError {}

#[derive(Debug, Default)]
pub struct CombatRuntime {
    ready_at: HashMap<u64, u64>,
}

impl CombatRuntime {
    /// Validates and applies one basic attack at the supplied monotonic timestamp.
    ///
    /// # Errors
    ///
    /// Returns a [`CombatError`] when actor existence, faction, range, or cooldown
    /// validation fails.
    pub fn attack(
        &mut self,
        actors: &mut ActorRegistry,
        attacker_id: u64,
        target_id: u64,
        now_ms: u64,
        profile: AttackProfile,
    ) -> Result<DamageResult, CombatError> {
        let attacker = actors
            .get(attacker_id)
            .cloned()
            .ok_or(CombatError::ActorNotFound)?;
        let target = actors
            .get(target_id)
            .cloned()
            .ok_or(CombatError::ActorNotFound)?;
        if attacker.kind != ActorKind::Player
            || target.kind != ActorKind::Enemy
            || target.health == 0
        {
            return Err(CombatError::InvalidTarget);
        }
        let distance_squared = attacker
            .position
            .iter()
            .zip(target.position)
            .map(|(left, right)| (i64::from(*left) - i64::from(right)).pow(2))
            .sum::<i64>();
        if distance_squared > i64::from(profile.range).pow(2) {
            return Err(CombatError::OutOfRange);
        }
        if self
            .ready_at
            .get(&attacker_id)
            .is_some_and(|ready_at| now_ms < *ready_at)
        {
            return Err(CombatError::CooldownActive);
        }

        self.ready_at
            .insert(attacker_id, now_ms.saturating_add(profile.cooldown_ms));
        let target = actors
            .apply_damage(target_id, profile.damage)
            .ok_or(CombatError::ActorNotFound)?;
        Ok(DamageResult {
            damage: profile.damage,
            remaining_health: target.health,
            killed: target.health == 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use revenant_actors::{ActorKind, ActorRegistry};

    use super::{AttackProfile, CombatError, CombatRuntime};

    #[test]
    fn damage_cooldown_and_death_are_server_owned() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", [0, 0, 0], 100);
        let enemy = actors.spawn(ActorKind::Enemy, "relay-drone", [4, 0, 2], 100);
        let mut combat = CombatRuntime::default();
        let rifle = AttackProfile {
            damage: 40,
            range: 6,
            cooldown_ms: 250,
        };

        let first = combat
            .attack(&mut actors, player.id, enemy.id, 1_000, rifle)
            .unwrap();
        assert_eq!(first.remaining_health, 60);
        assert_eq!(
            combat.attack(&mut actors, player.id, enemy.id, 1_001, rifle),
            Err(CombatError::CooldownActive)
        );
        combat
            .attack(&mut actors, player.id, enemy.id, 1_250, rifle)
            .unwrap();
        let final_hit = combat
            .attack(&mut actors, player.id, enemy.id, 1_500, rifle)
            .unwrap();
        assert!(final_hit.killed);
        assert_eq!(final_hit.remaining_health, 0);
    }

    #[test]
    fn switching_profiles_does_not_reset_attacker_cooldown() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", [0, 0, 0], 100);
        let enemy = actors.spawn(ActorKind::Enemy, "relay-drone", [4, 0, 0], 100);
        let mut combat = CombatRuntime::default();
        let rifle = AttackProfile {
            damage: 40,
            range: 6,
            cooldown_ms: 250,
        };
        let sidearm = AttackProfile {
            damage: 25,
            range: 8,
            cooldown_ms: 150,
        };
        combat
            .attack(&mut actors, player.id, enemy.id, 1_000, rifle)
            .expect("rifle attack should succeed");
        assert_eq!(
            combat.attack(&mut actors, player.id, enemy.id, 1_151, sidearm),
            Err(CombatError::CooldownActive)
        );
        let result = combat
            .attack(&mut actors, player.id, enemy.id, 1_250, sidearm)
            .expect("sidearm should fire after original deadline");
        assert_eq!(result.damage, 25);
    }
}
