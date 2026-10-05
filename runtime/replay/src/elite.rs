use revenant_actors::{Actor, ActorKind};
use revenant_ai::{
    bulwark::{self, BulwarkAction},
    elite::{EliteAction, EliteBrain, EliteComposition},
    lancer::ChargeAction,
};
use serde::{Deserialize, Serialize};

use crate::{ReplayError, ReplayEvent, ReplayEventKind};

pub const PREFIX: &str = "elite-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EliteEvidence {
    pub elapsed_ms: u64,
    pub previous_confirmed_ms: u64,
    pub transition: EliteTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum EliteTransition {
    Started {
        composition: String,
        bulwark_id: u64,
        partner_id: u64,
        player_health: u32,
    },
    Step {
        player_position: [i32; 3],
        step: EliteStep,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum EliteStep {
    Braced {
        facing: [i32; 2],
    },
    Slam {
        facing: [i32; 2],
        damage: u32,
        health_after: u32,
    },
    Windup {
        origin: [i32; 3],
        target: [i32; 3],
    },
    Charged {
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
}

impl EliteStep {
    #[must_use]
    pub fn from_action(action: EliteAction, player_health: u32) -> Self {
        match action {
            EliteAction::Defense(BulwarkAction::Brace { facing }) => Self::Braced {
                facing: facing.vector(),
            },
            EliteAction::Defense(BulwarkAction::Slam { facing, damage }) => Self::Slam {
                facing: facing.vector(),
                damage,
                health_after: player_health.saturating_sub(damage),
            },
            EliteAction::Charge(ChargeAction::Windup { origin, target }) => {
                Self::Windup { origin, target }
            }
            EliteAction::Charge(ChargeAction::Resolved {
                origin,
                target,
                damage,
            }) => Self::Charged {
                origin,
                target,
                damage,
                health_after: player_health.saturating_sub(damage),
            },
            EliteAction::Repair(action) => Self::Repaired {
                target_id: action.target_id,
                amount: action.amount,
                health_before: action.health_before,
                health_after: action.health_after,
            },
        }
    }
}

/// # Errors
/// Returns a serialization error if the bounded evidence cannot encode.
pub fn encode_elite_evidence(evidence: &EliteEvidence) -> Result<String, serde_json::Error> {
    Ok(format!("{PREFIX}{}", serde_json::to_string(evidence)?))
}

fn actor(id: u64, kind: ActorKind, position: [i32; 3], health: u32) -> Actor {
    Actor {
        id,
        kind,
        position,
        health,
        max_health: health,
        archetype: String::new(),
    }
}

// Validate ordered health and warning transitions, including interrupted sessions.
#[allow(clippy::too_many_lines)]
pub(super) fn validate(
    events: &[ReplayEvent],
) -> Result<Option<(EliteComposition, &'static str)>, ReplayError> {
    let scoped = crate::campaign::counter_encounter_events(events, PREFIX)?;
    let events = scoped.as_deref().unwrap_or(events);
    let invalid = || ReplayError::InvalidFieldActivityEvidence;
    let mut encounter = None;
    let mut owner = "";
    let mut state = "Active";
    let mut player = actor(0, ActorKind::Player, [0; 3], 0);
    let mut guard = actor(0, ActorKind::Enemy, bulwark::SPAWN, bulwark::HEALTH);
    let mut partner = actor(0, ActorKind::Enemy, [0; 3], 0);
    let mut brain = None;
    let mut last_time = 0;
    let mut main_progressed = false;
    let mut pending_action = None;
    let mut pending_death = None;
    for event in events {
        if matches!(
            event.kind,
            ReplayEventKind::BossSpawned | ReplayEventKind::ActivityCompleted
        ) {
            if encounter.is_some() && (state == "Active" || player.health == 0) {
                return Err(invalid());
            }
            main_progressed = true;
        }
        if !event.payload.starts_with(PREFIX) {
            continue;
        }
        if event.payload.len() > 2048 {
            return Err(invalid());
        }
        let evidence: EliteEvidence =
            serde_json::from_str(&event.payload[PREFIX.len()..]).map_err(|_| invalid())?;
        let time = evidence.elapsed_ms;
        if let EliteTransition::Started {
            composition,
            bulwark_id,
            partner_id,
            player_health,
        } = evidence.transition
        {
            let composition = EliteComposition::from_id(&composition).ok_or_else(invalid)?;
            let players: Vec<_> = events
                .iter()
                .filter(|e| e.kind == ReplayEventKind::PlayerJoined)
                .collect();
            if encounter.is_some()
                || main_progressed
                || time != 0
                || evidence.previous_confirmed_ms != 0
                || event.kind != ReplayEventKind::EnemySpawned
                || event.actor_id != Some(bulwark_id)
                || player_health == 0
                || players.len() != 1
                || players[0].actor_id.is_none()
                || players[0].account_id != event.account_id
                || players[0].id >= event.id
                || events.iter().any(|e| {
                    e.id < event.id && [Some(bulwark_id), Some(partner_id)].contains(&e.actor_id)
                })
            {
                return Err(invalid());
            }
            brain = Some(EliteBrain::new(composition, bulwark_id, partner_id).ok_or_else(invalid)?);
            encounter = Some(composition);
            owner = &event.account_id;
            player = actor(
                players[0].actor_id.unwrap(),
                ActorKind::Player,
                composition.entrance(),
                player_health,
            );
            guard.id = bulwark_id;
            let (_, position, health) = composition.partner();
            partner = actor(partner_id, ActorKind::Enemy, position, health);
            continue;
        }
        let ai = brain.as_mut().ok_or_else(invalid)?;
        let death = event.kind == ReplayEventKind::EnemyDied;
        if state != "Active"
            || event.account_id != owner
            || time < last_time
            || !matches!(
                event.kind,
                ReplayEventKind::FieldActivity | ReplayEventKind::EnemyDied
            )
            || (!death && event.actor_id != Some(player.id))
            || (death
                && !matches!(evidence.transition, EliteTransition::Attack { target_id, health_after: 0, .. } if event.actor_id == Some(target_id)))
        {
            return Err(invalid());
        }
        if evidence.previous_confirmed_ms < last_time || evidence.previous_confirmed_ms > time {
            return Err(invalid());
        }
        if let Some(action) = pending_action.take() {
            ai.confirm_at(action, evidence.previous_confirmed_ms);
        }
        if let Some(actor_id) = pending_death.take() {
            ai.retire(actor_id, evidence.previous_confirmed_ms);
        }
        match evidence.transition {
            EliteTransition::Started { .. } => unreachable!(),
            EliteTransition::Step {
                player_position,
                step,
            } => {
                player.position = player_position;
                let expected = ai
                    .tick(time, Some(&guard), Some(&partner), &player)
                    .ok_or_else(invalid)?;
                if EliteStep::from_action(expected, player.health) != step {
                    return Err(invalid());
                }
                pending_action = Some(expected);
                match step {
                    EliteStep::Braced { .. } | EliteStep::Windup { .. } => {}
                    EliteStep::Slam { health_after, .. } => player.health = health_after,
                    EliteStep::Charged {
                        target,
                        health_after,
                        ..
                    } => {
                        partner.position = target;
                        player.health = health_after;
                    }
                    EliteStep::Repaired { health_after, .. } => guard.health = health_after,
                }
                if player.health == 0 {
                    state = "Failed";
                }
            }
            EliteTransition::Attack {
                target_id,
                player_position,
                damage,
                health_before,
                health_after,
            } => {
                let blocked =
                    target_id == guard.id && ai.defense().blocks(guard.position, player_position);
                let target = if target_id == guard.id {
                    &mut guard
                } else if target_id == partner.id {
                    &mut partner
                } else {
                    return Err(invalid());
                };
                if !bulwark::arena_contains(player_position)
                    || target.health == 0
                    || target.health != health_before
                    || (damage == 0) != blocked
                    || health_before.saturating_sub(damage) != health_after
                    || death != (health_after == 0)
                {
                    return Err(invalid());
                }
                target.health = health_after;
                if death {
                    pending_death = Some(target_id);
                }
                if guard.health == 0 && partner.health == 0 {
                    state = "Completed";
                }
            }
            EliteTransition::Abandoned => state = "Failed",
        }
        last_time = time;
    }
    if encounter.is_some()
        && events.iter().any(|event| {
            event.kind.is_route_event()
                || event.kind.is_cooperation_event()
                || event.kind == ReplayEventKind::FieldActivity
                    && !event.payload.starts_with(PREFIX)
                || event.kind == ReplayEventKind::EnemyDied
                    && [Some(guard.id), Some(partner.id)].contains(&event.actor_id)
                    && !event.payload.starts_with(PREFIX)
        })
    {
        return Err(invalid());
    }
    Ok(encounter.map(|composition| (composition, state)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconstruct;

    fn event(id: i64, kind: ReplayEventKind, actor_id: u64, payload: String) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            actor_id: Some(actor_id),
            account_id: "local:elite".to_owned(),
            session_id: "elite-test".to_owned(),
            timestamp: id.to_string(),
            activity_id: Some("relay_awakening".to_owned()),
            payload,
        }
    }

    fn add(
        events: &mut Vec<ReplayEvent>,
        elapsed_ms: u64,
        confirmed: u64,
        transition: EliteTransition,
    ) {
        let (kind, actor_id) = match transition {
            EliteTransition::Started { bulwark_id, .. } => {
                (ReplayEventKind::EnemySpawned, bulwark_id)
            }
            EliteTransition::Attack {
                target_id,
                health_after: 0,
                ..
            } => (ReplayEventKind::EnemyDied, target_id),
            _ => (ReplayEventKind::FieldActivity, 1),
        };
        events.push(event(
            i64::try_from(events.len()).unwrap() + 1,
            kind,
            actor_id,
            encode_elite_evidence(&EliteEvidence {
                elapsed_ms,
                previous_confirmed_ms: confirmed,
                transition,
            })
            .unwrap(),
        ));
    }

    fn start(composition: EliteComposition) -> Vec<ReplayEvent> {
        let mut events = vec![event(
            1,
            ReplayEventKind::PlayerJoined,
            1,
            "player joined".to_owned(),
        )];
        add(
            &mut events,
            0,
            0,
            EliteTransition::Started {
                composition: composition.id().to_owned(),
                bulwark_id: 2,
                partner_id: 3,
                player_health: 90,
            },
        );
        events
    }

    fn step(
        events: &mut Vec<ReplayEvent>,
        time: u64,
        confirmed: u64,
        position: [i32; 3],
        step: EliteStep,
    ) {
        add(
            events,
            time,
            confirmed,
            EliteTransition::Step {
                player_position: position,
                step,
            },
        );
    }

    #[test]
    fn elite_replay_preserves_slow_confirmation_when_heal_precedes_the_next_brace() {
        let mut events = start(EliteComposition::BastionLink);
        add(
            &mut events,
            100,
            0,
            EliteTransition::Attack {
                target_id: 2,
                player_position: [7, 0, -6],
                damage: 40,
                health_before: 240,
                health_after: 200,
            },
        );
        step(
            &mut events,
            900,
            100,
            [7, 0, -6],
            EliteStep::Repaired {
                target_id: 2,
                amount: 8,
                health_before: 200,
                health_after: 208,
            },
        );
        step(
            &mut events,
            1600,
            900,
            [7, 0, -6],
            EliteStep::Slam {
                facing: [-1, 0],
                damage: 0,
                health_after: 90,
            },
        );
        // The slam's write completed at 5000. Its recovery runs through 6800,
        // allowing this repair even though an unconfirmed replay would brace.
        step(
            &mut events,
            5100,
            5000,
            [7, 0, -6],
            EliteStep::Repaired {
                target_id: 2,
                amount: 8,
                health_before: 208,
                health_after: 216,
            },
        );
        assert_eq!(
            reconstruct(&events).unwrap().field_objectives["bastion_link"],
            "Active"
        );
        let good = events.clone();
        for replacement in ["1599", "5101"] {
            events = good.clone();
            events.last_mut().unwrap().payload = events.last().unwrap().payload.replace(
                "\"previous_confirmed_ms\":5000",
                &format!("\"previous_confirmed_ms\":{replacement}"),
            );
            assert!(reconstruct(&events).is_err());
        }
        events = good;
        events.last_mut().unwrap().payload = events
            .last()
            .unwrap()
            .payload
            .replace("\"amount\":8", "\"amount\":9");
        assert!(reconstruct(&events).is_err());
    }

    #[test]
    fn elite_replay_reconstructs_both_pairs_and_rejects_split_or_unrelated_deaths() {
        for composition in [
            EliteComposition::BastionLink,
            EliteComposition::CrossedGuard,
        ] {
            let mut events = start(composition);
            let mut time = 0;
            let mut confirmed = 0;
            for (target_id, initial) in [(2, 240u32), (3, composition.partner().2)] {
                let mut health = initial;
                while health > 0 {
                    let remaining = health.saturating_sub(40);
                    add(
                        &mut events,
                        time,
                        confirmed,
                        EliteTransition::Attack {
                            target_id,
                            player_position: [7, 0, -6],
                            damage: 40,
                            health_before: health,
                            health_after: remaining,
                        },
                    );
                    health = remaining;
                    confirmed = time;
                    time += 250;
                }
            }
            for end in 2..=events.len() {
                let state = reconstruct(&events[..end]).unwrap();
                assert_eq!(state.spawned_enemies, 2);
                assert_eq!(
                    state.field_objectives[composition.id()],
                    if end == events.len() {
                        "Completed"
                    } else {
                        "Active"
                    }
                );
                assert!(!state.completed);
            }
            let mut malformed = events.clone();
            malformed.last_mut().unwrap().kind = ReplayEventKind::FieldActivity;
            assert!(reconstruct(&malformed).is_err());
            malformed = events.clone();
            malformed.last_mut().unwrap().actor_id = Some(99);
            assert!(reconstruct(&malformed).is_err());
            malformed = events;
            malformed.last_mut().unwrap().payload = "enemy died: glass-lancer".to_owned();
            assert!(reconstruct(&malformed).is_err());
        }
    }

    #[test]
    fn crossed_guard_replay_checks_alternation_cancellation_and_withdrawal() {
        let mut events = start(EliteComposition::CrossedGuard);
        step(
            &mut events,
            1600,
            0,
            [6, 0, -8],
            EliteStep::Slam {
                facing: [-1, 0],
                damage: 14,
                health_after: 76,
            },
        );
        step(
            &mut events,
            2200,
            1600,
            [6, 0, -8],
            EliteStep::Windup {
                origin: [9, 0, -9],
                target: [6, 0, -9],
            },
        );
        step(
            &mut events,
            4000,
            2200,
            [6, 0, -8],
            EliteStep::Charged {
                origin: [9, 0, -9],
                target: [6, 0, -9],
                damage: 0,
                health_after: 76,
            },
        );
        step(
            &mut events,
            4600,
            4000,
            [8, 0, -6],
            EliteStep::Braced { facing: [0, 1] },
        );
        assert!(reconstruct(&events).is_ok());
        let mut malformed = events.clone();
        malformed[3].payload = malformed[3]
            .payload
            .replace("\"elapsed_ms\":2200", "\"elapsed_ms\":2199");
        assert!(reconstruct(&malformed).is_err());
        add(&mut events, 4700, 4600, EliteTransition::Abandoned);
        assert_eq!(
            reconstruct(&events).unwrap().field_objectives["crossed_guard"],
            "Failed"
        );
        let mut events = start(EliteComposition::CrossedGuard);
        for hit in 0..6u32 {
            add(
                &mut events,
                u64::from(hit) * 250,
                u64::from(hit.saturating_sub(1)) * 250,
                EliteTransition::Attack {
                    target_id: 2,
                    player_position: [7, 0, -6],
                    damage: 40,
                    health_before: 240 - hit * 40,
                    health_after: 200 - hit * 40,
                },
            );
        }
        step(
            &mut events,
            3600,
            3000,
            [6, 0, -8],
            EliteStep::Windup {
                origin: [9, 0, -9],
                target: [6, 0, -9],
            },
        );
        assert!(
            reconstruct(&events).is_ok(),
            "a delayed fatal write cancels the dead Bulwark's warning"
        );
        events.last_mut().unwrap().payload = events
            .last()
            .unwrap()
            .payload
            .replace("\"elapsed_ms\":3600", "\"elapsed_ms\":3599");
        assert!(reconstruct(&events).is_err());
    }
}
