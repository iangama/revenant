use std::env;
use std::process::ExitCode;

use revenant_ai::WARDEN_PRESSURE_PROFILE;
use revenant_cooperation::{
    CombatProfile, Contributions, CooperationError, CooperationState, LifeState,
    MutationDisposition, ParticipantReward, Phase, Role, Target, TerminalOutcome, CATALOG_REVISION,
    MAX_REVIVE_DISTANCE_SQUARED, OPERATION_DURATION_MS, PARTICIPANT_COUNT, PING_TTL_MS,
    REVIVE_CHANNEL_MS, REVIVE_HEALTH, REVIVE_WINDOW_MS, SUCCESS_REWARD,
};
use revenant_inventory::{ARC_SIDEARM_PROFILE, PULSE_RIFLE_PROFILE};
use revenant_modules::{
    all_canonical_loadouts, resolve_build, BaseCombatProfile, EffectiveCombatProfile, ModuleId,
    CANONICAL_BUILD_COUNT, CATALOG_REVISION as MODULE_CATALOG_REVISION,
};
use serde::Serialize;

const OPERATOR_BASE_HEALTH: u32 = 100;
const BASELINE_TWO_PLAYER_DRONE_HEALTH: u32 = 210;
const BASELINE_TWO_PLAYER_WARDEN_HEALTH: u32 = 360;
const EXPECTED_MATRIX_ROWS: usize = 900;
const DOMAIN_PROFILE: CombatProfile = CombatProfile {
    damage: 40,
    range: 6,
    cooldown_ms: 250,
    max_health: 100,
};

#[derive(Debug, Serialize)]
struct LabReport {
    schema: &'static str,
    catalog_revision: &'static str,
    module_catalog_revision: &'static str,
    matrix_formula: &'static str,
    matrix_rows: Vec<MatrixRow>,
    invariants: MatrixInvariants,
}

#[derive(Debug, Serialize)]
struct MatrixRow {
    row: usize,
    anchor: AdmittedBuild,
    runner: AdmittedBuild,
    contributions: Contributions,
    runner_health_before_hazard: u32,
    hazard_damage: u32,
    runner_health_after_hazard: u32,
    runner_health_after_revive: u32,
    revive_count: u8,
    baseline_drone_health: u32,
    baseline_warden_health: u32,
    baseline_warden_damage: u32,
    maximum_post_revive_incoming_damage: u64,
    nonlethal_post_revive_pressure: bool,
    m27_route_effect: Option<String>,
    rewards: [ParticipantReward; PARTICIPANT_COUNT],
    terminal: TerminalOutcome,
}

#[derive(Debug, Clone, Serialize)]
struct AdmittedBuild {
    weapon: &'static str,
    build: String,
    modules: Vec<ModuleId>,
    profile: CombatProfile,
}

#[derive(Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct MatrixInvariants {
    row_count: usize,
    expected_row_count: usize,
    weapons_per_role: usize,
    builds_per_weapon: usize,
    fixed_revive_health: u32,
    fixed_reward: ParticipantReward,
    every_row_complete: bool,
    every_row_nonlethal: bool,
    every_row_preserves_baseline: bool,
    every_row_has_no_m27_effect: bool,
}

#[derive(Debug, Serialize)]
struct DomainReport {
    schema: &'static str,
    catalog_revision: &'static str,
    timing: TimingContract,
    cases: Vec<DomainCase>,
}

#[derive(Debug, Serialize)]
struct TimingContract {
    operation_ms: u64,
    ping_ttl_ms: u64,
    revive_window_ms: u64,
    revive_channel_ms: u64,
    maximum_revive_distance_squared: i64,
}

#[derive(Debug, Serialize)]
struct DomainCase {
    case: String,
    accepted: bool,
    disposition: Option<MutationDisposition>,
    error: Option<String>,
    unchanged_on_rejection: bool,
    deadline_terminal_applied: bool,
    state: DomainStateView,
}

