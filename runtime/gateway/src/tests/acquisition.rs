use super::*;
use revenant_modules::acquisition::{ArcId, Milestone};
use revenant_protocol::AcquisitionClaimIntent;
use revenant_replay::AcquisitionMilestoneProof;

#[test]
fn acquisition_requires_capability_and_completion_and_never_emits_mission_loot() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut persistence = FixturePersistence::new(None, Arc::clone(&calls), Vec::new())
        .with_module_state(4, &[], &[], 0);
    persistence
        .acquisition_state
        .proofs
        .push(AcquisitionMilestoneProof {
            milestone: Milestone::MeridianRecovered,
            session_id: "verified-meridian".to_owned(),
            through_event_id: 10,
        });
    let (mut player, receiver) = fixture_participant("local:acquisition", 5036, &["pulse_rifle"]);
    player.module_state = persistence.persisted_module_state();
    player.content_capable = true;
    player.build_capable = true;
    let actor = player.actor.clone();
    let mut session = fixture_session(1, vec![player], SessionStage::Waiting, persistence);
    let intent = AcquisitionClaimIntent {
        arc_id: "meridian".to_owned(),
    };
    assert!(session.claim_acquisition(5036, &intent).is_err());
    assert!(calls.lock().unwrap().is_empty());
    assert!(receiver.try_recv().is_err());
    session.participants[0].acquisition_capable = true;
    session.claim_acquisition(5036, &intent).unwrap();
    assert!(
        matches!(receiver.recv().unwrap(), ServerMessage::AcquisitionClaimed(result) if !result.accepted && result.granted_fragments == 0)
    );
    assert!(!calls
        .lock()
        .unwrap()
        .iter()
        .any(|c| c == "acquisition_claimed"));
    session.stage = SessionStage::Complete;
    session.send_acquisition_state(5036).unwrap();
    assert!(
        matches!(receiver.recv().unwrap(), ServerMessage::AcquisitionSnapshot(state) if state.can_claim && state.arcs[0].status == "ready")
    );
    for replayed in [false, true] {
        session.claim_acquisition(5036, &intent).unwrap();
        let ServerMessage::AcquisitionClaimed(result) = receiver.recv().unwrap() else {
            panic!("expected commission result");
        };
        assert!(result.accepted);
        assert_eq!(result.replayed, replayed);
        assert_eq!(result.granted_fragments, if replayed { 0 } else { 2 });
        assert_eq!(result.modules.fragments, 6);
        assert_eq!(result.modules.catalog_revision, "m35-v2");
        assert_eq!(result.state.arcs[0].arc_id, ArcId::Meridian.as_str());
        assert_eq!(result.state.arcs[0].status, "claimed");
    }
    assert_eq!(session.participants[0].actor, actor);
    assert!(receiver.try_recv().is_err());
}
