use std::env;
use std::process::ExitCode;

use revenant_ai::{RELAY_DRONE_PRESSURE_PROFILE, WARDEN_PRESSURE_PROFILE};
use revenant_combat::{
    analyze_scenario, AttackProfile, CombatMetrics, CombatScenario, PressureProfile,
};
use revenant_inventory::{ARC_SIDEARM_PROFILE, PULSE_RIFLE_PROFILE};
use serde::Serialize;

const ROUND_TRIP_CASES_MS: [u64; 3] = [0, 75, 150];
const ATTACKER_CASES: [u32; 2] = [1, 2];
const BASELINE_CLIENT_ATTACK_INTERVAL_MS: u64 = 260;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LabMode {
    Baseline,
    Candidate,
}

impl LabMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Baseline => "m25_pre_tuning",
            Self::Candidate => "m25_block2_candidate",
        }
    }
}

#[derive(Clone, Copy)]
struct EnemyBaseline {
    name: &'static str,
    health: u32,
    candidate_health: u32,
    distance_squared: i64,
    pressure: PressureProfile,
}

const ENEMIES: [EnemyBaseline; 2] = [
    EnemyBaseline {
        name: "relay_drone",
        health: 100,
        candidate_health: 140,
        distance_squared: 4,
        pressure: RELAY_DRONE_PRESSURE_PROFILE,
    },
    EnemyBaseline {
        name: "warden",
        health: 120,
        candidate_health: 240,
        distance_squared: 4,
        pressure: WARDEN_PRESSURE_PROFILE,
    },
];

#[derive(Debug, Serialize)]
struct LabReport {
    schema: &'static str,
    baseline: &'static str,
    rows: Vec<LabRow>,
}

#[derive(Debug, Serialize)]
struct LabRow {
    weapon: &'static str,
    damage: u32,
    range: i32,
    cooldown_ms: u64,
    enemy: &'static str,
    target_health: u32,
    target_distance_squared: i64,
    attackers: u32,
    client_attack_interval_ms: u64,
    round_trip_ms: u64,
    required_hits: u32,
    required_volleys: u32,
    theoretical_ttk_ms: u64,
    effective_ttk_ms: u64,
    overkill_damage: u64,
    damage_per_second_milli: u64,
    cooldown_duty_basis_points: u64,
    range_margin_squared: i64,
    in_range: bool,
    expected_hostile_attacks: u32,
    expected_incoming_damage: u64,
}

