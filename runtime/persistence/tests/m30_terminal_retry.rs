//! Explicit Gate 5 retry of a real, already-committed cooperation session.

use std::env;

use postgres::{Client, NoTls};
use revenant_cooperation::MutationDisposition;
use revenant_persistence::{database_url_from_environment, Persistence};

#[test]
#[ignore = "requires the disposable M30 recovery matrix and an existing committed session"]
fn committed_cooperation_retry_preserves_rewards_and_replay() {
    let database_url =
        database_url_from_environment().expect("fixture URL file must be configured");
    let config: postgres::Config = database_url.parse().expect("fixture URL must parse");
    assert!(config
        .get_dbname()
        .is_some_and(|name| name.starts_with("m30f_")));
    assert_eq!(config.get_user(), Some("revenant_runtime"));
    assert_eq!(config.get_ports(), &[15451]);
    assert_eq!(
        config.get_hosts(),
        &[postgres::config::Host::Tcp("127.0.0.1".into())]
    );
    let session = env::var("M30_RETRY_SESSION").expect("committed fixture session is required");
    let mut connection = Client::connect(&database_url, NoTls).expect("fixture must be reachable");
    let row = connection.query_one(
        "SELECT terminal_elapsed_ms, participant_count FROM cooperation_operations \
         WHERE session_id=$1 AND terminal_outcome='succeeded' AND anchor_account_id LIKE 'local:m30%'",
        &[&session],
    ).expect("committed synthetic cooperation session must exist");
    let elapsed: i64 = row.get(0);
    let participants: i16 = row.get(1);
    assert_eq!(participants, 2);
    let mut persistence =
        Persistence::connect_existing(&database_url).expect("runtime connection must open");
    for _ in 0..2 {
        let receipt = persistence
            .complete_cooperation_operation_with_replay(
                &session,
                u64::try_from(elapsed).expect("elapsed time must be nonnegative"),
            )
            .expect("the exact committed terminal retry must succeed");
        assert_eq!(receipt.disposition, MutationDisposition::Replayed);
    }
    println!("M30_TERMINAL_RETRY participants=2 attempts=2 disposition=replayed");
}
