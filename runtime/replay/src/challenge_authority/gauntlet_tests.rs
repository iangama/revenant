use super::*;
use crate::{EliteTransition, GauntletAction, GauntletEffect, GauntletEvidence, PrismTransition};
use revenant_challenges::gauntlet::Stage;

struct Fixture {
    events: Vec<ReplayEvent>,
    state: ChallengeState,
    sequence: u64,
    time: u64,
    position: [i32; 3],
}

impl Fixture {
    fn new() -> Self {
        let (events, state) =
            admission(ContractId::RelayGauntlet, &ChallengeState::default(), false);
        Self {
            events,
            state,
            sequence: 0,
            time: 0,
            position: Stage::Support.entrance(),
        }
    }

    fn modified(preset: revenant_challenges::modifiers::Preset) -> Self {
        let mut f = Self::new();
        f.events.pop();
        f.state = checkpoint(
            &mut f.events,
            &ChallengeState::default(),
            Command::StartModified {
                contract: ContractId::RelayGauntlet,
                preset,
                equipment: equipment(),
            },
            Some(2),
        );
        f.position = preset.gauntlet_stages()[0].entrance();
        f
    }

    fn append(&mut self, actor: u64, action: GauntletAction, effect: GauntletEffect) {
        self.sequence += 1;
        let evidence = GauntletEvidence {
            run_id: 1,
            sequence: self.sequence,
            elapsed_ms: self.time,
            actor_id: actor,
            action,
            effect,
        };
        let proof = event(
            &mut self.events,
            evidence.kind(),
            actor,
            evidence.encode().unwrap(),
        );
        let rules = &self.state.active.as_ref().unwrap().rules;
        if let Some(command) = evidence.command_for(rules) {
            self.state = checkpoint(&mut self.events, &self.state, command, Some(proof));
        }
    }

    fn walk(&mut self, target: [i32; 3]) {
        while self.position != target {
            let axis = if self.position[0] == target[0] { 2 } else { 0 };
            self.position[axis] += (target[axis] - self.position[axis]).signum();
            self.time += 1;
            self.append(
                1,
                GauntletAction::Move {
                    position: self.position,
                },
                GauntletEffect::default(),
            );
        }
    }

    fn encounter(&mut self, stage: Stage, source: Vec<ReplayEvent>) {
        let offset = match stage {
            Stage::Support => 0,
            Stage::Bastion => 2,
            Stage::Prism => 4,
        };
        let mut previous = 0;
        let combat: Vec<_> = source
            .into_iter()
            .filter_map(|e| {
                GauntletAction::from_combat_payload(&e.payload)
                    .ok()
                    .map(|a| (e, a))
            })
            .collect();
        let last = combat.len() - 1;
        for (index, (event, mut action)) in combat.into_iter().enumerate() {
            let time = remap(&mut action, offset);
            self.time += time - previous;
            previous = time;
            if let Some(p) = position(&action) {
                self.walk(p);
            }
            let actor = event.actor_id.unwrap();
            self.append(
                if actor == 1 { 1 } else { actor + offset },
                action,
                GauntletEffect {
                    cleared: (index == last).then_some(stage),
                    ..GauntletEffect::default()
                },
            );
        }
        assert!(
            reconstruct(&self.events).is_ok(),
            "stage {stage:?}: {:?}",
            reconstruct(&self.events)
        );
        let order = self
            .state
            .active
            .as_ref()
            .or_else(|| self.state.last_result.as_ref().map(|r| &r.run))
            .unwrap()
            .rules
            .preset
            .gauntlet_stages();
        if let Some(next) = order
            .windows(2)
            .find_map(|pair| (pair[0] == stage).then_some(pair[1]))
        {
            self.walk(stage.entrance());
            self.time += 1200;
            self.append(
                1,
                GauntletAction::Observe,
                GauntletEffect {
                    next_stage: Some(next),
                    ..GauntletEffect::default()
                },
            );
            self.position = next.entrance();
        }
    }
}

fn remap(action: &mut GauntletAction, offset: u64) -> u64 {
    match action {
        GauntletAction::Support(e) => e.elapsed_ms,
        GauntletAction::Elite(e) => {
            match &mut e.transition {
                EliteTransition::Started {
                    bulwark_id,
                    partner_id,
                    ..
                } => {
                    *bulwark_id += offset;
                    *partner_id += offset;
                }
                EliteTransition::Attack { target_id, .. } => *target_id += offset,
                _ => {}
            }
            e.elapsed_ms
        }
        GauntletAction::Prism(e) => {
            if let PrismTransition::Started { boss_id, .. } = &mut e.transition {
                *boss_id += offset;
            }
            e.elapsed_ms
        }
        _ => unreachable!(),
    }
}

