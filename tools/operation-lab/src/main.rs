use std::env;
use std::process::ExitCode;

use revenant_ai::{RELAY_DRONE_PRESSURE_PROFILE, WARDEN_PRESSURE_PROFILE};
use revenant_combat::{
    analyze_scenario, AttackProfile, CombatMetrics, CombatScenario, PressureProfile,
};
use revenant_inventory::{ARC_SIDEARM_PROFILE, PULSE_RIFLE_PROFILE};
use revenant_modules::{
    all_canonical_loadouts, resolve_build, BaseCombatProfile, EffectiveCombatProfile, ModuleId,
    CATALOG_REVISION as MODULE_CATALOG_REVISION,
};
use revenant_operations::{
    apply_warden_health_effect, event_definition, resolve_event, route_definition,
    validate_catalog, OperationDisposition, RouteEventDefinition, RouteEventId, RouteId,
    RouteOperationState, RoutePhase, RouteReward, RouteSeed, RouteSelectionRequest,
    TerminalOutcome, CATALOG_REVISION, MAX_ROUTE_SEED, RESOLVER_REVISION, ROUTE_CATALOG,
    ROUTE_EVENT_CATALOG, ROUTE_OPERATION_DURATION_MS,
};
use serde::Serialize;

const OPERATOR_BASE_HEALTH: u32 = 100;
const DRONE_BASE_HEALTH: u32 = 140;
const WARDEN_BASE_HEALTH: u32 = 240;
const TARGET_DISTANCE_SQUARED: i64 = 4;
const PARTICIPANT_COUNTS: [u32; 2] = [1, 2];
const BASELINE_REWARD: RouteReward = RouteReward {
    fragments: 1,
    experience: 100,
};

