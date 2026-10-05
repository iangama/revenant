//! Optional core boss: sidestep locked lanes, then alternate center/perimeter.
//! Propose on a clone and confirm only after the corresponding durable write.

pub const ENTRANCE: [i32; 3] = [5, 0, 2];
pub const SPAWN: [i32; 3] = [8, 0, 0];
pub const HEALTH: u32 = 320;
pub const PHASE_HEALTH: u32 = HEALTH / 2;
pub const OPENING_MS: u64 = 700;
pub const LANE_WARNING_MS: u64 = 1_800;
pub const PULSE_WARNING_MS: u64 = 2_200;
pub const RECOVERY_MS: u64 = 1_800;
pub const PULSE_GAP_MS: u64 = 600;
pub const SHIFT_MS: u64 = 2_400;
pub const LANE_DAMAGE: u32 = 18;
pub const PULSE_DAMAGE: u32 = 20;

#[must_use]
pub fn arena_contains(p: [i32; 3]) -> bool {
    p[1] == 0 && (5..=11).contains(&p[0]) && (-3..=3).contains(&p[2])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrismPhase {
    Lanes,
    Pulses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrismPattern {
    AcrossX { z: i32 },
    AcrossZ { x: i32 },
    Center,
    Perimeter,
}

impl PrismPattern {
    #[must_use]
    pub const fn warning_ms(self) -> u64 {
        match self {
            Self::AcrossX { .. } | Self::AcrossZ { .. } => LANE_WARNING_MS,
            Self::Center | Self::Perimeter => PULSE_WARNING_MS,
        }
    }

    #[must_use]
    pub fn hits(self, position: [i32; 3]) -> bool {
        if !arena_contains(position) {
            return false;
        }
        match self {
            Self::AcrossX { z } => (-3..=3).contains(&z) && position[2] == z,
            Self::AcrossZ { x } => (5..=11).contains(&x) && position[0] == x,
            Self::Center => in_center(position),
            Self::Perimeter => !in_center(position),
        }
    }

    #[must_use]
    pub fn damage_at(self, position: [i32; 3]) -> u32 {
        if !self.hits(position) {
            return 0;
        }
        match self {
            Self::AcrossX { .. } | Self::AcrossZ { .. } => LANE_DAMAGE,
            Self::Center | Self::Perimeter => PULSE_DAMAGE,
        }
    }
}

fn in_center(p: [i32; 3]) -> bool {
    (7..=9).contains(&p[0]) && (-1..=1).contains(&p[2])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrismMode {
    Opening,
    Warning(PrismPattern),
    PulseGap,
    Recovery,
    Shifting,
    Defeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrismAction {
    Warning { pattern: PrismPattern },
    Resolved { pattern: PrismPattern, damage: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrismHit {
    pub damage: u32,
    pub health_after: u32,
    pub phase_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrismWarden {
    phase: PrismPhase,
    mode: PrismMode,
    ready_at_ms: u64,
    next_lane_x: bool,
}

impl Default for PrismWarden {
    fn default() -> Self {
        Self {
            phase: PrismPhase::Lanes,
            mode: PrismMode::Opening,
            ready_at_ms: OPENING_MS,
            next_lane_x: true,
        }
    }
}

impl PrismWarden {
    #[must_use]
    pub const fn phase(&self) -> PrismPhase {
        self.phase
    }

    #[must_use]
    pub const fn mode(&self) -> PrismMode {
        self.mode
    }

    #[must_use]
    pub fn vulnerable(&self) -> bool {
        self.mode == PrismMode::Recovery
    }

    /// Call after a persisted AI action or phase-changing hit, never an ordinary
    /// shot: shooting must not extend a recovery or delay the next attack.
    pub fn confirm_at(&mut self, confirmed_ms: u64) {
        let interval = match self.mode {
            PrismMode::Opening => OPENING_MS,
            PrismMode::Warning(pattern) => pattern.warning_ms(),
            PrismMode::PulseGap => PULSE_GAP_MS,
            PrismMode::Recovery => RECOVERY_MS,
            PrismMode::Shifting => SHIFT_MS,
            PrismMode::Defeated => return,
        };
        self.ready_at_ms = self.ready_at_ms.max(confirmed_ms.saturating_add(interval));
    }

    /// A half-health floor ensures phase two cannot be skipped by a large hit.
    /// Both pulses must resolve before its first vulnerable recovery window.
    pub fn hit(&mut self, now_ms: u64, health_before: u32, requested: u32) -> Option<PrismHit> {
        if !self.valid_health(health_before) || self.mode == PrismMode::Defeated {
            return None;
        }
        let available = if self.phase == PrismPhase::Lanes {
            health_before - PHASE_HEALTH
        } else {
            health_before
        };
        let damage = if self.vulnerable() {
            requested.min(available)
        } else {
            0
        };
        let health_after = health_before - damage;
        let phase_changed = self.phase == PrismPhase::Lanes && health_after == PHASE_HEALTH;
        if phase_changed {
            self.phase = PrismPhase::Pulses;
            self.mode = PrismMode::Shifting;
            self.ready_at_ms = now_ms.saturating_add(SHIFT_MS);
        } else if health_after == 0 {
            self.mode = PrismMode::Defeated;
        }
        Some(PrismHit {
            damage,
            health_after,
            phase_changed,
        })
    }

    pub fn tick(&mut self, now_ms: u64, health: u32, player: [i32; 3]) -> Option<PrismAction> {
        if now_ms < self.ready_at_ms || !self.valid_health(health) || !arena_contains(player) {
            return None;
        }
        if let PrismMode::Warning(pattern) = self.mode {
            self.mode = if pattern == PrismPattern::Center {
                PrismMode::PulseGap
            } else {
                PrismMode::Recovery
            };
            self.ready_at_ms = now_ms;
            self.confirm_at(now_ms);
            return Some(PrismAction::Resolved {
                pattern,
                damage: pattern.damage_at(player),
            });
        }
        let pattern = match self.mode {
            PrismMode::Opening | PrismMode::Recovery if self.phase == PrismPhase::Lanes => {
                let pattern = if self.next_lane_x {
                    PrismPattern::AcrossX { z: player[2] }
                } else {
                    PrismPattern::AcrossZ { x: player[0] }
                };
                self.next_lane_x = !self.next_lane_x;
                pattern
            }
            PrismMode::Shifting | PrismMode::Recovery => PrismPattern::Center,
            PrismMode::PulseGap => PrismPattern::Perimeter,
            PrismMode::Defeated | PrismMode::Warning(_) | PrismMode::Opening => return None,
        };
        self.mode = PrismMode::Warning(pattern);
        self.ready_at_ms = now_ms.saturating_add(pattern.warning_ms());
        Some(PrismAction::Warning { pattern })
    }

    fn valid_health(&self, health: u32) -> bool {
        match self.phase {
            PrismPhase::Lanes => (PHASE_HEALTH + 1..=HEALTH).contains(&health),
            PrismPhase::Pulses => (1..=PHASE_HEALTH).contains(&health),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_recovery(ai: &mut PrismWarden) {
        assert_eq!(ai.tick(699, HEALTH, ENTRANCE), None);
        assert_eq!(
            ai.tick(700, HEALTH, ENTRANCE),
            Some(PrismAction::Warning {
                pattern: PrismPattern::AcrossX { z: 2 },
            })
        );
        assert_eq!(ai.tick(2499, HEALTH, [5, 0, 1]), None);
        assert_eq!(
            ai.tick(2500, HEALTH, [5, 0, 1]),
            Some(PrismAction::Resolved {
                pattern: PrismPattern::AcrossX { z: 2 },
                damage: 0,
            })
        );
        assert!(ai.vulnerable());
    }

    #[test]
    fn prism_locks_lane_then_exposes_recovery_and_alternates_orientation() {
        let mut ai = PrismWarden::default();
        assert_eq!(ai.hit(0, HEALTH, u32::MAX).unwrap().damage, 0);
        open_recovery(&mut ai);
        assert_eq!(
            ai.hit(3000, HEALTH, 40),
            Some(PrismHit {
                damage: 40,
                health_after: 280,
                phase_changed: false,
            })
        );
        assert_eq!(ai.tick(4299, 280, [6, 0, -2]), None);
        assert_eq!(
            ai.tick(4300, 280, [6, 0, -2]),
            Some(PrismAction::Warning {
                pattern: PrismPattern::AcrossZ { x: 6 },
            })
        );
        assert_eq!(ai.hit(4400, 280, 40).unwrap().damage, 0);
        assert_eq!(
            ai.tick(6100, 280, [6, 0, 3]),
            Some(PrismAction::Resolved {
                pattern: PrismPattern::AcrossZ { x: 6 },
                damage: LANE_DAMAGE,
            })
        );
    }

    #[test]
    fn prism_phase_floor_requires_both_spatial_pulses_before_final_damage() {
        let mut ai = PrismWarden::default();
        open_recovery(&mut ai);
        assert_eq!(
            ai.hit(2500, HEALTH, u32::MAX),
            Some(PrismHit {
                damage: 160,
                health_after: PHASE_HEALTH,
                phase_changed: true,
            })
        );
        assert_eq!(ai.phase(), PrismPhase::Pulses);
        assert_eq!(ai.mode(), PrismMode::Shifting);
        assert_eq!(ai.hit(2600, PHASE_HEALTH, u32::MAX).unwrap().damage, 0);
        assert_eq!(ai.tick(4899, PHASE_HEALTH, SPAWN), None);
        assert_eq!(
            ai.tick(4900, PHASE_HEALTH, SPAWN),
            Some(PrismAction::Warning {
                pattern: PrismPattern::Center
            })
        );
        assert_eq!(
            ai.tick(7100, PHASE_HEALTH, [6, 0, 0]),
            Some(PrismAction::Resolved {
                pattern: PrismPattern::Center,
                damage: 0
            })
        );
        assert_eq!(ai.mode(), PrismMode::PulseGap);
        assert_eq!(ai.hit(7100, PHASE_HEALTH, u32::MAX).unwrap().damage, 0);
        assert_eq!(ai.tick(7699, PHASE_HEALTH, [6, 0, 0]), None);
        assert_eq!(
            ai.tick(7700, PHASE_HEALTH, [6, 0, 0]),
            Some(PrismAction::Warning {
                pattern: PrismPattern::Perimeter
            })
        );
        assert_eq!(
            ai.tick(9900, PHASE_HEALTH, [7, 0, 1]),
            Some(PrismAction::Resolved {
                pattern: PrismPattern::Perimeter,
                damage: 0
            })
        );
        assert_eq!(
            ai.hit(10_000, PHASE_HEALTH, u32::MAX),
            Some(PrismHit {
                damage: 160,
                health_after: 0,
                phase_changed: false
            })
        );
        assert_eq!(ai.mode(), PrismMode::Defeated);
        assert_eq!(ai.tick(u64::MAX, 0, SPAWN), None);
        assert_eq!(ai.hit(u64::MAX, 0, 40), None);
    }

    #[test]
    fn prism_slow_confirmation_preserves_warning_gap_recovery_and_phase_pause() {
        let mut ai = PrismWarden::default();
        ai.tick(700, HEALTH, ENTRANCE).unwrap();
        ai.confirm_at(10_000);
        assert_eq!(ai.tick(11_799, HEALTH, ENTRANCE), None);
        assert!(matches!(
            ai.tick(11_800, HEALTH, ENTRANCE),
            Some(PrismAction::Resolved {
                damage: LANE_DAMAGE,
                ..
            })
        ));
        ai.confirm_at(20_000);
        assert_eq!(ai.tick(21_799, HEALTH, ENTRANCE), None);
        assert!(ai.hit(21_799, HEALTH, 160).unwrap().phase_changed);
        ai.confirm_at(30_000);
        assert_eq!(ai.tick(32_399, PHASE_HEALTH, SPAWN), None);
        ai.tick(32_400, PHASE_HEALTH, SPAWN).unwrap();
        ai.confirm_at(40_000);
        assert_eq!(ai.tick(42_199, PHASE_HEALTH, SPAWN), None);
        assert_eq!(
            ai.tick(42_200, PHASE_HEALTH, SPAWN),
            Some(PrismAction::Resolved {
                pattern: PrismPattern::Center,
                damage: PULSE_DAMAGE
            })
        );
        ai.confirm_at(50_000);
        assert_eq!(ai.tick(50_599, PHASE_HEALTH, SPAWN), None);
        ai.tick(50_600, PHASE_HEALTH, SPAWN).unwrap();
        ai.confirm_at(60_000);
        assert_eq!(ai.tick(62_199, PHASE_HEALTH, ENTRANCE), None);
        assert_eq!(
            ai.tick(62_200, PHASE_HEALTH, ENTRANCE),
            Some(PrismAction::Resolved {
                pattern: PrismPattern::Perimeter,
                damage: PULSE_DAMAGE
            })
        );
        ai.confirm_at(70_000);
        assert_eq!(ai.tick(71_799, PHASE_HEALTH, SPAWN), None);
        assert_eq!(
            ai.tick(71_800, PHASE_HEALTH, SPAWN),
            Some(PrismAction::Warning {
                pattern: PrismPattern::Center
            })
        );
    }

    #[test]
    fn prism_geometry_partitions_arena_and_rejects_invalid_inputs_without_overflow() {
        for x in 5..=11 {
            for z in -3..=3 {
                let p = [x, 0, z];
                assert_ne!(
                    PrismPattern::Center.hits(p),
                    PrismPattern::Perimeter.hits(p)
                );
            }
        }
        let mut ai = PrismWarden::default();
        for p in [[i32::MAX, 0, i32::MIN], [8, 1, 0], [4, 0, 0], [8, 0, 4]] {
            assert!(!PrismPattern::Center.hits(p));
            assert!(!PrismPattern::Perimeter.hits(p));
            assert_eq!(ai.tick(10_000, HEALTH, p), None);
        }
        assert!(!PrismPattern::AcrossX { z: i32::MIN }.hits(SPAWN));
        assert!(!PrismPattern::AcrossZ { x: i32::MAX }.hits(SPAWN));
        assert_eq!(ai.hit(0, 0, 40), None);
        assert_eq!(ai.hit(0, HEALTH + 1, 40), None);
        assert_eq!(ai.hit(0, PHASE_HEALTH, 40), None);
    }
}