#[derive(Debug, Serialize)]
struct DomainStateView {
    phase: Phase,
    contributions: Contributions,
    anchor_health: u32,
    anchor_life: LifeState,
    runner_health: u32,
    runner_life: LifeState,
    revive_count: u8,
    terminal: Option<TerminalOutcome>,
    rewards: Option<[ParticipantReward; PARTICIPANT_COUNT]>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cooperation lab failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() > 1
        || arguments
            .iter()
            .any(|value| value != "--json" && value != "--domain-json")
    {
        return Err("usage: revenant-cooperation-lab [--json|--domain-json]".into());
    }
    if arguments
        .first()
        .is_some_and(|value| value == "--domain-json")
    {
        println!("{}", serde_json::to_string_pretty(&build_domain_report()?)?);
        return Ok(());
    }
    let report = build_report()?;
    if arguments.first().is_some_and(|value| value == "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_summary(&report);
    }
    Ok(())
}

fn build_report() -> Result<LabReport, Box<dyn std::error::Error>> {
    let weapons = weapon_profiles();
    let loadouts = all_canonical_loadouts();
    if weapons.len() != 2 || loadouts.len() != CANONICAL_BUILD_COUNT {
        return Err("weapon/build catalog cardinality changed".into());
    }
    let maximum_post_revive_incoming_damage = u64::from(WARDEN_PRESSURE_PROFILE.damage)
        .checked_mul(u64::from(WARDEN_PRESSURE_PROFILE.maximum_counterattacks))
        .ok_or("Warden pressure arithmetic overflowed")?;
    let capacity = weapons
        .len()
        .checked_mul(loadouts.len())
        .and_then(|side| side.checked_mul(side))
        .ok_or("matrix cardinality overflowed")?;
    if capacity != EXPECTED_MATRIX_ROWS {
        return Err("matrix formula no longer resolves to 900 rows".into());
    }
    let mut matrix_rows = Vec::with_capacity(capacity);
    for (anchor_weapon, anchor_base) in weapons {
        for anchor_loadout in &loadouts {
            let anchor = admitted_build(anchor_weapon, anchor_base, anchor_loadout)?;
            for (runner_weapon, runner_base) in weapons {
                for runner_loadout in &loadouts {
                    let runner = admitted_build(runner_weapon, runner_base, runner_loadout)?;
                    matrix_rows.push(run_matrix_row(
                        matrix_rows.len(),
                        anchor.clone(),
                        runner,
                        maximum_post_revive_incoming_damage,
                    )?);
                }
            }
        }
    }
    verify_matrix(&matrix_rows)?;
    Ok(LabReport {
        schema: "RevenantCooperationLabV1",
        catalog_revision: CATALOG_REVISION,
        module_catalog_revision: MODULE_CATALOG_REVISION,
        matrix_formula: "2 anchor weapons * 15 anchor builds * 2 runner weapons * 15 runner builds",
        invariants: MatrixInvariants {
            row_count: matrix_rows.len(),
            expected_row_count: EXPECTED_MATRIX_ROWS,
            weapons_per_role: weapons.len(),
            builds_per_weapon: loadouts.len(),
            fixed_revive_health: REVIVE_HEALTH,
            fixed_reward: SUCCESS_REWARD,
            every_row_complete: true,
            every_row_nonlethal: true,
            every_row_preserves_baseline: true,
            every_row_has_no_m27_effect: true,
        },
        matrix_rows,
    })
}

