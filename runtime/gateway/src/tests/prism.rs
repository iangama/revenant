use super::*;
use revenant_ai::prism;
use revenant_protocol::{PrismMode, PrismPhase};

const PLAYER: u64 = 5034;
type Fixture = (
    SharedSession,
    Receiver<ServerMessage>,
    Arc<Mutex<Vec<String>>>,
);

fn rewards() -> Vec<Option<CompletionRewards>> {
    vec![Some(CompletionRewards {
        item_quantity: 1,
        experience_granted: 100,
        previous_level: 1,
        level: 1,
        experience: 100,
    })]
}

fn setup(capable: bool) -> Fixture {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (mut participant, receiver) =
        fixture_participant("local:prism-test", PLAYER, &["pulse_rifle"]);
    participant.prism_capable = capable;
    let persistence = FixturePersistence::new(None, calls.clone(), rewards());
    let mut session = fixture_session(1, vec![participant], SessionStage::Waiting, persistence);
    start_and_clear_drone(&mut session, PLAYER);
    drain_messages(&receiver);
    (session, receiver, calls)
}

fn fail_once(session: &mut SharedSession, calls: &Arc<Mutex<Vec<String>>>, kind: &str) {
    session.persistence = Box::new(FixturePersistence::new(
        Some(kind),
        calls.clone(),
        rewards(),
    ));
}

fn begin(session: &mut SharedSession, receiver: &Receiver<ServerMessage>) -> u64 {
    session.move_player(PLAYER, prism::ENTRANCE).unwrap();
    let boss = session.enemy_id.unwrap();
    assert_eq!(session.actors.get(boss).unwrap().archetype, "prism-warden");
    assert!(drain_messages(receiver)
        .iter()
        .any(|m| matches!(m, ServerMessage::PrismState(s) if s.mode == PrismMode::Opening)));
    boss
}

fn through_pulses(session: &mut SharedSession, receiver: &Receiver<ServerMessage>, boss: u64) {
    session.tick_prism_at(700).unwrap();
    session.move_player(PLAYER, [5, 0, 1]).unwrap();
    session.tick_prism_at(2500).unwrap();
    for time in [2500, 2750, 3000, 3250] {
        session.attack_at(PLAYER, boss, time).unwrap();
    }
    assert_eq!(session.actors.get(boss).unwrap().health, 160);
    assert!(drain_messages(receiver).iter().any(|m| matches!(m, ServerMessage::PrismState(s) if s.phase == PrismPhase::Pulses && s.mode == PrismMode::Shifting)));
    session.tick_prism_at(5649).unwrap();
    assert!(drain_messages(receiver).is_empty());
    session.tick_prism_at(5650).unwrap();
    session.tick_prism_at(7850).unwrap();
    session.attack_at(PLAYER, boss, 8000).unwrap();
    assert_eq!(session.actors.get(boss).unwrap().health, 160);
    session.tick_prism_at(8450).unwrap();
    session.move_player(PLAYER, [8, 0, 0]).unwrap();
    session.tick_prism_at(10650).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, 90);
    for time in [10650, 10900, 11150] {
        session.attack_at(PLAYER, boss, time).unwrap();
    }
    drain_messages(receiver);
}

