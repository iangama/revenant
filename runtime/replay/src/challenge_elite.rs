//! Additional challenge bindings around the retained elite encounter validator.
use revenant_ai::{bulwark, elite::EliteComposition};
use revenant_challenges::{ChallengeRun, Command, EndReason, Objective};

use crate::{
    invalid_module, EliteEvidence, EliteStep, EliteTransition, ReplayError, ReplayEvent,
    ReplayEventKind as Kind,
};

fn evidence(event: &ReplayEvent) -> Result<EliteEvidence, ReplayError> {
    if event.payload.len() > 2048 {
        return Err(invalid_module("elite challenge proof exceeds its bound"));
    }
    let body = event
        .payload
        .strip_prefix(crate::elite::PREFIX)
        .ok_or_else(|| invalid_module("elite challenge lacks elite evidence"))?;
    serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))
}

pub(super) fn verify_terminal(
    prefix: &[ReplayEvent],
    proof: &ReplayEvent,
    command: &Command,
    player: Option<u64>,
) -> Result<(), ReplayError> {
    let invalid = || invalid_module("elite challenge terminal lacks its atomic fatal hit");
    if prefix.last() != Some(proof) {
        return Err(invalid());
    }
    let spawn = prefix
        .iter()
        .find(|e| e.kind == Kind::EnemySpawned)
        .ok_or_else(invalid)?;
    let EliteTransition::Started {
        composition,
        bulwark_id,
        partner_id,
        ..
    } = evidence(spawn)?.transition
    else {
        return Err(invalid());
    };
    if composition != EliteComposition::BastionLink.id() {
        return Err(invalid());
    }
    match command {
        Command::Confirm { objective, .. } => {
            let target = match objective {
                Objective::BulwarkCleared => bulwark_id,
                Objective::MenderCleared => partner_id,
                _ => return Err(invalid()),
            };
            if proof.kind != Kind::EnemyDied
                || proof.actor_id != Some(target)
                || !matches!(evidence(proof)?.transition, EliteTransition::Attack { target_id, health_after: 0, damage: 1.., .. } if target_id == target)
            {
                return Err(invalid());
            }
        }
        Command::End {
            reason: EndReason::Defeated,
            ..
        } => {
            if proof.kind != Kind::FieldActivity
                || proof.actor_id != player
                || !matches!(
                    evidence(proof)?.transition,
                    EliteTransition::Step {
                        step: EliteStep::Slam {
                            damage: 1..,
                            health_after: 0,
                            ..
                        },
                        ..
                    }
                )
            {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }
    Ok(())
}

pub(super) fn validate(events: &[ReplayEvent], run: &ChallengeRun) -> Result<(), ReplayError> {
    validate_entry(events, run, None)
}

pub(super) fn validate_entry(
    events: &[ReplayEvent],
    run: &ChallengeRun,
    entry_health: Option<u32>,
) -> Result<(), ReplayError> {
    let invalid = || invalid_module("elite challenge differs from its admitted build or encounter");
    let base = revenant_inventory::weapon_profile(&run.equipment.weapon_id).ok_or_else(invalid)?;
    let build = revenant_modules::resolve_build(
        &run.equipment.catalog_revision,
        revenant_modules::BaseCombatProfile {
            damage: base.damage,
            range: base.range,
            cooldown_ms: base.cooldown_ms,
            max_health: 100,
        },
        &run.equipment.modules,
    )
    .map_err(|_| invalid())?;
    let mut targets = None;
    let mut ready_at = 0;
    for (index, event) in events.iter().enumerate() {
        if event.kind == Kind::ChallengeCheckpoint {
            continue;
        }
        let proof = evidence(event)?;
        match proof.transition {
            EliteTransition::Started {
                composition,
                bulwark_id,
                partner_id,
                player_health,
            } => {
                if targets.is_some()
                    || composition != EliteComposition::BastionLink.id()
                    || player_health != entry_health.unwrap_or(build.profile.max_health)
                {
                    return Err(invalid());
                }
                targets = Some((bulwark_id, partner_id));
            }
            EliteTransition::Attack {
                target_id,
                player_position,
                damage,
                health_after,
                ..
            } => {
                let (guard, partner) = targets.ok_or_else(invalid)?;
                let position = if target_id == guard {
                    bulwark::SPAWN
                } else if target_id == partner {
                    EliteComposition::BastionLink.partner().1
                } else {
                    return Err(invalid());
                };
                let distance: i128 = player_position
                    .iter()
                    .zip(position)
                    .map(|(a, b)| (i128::from(*a) - i128::from(b)).pow(2))
                    .sum();
                let distance = i64::try_from(distance).map_err(|_| invalid())?;
                if distance > i64::from(build.profile.range).pow(2) || proof.elapsed_ms < ready_at {
                    return Err(invalid());
                }
                // The retained elite validator proves whether the shield blocked
                // this shot; weapon geometry still applies to blocked attempts.
                let expected = revenant_inventory::positioned_damage(
                    &run.equipment.weapon_id,
                    build.profile.damage,
                    distance,
                )
                .ok_or_else(invalid)?;
                if damage != 0 && damage != expected {
                    return Err(invalid());
                }
                ready_at = proof.elapsed_ms.saturating_add(build.profile.cooldown_ms);
                if health_after == 0 && entry_health.is_none() {
                    require_checkpoint(events, index)?;
                }
            }
            EliteTransition::Step {
                step: EliteStep::Slam {
                    health_after: 0, ..
                },
                ..
            } if entry_health.is_none() => require_checkpoint(events, index)?,
            EliteTransition::Step { .. } => {}
            EliteTransition::Abandoned => return Err(invalid()),
        }
    }
    Ok(())
}

fn require_checkpoint(events: &[ReplayEvent], index: usize) -> Result<(), ReplayError> {
    let next = events
        .get(index + 1)
        .filter(|e| e.kind == Kind::ChallengeCheckpoint)
        .ok_or_else(|| invalid_module("elite fatal hit lacks an adjacent checkpoint"))?;
    let checkpoint = crate::decode_challenge_payload(&next.payload)?;
    if checkpoint.proof_event_id != Some(events[index].id) {
        return Err(invalid_module(
            "elite fatal hit belongs to another checkpoint",
        ));
    }
    Ok(())
}
