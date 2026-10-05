use super::*;
use crate::challenge_mastery_attempt;
use revenant_challenges::{
    mastery::{Goal, Reason},
    modifiers::Preset,
};

#[test]
fn mastery_replays_clean_and_injured_prism_and_rejects_forged_proofs() {
    for (hit, reason) in [(false, Reason::Achieved), (true, Reason::AvoidPrismDamage)] {
        let (events, _) = prism::fixture_with_hit(true, hit);
        let attempt = challenge_mastery_attempt(&events).unwrap().unwrap();
        assert_eq!(attempt.assessments[0].goal, Goal::PrismExecution);
        assert_eq!(attempt.assessments[0].reason, reason);
        assert_eq!(attempt.prism_damage == Some(0), !hit);
        let mut foreign = events.clone();
        foreign[2].account_id = "local:foreign".into();
        assert!(challenge_mastery_attempt(&foreign).is_err());
        assert!(challenge_mastery_attempt(&events[..events.len() - 1]).is_err());
    }
    let (events, _) = prism::fixture(false);
    assert_eq!(
        challenge_mastery_attempt(&events)
            .unwrap()
            .unwrap()
            .assessments[0]
            .reason,
        Reason::CompleteContract
    );
}

#[test]
fn mastery_route_uses_proved_moves_without_a_time_requirement() {
    for preset in [Preset::WestApproach, Preset::WestApproachPace, Preset::Pace] {
        let (events, _) = route::fixture(preset);
        let attempt = challenge_mastery_attempt(&events).unwrap().unwrap();
        let expected = if preset == Preset::Pace {
            Reason::UseWestRoute
        } else {
            Reason::Achieved
        };
        assert_eq!(attempt.assessments[0].reason, expected);
        if preset != Preset::Pace {
            assert!(attempt.route_moves.unwrap() <= 40);
        }
        let mut missing = events;
        missing.remove(4);
        assert!(challenge_mastery_attempt(&missing).is_err());
    }
}

#[test]
fn mastery_requires_the_admitted_build_and_a_terminal_attempt() {
    let (events, _) = combat();
    let attempt = challenge_mastery_attempt(&events).unwrap().unwrap();
    assert_eq!(attempt.assessments[0].reason, Reason::UseBuild);
    let (entry, _) = admission(ContractId::CloseQuarters, &ChallengeState::default(), false);
    assert!(challenge_mastery_attempt(&entry).unwrap().is_none());
    let (events, _) = elite_combat();
    assert_eq!(
        challenge_mastery_attempt(&events)
            .unwrap()
            .unwrap()
            .assessments[0]
            .reason,
        Reason::Achieved
    );
    let (events, _) = elite_combat_order(true);
    assert_eq!(
        challenge_mastery_attempt(&events)
            .unwrap()
            .unwrap()
            .assessments[0]
            .reason,
        Reason::MenderFirst
    );
}
