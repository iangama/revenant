use super::{ContractId, Fixture, Outcome, ServerMessage, SessionStage, SharedSession};
use revenant_challenges::gauntlet::{Phase, Stage};
use revenant_replay::GauntletAction;
use std::sync::mpsc::Receiver;

struct Driver {
    session: SharedSession,
    messages: Receiver<ServerMessage>,
    output: Vec<ServerMessage>,
}

impl Driver {
    fn new(f: &Fixture, retry: Option<u64>) -> Self {
        let (session, messages) = f.enter(ContractId::RelayGauntlet, retry);
        let mut result = Self {
            session,
            messages,
            output: vec![],
        };
        result.drain();
        result
    }
    fn drain(&mut self) {
        for message in self.messages.try_iter() {
            assert!(!matches!(
                message,
                ServerMessage::LootGranted(_)
                    | ServerMessage::ProgressionGranted(_)
                    | ServerMessage::ActivityComplete(_)
            ));
            self.output.push(message);
        }
    }
    fn now(&self) -> u64 {
        self.session
            .challenge
            .as_ref()
            .unwrap()
            .gauntlet
            .as_ref()
            .unwrap()
            .last_elapsed_ms
    }
    fn phase(&self) -> Phase {
        self.session
            .challenge
            .as_ref()
            .unwrap()
            .gauntlet
            .as_ref()
            .unwrap()
            .world
            .phase()
    }
    fn walk(&mut self, target: [i32; 3]) {
        while self.session.actors.get(41).unwrap().position != target {
            let mut next = self.session.actors.get(41).unwrap().position;
            let axis = if next[0] == target[0] { 2 } else { 0 };
            next[axis] += (target[axis] - next[axis]).signum();
            self.session
                .gauntlet_step_at(
                    41,
                    self.now() + 100,
                    GauntletAction::Move { position: next },
                )
                .unwrap();
            assert_eq!(self.session.actors.get(41).unwrap().position, next);
            self.drain();
        }
    }
    fn attack(&mut self, target: u64, time: u64) {
        self.session
            .attack_at(41, target, time)
            .unwrap_or_else(|e| {
                panic!("gauntlet {:?} attack {target} at {time}: {e}", self.phase())
            });
        self.drain();
    }
    fn prism_tick(&mut self, time: u64) {
        self.session.tick_prism_at(time).unwrap();
        self.drain();
    }
    fn support(&mut self) {
        self.walk([5, 0, 8]);
        // Persistence may take longer than the original fixed 1.2/3s ticks
        // under the full PostgreSQL suite. Keep the entire warning observable.
        let mut time = self.combat_time().max(1200);
        self.session.tick_support_at(time).unwrap();
        self.drain();
        time = self.combat_time().max(time) + 1800;
        self.session.tick_support_at(time).unwrap();
        self.drain();
        assert_eq!(self.session.actors.get(41).unwrap().health, 82);
        let pair = self.session.support.as_ref().unwrap();
        let ids = [pair.mender_id, pair.lancer_id];
        for target in ids {
            while self.session.actors.get(target).is_some() {
                time = time.max(self.combat_time());
                self.attack(target, time);
                time += 250;
            }
        }
        assert_eq!(self.phase(), Phase::Transfer(Stage::Support));
    }
    fn combat_time(&self) -> u64 {
        u64::try_from(self.session.combat_started.elapsed().as_millis()).unwrap()
    }
    fn transfer(&mut self, stage: Stage) {
        self.walk(stage.entrance());
        self.session
            .gauntlet_step_at(41, self.now() + 1200, GauntletAction::Observe)
            .unwrap();
        self.drain();
        assert_eq!(self.phase(), Phase::Combat(stage.next().unwrap()));
        assert_eq!(
            self.session.actors.get(41).unwrap().position,
            stage.next().unwrap().entrance()
        );
    }
    fn bastion(&mut self) {
        self.walk([10, 0, -8]);
        let pair = self.session.elite.as_ref().unwrap();
        let ids = [pair.partner_id, pair.bulwark_id];
        // Keep deterministic combat time ahead of real persistence latency.
        let mut time = 10_000;
        for target in ids {
            while self.session.actors.get(target).is_some() {
                self.attack(target, time);
                time += 250;
            }
        }
        assert_eq!(self.phase(), Phase::Transfer(Stage::Bastion));
    }
    fn last_hit(&mut self) -> u64 {
        self.support();
        self.transfer(Stage::Support);
        assert_eq!(self.session.actors.get(41).unwrap().health, 100);
        self.bastion();
        self.transfer(Stage::Bastion);
        let boss = self.session.enemy_id.unwrap();
        self.attack(boss, 0);
        self.prism_tick(700);
        self.walk([5, 0, 1]);
        self.prism_tick(2500);
        for time in [2500, 2750, 3000, 3250] {
            self.attack(boss, time);
        }
        assert_eq!(self.session.actors.get(boss).unwrap().health, 160);
        self.attack(boss, 3500);
        self.walk([6, 0, 0]);
        for time in [5650, 7850, 8450] {
            self.prism_tick(time);
        }
        self.walk([8, 0, 0]);
        self.prism_tick(10650);
        for time in [10650, 10900, 11150] {
            self.attack(boss, time);
        }
        assert_eq!(self.session.actors.get(boss).unwrap().health, 40);
        boss
    }
}

