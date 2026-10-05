use super::{
    admission, checkpoint, event, reconstruct, ChallengeState, Command, ContractId, EndReason,
    Kind, Objective, Outcome, ReplayEvent,
};
use crate::{encode_prism_evidence, PrismEvidence, PrismStep, PrismTransition};
use revenant_ai::prism::{self, PrismMode, PrismWarden};

fn append(
    events: &mut Vec<ReplayEvent>,
    time: u64,
    confirmed: &mut u64,
    transition: PrismTransition,
) -> i64 {
    let (kind, actor) = match transition {
        PrismTransition::Started { .. } => (Kind::BossSpawned, 2),
        PrismTransition::Attack {
            health_after: 0, ..
        } => (Kind::EnemyDied, 2),
        _ => (Kind::FieldActivity, 1),
    };
    let proof = event(
        events,
        kind,
        actor,
        encode_prism_evidence(&PrismEvidence {
            elapsed_ms: time,
            previous_confirmed_ms: *confirmed,
            transition,
        })
        .unwrap(),
    );
    *confirmed = time;
    proof
}

pub(super) fn fixture(win: bool) -> (Vec<ReplayEvent>, ChallengeState) {
    fixture_with_hit(win, false)
}

pub(super) fn fixture_with_hit(
    win: bool,
    take_one_hit: bool,
) -> (Vec<ReplayEvent>, ChallengeState) {
    let (mut events, state) = admission(
        ContractId::PrismDiscipline,
        &ChallengeState::default(),
        false,
    );
    let mut confirmed = 0;
    append(
        &mut events,
        0,
        &mut confirmed,
        PrismTransition::Started {
            boss_id: 2,
            player_health: 100,
        },
    );
    let mut brain = PrismWarden::default();
    let (mut boss, mut health, mut ready) = (prism::HEALTH, 100u32, 0);
    for time in (0..60_000).step_by(100) {
        let position = match brain.mode() {
            PrismMode::Warning(pattern) => {
                let candidates = [[5, 0, -3], [6, 0, -2], [8, 0, 0], [11, 0, 3], [7, 0, 1]];
                *candidates
                    .iter()
                    .find(|p| pattern.hits(**p) != (win && !(take_one_hit && health == 100)))
                    .unwrap()
            }
            _ => [7, 0, 1],
        };
        if let Some(action) = brain.tick(time, boss, position) {
            let step = PrismStep::from_action(action, health);
            if let PrismStep::Resolved { health_after, .. } = step {
                health = health_after;
            }
            let proof = append(
                &mut events,
                time,
                &mut confirmed,
                PrismTransition::Step {
                    player_position: position,
                    step,
                },
            );
            brain.confirm_at(time);
            if health == 0 {
                let terminal = checkpoint(
                    &mut events,
                    &state,
                    Command::End {
                        run_id: 1,
                        reason: EndReason::Defeated,
                    },
                    Some(proof),
                );
                return (events, terminal);
            }
        }
        if win && (brain.vulnerable() || time == 0) && time >= ready {
            let hit = brain.hit(time, boss, 40).unwrap();
            let proof = append(
                &mut events,
                time,
                &mut confirmed,
                PrismTransition::Attack {
                    player_position: position,
                    damage: hit.damage,
                    health_before: boss,
                    health_after: hit.health_after,
                    phase_changed: hit.phase_changed,
                },
            );
            boss = hit.health_after;
            ready = time + 250;
            if hit.phase_changed {
                brain.confirm_at(time);
            }
            if boss == 0 {
                let terminal = checkpoint(
                    &mut events,
                    &state,
                    Command::Confirm {
                        run_id: 1,
                        objective: Objective::PrismCleared,
                    },
                    Some(proof),
                );
                return (events, terminal);
            }
        }
    }
    panic!("Prism fixture did not terminate");
}

#[test]
fn challenge_prism_requires_both_phases_and_records_no_rewards() {
    let (events, expected) = fixture(true);
    let result = reconstruct(&events).unwrap();
    assert_eq!(result.challenge.unwrap().state, expected);
    assert_eq!((result.loot_grants, result.progression_grants), (0, 0));
    assert!(!result.completed);
    assert_eq!(expected.records[0].rules.seed, 37005);
    assert!(events
        .iter()
        .any(|e| e.payload.contains("\"phase_changed\":true")));
    let mut no_pulses = events.clone();
    no_pulses.retain(|e| {
        !e.payload.contains("\"shape\":\"center\"")
            && !e.payload.contains("\"shape\":\"perimeter\"")
    });
    assert!(reconstruct(&no_pulses).is_err());
    assert!(reconstruct(&events[..events.len() - 1]).is_err());
    let mut foreign = events.clone();
    foreign[2].session_id = "foreign".into();
    assert!(reconstruct(&foreign).is_err());
    let mut missing = events;
    missing.remove(2);
    assert!(reconstruct(&missing).is_err());
}

#[test]
fn challenge_prism_rejects_unaccepted_damage_cooldown_and_starting_health() {
    let (events, _) = fixture(true);
    let index = events
        .iter()
        .position(|e| {
            e.payload.starts_with(crate::prism::PREFIX) && e.payload.contains("\"damage\":40")
        })
        .unwrap();
    for corruption in ["damage", "cooldown", "health", "range"] {
        let mut bad = events[..=index + 1].to_vec();
        match corruption {
            "damage" => {
                bad.truncate(index + 1);
                bad[index].payload = bad[index]
                    .payload
                    .replace("\"damage\":40", "\"damage\":80")
                    .replace("\"health_after\":280", "\"health_after\":240");
            }
            "health" => {
                bad[3].payload = bad[3]
                    .payload
                    .replace("\"player_health\":100", "\"player_health\":90");
            }
            "range" => {
                bad.truncate(index + 1);
                bad[index].payload = bad[index]
                    .payload
                    .replace("[5,0,-3]", "[50,0,-3]")
                    .replace("[6,0,-2]", "[60,0,-2]");
            }
            _ => {
                let mut proof: PrismEvidence = serde_json::from_str(
                    bad[index + 1]
                        .payload
                        .strip_prefix(crate::prism::PREFIX)
                        .unwrap(),
                )
                .unwrap();
                proof.elapsed_ms = proof.previous_confirmed_ms + 249;
                bad[index + 1].payload = encode_prism_evidence(&proof).unwrap();
            }
        }
        assert!(reconstruct(&bad).is_err(), "{corruption}");
    }
}

#[test]
fn challenge_prism_defeat_requires_the_adjacent_fatal_pattern() {
    let (events, expected) = fixture(false);
    assert_eq!(
        reconstruct(&events).unwrap().challenge.unwrap().state,
        expected
    );
    assert_eq!(expected.last_result.unwrap().outcome, Outcome::Defeated);
    assert!(expected.records.is_empty());
    assert!(reconstruct(&events[..events.len() - 1]).is_err());
    let mut bad = events;
    let last = bad.last_mut().unwrap();
    let mut checkpoint = crate::decode_challenge_payload(&last.payload).unwrap();
    checkpoint.proof_event_id = checkpoint.proof_event_id.map(|id| id - 1);
    last.payload = crate::encode_challenge_payload(&checkpoint).unwrap();
    assert!(reconstruct(&bad).is_err());
}
