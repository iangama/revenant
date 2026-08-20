use revenant_actors::ActorRegistry;

pub const DETECTION_RANGE: i32 = 12;
pub const ATTACK_RANGE: i32 = 2;
pub const ENEMY_ATTACK_DAMAGE: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Detect,
    Chase,
    Attack,
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiEvent {
    StateChanged(AiState),
    Moved {
        actor_id: u64,
        position: [i32; 3],
    },
    Attacked {
        source_actor_id: u64,
        target_actor_id: u64,
        damage: u32,
        remaining_health: u32,
    },
}

#[derive(Debug)]
pub struct AiController {
    state: AiState,
}

impl Default for AiController {
    fn default() -> Self {
        Self {
            state: AiState::Idle,
        }
    }
}

impl AiController {
    #[must_use]
    pub fn state(&self) -> AiState {
        self.state
    }

    pub fn tick(
        &mut self,
        actors: &mut ActorRegistry,
        enemy_id: u64,
        player_id: u64,
    ) -> Vec<AiEvent> {
        let Some(enemy) = actors.get(enemy_id).cloned() else {
            self.state = AiState::Dead;
            return vec![AiEvent::StateChanged(AiState::Dead)];
        };
        let Some(player) = actors.get(player_id).cloned() else {
            return Vec::new();
        };
        let distance_squared = squared_distance(enemy.position, player.position);

        match self.state {
            AiState::Idle if distance_squared <= i64::from(DETECTION_RANGE).pow(2) => {
                self.state = AiState::Detect;
                vec![AiEvent::StateChanged(self.state)]
            }
            AiState::Detect => {
                self.state = AiState::Chase;
                vec![AiEvent::StateChanged(self.state)]
            }
            AiState::Chase if distance_squared <= i64::from(ATTACK_RANGE).pow(2) => {
                self.state = AiState::Attack;
                vec![AiEvent::StateChanged(self.state)]
            }
            AiState::Chase => {
                let mut position = enemy.position;
                for (coordinate, target) in position.iter_mut().zip(player.position) {
                    *coordinate += (target - *coordinate).signum();
                }
                actors.update_position(enemy_id, position);
                vec![AiEvent::Moved {
                    actor_id: enemy_id,
                    position,
                }]
            }
            AiState::Attack => actors
                .apply_damage(player_id, ENEMY_ATTACK_DAMAGE)
                .map_or_else(Vec::new, |player| {
                    vec![AiEvent::Attacked {
                        source_actor_id: enemy_id,
                        target_actor_id: player_id,
                        damage: ENEMY_ATTACK_DAMAGE,
                        remaining_health: player.health,
                    }]
                }),
            AiState::Idle | AiState::Dead => Vec::new(),
        }
    }
}

fn squared_distance(left: [i32; 3], right: [i32; 3]) -> i64 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| (i64::from(left) - i64::from(right)).pow(2))
        .sum()
}

#[cfg(test)]
mod tests {
    use revenant_actors::{ActorKind, ActorRegistry};

    use super::{AiController, AiEvent, AiState};

    #[test]
    fn enemy_detects_chases_and_damages_player() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", [0, 0, 0], 100);
        let enemy = actors.spawn(ActorKind::Enemy, "relay-drone", [4, 0, 2], 100);
        let mut ai = AiController::default();
        let mut events = Vec::new();
        for _ in 0..10 {
            events.extend(ai.tick(&mut actors, enemy.id, player.id));
            if events
                .iter()
                .any(|event| matches!(event, AiEvent::Attacked { .. }))
            {
                break;
            }
        }

        assert_eq!(ai.state(), AiState::Attack);
        assert!(events
            .iter()
            .any(|event| matches!(event, AiEvent::Moved { .. })));
        assert_eq!(actors.get(player.id).unwrap().health, 90);
    }
}