#[derive(Debug, Serialize)]
struct LabReport {
    schema: &'static str,
    catalog_revision: &'static str,
    resolver_revision: &'static str,
    module_catalog_revision: &'static str,
    duration_budget_ms: u64,
    routes: Vec<revenant_operations::RouteDefinition>,
    events: Vec<RouteEventDefinition>,
    seed_vectors: Vec<SeedVector>,
    routed_rows: Vec<RoutedEncounterRow>,
    baseline_rows: Vec<BaselineEncounterRow>,
    tradeoffs: Vec<TradeoffSummary>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct SeedVector {
    route_id: RouteId,
    seed: u64,
    event_id: RouteEventId,
}

#[derive(Debug, Serialize)]
struct RoutedEncounterRow {
    route_id: RouteId,
    event_id: RouteEventId,
    representative_seed: u64,
    participants: u32,
    weapon: &'static str,
    build: String,
    modules: Vec<ModuleId>,
    profile: ProfileView,
    reward: RouteReward,
    objective_count: usize,
    objective_path: Vec<String>,
    drone: EncounterView,
    warden: EncounterView,
    total_optimal_incoming_damage: u64,
    combat_duration_lower_bound_ms: u64,
    within_duration_budget: bool,
    nonlethal_optimal_pressure: bool,
}

#[derive(Debug, Serialize)]
struct BaselineEncounterRow {
    participants: u32,
    weapon: &'static str,
    build: String,
    modules: Vec<ModuleId>,
    profile: ProfileView,
    reward: RouteReward,
    objective_count: usize,
    drone: EncounterView,
    warden: EncounterView,
    total_optimal_incoming_damage: u64,
    combat_duration_lower_bound_ms: u64,
    nonlethal_optimal_pressure: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct ProfileView {
    damage: u32,
    range: i32,
    cooldown_ms: u64,
    max_health: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct EncounterView {
    target_health: u32,
    required_hits: u32,
    required_volleys: u32,
    theoretical_ttk_ms: u64,
    expected_hostile_attacks: u32,
    expected_incoming_damage: u64,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct RouteAggregate {
    reward: RouteReward,
    total_incoming_sum: u64,
    maximum_incoming: u64,
    duration_lower_bound_sum_ms: u64,
    maximum_duration_lower_bound_ms: u64,
}

#[derive(Debug, Serialize)]
struct TradeoffSummary {
    participants: u32,
    weapon: &'static str,
    build: String,
    breach: RouteAggregate,
    stabilize: RouteAggregate,
    breach_dominates: bool,
    stabilize_dominates: bool,
}

#[derive(Debug, Serialize)]
struct DomainReport {
    schema: &'static str,
    catalog_revision: &'static str,
    cases: Vec<DomainCase>,
}

#[derive(Debug, Serialize)]
struct DomainCase {
    case: &'static str,
    accepted: bool,
    disposition: Option<OperationDisposition>,
    error: Option<String>,
    unchanged_on_error: Option<bool>,
    state: DomainStateView,
}

#[derive(Debug, Serialize)]
struct DomainStateView {
    phase: RoutePhase,
    selected_operation: Option<String>,
    route_id: Option<RouteId>,
    event_id: Option<RouteEventId>,
    terminal_outcome: Option<TerminalOutcome>,
    terminal_reward: Option<RouteReward>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("operation lab failed: {error}");
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
        return Err("usage: revenant-operation-lab [--json|--domain-json]".into());
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
        print_table(&report);
    }
    Ok(())
}

fn build_report() -> Result<LabReport, Box<dyn std::error::Error>> {
    validate_catalog(&ROUTE_CATALOG, &ROUTE_EVENT_CATALOG)?;
    let seed_vectors = seed_vectors();
    let representative_seeds = RouteEventId::ALL
        .into_iter()
        .map(|event_id| representative_seed(event_id).map(|seed| (event_id, seed)))
        .collect::<Result<Vec<_>, _>>()?;
    let weapons = weapon_profiles();
    let mut routed_rows = Vec::with_capacity(240);
    let mut baseline_rows = Vec::with_capacity(60);
    for participants in PARTICIPANT_COUNTS {
        for (weapon, base) in weapons {
            for loadout in all_canonical_loadouts() {
                let resolved = resolve_build(MODULE_CATALOG_REVISION, base, &loadout)?;
                baseline_rows.push(baseline_row(
                    participants,
                    weapon,
                    &resolved.modules,
                    resolved.profile,
                )?);
                for route_id in RouteId::ALL {
                    let route = route_definition(route_id);
                    for event_id in route.events {
                        let representative_seed = representative_seeds
                            .iter()
                            .find_map(|(candidate, seed)| (*candidate == event_id).then_some(*seed))
                            .ok_or("event has no representative seed")?;
                        routed_rows.push(routed_row(
                            route_id,
                            event_id,
                            representative_seed,
                            participants,
                            weapon,
                            &resolved.modules,
                            resolved.profile,
                        )?);
                    }
                }
            }
        }
    }
    let tradeoffs = tradeoff_summaries(&routed_rows)?;
    Ok(LabReport {
        schema: "RevenantOperationLabV1",
        catalog_revision: CATALOG_REVISION,
        resolver_revision: RESOLVER_REVISION,
        module_catalog_revision: MODULE_CATALOG_REVISION,
        duration_budget_ms: ROUTE_OPERATION_DURATION_MS,
        routes: ROUTE_CATALOG.to_vec(),
        events: ROUTE_EVENT_CATALOG.to_vec(),
        seed_vectors,
        routed_rows,
        baseline_rows,
        tradeoffs,
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

fn baseline_row(
    participants: u32,
    weapon: &'static str,
    modules: &[ModuleId],
    profile: EffectiveCombatProfile,
) -> Result<BaselineEncounterRow, Box<dyn std::error::Error>> {
    let drone_health = scaled_enemy_health(DRONE_BASE_HEALTH, participants)?;
    let warden_health = scaled_enemy_health(WARDEN_BASE_HEALTH, participants)?;
    let drone = encounter(
        profile,
        drone_health,
        participants,
        RELAY_DRONE_PRESSURE_PROFILE,
    )?;
    let warden = encounter(
        profile,
        warden_health,
        participants,
        WARDEN_PRESSURE_PROFILE,
    )?;
    let total_incoming = checked_sum(
        drone.expected_incoming_damage,
        warden.expected_incoming_damage,
    )?;
    let duration = checked_sum(drone.theoretical_ttk_ms, warden.theoretical_ttk_ms)?;
    Ok(BaselineEncounterRow {
        participants,
        weapon,
        build: build_label(modules),
        modules: modules.to_vec(),
        profile: profile_view(profile),
        reward: BASELINE_REWARD,
        objective_count: 3,
        drone,
        warden,
        total_optimal_incoming_damage: total_incoming,
        combat_duration_lower_bound_ms: duration,
        nonlethal_optimal_pressure: total_incoming < u64::from(profile.max_health),
    })
}

fn routed_row(
    route_id: RouteId,
    event_id: RouteEventId,
    representative_seed: u64,
    participants: u32,
    weapon: &'static str,
    modules: &[ModuleId],
    profile: EffectiveCombatProfile,
) -> Result<RoutedEncounterRow, Box<dyn std::error::Error>> {
    let route = route_definition(route_id);
    let event = event_definition(event_id);
    if event.route_id != route_id {
        return Err("route/event mismatch".into());
    }
    let seed = RouteSeed::new(representative_seed)?;
    if resolve_event(route_id, seed).event_id != event_id {
        return Err("representative seed resolved a different event".into());
    }
    let drone_health = scaled_enemy_health(DRONE_BASE_HEALTH, participants)?;
    let base_warden_health = scaled_enemy_health(WARDEN_BASE_HEALTH, participants)?;
    let warden_health = apply_warden_health_effect(base_warden_health, event.effect)?;
    let drone = encounter(
        profile,
        drone_health,
        participants,
        RELAY_DRONE_PRESSURE_PROFILE,
    )?;
    let warden = encounter(
        profile,
        warden_health,
        participants,
        PressureProfile {
            damage: event.effect.warden_counter_damage,
            ..WARDEN_PRESSURE_PROFILE
        },
    )?;
    let total_incoming = checked_sum(
        drone.expected_incoming_damage,
        warden.expected_incoming_damage,
    )?;
    let duration = checked_sum(drone.theoretical_ttk_ms, warden.theoretical_ttk_ms)?;
    Ok(RoutedEncounterRow {
        route_id,
        event_id,
        representative_seed,
        participants,
        weapon,
        build: build_label(modules),
        modules: modules.to_vec(),
        profile: profile_view(profile),
        reward: route.reward,
        objective_count: route.objective_path.len(),
        objective_path: route
            .objective_path
            .iter()
            .map(ToString::to_string)
            .collect(),
        drone,
        warden,
        total_optimal_incoming_damage: total_incoming,
        combat_duration_lower_bound_ms: duration,
        within_duration_budget: duration <= route.duration_ms,
        nonlethal_optimal_pressure: total_incoming < u64::from(profile.max_health),
    })
}

fn encounter(
    profile: EffectiveCombatProfile,
    target_health: u32,
    participants: u32,
    pressure: PressureProfile,
) -> Result<EncounterView, revenant_combat::CombatAnalysisError> {
    let metrics = analyze_scenario(CombatScenario {
        attack: AttackProfile {
            damage: profile.damage,
            range: profile.range,
            cooldown_ms: profile.cooldown_ms,
        },
        target_health,
        target_distance_squared: TARGET_DISTANCE_SQUARED,
        attacker_count: participants,
        client_attack_interval_ms: profile.cooldown_ms,
        round_trip_ms: 0,
        pressure,
    })?;
    Ok(encounter_view(target_health, metrics))
}

const fn encounter_view(target_health: u32, metrics: CombatMetrics) -> EncounterView {
    EncounterView {
        target_health,
        required_hits: metrics.required_hits,
        required_volleys: metrics.required_volleys,
        theoretical_ttk_ms: metrics.theoretical_ttk_ms,
        expected_hostile_attacks: metrics.expected_hostile_attacks,
        expected_incoming_damage: metrics.expected_incoming_damage,
    }
}

const fn profile_view(profile: EffectiveCombatProfile) -> ProfileView {
    ProfileView {
        damage: profile.damage,
        range: profile.range,
        cooldown_ms: profile.cooldown_ms,
        max_health: profile.max_health,
    }
}

fn scaled_enemy_health(base_health: u32, participants: u32) -> Result<u32, &'static str> {
    if !PARTICIPANT_COUNTS.contains(&participants) {
        return Err("participant count is outside M27 matrix");
    }
    base_health
        .checked_mul(participants + 1)
        .map(|scaled| scaled / 2)
        .ok_or("enemy health scaling overflowed")
}

fn checked_sum(left: u64, right: u64) -> Result<u64, &'static str> {
    left.checked_add(right).ok_or("operation metric overflowed")
}

fn tradeoff_summaries(
    rows: &[RoutedEncounterRow],
) -> Result<Vec<TradeoffSummary>, Box<dyn std::error::Error>> {
    let mut summaries = Vec::with_capacity(60);
    for participants in PARTICIPANT_COUNTS {
        for (weapon, _) in weapon_profiles() {
            for loadout in all_canonical_loadouts() {
                let build = build_label(&loadout);
                let breach = aggregate_route(rows, participants, weapon, &build, RouteId::Breach)?;
                let stabilize =
                    aggregate_route(rows, participants, weapon, &build, RouteId::Stabilize)?;
                summaries.push(TradeoffSummary {
                    participants,
                    weapon,
                    build,
                    breach,
                    stabilize,
                    breach_dominates: route_dominates(breach, stabilize),
                    stabilize_dominates: route_dominates(stabilize, breach),
                });
            }
        }
    }
    Ok(summaries)
}

fn aggregate_route(
    rows: &[RoutedEncounterRow],
    participants: u32,
    weapon: &str,
    build: &str,
    route_id: RouteId,
) -> Result<RouteAggregate, &'static str> {
    let matching = rows
        .iter()
        .filter(|row| {
            row.participants == participants
                && row.weapon == weapon
                && row.build == build
                && row.route_id == route_id
        })
        .collect::<Vec<_>>();
    if matching.len() != 2 {
        return Err("tradeoff group does not contain exactly two route events");
    }
    Ok(RouteAggregate {
        reward: matching[0].reward,
        total_incoming_sum: checked_sum(
            matching[0].total_optimal_incoming_damage,
            matching[1].total_optimal_incoming_damage,
        )?,
        maximum_incoming: matching
            .iter()
            .map(|row| row.total_optimal_incoming_damage)
            .max()
            .ok_or("route aggregate is empty")?,
        duration_lower_bound_sum_ms: checked_sum(
            matching[0].combat_duration_lower_bound_ms,
            matching[1].combat_duration_lower_bound_ms,
        )?,
        maximum_duration_lower_bound_ms: matching
            .iter()
            .map(|row| row.combat_duration_lower_bound_ms)
            .max()
            .ok_or("route aggregate is empty")?,
    })
}

const fn route_dominates(candidate: RouteAggregate, other: RouteAggregate) -> bool {
    let never_worse = candidate.reward.fragments >= other.reward.fragments
        && candidate.reward.experience >= other.reward.experience
        && candidate.total_incoming_sum <= other.total_incoming_sum
        && candidate.maximum_incoming <= other.maximum_incoming
        && candidate.duration_lower_bound_sum_ms <= other.duration_lower_bound_sum_ms
        && candidate.maximum_duration_lower_bound_ms <= other.maximum_duration_lower_bound_ms;
    let strictly_better = candidate.reward.fragments > other.reward.fragments
        || candidate.reward.experience > other.reward.experience
        || candidate.total_incoming_sum < other.total_incoming_sum
        || candidate.maximum_incoming < other.maximum_incoming
        || candidate.duration_lower_bound_sum_ms < other.duration_lower_bound_sum_ms
        || candidate.maximum_duration_lower_bound_ms < other.maximum_duration_lower_bound_ms;
    never_worse && strictly_better
}

fn seed_vectors() -> Vec<SeedVector> {
    let boundary_seeds = [0, 1, 2, 3, MAX_ROUTE_SEED - 1, MAX_ROUTE_SEED];
    RouteId::ALL
        .into_iter()
        .flat_map(|route_id| {
            boundary_seeds.map(move |seed| {
                let resolved = resolve_event(
                    route_id,
                    RouteSeed::new(seed).expect("fixed boundary seed should validate"),
                );
                SeedVector {
                    route_id,
                    seed,
                    event_id: resolved.event_id,
                }
            })
        })
        .collect()
}

fn representative_seed(event_id: RouteEventId) -> Result<u64, &'static str> {
    (0..1_024)
        .find(|seed| {
            resolve_event(
                event_id.route(),
                RouteSeed::new(*seed).expect("bounded representative seed should validate"),
            )
            .event_id
                == event_id
        })
        .ok_or("event is unreachable in representative seed range")
}

fn build_label(modules: &[ModuleId]) -> String {
    if modules.is_empty() {
        return "empty".to_owned();
    }
    modules
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("+")
}

fn build_domain_report() -> Result<DomainReport, Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    append_selection_and_terminal_cases(&mut cases)?;
    append_authority_cases(&mut cases)?;
    append_timeout_cases(&mut cases)?;
    append_phase_and_input_cases(&mut cases)?;
    Ok(DomainReport {
        schema: "RevenantOperationDomainLabV1",
        catalog_revision: CATALOG_REVISION,
        cases,
    })
}

fn append_selection_and_terminal_cases(
    cases: &mut Vec<DomainCase>,
) -> Result<(), revenant_operations::RouteError> {
    let mut applied = ready_state(&["leader", "peer"])?;
    cases.push(select_case(
        "selection_applied",
        &mut applied,
        "leader",
        "route-1",
        RouteId::Breach,
        7,
    ));
    cases.push(select_case(
        "selection_retry_different_seed",
        &mut applied,
        "leader",
        "route-1",
        RouteId::Breach,
        8,
    ));
    cases.push(select_case(
        "selection_conflicting_route",
        &mut applied,
        "leader",
        "route-1",
        RouteId::Stabilize,
        8,
    ));
    cases.push(select_case(
        "selection_second_operation",
        &mut applied,
        "leader",
        "route-2",
        RouteId::Breach,
        8,
    ));
    cases.push(complete_case(
        "completion_at_deadline",
        &mut applied,
        ROUTE_OPERATION_DURATION_MS,
    ));
    cases.push(complete_case(
        "completion_retry",
        &mut applied,
        ROUTE_OPERATION_DURATION_MS,
    ));
    cases.push(fail_case(
        "terminal_conflict",
        &mut applied,
        ROUTE_OPERATION_DURATION_MS + 1,
    ));
    Ok(())
}

fn append_authority_cases(
    cases: &mut Vec<DomainCase>,
) -> Result<(), revenant_operations::RouteError> {
    let mut incomplete = RouteOperationState::new(&["leader", "peer"])?;
    incomplete.open_choice()?;
    cases.push(select_case(
        "capability_incomplete",
        &mut incomplete,
        "leader",
        "unreserved",
        RouteId::Stabilize,
        3,
    ));
    incomplete.mark_capable("leader")?;
    incomplete.mark_capable("peer")?;
    cases.push(select_case(
        "non_leader",
        &mut incomplete,
        "peer",
        "unreserved",
        RouteId::Stabilize,
        3,
    ));
    cases.push(select_case(
        "rejected_id_remains_unreserved",
        &mut incomplete,
        "leader",
        "unreserved",
        RouteId::Stabilize,
        3,
    ));
    Ok(())
}

fn append_timeout_cases(
    cases: &mut Vec<DomainCase>,
) -> Result<(), revenant_operations::RouteError> {
    let mut failure = ready_state(&["leader"])?;
    cases.push(select_case(
        "timeout_selection",
        &mut failure,
        "leader",
        "timeout-1",
        RouteId::Breach,
        11,
    ));
    cases.push(fail_case(
        "timeout_at_deadline_rejected",
        &mut failure,
        ROUTE_OPERATION_DURATION_MS,
    ));
    cases.push(complete_case(
        "completion_after_deadline_rejected",
        &mut failure,
        ROUTE_OPERATION_DURATION_MS + 1,
    ));
    cases.push(fail_case(
        "timeout_after_deadline",
        &mut failure,
        ROUTE_OPERATION_DURATION_MS + 1,
    ));
    Ok(())
}

fn append_phase_and_input_cases(
    cases: &mut Vec<DomainCase>,
) -> Result<(), revenant_operations::RouteError> {
    let mut baseline = ready_state(&["leader"])?;
    baseline.lock_baseline()?;
    cases.push(select_case(
        "baseline_late_choice",
        &mut baseline,
        "leader",
        "late-route",
        RouteId::Breach,
        0,
    ));

    let mut drone = RouteOperationState::new(&["leader"])?;
    drone.mark_capable("leader")?;
    cases.push(select_case(
        "wrong_phase_drone",
        &mut drone,
        "leader",
        "early-route",
        RouteId::Breach,
        0,
    ));
    let mut unknown = ready_state(&["leader"])?;
    cases.push(select_case(
        "unknown_participant",
        &mut unknown,
        "intruder",
        "route-unknown",
        RouteId::Breach,
        0,
    ));
    cases.push(select_case(
        "invalid_operation_id",
        &mut unknown,
        "leader",
        "invalid_id",
        RouteId::Breach,
        0,
    ));
    Ok(())
}

fn ready_state(
    participant_ids: &[&str],
) -> Result<RouteOperationState, revenant_operations::RouteError> {
    let mut state = RouteOperationState::new(participant_ids)?;
    for participant_id in participant_ids {
        state.mark_capable(participant_id)?;
    }
    state.open_choice()?;
    Ok(state)
}

fn select_case(
    case: &'static str,
    state: &mut RouteOperationState,
    requester_id: &str,
    operation_id: &str,
    route_id: RouteId,
    seed: u64,
) -> DomainCase {
    let before = state.clone();
    match state.select(
        requester_id,
        RouteSelectionRequest {
            operation_id,
            route_id,
        },
        RouteSeed::new(seed).expect("domain case seed should validate"),
    ) {
        Ok(receipt) => accepted_case(case, receipt.disposition, state),
        Err(error) => rejected_case(case, error.to_string(), *state == before, state),
    }
}

fn complete_case(
    case: &'static str,
    state: &mut RouteOperationState,
    elapsed_ms: u64,
) -> DomainCase {
    let before = state.clone();
    match state.complete(elapsed_ms) {
        Ok(receipt) => accepted_case(case, receipt.disposition, state),
        Err(error) => rejected_case(case, error.to_string(), *state == before, state),
    }
}

fn fail_case(case: &'static str, state: &mut RouteOperationState, elapsed_ms: u64) -> DomainCase {
    let before = state.clone();
    match state.fail_timeout(elapsed_ms) {
        Ok(receipt) => accepted_case(case, receipt.disposition, state),
        Err(error) => rejected_case(case, error.to_string(), *state == before, state),
    }
}

fn accepted_case(
    case: &'static str,
    disposition: OperationDisposition,
    state: &RouteOperationState,
) -> DomainCase {
    DomainCase {
        case,
        accepted: true,
        disposition: Some(disposition),
        error: None,
        unchanged_on_error: None,
        state: domain_state_view(state),
    }
}

fn rejected_case(
    case: &'static str,
    error: String,
    unchanged: bool,
    state: &RouteOperationState,
) -> DomainCase {
    DomainCase {
        case,
        accepted: false,
        disposition: None,
        error: Some(error),
        unchanged_on_error: Some(unchanged),
        state: domain_state_view(state),
    }
}

fn domain_state_view(state: &RouteOperationState) -> DomainStateView {
    DomainStateView {
        phase: state.phase(),
        selected_operation: state
            .selection()
            .map(|selection| selection.operation_id.clone()),
        route_id: state.selection().map(|selection| selection.route_id),
        event_id: state.selection().map(|selection| selection.event_id),
        terminal_outcome: state.terminal().map(|summary| summary.outcome),
        terminal_reward: state.terminal().and_then(|summary| summary.reward),
    }
}

fn print_table(report: &LabReport) {
    println!(
        "{} {} {} routed={} baseline={} tradeoffs={}",
        report.schema,
        report.catalog_revision,
        report.resolver_revision,
        report.routed_rows.len(),
        report.baseline_rows.len(),
        report.tradeoffs.len()
    );
    println!("route event players weapon build hp hits ttk_ms incoming fragments xp objectives bounded nonlethal");
    for row in &report.routed_rows {
        println!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            row.route_id,
            row.event_id,
            row.participants,
            row.weapon,
            row.build,
            row.warden.target_health,
            row.warden.required_hits,
            row.combat_duration_lower_bound_ms,
            row.total_optimal_incoming_damage,
            row.reward.fragments,
            row.reward.experience,
            row.objective_count,
            row.within_duration_budget,
            row.nonlethal_optimal_pressure
        );
    }
    let dominated = report
        .tradeoffs
        .iter()
        .filter(|summary| summary.breach_dominates || summary.stabilize_dominates)
        .count();
    println!("route_dominance_failures {dominated}");
}

