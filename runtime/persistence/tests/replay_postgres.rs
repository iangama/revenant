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
