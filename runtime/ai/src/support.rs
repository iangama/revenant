//! Bounded allied repair pressure for the next authored multi-enemy encounter.

use revenant_actors::{Actor, ActorKind};

pub const ENTRANCE: [i32; 3] = [2, 0, 6];
pub const SPAWN: [i32; 3] = [7, 0, 6];
pub const HEALTH: u32 = 120;
pub const REPAIR_AMOUNT: u32 = 8;
pub const REPAIR_INTERVAL_MS: u64 = 1_600;
pub const LINK_RANGE: i64 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairAction {
    pub target_id: u64,
    pub amount: u32,
    pub health_before: u32,
    pub health_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayMender {
    ready_at_ms: u64,
}

impl Default for RelayMender {
    fn default() -> Self {
        Self { ready_at_ms: 900 }
    }
}

impl RelayMender {
    pub fn tick(&mut self, now_ms: u64, source: &Actor, allies: &[Actor]) -> Option<RepairAction> {
        if now_ms < self.ready_at_ms || source.kind != ActorKind::Enemy || source.health == 0 {
            return None;
        }
        let target = allies
            .iter()
            .filter(|ally| eligible(source, ally))
            .min_by(|a, b| {
                (u64::from(a.health) * u64::from(b.max_health))
                    .cmp(&(u64::from(b.health) * u64::from(a.max_health)))
                    .then_with(|| a.id.cmp(&b.id))
            })?;
        let amount = REPAIR_AMOUNT.min(target.max_health - target.health);
        self.ready_at_ms = now_ms.saturating_add(REPAIR_INTERVAL_MS);
        Some(RepairAction {
            target_id: target.id,
            amount,
            health_before: target.health,
            health_after: target.health + amount,
        })
    }

    pub fn confirm_at(&mut self, now_ms: u64) {
        self.ready_at_ms = self
            .ready_at_ms
            .max(now_ms.saturating_add(REPAIR_INTERVAL_MS));
    }
}

fn eligible(source: &Actor, ally: &Actor) -> bool {
    if source.id == ally.id
        || ally.kind != ActorKind::Enemy
        || ally.health == 0
        || ally.health >= ally.max_health
        || source.position[1] != ally.position[1]
    {
        return false;
    }
    let dx = (i64::from(source.position[0]) - i64::from(ally.position[0])).abs();
    let dz = (i64::from(source.position[2]) - i64::from(ally.position[2])).abs();
    dx <= LINK_RANGE && dz <= LINK_RANGE && dx * dx + dz * dz <= LINK_RANGE * LINK_RANGE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enemy(id: u64, health: u32, max_health: u32, position: [i32; 3]) -> Actor {
        Actor {
            id,
            health,
            max_health,
            position,
            kind: ActorKind::Enemy,
            archetype: "fixture".to_owned(),
        }
    }

    #[test]
    fn repair_chooses_lowest_fraction_then_stable_id_and_never_revives() {
        let source = enemy(1, 100, 100, [0, 0, 0]);
        let mut allies = vec![
            enemy(3, 50, 100, [1, 0, 0]),
            enemy(2, 100, 200, [2, 0, 0]),
            enemy(4, 0, 100, [0, 0, 1]),
            enemy(5, 1, 100, [7, 0, 0]),
            source.clone(),
        ];
        let mut ai = RelayMender::default();
        assert_eq!(ai.tick(899, &source, &allies), None);
        assert_eq!(ai.tick(900, &source, &allies).unwrap().target_id, 2);
        allies.reverse();
        assert_eq!(
            RelayMender::default()
                .tick(900, &source, &allies)
                .unwrap()
                .target_id,
            2
        );
        assert_eq!(ai.tick(2499, &source, &allies), None);
        ai.confirm_at(3000);
        assert_eq!(ai.tick(4599, &source, &allies), None);
        assert_eq!(ai.tick(4600, &source, &allies).unwrap().amount, 8);
    }

    #[test]
    fn repair_is_capped_and_stops_when_the_support_dies() {
        let mut source = enemy(1, 100, 100, [0, 0, 0]);
        let mut ally = enemy(2, 98, 100, [6, 0, 0]);
        let mut ai = RelayMender::default();
        let action = ai.tick(900, &source, &[ally.clone()]).unwrap();
        assert_eq!((action.amount, action.health_after), (2, 100));
        source.health = 0;
        assert_eq!(ai.tick(2500, &source, &[ally.clone()]), None);
        source.health = 100;
        ally.position = [i32::MAX, 0, i32::MIN];
        assert_eq!(ai.tick(2500, &source, &[ally]), None);
    }
}
