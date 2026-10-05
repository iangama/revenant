use revenant_ai::prism::{self, PrismAction, PrismPattern, PrismWarden};
use serde::{Deserialize, Serialize};

use crate::{ReplayError, ReplayEvent, ReplayEventKind};

pub const PREFIX: &str = "prism-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrismEvidence {
    pub elapsed_ms: u64,
    pub previous_confirmed_ms: u64,
    pub transition: PrismTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrismTransition {
    Started {
        boss_id: u64,
        player_health: u32,
    },
    Step {
        player_position: [i32; 3],
        step: PrismStep,
    },
    Attack {
        player_position: [i32; 3],
        damage: u32,
        health_before: u32,
        health_after: u32,
        phase_changed: bool,
    },
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrismPatternEvidence {
    AcrossX { z: i32 },
    AcrossZ { x: i32 },
    Center,
    Perimeter,
}

impl From<PrismPattern> for PrismPatternEvidence {
    fn from(value: PrismPattern) -> Self {
        match value {
            PrismPattern::AcrossX { z } => Self::AcrossX { z },
            PrismPattern::AcrossZ { x } => Self::AcrossZ { x },
            PrismPattern::Center => Self::Center,
            PrismPattern::Perimeter => Self::Perimeter,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrismStep {
    Warning {
        pattern: PrismPatternEvidence,
    },
    Resolved {
        pattern: PrismPatternEvidence,
        damage: u32,
        health_after: u32,
    },
}

impl PrismStep {
    #[must_use]
    pub fn from_action(action: PrismAction, player_health: u32) -> Self {
        match action {
            PrismAction::Warning { pattern } => Self::Warning {
                pattern: pattern.into(),
            },
            PrismAction::Resolved { pattern, damage } => Self::Resolved {
                pattern: pattern.into(),
                damage,
                health_after: player_health.saturating_sub(damage),
            },
        }
    }
}

/// # Errors
/// Returns a serialization error if the bounded evidence cannot encode.
pub fn encode_prism_evidence(evidence: &PrismEvidence) -> Result<String, serde_json::Error> {
    Ok(format!("{PREFIX}{}", serde_json::to_string(evidence)?))
}

#[allow(clippy::too_many_lines)]
pub(super) fn validate(events: &[ReplayEvent]) -> Result<Option<&'static str>, ReplayError> {
    validate_encounter(events, false)
}

// Only the gauntlet wrapper calls this after verifying its solo admission,
// ordered transfer and exact carried health. Raw global replay never opts in.
#[allow(clippy::too_many_lines)]
pub(super) fn validate_encounter(
    events: &[ReplayEvent],
    gauntlet_admitted: bool,
) -> Result<Option<&'static str>, ReplayError> {
    let invalid = || ReplayError::InvalidFieldActivityEvidence;
    let mut boss_id = None;
    let mut player_id = None;
    let mut owner = "";
    let mut player_health = 0;
    let mut boss_health = prism::HEALTH;
    let mut brain = PrismWarden::default();
    let mut state = "Active";
    let mut last_time = 0;
    let mut pending_confirmation = false;
    let mut main_progressed = false;
    let mut fallback_boss = None;
    let mut fallback_dead = false;
    for event in events {
        if !event.payload.starts_with(PREFIX) {
            if event.kind == ReplayEventKind::BossSpawned {
                main_progressed = true;
                if boss_id.is_some() {
                    if state != "Abandoned" || fallback_boss.is_some() || event.actor_id.is_none() {
                        return Err(invalid());
                    }
                    fallback_boss = event.actor_id;
                }
            }
            if event.kind == ReplayEventKind::EnemyDied
                && fallback_boss.is_some()
                && event.actor_id == fallback_boss
                && event.account_id == owner
            {
                fallback_dead = true;
            }
            if event.kind == ReplayEventKind::ActivityCompleted {
                main_progressed = true;
                if boss_id.is_some()
                    && state != "Completed"
                    && !(state == "Abandoned" && fallback_dead)
                {
                    return Err(invalid());
                }
            }
            continue;
        }
        if event.payload.len() > 2048 {
            return Err(invalid());
        }
        let evidence: PrismEvidence =
            serde_json::from_str(&event.payload[PREFIX.len()..]).map_err(|_| invalid())?;
        let time = evidence.elapsed_ms;
        if let PrismTransition::Started {
            boss_id: id,
            player_health: health,
        } = evidence.transition
        {
            let players: Vec<_> = events
                .iter()
                .filter(|e| e.kind == ReplayEventKind::PlayerJoined)
                .collect();
            if boss_id.is_some()
                || main_progressed
                || time != 0
                || evidence.previous_confirmed_ms != 0
                || event.kind != ReplayEventKind::BossSpawned
                || event.actor_id != Some(id)
                || id == 0
                || health == 0
                || players.len() != 1
                || players[0].actor_id.is_none()
                || players[0].account_id != event.account_id
                || players[0].id >= event.id
                || events
                    .iter()
                    .any(|e| e.id < event.id && e.actor_id == Some(id))
                || !(gauntlet_admitted
                    || crate::campaign::prism_admission(events, event)?
                    || crate::challenge_prism::admission(events, event)?
                    || events.iter().any(|e| {
                        e.id < event.id
                            && e.kind == ReplayEventKind::EnemyDied
                            && e.payload == "enemy died: relay-drone"
                    }))
            {
                return Err(invalid());
            }
            boss_id = Some(id);
            player_id = players[0].actor_id;
            owner = &event.account_id;
            player_health = health;
            continue;
        }
        let death = event.kind == ReplayEventKind::EnemyDied;
        if boss_id.is_none()
            || state != "Active"
            || event.account_id != owner
            || time < last_time
            || evidence.previous_confirmed_ms < last_time
            || evidence.previous_confirmed_ms > time
            || !matches!(
                event.kind,
                ReplayEventKind::FieldActivity | ReplayEventKind::EnemyDied
            )
            || (!death && event.actor_id != player_id)
            || (death
                && (event.actor_id != boss_id
                    || !matches!(
                        evidence.transition,
                        PrismTransition::Attack {
                            health_after: 0,
                            ..
                        }
                    )))
        {
            return Err(invalid());
        }
        if pending_confirmation {
            brain.confirm_at(evidence.previous_confirmed_ms);
            pending_confirmation = false;
        }
        match evidence.transition {
            PrismTransition::Started { .. } => unreachable!(),
            PrismTransition::Step {
                player_position,
                step,
            } => {
                let expected = brain
                    .tick(time, boss_health, player_position)
                    .ok_or_else(invalid)?;
                if PrismStep::from_action(expected, player_health) != step {
                    return Err(invalid());
                }
                if let PrismStep::Resolved { health_after, .. } = step {
                    player_health = health_after;
                    if player_health == 0 {
                        state = "Failed";
                    }
                }
                pending_confirmation = true;
            }
            PrismTransition::Attack {
                player_position,
                damage,
                health_before,
                health_after,
                phase_changed,
            } => {
                if !prism::arena_contains(player_position)
                    || health_before != boss_health
                    || (damage == 0) == brain.vulnerable()
                    || death != (health_after == 0)
                {
                    return Err(invalid());
                }
                let hit = brain.hit(time, health_before, damage).ok_or_else(invalid)?;
                if hit.damage != damage
                    || hit.health_after != health_after
                    || hit.phase_changed != phase_changed
                {
                    return Err(invalid());
                }
                boss_health = health_after;
                pending_confirmation = phase_changed;
                if death {
                    state = "Completed";
                }
            }
            PrismTransition::Abandoned => state = "Abandoned",
        }
        last_time = time;
    }
    if boss_id.is_some()
        && events.iter().any(|e| {
            e.kind.is_route_event()
                || e.kind.is_cooperation_event()
                || e.kind == ReplayEventKind::FieldActivity && !e.payload.starts_with(PREFIX)
                || e.kind == ReplayEventKind::EnemyDied
                    && e.actor_id == boss_id
                    && !e.payload.starts_with(PREFIX)
        })
    {
        return Err(invalid());
    }
    Ok(boss_id.map(|_| {
        if state == "Abandoned" {
            "Failed"
        } else {
            state
        }
    }))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::reconstruct;

    fn event(id: i64, kind: ReplayEventKind, actor_id: u64, payload: String) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            actor_id: Some(actor_id),
            account_id: "local:prism".to_owned(),
            session_id: "prism-test".to_owned(),
            timestamp: id.to_string(),
            activity_id: Some("relay_awakening".to_owned()),
            payload,
        }
    }

    fn add(events: &mut Vec<ReplayEvent>, time: u64, confirmed: u64, transition: PrismTransition) {
        let (kind, actor) = match transition {
            PrismTransition::Started { boss_id, .. } => (ReplayEventKind::BossSpawned, boss_id),
            PrismTransition::Attack {
                health_after: 0, ..
            } => (ReplayEventKind::EnemyDied, 3),
            _ => (ReplayEventKind::FieldActivity, 1),
        };
        events.push(event(
            i64::try_from(events.len()).unwrap() + 1,
            kind,
            actor,
            encode_prism_evidence(&PrismEvidence {
                elapsed_ms: time,
                previous_confirmed_ms: confirmed,
                transition,
            })
            .unwrap(),
        ));
    }

    fn start() -> Vec<ReplayEvent> {
        let mut events = vec![
            event(
                1,
                ReplayEventKind::PlayerJoined,
                1,
                "player joined".to_owned(),
            ),
            event(
                2,
                ReplayEventKind::ActivityStarted,
                1,
                "activity started".to_owned(),
            ),
            event(
                3,
                ReplayEventKind::EnemySpawned,
                2,
                "enemy spawned: relay-drone".to_owned(),
            ),
            event(
                4,
                ReplayEventKind::EnemyDied,
                2,
                "enemy died: relay-drone".to_owned(),
            ),
        ];
        add(
            &mut events,
            0,
            0,
            PrismTransition::Started {
                boss_id: 3,
                player_health: 90,
            },
        );
        events
    }

    fn step(
        events: &mut Vec<ReplayEvent>,
        time: u64,
        confirmed: u64,
        pos: [i32; 3],
        action: PrismAction,
    ) {
        add(
            events,
            time,
            confirmed,
            PrismTransition::Step {
                player_position: pos,
                step: PrismStep::from_action(action, 90),
            },
        );
    }

    fn attack(events: &mut Vec<ReplayEvent>, time: u64, confirmed: u64, before: u32, damage: u32) {
        add(
            events,
            time,
            confirmed,
            PrismTransition::Attack {
                player_position: prism::ENTRANCE,
                damage,
                health_before: before,
                health_after: before - damage,
                phase_changed: before == 320 && damage == 160,
            },
        );
    }

    fn through_shift() -> Vec<ReplayEvent> {
        let mut events = start();
        attack(&mut events, 1, 0, 320, 0);
        step(
            &mut events,
            700,
            1,
            prism::ENTRANCE,
            PrismAction::Warning {
                pattern: PrismPattern::AcrossX { z: 2 },
            },
        );
        step(
            &mut events,
            5000,
            3200,
            [5, 0, 1],
            PrismAction::Resolved {
                pattern: PrismPattern::AcrossX { z: 2 },
                damage: 0,
            },
        );
        attack(&mut events, 5100, 5050, 320, 160);
        events
    }

    pub(crate) fn completed_fixture() -> Vec<ReplayEvent> {
        let mut events = through_shift();
        step(
            &mut events,
            10_400,
            8000,
            prism::ENTRANCE,
            PrismAction::Warning {
                pattern: PrismPattern::Center,
            },
        );
        step(
            &mut events,
            12_600,
            10_400,
            prism::ENTRANCE,
            PrismAction::Resolved {
                pattern: PrismPattern::Center,
                damage: 0,
            },
        );
        step(
            &mut events,
            20_600,
            20_000,
            prism::ENTRANCE,
            PrismAction::Warning {
                pattern: PrismPattern::Perimeter,
            },
        );
        step(
            &mut events,
            22_800,
            20_600,
            prism::SPAWN,
            PrismAction::Resolved {
                pattern: PrismPattern::Perimeter,
                damage: 0,
            },
        );
        attack(&mut events, 23_000, 22_800, 160, 160);
        events
    }

    #[test]
    fn prism_replay_preserves_delayed_full_phases_and_atomic_death() {
        let mut events = completed_fixture();
        for end in 5..events.len() {
            assert_eq!(validate(&events[..end]).unwrap(), Some("Active"));
        }
        assert_eq!(validate(&events).unwrap(), Some("Completed"));
        let mut split_death = events.clone();
        split_death.last_mut().unwrap().kind = ReplayEventKind::FieldActivity;
        assert!(validate(&split_death).is_err());
        events.push(event(
            i64::try_from(events.len()).unwrap() + 1,
            ReplayEventKind::ActivityCompleted,
            1,
            "activity completed".to_owned(),
        ));
        let replay = reconstruct(&events).unwrap();
        assert!(replay.completed);
        assert!(replay.boss_spawned);
        assert_eq!(replay.spawned_enemies, 1);
        assert_eq!(replay.defeated_enemies, 2);
        assert_eq!(
            replay
                .field_objectives
                .get("prism_warden")
                .map(String::as_str),
            Some("Completed")
        );
        assert_acquisition_proof(&events);
    }

    fn assert_acquisition_proof(source: &[ReplayEvent]) {
        use crate::{
            encode_module_state_snapshot, module_state_evidence_for_catalog,
            ModuleStateSnapshotPayloadV1, ReplayProtocolGeneration, MODULE_REPLAY_SCHEMA_VERSION,
        };
        let mut events = source.to_vec();
        let snapshot = encode_module_state_snapshot(&ModuleStateSnapshotPayloadV1 {
            schema_version: MODULE_REPLAY_SCHEMA_VERSION,
            character_id: "local:prism:operator".to_owned(),
            protocol_generation: ReplayProtocolGeneration::V2,
            state: module_state_evidence_for_catalog(
                "m35-v2",
                0,
                &[],
                &[],
                0,
                ReplayProtocolGeneration::V2,
            )
            .unwrap(),
        })
        .unwrap();
        events.insert(
            1,
            event(2, ReplayEventKind::ModuleStateSnapshot, 1, snapshot),
        );
        for (index, e) in events.iter_mut().enumerate() {
            e.id = i64::try_from(index).unwrap() + 1;
        }
        assert!(
            crate::acquisition_milestones(&events, "local:prism:operator")
                .unwrap()
                .is_empty()
        );
        events.push(event(
            i64::try_from(events.len()).unwrap() + 1,
            ReplayEventKind::LootGranted,
            1,
            "loot granted: relay_core_fragment x1 (total 1)".to_owned(),
        ));
        assert_eq!(
            crate::acquisition_milestones(&events, "local:prism:operator").unwrap(),
            [revenant_modules::acquisition::Milestone::PrismCompleted]
        );
        assert!(crate::acquisition_milestones(&events, "another-character")
            .unwrap()
            .is_empty());
        events.retain(|e| e.kind != ReplayEventKind::ActivityCompleted);
        assert!(
            crate::acquisition_milestones(&events, "local:prism:operator")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn prism_replay_rejects_early_phase_warning_damage_and_false_confirmation() {
        let events = through_shift();
        let mut early = events.clone();
        step(
            &mut early,
            10_399,
            8000,
            prism::ENTRANCE,
            PrismAction::Warning {
                pattern: PrismPattern::Center,
            },
        );
        assert!(validate(&early).is_err());
        let mut skip_pulses = events.clone();
        attack(&mut skip_pulses, 11_000, 8000, 160, 160);
        assert!(validate(&skip_pulses).is_err());
        let mut wrong_lane = start();
        step(
            &mut wrong_lane,
            700,
            0,
            prism::ENTRANCE,
            PrismAction::Warning {
                pattern: PrismPattern::AcrossX { z: 1 },
            },
        );
        assert!(validate(&wrong_lane).is_err());
        let mut impossible_clock = events;
        add(
            &mut impossible_clock,
            9000,
            9001,
            PrismTransition::Abandoned,
        );
        assert!(validate(&impossible_clock).is_err());
        let mut missing_death = start();
        missing_death.retain(|e| e.kind != ReplayEventKind::EnemyDied);
        assert!(validate(&missing_death).is_err());
    }

    #[test]
    fn prism_replay_withdrawal_requires_fallback_boss_and_defeat_cannot_complete() {
        let mut events = start();
        add(&mut events, 100, 0, PrismTransition::Abandoned);
        assert_eq!(validate(&events).unwrap(), Some("Failed"));
        let mut false_completion = events.clone();
        false_completion.push(event(
            7,
            ReplayEventKind::ActivityCompleted,
            1,
            "activity completed".to_owned(),
        ));
        assert!(validate(&false_completion).is_err());
        events.push(event(
            7,
            ReplayEventKind::BossSpawned,
            4,
            "boss spawned: warden".to_owned(),
        ));
        events.push(event(
            8,
            ReplayEventKind::EnemyDied,
            4,
            "enemy died: warden".to_owned(),
        ));
        events.push(event(
            9,
            ReplayEventKind::ActivityCompleted,
            1,
            "activity completed".to_owned(),
        ));
        assert!(reconstruct(&events).unwrap().completed);

        let mut lethal = start();
        lethal.last_mut().unwrap().payload = encode_prism_evidence(&PrismEvidence {
            elapsed_ms: 0,
            previous_confirmed_ms: 0,
            transition: PrismTransition::Started {
                boss_id: 3,
                player_health: 18,
            },
        })
        .unwrap();
        add(
            &mut lethal,
            700,
            0,
            PrismTransition::Step {
                player_position: prism::ENTRANCE,
                step: PrismStep::Warning {
                    pattern: PrismPatternEvidence::AcrossX { z: 2 },
                },
            },
        );
        add(
            &mut lethal,
            2500,
            700,
            PrismTransition::Step {
                player_position: prism::ENTRANCE,
                step: PrismStep::Resolved {
                    pattern: PrismPatternEvidence::AcrossX { z: 2 },
                    damage: 18,
                    health_after: 0,
                },
            },
        );
        assert_eq!(validate(&lethal).unwrap(), Some("Failed"));
        lethal.push(event(
            8,
            ReplayEventKind::ActivityCompleted,
            1,
            "activity completed".to_owned(),
        ));
        assert!(validate(&lethal).is_err());
    }
}