#[test]
fn prism_failed_death_and_completion_retry_without_duplicate_fatal_event_or_reward() {
    let (mut session, receiver, calls) = setup(true);
    let boss = begin(&mut session, &receiver);
    through_pulses(&mut session, &receiver, boss);
    fail_once(&mut session, &calls, "enemy_died");
    assert!(session.attack_at(PLAYER, boss, 11400).is_err());
    assert_eq!(session.actors.get(boss).unwrap().health, 40);
    assert!(drain_messages(&receiver).is_empty());
    fail_once(&mut session, &calls, "completion_transaction");
    assert!(session.attack_at(PLAYER, boss, 11400).is_err());
    assert_eq!(session.stage, SessionStage::Boss);
    assert_eq!(session.actors.get(boss).unwrap().health, 0);
    assert!(drain_messages(&receiver).is_empty());
    let deaths_before = calls
        .lock()
        .unwrap()
        .iter()
        .filter(|s| *s == "enemy_died")
        .count();
    session.tick_prism_at(11500).unwrap();
    session.tick_prism_at(11600).unwrap();
    assert_eq!(session.stage, SessionStage::Complete);
    assert!(!session.prism_fighting());
    assert!(session.actors.get(boss).is_none());
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|s| *s == "enemy_died")
            .count(),
        deaths_before
    );
    let messages = drain_messages(&receiver);
    assert_eq!(
        messages
            .iter()
            .filter(|m| matches!(m,ServerMessage::LootGranted(r) if r.quantity == 1))
            .count(),
        1
    );
    assert_eq!(
        messages
            .iter()
            .filter(
                |m| matches!(m,ServerMessage::ProgressionGranted(r) if r.experience_granted == 100)
            )
            .count(),
        1
    );
    assert_eq!(
        messages
            .iter()
            .filter(|m| matches!(m, ServerMessage::ActivityComplete(_)))
            .count(),
        1
    );
    assert!(matches!(messages.first(),Some(ServerMessage::DamageApplied(d)) if d.killed));
    session.disconnect(PLAYER).unwrap();
    assert!(session.prism.is_none());
}

#[test]
fn prism_failed_spawn_block_warning_and_resolution_preserve_state_until_confirmation() {
    let (mut session, receiver, calls) = setup(true);
    let before = session.actors.get(PLAYER).unwrap().position;
    fail_once(&mut session, &calls, "boss_spawned");
    assert!(session.move_player(PLAYER, prism::ENTRANCE).is_err());
    assert!(session.prism.is_none());
    assert_eq!(session.actors.get(PLAYER).unwrap().position, before);
    assert_eq!(session.stage, SessionStage::Door);
    assert!(drain_messages(&receiver).is_empty());
    let boss = begin(&mut session, &receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.attack_at(PLAYER, boss, 0).is_err());
    assert!(drain_messages(&receiver).is_empty());
    session.attack_at(PLAYER, boss, 0).unwrap();
    assert_eq!(session.actors.get(boss).unwrap().health, 320);
    assert!(session.attack_at(PLAYER, boss, 100).is_err());
    drain_messages(&receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_prism_at(700).is_err());
    assert!(drain_messages(&receiver).is_empty());
    session.tick_prism_at(700).unwrap();
    assert!(drain_messages(&receiver).iter().any(|m| matches!(m, ServerMessage::PrismState(s) if s.mode == PrismMode::Warning && s.interval_ms == 1800)));
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.tick_prism_at(2500).is_err());
    assert_eq!(session.actors.get(PLAYER).unwrap().health, 90);
    assert!(drain_messages(&receiver).is_empty());
    session.tick_prism_at(2500).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, 72);
    session.tick_prism_at(2500).unwrap();
    assert_eq!(session.actors.get(PLAYER).unwrap().health, 72);
}

#[test]
fn prism_old_clients_skip_entry_and_failed_retreat_restores_normal_core_on_retry() {
    let (mut legacy, receiver, _) = setup(false);
    legacy.move_player(PLAYER, prism::ENTRANCE).unwrap();
    assert!(legacy.prism.is_none());
    assert!(!drain_messages(&receiver)
        .iter()
        .any(|m| matches!(m, ServerMessage::PrismState(_))));
    let (mut session, receiver, calls) = setup(true);
    let boss = begin(&mut session, &receiver);
    fail_once(&mut session, &calls, "field_activity");
    assert!(session.move_player(PLAYER, [4, 0, 2]).is_err());
    assert!(session.prism_fighting());
    assert_eq!(
        session.actors.get(PLAYER).unwrap().position,
        prism::ENTRANCE
    );
    assert!(drain_messages(&receiver).is_empty());
    session.move_player(PLAYER, [4, 0, 2]).unwrap();
    assert!(!session.prism_fighting());
    assert!(session.actors.get(boss).is_none());
    assert_eq!(session.stage, SessionStage::Door);
    assert!(drain_messages(&receiver).iter().any(|m| matches!(m,ServerMessage::ObjectiveUpdate(o) if o.objective_id == "defeat_warden" && o.state == "Pending")));
    session.move_player(PLAYER, [6, 0, 0]).unwrap();
    defeat_active_warden(&mut session, PLAYER);
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|s| *s == "completion_transaction")
            .count(),
        1
    );
}