fn weapon_profiles() -> [(&'static str, BaseCombatProfile); 2] {
    [
        (
            PULSE_RIFLE_PROFILE.item_id,
            BaseCombatProfile {
                damage: PULSE_RIFLE_PROFILE.damage,
                range: PULSE_RIFLE_PROFILE.range,
                cooldown_ms: PULSE_RIFLE_PROFILE.cooldown_ms,
                max_health: OPERATOR_BASE_HEALTH,
            },
        ),
        (
            ARC_SIDEARM_PROFILE.item_id,
            BaseCombatProfile {
                damage: ARC_SIDEARM_PROFILE.damage,
                range: ARC_SIDEARM_PROFILE.range,
                cooldown_ms: ARC_SIDEARM_PROFILE.cooldown_ms,
                max_health: OPERATOR_BASE_HEALTH,
            },
        ),
    ]
}

fn admitted_build(
    weapon: &'static str,
    base: BaseCombatProfile,
    loadout: &[ModuleId],
) -> Result<AdmittedBuild, Box<dyn std::error::Error>> {
    let resolved = resolve_build(MODULE_CATALOG_REVISION, base, loadout)?;
    Ok(AdmittedBuild {
        weapon,
        build: build_name(&resolved.modules),
        modules: resolved.modules,
        profile: cooperation_profile(resolved.profile),
    })
}

fn cooperation_profile(profile: EffectiveCombatProfile) -> CombatProfile {
    CombatProfile {
        damage: profile.damage,
        range: profile.range,
        cooldown_ms: profile.cooldown_ms,
        max_health: profile.max_health,
    }
}

fn build_name(modules: &[ModuleId]) -> String {
    if modules.is_empty() {
        return "empty".to_owned();
    }
    modules
        .iter()
        .map(|module| module.as_str())
        .collect::<Vec<_>>()
        .join("+")
}

fn run_matrix_row(
    row: usize,
    anchor: AdmittedBuild,
    runner: AdmittedBuild,
    maximum_post_revive_incoming_damage: u64,
) -> Result<MatrixRow, Box<dyn std::error::Error>> {
    let runner_health_before_hazard = runner.profile.max_health;
    let mut state = CooperationState::start("matrix-start", [anchor.profile, runner.profile])?;
    state.arrive_anchor(Role::Anchor, Target::RelayAnchor, 1_000)?;
    state.ping(Role::Anchor, Target::RelayConsole, "matrix-ping", 2_000)?;
    state.arrive_runner(Role::Runner, Target::RelayConsole, 3_000)?;
    let runner_health_after_hazard = state.participants()[Role::Runner.index()].current_health;
    state.start_revive(
        Role::Anchor,
        Role::Runner,
        MAX_REVIVE_DISTANCE_SQUARED,
        "matrix-revive",
        4_000,
    )?;
    state.observe_revive(
        Role::Anchor,
        Role::Runner,
        MAX_REVIVE_DISTANCE_SQUARED,
        "matrix-revive",
        6_000,
    )?;
    let runner_health_after_revive = state.participants()[Role::Runner.index()].current_health;
    state.complete_warden(10_000)?;
    let rewards = state.reward().ok_or("successful row has no reward")?;
    Ok(MatrixRow {
        row,
        anchor,
        runner,
        contributions: state.contributions(),
        runner_health_before_hazard,
        hazard_damage: runner_health_before_hazard,
        runner_health_after_hazard,
        runner_health_after_revive,
        revive_count: state.revive_count(),
        baseline_drone_health: BASELINE_TWO_PLAYER_DRONE_HEALTH,
        baseline_warden_health: BASELINE_TWO_PLAYER_WARDEN_HEALTH,
        baseline_warden_damage: WARDEN_PRESSURE_PROFILE.damage,
        maximum_post_revive_incoming_damage,
        nonlethal_post_revive_pressure: maximum_post_revive_incoming_damage
            < u64::from(runner_health_after_revive),
        m27_route_effect: None,
        rewards,
        terminal: state.terminal().ok_or("successful row has no terminal")?,
    })
}

fn verify_matrix(rows: &[MatrixRow]) -> Result<(), Box<dyn std::error::Error>> {
    if rows.len() != EXPECTED_MATRIX_ROWS {
        return Err("cooperation matrix row count changed".into());
    }
    for (expected_index, row) in rows.iter().enumerate() {
        if row.row != expected_index
            || !row.contributions.complete()
            || row.runner_health_before_hazard != row.hazard_damage
            || row.runner_health_after_hazard != 0
            || row.runner_health_after_revive != REVIVE_HEALTH
            || row.revive_count != 1
            || row.baseline_drone_health != BASELINE_TWO_PLAYER_DRONE_HEALTH
            || row.baseline_warden_health != BASELINE_TWO_PLAYER_WARDEN_HEALTH
            || row.baseline_warden_damage != WARDEN_PRESSURE_PROFILE.damage
            || !row.nonlethal_post_revive_pressure
            || row.m27_route_effect.is_some()
            || row.rewards != [SUCCESS_REWARD; PARTICIPANT_COUNT]
            || row.terminal != TerminalOutcome::Succeeded
        {
            return Err(format!("matrix invariant failed at row {expected_index}").into());
        }
    }
    Ok(())
}

fn build_domain_report() -> Result<DomainReport, Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    append_authority_and_retry_cases(&mut cases)?;
    append_timer_cases(&mut cases)?;
    append_channel_cases(&mut cases)?;
    append_terminal_cases(&mut cases)?;
    append_disconnect_cases(&mut cases)?;
    Ok(DomainReport {
        schema: "RevenantCooperationDomainLabV1",
        catalog_revision: CATALOG_REVISION,
        timing: TimingContract {
            operation_ms: OPERATION_DURATION_MS,
            ping_ttl_ms: PING_TTL_MS,
            revive_window_ms: REVIVE_WINDOW_MS,
            revive_channel_ms: REVIVE_CHANNEL_MS,
            maximum_revive_distance_squared: MAX_REVIVE_DISTANCE_SQUARED,
        },
        cases,
    })
}

