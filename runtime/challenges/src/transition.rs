use serde::{Deserialize, Serialize};

use crate::modifiers::Preset;
use crate::{
    AttemptResult, ChallengeError, ChallengeRun, ChallengeState, ContractId, Equipment, Objective,
    Outcome, ELITE_REVISION, EXPANDED_REVISION, GAUNTLET_REVISION, MAX_REVISION, MODIFIER_REVISION,
    PRISM_REVISION, REVISION, SIGNAL_REVISION, SURVIVAL_REVISION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Abandoned,
    Defeated,
    Interrupted,
}

impl From<EndReason> for Outcome {
    fn from(reason: EndReason) -> Self {
        match reason {
            EndReason::Abandoned => Self::Abandoned,
            EndReason::Defeated => Self::Defeated,
            EndReason::Interrupted => Self::Interrupted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Start {
        contract: ContractId,
        equipment: Equipment,
    },
    StartModified {
        contract: ContractId,
        preset: Preset,
        equipment: Equipment,
    },
    Retry {
        run_id: u64,
        equipment: Equipment,
    },
    Confirm {
        run_id: u64,
        objective: Objective,
    },
    ConfirmTimed {
        run_id: u64,
        objective: Objective,
        elapsed_ms: u64,
    },
    End {
        run_id: u64,
        reason: EndReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub state: ChallengeState,
    /// A non-power completion fact, with no fragment, XP or item grant.
    pub first_completion: Option<ContractId>,
}

/// Applies one authenticated command. A durable operation ID must resolve an
/// identical transport retry before calling this function with a stale revision.
///
/// # Errors
/// Rejects invalid policy/state/equipment, stale revisions, reused attempt IDs,
/// duplicate objectives and incomplete or unrelated objective sequences.
pub fn apply(
    policy: &str,
    before: &ChallengeState,
    expected_revision: u64,
    command: &Command,
) -> Result<Transition, ChallengeError> {
    if ![
        REVISION,
        EXPANDED_REVISION,
        ELITE_REVISION,
        PRISM_REVISION,
        SIGNAL_REVISION,
        SURVIVAL_REVISION,
        GAUNTLET_REVISION,
        MODIFIER_REVISION,
    ]
    .contains(&policy)
        || !before.supports_policy(policy)
    {
        return Err(ChallengeError::UnsupportedRevision);
    }
    before.validate()?;
    if before.revision != expected_revision {
        return Err(ChallengeError::StaleRevision);
    }
    let mut next = before.clone();
    next.revision = before
        .revision
        .checked_add(1)
        .filter(|v| *v <= MAX_REVISION)
        .ok_or(ChallengeError::RevisionOverflow)?;
    let mut first_completion = None;
    match command {
        Command::Start {
            contract,
            equipment,
        } => {
            if !contract.available_in(policy) {
                return Err(ChallengeError::UnsupportedRevision);
            }
            start(&mut next, *contract, Preset::Baseline, equipment)?;
        }
        Command::StartModified {
            contract,
            preset,
            equipment,
        } => {
            if policy != MODIFIER_REVISION {
                return Err(ChallengeError::UnsupportedRevision);
            }
            if preset.is_baseline() {
                return Err(ChallengeError::InvalidModifiers);
            }
            start(&mut next, *contract, *preset, equipment)?;
        }
        Command::Retry { run_id, equipment } => {
            let previous = next
                .active
                .as_ref()
                .or_else(|| next.last_result.as_ref().map(|r| &r.run))
                .filter(|r| r.run_id == *run_id)
                .ok_or(ChallengeError::WrongRun)?;
            let contract = previous.rules.contract;
            let preset = previous.rules.preset;
            if next.active.is_some() {
                end(&mut next, *run_id, Outcome::Interrupted)?;
            }
            start(&mut next, contract, preset, equipment)?;
        }
        Command::Confirm { run_id, objective } => {
            first_completion = confirm(&mut next, *run_id, *objective, None)?;
        }
        Command::ConfirmTimed {
            run_id,
            objective,
            elapsed_ms,
        } => {
            first_completion = confirm(&mut next, *run_id, *objective, Some(*elapsed_ms))?;
        }
        Command::End { run_id, reason } => end(&mut next, *run_id, (*reason).into())?,
    }
    next.validate()?;
    Ok(Transition {
        state: next,
        first_completion,
    })
}

fn start(
    state: &mut ChallengeState,
    contract: ContractId,
    preset: Preset,
    equipment: &Equipment,
) -> Result<(), ChallengeError> {
    if state.active.is_some() {
        return Err(ChallengeError::ActiveRun);
    }
    equipment.validate()?;
    if contract == ContractId::DistantSignal {
        crate::signal::SignalRun::new(equipment)?;
    }
    state.active = Some(ChallengeRun {
        run_id: state.revision,
        rules: contract.rules_with_preset(preset)?,
        equipment: equipment.clone(),
        objectives: 0,
        elapsed_ms: None,
    });
    Ok(())
}

fn end(state: &mut ChallengeState, run_id: u64, outcome: Outcome) -> Result<(), ChallengeError> {
    if state.active.as_ref().is_none_or(|r| r.run_id != run_id) {
        return Err(ChallengeError::WrongRun);
    }
    state.last_result = state
        .active
        .take()
        .map(|run| AttemptResult { run, outcome });
    Ok(())
}

fn confirm(
    state: &mut ChallengeState,
    run_id: u64,
    objective: Objective,
    elapsed_ms: Option<u64>,
) -> Result<Option<ContractId>, ChallengeError> {
    let run = state
        .active
        .as_mut()
        .filter(|r| r.run_id == run_id)
        .ok_or(ChallengeError::WrongRun)?;
    let bit = run
        .rules
        .objective_bit(objective)
        .ok_or(ChallengeError::WrongObjective)?;
    if run.objectives & bit != 0
        || (objective == Objective::Returned && run.objectives != 7)
        || (matches!(
            run.rules.contract,
            ContractId::CoolantRecovery | ContractId::LastReserve | ContractId::RelayGauntlet
        ) && run.objectives != bit - 1)
    {
        return Err(ChallengeError::WrongObjective);
    }
    let complete = run.objectives | bit == run.rules.contract.complete_mask();
    if elapsed_ms.is_some()
        != (complete && run.rules.preset.time_goal_ms(run.rules.contract).is_some())
    {
        return Err(ChallengeError::WrongObjective);
    }
    run.objectives |= bit;
    run.elapsed_ms = elapsed_ms;
    let mut first = None;
    if complete {
        if let Some(record) = state.records.iter_mut().find(|r| r.rules == run.rules) {
            if elapsed_ms
                .zip(record.elapsed_ms)
                .is_some_and(|(new, old)| new < old)
            {
                *record = run.clone();
            }
        } else {
            first = Some(run.rules.contract);
            state.records.push(run.clone());
        }
        end(state, run_id, Outcome::Completed)?;
    }
    Ok(first)
}
