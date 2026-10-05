use revenant_actors::{ActorKind, ActorRegistry};

use crate::AiEvent;

pub const SENTINEL_POSITION: [i32; 3] = [-4, 0, -4];
pub const SIGNAL_APPROACH: [i32; 3] = [-4, 0, 4];
pub const SIGNAL_TERMINAL: [i32; 3] = [-4, 0, -3];
pub const SENTINEL_HEALTH: u32 = 200;
pub const SHOT_INTERVAL_MS: u64 = 1_800;
pub const SHOT_DAMAGE: u32 = 12;
pub const SHOT_RANGE_SQUARED: i64 = 144;

#[derive(Debug)]
pub struct SignalSentinel {
    next_shot_ms: u64,
}

impl Default for SignalSentinel {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalSentinel {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_shot_ms: SHOT_INTERVAL_MS,
        }
    }

    pub fn tick(
        &mut self,
        actors: &mut ActorRegistry,
        enemy_id: u64,
        player_id: u64,
        now_ms: u64,
    ) -> Option<AiEvent> {
        if now_ms < self.next_shot_ms {
            return None;
        }
        // Do not accumulate shots while a player is protected or the server is busy.
        self.next_shot_ms = now_ms.saturating_add(SHOT_INTERVAL_MS);
        let enemy = actors.get(enemy_id)?;
        let player = actors.get(player_id)?;
        if enemy.kind != ActorKind::Enemy
            || enemy.health == 0
            || player.kind != ActorKind::Player
            || player.health == 0
            || crate::squared_distance(enemy.position, player.position) > SHOT_RANGE_SQUARED
            || cover_blocks_segment(enemy.position, player.position)
        {
            return None;
        }
        let player = actors.apply_damage(player_id, SHOT_DAMAGE)?;
        Some(AiEvent::Attacked {
            source_actor_id: enemy_id,
            target_actor_id: player_id,
            damage: SHOT_DAMAGE,
            remaining_health: player.health,
            killed: player.health == 0,
        })
    }
}

/// Intersects the west-sector barrier footprint. The same geometry blocks
/// movement and fire in both directions; targets must go around its ends.
#[must_use]
pub fn cover_blocks_segment(from: [i32; 3], to: [i32; 3]) -> bool {
    let mut near: f64 = 0.0;
    let mut far: f64 = 1.0;
    for (axis, minimum, maximum) in [(0, -6.0, -2.0), (2, 0.5, 1.5)] {
        let origin = f64::from(from[axis]);
        let delta = f64::from(to[axis]) - origin;
        if delta == 0.0 {
            if origin < minimum || origin > maximum {
                return false;
            }
        } else {
            let first = (minimum - origin) / delta;
            let last = (maximum - origin) / delta;
            near = near.max(first.min(last));
            far = far.min(first.max(last));
            if near > far {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrier_blocks_both_directions_but_leaves_flanks_open() {
        assert!(cover_blocks_segment(SIGNAL_APPROACH, SENTINEL_POSITION));
        assert!(cover_blocks_segment(SENTINEL_POSITION, SIGNAL_APPROACH));
        assert!(cover_blocks_segment([-4, 0, 2], [-4, 0, 0]));
        assert!(!cover_blocks_segment([-7, 0, 2], [-7, 0, 0]));
        assert!(!cover_blocks_segment([-7, 0, 0], SENTINEL_POSITION));
    }

    #[test]
    fn ranged_fire_respects_cover_range_cadence_and_death() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", SIGNAL_APPROACH, 100);
        let enemy = actors.spawn(
            ActorKind::Enemy,
            "signal-sentinel",
            SENTINEL_POSITION,
            SENTINEL_HEALTH,
        );
        let mut sentinel = SignalSentinel::new();
        assert_eq!(
            sentinel.tick(&mut actors, enemy.id, player.id, SHOT_INTERVAL_MS),
            None
        );
        actors.update_position(player.id, [-7, 0, 0]);
        assert_eq!(
            sentinel.tick(&mut actors, enemy.id, player.id, SHOT_INTERVAL_MS + 1),
            None
        );
        assert!(matches!(
            sentinel.tick(&mut actors, enemy.id, player.id, 2 * SHOT_INTERVAL_MS),
            Some(AiEvent::Attacked {
                damage: 12,
                remaining_health: 88,
                ..
            })
        ));
        actors.update_position(player.id, [12, 0, -12]);
        assert_eq!(
            sentinel.tick(&mut actors, enemy.id, player.id, 3 * SHOT_INTERVAL_MS),
            None
        );
        actors.update_position(player.id, [-7, 0, 0]);
        actors.apply_damage(enemy.id, SENTINEL_HEALTH);
        assert_eq!(
            sentinel.tick(&mut actors, enemy.id, player.id, 4 * SHOT_INTERVAL_MS),
            None
        );
        assert_eq!(actors.get(player.id).unwrap().health, 88);
    }
}