fn append_authority_and_retry_cases(cases: &mut Vec<DomainCase>) -> Result<(), CooperationError> {
    let mut start = new_state()?;
    record(cases, "start_same_input_retry", &mut start, |state| {
        state.retry_start("domain-start", [DOMAIN_PROFILE, DOMAIN_PROFILE], [100, 100])
    });
    record(cases, "start_second_identifier", &mut start, |state| {
        state.retry_start(
            "domain-second",
            [DOMAIN_PROFILE, DOMAIN_PROFILE],
            [100, 100],
        )
    });
    let conflicting_profile = CombatProfile {
        max_health: 120,
        ..DOMAIN_PROFILE
    };
    record(cases, "start_profile_conflict", &mut start, |state| {
        state.retry_start(
            "domain-start",
            [conflicting_profile, DOMAIN_PROFILE],
            [100, 100],
        )
    });
    record(cases, "start_health_conflict", &mut start, |state| {
        state.retry_start("domain-start", [DOMAIN_PROFILE, DOMAIN_PROFILE], [90, 100])
    });

    let mut wrong_order = new_state()?;
    record(cases, "runner_before_ping", &mut wrong_order, |state| {
        state.arrive_runner(Role::Runner, Target::RelayConsole, 0)
    });
    record(cases, "wrong_anchor_role", &mut wrong_order, |state| {
        state.arrive_anchor(Role::Runner, Target::RelayAnchor, 0)
    });
    record(cases, "wrong_anchor_target", &mut wrong_order, |state| {
        state.arrive_anchor(Role::Anchor, Target::RelayConsole, 0)
    });
    let mut wrong_runner = state_at(Phase::AwaitingRunner)?;
    record(cases, "wrong_runner_role", &mut wrong_runner, |state| {
        state.arrive_runner(Role::Anchor, Target::RelayConsole, 1_000)
    });
    record(cases, "wrong_runner_target", &mut wrong_runner, |state| {
        state.arrive_runner(Role::Runner, Target::RelayAnchor, 1_000)
    });

    let mut ping = state_at(Phase::AwaitingPing)?;
    record(cases, "ping_applied", &mut ping, |state| {
        state.ping(Role::Anchor, Target::RelayConsole, "ping-1", 1_000)
    });
    record(cases, "ping_same_input_retry", &mut ping, |state| {
        state.ping(Role::Anchor, Target::RelayConsole, "ping-1", 1_100)
    });
    record(cases, "ping_second_identifier", &mut ping, |state| {
        state.ping(Role::Anchor, Target::RelayConsole, "ping-2", 1_100)
    });
    record(cases, "ping_wrong_role", &mut ping, |state| {
        state.ping(Role::Runner, Target::RelayConsole, "ping-1", 1_100)
    });
    Ok(())
}