#[cfg(test)]
mod tests {
    use super::{build_domain_report, build_report};
    use revenant_operations::{RouteEventId, RouteId, MAX_OPTIMAL_INCOMING_DAMAGE};

    #[test]
    fn report_covers_exact_routed_and_baseline_state_spaces() {
        let report = build_report().expect("operation report should build");
        assert_eq!(report.routed_rows.len(), 240);
        assert_eq!(report.baseline_rows.len(), 60);
        assert_eq!(report.tradeoffs.len(), 60);
        for route_id in RouteId::ALL {
            assert_eq!(
                report
                    .routed_rows
                    .iter()
                    .filter(|row| row.route_id == route_id)
                    .count(),
                120
            );
        }
        for event_id in RouteEventId::ALL {
            assert_eq!(
                report
                    .routed_rows
                    .iter()
                    .filter(|row| row.event_id == event_id)
                    .count(),
                60
            );
        }
    }

    #[test]
    fn every_route_row_respects_pressure_duration_and_reward_bounds() {
        let report = build_report().expect("operation report should build");
        for row in &report.routed_rows {
            assert!(row.within_duration_budget);
            assert!(row.nonlethal_optimal_pressure);
            assert!(row.total_optimal_incoming_damage <= u64::from(MAX_OPTIMAL_INCOMING_DAMAGE));
            assert!((1..=2).contains(&row.reward.fragments));
            assert!([100, 125, 150, 175].contains(&row.reward.experience));
            assert!((3..=4).contains(&row.objective_count));
        }
    }

