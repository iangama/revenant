//! One fresh solo session per challenge attempt. Durable character history is
//! checked separately by persistence; this module binds transitions to the
//! session's admitted equipment and immutable server world evidence.
use std::collections::BTreeSet;

use revenant_challenges::{ChallengeState, Command, ContractId, EndReason, Objective, REVISION};
use serde::{Deserialize, Serialize};

use crate::{
    decode_challenge_payload, decode_module_payload, invalid_module, ChallengeReplayPayload,
    DecodedModuleReplayPayload, ReplayError, ReplayEvent, ReplayEventKind as Kind,
    ReplayProtocolGeneration, SupportEvidence, SupportTransition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedChallenge {
    pub character_id: String,
    pub state: ChallengeState,
    pub checkpoints: usize,
}

/// The gateway supplies its actor's actual position. Persistence must write this
/// together with its checkpoint; neither object is accepted from the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeVisitProof {
    pub policy_revision: String,
    pub character_id: String,
    pub state_revision: u64,
    pub run_id: u64,
    pub objective: Objective,
    pub position: [i32; 3],
}

impl ChallengeVisitProof {
    /// # Errors
    /// Rejects foreign objectives, stale runs and positions outside the station.
    pub fn new(
        character: &str,
        state: &ChallengeState,
        command: &Command,
        position: [i32; 3],
    ) -> Result<Self, ReplayError> {
        Self::for_policy(REVISION, character, state, command, position)
    }

    /// # Errors
    /// Rejects unsupported policies and recovery holds, which require a complete
    /// movement/time stream rather than an instantaneous spatial visit.
    pub fn for_policy(
        policy: &str,
        character: &str,
        state: &ChallengeState,
        command: &Command,
        position: [i32; 3],
    ) -> Result<Self, ReplayError> {
        if state
            .active
            .as_ref()
            .is_some_and(|run| run.rules.contract == ContractId::CoolantRecovery)
        {
            return Err(invalid_module("recovery requires continuous-hold evidence"));
        }
        revenant_challenges::apply(policy, state, state.revision, command)
            .map_err(|e| invalid_module(e.to_string()))?;
        let Command::Confirm { run_id, objective } = command else {
            return Err(invalid_module("challenge visit requires a confirm command"));
        };
        let station = state
            .active
            .as_ref()
            .and_then(|run| run.rules.objective_position(*objective));
        if character.is_empty() || character.len() > 256 || station != Some(position) {
            return Err(invalid_module(
                "challenge visit differs from the actual station",
            ));
        }
        Ok(Self {
            policy_revision: policy.to_owned(),
            character_id: character.to_owned(),
            state_revision: state.revision,
            run_id: *run_id,
            objective: *objective,
            position,
        })
    }

    /// # Errors
    /// Returns an error if the spatial evidence cannot be encoded.
    pub fn encode(&self) -> Result<String, ReplayError> {
        serde_json::to_string(self).map_err(|e| invalid_module(e.to_string()))
    }
}