#[test]
fn challenge_world_gauntlet_carries_health_recovers_and_completes_three_stages_without_grants() {
    let Some(f) = Fixture::new() else { return };
    let mut d = Driver::new(&f, None);
    let boss = d.last_hit();
    d.attack(boss, 11400);
    assert_eq!(d.session.stage, SessionStage::Complete);
    assert_eq!(d.phase(), Phase::Completed);
    let state = f.state();
    assert_eq!(state.last_result.unwrap().outcome, Outcome::Completed);
    assert_eq!(state.records[0].rules.seed, 37008);
    assert_eq!(state.records[0].objectives, 7);
    assert!(d.output.iter().any(|m| matches!(m,ServerMessage::ChallengeGauntletState(s) if s.recovered_health==18 && s.stage==2 && s.health==100)));
    assert!(d
        .output
        .iter()
        .any(|m| matches!(m,ServerMessage::ChallengeGauntletState(s) if s.phase=="completed")));
}

#[test]
fn challenge_world_gauntlet_storage_failures_do_not_publish_move_transfer_or_final_hit() {
    for boundary in ["move", "transfer", "final"] {
        let Some(f) = Fixture::new() else { return };
        let mut d = Driver::new(&f, None);
        let boss = if boundary == "final" {
            Some(d.last_hit())
        } else {
            None
        };
        if boundary == "transfer" {
            d.support();
            d.walk(Stage::Support.entrance());
        }
        d.drain();
        let actor = d.session.actors.get(41).unwrap().clone();
        let phase = d.phase();
        let run = f.state().active.unwrap().run_id;
        f.end_externally(&d.session);
        let failed = if let Some(boss) = boss {
            d.session.attack_at(41, boss, 11400)
        } else {
            d.session.gauntlet_step_at(
                41,
                d.now() + 1200,
                if boundary == "move" {
                    GauntletAction::Move {
                        position: [3, 0, 6],
                    }
                } else {
                    GauntletAction::Observe
                },
            )
        };
        assert!(failed.is_err(), "{boundary}");
        assert_eq!(d.session.actors.get(41).unwrap(), &actor);
        assert_eq!(d.phase(), phase);
        assert!(d.messages.try_recv().is_err());
        assert!(f.state().records.is_empty());
        if let Some(boss) = boss {
            assert_eq!(d.session.actors.get(boss).unwrap().health, 40);
        }
        let mut retry = Driver::new(&f, Some(run));
        assert_eq!(retry.phase(), Phase::Combat(Stage::Support));
        assert_eq!(retry.session.actors.get(41).unwrap().health, 100);
        assert_eq!(f.state().active.unwrap().objectives, 0);
        retry.session.disconnect(41).unwrap();
    }
}

#[test]
fn challenge_world_gauntlet_defeat_is_atomic_and_retry_starts_at_first_pair() {
    let Some(f) = Fixture::new() else { return };
    let mut d = Driver::new(&f, None);
    d.walk([5, 0, 8]);
    for time in (1200..30_000).step_by(100) {
        d.session.tick_support_at(time).unwrap();
        d.drain();
        if d.session.actors.get(41).unwrap().health == 0 {
            break;
        }
    }
    assert_eq!(d.phase(), Phase::Defeated);
    assert_eq!(d.session.stage, SessionStage::Failed);
    let state = f.state();
    assert_eq!(state.last_result.unwrap().outcome, Outcome::Defeated);
    assert!(state.records.is_empty());
    assert!(d.output.iter().any(|m| matches!(m,ServerMessage::ChallengeGauntletState(s) if s.phase=="defeated" && s.stage==1 && s.health==0)));
    let mut retry = Driver::new(&f, Some(1));
    assert_eq!(retry.phase(), Phase::Combat(Stage::Support));
    retry.session.disconnect(41).unwrap();
}

#[test]
fn challenge_world_single_reserve_uses_separate_pad_and_persists_only_one_heal() {
    use revenant_challenges::modifiers::Preset;
    let Some(f) = Fixture::new() else { return };
    let (session, messages) =
        f.enter_preset(ContractId::RelayGauntlet, None, Preset::SingleReservePace);
    let mut d = Driver {
        session,
        messages,
        output: vec![],
    };
    d.drain();
    d.support();
    d.walk([3, 0, 6]);
    d.session
        .gauntlet_step_at(41, d.now() + 1199, GauntletAction::Observe)
        .unwrap();
    assert_eq!(d.session.actors.get(41).unwrap().health, 82);
    d.walk([4, 0, 6]);
    d.walk([3, 0, 6]);
    d.session
        .gauntlet_step_at(41, d.now() + 1200, GauntletAction::Observe)
        .unwrap();
    d.drain();
    assert_eq!(d.session.actors.get(41).unwrap().health, 100);
    assert_eq!(d.phase(), Phase::Transfer(Stage::Support));
    assert!(d.output.iter().any(|m| matches!(m, ServerMessage::ChallengeGauntletState(s) if s.recovered_health == 18 && s.modifier.as_ref().is_some_and(|p| p.reserve_used))));
    d.transfer(Stage::Support);
    assert_eq!(
        f.state().active.unwrap().rules.preset,
        Preset::SingleReservePace
    );
    assert_eq!(d.session.actors.get(41).unwrap().health, 100);
    let (restarted, _) = f.enter_preset(
        ContractId::RelayGauntlet,
        Some(1),
        Preset::SingleReservePace,
    );
    assert_eq!(restarted.actors.get(41).unwrap().health, 100);
    assert!(!restarted
        .challenge
        .as_ref()
        .unwrap()
        .gauntlet
        .as_ref()
        .unwrap()
        .world
        .reserve_used());
}
