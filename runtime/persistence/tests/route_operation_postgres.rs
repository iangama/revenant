use std::env;
use std::sync::{Arc, Barrier, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use postgres::{Client, NoTls};
use revenant_operations::{
    OperationDisposition, RouteError, RouteId, RouteSeed, TerminalOutcome, MAX_ROUTE_SEED,
};
use revenant_persistence::{
    Persistence, RouteOperationSelection, RouteParticipant, RoutePersistenceError,
};

fn test_persistence(database_url: &str) -> Persistence {
    static SCHEMA_READY: OnceLock<()> = OnceLock::new();
    SCHEMA_READY.get_or_init(|| {
        drop(Persistence::connect(database_url).expect("PostgreSQL schema should initialize"));
    });
    Persistence::connect_existing(database_url).expect("PostgreSQL should connect")
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos()
}

#[derive(Debug, Clone)]
struct ParticipantFixture {
    account: String,
    character: String,
    actor: u64,
}

impl ParticipantFixture {
    fn borrowed(&self) -> RouteParticipant<'_> {
        RouteParticipant {
            account_id: &self.account,
            character_id: &self.character,
            actor_id: self.actor,
        }
    }
}

fn prepare_participants(database_url: &str, label: &str, count: usize) -> Vec<ParticipantFixture> {
    let suffix = unique_suffix();
    let mut persistence = test_persistence(database_url);
    (0..count)
        .map(|index| {
            let account = format!("m27_{label}_{index}_{suffix}");
            let character = format!("{account}:operator");
            persistence
                .ensure_local_account(&account, &format!("M27 {label} {index}"))
                .expect("route participant should persist");
            ParticipantFixture {
                account,
                character,
                actor: u64::try_from(suffix % 9_000_000_000_000_000_000)
                    .expect("actor suffix should fit")
                    + u64::try_from(index).expect("participant index should fit"),
            }
        })
        .collect()
}

fn select(
    persistence: &mut Persistence,
    session: &str,
    operation: &str,
    route: RouteId,
    seed: u64,
    fixtures: &[ParticipantFixture],
) -> Result<
    revenant_operations::OperationReceipt<revenant_operations::RouteSelectionOutcome>,
    RoutePersistenceError,
> {
    let participants = fixtures
        .iter()
        .map(ParticipantFixture::borrowed)
        .collect::<Vec<_>>();
    persistence.select_route_operation(&RouteOperationSelection {
        session_id: session,
        activity_id: "relay_awakening",
        requester_account_id: &fixtures[0].account,
        operation_id: operation,
        route_id: route,
        candidate_seed: RouteSeed::new(seed).expect("test seed should be valid"),
        participants: &participants,
    })
}

#[test]
fn selection_retry_conflict_success_and_reconnect_are_exact() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare_participants(&database_url, "success", 2);
    let session = format!("m27-success-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    let applied = select(
        &mut persistence,
        &session,
        "route-success",
        RouteId::Breach,
        0,
        &fixtures,
    )
    .expect("route selection should apply");
    assert_eq!(applied.disposition, OperationDisposition::Applied);

    let mut reconnected = Persistence::connect_existing(&database_url)
        .expect("fresh persistence connection should open");
    let replayed = select(
        &mut reconnected,
        &session,
        "route-success",
        RouteId::Breach,
        MAX_ROUTE_SEED,
        &fixtures,
    )
    .expect("same route request should replay");
    assert_eq!(replayed.disposition, OperationDisposition::Replayed);
    assert_eq!(replayed.outcome, applied.outcome);
    assert!(matches!(
        select(
            &mut reconnected,
            &session,
            "route-conflict",
            RouteId::Stabilize,
            2,
            &fixtures,
        ),
        Err(RoutePersistenceError::Domain(
            RouteError::IdempotencyConflict
        ))
    ));

    let completed = reconnected
        .complete_route_operation(&session, 90_000)
        .expect("deadline-boundary success should commit");
    assert_eq!(completed.disposition, OperationDisposition::Applied);
    assert_eq!(completed.outcome.outcome, TerminalOutcome::Succeeded);
    assert_eq!(
        completed.outcome.reward.expect("success reward").fragments,
        2
    );
    let retry = reconnected
        .complete_route_operation(&session, 90_000)
        .expect("terminal retry should replay");
    assert_eq!(retry.disposition, OperationDisposition::Replayed);
    assert_eq!(retry.outcome, completed.outcome);
    assert!(matches!(
        reconnected.fail_route_operation_timeout(&session, 90_001),
        Err(RoutePersistenceError::Domain(RouteError::TerminalConflict))
    ));
    assert_success_rows(&database_url, &session, &fixtures, 2, 100);
}

