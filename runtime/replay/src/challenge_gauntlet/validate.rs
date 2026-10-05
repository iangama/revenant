use super::{GauntletAction, GauntletEffect, GauntletEvidence};
use crate::{
    decode_challenge_payload, invalid_module, EliteStep, EliteTransition, PrismStep,
    PrismTransition, ReplayError, ReplayEvent, ReplayEventKind as Kind, SupportTransition,
};
use revenant_challenges::{
    gauntlet::{GauntletRun, Phase, Stage},
    ChallengeRun, ContractId,
};
use std::collections::BTreeSet;

fn invalid() -> ReplayError {
    invalid_module("gauntlet stream differs from admitted stages, combat or transfers")
}

struct Tape {
    stage: Stage,
    events: Vec<ReplayEvent>,
    entry_health: u32,
    health: u32,
    living: BTreeSet<u64>,
    started_at: u64,
}

impl Tape {
    fn new(stage: Stage, anchor: &ReplayEvent, health: u32) -> Self {
        // The outer authority already validated the real solo module admission.
        // Project that admitted identity for the retained encounter validator.
        let mut player = anchor.clone();
        player.kind = Kind::PlayerJoined;
        player.payload = "player joined".into();
        Self {
            stage,
            events: vec![player],
            entry_health: health,
            health,
            living: BTreeSet::new(),
            started_at: 0,
        }
    }

    fn push(
        &mut self,
        event: &ReplayEvent,
        proof: &GauntletEvidence,
        position: [i32; 3],
        used: &mut BTreeSet<u64>,
    ) -> Result<(), ReplayError> {
        let (stage, time) = match &proof.action {
            GauntletAction::Support(e) => (Stage::Support, e.elapsed_ms),
            GauntletAction::Elite(e) => (Stage::Bastion, e.elapsed_ms),
            GauntletAction::Prism(e) => (Stage::Prism, e.elapsed_ms),
            _ => return Err(invalid()),
        };
        if stage != self.stage {
            return Err(invalid());
        }
        if let Some((health, ids)) = started(&proof.action) {
            if self.events.len() != 1 || health != self.entry_health {
                return Err(invalid());
            }
            for id in ids {
                if id == 0 || !used.insert(id) {
                    return Err(invalid());
                }
                self.living.insert(id);
            }
            self.started_at = proof.elapsed_ms;
        } else if self.events.len() == 1 {
            return Err(invalid());
        }
        if time > proof.elapsed_ms.saturating_sub(self.started_at)
            || actual_position(&proof.action).is_some_and(|p| p != position)
        {
            return Err(invalid());
        }
        if let Some(health) = health_after(&proof.action) {
            self.health = health;
        }
        if event.kind == Kind::EnemyDied && !self.living.remove(&proof.actor_id) {
            return Err(invalid());
        }
        let mut raw = event.clone();
        raw.payload = proof.action.payload()?;
        self.events.push(raw);
        Ok(())
    }

    fn verify(&self, run: &ChallengeRun) -> Result<(), ReplayError> {
        if self.events.len() == 1 {
            return Ok(());
        }
        let expected = if self.health == 0 {
            "Failed"
        } else if self.living.is_empty() {
            "Completed"
        } else {
            "Active"
        };
        let actual = match self.stage {
            Stage::Support => {
                crate::challenge_authority::validate_combat_entry(
                    &self.events,
                    &run.equipment,
                    Some(self.entry_health),
                )?;
                crate::support::validate(&self.events)?
            }
            Stage::Bastion => {
                crate::challenge_elite::validate_entry(
                    &self.events[1..],
                    run,
                    Some(self.entry_health),
                )?;
                crate::elite::validate(&self.events)?.map(|(_, state)| state)
            }
            Stage::Prism => {
                crate::challenge_prism::validate_entry(
                    &self.events[1..],
                    run,
                    Some(self.entry_health),
                )?;
                crate::prism::validate_encounter(&self.events, true)?
            }
        };
        if actual != Some(expected) {
            return Err(invalid());
        }
        Ok(())
    }
}

