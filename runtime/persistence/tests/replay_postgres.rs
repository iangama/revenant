use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use revenant_persistence::{NewReplayEvent, Persistence};

#[test]
fn appends_and_loads_replay_events_from_postgres() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    let account_id = format!("local:replay-test-{suffix}");
    let session_id = format!("session-test-{suffix}");
    let mut persistence = Persistence::connect(&database_url).expect("PostgreSQL should connect");
    persistence
        .ensure_local_account(&account_id, "replay-test")
        .expect("test account should persist");
    for event_type in ["player_joined", "activity_completed"] {
        persistence
            .append_replay_event(&NewReplayEvent {
                event_type,
                session_id: &session_id,
                account_id: &account_id,
                activity_id: Some("relay_awakening"),
                actor_id: Some(42),
                payload: event_type,
            })
            .expect("event should append");
    }
    let events = persistence
        .replay_events(&session_id)
        .expect("events should load");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "player_joined");
    assert_eq!(events[1].event_type, "activity_completed");
    let sessions = persistence
        .replay_sessions(100)
        .expect("sessions should load");
    let session = sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .expect("inserted session should be listed");
    assert_eq!(session.event_count, 2);
    assert_eq!(session.participant_count, 1);
    assert!(session.completed);
    assert_eq!(
        persistence
            .latest_completed_session_id(&account_id)
            .expect("latest session should query")
            .as_deref(),
        Some(session_id.as_str())
    );
}

#[test]
fn grants_activity_reward_once_per_session() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    let account_id = format!("local:reward-test-{suffix}");
    let character_id = format!("{account_id}:operator");
    let session_id = format!("reward-session-{suffix}");
    let mut persistence = Persistence::connect(&database_url).expect("PostgreSQL should connect");
    persistence
        .ensure_local_account(&account_id, "reward-test")
        .expect("test account should persist");

    let first = persistence
        .complete_activity_with_reward(
            &session_id,
            &account_id,
            &character_id,
            "relay_awakening",
            "relay_core_fragment",
            1,
        )
        .expect("first reward should persist");
    let duplicate = persistence
        .complete_activity_with_reward(
            &session_id,
            &account_id,
            &character_id,
            "relay_awakening",
            "relay_core_fragment",
            1,
        )
        .expect("duplicate reward should be harmless");

    assert_eq!(first, Some(1));
    assert_eq!(duplicate, None);
    let inventory = persistence
        .inventory_for(&character_id)
        .expect("inventory should load");
    assert!(inventory
        .iter()
        .any(|item| item.item_id == "relay_core_fragment" && item.quantity == 1));
    assert_eq!(
        persistence
            .activity_completion_count(&account_id, "relay_awakening")
            .expect("completion count should load"),
        1
    );
}