fn assert_success_rows(
    database_url: &str,
    session: &str,
    fixtures: &[ParticipantFixture],
    fragments: i32,
    experience: i64,
) {
    let mut client = Client::connect(database_url, NoTls).expect("assertion connection");
    let counts = client
        .query_one(
            "SELECT \
                (SELECT COUNT(*) FROM route_operations WHERE session_id = $1), \
                (SELECT COUNT(*) FROM route_operation_participants WHERE session_id = $1), \
                (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), \
                (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1), \
                (SELECT COUNT(*) FROM activity_history WHERE character_id = ANY($2))",
            &[
                &session,
                &fixtures
                    .iter()
                    .map(|value| value.character.as_str())
                    .collect::<Vec<_>>(),
            ],
        )
        .expect("route result counts should query");
    assert_eq!(
        (0..5)
            .map(|index| counts.get::<_, i64>(index))
            .collect::<Vec<_>>(),
        vec![1, 2, 2, 2, 2]
    );
    for fixture in fixtures {
        let state = client
            .query_one(
                "SELECT \
                    (SELECT quantity FROM inventory WHERE character_id = $1 \
                        AND item_id = 'relay_core_fragment'), \
                    (SELECT experience FROM progression WHERE character_id = $1)",
                &[&fixture.character],
            )
            .expect("participant rewards should query");
        assert_eq!(
            (state.get::<_, i32>(0), state.get::<_, i64>(1)),
            (fragments, experience)
        );
    }
}

#[test]
fn timeout_boundary_is_durable_and_grants_nothing() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare_participants(&database_url, "timeout", 1);
    let session = format!("m27-timeout-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    select(
        &mut persistence,
        &session,
        "route-timeout",
        RouteId::Stabilize,
        3,
        &fixtures,
    )
    .expect("timeout selection should apply");
    assert!(matches!(
        persistence.fail_route_operation_timeout(&session, 90_000),
        Err(RoutePersistenceError::Domain(
            RouteError::DeadlineNotReached { .. }
        ))
    ));
    let failed = persistence
        .fail_route_operation_timeout(&session, 90_001)
        .expect("first post-deadline observation should fail operation");
    assert_eq!(failed.outcome.outcome, TerminalOutcome::FailedTimeout);
    assert_eq!(failed.outcome.reward, None);
    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    let counts = client
        .query_one(
            "SELECT \
                (SELECT COUNT(*) FROM activity_history WHERE character_id = $1), \
                (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $2), \
                (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $2), \
                (SELECT COUNT(*) FROM route_operations WHERE session_id = $2 \
                    AND terminal_outcome = 'failed_timeout' AND elapsed_ms = 90001)",
            &[&fixtures[0].character, &session],
        )
        .expect("timeout counts should query");
    assert_eq!(
        (0..4)
            .map(|index| counts.get::<_, i64>(index))
            .collect::<Vec<_>>(),
        vec![0, 0, 0, 1]
    );
}

#[test]
fn business_and_identity_rejections_leave_session_unreserved() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare_participants(&database_url, "rejection", 2);
    let participants = fixtures
        .iter()
        .map(ParticipantFixture::borrowed)
        .collect::<Vec<_>>();
    let session = format!("m27-rejection-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    let wrong_leader = persistence
        .select_route_operation(&RouteOperationSelection {
            session_id: &session,
            activity_id: "relay_awakening",
            requester_account_id: &fixtures[1].account,
            operation_id: "rejected-then-valid",
            route_id: RouteId::Stabilize,
            candidate_seed: RouteSeed::new(8).expect("seed should validate"),
            participants: &participants,
        })
        .expect_err("non-leader selection should reject");
    assert!(matches!(
        wrong_leader,
        RoutePersistenceError::Domain(RouteError::NotLeader)
    ));
    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    assert_eq!(
        route_write_counts(&mut client, &session),
        vec![0, 0, 0, 0, 0]
    );
    drop(client);
    let applied = select(
        &mut persistence,
        &session,
        "rejected-then-valid",
        RouteId::Stabilize,
        8,
        &fixtures,
    )
    .expect("rejected operation identity should remain available");
    assert_eq!(applied.disposition, OperationDisposition::Applied);
}

