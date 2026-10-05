use std::env;
use std::process::ExitCode;

use revenant_ai::{RELAY_DRONE_PRESSURE_PROFILE, WARDEN_PRESSURE_PROFILE};
use revenant_combat::{analyze_scenario, AttackProfile, CombatMetrics, CombatScenario};
use revenant_inventory::{ARC_SIDEARM_PROFILE, PULSE_RIFLE_PROFILE};
use revenant_modules::{
    all_canonical_loadouts, dominates, resolve_build, ActivityPhase, BaseCombatProfile,
    CombinationRequest, EffectiveCombatProfile, LoadoutRequest, ModuleDefinition, ModuleId,
    ModuleMutationError, ModuleState, CATALOG_REVISION, MAX_OPERATION_RECORDS_PER_KIND,
    MODULE_CATALOG,
};
use serde::Serialize;

const OPERATOR_BASE_HEALTH: u32 = 100;
const DRONE_HEALTH: u32 = 140;
const WARDEN_HEALTH: u32 = 240;
const TARGET_DISTANCE_SQUARED: i64 = 4;

#[derive(Debug, Serialize)]
struct LabReport {
    schema: &'static str,
    catalog_revision: &'static str,
    catalog: Vec<ModuleDefinition>,
    builds: Vec<BuildRow>,
    frontiers: Vec<FrontierSummary>,
    recipe_cases: Vec<RecipeCase>,
}

