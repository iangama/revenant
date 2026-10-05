//! Cardinal protection for the next optional encounter. Facing locks per stance.

pub const ENTRANCE: [i32; 3] = [6, 0, -8];
pub const SPAWN: [i32; 3] = [8, 0, -8];
pub const HEALTH: u32 = 240;
pub const DAMAGE: u32 = 14;
pub const WINDUP_MS: u64 = 1_600;
pub const RECOVERY_MS: u64 = 1_800;

#[must_use]
pub fn arena_contains(p: [i32; 3]) -> bool {
    p[1] == 0 && (5..=10).contains(&p[0]) && (-10..=-5).contains(&p[2])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulwarkAction {
    Brace { facing: Facing },
    Slam { facing: Facing, damage: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteelBulwark {
    facing: Facing,
    braced: bool,
    ready_at_ms: u64,
}

impl Default for SteelBulwark {
    fn default() -> Self {
        Self {
            facing: Facing::West,
            braced: true,
            ready_at_ms: WINDUP_MS,
        }
    }
}

impl SteelBulwark {
    #[must_use]
    pub fn facing(&self) -> Facing {
        self.facing
    }

    #[must_use]
    pub fn braced(&self) -> bool {
        self.braced
    }

    #[must_use]
    pub fn blocks(&self, origin: [i32; 3], attacker: [i32; 3]) -> bool {
        self.braced && self.facing.protects(origin, attacker)
    }

    pub fn confirm_at(&mut self, now_ms: u64) {
        let interval = if self.braced { WINDUP_MS } else { RECOVERY_MS };
        self.ready_at_ms = self.ready_at_ms.max(now_ms.saturating_add(interval));
    }

    pub fn tick(
        &mut self,
        now_ms: u64,
        origin: [i32; 3],
        player: [i32; 3],
    ) -> Option<BulwarkAction> {
        if now_ms < self.ready_at_ms || !arena_contains(origin) || !arena_contains(player) {
            return None;
        }
        if self.braced {
            self.braced = false;
            self.ready_at_ms = now_ms.saturating_add(RECOVERY_MS);
            Some(BulwarkAction::Slam {
                facing: self.facing,
                damage: if slam_hits(self.facing, origin, player) {
                    DAMAGE
                } else {
                    0
                },
            })
        } else {
            self.facing = self.facing.toward(origin, player);
            self.braced = true;
            self.ready_at_ms = now_ms.saturating_add(WINDUP_MS);
            Some(BulwarkAction::Brace {
                facing: self.facing,
            })
        }
    }
}

#[must_use]
pub fn slam_hits(facing: Facing, origin: [i32; 3], player: [i32; 3]) -> bool {
    if !arena_contains(origin) || !arena_contains(player) || !facing.protects(origin, player) {
        return false;
    }
    let dx = player[0] - origin[0];
    let dz = player[2] - origin[2];
    dx * dx + dz * dz <= 4
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    #[must_use]
    pub const fn vector(self) -> [i32; 2] {
        match self {
            Self::North => [0, -1],
            Self::East => [1, 0],
            Self::South => [0, 1],
            Self::West => [-1, 0],
        }
    }

    #[must_use]
    pub const fn from_vector(vector: [i32; 2]) -> Option<Self> {
        match vector {
            [0, -1] => Some(Self::North),
            [1, 0] => Some(Self::East),
            [0, 1] => Some(Self::South),
            [-1, 0] => Some(Self::West),
            _ => None,
        }
    }

    /// Select the dominant direction; coincident actors preserve current facing.
    #[must_use]
    pub fn toward(self, origin: [i32; 3], target: [i32; 3]) -> Self {
        let dx = i64::from(target[0]) - i64::from(origin[0]);
        let dz = i64::from(target[2]) - i64::from(origin[2]);
        if dx == 0 && dz == 0 {
            self
        } else if dx.abs() >= dz.abs() {
            if dx > 0 {
                Self::East
            } else {
                Self::West
            }
        } else if dz > 0 {
            Self::South
        } else {
            Self::North
        }
    }

    /// The forward 90-degree sector is protected; side and rear remain exposed.
    #[must_use]
    pub fn protects(self, origin: [i32; 3], attacker: [i32; 3]) -> bool {
        if origin[1] != attacker[1] {
            return false;
        }
        let dx = i64::from(attacker[0]) - i64::from(origin[0]);
        let dz = i64::from(attacker[2]) - i64::from(origin[2]);
        let (forward, lateral) = match self {
            Self::North => (-dz, dx),
            Self::East => (dx, dz),
            Self::South => (dz, dx),
            Self::West => (-dx, dz),
        };
        forward > 0 && forward >= lateral.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulwark_flank_avoids_slam_and_recovery_exposes_the_front() {
        let mut ai = SteelBulwark::default();
        assert!(ai.blocks(SPAWN, ENTRANCE));
        assert_eq!(ai.tick(1599, SPAWN, ENTRANCE), None);
        let flank = [8, 0, -6];
        assert!(!ai.blocks(SPAWN, flank));
        assert_eq!(
            ai.tick(1600, SPAWN, flank),
            Some(BulwarkAction::Slam {
                facing: Facing::West,
                damage: 0
            })
        );
        assert!(!ai.blocks(SPAWN, ENTRANCE));
        assert_eq!(ai.tick(3399, SPAWN, flank), None);
        assert_eq!(
            ai.tick(3400, SPAWN, flank),
            Some(BulwarkAction::Brace {
                facing: Facing::South
            })
        );
        assert!(ai.blocks(SPAWN, flank));
        assert!(!ai.blocks(SPAWN, ENTRANCE));
        assert_eq!(
            ai.tick(5000, SPAWN, flank),
            Some(BulwarkAction::Slam {
                facing: Facing::South,
                damage: DAMAGE
            })
        );
    }

    #[test]
    fn bulwark_delayed_confirmation_cannot_shorten_the_new_warning() {
        let mut ai = SteelBulwark::default();
        ai.confirm_at(1000);
        assert_eq!(ai.tick(2599, SPAWN, ENTRANCE), None);
        assert!(matches!(
            ai.tick(2600, SPAWN, ENTRANCE),
            Some(BulwarkAction::Slam { damage: DAMAGE, .. })
        ));
        ai.confirm_at(5000);
        assert_eq!(ai.tick(6799, SPAWN, ENTRANCE), None);
        assert!(matches!(
            ai.tick(6800, SPAWN, ENTRANCE),
            Some(BulwarkAction::Brace { .. })
        ));
        assert_eq!(ai.tick(6800, SPAWN, ENTRANCE), None);
    }

    #[test]
    fn locked_facing_allows_flanking_without_rotating_toward_each_shot() {
        let origin = [8, 0, 8];
        let facing = Facing::North.toward(origin, [4, 0, 8]);
        assert_eq!(facing, Facing::West);
        assert!(facing.protects(origin, [4, 0, 8]));
        assert!(facing.protects(origin, [6, 0, 6]));
        assert!(!facing.protects(origin, [8, 0, 6]));
        assert!(!facing.protects(origin, [10, 0, 8]));
        assert!(!facing.protects(origin, origin));
        assert!(!facing.protects(origin, [4, 1, 8]));
        assert_eq!(facing.toward(origin, origin), facing);
    }

    #[test]
    fn cardinal_sectors_and_extreme_coordinates_are_well_defined() {
        for (facing, target) in [
            (Facing::North, [0, 0, -3]),
            (Facing::East, [3, 0, 0]),
            (Facing::South, [0, 0, 3]),
            (Facing::West, [-3, 0, 0]),
        ] {
            assert_eq!(Facing::North.toward([0, 0, 0], target), facing);
            assert!(facing.protects([0, 0, 0], target));
        }
        assert!(Facing::East.protects([i32::MIN, 0, 0], [i32::MAX, 0, 0]));
    }
}