    #[test]
    fn baseline_rows_preserve_m26_combat_reward_and_objective_contract() {
        let report = build_report().expect("operation report should build");
        for row in &report.baseline_rows {
            assert_eq!(row.reward.fragments, 1);
            assert_eq!(row.reward.experience, 100);
            assert_eq!(row.objective_count, 3);
            assert_eq!(row.total_optimal_incoming_damage, 40);
            assert!(row.nonlethal_optimal_pressure);
            let expected_drone_health = if row.participants == 1 { 140 } else { 210 };
            let expected_warden_health = if row.participants == 1 { 240 } else { 360 };
            assert_eq!(row.drone.target_health, expected_drone_health);
            assert_eq!(row.warden.target_health, expected_warden_health);
        }
    }

    #[test]
    fn route_pair_is_non_dominated_for_every_build_and_participant_count() {
        let report = build_report().expect("operation report should build");
        for summary in &report.tradeoffs {
            assert!(!summary.breach_dominates);
            assert!(!summary.stabilize_dominates);
            assert!(summary.breach.reward.fragments > summary.stabilize.reward.fragments);
            assert!(summary.stabilize.reward.experience > summary.breach.reward.experience);
            assert!(summary.stabilize.total_incoming_sum < summary.breach.total_incoming_sum);
            assert!(summary.stabilize.maximum_incoming < summary.breach.maximum_incoming);
        }
    }

