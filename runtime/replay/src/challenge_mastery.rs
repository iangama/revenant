//! Mastery-v1 is a projection of authenticated, immutable attempt evidence.
//! Always validate the whole session before interpreting a clean-play summary.
use revenant_challenges::{
    mastery::{self, Attempt, Facts},
    ContractId,
};

use crate::{
    invalid_module, EliteEvidence, EliteTransition, PrismEvidence, PrismStep, PrismTransition,
    ReplayError, ReplayEvent, SurvivalEvidence,
};

/// Returns a terminal attempt's mastery assessment after full replay validation.
/// Incomplete sessions produce no award. Persistence must separately bind the
/// session to the owner's continuous character journal.
///
/// # Errors
/// Rejects invalid world/admission/checkpoint evidence before deriving any goal.
pub fn challenge_mastery_attempt(events: &[ReplayEvent]) -> Result<Option<Attempt>, ReplayError> {
    let Some(challenge) = crate::reconstruct(events)?.challenge else {
        return Ok(None);
    };
    if challenge.state.active.is_some() {
        return Ok(None);
    }
    let Some(result) = challenge.state.last_result else {
        return Ok(None);
    };
    let facts = match result.run.rules.contract {
        ContractId::LastReserve => reserve_facts(events)?,
        ContractId::MeridianCircuit
            if result
                .run
                .rules
                .preset
                .as_str()
                .starts_with("west_approach") =>
        {
            Facts {
                west_route_moves: Some(
                    events
                        .iter()
                        .filter(|event| event.payload.starts_with(crate::challenge_route::PREFIX))
                        .count() as u64,
                ),
                ..Facts::default()
            }
        }
        ContractId::BastionLink => priority_facts(events)?,
        ContractId::PrismDiscipline => prism_facts(events)?,
        _ => Facts::default(),
    };
    Ok(Some(mastery::assess(&result, &facts)))
}

fn reserve_facts(events: &[ReplayEvent]) -> Result<Facts, ReplayError> {
    let mut seen = false;
    let mut recovered = false;
    for event in events
        .iter()
        .filter(|event| event.payload.starts_with(crate::challenge_survival::PREFIX))
    {
        let evidence = SurvivalEvidence::decode(&event.payload)?;
        seen = true;
        recovered |= evidence.effect.recovered_health > 0;
    }
    Ok(Facts {
        reserve_unused: seen && !recovered,
        ..Facts::default()
    })
}

fn priority_facts(events: &[ReplayEvent]) -> Result<Facts, ReplayError> {
    let mut mender = None;
    let mut bulwark = None;
    let mut mender_dead = false;
    let mut premature_damage = false;
    for event in events {
        let Some(body) = event.payload.strip_prefix(crate::elite::PREFIX) else {
            continue;
        };
        let evidence: EliteEvidence =
            serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))?;
        match evidence.transition {
            EliteTransition::Started {
                bulwark_id,
                partner_id,
                ..
            } => {
                bulwark = Some(bulwark_id);
                mender = Some(partner_id);
            }
            EliteTransition::Attack {
                target_id,
                damage,
                health_after,
                ..
            } => {
                if Some(target_id) == bulwark && damage > 0 && !mender_dead {
                    premature_damage = true;
                }
                if Some(target_id) == mender && health_after == 0 {
                    mender_dead = true;
                }
            }
            _ => {}
        }
    }
    Ok(Facts {
        mender_before_bulwark_damage: mender_dead && !premature_damage,
        ..Facts::default()
    })
}

fn prism_facts(events: &[ReplayEvent]) -> Result<Facts, ReplayError> {
    let mut damage = None::<u64>;
    for event in events {
        let Some(body) = event.payload.strip_prefix(crate::prism::PREFIX) else {
            continue;
        };
        let evidence: PrismEvidence =
            serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))?;
        match evidence.transition {
            PrismTransition::Started { .. } => damage = Some(0),
            PrismTransition::Step {
                step: PrismStep::Resolved { damage: hit, .. },
                ..
            } => {
                damage = Some(
                    damage
                        .unwrap_or(0)
                        .checked_add(u64::from(hit))
                        .ok_or_else(|| invalid_module("mastery damage overflow"))?,
                );
            }
            _ => {}
        }
    }
    Ok(Facts {
        prism_damage: damage,
        ..Facts::default()
    })
}
