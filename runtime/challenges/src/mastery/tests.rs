use super::*;
use crate::{modifiers::Preset, ChallengeRun};

fn result(goal: Goal) -> AttemptResult {
    let (weapon, modules) = match goal {
        Goal::Breacher => (
            "scatter_caster",
            vec![ModuleId::FocusLens, ModuleId::BreachShunt],
        ),
        Goal::Marksman => (
            "rail_driver",
            vec![ModuleId::StandoffOptic, ModuleId::SkirmishDrive],
        ),
        Goal::Guard => (
            "pulse_rifle",
            vec![ModuleId::CycleBypass, ModuleId::AblativeShell],
        ),
        _ => ("pulse_rifle", vec![]),
    };
    AttemptResult {
        run: ChallengeRun {
            run_id: 1,
            rules: goal
                .contract()
                .rules_with_preset(if goal == Goal::RoutePlanner {
                    Preset::WestApproach
                } else {
                    Preset::Baseline
                })
                .unwrap(),
            equipment: Equipment {
                weapon_id: weapon.into(),
                catalog_revision: revenant_modules::BUILD_CATALOG_REVISION.into(),
                loadout_revision: 1,
                modules,
            },
            objectives: goal.contract().complete_mask(),
            elapsed_ms: None,
        },
        outcome: Outcome::Completed,
    }
}

fn clean() -> Facts {
    Facts {
        reserve_unused: true,
        west_route_moves: Some(38),
        mender_before_bulwark_damage: true,
        prism_damage: Some(0),
    }
}

#[test]
fn six_goals_require_completed_attempts_and_each_builds_actual_equipment() {
    for goal in Goal::ALL {
        let mut terminal = result(goal);
        terminal.run.equipment.validate().unwrap();
        assert_eq!(
            assess(&terminal, &clean()).assessments,
            vec![Assessment {
                goal,
                reason: Reason::Achieved
            }]
        );
        for outcome in [Outcome::Abandoned, Outcome::Defeated, Outcome::Interrupted] {
            terminal.outcome = outcome;
            assert_eq!(
                assess(&terminal, &clean()).assessments[0].reason,
                Reason::CompleteContract
            );
        }
        terminal.outcome = Outcome::Completed;
        if matches!(goal, Goal::Breacher | Goal::Marksman | Goal::Guard) {
            terminal.run.equipment.modules.pop();
            assert_eq!(
                assess(&terminal, &clean()).assessments[0].reason,
                Reason::UseBuild
            );
            terminal = result(goal);
            terminal.run.equipment.weapon_id = "arc_sidearm".into();
            assert_eq!(
                assess(&terminal, &clean()).assessments[0].reason,
                Reason::UseBuild
            );
        }
    }
}

#[test]
fn missing_proof_and_a_single_missed_condition_never_award_clean_goals() {
    for (goal, reason) in [
        (Goal::Guard, Reason::PreserveReserve),
        (Goal::RoutePlanner, Reason::UseWestRoute),
        (Goal::TargetPriority, Reason::MenderFirst),
        (Goal::PrismExecution, Reason::AvoidPrismDamage),
    ] {
        assert_eq!(
            assess(&result(goal), &Facts::default()).assessments[0].reason,
            reason
        );
    }
    let mut facts = clean();
    facts.west_route_moves = Some(40);
    assert_eq!(
        assess(&result(Goal::RoutePlanner), &facts).assessments[0].reason,
        Reason::Achieved
    );
    facts.west_route_moves = Some(41);
    assert_eq!(
        assess(&result(Goal::RoutePlanner), &facts).assessments[0].reason,
        Reason::ReduceRouteMoves
    );
    facts.prism_damage = Some(1);
    assert_eq!(
        assess(&result(Goal::PrismExecution), &facts).assessments[0].reason,
        Reason::AvoidPrismDamage
    );
}

#[test]
fn earliest_proofs_and_three_titles_survive_repetition_and_later_failures() {
    let mut archive = Archive::default();
    for (index, goal) in Goal::ALL.into_iter().enumerate() {
        let mut terminal = result(goal);
        terminal.run.run_id = (index as u64 + 1) * 10;
        let attempt = assess(&terminal, &clean());
        archive.observe(attempt.clone());
        archive.observe(attempt);
    }
    assert_eq!(archive.records.len(), 6);
    assert_eq!(
        archive.badges(),
        vec![
            BadgeRecord {
                badge: Badge::ArsenalAdept,
                run_id: 30
            },
            BadgeRecord {
                badge: Badge::FieldTactician,
                run_id: 50
            },
            BadgeRecord {
                badge: Badge::PrismAdept,
                run_id: 60
            },
        ]
    );
    let records = archive.records.clone();
    let mut repeated = result(Goal::Breacher);
    repeated.run.run_id = 70;
    archive.observe(assess(&repeated, &clean()));
    repeated.run.run_id = 80;
    repeated.outcome = Outcome::Defeated;
    archive.observe(assess(&repeated, &clean()));
    assert_eq!(archive.records, records);
    assert_eq!(archive.last_attempt.as_ref().unwrap().run_id, 80);
    assert_eq!(
        archive.last_attempt.as_ref().unwrap().assessments[0].reason,
        Reason::CompleteContract
    );
    repeated.run.run_id = 5;
    repeated.outcome = Outcome::Completed;
    archive.observe(assess(&repeated, &clean()));
    assert_eq!(archive.records[0].run_id, 5);
    assert_eq!(archive.last_attempt.unwrap().run_id, 80);
}