pub(crate) fn validate(events: &[ReplayEvent], run: &ChallengeRun) -> Result<(), ReplayError> {
    let anchor = events.first().ok_or_else(invalid)?;
    if run.rules.contract != ContractId::RelayGauntlet || anchor.kind != Kind::ChallengeCheckpoint {
        return Err(invalid());
    }
    let player = anchor.actor_id.ok_or_else(invalid)?;
    let mut world =
        GauntletRun::with_preset(&run.equipment, run.rules.preset).map_err(|_| invalid())?;
    let first = run.rules.preset.gauntlet_stages()[0];
    let mut position = first.entrance();
    let mut used = BTreeSet::from([player]);
    let mut tape = Tape::new(first, anchor, world.health());
    let mut sequence = 0;
    let mut last_time = 0;
    let mut objectives = 0;
    for (index, event) in events.iter().enumerate().skip(1) {
        if event.kind == Kind::ChallengeCheckpoint {
            continue;
        }
        let proof = GauntletEvidence::decode(&event.payload)?;
        sequence += 1;
        if proof.run_id != run.run_id
            || proof.sequence != sequence
            || proof.elapsed_ms < last_time
            || proof.kind() != event.kind
            || event.actor_id != Some(proof.actor_id)
        {
            return Err(invalid());
        }
        last_time = proof.elapsed_ms;
        let mut effect = GauntletEffect::default();
        match &proof.action {
            GauntletAction::Move { position: next } => {
                check_move(
                    &world,
                    position,
                    *next,
                    proof.actor_id == player,
                    tape.events.len() > 1,
                )?;
                position = *next;
            }
            GauntletAction::Observe => {
                if proof.actor_id != player || !matches!(world.phase(), Phase::Transfer(_)) {
                    return Err(invalid());
                }
            }
            _ => {
                let Phase::Combat(stage) = world.phase() else {
                    return Err(invalid());
                };
                tape.push(event, &proof, position, &mut used)?;
                if tape.health == 0 || tape.living.is_empty() {
                    tape.verify(run)?;
                    world = world
                        .finish_encounter(proof.elapsed_ms, stage, tape.health)
                        .map_err(|_| invalid())?;
                    effect.defeated = tape.health == 0;
                    effect.cleared = (!effect.defeated).then_some(stage);
                    if effect.cleared.is_some() {
                        let objective = match stage {
                            Stage::Support => {
                                revenant_challenges::Objective::GauntletSupportCleared
                            }
                            Stage::Bastion => {
                                revenant_challenges::Objective::GauntletBastionCleared
                            }
                            Stage::Prism => revenant_challenges::Objective::GauntletPrismCleared,
                        };
                        objectives |= run.rules.objective_bit(objective).ok_or_else(invalid)?;
                    }
                }
            }
        }
        if matches!(
            proof.action,
            GauntletAction::Move { .. } | GauntletAction::Observe
        ) && matches!(world.phase(), Phase::Transfer(_))
        {
            let next = world
                .observe_at(proof.elapsed_ms, position)
                .map_err(|_| invalid())?;
            effect.recovered_health = next.recovered_health;
            effect.next_stage = next.next_stage;
            world = next.run;
            if let Some(stage) = next.next_stage {
                position = stage.entrance();
                tape = Tape::new(stage, anchor, world.health());
            }
        }
        if effect != proof.effect {
            return Err(invalid());
        }
        check_checkpoint(events, index, event.id, &proof, &run.rules)?;
    }
    tape.verify(run)?;
    if run.objectives != objectives {
        return Err(invalid());
    }
    Ok(())
}

fn check_checkpoint(
    events: &[ReplayEvent],
    index: usize,
    proof_id: i64,
    proof: &GauntletEvidence,
    rules: &revenant_challenges::Rules,
) -> Result<(), ReplayError> {
    if let Some(command) = proof.command_for(rules) {
        let checkpoint = events
            .get(index + 1)
            .filter(|e| e.kind == Kind::ChallengeCheckpoint)
            .ok_or_else(invalid)?;
        let payload = decode_challenge_payload(&checkpoint.payload)?;
        if payload.command != command || payload.proof_event_id != Some(proof_id) {
            return Err(invalid());
        }
    }
    Ok(())
}

fn check_move(
    world: &GauntletRun,
    position: [i32; 3],
    next: [i32; 3],
    owned: bool,
    spawned: bool,
) -> Result<(), ReplayError> {
    let (Phase::Combat(stage) | Phase::Transfer(stage)) = world.phase() else {
        return Err(invalid());
    };
    let distance: i64 = position
        .iter()
        .zip(next)
        .map(|(a, b)| (i64::from(*a) - i64::from(b)).abs())
        .sum();
    if !owned || !stage.walkable(next) || distance != 1 || !spawned {
        return Err(invalid());
    }
    Ok(())
}

fn started(action: &GauntletAction) -> Option<(u32, Vec<u64>)> {
    match action {
        GauntletAction::Support(e) => match e.transition {
            SupportTransition::Started {
                player_health,
                lancer_id,
                mender_id,
            } => Some((player_health, vec![lancer_id, mender_id])),
            _ => None,
        },
        GauntletAction::Elite(e) => match e.transition {
            EliteTransition::Started {
                player_health,
                bulwark_id,
                partner_id,
                ..
            } => Some((player_health, vec![bulwark_id, partner_id])),
            _ => None,
        },
        GauntletAction::Prism(e) => match e.transition {
            PrismTransition::Started {
                player_health,
                boss_id,
            } => Some((player_health, vec![boss_id])),
            _ => None,
        },
        _ => None,
    }
}

fn actual_position(action: &GauntletAction) -> Option<[i32; 3]> {
    match action {
        GauntletAction::Support(e) => match e.transition {
            SupportTransition::Windup {
                player_position, ..
            }
            | SupportTransition::Charged {
                player_position, ..
            }
            | SupportTransition::Attack {
                player_position, ..
            } => Some(player_position),
            _ => None,
        },
        GauntletAction::Elite(e) => match e.transition {
            EliteTransition::Step {
                player_position, ..
            }
            | EliteTransition::Attack {
                player_position, ..
            } => Some(player_position),
            _ => None,
        },
        GauntletAction::Prism(e) => match e.transition {
            PrismTransition::Step {
                player_position, ..
            }
            | PrismTransition::Attack {
                player_position, ..
            } => Some(player_position),
            _ => None,
        },
        _ => None,
    }
}

fn health_after(action: &GauntletAction) -> Option<u32> {
    match action {
        GauntletAction::Support(e) => match e.transition {
            SupportTransition::Charged { health_after, .. } => Some(health_after),
            _ => None,
        },
        GauntletAction::Elite(e) => match e.transition {
            EliteTransition::Step {
                step: EliteStep::Slam { health_after, .. } | EliteStep::Charged { health_after, .. },
                ..
            } => Some(health_after),
            _ => None,
        },
        GauntletAction::Prism(e) => match e.transition {
            PrismTransition::Step {
                step: PrismStep::Resolved { health_after, .. },
                ..
            } => Some(health_after),
            _ => None,
        },
        _ => None,
    }
}