fn position(action: &GauntletAction) -> Option<[i32; 3]> {
    match action {
        GauntletAction::Support(e) => match e.transition {
            SupportTransition::Attack {
                player_position, ..
            } => Some(player_position),
            _ => None,
        },
        GauntletAction::Elite(e) => match e.transition {
            EliteTransition::Attack {
                player_position, ..
            } => Some(player_position),
            _ => None,
        },
        GauntletAction::Prism(e) => match e.transition {
            PrismTransition::Attack {
                player_position, ..
            }
            | PrismTransition::Step {
                player_position, ..
            } => Some(player_position),
            _ => None,
        },
        _ => None,
    }
}

fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.encounter(Stage::Support, combat().0);
    f.encounter(Stage::Bastion, elite_combat().0);
    f.encounter(Stage::Prism, prism::fixture(true).0);
    f
}

#[test]
fn challenge_gauntlet_reuses_all_three_encounters_and_preserves_one_unrewarded_record() {
    let f = fixture();
    let reconstructed = reconstruct(&f.events).unwrap();
    assert_eq!(reconstructed.challenge.unwrap().state, f.state);
    assert_eq!(
        (reconstructed.loot_grants, reconstructed.progression_grants),
        (0, 0)
    );
    assert_eq!(f.state.records[0].rules.seed, 37008);
    assert_eq!(f.state.records[0].objectives, 7);
    assert!(reconstruct(&f.events[..f.events.len() - 1]).is_err());
}

#[test]
fn challenge_gauntlet_rejects_skipped_holds_healing_actor_reuse_and_bypassed_combat() {
    let f = fixture();
    let transfer = f
        .events
        .iter()
        .position(|e| {
            GauntletEvidence::decode(&e.payload)
                .is_ok_and(|p| p.effect.next_stage == Some(Stage::Bastion))
        })
        .unwrap();
    for change in 0..5 {
        let mut bad = f.events.clone();
        let mut proof = GauntletEvidence::decode(&bad[transfer].payload).unwrap();
        match change {
            0 => proof.elapsed_ms -= 1,
            1 => proof.effect.recovered_health = 24,
            2 => proof.effect.next_stage = Some(Stage::Prism),
            3 => proof.run_id += 1,
            _ => proof.sequence += 1,
        }
        bad[transfer].payload = proof.encode().unwrap();
        assert!(
            reconstruct(&bad).is_err(),
            "accepted transfer variant {change}"
        );
    }
    let spawn = transfer + 1;
    for change in 0..2 {
        let mut bad = f.events.clone();
        let mut proof = GauntletEvidence::decode(&bad[spawn].payload).unwrap();
        let GauntletAction::Elite(e) = &mut proof.action else {
            panic!("missing elite spawn")
        };
        let EliteTransition::Started {
            player_health,
            partner_id,
            ..
        } = &mut e.transition
        else {
            unreachable!()
        };
        if change == 0 {
            *player_health -= 1;
        } else {
            *partner_id = 2;
        }
        bad[spawn].payload = proof.encode().unwrap();
        assert!(reconstruct(&bad).is_err());
    }
    let mut missing = f.events.clone();
    missing.remove(4);
    assert!(reconstruct(&missing).is_err());
    let mut bad = f.events;
    bad[3].account_id = "foreign".into();
    assert!(reconstruct(&bad).is_err());
}

#[test]
fn challenge_gauntlet_modifiers_preserve_order_and_record_proved_terminal_time() {
    use revenant_challenges::modifiers::Preset;
    for preset in Preset::ALL
        .into_iter()
        .filter(|p| !p.is_baseline() && p.available_for(ContractId::RelayGauntlet))
    {
        let mut f = Fixture::modified(preset);
        for stage in preset.gauntlet_stages() {
            let source = match stage {
                Stage::Support => combat().0,
                Stage::Bastion => elite_combat().0,
                Stage::Prism => prism::fixture(true).0,
            };
            f.encounter(stage, source);
        }
        let result = reconstruct(&f.events).unwrap();
        assert_eq!(result.challenge.unwrap().state, f.state);
        assert_eq!((result.loot_grants, result.progression_grants), (0, 0));
        let record = &f.state.records[0];
        assert_eq!(record.rules.preset, preset);
        assert_eq!(
            record.elapsed_ms,
            preset
                .time_goal_ms(ContractId::RelayGauntlet)
                .map(|_| f.time)
        );
        if record.elapsed_ms.is_some() {
            let last = f.events.last_mut().unwrap();
            let mut payload = decode_challenge_payload(&last.payload).unwrap();
            let Command::ConfirmTimed { elapsed_ms, .. } = &mut payload.command else {
                unreachable!()
            };
            *elapsed_ms += 1;
            payload.after = apply(
                &payload.policy_revision,
                &payload.before,
                payload.before.revision,
                &payload.command,
            )
            .unwrap()
            .state;
            last.payload = encode_challenge_payload(&payload).unwrap();
            assert!(reconstruct(&f.events).is_err());
        }
    }
}
