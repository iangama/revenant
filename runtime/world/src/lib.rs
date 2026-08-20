use revenant_actors::allocate_actor_id;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerActor {
    pub actor_id: u64,
    pub character_id: String,
    pub world_id: String,
    pub position: [f32; 3],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WorldService;

impl WorldService {
    #[must_use]
    pub fn join(self, character_id: &str) -> PlayerActor {
        PlayerActor {
            actor_id: allocate_actor_id(),
            character_id: character_id.to_owned(),
            world_id: "relay-hub".to_owned(),
            position: [0.0, 0.0, 0.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorldService;

    #[test]
    fn join_creates_distinct_server_owned_actor_ids() {
        let first = WorldService.join("character:one");
        let second = WorldService.join("character:two");
        assert_ne!(first.actor_id, second.actor_id);
        assert_eq!(first.world_id, "relay-hub");
    }
}
