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
pub struct PressureProfile {
    pub damage: u32,
    pub initial_attacks: u32,
    pub accepted_hits_per_counterattack: u32,
    pub maximum_counterattacks: u32,
}

impl PressureProfile {
    pub const NONE: Self = Self {
        damage: 0,
        initial_attacks: 0,
        accepted_hits_per_counterattack: 0,
        maximum_counterattacks: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatScenario {
    pub attack: AttackProfile,
    pub target_health: u32,
    pub target_distance_squared: i64,
    pub attacker_count: u32,
    pub client_attack_interval_ms: u64,
    pub round_trip_ms: u64,
    pub pressure: PressureProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatMetrics {
    pub required_hits: u32,
    pub required_volleys: u32,
    pub effective_attack_interval_ms: u64,
    pub theoretical_ttk_ms: u64,
    pub effective_ttk_ms: u64,
    pub overkill_damage: u64,
    pub damage_per_second_milli: u64,
    pub cooldown_duty_basis_points: u64,
    pub range_margin_squared: i64,
    pub in_range: bool,
    pub expected_hostile_attacks: u32,
    pub expected_incoming_damage: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatAnalysisError {
    ZeroDamage,
    NegativeRange,
    ZeroCooldown,
    ZeroTargetHealth,
    NegativeDistanceSquared,
    ZeroAttackers,
    InvalidPressure,
    ArithmeticOverflow,
}

impl Display for CombatAnalysisError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroDamage => "combat analysis requires positive attack damage",
            Self::NegativeRange => "combat analysis requires a non-negative range",
            Self::ZeroCooldown => "combat analysis requires a positive cooldown",
            Self::ZeroTargetHealth => "combat analysis requires positive target health",
            Self::NegativeDistanceSquared => {
                "combat analysis requires a non-negative squared distance"
            }
            Self::ZeroAttackers => "combat analysis requires at least one attacker",
            Self::InvalidPressure => {
                "hostile pressure requires positive damage and a valid hit interval"
            }
            Self::ArithmeticOverflow => "combat analysis arithmetic overflowed",
        };
        formatter.write_str(message)
    }
}

impl Error for CombatAnalysisError {}

/// Derives deterministic timing, range, damage, and pressure metrics for one
/// combat scenario without consulting a clock or mutating gameplay state.
///
/// Time-to-defeat begins at the first accepted player hit. Simulated round-trip
/// delay or a conservative client input gate can extend the effective interval
/// but can never shorten the authoritative cooldown.
///
/// # Errors
///
/// Returns [`CombatAnalysisError`] when a profile is invalid or an intermediate
/// calculation cannot be represented safely.
pub fn analyze_scenario(scenario: CombatScenario) -> Result<CombatMetrics, CombatAnalysisError> {
    validate_scenario(scenario)?;

    let required_hits = scenario.target_health.div_ceil(scenario.attack.damage);
    let required_volleys = required_hits.div_ceil(scenario.attacker_count);
    let theoretical_ttk_ms = u64::from(required_volleys.saturating_sub(1))
        .checked_mul(scenario.attack.cooldown_ms)
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?;
    let effective_attack_interval_ms = scenario
        .attack
        .cooldown_ms
        .max(scenario.client_attack_interval_ms)
        .max(scenario.round_trip_ms);
    let effective_ttk_ms = u64::from(required_volleys.saturating_sub(1))
        .checked_mul(effective_attack_interval_ms)
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?;
    let total_damage = u64::from(required_hits)
        .checked_mul(u64::from(scenario.attack.damage))
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?;
    let overkill_damage = total_damage
        .checked_sub(u64::from(scenario.target_health))
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?;
    let damage_per_second_milli = u64::from(scenario.attack.damage)
        .checked_mul(1_000_000)
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?
        / scenario.attack.cooldown_ms;
    let cooldown_duty_basis_points = scenario
        .attack
        .cooldown_ms
        .checked_mul(10_000)
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?
        / effective_attack_interval_ms;
    let range_squared = i64::from(scenario.attack.range).pow(2);
    let range_margin_squared = range_squared
        .checked_sub(scenario.target_distance_squared)
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?;
    let expected_hostile_attacks = hostile_attacks_for_hits(required_hits, scenario.pressure)?;
    let expected_incoming_damage = u64::from(expected_hostile_attacks)
        .checked_mul(u64::from(scenario.pressure.damage))
        .ok_or(CombatAnalysisError::ArithmeticOverflow)?;

    Ok(CombatMetrics {
        required_hits,
        required_volleys,
        effective_attack_interval_ms,
        theoretical_ttk_ms,
        effective_ttk_ms,
        overkill_damage,
        damage_per_second_milli,
        cooldown_duty_basis_points,
        range_margin_squared,
        in_range: range_margin_squared >= 0,
        expected_hostile_attacks,
        expected_incoming_damage,
    })
}

fn validate_scenario(scenario: CombatScenario) -> Result<(), CombatAnalysisError> {
    if scenario.attack.damage == 0 {
        return Err(CombatAnalysisError::ZeroDamage);
    }
    if scenario.attack.range < 0 {
        return Err(CombatAnalysisError::NegativeRange);
    }
    if scenario.attack.cooldown_ms == 0 {
        return Err(CombatAnalysisError::ZeroCooldown);
    }
    if scenario.target_health == 0 {
        return Err(CombatAnalysisError::ZeroTargetHealth);
    }
    if scenario.target_distance_squared < 0 {
        return Err(CombatAnalysisError::NegativeDistanceSquared);
    }
    if scenario.attacker_count == 0 {
        return Err(CombatAnalysisError::ZeroAttackers);
    }
    if (scenario.pressure.initial_attacks > 0 || scenario.pressure.maximum_counterattacks > 0)
        && scenario.pressure.damage == 0
    {
        return Err(CombatAnalysisError::InvalidPressure);
    }
    if scenario.pressure.maximum_counterattacks > 0
        && scenario.pressure.accepted_hits_per_counterattack == 0
    {
        return Err(CombatAnalysisError::InvalidPressure);
    }
    Ok(())
}

fn hostile_attacks_for_hits(
    required_hits: u32,
    pressure: PressureProfile,
) -> Result<u32, CombatAnalysisError> {
    let nonlethal_hits = required_hits.saturating_sub(1);
    let counterattacks = if pressure.maximum_counterattacks == 0 {
        0
    } else {
        (nonlethal_hits / pressure.accepted_hits_per_counterattack)
            .min(pressure.maximum_counterattacks)
    };
    pressure
        .initial_attacks
        .checked_add(counterattacks)
        .ok_or(CombatAnalysisError::ArithmeticOverflow)
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

#[derive(Debug, Default, Clone)]
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
            || attacker.health == 0
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

    use super::{
        analyze_scenario, AttackProfile, CombatAnalysisError, CombatError, CombatRuntime,
        CombatScenario, PressureProfile,
    };

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

    #[test]
    fn defeated_players_cannot_attack() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", [0, 0, 0], 100);
        let enemy = actors.spawn(ActorKind::Enemy, "relay-drone", [2, 0, 0], 100);
        actors.apply_damage(player.id, 100);
        let mut combat = CombatRuntime::default();

        assert_eq!(
            combat.attack(
                &mut actors,
                player.id,
                enemy.id,
                1_000,
                AttackProfile {
                    damage: 40,
                    range: 6,
                    cooldown_ms: 250,
                },
            ),
            Err(CombatError::InvalidTarget)
        );
        assert_eq!(actors.get(enemy.id).unwrap().health, 100);
    }

