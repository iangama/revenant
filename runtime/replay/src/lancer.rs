use revenant_ai::lancer::{arena_contains, charge_hits, DAMAGE, RECOVERY_MS, SPAWN, WINDUP_MS};
use serde::{Deserialize, Serialize};

use crate::{ReplayError, ReplayEvent, ReplayEventKind};

pub const PREFIX: &str = "lancer-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum LancerEvidence {
    Windup {
        origin: [i32; 3],
        target: [i32; 3],
        elapsed_ms: u64,
    },
    Resolved {
        origin: [i32; 3],
        target: [i32; 3],
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
/// Returns a serialization error if the bounded typed evidence cannot encode.
pub fn encode_lancer_evidence(evidence: &LancerEvidence) -> Result<String, serde_json::Error> {
    Ok(format!("{PREFIX}{}", serde_json::to_string(evidence)?))
}

// Keep the ordered encounter transitions together for review.
#[allow(clippy::too_many_lines)]
pub(super) fn validate(events: &[ReplayEvent]) -> Result<Option<&'static str>, ReplayError> {
    let invalid = || ReplayError::InvalidFieldActivityEvidence;
    let mut encounter = None;
    let mut state = None;
    let mut locked = None;
    let mut position = SPAWN;
    let mut next_windup = 700;
    let mut last_time = 0;
    let mut player_actor = None;
    let mut last_health = None;
    let mut main_progressed = false;
    for event in events {
        main_progressed |= matches!(
            event.kind,
            ReplayEventKind::BossSpawned | ReplayEventKind::ActivityCompleted
        );
        if event.kind == ReplayEventKind::EnemySpawned
            && event.payload == "enemy spawned: glass-lancer"
        {
            if encounter.is_some() || event.actor_id.is_none() || main_progressed {
                return Err(invalid());
            }
            encounter = Some((event.account_id.as_str(), event.actor_id));
            let players: Vec<_> = events
                .iter()
                .filter(|other| other.kind == ReplayEventKind::PlayerJoined)
                .collect();
            if players.len() != 1
                || players[0].account_id != event.account_id
                || players[0].actor_id.is_none()
                || players[0].id >= event.id
            {
                return Err(invalid());
            }
            player_actor = players[0].actor_id;
            state = Some("Active");
        } else if event.kind == ReplayEventKind::EnemyDied
            && event.payload == "enemy died: glass-lancer"
        {
            if encounter != Some((event.account_id.as_str(), event.actor_id))
                || state != Some("Active")
            {
                return Err(invalid());
            }
            state = Some("Completed");
        } else if event.kind == ReplayEventKind::FieldActivity && event.payload.starts_with(PREFIX)
        {
            if state != Some("Active")
                || event.actor_id != player_actor
                || encounter.map(|e| e.0) != Some(event.account_id.as_str())
                || event.payload.len() > 1024
            {
                return Err(invalid());
            }
            let evidence: LancerEvidence =
                serde_json::from_str(&event.payload[PREFIX.len()..]).map_err(|_| invalid())?;
            let time = match evidence {
                LancerEvidence::Windup {
                    origin,
                    target,
                    elapsed_ms,
                } => {
                    if locked.is_some()
                        || origin != position
                        || !arena_contains(target)
                        || (origin[0] != target[0] && origin[2] != target[2])
                        || elapsed_ms < next_windup
                    {
                        return Err(invalid());
                    }
                    locked = Some((origin, target, elapsed_ms));
                    elapsed_ms
                }
                LancerEvidence::Resolved {
                    origin,
                    target,
                    player_position,
                    elapsed_ms,
                    damage,
                    health_before,
                    health_after,
                } => {
                    let (from, to, started) = locked.take().ok_or_else(invalid)?;
                    if origin != from
                        || target != to
                        || elapsed_ms < started.saturating_add(WINDUP_MS)
                        || !arena_contains(player_position)
                        || health_before == 0
                        || health_before.saturating_sub(damage) != health_after
                        || last_health.is_some_and(|health| health != health_before)
                        || damage
                            != if charge_hits(origin, target, player_position) {
                                DAMAGE
                            } else {
                                0
                            }
                    {
                        return Err(invalid());
                    }
                    position = target;
                    last_health = Some(health_after);
                    if health_after == 0 {
                        state = Some("Failed");
                    }
                    next_windup = elapsed_ms.saturating_add(RECOVERY_MS);
                    elapsed_ms
                }
                LancerEvidence::Abandoned { elapsed_ms } => {
                    state = Some("Failed");
                    elapsed_ms
                }
            };
            if time < last_time {
                return Err(invalid());
            }
            last_time = time;
        } else if (state == Some("Active") || last_health == Some(0))
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
    use revenant_ai::lancer::ENTRANCE;

    fn event(id: i64, kind: ReplayEventKind, actor: u64, payload: String) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            actor_id: Some(actor),
            account_id: "local:lancer".to_owned(),
            session_id: "lancer-test".to_owned(),
            timestamp: id.to_string(),
            activity_id: Some("relay_awakening".to_owned()),
            payload,
        }
    }

    fn trace() -> Vec<ReplayEvent> {
        vec![
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
                "enemy spawned: glass-lancer".to_owned(),
            ),
            event(
                3,
                ReplayEventKind::FieldActivity,
                1,
                encode_lancer_evidence(&LancerEvidence::Windup {
                    origin: SPAWN,
                    target: ENTRANCE,
                    elapsed_ms: 700,
                })
                .unwrap(),
            ),
            event(
                4,
                ReplayEventKind::FieldActivity,
                1,
                encode_lancer_evidence(&LancerEvidence::Resolved {
                    origin: SPAWN,
                    target: ENTRANCE,
                    player_position: [4, 0, 9],
                    elapsed_ms: 2500,
                    damage: 0,
                    health_before: 90,
                    health_after: 90,
                })
                .unwrap(),
            ),
            event(
                5,
                ReplayEventKind::EnemyDied,
                2,
                "enemy died: glass-lancer".to_owned(),
            ),
        ]
    }

    #[test]
    fn lancer_replay_preserves_partial_completed_abandoned_and_lethal_outcomes() {
        let mut events = trace();
        for end in 2..=5 {
            let state = reconstruct(&events[..end]).unwrap();
            assert_eq!(
                state.field_objectives["glass_lancer"],
                if end == 5 { "Completed" } else { "Active" }
            );
            assert!(!state.completed);
            assert_eq!(state.loot_grants, 0);
        }
        events[4] = event(
            5,
            ReplayEventKind::FieldActivity,
            1,
            encode_lancer_evidence(&LancerEvidence::Abandoned { elapsed_ms: 2600 }).unwrap(),
        );
        assert_eq!(
            reconstruct(&events).unwrap().field_objectives["glass_lancer"],
            "Failed"
        );
        events.truncate(4);
        events[3].payload = encode_lancer_evidence(&LancerEvidence::Resolved {
            origin: SPAWN,
            target: ENTRANCE,
            player_position: ENTRANCE,
            elapsed_ms: 2500,
            damage: 18,
            health_before: 10,
            health_after: 0,
        })
        .unwrap();
        assert_eq!(
            reconstruct(&events).unwrap().field_objectives["glass_lancer"],
            "Failed"
        );
    }

    #[test]
    fn lancer_replay_rejects_early_forged_repeated_or_wrong_actor_actions() {
        for corruption in [
            "early",
            "damage",
            "health",
            "actor",
            "late_spawn",
            "duplicate",
        ] {
            let mut events = trace();
            match corruption {
                "early" => events[3].payload = events[3].payload.replace("2500", "2499"),
                "damage" => {
                    events[3].payload = events[3].payload.replace("\"damage\":0", "\"damage\":18");
                }
                "health" => {
                    events[3].payload = events[3]
                        .payload
                        .replace("\"health_after\":90", "\"health_after\":100");
                }
                "actor" => events[3].actor_id = Some(77),
                "late_spawn" => events.insert(
                    0,
                    event(
                        0,
                        ReplayEventKind::BossSpawned,
                        99,
                        "boss spawned: warden".to_owned(),
                    ),
                ),
                _ => events[3].payload = events[2].payload.clone(),
            }
            assert!(reconstruct(&events).is_err(), "{corruption}");
        }
    }
}
