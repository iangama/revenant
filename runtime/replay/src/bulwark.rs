use revenant_ai::bulwark::{
    arena_contains, slam_hits, Facing, DAMAGE, HEALTH, RECOVERY_MS, SPAWN, WINDUP_MS,
};
use serde::{Deserialize, Serialize};

use crate::{ReplayError, ReplayEvent, ReplayEventKind};

pub const PREFIX: &str = "bulwark-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum BulwarkEvidence {
    Braced {
        facing: [i32; 2],
        player_position: [i32; 3],
        elapsed_ms: u64,
    },
    Slam {
        facing: [i32; 2],
        player_position: [i32; 3],
        elapsed_ms: u64,
        damage: u32,
        health_before: u32,
        health_after: u32,
    },
    Attack {
        player_position: [i32; 3],
        elapsed_ms: u64,
        damage: u32,
        health_before: u32,
        health_after: u32,
    },
    Abandoned {
        elapsed_ms: u64,
    },
}

/// # Errors
/// Returns a serialization error if the typed evidence cannot encode.
pub fn encode_bulwark_evidence(evidence: &BulwarkEvidence) -> Result<String, serde_json::Error> {
    Ok(format!("{PREFIX}{}", serde_json::to_string(evidence)?))
}

// The evidence transitions stay ordered in one place for review.
#[allow(clippy::too_many_lines)]
pub(super) fn validate(events: &[ReplayEvent]) -> Result<Option<&'static str>, ReplayError> {
    let invalid = || ReplayError::InvalidFieldActivityEvidence;
    let mut encounter = None;
    let mut player_id = None;
    let mut state = None;
    let mut main_progressed = false;
    let mut facing = Facing::West;
    let mut braced = true;
    let mut ready_at = WINDUP_MS;
    let mut enemy_health = HEALTH;
    let mut player_health = None;
    let mut last_time = 0;
    for event in events {
        main_progressed |= matches!(
            event.kind,
            ReplayEventKind::BossSpawned | ReplayEventKind::ActivityCompleted
        );
        if event.kind == ReplayEventKind::EnemySpawned
            && event.payload == "enemy spawned: steel-bulwark"
        {
            if encounter.is_some() || event.actor_id.is_none() || main_progressed {
                return Err(invalid());
            }
            let players: Vec<_> = events
                .iter()
                .filter(|e| e.kind == ReplayEventKind::PlayerJoined)
                .collect();
            if players.len() != 1
                || players[0].actor_id.is_none()
                || players[0].account_id != event.account_id
                || players[0].id >= event.id
            {
                return Err(invalid());
            }
            encounter = Some((event.account_id.as_str(), event.actor_id));
            player_id = players[0].actor_id;
            state = Some("Active");
        } else if matches!(
            event.kind,
            ReplayEventKind::FieldActivity | ReplayEventKind::EnemyDied
        ) && event.payload.starts_with(PREFIX)
        {
            let death = event.kind == ReplayEventKind::EnemyDied;
            if state != Some("Active")
                || event.payload.len() > 1024
                || encounter.map(|e| e.0) != Some(event.account_id.as_str())
                || event.actor_id
                    != if death {
                        encounter.and_then(|e| e.1)
                    } else {
                        player_id
                    }
            {
                return Err(invalid());
            }
            let evidence: BulwarkEvidence =
                serde_json::from_str(&event.payload[PREFIX.len()..]).map_err(|_| invalid())?;
            if death
                && !matches!(
                    evidence,
                    BulwarkEvidence::Attack {
                        health_after: 0,
                        ..
                    }
                )
            {
                return Err(invalid());
            }
            let time = match evidence {
                BulwarkEvidence::Braced {
                    facing: direction,
                    player_position,
                    elapsed_ms,
                } => {
                    let next = Facing::from_vector(direction).ok_or_else(invalid)?;
                    if braced
                        || elapsed_ms < ready_at
                        || !arena_contains(player_position)
                        || next != facing.toward(SPAWN, player_position)
                    {
                        return Err(invalid());
                    }
                    facing = next;
                    braced = true;
                    ready_at = elapsed_ms.saturating_add(WINDUP_MS);
                    elapsed_ms
                }
                BulwarkEvidence::Slam {
                    facing: direction,
                    player_position,
                    elapsed_ms,
                    damage,
                    health_before,
                    health_after,
                } => {
                    if !braced
                        || elapsed_ms < ready_at
                        || direction != facing.vector()
                        || !arena_contains(player_position)
                        || health_before == 0
                        || health_before.saturating_sub(damage) != health_after
                        || player_health.is_some_and(|health| health != health_before)
                        || damage
                            != if slam_hits(facing, SPAWN, player_position) {
                                DAMAGE
                            } else {
                                0
                            }
                    {
                        return Err(invalid());
                    }
                    player_health = Some(health_after);
                    if health_after == 0 {
                        state = Some("Failed");
                    }
                    braced = false;
                    ready_at = elapsed_ms.saturating_add(RECOVERY_MS);
                    elapsed_ms
                }
                BulwarkEvidence::Attack {
                    player_position,
                    elapsed_ms,
                    damage,
                    health_before,
                    health_after,
                } => {
                    let blocked = braced && facing.protects(SPAWN, player_position);
                    if !arena_contains(player_position)
                        || health_before != enemy_health
                        || health_before.saturating_sub(damage) != health_after
                        || (damage == 0) != blocked
                        || death != (health_after == 0)
                    {
                        return Err(invalid());
                    }
                    enemy_health = health_after;
                    if death {
                        state = Some("Completed");
                    }
                    elapsed_ms
                }
                BulwarkEvidence::Abandoned { elapsed_ms } => {
                    state = Some("Failed");
                    elapsed_ms
                }
            };
            if time < last_time {
                return Err(invalid());
            }
            last_time = time;
        } else if event.kind == ReplayEventKind::EnemyDied
            && event.payload == "enemy died: steel-bulwark"
            || (state == Some("Active") || player_health == Some(0))
                && matches!(
                    event.kind,
                    ReplayEventKind::BossSpawned | ReplayEventKind::ActivityCompleted
                )
        {
            return Err(invalid());
        }
    }
    if encounter.is_some()
        && events.iter().any(|event| {
            event.kind.is_route_event()
                || event.kind.is_cooperation_event()
                || event.kind == ReplayEventKind::FieldActivity
                    && !event.payload.starts_with(PREFIX)
        })
    {
        return Err(invalid());
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconstruct;
    use revenant_ai::bulwark::ENTRANCE;

    fn event(id: i64, kind: ReplayEventKind, actor: u64, payload: String) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            actor_id: Some(actor),
            account_id: "local:bulwark".to_owned(),
            session_id: "bulwark-test".to_owned(),
            timestamp: id.to_string(),
            activity_id: Some("relay_awakening".to_owned()),
            payload,
        }
    }

    fn trace() -> Vec<ReplayEvent> {
        let mut events = vec![
            event(
                1,
                ReplayEventKind::PlayerJoined,
                1,
                "player joined".to_owned(),
            ),
            event(
                2,
                ReplayEventKind::EnemySpawned,
                2,
                "enemy spawned: steel-bulwark".to_owned(),
            ),
        ];
        for (index, evidence) in [
            BulwarkEvidence::Attack {
                player_position: ENTRANCE,
                elapsed_ms: 500,
                damage: 0,
                health_before: 240,
                health_after: 240,
            },
            BulwarkEvidence::Slam {
                facing: [-1, 0],
                player_position: [8, 0, -6],
                elapsed_ms: 1600,
                damage: 0,
                health_before: 90,
                health_after: 90,
            },
            BulwarkEvidence::Braced {
                facing: [0, 1],
                player_position: [8, 0, -6],
                elapsed_ms: 3400,
            },
            BulwarkEvidence::Slam {
                facing: [0, 1],
                player_position: [8, 0, -6],
                elapsed_ms: 5000,
                damage: 14,
                health_before: 90,
                health_after: 76,
            },
        ]
        .iter()
        .enumerate()
        {
            events.push(event(
                i64::try_from(index).unwrap() + 3,
                ReplayEventKind::FieldActivity,
                1,
                encode_bulwark_evidence(evidence).unwrap(),
            ));
        }
        for index in 0..6u32 {
            let damage = BulwarkEvidence::Attack {
                player_position: ENTRANCE,
                elapsed_ms: 5100 + u64::from(index) * 250,
                damage: 40,
                health_before: 240 - index * 40,
                health_after: 200 - index * 40,
            };
            events.push(event(
                i64::from(index) + 7,
                if index == 5 {
                    ReplayEventKind::EnemyDied
                } else {
                    ReplayEventKind::FieldActivity
                },
                if index == 5 { 2 } else { 1 },
                encode_bulwark_evidence(&damage).unwrap(),
            ));
        }
        events
    }

    #[test]
    fn bulwark_replay_reconstructs_shield_recovery_and_atomic_death() {
        let events = trace();
        for end in 2..=events.len() {
            let replay = reconstruct(&events[..end]).unwrap();
            assert_eq!(
                replay.field_objectives["steel_bulwark"],
                if end == events.len() {
                    "Completed"
                } else {
                    "Active"
                }
            );
            assert_eq!(replay.defeated_enemies, usize::from(end == events.len()));
            assert!(!replay.completed);
        }
    }

    #[test]
    fn bulwark_withdrawal_and_lethal_slam_remain_failed_optional_objectives() {
        for lethal in [false, true] {
            let mut events = trace();
            events.truncate(3);
            let evidence = if lethal {
                BulwarkEvidence::Slam {
                    facing: [-1, 0],
                    player_position: ENTRANCE,
                    elapsed_ms: 1600,
                    damage: DAMAGE,
                    health_before: 7,
                    health_after: 0,
                }
            } else {
                BulwarkEvidence::Abandoned { elapsed_ms: 600 }
            };
            events.push(event(
                4,
                ReplayEventKind::FieldActivity,
                1,
                encode_bulwark_evidence(&evidence).unwrap(),
            ));
            let replay = reconstruct(&events).unwrap();
            assert_eq!(replay.field_objectives["steel_bulwark"], "Failed");
            assert_eq!(replay.defeated_enemies, 0);
            assert!(!replay.completed);
            if lethal {
                events.push(event(
                    5,
                    ReplayEventKind::BossSpawned,
                    3,
                    "boss spawned: warden".to_owned(),
                ));
                assert!(reconstruct(&events).is_err());
            }
        }
    }

    #[test]
    fn bulwark_replay_rejects_false_front_hits_early_slam_and_split_death() {
        for corruption in ["shield", "early", "actor", "death", "health"] {
            let mut events = trace();
            match corruption {
                "shield" => {
                    events[2].payload = events[2]
                        .payload
                        .replace("\"damage\":0", "\"damage\":40")
                        .replace("\"health_after\":240", "\"health_after\":200");
                }
                "early" => events[3].payload = events[3].payload.replace("1600", "1599"),
                "actor" => events[3].actor_id = Some(99),
                "death" => events.last_mut().unwrap().kind = ReplayEventKind::FieldActivity,
                _ => {
                    events[6].payload = events[6]
                        .payload
                        .replace("\"health_after\":200", "\"health_after\":199");
                }
            }
            assert!(reconstruct(&events).is_err(), "{corruption}");
        }
    }
}
