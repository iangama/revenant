use revenant_actors::{Actor, ActorKind};
use revenant_ai::{
    lancer::{self, ChargeAction, GlassLancer},
    support::{self, RelayMender},
};
use serde::{Deserialize, Serialize};

use crate::{ReplayError, ReplayEvent, ReplayEventKind};

pub const PREFIX: &str = "support-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportEvidence {
    pub elapsed_ms: u64,
    pub transition: SupportTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum SupportTransition {
    Started {
        lancer_id: u64,
        mender_id: u64,
        player_health: u32,
    },
    Windup {
        player_position: [i32; 3],
        origin: [i32; 3],
        target: [i32; 3],
    },
    Charged {
        player_position: [i32; 3],
        origin: [i32; 3],
        target: [i32; 3],
        damage: u32,
        health_after: u32,
    },
    Repaired {
        target_id: u64,
        amount: u32,
        health_before: u32,
        health_after: u32,
    },
    Attack {
        target_id: u64,
        player_position: [i32; 3],
        damage: u32,
        health_before: u32,
        health_after: u32,
    },
    Abandoned,
}

/// # Errors
/// Returns a serialization error if the bounded encounter evidence cannot encode.
pub fn encode_support_evidence(evidence: &SupportEvidence) -> Result<String, serde_json::Error> {
    Ok(format!("{PREFIX}{}", serde_json::to_string(evidence)?))
}

fn enemy(id: u64, position: [i32; 3], health: u32) -> Actor {
    Actor {
        id,
        position,
        health,
        max_health: health,
        kind: ActorKind::Enemy,
        archetype: String::new(),
    }
}

