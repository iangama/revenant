use std::{error::Error, time::Instant};

use revenant_ai::sentinel::{
    cover_blocks_segment, SignalSentinel, SENTINEL_HEALTH, SENTINEL_POSITION, SIGNAL_APPROACH,
    SIGNAL_TERMINAL,
};
use revenant_operations::RoutePhase;

use super::{
    actor_spawn_message, ActorDestroy, ActorKind, AiEvent, CombatRuntime, ObjectiveUpdate,
    ProtocolGeneration, ReplayEventKind, ServerMessage, SessionStage, SharedSession,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Fighting,
    Recovery,
    Recovered,
    Failed,
}

pub(super) struct SignalEncounter {
    enemy_id: u64,
    player_id: u64,
    phase: Phase,
    started_at: Instant,
    sentinel: SignalSentinel,
}

impl SharedSession {
    pub(super) fn signal_fighting(&self) -> bool {
        self.signal
            .as_ref()
            .is_some_and(|signal| signal.phase == Phase::Fighting)
    }

    pub(super) fn signal_shot_blocked(&self, player_id: u64, enemy_id: u64) -> bool {
        self.actors
            .get(player_id)
            .zip(self.actors.get(enemy_id))
            .is_some_and(|(player, enemy)| cover_blocks_segment(player.position, enemy.position))
    }

    pub(super) fn signal_movement_blocked(&self, player_id: u64, position: [i32; 3]) -> bool {
        (self.signal.is_some() || self.signal_available())
            && self
                .actors
                .get(player_id)
                .is_some_and(|player| cover_blocks_segment(player.position, position))
    }

    pub(super) fn handle_signal_movement(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<bool, Box<dyn Error>> {
        if let Some(signal) = self.signal.as_mut() {
            if signal.phase == Phase::Recovery && position == SIGNAL_TERMINAL {
                signal.phase = Phase::Recovered;
                self.broadcast(signal_objective("Completed", 2));
                return Ok(true);
            }
            return Ok(matches!(signal.phase, Phase::Fighting | Phase::Recovery));
        }
        if position != SIGNAL_APPROACH || !self.signal_available() {
            return Ok(false);
        }
        self.start_signal(player_id)?;
        Ok(true)
    }

    fn signal_available(&self) -> bool {
        self.signal.is_none()
            && self.stage == SessionStage::Door
            && self.expected_players == 1
            && self.participants.len() == 1
            && self.participants[0].protocol_generation == ProtocolGeneration::CurrentV2
            && self.cooperation.is_none()
            && self
                .route
                .as_ref()
                .is_some_and(|route| route.operation.phase() == RoutePhase::ChoiceOpen)
    }

    fn start_signal(&mut self, player_id: u64) -> Result<(), Box<dyn Error>> {
        // This optional excursion occupies the normal, unrouted mission path.
        self.route
            .as_mut()
            .expect("eligible route exists")
            .operation
            .lock_baseline()?;
        self.spawn_signal_enemy(player_id)?;
        self.broadcast(signal_objective("Active", 0));
        self.broadcast_route_state("signal excursion selected; standard mission rewards")?;
        Ok(())
    }

    pub(super) fn spawn_signal_enemy(&mut self, player_id: u64) -> Result<(), Box<dyn Error>> {
        let enemy = self.actors.spawn(
            ActorKind::Enemy,
            "signal-sentinel",
            SENTINEL_POSITION,
            SENTINEL_HEALTH,
        );
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::EnemySpawned,
            &owner,
            Some(enemy.id),
            "enemy spawned: signal-sentinel",
        )?;
        self.signal = Some(SignalEncounter {
            enemy_id: enemy.id,
            player_id,
            phase: Phase::Fighting,
            started_at: Instant::now(),
            sentinel: SignalSentinel::new(),
        });
        self.enemy_id = Some(enemy.id);
        self.enemy_ai = None;
        self.combat = CombatRuntime::default();
        self.combat_started = Instant::now();
        self.broadcast(actor_spawn_message(&enemy));
        Ok(())
    }

    pub(super) fn tick_signal(&mut self) {
        let Some(signal) = self.signal.as_ref() else {
            return;
        };
        let elapsed = u64::try_from(signal.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.tick_signal_at(elapsed);
    }

    pub(super) fn tick_signal_at(&mut self, elapsed: u64) {
        let Some(signal) = self.signal.as_mut() else {
            return;
        };
        if signal.phase != Phase::Fighting || self.stage == SessionStage::Failed {
            return;
        }
        let Some(event) =
            signal
                .sentinel
                .tick(&mut self.actors, signal.enemy_id, signal.player_id, elapsed)
        else {
            return;
        };
        let killed = matches!(event, AiEvent::Attacked { killed: true, .. });
        if killed {
            signal.phase = Phase::Failed;
        }
        self.apply_ai_event(&event);
        if killed {
            self.stage = SessionStage::Failed;
            if self.campaign.is_none() {
                self.broadcast(signal_objective("Failed", 0));
            }
        }
    }

    pub(super) fn signal_sentinel_defeated(
        &mut self,
        target_id: u64,
        fatal_damage: ServerMessage,
    ) -> Result<(), Box<dyn Error>> {
        let owner = self.owner_account()?.to_owned();
        self.append_event(
            ReplayEventKind::EnemyDied,
            &owner,
            Some(target_id),
            "enemy died: signal-sentinel",
        )?;
        self.signal
            .as_mut()
            .expect("active signal encounter exists")
            .phase = Phase::Recovery;
        self.actors.destroy(target_id);
        self.enemy_id = None;
        self.broadcast(fatal_damage);
        self.broadcast(ServerMessage::ActorDestroy(ActorDestroy {
            actor_id: target_id,
        }));
        self.broadcast(signal_objective("Active", 1));
        Ok(())
    }
}

fn signal_objective(state: &str, progress: u32) -> ServerMessage {
    ServerMessage::ObjectiveUpdate(ObjectiveUpdate {
        objective_id: "recover_lost_signal".to_owned(),
        objective_type: "ReachArea".to_owned(),
        state: state.to_owned(),
        progress,
        target: 2,
    })
}
