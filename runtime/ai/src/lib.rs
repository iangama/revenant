use revenant_actors::{ActorKind, ActorRegistry};
use revenant_combat::PressureProfile;

pub mod bulwark;
pub mod elite;
pub mod lancer;
pub mod prism;
pub mod sentinel;
pub mod support;

pub const DETECTION_RANGE: i32 = 12;
pub const ATTACK_RANGE: i32 = 2;
pub const RELAY_DRONE_PRESSURE_PROFILE: PressureProfile = PressureProfile {
    damage: 10,
    initial_attacks: 1,
    accepted_hits_per_counterattack: 0,
    maximum_counterattacks: 0,
};
pub const WARDEN_PRESSURE_PROFILE: PressureProfile = PressureProfile {
    damage: 15,
    initial_attacks: 0,
    accepted_hits_per_counterattack: 2,
    maximum_counterattacks: 2,
};

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
        killed: bool,
    },
}

#[derive(Debug, Clone)]
pub struct AiController {
    state: AiState,
    pressure: PressureProfile,
    initial_attacks: u32,
    accepted_hits: u32,
    counterattacks: u32,
}

impl Default for AiController {
    fn default() -> Self {
        Self::new(RELAY_DRONE_PRESSURE_PROFILE)
    }
}

impl AiController {
    #[must_use]
    pub const fn new(pressure: PressureProfile) -> Self {
        Self {
            state: AiState::Idle,
            pressure,
            initial_attacks: 0,
            accepted_hits: 0,
            counterattacks: 0,
        }
    }

    #[must_use]
    pub fn state(&self) -> AiState {
        self.state
    }