fn append_timer_cases(cases: &mut Vec<DomainCase>) -> Result<(), CooperationError> {
    let mut ping_exact = state_at(Phase::AwaitingRunner)?;
    record(cases, "ping_ttl_exact", &mut ping_exact, |state| {
        state.arrive_runner(Role::Runner, Target::RelayConsole, 5_100)
    });
    let mut ping_expired = state_at(Phase::AwaitingRunner)?;
    record(cases, "ping_ttl_plus_one", &mut ping_expired, |state| {
        state.arrive_runner(Role::Runner, Target::RelayConsole, 5_101)
    });
    let mut revive_exact = state_at(Phase::RunnerDowned)?;
    record(
        cases,
        "revive_window_exact_start",
        &mut revive_exact,
        |state| state.start_revive(Role::Anchor, Role::Runner, 4, "late-revive", 15_100),
    );
    let mut revive_expired = state_at(Phase::RunnerDowned)?;
    record(
        cases,
        "revive_window_plus_one",
        &mut revive_expired,
        |state| state.start_revive(Role::Anchor, Role::Runner, 4, "late-revive", 15_101),
    );
    let mut revive_complete_exact = state_at(Phase::RunnerDowned)?;
    revive_complete_exact.start_revive(Role::Anchor, Role::Runner, 4, "exact-complete", 13_100)?;
    record(
        cases,
        "revive_window_exact_completion",
        &mut revive_complete_exact,
        |state| state.observe_revive(Role::Anchor, Role::Runner, 4, "exact-complete", 15_100),
    );
    let mut operation_exact = state_at(Phase::EncounterActive)?;
    record(
        cases,
        "operation_deadline_exact_success",
        &mut operation_exact,
        |state| state.complete_warden(OPERATION_DURATION_MS),
    );
    let mut operation_expired = state_at(Phase::EncounterActive)?;
    record(
        cases,
        "operation_deadline_plus_one",
        &mut operation_expired,
        |state| state.complete_warden(OPERATION_DURATION_MS + 1),
    );
    Ok(())
}

fn append_channel_cases(cases: &mut Vec<DomainCase>) -> Result<(), CooperationError> {
    let mut downed = state_at(Phase::RunnerDowned)?;
    record(cases, "second_scripted_down", &mut downed, |state| {
        state.arrive_runner(Role::Runner, Target::RelayConsole, 1_000)
    });
    record(cases, "revive_wrong_role", &mut downed, |state| {
        state.start_revive(Role::Runner, Role::Runner, 4, "revive", 1_000)
    });
    record(cases, "revive_wrong_target", &mut downed, |state| {
        state.start_revive(Role::Anchor, Role::Anchor, 4, "revive", 1_000)
    });
    record(cases, "revive_out_of_range", &mut downed, |state| {
        state.start_revive(Role::Anchor, Role::Runner, 5, "revive", 1_000)
    });

    let mut channel = state_at(Phase::ReviveChannel)?;
    record(cases, "channel_same_input_retry", &mut channel, |state| {
        state.start_revive(Role::Anchor, Role::Runner, 4, "revive-1", 2_000)
    });
    record(
        cases,
        "channel_identifier_conflict",
        &mut channel,
        |state| state.start_revive(Role::Anchor, Role::Runner, 4, "revive-2", 2_000),
    );
    record(cases, "channel_before_boundary", &mut channel, |state| {
        state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 3_099)
    });
    record(cases, "channel_exact_boundary", &mut channel, |state| {
        state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 3_100)
    });
    record(cases, "completed_revive_retry", &mut channel, |state| {
        state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 4_000)
    });
    record(cases, "second_revive", &mut channel, |state| {
        state.start_revive(Role::Anchor, Role::Runner, 4, "revive-2", 4_000)
    });

    let mut reversed = state_at(Phase::ReviveChannel)?;
    record(cases, "channel_time_reversal", &mut reversed, |state| {
        state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 1_099)
    });

    let mut cancelled = state_at(Phase::ReviveChannel)?;
    record(
        cases,
        "channel_out_of_range_cancel",
        &mut cancelled,
        |state| state.observe_revive(Role::Anchor, Role::Runner, 5, "revive-1", 2_000),
    );
    record(
        cases,
        "cancelled_identifier_reused",
        &mut cancelled,
        |state| state.start_revive(Role::Anchor, Role::Runner, 4, "revive-1", 2_100),
    );
    Ok(())
}

