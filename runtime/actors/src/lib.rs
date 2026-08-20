use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ACTOR_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    Player,
    Enemy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: u64,
    pub kind: ActorKind,
    pub archetype: String,
    pub position: [i32; 3],
    pub health: u32,
    pub max_health: u32,
}

#[derive(Debug, Default)]
pub struct ActorRegistry {
    actors: HashMap<u64, Actor>,
}

#[must_use]
pub fn allocate_actor_id() -> u64 {
    NEXT_ACTOR_ID.fetch_add(1, Ordering::Relaxed)
}

impl ActorRegistry {
    pub fn spawn(
        &mut self,
        kind: ActorKind,
        archetype: &str,
        position: [i32; 3],
        max_health: u32,
    ) -> Actor {
        let actor = Actor {
            id: allocate_actor_id(),
            kind,
            archetype: archetype.to_owned(),
            position,
            health: max_health,
            max_health,
        };
        self.actors.insert(actor.id, actor.clone());
        actor
    }

    pub fn insert(&mut self, actor: Actor) {
        self.actors.insert(actor.id, actor);
    }

    pub fn update_position(&mut self, actor_id: u64, position: [i32; 3]) -> Option<Actor> {
        let actor = self.actors.get_mut(&actor_id)?;
        actor.position = position;
        Some(actor.clone())
    }

    pub fn destroy(&mut self, actor_id: u64) -> Option<Actor> {
        self.actors.remove(&actor_id)
    }

    pub fn apply_damage(&mut self, actor_id: u64, damage: u32) -> Option<Actor> {
        let actor = self.actors.get_mut(&actor_id)?;
        actor.health = actor.health.saturating_sub(damage);
        Some(actor.clone())
    }

    #[must_use]
    pub fn get(&self, actor_id: u64) -> Option<&Actor> {
        self.actors.get(&actor_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{ActorKind, ActorRegistry};

    #[test]
    fn actor_lifecycle_is_explicit() {
        let mut registry = ActorRegistry::default();
        let spawned = registry.spawn(ActorKind::Enemy, "relay-drone", [4, 0, 2], 100);
        assert_eq!(registry.get(spawned.id), Some(&spawned));

        let updated = registry
            .update_position(spawned.id, [5, 0, 2])
            .expect("spawned actor should update");
        assert_eq!(updated.position, [5, 0, 2]);

        let damaged = registry
            .apply_damage(spawned.id, 40)
            .expect("spawned actor should take damage");
        assert_eq!(damaged.health, 60);

        assert_eq!(registry.destroy(spawned.id), Some(damaged));
        assert!(registry.get(spawned.id).is_none());
    }
}