pub(super) fn reconstruct(
    events: &[ReplayEvent],
) -> Result<Option<ReconstructedChallenge>, ReplayError> {
    let first = events
        .iter()
        .position(|e| e.kind == Kind::ChallengeCheckpoint);
    let challenge_session = events.iter().any(|e| {
        matches!(e.kind, Kind::ChallengeCheckpoint | Kind::ChallengeObjective)
            || e.payload.starts_with(crate::challenge_recovery::PREFIX)
            || e.payload.starts_with(crate::challenge_signal::PREFIX)
            || e.payload.starts_with(crate::challenge_survival::PREFIX)
            || e.payload.starts_with(crate::challenge_gauntlet::PREFIX)
            || e.payload.starts_with(crate::challenge_route::PREFIX)
            || ContractId::ALL
                .iter()
                .any(|c| e.activity_id.as_deref() == Some(c.activity_id()))
    });
    if !challenge_session {
        return Ok(None);
    }
    let Some(first) = first else {
        if events.iter().any(|e| !admission_kind(e.kind)) {
            return Err(invalid_module(
                "challenge world evidence precedes admission",
            ));
        }
        return Ok(None);
    };
    let admission = &events[first];
    let payload = decode_challenge_payload(&admission.payload)?;
    validate_admission(&events[..first], admission, &payload)?;
    let mut result = ReconstructedChallenge {
        character_id: payload.character_id.clone(),
        state: payload.after,
        checkpoints: 1,
    };
    let mut operations = BTreeSet::from([payload.operation_id]);
    let mut boundary = admission;
    for (index, event) in events.iter().enumerate().skip(first + 1) {
        if !same_session_owner(admission, event) || !world_kind(event) {
            return Err(invalid_module(
                "challenge session contains foreign or unrelated events",
            ));
        }
        if result.state.active.is_none() {
            return Err(invalid_module(
                "challenge session continued after its terminal",
            ));
        }
        if event.kind == Kind::ChallengeObjective {
            let next = events
                .get(index + 1)
                .filter(|e| e.kind == Kind::ChallengeCheckpoint)
                .ok_or_else(|| invalid_module("challenge visit lacks its atomic checkpoint"))?;
            let next_payload = decode_challenge_payload(&next.payload)?;
            if next_payload.proof_event_id != Some(event.id) {
                return Err(invalid_module(
                    "challenge visit is not consumed by its checkpoint",
                ));
            }
        }
        if event.kind != Kind::ChallengeCheckpoint {
            continue;
        }
        let entry = decode_challenge_payload(&event.payload)?;
        if entry.character_id != result.character_id
            || entry.before != result.state
            || !operations.insert(entry.operation_id.clone())
            || event.actor_id != admission.actor_id
        {
            return Err(invalid_module(
                "challenge session branches or changes participant",
            ));
        }
        verify_transition(&events[..index], boundary, event, &entry)?;
        result.state = entry.after;
        result.checkpoints += 1;
        boundary = event;
    }
    validate_encounter(&events[first..], &result)?;
    Ok(Some(result))
}

fn admission_kind(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::PlayerJoined | Kind::ModuleStateSnapshot | Kind::ActivityStarted
    )
}

fn same_session_owner(left: &ReplayEvent, right: &ReplayEvent) -> bool {
    left.session_id == right.session_id
        && left.account_id == right.account_id
        && left.activity_id == right.activity_id
}

fn world_kind(event: &ReplayEvent) -> bool {
    matches!(
        event.kind,
        Kind::ChallengeCheckpoint | Kind::ChallengeObjective
    ) || (matches!(
        event.kind,
        Kind::EnemySpawned | Kind::BossSpawned | Kind::EnemyDied | Kind::FieldActivity
    ) && (event.payload.starts_with(crate::challenge_route::PREFIX)
        || event.payload.starts_with(crate::challenge_gauntlet::PREFIX)
        || event.payload.starts_with(crate::challenge_survival::PREFIX)
        || event.payload.starts_with(crate::challenge_signal::PREFIX)
        || event.payload.starts_with(crate::support::PREFIX)
        || event.payload.starts_with(crate::elite::PREFIX)))
        || (event.kind == Kind::FieldActivity
            && event.payload.starts_with(crate::challenge_recovery::PREFIX))
        || (matches!(
            event.kind,
            Kind::BossSpawned | Kind::EnemyDied | Kind::FieldActivity
        ) && event.payload.starts_with(crate::prism::PREFIX))
}