#[test]
fn prism_lethal_lane_stops_ticks_and_grants_nothing() {
    let (mut session, receiver, calls) = setup(true);
    let boss = begin(&mut session, &receiver);
    for index in 0..5 {
        session.tick_prism_at(700 + index * 3600).unwrap();
        session.tick_prism_at(2500 + index * 3600).unwrap();
    }
    assert_eq!(session.actors.get(PLAYER).unwrap().health, 0);
    assert_eq!(session.stage, SessionStage::Failed);
    assert!(!session.prism_fighting());
    drain_messages(&receiver);
    session.tick_prism_at(100_000).unwrap();
    assert!(session.attack_at(PLAYER, boss, 100_000).is_err());
    assert!(drain_messages(&receiver).is_empty());
    assert!(!calls
        .lock()
        .unwrap()
        .iter()
        .any(|s| s == "completion_transaction"));
}

#[test]
fn campaign_prism_retries_checkpoint_without_repeating_death_proof_or_reward() {
    use revenant_campaign::{CampaignState, ChapterId, ChapterRun, RunMode};
    let (mut session, receiver, calls) = setup(true);
    let boss = begin(&mut session, &receiver);
    through_pulses(&mut session, &receiver, boss);
    let state = CampaignState {
        story: revenant_campaign::story::StoryState::default(),
        revision: 27,
        cleared_chapters: 5,
        active: Some(ChapterRun {
            run_id: "prism-failure".to_owned(),
            chapter: ChapterId::PrismCore,
            mode: RunMode::Progress,
            checkpoint: 1,
        }),
    };
    let mut persistence =
        FixturePersistence::new(Some("campaign_checkpoint"), calls.clone(), Vec::new());
    persistence.campaign_state = Some(state.clone());
    session.persistence = Box::new(persistence);
    session.campaign = Some(state);
    session.route = None;
    session.activity = ScriptedActivity::from_source(include_str!(
        "../../../../scripts/activities/campaign_prism.lua"
    ))
    .unwrap();
    session.activity.apply_trigger(&WorldTrigger::AreaReached {
        area_id: "prism_arrival".to_owned(),
    });
    assert!(session.attack_at(PLAYER, boss, 11400).is_err());
    assert_eq!(session.actors.get(boss).unwrap().health, 0);
    assert!(drain_messages(&receiver).is_empty());
    let count = |name: &str| {
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.as_str() == name)
            .count()
    };
    let deaths = count("enemy_died");
    assert_eq!(count("campaign_objective"), 1);
    session.tick_prism_at(11500).unwrap();
    session.tick_prism_at(11600).unwrap();
    assert_eq!(count("enemy_died"), deaths);
    assert_eq!(count("campaign_objective"), 1);
    assert_eq!(count("campaign_checkpoint"), 2);
    assert_eq!(count("completion_transaction"), 0);
    assert_eq!(session.stage, SessionStage::Door);
    assert_eq!(
        session
            .campaign
            .as_ref()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .checkpoint,
        2
    );
    assert!(!session.prism_fighting() && session.enemy_id.is_none());
    assert!(session.actors.get(boss).is_none());
    let messages = drain_messages(&receiver);
    assert!(!messages.iter().any(|m| matches!(
        m,
        ServerMessage::ActivityComplete(_)
            | ServerMessage::LootGranted(_)
            | ServerMessage::ProgressionGranted(_)
    )));
    assert!(messages.iter().any(|m| matches!(m, ServerMessage::ObjectiveUpdate(o) if o.objective_id == "prism_shutdown" && o.state == "Active")));
}