#[test]
fn concurrent_same_selection_persists_one_resolution() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixture = prepare_participants(&database_url, "concurrent", 1)
        .pop()
        .expect("one fixture");
    let session = format!("m27-concurrent-{}", unique_suffix());
    let barrier = Arc::new(Barrier::new(2));
    let handles = [0, MAX_ROUTE_SEED]
        .into_iter()
        .map(|seed| {
            let database_url = database_url.clone();
            let fixture = fixture.clone();
            let session = session.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut persistence = Persistence::connect_existing(&database_url)
                    .expect("concurrent connection should open");
                barrier.wait();
                select(
                    &mut persistence,
                    &session,
                    "route-concurrent",
                    RouteId::Breach,
                    seed,
                    &[fixture],
                )
                .expect("concurrent selection should converge")
            })
        })
        .collect::<Vec<_>>();
    let receipts = handles
        .into_iter()
        .map(|handle| handle.join().expect("selection thread should join"))
        .collect::<Vec<_>>();
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.disposition == OperationDisposition::Applied)
            .count(),
        1
    );
    assert_eq!(receipts[0].outcome, receipts[1].outcome);
    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    assert_eq!(
        client
            .query_one(
                "SELECT COUNT(*) FROM route_operations WHERE session_id = $1",
                &[&session],
            )
            .expect("operation count should query")
            .get::<_, i64>(0),
        1
    );
}

#[test]
fn database_constraints_reject_catalog_and_terminal_drift() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare_participants(&database_url, "constraints", 1);
    let session = format!("m27-constraints-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    select(
        &mut persistence,
        &session,
        "route-constraints",
        RouteId::Breach,
        0,
        &fixtures,
    )
    .expect("constraint fixture should persist");
    for statement in [
        "UPDATE route_operations SET event_id = 'shielded_channel' WHERE session_id = $1",
        "UPDATE route_operations SET reward_quantity = 1 WHERE session_id = $1",
        "UPDATE route_operations SET objective_path = \
            '[\"clear_drone_group\", \"reach_relay_stabilizer\", \
              \"reach_relay_door\", \"defeat_warden\"]'::JSONB WHERE session_id = $1",
        "UPDATE route_operations SET terminal_outcome = 'succeeded', elapsed_ms = 90001, \
            terminal_at = NOW() WHERE session_id = $1",
    ] {
        let mut client = Client::connect(&database_url, NoTls).expect("constraint connection");
        assert!(
            client.execute(statement, &[&session]).is_err(),
            "constraint drift should reject: {statement}"
        );
    }
    let replayed = select(
        &mut persistence,
        &session,
        "route-constraints",
        RouteId::Breach,
        MAX_ROUTE_SEED,
        &fixtures,
    )
    .expect("failed SQL mutations should preserve canonical selection");
    assert_eq!(replayed.disposition, OperationDisposition::Replayed);
}

struct FailureTrigger {
    database_url: String,
    table: &'static str,
    trigger: String,
    function: String,
}

impl FailureTrigger {
    fn install(
        database_url: &str,
        label: &str,
        table: &'static str,
        event: &'static str,
        predicate: &str,
    ) -> Self {
        let suffix = unique_suffix();
        let trigger = format!("m27_fail_trigger_{label}_{suffix}");
        let function = format!("m27_fail_function_{label}_{suffix}");
        let mut client = Client::connect(database_url, NoTls).expect("failure connection");
        client
            .batch_execute(&format!(
                "CREATE FUNCTION {function}() RETURNS TRIGGER LANGUAGE plpgsql AS $m27$ \
                 BEGIN IF {predicate} THEN RAISE EXCEPTION 'm27 injected failure'; \
                 END IF; RETURN NEW; END; $m27$; \
                 CREATE TRIGGER {trigger} BEFORE {event} ON {table} \
                 FOR EACH ROW EXECUTE FUNCTION {function}();"
            ))
            .expect("failure trigger should install");
        Self {
            database_url: database_url.to_owned(),
            table,
            trigger,
            function,
        }
    }
}

impl Drop for FailureTrigger {
    fn drop(&mut self) {
        if let Ok(mut client) = Client::connect(&self.database_url, NoTls) {
            let _ = client.batch_execute(&format!(
                "DROP TRIGGER IF EXISTS {} ON {}; DROP FUNCTION IF EXISTS {}();",
                self.trigger, self.table, self.function
            ));
        }
    }
}