pub(super) fn validate_admission(
    prefix: &[ReplayEvent],
    event: &ReplayEvent,
    payload: &ChallengeReplayPayload,
) -> Result<(), ReplayError> {
    let (Command::Start { equipment, .. }
    | Command::StartModified { equipment, .. }
    | Command::Retry { equipment, .. }) = &payload.command
    else {
        return Err(invalid_module(
            "challenge session must start a fresh attempt",
        ));
    };
    let run = payload
        .after
        .active
        .as_ref()
        .ok_or_else(|| invalid_module("challenge admission lacks its attempt"))?;
    let snapshots: Vec<_> = prefix
        .iter()
        .filter(|e| e.kind == Kind::ModuleStateSnapshot)
        .collect();
    let players: Vec<_> = prefix
        .iter()
        .filter(|e| e.kind == Kind::PlayerJoined)
        .collect();
    if event.actor_id.is_none_or(|id| id == 0)
        || event.account_id.is_empty()
        || event.session_id.is_empty()
        || event.activity_id.as_deref() != Some(run.rules.contract.activity_id())
        || snapshots.len() != 1
        || players.len() != 1
        || prefix.iter().any(|e| {
            !admission_kind(e.kind) || !same_session_owner(event, e) || e.actor_id != event.actor_id
        })
    {
        return Err(invalid_module(
            "challenge requires one fresh solo V2 admission",
        ));
    }
    let snapshot = snapshots[0];
    let Some(DecodedModuleReplayPayload::ModuleStateSnapshot(module)) =
        decode_module_payload(snapshot.kind, &snapshot.payload)?
    else {
        return Err(invalid_module("challenge module admission is absent"));
    };
    if payload.proof_event_id != Some(snapshot.id)
        || module.character_id != payload.character_id
        || module.protocol_generation != ReplayProtocolGeneration::V2
        || equipment.catalog_revision != module.state.catalog_revision
        || equipment.loadout_revision != module.state.loadout_revision
        || equipment.modules != module.state.applied_loadout
        || !module
            .state
            .weapons
            .iter()
            .any(|w| w.item_id == equipment.weapon_id)
    {
        return Err(invalid_module(
            "challenge equipment differs from admitted modules",
        ));
    }
    Ok(())
}

fn verify_special_terminal(
    prefix: &[ReplayEvent],
    proof: &ReplayEvent,
    entry: &ChallengeReplayPayload,
    player: Option<u64>,
) -> Option<Result<(), ReplayError>> {
    let run = entry.before.active.as_ref()?;
    match run.rules.contract {
        ContractId::MeridianCircuit if !run.rules.preset.is_baseline() => Some(
            crate::challenge_route::verify_terminal(prefix, proof, &entry.command, run),
        ),
        ContractId::RelayGauntlet => Some(crate::challenge_gauntlet::verify_terminal(
            prefix,
            proof,
            &entry.command,
        )),
        ContractId::LastReserve => Some(crate::challenge_survival::verify_terminal(
            prefix,
            proof,
            &entry.command,
            player,
        )),
        ContractId::DistantSignal => Some(crate::challenge_signal::verify_terminal(
            prefix,
            proof,
            &entry.command,
            player,
        )),
        ContractId::PrismDiscipline => Some(crate::challenge_prism::verify_terminal(
            prefix,
            proof,
            &entry.command,
            player,
        )),
        ContractId::BastionLink => Some(crate::challenge_elite::verify_terminal(
            prefix,
            proof,
            &entry.command,
            player,
        )),
        _ => None,
    }
}

