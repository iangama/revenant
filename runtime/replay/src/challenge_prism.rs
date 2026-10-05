//! Build and terminal bindings for the retained two-phase Prism validator.
use revenant_ai::prism;
use revenant_challenges::{ChallengeRun, Command, ContractId, EndReason, Objective};

use crate::{
    invalid_module, PrismEvidence, PrismStep, PrismTransition, ReplayError, ReplayEvent,
    ReplayEventKind as Kind,
};

fn evidence(event: &ReplayEvent) -> Result<PrismEvidence, ReplayError> {
    if event.payload.len() > 2048 {
        return Err(invalid_module("Prism challenge proof exceeds its bound"));
    }
    let body = event
        .payload
        .strip_prefix(crate::prism::PREFIX)
        .ok_or_else(|| invalid_module("Prism challenge lacks boss evidence"))?;
    serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))
}

pub(super) fn admission(events: &[ReplayEvent], spawn: &ReplayEvent) -> Result<bool, ReplayError> {
    if spawn.activity_id.as_deref() != Some(ContractId::PrismDiscipline.activity_id()) {
        return Ok(false);
    }
    let index = events
        .iter()
        .position(|e| e.kind == Kind::ChallengeCheckpoint && e.id < spawn.id)
        .ok_or_else(|| invalid_module("Prism challenge spawn lacks admission"))?;
    let boundary = &events[index];
    let payload = crate::decode_challenge_payload(&boundary.payload)?;
    crate::challenge_authority::validate_admission(&events[..index], boundary, &payload)?;
    if payload
        .after
        .active
        .as_ref()
        .is_none_or(|run| run.rules.contract != ContractId::PrismDiscipline)
        || boundary.session_id != spawn.session_id
        || boundary.account_id != spawn.account_id
        || boundary.activity_id != spawn.activity_id
    {
        return Err(invalid_module("Prism spawn belongs to another challenge"));
    }
    Ok(true)
}

pub(super) fn verify_terminal(
    prefix: &[ReplayEvent],
    proof: &ReplayEvent,
    command: &Command,
    player: Option<u64>,
) -> Result<(), ReplayError> {
    let invalid = || invalid_module("Prism terminal lacks its adjacent fatal event");
    if prefix.last() != Some(proof) {
        return Err(invalid());
    }
    match command {
        Command::Confirm {
            objective: Objective::PrismCleared,
            ..
        } => {
            let spawn = prefix
                .iter()
                .find(|e| e.kind == Kind::BossSpawned)
                .ok_or_else(invalid)?;
            let PrismTransition::Started { boss_id, .. } = evidence(spawn)?.transition else {
                return Err(invalid());
            };
            if proof.kind != Kind::EnemyDied
                || proof.actor_id != Some(boss_id)
                || !matches!(
                    evidence(proof)?.transition,
                    PrismTransition::Attack {
                        health_after: 0,
                        damage: 1..,
                        ..
                    }
                )
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
                    PrismTransition::Step {
                        step: PrismStep::Resolved {
                            health_after: 0,
                            damage: 1..,
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
    let invalid = || invalid_module("Prism challenge differs from its admitted build");
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
    let mut ready_at = 0;
    for (index, event) in events.iter().enumerate() {
        if event.kind == Kind::ChallengeCheckpoint {
            continue;
        }
        let proof = evidence(event)?;
        match proof.transition {
            PrismTransition::Started { player_health, .. } => {
                if player_health != entry_health.unwrap_or(build.profile.max_health) {
                    return Err(invalid());
                }
            }
            PrismTransition::Attack {
                player_position,
                damage,
                health_before,
                health_after,
                ..
            } => {
                let distance: i128 = player_position
                    .iter()
                    .zip(prism::SPAWN)
                    .map(|(a, b)| (i128::from(*a) - i128::from(b)).pow(2))
                    .sum();
                let distance = i64::try_from(distance).map_err(|_| invalid())?;
                if distance > i64::from(build.profile.range).pow(2) || proof.elapsed_ms < ready_at {
                    return Err(invalid());
                }
                let requested = revenant_inventory::positioned_damage(
                    &run.equipment.weapon_id,
                    build.profile.damage,
                    distance,
                )
                .ok_or_else(invalid)?;
                // The retained validator proves vulnerability and both phases;
                // the admitted weapon supplies damage before the phase floor.
                let available = if health_before > prism::PHASE_HEALTH {
                    health_before - prism::PHASE_HEALTH
                } else {
                    health_before
                };
                if damage != 0 && damage != requested.min(available) {
                    return Err(invalid());
                }
                ready_at = proof.elapsed_ms.saturating_add(build.profile.cooldown_ms);
                if health_after == 0 && entry_health.is_none() {
                    require_checkpoint(events, index)?;
                }
            }
            PrismTransition::Step {
                step:
                    PrismStep::Resolved {
                        health_after: 0, ..
                    },
                ..
            } if entry_health.is_none() => require_checkpoint(events, index)?,
            PrismTransition::Step { .. } => {}
            PrismTransition::Abandoned => return Err(invalid()),
        }
    }
    Ok(())
}

fn require_checkpoint(events: &[ReplayEvent], index: usize) -> Result<(), ReplayError> {
    let next = events
        .get(index + 1)
        .filter(|e| e.kind == Kind::ChallengeCheckpoint)
        .ok_or_else(|| invalid_module("Prism fatal event lacks an atomic checkpoint"))?;
    if crate::decode_challenge_payload(&next.payload)?.proof_event_id != Some(events[index].id) {
        return Err(invalid_module(
            "Prism fatal event belongs to another checkpoint",
        ));
    }
    Ok(())
}