fn append_terminal_cases(cases: &mut Vec<DomainCase>) -> Result<(), CooperationError> {
    let mut success = state_at(Phase::EncounterActive)?;
    record(cases, "success", &mut success, |state| {
        state.complete_warden(10_000)
    });
    record(cases, "success_terminal_replay", &mut success, |state| {
        state.complete_warden(u64::MAX)
    });
    for role in Role::ALL {
        let mut defeated = state_at(Phase::EncounterActive)?;
        record(
            cases,
            &format!("participant_defeated_{role}"),
            &mut defeated,
            |state| state.defeat_participant(role, 10_000),
        );
    }
    let crash_incomplete = state_at(Phase::ReviveChannel)?;
    cases.push(DomainCase {
        case: "crash_incomplete_snapshot".to_owned(),
        accepted: true,
        disposition: None,
        error: None,
        unchanged_on_rejection: true,
        deadline_terminal_applied: false,
        state: state_view(&crash_incomplete),
    });
    Ok(())
}

fn append_disconnect_cases(cases: &mut Vec<DomainCase>) -> Result<(), CooperationError> {
    for phase in [
        Phase::AwaitingAnchor,
        Phase::AwaitingPing,
        Phase::AwaitingRunner,
        Phase::RunnerDowned,
        Phase::ReviveChannel,
        Phase::EncounterActive,
    ] {
        for role in Role::ALL {
            let mut state = state_at(phase)?;
            record(
                cases,
                &format!("disconnect_{phase:?}_{role}"),
                &mut state,
                |candidate| candidate.disconnect(role, 4_000),
            );
        }
    }
    Ok(())
}

fn new_state() -> Result<CooperationState, CooperationError> {
    CooperationState::start("domain-start", [DOMAIN_PROFILE, DOMAIN_PROFILE])
}

fn state_at(phase: Phase) -> Result<CooperationState, CooperationError> {
    let mut state = new_state()?;
    if phase == Phase::AwaitingAnchor {
        return Ok(state);
    }
    state.arrive_anchor(Role::Anchor, Target::RelayAnchor, 0)?;
    if phase == Phase::AwaitingPing {
        return Ok(state);
    }
    state.ping(Role::Anchor, Target::RelayConsole, "ping-1", 100)?;
    if phase == Phase::AwaitingRunner {
        return Ok(state);
    }
    state.arrive_runner(Role::Runner, Target::RelayConsole, 100)?;
    if phase == Phase::RunnerDowned {
        return Ok(state);
    }
    state.start_revive(Role::Anchor, Role::Runner, 4, "revive-1", 1_100)?;
    if phase == Phase::ReviveChannel {
        return Ok(state);
    }
    state.observe_revive(Role::Anchor, Role::Runner, 4, "revive-1", 3_100)?;
    if phase == Phase::EncounterActive {
        return Ok(state);
    }
    Err(CooperationError::WrongPhase {
        expected: Phase::EncounterActive,
        actual: phase,
    })
}

fn record<F>(cases: &mut Vec<DomainCase>, case: &str, state: &mut CooperationState, action: F)
where
    F: FnOnce(&mut CooperationState) -> Result<MutationDisposition, CooperationError>,
{
    let before = state.clone();
    let result = action(state);
    let deadline_terminal_applied = matches!(result, Err(CooperationError::DeadlineExpired(_)));
    let unchanged_on_rejection = result.is_ok() || deadline_terminal_applied || *state == before;
    let (accepted, disposition, error) = match result {
        Ok(disposition) => (true, Some(disposition), None),
        Err(error @ CooperationError::DeadlineExpired(_)) => (
            true,
            Some(MutationDisposition::Applied),
            Some(error.to_string()),
        ),
        Err(error) => (false, None, Some(error.to_string())),
    };
    cases.push(DomainCase {
        case: case.to_owned(),
        accepted,
        disposition,
        error,
        unchanged_on_rejection,
        deadline_terminal_applied,
        state: state_view(state),
    });
}

