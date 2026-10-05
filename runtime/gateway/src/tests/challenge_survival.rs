use super::{ContractId, Fixture, Outcome, ServerMessage, SessionStage, SharedSession};
use revenant_challenges::survival::{
    Action, CHARGE_MS, EAST_RELAY, RECOVERY_MS, RESERVE_PAD, SPAWN, WEST_RELAY,
};
use std::sync::mpsc::Receiver;

struct Driver {
    session: SharedSession,
    messages: Receiver<ServerMessage>,
    output: Vec<ServerMessage>,
    now: u64,
}

impl Driver {
    fn new(f: &Fixture, retry: Option<u64>) -> Self {
        let (session, messages) = f.enter(ContractId::LastReserve, retry);
        let output = messages.try_iter().collect();
        Self {
            session,
            messages,
            output,
            now: 0,
        }
    }

    fn step(&mut self, after: u64, action: Action) {
        self.now += after;
        self.session
            .apply_survival_action_at(41, self.now, action)
            .unwrap();
        // Consume the bounded outbound channel just as a connected client does.
        self.output.extend(self.messages.try_iter());
    }

    fn walk(&mut self, target: [i32; 3]) {
        while self.session.actors.get(41).unwrap().position != target {
            let mut position = self.session.actors.get(41).unwrap().position;
            let axis = if position[0] == target[0] { 2 } else { 0 };
            position[axis] += (target[axis] - position[axis]).signum();
            self.step(200, Action::Move { position });
        }
    }

    fn hold(&mut self, duration: u64) {
        for _ in 0..duration / 100 {
            self.step(100, Action::Observe);
        }
    }

    fn reserve(&mut self) {
        self.walk([-8, 0, 4]);
        self.walk(WEST_RELAY);
        self.hold(CHARGE_MS);
        self.walk([-8, 0, 4]);
        self.walk(RESERVE_PAD);
    }

    fn east(&mut self) {
        self.walk([0, 0, 5]);
        self.walk(EAST_RELAY);
        self.hold(CHARGE_MS);
        self.walk([0, 0, 4]);
        self.walk([-3, 0, 4]);
    }
}

#[test]
fn challenge_world_survival_projects_committed_recovery_and_completion_without_rewards() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let mut d = Driver::new(&f, None);
    let enemy = d.session.enemy_id.unwrap();
    d.session.attack_at(41, enemy, 0).unwrap();
    assert_eq!(d.session.actors.get(enemy).unwrap().health, 200);
    d.reserve();
    let damaged = d.session.actors.get(41).unwrap().health;
    d.hold(RECOVERY_MS);
    assert_eq!(
        d.session.actors.get(41).unwrap().health,
        (damaged + 36).min(100)
    );
    d.east();
    d.walk(SPAWN);
    assert_eq!(d.session.stage, SessionStage::Complete);
    assert_eq!(f.state().records[0].rules.seed, 37007);
    assert!(d.output.iter().any(|m| matches!(m, ServerMessage::ChallengeSurvivalState(s) if s.reserve_used && s.recovered_health > 0)));
    assert!(d.output.iter().any(|m| matches!(m, ServerMessage::ChallengeSurvivalState(s) if s.phase == "completed" && s.health > 0 && s.charge_remaining_ms.is_none())));
    assert!(!d.output.iter().any(|m| matches!(
        m,
        ServerMessage::LootGranted(_)
            | ServerMessage::ProgressionGranted(_)
            | ServerMessage::ActivityComplete(_)
    )));
}

#[test]
fn challenge_world_survival_storage_failure_withholds_movement_healing_completion_and_death() {
    for boundary in 0..4 {
        let Some(f) = Fixture::new() else {
            return;
        };
        let mut d = Driver::new(&f, None);
        if boundary == 1 || boundary == 2 {
            d.reserve();
        }
        if boundary == 2 {
            d.hold(RECOVERY_MS);
            d.east();
        }
        if boundary == 3 {
            d.walk([-8, 0, 4]);
            d.walk(WEST_RELAY);
            d.hold(8 * 1800);
        }
        let actor = d.session.actors.get(41).unwrap().clone();
        d.output.clear();
        f.end_externally(&d.session);
        let (after_ms, action) = match boundary {
            0 => (
                200,
                Action::Move {
                    position: [-5, 0, 4],
                },
            ),
            1 => (RECOVERY_MS, Action::Observe),
            2 => (200, Action::Move { position: SPAWN }),
            _ => (1800, Action::Observe),
        };
        assert!(d
            .session
            .apply_survival_action_at(41, d.now + after_ms, action)
            .is_err());
        assert_eq!(d.session.actors.get(41).unwrap(), &actor);
        assert!(d.messages.try_recv().is_err());
        assert!(f.state().records.is_empty());
        let mut retry = Driver::new(&f, Some(1));
        assert_eq!(retry.session.actors.get(41).unwrap().position, SPAWN);
        assert_eq!(retry.session.actors.get(41).unwrap().health, 100);
        assert!(retry.output.iter().any(|m| matches!(m, ServerMessage::ChallengeSurvivalState(s) if !s.reserve_used && s.phase == "west" && s.sequence == 1 && s.run_id > 1)));
        retry.session.disconnect(41).unwrap();
    }
}

#[test]
fn challenge_world_survival_fatal_shot_is_atomic_and_retry_restores_the_reserve() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let mut d = Driver::new(&f, None);
    d.walk([-8, 0, 4]);
    d.walk(WEST_RELAY);
    d.hold(8 * 1800);
    assert_eq!(d.session.actors.get(41).unwrap().health, 4);
    d.output.clear();
    d.hold(1800);
    assert_eq!(d.session.actors.get(41).unwrap().health, 0);
    assert_eq!(d.session.stage, SessionStage::Failed);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Defeated);
    assert!(d.output.iter().any(|m| matches!(m, ServerMessage::DamageApplied(hit) if hit.killed && hit.remaining_health == 0)));
    let mut retry = Driver::new(&f, Some(1));
    assert_eq!(retry.session.actors.get(41).unwrap().health, 100);
    assert!(retry.output.iter().any(|m| matches!(m, ServerMessage::ChallengeSurvivalState(s) if !s.reserve_used && s.health == 100)));
    retry.session.disconnect(41).unwrap();
}
