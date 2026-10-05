use super::*;
use revenant_campaign::{story, CampaignState, ChapterId, ChapterRun, RunMode};
use revenant_protocol::{
    CampaignEntryMode, CampaignJoinRequest, CampaignStoryAction, CampaignStoryIntent,
};

#[test]
fn story_uses_real_actor_position_and_publishes_only_committed_state() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (mut participant, receiver) = fixture_participant("local:story", 5060, &["pulse_rifle"]);
    participant.campaign_entry = Some(CampaignJoinRequest {
        character_id: participant.character_id.clone(),
        chapter_id: "meridian_readings".to_owned(),
        mode: CampaignEntryMode::Resume,
        expected_revision: 5,
    });
    let state = CampaignState {
        revision: 5,
        cleared_chapters: 1,
        active: Some(ChapterRun {
            run_id: "story-run".to_owned(),
            chapter: ChapterId::MeridianReadings,
            mode: RunMode::Progress,
            checkpoint: 2,
        }),
        ..Default::default()
    };
    let mut persistence =
        FixturePersistence::new(Some("campaign_checkpoint"), calls.clone(), Vec::new());
    persistence.campaign_state = Some(state.clone());
    let mut session = fixture_session(1, vec![participant], SessionStage::Door, persistence);
    session.campaign = Some(state.clone());
    let intent = CampaignStoryIntent {
        operation_id: "one-choice".to_owned(),
        run_id: "story-run".to_owned(),
        expected_revision: 5,
        action: CampaignStoryAction::SupplyService,
    };
    session.interact_campaign_story(5060, &intent).unwrap();
    assert!(calls.lock().unwrap().is_empty());
    assert!(drain_messages(&receiver)
        .iter()
        .any(|m| matches!(m, ServerMessage::CampaignStoryResult(r) if r.status == "rejected")));
    session.actors.update_position(5060, story::DEPARTURE_BOARD);
    session.interact_campaign_story(5060, &intent).unwrap();
    assert_eq!(session.campaign, Some(state));
    assert!(drain_messages(&receiver)
        .iter()
        .any(|m| matches!(m, ServerMessage::CampaignStoryResult(r) if r.status == "unconfirmed")));
    session.interact_campaign_story(5060, &intent).unwrap();
    assert_eq!(
        session.campaign.as_ref().unwrap().story.supply,
        Some(story::SupplyApproach::Service)
    );
    let messages = drain_messages(&receiver);
    assert!(messages
        .iter()
        .any(|m| matches!(m, ServerMessage::CampaignStoryResult(r) if r.status == "accepted")));
    assert!(!messages.iter().any(|m| matches!(
        m,
        ServerMessage::LootGranted(_)
            | ServerMessage::ProgressionGranted(_)
            | ServerMessage::ActivityComplete(_)
    )));
}