    #[test]
    fn all_events_have_stable_representative_and_boundary_vectors() {
        let report = build_report().expect("operation report should build");
        assert_eq!(report.seed_vectors.len(), 12);
        for event_id in RouteEventId::ALL {
            let seeds = report
                .routed_rows
                .iter()
                .filter(|row| row.event_id == event_id)
                .map(|row| row.representative_seed)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(seeds.len(), 1);
        }
    }

    #[test]
    fn report_serialization_is_deterministic() {
        let first = serde_json::to_string(&build_report().expect("first report should build"))
            .expect("first report should serialize");
        let second = serde_json::to_string(&build_report().expect("second report should build"))
            .expect("second report should serialize");
        assert_eq!(first, second);
    }

    #[test]
    fn domain_report_covers_authority_retry_conflict_deadline_and_baseline() {
        let report = build_domain_report().expect("domain report should build");
        assert_eq!(report.cases.len(), 18);
        assert!(report
            .cases
            .iter()
            .filter(|case| !case.accepted)
            .all(|case| case.unchanged_on_error == Some(true)));
        let retry = report
            .cases
            .iter()
            .find(|case| case.case == "selection_retry_different_seed")
            .expect("retry case should exist");
        assert_eq!(
            retry.disposition,
            Some(revenant_operations::OperationDisposition::Replayed)
        );
        let failure = report
            .cases
            .iter()
            .find(|case| case.case == "timeout_after_deadline")
            .expect("timeout case should exist");
        assert_eq!(
            failure.state.terminal_outcome,
            Some(revenant_operations::TerminalOutcome::FailedTimeout)
        );
        assert_eq!(failure.state.terminal_reward, None);
    }

    #[test]
    fn domain_report_serialization_is_deterministic() {
        let first = serde_json::to_string(
            &build_domain_report().expect("first domain report should build"),
        )
        .expect("first domain report should serialize");
        let second = serde_json::to_string(
            &build_domain_report().expect("second domain report should build"),
        )
        .expect("second domain report should serialize");
        assert_eq!(first, second);
    }
}