fn verify_transition(
    prefix: &[ReplayEvent],
    boundary: &ReplayEvent,
    event: &ReplayEvent,
    entry: &ChallengeReplayPayload,
) -> Result<(), ReplayError> {
    if matches!(
        entry.command,
        Command::Start { .. } | Command::StartModified { .. } | Command::Retry { .. }
    ) {
        return Err(invalid_module("challenge retry requires a fresh session"));
    }
    if matches!(
        entry.command,
        Command::End {
            reason: EndReason::Abandoned | EndReason::Interrupted,
            ..
        }
    ) {
        return Ok(());
    }
    let proof = prefix
        .iter()
        .find(|e| Some(e.id) == entry.proof_event_id)
        .filter(|e| e.id > boundary.id && same_session_owner(event, e))
        .ok_or_else(|| invalid_module("challenge proof is outside the current attempt boundary"))?;
    if let Some(result) = verify_special_terminal(prefix, proof, entry, event.actor_id) {
        return result;
    }
    match entry.command {
        Command::Confirm { run_id, objective }
            if entry
                .before
                .active
                .as_ref()
                .is_some_and(|run| run.rules.contract == ContractId::CoolantRecovery) =>
        {
            let evidence = crate::RecoveryEvidence::decode(&proof.payload)?;
            if proof.kind != Kind::FieldActivity
                || proof.actor_id != event.actor_id
                || prefix.last() != Some(proof)
                || evidence.run_id != run_id
                || evidence.objective() != Some(objective)
            {
                return Err(invalid_module(
                    "recovery checkpoint lacks its atomic server step",
                ));
            }
        }
        Command::Confirm { objective, .. } if objective.position().is_some() => {
            if proof.kind != Kind::ChallengeObjective
                || proof.actor_id != event.actor_id
                || proof.payload.len() > 2048
                || prefix.last() != Some(proof)
            {
                return Err(invalid_module(
                    "challenge visit is not an atomic actor proof",
                ));
            }
            let actual: ChallengeVisitProof =
                serde_json::from_str(&proof.payload).map_err(|e| invalid_module(e.to_string()))?;
            let expected = ChallengeVisitProof::for_policy(
                &entry.policy_revision,
                &entry.character_id,
                &entry.before,
                &entry.command,
                actual.position,
            )?;
            if actual != expected {
                return Err(invalid_module("challenge visit belongs to another attempt"));
            }
        }
        Command::Confirm { objective, .. } => verify_combat(prefix, proof, objective)?,
        Command::End {
            reason: EndReason::Defeated,
            ..
        } => {
            let evidence = support_evidence(proof)?;
            if proof.kind != Kind::FieldActivity
                || proof.actor_id != event.actor_id
                || !matches!(
                    evidence.transition,
                    SupportTransition::Charged {
                        health_after: 0,
                        damage: 1..,
                        ..
                    }
                )
            {
                return Err(invalid_module("challenge defeat lacks a fatal hit"));
            }
        }
        _ => return Err(invalid_module("unsupported challenge proof command")),
    }
    Ok(())
}

fn support_evidence(event: &ReplayEvent) -> Result<SupportEvidence, ReplayError> {
    if event.payload.len() > 1024 {
        return Err(invalid_module("challenge combat proof exceeds its bound"));
    }
    let body = event
        .payload
        .strip_prefix(crate::support::PREFIX)
        .ok_or_else(|| invalid_module("challenge combat lacks support evidence"))?;
    serde_json::from_str(body).map_err(|e| invalid_module(e.to_string()))
}

fn verify_combat(
    prefix: &[ReplayEvent],
    proof: &ReplayEvent,
    objective: Objective,
) -> Result<(), ReplayError> {
    let spawn = prefix
        .iter()
        .find(|e| e.kind == Kind::EnemySpawned)
        .ok_or_else(|| invalid_module("challenge combat lacks its spawn"))?;
    let SupportTransition::Started {
        lancer_id,
        mender_id,
        ..
    } = support_evidence(spawn)?.transition
    else {
        return Err(invalid_module("challenge spawn has the wrong evidence"));
    };
    let target = match objective {
        Objective::LancerCleared => lancer_id,
        Objective::MenderCleared => mender_id,
        _ => return Err(invalid_module("challenge combat objective is invalid")),
    };
    if proof.kind != Kind::EnemyDied
        || proof.actor_id != Some(target)
        || !matches!(support_evidence(proof)?.transition,
            SupportTransition::Attack { target_id, health_after: 0, .. } if target_id == target)
    {
        return Err(invalid_module(
            "challenge clear lacks the correct fatal attack",
        ));
    }
    Ok(())
}

