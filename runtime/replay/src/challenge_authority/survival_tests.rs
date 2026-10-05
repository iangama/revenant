use super::{admission, checkpoint, event, reconstruct, ChallengeState, ContractId, ReplayEvent};
use crate::SurvivalEvidence;
use revenant_challenges::survival::{
    Action, Effect, SurvivalRun, CHARGE_MS, EAST_RELAY, RECOVERY_MS, RESERVE_PAD, SPAWN, WEST_RELAY,
};

struct Fixture {
    events: Vec<ReplayEvent>,
    state: ChallengeState,
    world: SurvivalRun,
    proof: SurvivalEvidence,
}

impl Fixture {
    fn new() -> Self {
        let (mut events, state) =
            admission(ContractId::LastReserve, &ChallengeState::default(), false);
        let world = SurvivalRun::new(&state.active.as_ref().unwrap().equipment).unwrap();
        let proof = SurvivalEvidence {
            run_id: 1,
            sequence: 1,
            elapsed_ms: 0,
            enemy_id: 2,
            action: None,
            effect: Effect::default(),
        };
        event(
            &mut events,
            proof.kind(),
            proof.actor(1),
            proof.encode().unwrap(),
        );
        Self {
            events,
            state,
            world,
            proof,
        }
    }

    fn step(&mut self, after_ms: u64, action: Action) {
        self.proof.sequence += 1;
        self.proof.elapsed_ms += after_ms;
        self.proof.action = Some(action);
        let update = self.world.apply(self.proof.elapsed_ms, action).unwrap();
        self.proof.effect = update.effect;
        self.world = update.run;
        let id = event(
            &mut self.events,
            self.proof.kind(),
            self.proof.actor(1),
            self.proof.encode().unwrap(),
        );
        if let Some(command) = self.proof.command() {
            self.state = checkpoint(&mut self.events, &self.state, command, Some(id));
        }
    }

    fn walk(&mut self, target: [i32; 3]) {
        while self.world.position() != target {
            let mut position = self.world.position();
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
}

fn fixture(win: bool) -> Fixture {
    let mut f = Fixture::new();
    f.walk([-8, 0, 4]);
    f.walk(WEST_RELAY);
    if !win {
        while f.state.active.is_some() {
            f.step(1_800, Action::Observe);
        }
        return f;
    }
    f.hold(CHARGE_MS);
    f.walk([-8, 0, 4]);
    f.walk(RESERVE_PAD);
    f.hold(RECOVERY_MS);
    assert!(f.world.reserve_used());
    f.walk([0, 0, 5]);
    f.walk(EAST_RELAY);
    f.hold(CHARGE_MS);
    f.walk([0, 0, 4]);
    f.walk(SPAWN);
    assert!(f.state.active.is_none());
    f
}

#[test]
fn challenge_survival_reconstructs_holds_reserve_victory_and_defeat_without_rewards() {
    for win in [true, false] {
        let f = fixture(win);
        let replay = reconstruct(&f.events).unwrap();
        assert_eq!(replay.challenge.unwrap().state, f.state);
        assert_eq!(replay.loot_grants, 0);
        assert_eq!(replay.progression_grants, 0);
        assert_eq!(f.state.records.len(), usize::from(win));
    }
}

#[test]
fn challenge_survival_rejects_forged_identity_healing_holds_damage_and_missing_steps() {
    let f = fixture(true);
    let healing = f
        .events
        .iter()
        .position(|e| {
            SurvivalEvidence::decode(&e.payload).is_ok_and(|p| p.effect.recovered_health > 0)
        })
        .unwrap();
    for change in 0..8 {
        let mut events = f.events.clone();
        let mut proof = SurvivalEvidence::decode(&events[healing].payload).unwrap();
        match change {
            0 => proof.sequence += 1,
            1 => proof.run_id += 1,
            2 => proof.enemy_id = 99,
            3 => proof.effect.recovered_health += 1,
            4 => proof.elapsed_ms -= 1,
            5 => proof.effect.incoming_damage = 12,
            6 => events[healing].actor_id = Some(99),
            7 => {
                proof.action = Some(Action::Move {
                    position: WEST_RELAY,
                });
            }
            _ => unreachable!(),
        }
        events[healing].payload = proof.encode().unwrap();
        assert!(reconstruct(&events).is_err(), "forged reserve {change}");
    }
    let mut missing = f.events.clone();
    missing.remove(healing - 1);
    assert!(reconstruct(&missing).is_err());
    let mut split = f.events.clone();
    split.pop();
    assert!(reconstruct(&split).is_err());
    let mut duplicated = f.events.clone();
    let repeated = duplicated[healing].clone();
    duplicated.insert(healing + 1, repeated);
    assert!(reconstruct(&duplicated).is_err());
}

#[test]
fn challenge_survival_rejects_unproved_checkpoint_and_preserves_prior_records() {
    let f = fixture(true);
    let mut missing = f.events.clone();
    let first = missing
        .iter()
        .position(|e| {
            SurvivalEvidence::decode(&e.payload).is_ok_and(|p| p.effect.checkpoint.is_some())
        })
        .unwrap();
    missing.remove(first);
    assert!(reconstruct(&missing).is_err());
    let (events, next) = admission(ContractId::MeridianCircuit, &f.state, false);
    assert_eq!(reconstruct(&events).unwrap().challenge.unwrap().state, next);
    assert_eq!(next.records, f.state.records);
}