    #[test]
    fn analysis_reproduces_the_untuned_weapon_baseline() {
        let drone = analyze_scenario(CombatScenario {
            attack: AttackProfile {
                damage: 25,
                range: 8,
                cooldown_ms: 150,
            },
            target_health: 100,
            target_distance_squared: 4,
            attacker_count: 1,
            client_attack_interval_ms: 260,
            round_trip_ms: 0,
            pressure: PressureProfile {
                damage: 10,
                initial_attacks: 1,
                accepted_hits_per_counterattack: 0,
                maximum_counterattacks: 0,
            },
        })
        .expect("current sidearm/drone scenario should be valid");
        assert_eq!(drone.required_hits, 4);
        assert_eq!(drone.theoretical_ttk_ms, 450);
        assert_eq!(drone.effective_ttk_ms, 780);
        assert_eq!(drone.overkill_damage, 0);
        assert_eq!(drone.range_margin_squared, 60);
        assert_eq!(drone.expected_incoming_damage, 10);

        let warden = analyze_scenario(CombatScenario {
            attack: AttackProfile {
                damage: 40,
                range: 6,
                cooldown_ms: 250,
            },
            target_health: 120,
            target_distance_squared: 4,
            attacker_count: 1,
            client_attack_interval_ms: 260,
            round_trip_ms: 0,
            pressure: PressureProfile::NONE,
        })
        .expect("current rifle/Warden scenario should be valid");
        assert_eq!(warden.required_hits, 3);
        assert_eq!(warden.theoretical_ttk_ms, 500);
        assert_eq!(warden.effective_ttk_ms, 520);
        assert_eq!(warden.overkill_damage, 0);
        assert_eq!(warden.expected_hostile_attacks, 0);
    }