fn validate_encounter(
    events: &[ReplayEvent],
    result: &ReconstructedChallenge,
) -> Result<(), ReplayError> {
    let run = result
        .state
        .active
        .as_ref()
        .or_else(|| result.state.last_result.as_ref().map(|r| &r.run))
        .ok_or_else(|| invalid_module("challenge encounter lacks a run"))?;
    if run.rules.contract == ContractId::MeridianCircuit && !run.rules.preset.is_baseline() {
        return crate::challenge_route::validate(events, run);
    }
    if run.rules.contract == ContractId::RelayGauntlet {
        return crate::challenge_gauntlet::validate(events, run);
    }
    if run.rules.contract == ContractId::LastReserve {
        return crate::challenge_survival::validate(events, run);
    }
    if run.rules.contract == ContractId::DistantSignal {
        return crate::challenge_signal::validate(events, run);
    }
    if run.rules.contract == ContractId::PrismDiscipline {
        return crate::challenge_prism::validate(events, run);
    }
    if run.rules.contract == ContractId::CoolantRecovery {
        return crate::challenge_recovery::validate(events, run);
    }
    if run.rules.contract == ContractId::BastionLink {
        return crate::challenge_elite::validate(events, run);
    }
    if events.iter().any(|event| {
        event.payload.starts_with(crate::challenge_route::PREFIX)
            || event.payload.starts_with(crate::elite::PREFIX)
            || event.payload.starts_with(crate::challenge_signal::PREFIX)
            || event.payload.starts_with(crate::challenge_survival::PREFIX)
            || event.payload.starts_with(crate::challenge_gauntlet::PREFIX)
            || event.payload.starts_with(crate::prism::PREFIX)
    }) {
        return Err(invalid_module("elite evidence belongs to another contract"));
    }
    if events
        .iter()
        .any(|event| event.payload.starts_with(crate::challenge_recovery::PREFIX))
    {
        return Err(invalid_module(
            "recovery evidence belongs to another contract",
        ));
    }
    if run.rules.contract == ContractId::MeridianCircuit {
        if events
            .iter()
            .any(|e| !matches!(e.kind, Kind::ChallengeCheckpoint | Kind::ChallengeObjective))
        {
            return Err(invalid_module("exploration challenge contains combat"));
        }
    } else if events.iter().any(|e| e.kind == Kind::ChallengeObjective) {
        return Err(invalid_module("combat challenge contains exploration"));
    } else {
        validate_combat_equipment(events, &run.equipment)?;
    }
    Ok(())
}

pub(super) fn validate_combat_equipment(
    events: &[ReplayEvent],
    equipment: &revenant_challenges::Equipment,
) -> Result<(), ReplayError> {
    validate_combat_entry(events, equipment, None)
}

pub(super) fn validate_combat_entry(
    events: &[ReplayEvent],
    equipment: &revenant_challenges::Equipment,
    entry_health: Option<u32>,
) -> Result<(), ReplayError> {
    let invalid = || invalid_module("challenge combat differs from admitted equipment");
    let base = revenant_inventory::weapon_profile(&equipment.weapon_id).ok_or_else(invalid)?;
    let build = revenant_modules::resolve_build(
        &equipment.catalog_revision,
        revenant_modules::BaseCombatProfile {
            damage: base.damage,
            range: base.range,
            cooldown_ms: base.cooldown_ms,
            max_health: 100,
        },
        &equipment.modules,
    )
    .map_err(|_| invalid())?;
    let mut lancer_id = None;
    let mut lancer_position = revenant_ai::lancer::SPAWN;
    let mut ready_at = 0;
    for event in events
        .iter()
        .filter(|e| e.payload.starts_with(crate::support::PREFIX))
    {
        let evidence = support_evidence(event)?;
        match evidence.transition {
            SupportTransition::Started {
                lancer_id: id,
                player_health,
                ..
            } => {
                if player_health != entry_health.unwrap_or(build.profile.max_health) {
                    return Err(invalid());
                }
                lancer_id = Some(id);
            }
            SupportTransition::Charged { target, .. } => lancer_position = target,
            SupportTransition::Attack {
                target_id,
                player_position,
                damage,
                ..
            } => {
                let target = if Some(target_id) == lancer_id {
                    lancer_position
                } else {
                    revenant_ai::support::SPAWN
                };
                // Both positions are bounded by the encounter validator before
                // this function. Wide arithmetic also rejects corrupt extremes.
                let distance: i128 = player_position
                    .iter()
                    .zip(target)
                    .map(|(a, b)| (i128::from(*a) - i128::from(b)).pow(2))
                    .sum();
                let distance = i64::try_from(distance).map_err(|_| invalid())?;
                if distance > i64::from(build.profile.range).pow(2)
                    || revenant_inventory::positioned_damage(
                        &equipment.weapon_id,
                        build.profile.damage,
                        distance,
                    ) != Some(damage)
                    || evidence.elapsed_ms < ready_at
                {
                    return Err(invalid());
                }
                ready_at = evidence
                    .elapsed_ms
                    .saturating_add(build.profile.cooldown_ms);
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