// Validate the ordered health/AI transitions together, including partial sessions.
#[allow(clippy::too_many_lines)]
pub(super) fn validate(events: &[ReplayEvent]) -> Result<Option<&'static str>, ReplayError> {
    let scoped = crate::campaign::counter_encounter_events(events, PREFIX)?;
    let events = scoped.as_deref().unwrap_or(events);
    let invalid = || ReplayError::InvalidFieldActivityEvidence;
    let mut state = None;
    let mut owner = "";
    let mut player_id = None;
    let mut player_health = 0;
    let mut lancer = enemy(0, lancer::SPAWN, lancer::HEALTH);
    let mut mender = enemy(0, support::SPAWN, support::HEALTH);
    let mut charge_ai = GlassLancer::default();
    let mut repair_ai = RelayMender::default();
    let mut last_time = 0;
    let mut main_progressed = false;
    for event in events {
        if matches!(
            event.kind,
            ReplayEventKind::BossSpawned | ReplayEventKind::ActivityCompleted
        ) {
            if state == Some("Active") || (state.is_some() && player_health == 0) {
                return Err(invalid());
            }
            main_progressed = true;
        }
        if !event.payload.starts_with(PREFIX) {
            continue;
        }
        if event.payload.len() > 1024 {
            return Err(invalid());
        }
        let evidence: SupportEvidence =
            serde_json::from_str(&event.payload[PREFIX.len()..]).map_err(|_| invalid())?;
        let time = evidence.elapsed_ms;
        if let SupportTransition::Started {
            lancer_id,
            mender_id,
            player_health: health,
        } = evidence.transition
        {
            let players: Vec<_> = events
                .iter()
                .filter(|e| e.kind == ReplayEventKind::PlayerJoined)
                .collect();
            if state.is_some()
                || main_progressed
                || time != 0
                || event.kind != ReplayEventKind::EnemySpawned
                || event.actor_id != Some(lancer_id)
                || lancer_id == 0
                || mender_id == 0
                || lancer_id == mender_id
                || health == 0
                || players.len() != 1
                || players[0].account_id != event.account_id
                || players[0].id >= event.id
                || players[0].actor_id.is_none()
                || events.iter().any(|e| {
                    e.id < event.id
                        && (e.actor_id == Some(lancer_id) || e.actor_id == Some(mender_id))
                })
            {
                return Err(invalid());
            }
            owner = &event.account_id;
            player_id = players[0].actor_id;
            player_health = health;
            lancer.id = lancer_id;
            mender.id = mender_id;
            state = Some("Active");
            continue;
        }
        let death = event.kind == ReplayEventKind::EnemyDied;
        if state != Some("Active")
            || event.account_id != owner
            || time < last_time
            || !matches!(
                event.kind,
                ReplayEventKind::FieldActivity | ReplayEventKind::EnemyDied
            )
            || (!death && event.actor_id != player_id)
            || (death
                && !matches!(evidence.transition, SupportTransition::Attack { target_id, health_after: 0, .. } if event.actor_id == Some(target_id)))
        {
            return Err(invalid());
        }
        match evidence.transition {
            SupportTransition::Started { .. } => unreachable!(),
            SupportTransition::Windup {
                player_position,
                origin,
                target,
            } => {
                if lancer.health == 0
                    || charge_ai.tick(time, lancer.position, player_position)
                        != Some(ChargeAction::Windup { origin, target })
                {
                    return Err(invalid());
                }
            }
            SupportTransition::Charged {
                player_position,
                origin,
                target,
                damage,
                health_after,
            } => {
                if lancer.health == 0
                    || charge_ai.tick(time, lancer.position, player_position)
                        != Some(ChargeAction::Resolved {
                            origin,
                            target,
                            damage,
                        })
                    || player_health.saturating_sub(damage) != health_after
                {
                    return Err(invalid());
                }
                lancer.position = target;
                player_health = health_after;
                if player_health == 0 {
                    state = Some("Failed");
                }
            }
            SupportTransition::Repaired {
                target_id,
                amount,
                health_before,
                health_after,
            } => {
                let action = repair_ai
                    .tick(time, &mender, &[lancer.clone()])
                    .ok_or_else(invalid)?;
                if (
                    action.target_id,
                    action.amount,
                    action.health_before,
                    action.health_after,
                ) != (target_id, amount, health_before, health_after)
                {
                    return Err(invalid());
                }
                lancer.health = health_after;
            }
            SupportTransition::Attack {
                target_id,
                player_position,
                damage,
                health_before,
                health_after,
            } => {
                let target = if target_id == lancer.id {
                    &mut lancer
                } else if target_id == mender.id {
                    &mut mender
                } else {
                    return Err(invalid());
                };
                if !lancer::arena_contains(player_position)
                    || target.health == 0
                    || damage == 0
                    || target.health != health_before
                    || health_before.saturating_sub(damage) != health_after
                    || death != (health_after == 0)
                {
                    return Err(invalid());
                }
                target.health = health_after;
                if lancer.health == 0 && mender.health == 0 {
                    state = Some("Completed");
                }
            }
            SupportTransition::Abandoned => state = Some("Failed"),
        }
        last_time = time;
    }
    if state.is_some()
        && events.iter().any(|event| {
            event.kind.is_route_event()
                || event.kind.is_cooperation_event()
                || event.kind == ReplayEventKind::FieldActivity
                    && !event.payload.starts_with(PREFIX)
                || event.kind == ReplayEventKind::EnemyDied
                    && [Some(lancer.id), Some(mender.id)].contains(&event.actor_id)
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

    fn event(id: i64, kind: ReplayEventKind, actor: u64, payload: String) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            actor_id: Some(actor),
            account_id: "local:support".to_owned(),
            session_id: "support-test".to_owned(),
            timestamp: id.to_string(),
            activity_id: Some("relay_awakening".to_owned()),
            payload,
        }
    }

    fn add(events: &mut Vec<ReplayEvent>, time: u64, transition: SupportTransition) {
        let (kind, actor) = match transition {
            SupportTransition::Started { lancer_id, .. } => {
                (ReplayEventKind::EnemySpawned, lancer_id)
            }
            SupportTransition::Attack {
                target_id,
                health_after: 0,
                ..
            } => (ReplayEventKind::EnemyDied, target_id),
            _ => (ReplayEventKind::FieldActivity, 1),
        };
        events.push(event(
            i64::try_from(events.len()).unwrap() + 1,
            kind,
            actor,
            encode_support_evidence(&SupportEvidence {
                elapsed_ms: time,
                transition,
            })
            .unwrap(),
        ));
    }

    fn trace() -> Vec<ReplayEvent> {
        let mut events = vec![event(
            1,
            ReplayEventKind::PlayerJoined,
            1,
            "player joined".to_owned(),
        )];
        add(
            &mut events,
            0,
            SupportTransition::Started {
                lancer_id: 2,
                mender_id: 3,
                player_health: 90,
            },
        );
        add(
            &mut events,
            0,
            SupportTransition::Attack {
                target_id: 2,
                player_position: [5, 0, 8],
                damage: 40,
                health_before: 200,
                health_after: 160,
            },
        );
        add(
            &mut events,
            700,
            SupportTransition::Windup {
                player_position: [5, 0, 8],
                origin: lancer::SPAWN,
                target: [5, 0, 8],
            },
        );
        add(
            &mut events,
            900,
            SupportTransition::Repaired {
                target_id: 2,
                amount: 8,
                health_before: 160,
                health_after: 168,
            },
        );
        add(
            &mut events,
            2500,
            SupportTransition::Charged {
                player_position: [5, 0, 9],
                origin: lancer::SPAWN,
                target: [5, 0, 8],
                damage: 0,
                health_after: 90,
            },
        );
        for (i, (target_id, health)) in [
            (3, 120u32),
            (3, 80),
            (3, 40),
            (2, 168),
            (2, 128),
            (2, 88),
            (2, 48),
            (2, 8),
        ]
        .into_iter()
        .enumerate()
        {
            add(
                &mut events,
                2600 + u64::try_from(i).unwrap() * 250,
                SupportTransition::Attack {
                    target_id,
                    player_position: [5, 0, 9],
                    damage: 40,
                    health_before: health,
                    health_after: health.saturating_sub(40),
                },
            );
        }
        events
    }

    #[test]
    fn support_replay_reconstructs_repairs_partial_sessions_and_two_atomic_deaths() {
        let events = trace();
        for end in 2..=events.len() {
            let replay = reconstruct(&events[..end]).unwrap();
            assert_eq!(replay.spawned_enemies, 2);
            assert_eq!(
                replay.field_objectives["relay_mender"],
                if end == events.len() {
                    "Completed"
                } else {
                    "Active"
                }
            );
            assert!(!replay.completed);
        }
        assert_eq!(reconstruct(&events).unwrap().defeated_enemies, 2);
    }

    #[test]
    fn support_replay_rejects_forged_heals_early_pulses_wrong_actors_and_split_death() {
        for corruption in [
            "amount",
            "before",
            "target",
            "early",
            "actor",
            "death",
            "duplicate_spawn",
            "dead_support",
            "wrong_kind",
            "no_terminal",
        ] {
            let mut events = trace();
            match corruption {
                "amount" => {
                    events[4].payload = events[4].payload.replace("\"amount\":8", "\"amount\":9");
                }
                "before" => {
                    events[4].payload = events[4]
                        .payload
                        .replace("\"health_before\":160", "\"health_before\":159");
                }
                "target" => {
                    events[4].payload = events[4]
                        .payload
                        .replace("\"target_id\":2", "\"target_id\":3");
                }
                "early" => events[4].payload = events[4].payload.replace("900", "899"),
                "actor" => events[4].actor_id = Some(3),
                "death" => events.last_mut().unwrap().kind = ReplayEventKind::FieldActivity,
                "duplicate_spawn" => {
                    events[1].payload = events[1]
                        .payload
                        .replace("\"mender_id\":3", "\"mender_id\":2");
                }
                "dead_support" => {
                    events.truncate(9);
                    add(
                        &mut events,
                        4000,
                        SupportTransition::Repaired {
                            target_id: 2,
                            amount: 8,
                            health_before: 168,
                            health_after: 176,
                        },
                    );
                }
                "wrong_kind" => events[4].kind = ReplayEventKind::EnemySpawned,
                _ => {
                    events.truncate(7);
                    events.push(event(
                        8,
                        ReplayEventKind::BossSpawned,
                        9,
                        "boss spawned: warden".to_owned(),
                    ));
                }
            }
            assert!(reconstruct(&events).is_err(), "{corruption}");
        }
    }

    #[test]
    fn support_replay_accepts_withdrawal_and_lethal_charge_without_rewards() {
        for lethal in [false, true] {
            let mut events = trace();
            events.truncate(4);
            if lethal {
                events[1].payload = events[1]
                    .payload
                    .replace("\"player_health\":90", "\"player_health\":7");
                add(
                    &mut events,
                    2500,
                    SupportTransition::Charged {
                        player_position: [5, 0, 8],
                        origin: lancer::SPAWN,
                        target: [5, 0, 8],
                        damage: 18,
                        health_after: 0,
                    },
                );
            } else {
                add(&mut events, 800, SupportTransition::Abandoned);
            }
            let replay = reconstruct(&events).unwrap();
            assert_eq!(replay.field_objectives["relay_mender"], "Failed");
            assert_eq!(replay.defeated_enemies, 0);
            assert!(!replay.completed);
        }
    }
}