    #[test]
    fn analysis_bounds_multiplayer_latency_overkill_and_pressure() {
        let metrics = analyze_scenario(CombatScenario {
            attack: AttackProfile {
                damage: 34,
                range: 6,
                cooldown_ms: 320,
            },
            target_health: 100,
            target_distance_squared: 36,
            attacker_count: 2,
            client_attack_interval_ms: 320,
            round_trip_ms: 400,
            pressure: PressureProfile {
                damage: 12,
                initial_attacks: 0,
                accepted_hits_per_counterattack: 2,
                maximum_counterattacks: 3,
            },
        })
        .expect("bounded multiplayer scenario should be valid");
        assert_eq!(metrics.required_hits, 3);
        assert_eq!(metrics.required_volleys, 2);
        assert_eq!(metrics.theoretical_ttk_ms, 320);
        assert_eq!(metrics.effective_ttk_ms, 400);
        assert_eq!(metrics.overkill_damage, 2);
        assert_eq!(metrics.cooldown_duty_basis_points, 8_000);
        assert_eq!(metrics.expected_hostile_attacks, 1);
        assert_eq!(metrics.expected_incoming_damage, 12);
        assert!(metrics.in_range);
    }

    #[test]
    fn analysis_rejects_invalid_or_overflowing_scenarios() {
        let base = CombatScenario {
            attack: AttackProfile {
                damage: 1,
                range: 1,
                cooldown_ms: 1,
            },
            target_health: 1,
            target_distance_squared: 0,
            attacker_count: 1,
            client_attack_interval_ms: 1,
            round_trip_ms: 0,
            pressure: PressureProfile::NONE,
        };
        assert_eq!(
            analyze_scenario(CombatScenario {
                attack: AttackProfile {
                    damage: 0,
                    ..base.attack
                },
                ..base
            }),
            Err(CombatAnalysisError::ZeroDamage)
        );
        assert_eq!(
            analyze_scenario(CombatScenario {
                attacker_count: 0,
                ..base
            }),
            Err(CombatAnalysisError::ZeroAttackers)
        );
        assert_eq!(
            analyze_scenario(CombatScenario {
                pressure: PressureProfile {
                    damage: 1,
                    initial_attacks: 0,
                    accepted_hits_per_counterattack: 0,
                    maximum_counterattacks: 2,
                },
                ..base
            }),
            Err(CombatAnalysisError::InvalidPressure)
        );
        assert_eq!(
            analyze_scenario(CombatScenario {
                attack: AttackProfile {
                    cooldown_ms: u64::MAX,
                    ..base.attack
                },
                target_health: u32::MAX,
                ..base
            }),
            Err(CombatAnalysisError::ArithmeticOverflow)
        );
    }
}