#[derive(Debug, Serialize)]
struct BuildRow {
    weapon: &'static str,
    build: String,
    modules: Vec<ModuleId>,
    damage: u32,
    range: i32,
    cooldown_ms: u64,
    max_health: u32,
    drone: EncounterRow,
    warden: EncounterRow,
    total_optimal_incoming_damage: u64,
    nonlethal_optimal_pressure: bool,
    pareto_non_dominated: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct EncounterRow {
    target_health: u32,
    required_hits: u32,
    theoretical_ttk_ms: u64,
    overkill_damage: u64,
    expected_hostile_attacks: u32,
    expected_incoming_damage: u64,
}

#[derive(Debug, Serialize)]
struct FrontierSummary {
    weapon: &'static str,
    count: usize,
    builds: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RecipeCase {
    module: ModuleId,
    case: &'static str,
    fragment_quantity: u32,
    owns_module: bool,
    pure_result: &'static str,
}

#[derive(Debug, Serialize)]
struct DomainReport {
    schema: &'static str,
    catalog_revision: &'static str,
    cases: Vec<DomainCase>,
}

#[derive(Debug, Serialize)]
struct DomainCase {
    case: String,
    accepted: bool,
    disposition: Option<String>,
    error: Option<String>,
    unchanged_on_error: Option<bool>,
    state: DomainStateView,
}

#[derive(Debug, Serialize)]
struct DomainStateView {
    fragments: u32,
    owned_modules: Vec<ModuleId>,
    loadout: Vec<ModuleId>,
    revision: u64,
    combination_operations: usize,
    loadout_operations: usize,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("module lab failed: {error}");
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
        return Err("usage: revenant-module-lab [--json|--domain-json]".into());
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

fn build_domain_report() -> Result<DomainReport, Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    append_combination_cases(&mut cases)?;
    append_valid_loadout_cases(&mut cases)?;
    append_idempotency_cases(&mut cases)?;
    append_invalid_loadout_cases(&mut cases)?;
    append_loadout_bound_cases(&mut cases)?;
    Ok(DomainReport {
        schema: "RevenantModuleDomainLabV1",
        catalog_revision: CATALOG_REVISION,
        cases,
    })
}

fn append_combination_cases(cases: &mut Vec<DomainCase>) -> Result<(), ModuleMutationError> {
    let mut combination = ModuleState::new(4, &[], &[], 0)?;
    cases.push(run_combination_case(
        "combine_exact",
        &mut combination,
        ActivityPhase::Complete,
        "combine-1",
        ModuleId::ForceMatrix,
    ));
    cases.push(run_combination_case(
        "combine_same_operation_retry",
        &mut combination,
        ActivityPhase::Active,
        "combine-1",
        ModuleId::ForceMatrix,
    ));
    cases.push(run_combination_case(
        "combine_conflicting_operation",
        &mut combination,
        ActivityPhase::Complete,
        "combine-1",
        ModuleId::TempoRegulator,
    ));

    for (name, mut state, phase, operation_id, module_id) in [
        (
            "combine_insufficient",
            ModuleState::new(1, &[], &[], 0)?,
            ActivityPhase::Complete,
            "insufficient",
            ModuleId::TempoRegulator,
        ),
        (
            "combine_already_owned",
            ModuleState::new(2, &[ModuleId::ForceMatrix], &[], 0)?,
            ActivityPhase::Complete,
            "already-owned",
            ModuleId::ForceMatrix,
        ),
        (
            "combine_active_locked",
            ModuleState::new(2, &[], &[], 0)?,
            ActivityPhase::Active,
            "active-locked",
            ModuleId::ForceMatrix,
        ),
        (
            "combine_invalid_operation",
            ModuleState::new(2, &[], &[], 0)?,
            ActivityPhase::Complete,
            "invalid_id",
            ModuleId::ForceMatrix,
        ),
    ] {
        cases.push(run_combination_case(
            name,
            &mut state,
            phase,
            operation_id,
            module_id,
        ));
    }
    Ok(())
}

fn append_valid_loadout_cases(cases: &mut Vec<DomainCase>) -> Result<(), ModuleMutationError> {
    for (index, loadout) in all_canonical_loadouts().into_iter().enumerate() {
        let initial = if loadout.is_empty() {
            vec![ModuleId::ForceMatrix]
        } else {
            Vec::new()
        };
        let mut state = ModuleState::new(0, &ModuleId::LEGACY, &initial, 0)?;
        cases.push(run_loadout_case(
            &format!("loadout_valid_{index:02}"),
            &mut state,
            ActivityPhase::Complete,
            &format!("matrix-{index}"),
            0,
            &loadout,
        ));
    }
    Ok(())
}

fn append_idempotency_cases(cases: &mut Vec<DomainCase>) -> Result<(), ModuleMutationError> {
    let first_loadout = [
        ModuleId::ReachLattice,
        ModuleId::ForceMatrix,
        ModuleId::TempoRegulator,
    ];
    let retry_loadout = [
        ModuleId::TempoRegulator,
        ModuleId::ReachLattice,
        ModuleId::ForceMatrix,
    ];
    let conflicting_loadout = [ModuleId::WardCapacitor];
    let mut idempotent = ModuleState::new(0, &ModuleId::LEGACY, &[], 0)?;
    cases.push(run_loadout_case(
        "loadout_noncanonical_applied",
        &mut idempotent,
        ActivityPhase::Complete,
        "loadout-retry",
        0,
        &first_loadout,
    ));
    cases.push(run_loadout_case(
        "loadout_canonical_retry",
        &mut idempotent,
        ActivityPhase::Active,
        "loadout-retry",
        0,
        &retry_loadout,
    ));
    cases.push(run_loadout_case(
        "loadout_conflicting_operation",
        &mut idempotent,
        ActivityPhase::Complete,
        "loadout-retry",
        0,
        &conflicting_loadout,
    ));
    Ok(())
}

fn append_invalid_loadout_cases(cases: &mut Vec<DomainCase>) -> Result<(), ModuleMutationError> {
    let force = [ModuleId::ForceMatrix];
    let duplicate = [ModuleId::ForceMatrix, ModuleId::ForceMatrix];
    let all = ModuleId::LEGACY;
    for (name, mut state, phase, operation_id, expected_revision, modules) in [
        (
            "loadout_active_locked",
            ModuleState::new(0, &[ModuleId::ForceMatrix], &[], 4)?,
            ActivityPhase::Active,
            "active-loadout",
            4,
            force.as_slice(),
        ),
        (
            "loadout_stale_revision",
            ModuleState::new(0, &[ModuleId::ForceMatrix], &[], 4)?,
            ActivityPhase::Complete,
            "stale-loadout",
            3,
            force.as_slice(),
        ),
        (
            "loadout_unowned",
            ModuleState::new(0, &[], &[], 0)?,
            ActivityPhase::Complete,
            "unowned-loadout",
            0,
            force.as_slice(),
        ),
        (
            "loadout_duplicate",
            ModuleState::new(0, &[ModuleId::ForceMatrix], &[], 0)?,
            ActivityPhase::Complete,
            "duplicate-loadout",
            0,
            duplicate.as_slice(),
        ),
        (
            "loadout_over_capacity",
            ModuleState::new(0, &ModuleId::LEGACY, &[], 0)?,
            ActivityPhase::Complete,
            "capacity-loadout",
            0,
            all.as_slice(),
        ),
        (
            "loadout_invalid_operation",
            ModuleState::new(0, &[ModuleId::ForceMatrix], &[], 0)?,
            ActivityPhase::Complete,
            "invalid_id",
            0,
            force.as_slice(),
        ),
    ] {
        cases.push(run_loadout_case(
            name,
            &mut state,
            phase,
            operation_id,
            expected_revision,
            modules,
        ));
    }
    Ok(())
}

fn append_loadout_bound_cases(cases: &mut Vec<DomainCase>) -> Result<(), ModuleMutationError> {
    let force = [ModuleId::ForceMatrix];
    let mut unchanged = ModuleState::new(0, &[ModuleId::ForceMatrix], &force, 5)?;
    cases.push(run_loadout_case(
        "loadout_unchanged",
        &mut unchanged,
        ActivityPhase::Complete,
        "unchanged",
        5,
        &force,
    ));

    let mut overflow = ModuleState::new(0, &[ModuleId::ForceMatrix], &[], u64::MAX)?;
    cases.push(run_loadout_case(
        "loadout_revision_overflow",
        &mut overflow,
        ActivityPhase::Complete,
        "overflow",
        u64::MAX,
        &force,
    ));

    let mut bounded = ModuleState::new(0, &[ModuleId::ForceMatrix], &[], 0)?;
    for operation in 0..MAX_OPERATION_RECORDS_PER_KIND {
        let modules = if operation % 2 == 0 {
            force.to_vec()
        } else {
            Vec::new()
        };
        bounded.set_loadout(
            ActivityPhase::Complete,
            LoadoutRequest {
                operation_id: &format!("bounded-{operation}"),
                expected_revision: bounded.revision(),
                modules: &modules,
            },
        )?;
    }
    let limit_modules = if bounded.loadout().is_empty() {
        force.to_vec()
    } else {
        Vec::new()
    };
    let limit_revision = bounded.revision();
    cases.push(run_loadout_case(
        "loadout_operation_limit",
        &mut bounded,
        ActivityPhase::Complete,
        "bounded-limit",
        limit_revision,
        &limit_modules,
    ));
    Ok(())
}

fn run_combination_case(
    case: &str,
    state: &mut ModuleState,
    phase: ActivityPhase,
    operation_id: &str,
    module_id: ModuleId,
) -> DomainCase {
    let before = state.clone();
    match state.combine(
        phase,
        CombinationRequest {
            operation_id,
            module_id,
        },
    ) {
        Ok(receipt) => DomainCase {
            case: case.to_owned(),
            accepted: true,
            disposition: Some(format!("{:?}", receipt.disposition).to_lowercase()),
            error: None,
            unchanged_on_error: None,
            state: domain_state_view(state),
        },
        Err(error) => DomainCase {
            case: case.to_owned(),
            accepted: false,
            disposition: None,
            error: Some(error.to_string()),
            unchanged_on_error: Some(*state == before),
            state: domain_state_view(state),
        },
    }
}

fn run_loadout_case(
    case: &str,
    state: &mut ModuleState,
    phase: ActivityPhase,
    operation_id: &str,
    expected_revision: u64,
    modules: &[ModuleId],
) -> DomainCase {
    let before = state.clone();
    match state.set_loadout(
        phase,
        LoadoutRequest {
            operation_id,
            expected_revision,
            modules,
        },
    ) {
        Ok(receipt) => DomainCase {
            case: case.to_owned(),
            accepted: true,
            disposition: Some(format!("{:?}", receipt.disposition).to_lowercase()),
            error: None,
            unchanged_on_error: None,
            state: domain_state_view(state),
        },
        Err(error) => DomainCase {
            case: case.to_owned(),
            accepted: false,
            disposition: None,
            error: Some(error.to_string()),
            unchanged_on_error: Some(*state == before),
            state: domain_state_view(state),
        },
    }
}

fn domain_state_view(state: &ModuleState) -> DomainStateView {
    DomainStateView {
        fragments: state.fragments(),
        owned_modules: state.owned_modules(),
        loadout: state.loadout().to_vec(),
        revision: state.revision(),
        combination_operations: state.combination_operation_count(),
        loadout_operations: state.loadout_operation_count(),
    }
}

fn build_report() -> Result<LabReport, Box<dyn std::error::Error>> {
    let weapons = [
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
    ];
    let mut builds = Vec::with_capacity(30);
    for (weapon, base) in weapons {
        for loadout in all_canonical_loadouts() {
            let resolved = resolve_build(CATALOG_REVISION, base, &loadout)?;
            let drone = encounter(resolved.profile, DRONE_HEALTH, RELAY_DRONE_PRESSURE_PROFILE)?;
            let warden = encounter(resolved.profile, WARDEN_HEALTH, WARDEN_PRESSURE_PROFILE)?;
            let incoming = drone
                .expected_incoming_damage
                .checked_add(warden.expected_incoming_damage)
                .ok_or("incoming damage overflowed")?;
            builds.push(BuildRow {
                weapon,
                build: build_label(&resolved.modules),
                modules: resolved.modules,
                damage: resolved.profile.damage,
                range: resolved.profile.range,
                cooldown_ms: resolved.profile.cooldown_ms,
                max_health: resolved.profile.max_health,
                drone,
                warden,
                total_optimal_incoming_damage: incoming,
                nonlethal_optimal_pressure: incoming < u64::from(resolved.profile.max_health),
                pareto_non_dominated: false,
            });
        }
    }
    let frontier_flags = builds
        .iter()
        .map(|candidate| {
            !builds.iter().any(|other| {
                other.weapon == candidate.weapon
                    && other.build != candidate.build
                    && dominates(row_profile(other), row_profile(candidate))
            })
        })
        .collect::<Vec<_>>();
    for (build, frontier) in builds.iter_mut().zip(frontier_flags) {
        build.pareto_non_dominated = frontier;
    }
    let frontiers = weapons
        .into_iter()
        .map(|(weapon, _)| {
            let names = builds
                .iter()
                .filter(|row| row.weapon == weapon && row.pareto_non_dominated)
                .map(|row| row.build.clone())
                .collect::<Vec<_>>();
            FrontierSummary {
                weapon,
                count: names.len(),
                builds: names,
            }
        })
        .collect();
    Ok(LabReport {
        schema: "RevenantModuleLabV1",
        catalog_revision: CATALOG_REVISION,
        catalog: MODULE_CATALOG.to_vec(),
        builds,
        frontiers,
        recipe_cases: recipe_cases(),
    })
}

fn encounter(
    profile: EffectiveCombatProfile,
    target_health: u32,
    pressure: revenant_combat::PressureProfile,
) -> Result<EncounterRow, revenant_combat::CombatAnalysisError> {
    let metrics = analyze_scenario(CombatScenario {
        attack: AttackProfile {
            damage: profile.damage,
            range: profile.range,
            cooldown_ms: profile.cooldown_ms,
        },
        target_health,
        target_distance_squared: TARGET_DISTANCE_SQUARED,
        attacker_count: 1,
        client_attack_interval_ms: profile.cooldown_ms,
        round_trip_ms: 0,
        pressure,
    })?;
    Ok(encounter_row(target_health, metrics))
}

const fn encounter_row(target_health: u32, metrics: CombatMetrics) -> EncounterRow {
    EncounterRow {
        target_health,
        required_hits: metrics.required_hits,
        theoretical_ttk_ms: metrics.theoretical_ttk_ms,
        overkill_damage: metrics.overkill_damage,
        expected_hostile_attacks: metrics.expected_hostile_attacks,
        expected_incoming_damage: metrics.expected_incoming_damage,
    }
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

const fn row_profile(row: &BuildRow) -> EffectiveCombatProfile {
    EffectiveCombatProfile {
        damage: row.damage,
        range: row.range,
        cooldown_ms: row.cooldown_ms,
        max_health: row.max_health,
    }
}

fn recipe_cases() -> Vec<RecipeCase> {
    MODULE_CATALOG
        .iter()
        .flat_map(|definition| {
            let cost = definition.recipe_fragments;
            [
                recipe_case(
                    definition.module_id,
                    "zero",
                    0,
                    false,
                    "insufficient_fragments",
                ),
                recipe_case(
                    definition.module_id,
                    "one_below",
                    cost.saturating_sub(1),
                    false,
                    "insufficient_fragments",
                ),
                recipe_case(definition.module_id, "exact", cost, false, "eligible"),
                recipe_case(
                    definition.module_id,
                    "one_above",
                    cost.saturating_add(1),
                    false,
                    "eligible",
                ),
                recipe_case(
                    definition.module_id,
                    "already_owned",
                    cost,
                    true,
                    "already_owned",
                ),
                recipe_case(
                    definition.module_id,
                    "same_operation_retry",
                    cost,
                    false,
                    "operation_semantics_gate_3",
                ),
                recipe_case(
                    definition.module_id,
                    "conflicting_operation",
                    cost,
                    false,
                    "operation_semantics_gate_3",
                ),
            ]
        })
        .collect()
}

const fn recipe_case(
    module: ModuleId,
    case: &'static str,
    fragment_quantity: u32,
    owns_module: bool,
    pure_result: &'static str,
) -> RecipeCase {
    RecipeCase {
        module,
        case,
        fragment_quantity,
        owns_module,
        pure_result,
    }
}

fn print_table(report: &LabReport) {
    println!("{} {}", report.schema, report.catalog_revision);
    println!("weapon build dmg range cooldown_ms hp drone_hits drone_ttk warden_hits warden_ttk incoming pareto");
    for row in &report.builds {
        println!(
            "{} {} {} {} {} {} {} {} {} {} {} {}",
            row.weapon,
            row.build,
            row.damage,
            row.range,
            row.cooldown_ms,
            row.max_health,
            row.drone.required_hits,
            row.drone.theoretical_ttk_ms,
            row.warden.required_hits,
            row.warden.theoretical_ttk_ms,
            row.total_optimal_incoming_damage,
            row.pareto_non_dominated,
        );
    }
    for frontier in &report.frontiers {
        println!(
            "frontier {} {} {}",
            frontier.weapon,
            frontier.count,
            frontier.builds.join(",")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{build_domain_report, build_report};

    #[test]
    fn report_covers_every_build_recipe_and_exact_candidate() {
        let report = build_report().expect("module report should build");
        assert_eq!(report.builds.len(), 30);
        assert_eq!(report.recipe_cases.len(), 28);
        assert!(report.frontiers.iter().all(|frontier| frontier.count >= 3));
        let force_rifle = report
            .builds
            .iter()
            .find(|row| row.weapon == "pulse_rifle" && row.build == "module_force_matrix")
            .expect("force rifle row should exist");
        assert_eq!(
            (
                force_rifle.damage,
                force_rifle.cooldown_ms,
                force_rifle.drone.theoretical_ttk_ms,
                force_rifle.warden.theoretical_ttk_ms,
            ),
            (48, 300, 600, 1_200)
        );
        assert!(report
            .catalog
            .iter()
            .all(|definition| definition.recipe_fragments == 2));
        let encoded = serde_json::to_value(&report.catalog).expect("catalog should serialize");
        assert_eq!(encoded[0]["module_id"], "module_force_matrix");
    }

    #[test]
    fn every_build_meets_encounter_pressure_and_dominance_bounds() {
        let report = build_report().expect("module report should build");
        for row in &report.builds {
            assert!((600..=1_200).contains(&row.drone.theoretical_ttk_ms));
            assert!((1_200..=2_600).contains(&row.warden.theoretical_ttk_ms));
            assert_eq!(row.total_optimal_incoming_damage, 40);
            assert!(row.nonlethal_optimal_pressure);
        }
        for weapon in ["pulse_rifle", "arc_sidearm"] {
            let rows = report
                .builds
                .iter()
                .filter(|row| row.weapon == weapon)
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 15);
            assert!(!rows.iter().any(|candidate| {
                rows.iter().all(|other| {
                    candidate.build == other.build
                        || revenant_modules::dominates(
                            super::row_profile(candidate),
                            super::row_profile(other),
                        )
                })
            }));
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
    fn domain_report_covers_valid_invalid_retry_conflict_and_bounds() {
        let report = build_domain_report().expect("domain report should build");
        assert_eq!(report.cases.len(), 34);
        assert!(report
            .cases
            .iter()
            .filter(|case| !case.accepted)
            .all(|case| case.unchanged_on_error == Some(true)));
        assert_eq!(
            report
                .cases
                .iter()
                .filter(|case| case.case.starts_with("loadout_valid_") && case.accepted)
                .count(),
            15
        );
        let combine = report
            .cases
            .iter()
            .find(|case| case.case == "combine_exact")
            .expect("combination case should exist");
        assert_eq!(combine.state.fragments, 2);
        assert_eq!(
            combine.state.owned_modules,
            vec![revenant_modules::ModuleId::ForceMatrix]
        );
        let retry = report
            .cases
            .iter()
            .find(|case| case.case == "loadout_canonical_retry")
            .expect("loadout retry should exist");
        assert_eq!(retry.disposition.as_deref(), Some("replayed"));
        let limit = report
            .cases
            .iter()
            .find(|case| case.case == "loadout_operation_limit")
            .expect("operation limit should exist");
        assert!(!limit.accepted);
        assert_eq!(limit.state.loadout_operations, 128);
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