fn state_view(state: &CooperationState) -> DomainStateView {
    DomainStateView {
        phase: state.phase(),
        contributions: state.contributions(),
        anchor_health: state.participants()[Role::Anchor.index()].current_health,
        anchor_life: state.participants()[Role::Anchor.index()].life,
        runner_health: state.participants()[Role::Runner.index()].current_health,
        runner_life: state.participants()[Role::Runner.index()].life,
        revive_count: state.revive_count(),
        terminal: state.terminal(),
        rewards: state.reward(),
    }
}

fn print_summary(report: &LabReport) {
    println!("Revenant cooperation lab ({})", report.catalog_revision);
    println!("matrix rows: {}", report.invariants.row_count);
    println!("formula: {}", report.matrix_formula);
    println!(
        "fixed revive: {} health; reward per participant: {} fragments + {} XP",
        report.invariants.fixed_revive_health,
        report.invariants.fixed_reward.fragments,
        report.invariants.fixed_reward.experience
    );
    println!(
        "complete={} nonlethal={} baseline={} no_m27_effect={}",
        report.invariants.every_row_complete,
        report.invariants.every_row_nonlethal,
        report.invariants.every_row_preserves_baseline,
        report.invariants.every_row_has_no_m27_effect
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_is_exact_complete_and_deterministic() {
        let report = build_report().expect("matrix should build");
        assert_eq!(report.matrix_rows.len(), EXPECTED_MATRIX_ROWS);
        assert!(report.invariants.every_row_complete);
        assert!(report.invariants.every_row_nonlethal);
        assert!(report.invariants.every_row_preserves_baseline);
        assert!(report.invariants.every_row_has_no_m27_effect);
        let first = serde_json::to_vec(&report).expect("serialize first report");
        let second = serde_json::to_vec(&build_report().expect("build second report"))
            .expect("serialize second report");
        assert_eq!(first, second);
    }

    #[test]
    fn matrix_covers_each_ordered_weapon_build_pair_once() {
        let report = build_report().expect("matrix should build");
        let mut keys = report
            .matrix_rows
            .iter()
            .map(|row| {
                (
                    row.anchor.weapon,
                    row.anchor.build.clone(),
                    row.runner.weapon,
                    row.runner.build.clone(),
                )
            })
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), EXPECTED_MATRIX_ROWS);
    }

    #[test]
    fn every_matrix_row_preserves_fixed_hazard_revive_reward_and_pressure() {
        let report = build_report().expect("matrix should build");
        for row in report.matrix_rows {
            assert_eq!(row.runner_health_before_hazard, row.hazard_damage);
            assert_eq!(row.runner_health_after_hazard, 0);
            assert_eq!(row.runner_health_after_revive, REVIVE_HEALTH);
            assert_eq!(row.revive_count, 1);
            assert!(row.maximum_post_revive_incoming_damage < u64::from(REVIVE_HEALTH));
            assert_eq!(row.rewards, [SUCCESS_REWARD; PARTICIPANT_COUNT]);
            assert_eq!(row.terminal, TerminalOutcome::Succeeded);
            assert!(row.m27_route_effect.is_none());
        }
    }

    #[test]
    fn lifecycle_report_is_stable_and_covers_all_nonterminal_disconnects() {
        let report = build_domain_report().expect("domain report should build");
        assert_eq!(
            report
                .cases
                .iter()
                .filter(|case| case.case.starts_with("disconnect_"))
                .count(),
            12
        );
        assert!(report
            .cases
            .iter()
            .filter(|case| case.case.starts_with("disconnect_"))
            .all(|case| { case.state.terminal == Some(TerminalOutcome::AbandonedDisconnect) }));
        assert!(report.cases.iter().all(|case| case.unchanged_on_rejection));
        let first = serde_json::to_vec(&report).expect("serialize first report");
        let second = serde_json::to_vec(&build_domain_report().expect("second report"))
            .expect("serialize second report");
        assert_eq!(first, second);
    }
}