#[test]
fn selection_and_terminal_failures_roll_back_every_observed_write() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    for (label, table) in [
        ("selection_parent", "route_operations"),
        ("selection_participant", "route_operation_participants"),
    ] {
        assert_selection_failure(&database_url, label, table);
    }
    for (label, table, event) in [
        ("inventory_grant", "inventory_reward_grants", "INSERT"),
        ("inventory", "inventory", "INSERT"),
        ("progression_grant", "progression_reward_grants", "INSERT"),
        ("progression", "progression", "UPDATE"),
        ("character", "characters", "UPDATE"),
        ("history", "activity_history", "INSERT"),
        ("terminal", "route_operations", "UPDATE"),
    ] {
        assert_terminal_failure(&database_url, label, table, event);
    }
}

fn assert_selection_failure(database_url: &str, label: &str, table: &'static str) {
    let fixtures = prepare_participants(database_url, label, 1);
    let session = format!("m27-{label}-{}", unique_suffix());
    let predicate = format!("NEW.session_id = '{session}'");
    let mut persistence = test_persistence(database_url);
    {
        let _trigger = FailureTrigger::install(database_url, label, table, "INSERT", &predicate);
        assert!(matches!(
            select(
                &mut persistence,
                &session,
                "failure-selection",
                RouteId::Breach,
                0,
                &fixtures,
            ),
            Err(RoutePersistenceError::Database(_))
        ));
    }
    let mut client = Client::connect(database_url, NoTls).expect("assertion connection");
    assert_eq!(
        route_write_counts(&mut client, &session),
        vec![0, 0, 0, 0, 0]
    );
    select(
        &mut persistence,
        &session,
        "failure-selection",
        RouteId::Breach,
        0,
        &fixtures,
    )
    .expect("rolled-back selection identifier should remain available");
}

fn assert_terminal_failure(
    database_url: &str,
    label: &str,
    table: &'static str,
    event: &'static str,
) {
    let fixtures = prepare_participants(database_url, label, 1);
    let session = format!("m27-{label}-{}", unique_suffix());
    let mut persistence = test_persistence(database_url);
    select(
        &mut persistence,
        &session,
        "failure-terminal",
        RouteId::Breach,
        0,
        &fixtures,
    )
    .expect("terminal failure selection should persist");
    let predicate = if matches!(
        table,
        "route_operations" | "inventory_reward_grants" | "progression_reward_grants"
    ) {
        format!("NEW.session_id = '{session}'")
    } else {
        format!("NEW.character_id = '{}'", fixtures[0].character)
    };
    {
        let _trigger = FailureTrigger::install(database_url, label, table, event, &predicate);
        assert!(matches!(
            persistence.complete_route_operation(&session, 90_000),
            Err(RoutePersistenceError::Database(_))
        ));
    }
    let mut client = Client::connect(database_url, NoTls).expect("assertion connection");
    assert_eq!(
        route_write_counts(&mut client, &session),
        vec![1, 1, 0, 0, 0]
    );
    let state = client
        .query_one(
            "SELECT \
                (SELECT COUNT(*) FROM inventory WHERE character_id = $1 \
                    AND item_id = 'relay_core_fragment'), \
                (SELECT experience FROM progression WHERE character_id = $1), \
                (SELECT level FROM characters WHERE id = $1)",
            &[&fixtures[0].character],
        )
        .expect("rolled-back reward state should query");
    assert_eq!(
        (
            state.get::<_, i64>(0),
            state.get::<_, i64>(1),
            state.get::<_, i32>(2)
        ),
        (0, 0, 1)
    );
}

fn route_write_counts(client: &mut Client, session: &str) -> Vec<i64> {
    let row = client
        .query_one(
            "SELECT \
                (SELECT COUNT(*) FROM route_operations WHERE session_id = $1), \
                (SELECT COUNT(*) FROM route_operation_participants WHERE session_id = $1), \
                (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), \
                (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1), \
                (SELECT COUNT(*) FROM activity_history WHERE character_id IN (\
                    SELECT character_id FROM route_operation_participants WHERE session_id = $1))",
            &[&session],
        )
        .expect("route write counts should query");
    (0..5).map(|index| row.get(index)).collect()
}
