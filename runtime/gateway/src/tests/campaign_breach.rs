use super::*;
use revenant_campaign::{CampaignState, ChapterId, ChapterRun, RunMode};

#[test]
fn powered_breach_shield_absorbs_attacks_and_keeps_the_core_closed() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (participant, receiver) =
        fixture_participant("local:breach-shield", 5036, &["pulse_rifle"]);
    let persistence = FixturePersistence::new(None, calls, Vec::new());
    let mut session = fixture_session(1, vec![participant], SessionStage::Boss, persistence);
    session.campaign = Some(CampaignState {
        story: revenant_campaign::story::StoryState::default(),
        revision: 20,
        cleared_chapters: 4,
        active: Some(ChapterRun {
            run_id: "breach-test".to_owned(),
            chapter: ChapterId::TheBreach,
            mode: RunMode::Progress,
            checkpoint: 1,
        }),
    });
    let boss = session
        .actors
        .spawn(ActorKind::Enemy, "warden", [2, 0, 0], 240);
    session.enemy_id = Some(boss.id);
    session.attack_at(5036, boss.id, 0).unwrap();
    assert_eq!(session.actors.get(boss.id).unwrap().health, 240);
    assert!(drain_messages(&receiver).iter().any(|m| matches!(m, ServerMessage::DamageApplied(d) if d.damage == 0 && d.remaining_health == 240)));
    assert!(session.attack_at(5036, boss.id, 100).is_err());
    session
        .campaign
        .as_mut()
        .unwrap()
        .active
        .as_mut()
        .unwrap()
        .checkpoint = 2;
    session.attack_at(5036, boss.id, 250).unwrap();
    assert_eq!(session.actors.get(boss.id).unwrap().health, 200);
    session.actors.update_position(5036, [8, 0, 0]);
    session.move_campaign(5036, [9, 0, 0]).unwrap();
    assert_eq!(session.actors.get(5036).unwrap().position, [8, 0, 0]);
    session
        .campaign
        .as_mut()
        .unwrap()
        .active
        .as_mut()
        .unwrap()
        .checkpoint = 3;
    session.move_campaign(5036, [9, 0, 0]).unwrap();
    assert_eq!(session.actors.get(5036).unwrap().position, [9, 0, 0]);
}