    #[must_use]
    pub fn pressure_ready(&self) -> bool {
        self.state == AiState::Attack && self.initial_attacks >= self.pressure.initial_attacks
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
            AiState::Attack if self.initial_attacks < self.pressure.initial_attacks => {
                self.initial_attacks += 1;
                self.attack_target(actors, enemy_id, player_id)
                    .into_iter()
                    .collect()
            }
            AiState::Attack | AiState::Idle | AiState::Dead => Vec::new(),
        }
    }

    pub fn accepted_hit(
        &mut self,
        actors: &mut ActorRegistry,
        enemy_id: u64,
        player_id: Option<u64>,
    ) -> Option<AiEvent> {
        if self.state != AiState::Attack {
            return None;
        }
        self.accepted_hits = self.accepted_hits.saturating_add(1);
        if self.pressure.maximum_counterattacks == 0
            || self.pressure.accepted_hits_per_counterattack == 0
            || self.counterattacks >= self.pressure.maximum_counterattacks
            || !self
                .accepted_hits
                .is_multiple_of(self.pressure.accepted_hits_per_counterattack)
        {
            return None;
        }
        self.counterattacks += 1;
        self.attack_target(actors, enemy_id, player_id?)
    }

    fn attack_target(
        &self,
        actors: &mut ActorRegistry,
        enemy_id: u64,
        player_id: u64,
    ) -> Option<AiEvent> {
        let enemy = actors.get(enemy_id)?;
        let player = actors.get(player_id)?;
        if enemy.kind != ActorKind::Enemy
            || enemy.health == 0
            || player.kind != ActorKind::Player
            || player.health == 0
        {
            return None;
        }
        let player = actors.apply_damage(player_id, self.pressure.damage)?;
        Some(AiEvent::Attacked {
            source_actor_id: enemy_id,
            target_actor_id: player_id,
            damage: self.pressure.damage,
            remaining_health: player.health,
            killed: player.health == 0,
        })
    }

    /// Moves the Arc Surge Warden between two relay-core firing positions.
    /// Called only after a confirmed, nonlethal hit in that routed encounter.
    pub fn reposition_arc_warden(
        &self,
        actors: &mut ActorRegistry,
        enemy_id: u64,
        player_id: u64,
    ) -> Option<AiEvent> {
        let enemy = actors.get(enemy_id)?;
        let player = actors.get(player_id)?;
        if self.state != AiState::Attack
            || self.accepted_hits == 0
            || enemy.kind != ActorKind::Enemy
            || enemy.archetype != "warden"
            || enemy.health == 0
            || player.kind != ActorKind::Player
            || player.health == 0
        {
            return None;
        }
        let position = if self.accepted_hits.is_multiple_of(2) {
            [10, 0, 3]
        } else {
            [8, 0, -3]
        };
        if enemy.position == position {
            return None;
        }
        actors.update_position(enemy_id, position)?;
        Some(AiEvent::Moved {
            actor_id: enemy_id,
            position,
        })
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

    use super::{
        AiController, AiEvent, AiState, RELAY_DRONE_PRESSURE_PROFILE, WARDEN_PRESSURE_PROFILE,
    };

    fn engage(
        ai: &mut AiController,
        actors: &mut ActorRegistry,
        enemy_id: u64,
        player_id: u64,
    ) -> Vec<AiEvent> {
        let mut events = Vec::new();
        for _ in 0..16 {
            events.extend(ai.tick(actors, enemy_id, player_id));
            if ai.pressure_ready() {
                return events;
            }
        }
        panic!("enemy should reach its pressure-ready state");
    }

    #[test]
    fn arc_warden_repositions_only_after_hits_with_live_actors() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", [6, 0, 0], 100);
        let enemy = actors.spawn(ActorKind::Enemy, "warden", [8, 0, 0], 240);
        let mut ai = AiController::new(WARDEN_PRESSURE_PROFILE);
        engage(&mut ai, &mut actors, enemy.id, player.id);
        assert_eq!(
            ai.reposition_arc_warden(&mut actors, enemy.id, player.id),
            None
        );
        ai.accepted_hit(&mut actors, enemy.id, Some(player.id));
        assert!(matches!(
            ai.reposition_arc_warden(&mut actors, enemy.id, player.id),
            Some(AiEvent::Moved {
                position: [8, 0, -3],
                ..
            })
        ));
        assert_eq!(
            ai.reposition_arc_warden(&mut actors, enemy.id, player.id),
            None
        );
        ai.accepted_hit(&mut actors, enemy.id, Some(player.id));
        assert!(matches!(
            ai.reposition_arc_warden(&mut actors, enemy.id, player.id),
            Some(AiEvent::Moved {
                position: [10, 0, 3],
                ..
            })
        ));
        ai.accepted_hit(&mut actors, enemy.id, Some(player.id));
        actors.apply_damage(player.id, 100);
        assert_eq!(
            ai.reposition_arc_warden(&mut actors, enemy.id, player.id),
            None
        );
        actors.destroy(enemy.id);
        assert_eq!(
            ai.reposition_arc_warden(&mut actors, enemy.id, player.id),
            None
        );
    }

    #[test]
    fn enemy_detects_chases_and_damages_player() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", [0, 0, 0], 100);
        let enemy = actors.spawn(ActorKind::Enemy, "relay-drone", [4, 0, 2], 100);
        let mut ai = AiController::new(RELAY_DRONE_PRESSURE_PROFILE);
        let events = engage(&mut ai, &mut actors, enemy.id, player.id);

        assert_eq!(ai.state(), AiState::Attack);
        assert!(events
            .iter()
            .any(|event| matches!(event, AiEvent::Moved { .. })));
        assert_eq!(actors.get(player.id).unwrap().health, 90);
        assert!(ai.tick(&mut actors, enemy.id, player.id).is_empty());
        assert_eq!(actors.get(player.id).unwrap().health, 90);
    }

    #[test]
    fn warden_counterattacks_every_second_hit_and_targets_the_triggering_player() {
        let mut actors = ActorRegistry::default();
        let player_one = actors.spawn(ActorKind::Player, "operator-one", [0, 0, 0], 100);
        let player_two = actors.spawn(ActorKind::Player, "operator-two", [0, 0, 0], 100);
        let enemy = actors.spawn(ActorKind::Enemy, "warden", [2, 0, 0], 240);
        let mut ai = AiController::new(WARDEN_PRESSURE_PROFILE);
        let events = engage(&mut ai, &mut actors, enemy.id, player_one.id);
        assert!(!events
            .iter()
            .any(|event| matches!(event, AiEvent::Attacked { .. })));

        assert_eq!(
            ai.accepted_hit(&mut actors, enemy.id, Some(player_one.id)),
            None
        );
        let first = ai
            .accepted_hit(&mut actors, enemy.id, Some(player_two.id))
            .expect("second accepted hit should trigger a counterattack");
        assert!(matches!(
            first,
            AiEvent::Attacked {
                target_actor_id,
                damage: 15,
                remaining_health: 85,
                killed: false,
                ..
            } if target_actor_id == player_two.id
        ));
        assert_eq!(actors.get(player_one.id).unwrap().health, 100);
        assert_eq!(actors.get(player_two.id).unwrap().health, 85);

        assert_eq!(
            ai.accepted_hit(&mut actors, enemy.id, Some(player_two.id)),
            None
        );
        let second = ai
            .accepted_hit(&mut actors, enemy.id, Some(player_one.id))
            .expect("fourth accepted hit should trigger the final counterattack");
        assert!(matches!(
            second,
            AiEvent::Attacked {
                target_actor_id,
                damage: 15,
                remaining_health: 85,
                killed: false,
                ..
            } if target_actor_id == player_one.id
        ));
        for _ in 0..4 {
            assert_eq!(
                ai.accepted_hit(&mut actors, enemy.id, Some(player_one.id)),
                None
            );
        }
        assert_eq!(actors.get(player_one.id).unwrap().health, 85);
        assert_eq!(actors.get(player_two.id).unwrap().health, 85);
    }

    #[test]
    fn hostile_pressure_stops_at_player_death_enemy_death_and_target_loss() {
        let mut actors = ActorRegistry::default();
        let player = actors.spawn(ActorKind::Player, "operator", [0, 0, 0], 15);
        let enemy = actors.spawn(ActorKind::Enemy, "warden", [2, 0, 0], 240);
        let mut ai = AiController::new(WARDEN_PRESSURE_PROFILE);
        engage(&mut ai, &mut actors, enemy.id, player.id);

        assert_eq!(
            ai.accepted_hit(&mut actors, enemy.id, Some(player.id)),
            None
        );
        assert!(matches!(
            ai.accepted_hit(&mut actors, enemy.id, Some(player.id)),
            Some(AiEvent::Attacked {
                remaining_health: 0,
                killed: true,
                ..
            })
        ));
        assert_eq!(
            ai.accepted_hit(&mut actors, enemy.id, Some(player.id)),
            None
        );
        assert_eq!(
            ai.accepted_hit(&mut actors, enemy.id, Some(player.id)),
            None
        );
        assert_eq!(actors.get(player.id).unwrap().health, 0);

        let live_player = actors.spawn(ActorKind::Player, "live-operator", [0, 0, 0], 100);
        let defeated_enemy = actors.spawn(ActorKind::Enemy, "warden", [2, 0, 0], 240);
        let mut defeated_enemy_ai = AiController::new(WARDEN_PRESSURE_PROFILE);
        engage(
            &mut defeated_enemy_ai,
            &mut actors,
            defeated_enemy.id,
            live_player.id,
        );
        actors.apply_damage(defeated_enemy.id, 240);
        assert_eq!(
            defeated_enemy_ai.accepted_hit(&mut actors, defeated_enemy.id, Some(live_player.id)),
            None
        );
        assert_eq!(
            defeated_enemy_ai.accepted_hit(&mut actors, defeated_enemy.id, Some(live_player.id)),
            None
        );
        assert_eq!(actors.get(live_player.id).unwrap().health, 100);

        let abandoned_enemy = actors.spawn(ActorKind::Enemy, "warden", [2, 0, 0], 240);
        let mut abandoned_ai = AiController::new(WARDEN_PRESSURE_PROFILE);
        engage(
            &mut abandoned_ai,
            &mut actors,
            abandoned_enemy.id,
            live_player.id,
        );
        assert_eq!(
            abandoned_ai.accepted_hit(&mut actors, abandoned_enemy.id, Some(live_player.id)),
            None
        );
        actors.destroy(live_player.id);
        assert_eq!(
            abandoned_ai.accepted_hit(&mut actors, abandoned_enemy.id, None),
            None
        );
    }
}