impl LabRow {
    fn new(
        weapon: &'static str,
        attack: AttackProfile,
        enemy: EnemyBaseline,
        attackers: u32,
        round_trip_ms: u64,
        metrics: CombatMetrics,
    ) -> Self {
        Self {
            weapon,
            damage: attack.damage,
            range: attack.range,
            cooldown_ms: attack.cooldown_ms,
            enemy: enemy.name,
            target_health: enemy.health,
            target_distance_squared: enemy.distance_squared,
            attackers,
            client_attack_interval_ms: BASELINE_CLIENT_ATTACK_INTERVAL_MS,
            round_trip_ms,
            required_hits: metrics.required_hits,
            required_volleys: metrics.required_volleys,
            theoretical_ttk_ms: metrics.theoretical_ttk_ms,
            effective_ttk_ms: metrics.effective_ttk_ms,
            overkill_damage: metrics.overkill_damage,
            damage_per_second_milli: metrics.damage_per_second_milli,
            cooldown_duty_basis_points: metrics.cooldown_duty_basis_points,
            range_margin_squared: metrics.range_margin_squared,
            in_range: metrics.in_range,
            expected_hostile_attacks: metrics.expected_hostile_attacks,
            expected_incoming_damage: metrics.expected_incoming_damage,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("combat lab failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() > 2
        || arguments
            .iter()
            .any(|value| value != "--json" && value != "--candidate")
        || arguments.iter().filter(|value| *value == "--json").count() > 1
        || arguments
            .iter()
            .filter(|value| *value == "--candidate")
            .count()
            > 1
    {
        return Err("usage: revenant-combat-lab [--candidate] [--json]".into());
    }
    let mode = if arguments.iter().any(|value| value == "--candidate") {
        LabMode::Candidate
    } else {
        LabMode::Baseline
    };
    let report = build_report(mode)?;
    if arguments.iter().any(|value| value == "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_table(&report);
    }
    Ok(())
}

fn build_report(mode: LabMode) -> Result<LabReport, Box<dyn std::error::Error>> {
    let weapons = [
        (
            PULSE_RIFLE_PROFILE.item_id,
            AttackProfile {
                damage: PULSE_RIFLE_PROFILE.damage,
                range: PULSE_RIFLE_PROFILE.range,
                cooldown_ms: PULSE_RIFLE_PROFILE.cooldown_ms,
            },
        ),
        (
            ARC_SIDEARM_PROFILE.item_id,
            AttackProfile {
                damage: ARC_SIDEARM_PROFILE.damage,
                range: ARC_SIDEARM_PROFILE.range,
                cooldown_ms: ARC_SIDEARM_PROFILE.cooldown_ms,
            },
        ),
    ];
    let mut rows = Vec::new();
    for (weapon, attack) in weapons {
        for enemy in ENEMIES {
            for attackers in ATTACKER_CASES {
                for round_trip_ms in ROUND_TRIP_CASES_MS {
                    let target_health = match mode {
                        LabMode::Baseline => enemy.health,
                        LabMode::Candidate => enemy.candidate_health * (attackers + 1) / 2,
                    };
                    let client_attack_interval_ms = match mode {
                        LabMode::Baseline => BASELINE_CLIENT_ATTACK_INTERVAL_MS,
                        LabMode::Candidate => attack.cooldown_ms,
                    };
                    let metrics = analyze_scenario(CombatScenario {
                        attack,
                        target_health,
                        target_distance_squared: enemy.distance_squared,
                        attacker_count: attackers,
                        client_attack_interval_ms,
                        round_trip_ms,
                        pressure: if mode == LabMode::Baseline && enemy.name == "warden" {
                            PressureProfile::NONE
                        } else {
                            enemy.pressure
                        },
                    })?;
                    rows.push(LabRow::new(
                        weapon,
                        attack,
                        EnemyBaseline {
                            health: target_health,
                            ..enemy
                        },
                        attackers,
                        round_trip_ms,
                        metrics,
                    ));
                }
            }
        }
    }
    Ok(LabReport {
        schema: "RevenantCombatLabV1",
        baseline: mode.label(),
        rows,
    })
}

fn print_table(report: &LabReport) {
    println!("{} {}", report.schema, report.baseline);
    println!(
        "weapon enemy players rtt_ms hits volleys ttk_ms adjusted_ms dps overkill range_margin hostile_hits incoming"
    );
    for row in &report.rows {
        let damage_per_second = row.damage_per_second_milli / 1_000;
        let damage_per_second_fraction = row.damage_per_second_milli % 1_000;
        println!(
            "{} {} {} {} {} {} {} {} {}.{:03} {} {} {} {}",
            row.weapon,
            row.enemy,
            row.attackers,
            row.round_trip_ms,
            row.required_hits,
            row.required_volleys,
            row.theoretical_ttk_ms,
            row.effective_ttk_ms,
            damage_per_second,
            damage_per_second_fraction,
            row.overkill_damage,
            row.range_margin_squared,
            row.expected_hostile_attacks,
            row.expected_incoming_damage,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{build_report, LabMode};

    #[test]
    fn baseline_matrix_is_complete_and_exact() {
        let report = build_report(LabMode::Baseline).expect("baseline report should build");
        assert_eq!(report.rows.len(), 24);
        let sidearm_drone = report
            .rows
            .iter()
            .find(|row| {
                row.weapon == "arc_sidearm"
                    && row.enemy == "relay_drone"
                    && row.attackers == 1
                    && row.round_trip_ms == 0
            })
            .expect("sidearm/drone baseline row should exist");
        assert_eq!(sidearm_drone.required_hits, 4);
        assert_eq!(sidearm_drone.theoretical_ttk_ms, 450);
        assert_eq!(sidearm_drone.effective_ttk_ms, 780);
        assert_eq!(sidearm_drone.range_margin_squared, 60);
        assert_eq!(sidearm_drone.expected_incoming_damage, 10);

        let rifle_warden = report
            .rows
            .iter()
            .find(|row| {
                row.weapon == "pulse_rifle"
                    && row.enemy == "warden"
                    && row.attackers == 2
                    && row.round_trip_ms == 150
            })
            .expect("rifle/Warden multiplayer row should exist");
        assert_eq!(rifle_warden.required_hits, 3);
        assert_eq!(rifle_warden.required_volleys, 2);
        assert_eq!(rifle_warden.effective_ttk_ms, 260);
        assert_eq!(rifle_warden.expected_incoming_damage, 0);
    }

    #[test]
    fn block_two_candidate_meets_timing_and_multiplayer_budgets() {
        let report = build_report(LabMode::Candidate).expect("candidate report should build");
        assert_eq!(report.rows.len(), 24);
        for row in report.rows.iter().filter(|row| row.round_trip_ms == 0) {
            if row.attackers == 1 && row.enemy == "relay_drone" {
                assert!((600..=1_200).contains(&row.effective_ttk_ms));
            }
            if row.attackers == 1 && row.enemy == "warden" {
                assert!((1_200..=2_600).contains(&row.effective_ttk_ms));
                assert_eq!(row.expected_incoming_damage, 30);
            }
        }
        for weapon in ["pulse_rifle", "arc_sidearm"] {
            for enemy in ["relay_drone", "warden"] {
                let solo = report
                    .rows
                    .iter()
                    .find(|row| {
                        row.weapon == weapon
                            && row.enemy == enemy
                            && row.attackers == 1
                            && row.round_trip_ms == 0
                    })
                    .expect("candidate solo row should exist");
                let multiplayer = report
                    .rows
                    .iter()
                    .find(|row| {
                        row.weapon == weapon
                            && row.enemy == enemy
                            && row.attackers == 2
                            && row.round_trip_ms == 0
                    })
                    .expect("candidate multiplayer row should exist");
                assert!(multiplayer.effective_ttk_ms * 100 >= solo.effective_ttk_ms * 45);
            }
        }
    }
}
