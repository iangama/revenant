//! A telegraphed cardinal charge. Targets lock before damage; dodges are real.

pub const ENTRANCE: [i32; 3] = [4, 0, 8];
pub const SPAWN: [i32; 3] = [9, 0, 8];
pub const HEALTH: u32 = 200;
pub const DAMAGE: u32 = 18;
pub const WINDUP_MS: u64 = 1_800;
pub const RECOVERY_MS: u64 = 1_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeAction {
    Windup {
        origin: [i32; 3],
        target: [i32; 3],
    },
    Resolved {
        origin: [i32; 3],
        target: [i32; 3],
        damage: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlassLancer {
    ready_at_ms: u64,
    locked: Option<([i32; 3], [i32; 3])>,
}

impl Default for GlassLancer {
    fn default() -> Self {
        Self {
            ready_at_ms: 700,
            locked: None,
        }
    }
}

impl GlassLancer {
    /// Preserve the full warning/recovery interval after a slow durable write.
    pub fn confirm_at(&mut self, now_ms: u64) {
        let interval = if self.locked.is_some() {
            WINDUP_MS
        } else {
            RECOVERY_MS
        };
        self.ready_at_ms = self.ready_at_ms.max(now_ms.saturating_add(interval));
    }

    pub fn tick(&mut self, now_ms: u64, enemy: [i32; 3], player: [i32; 3]) -> Option<ChargeAction> {
        if now_ms < self.ready_at_ms || !arena_contains(enemy) || !arena_contains(player) {
            return None;
        }
        if let Some((origin, target)) = self.locked.take() {
            self.ready_at_ms = now_ms.saturating_add(RECOVERY_MS);
            return Some(ChargeAction::Resolved {
                origin,
                target,
                damage: if charge_hits(origin, target, player) {
                    DAMAGE
                } else {
                    0
                },
            });
        }
        let mut target = enemy;
        let axis = if (player[0] - enemy[0]).abs() >= (player[2] - enemy[2]).abs() {
            0
        } else {
            2
        };
        target[axis] = player[axis];
        self.locked = Some((enemy, target));
        self.ready_at_ms = now_ms.saturating_add(WINDUP_MS);
        Some(ChargeAction::Windup {
            origin: enemy,
            target,
        })
    }
}

#[must_use]
pub fn arena_contains(p: [i32; 3]) -> bool {
    p[1] == 0 && (2..=10).contains(&p[0]) && (5..=10).contains(&p[2])
}

#[must_use]
pub fn charge_hits(origin: [i32; 3], target: [i32; 3], player: [i32; 3]) -> bool {
    arena_contains(origin)
        && arena_contains(target)
        && arena_contains(player)
        && ((origin[0] == target[0]
            && player[0] == origin[0]
            && (origin[2].min(target[2])..=origin[2].max(target[2])).contains(&player[2]))
            || (origin[2] == target[2]
                && player[2] == origin[2]
                && (origin[0].min(target[0])..=origin[0].max(target[0])).contains(&player[0])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_locks_target_and_a_side_step_avoids_all_damage() {
        let mut ai = GlassLancer::default();
        assert_eq!(ai.tick(699, SPAWN, ENTRANCE), None);
        assert_eq!(
            ai.tick(700, SPAWN, ENTRANCE),
            Some(ChargeAction::Windup {
                origin: SPAWN,
                target: ENTRANCE
            })
        );
        assert_eq!(ai.tick(2499, SPAWN, [4, 0, 9]), None);
        assert_eq!(
            ai.tick(2500, SPAWN, [4, 0, 9]),
            Some(ChargeAction::Resolved {
                origin: SPAWN,
                target: ENTRANCE,
                damage: 0
            })
        );
        assert_eq!(ai.tick(3899, ENTRANCE, [4, 0, 9]), None);
        assert!(matches!(
            ai.tick(3900, ENTRANCE, [4, 0, 9]),
            Some(ChargeAction::Windup { .. })
        ));
    }

    #[test]
    fn delayed_ticks_never_stack_attacks_or_shorten_the_warning() {
        let mut ai = GlassLancer::default();
        assert!(matches!(
            ai.tick(10_000, SPAWN, ENTRANCE),
            Some(ChargeAction::Windup { .. })
        ));
        assert_eq!(ai.tick(11_799, SPAWN, ENTRANCE), None);
        assert!(matches!(
            ai.tick(20_000, SPAWN, ENTRANCE),
            Some(ChargeAction::Resolved { damage: DAMAGE, .. })
        ));
        assert_eq!(ai.tick(20_000, ENTRANCE, ENTRANCE), None);
        assert!(!charge_hits(SPAWN, ENTRANCE, [3, 0, 8]));
        assert!(!charge_hits(SPAWN, ENTRANCE, [5, 0, 9]));
        assert!(!charge_hits(SPAWN, ENTRANCE, [i32::MAX, 0, i32::MIN]));
    }

    #[test]
    fn slow_confirmation_preserves_the_entire_visible_warning_and_recovery() {
        let mut ai = GlassLancer::default();
        ai.tick(700, SPAWN, ENTRANCE).unwrap();
        ai.confirm_at(5000);
        assert_eq!(ai.tick(6799, SPAWN, ENTRANCE), None);
        assert!(matches!(
            ai.tick(6800, SPAWN, ENTRANCE),
            Some(ChargeAction::Resolved { .. })
        ));
        ai.confirm_at(8000);
        assert_eq!(ai.tick(9399, ENTRANCE, ENTRANCE), None);
        assert!(matches!(
            ai.tick(9400, ENTRANCE, ENTRANCE),
            Some(ChargeAction::Windup { .. })
        ));
    }
}
