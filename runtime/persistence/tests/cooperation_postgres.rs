use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use postgres::{Client, NoTls};
use revenant_cooperation::{
    CombatProfile, MutationDisposition, Phase, Role, TerminalOutcome, REVIVE_HEALTH, SUCCESS_REWARD,
};
use revenant_inventory::{weapon_profile, RELAY_CORE_FRAGMENT};
use revenant_modules::{
    resolve_build, BaseCombatProfile, ModuleId, CATALOG_REVISION as MODULE_CATALOG_REVISION,
};
use revenant_persistence::{
    CooperationOperationInput, CooperationParticipant, CooperationPersistenceError, Persistence,
};

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos()
}

#[derive(Clone)]
struct Fixture {
    account: String,
    character: String,
    actor: u64,
    weapon: &'static str,
    modules: Vec<ModuleId>,
    profile: CombatProfile,
    health: u32,
}

fn test_persistence(database_url: &str) -> Persistence {
    Persistence::connect_existing(database_url).expect("test persistence should connect")
}

fn prepare(database_url: &str, label: &str) -> [Fixture; 2] {
    let suffix = unique_suffix();
    let mut persistence = test_persistence(database_url);
    let weapons = ["pulse_rifle", "arc_sidearm"];
    std::array::from_fn(|index| {
        let account = format!("m28_{label}_{index}_{suffix}");
        let character = format!("{account}:operator");
        persistence
            .ensure_local_account(&account, &format!("M28 {label} {index}"))
            .expect("fixture account should persist");
        let modules = if index == 0 {
            vec![ModuleId::WardCapacitor]
        } else {
            Vec::new()
        };
        let weapon = weapon_profile(weapons[index]).expect("fixture weapon should exist");
        let resolved = resolve_build(
            MODULE_CATALOG_REVISION,
            BaseCombatProfile {
                damage: weapon.damage,
                range: weapon.range,
                cooldown_ms: weapon.cooldown_ms,
                max_health: 100,
            },
            &modules,
        )
        .expect("fixture build should resolve");
        Fixture {
            account,
            character,
            actor: u64::try_from((suffix % 8_000_000_000_000_000_000) + index as u128 + 1)
                .expect("actor should fit BIGINT"),
            weapon: weapons[index],
            modules,
            profile: CombatProfile {
                damage: resolved.profile.damage,
                range: resolved.profile.range,
                cooldown_ms: resolved.profile.cooldown_ms,
                max_health: resolved.profile.max_health,
            },
            health: if index == 0 { 110 } else { 90 },
        }
    })
}

fn participants(fixtures: &[Fixture; 2]) -> [CooperationParticipant<'_>; 2] {
    fixtures.each_ref().map(|fixture| CooperationParticipant {
        account_id: &fixture.account,
        character_id: &fixture.character,
        actor_id: fixture.actor,
        weapon_item_id: fixture.weapon,
        modules: &fixture.modules,
        profile: fixture.profile,
        current_health: fixture.health,
    })
}

fn start(
    persistence: &mut Persistence,
    session: &str,
    operation_id: &str,
    fixtures: &[Fixture; 2],
) -> Result<revenant_persistence::CooperationReceipt, CooperationPersistenceError> {
    let supplied = participants(fixtures);
    persistence.start_cooperation_operation(&CooperationOperationInput {
        session_id: session,
        activity_id: "relay_awakening",
        requester_account_id: &fixtures[0].account,
        operation_id,
        participants: &supplied,
    })
}

fn drive_to_encounter(
    persistence: &mut Persistence,
    session: &str,
) -> Result<(), CooperationPersistenceError> {
    persistence.arrive_cooperation_anchor(session, 100)?;
    persistence.ping_cooperation_operation(session, "ping-1", 200)?;
    persistence.down_cooperation_runner(session, 300)?;
    persistence.start_cooperation_revive(session, "revive-1", 4, 400)?;
    persistence.observe_cooperation_revive(session, "revive-1", 4, 2_400)?;
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn lifecycle_retries_cancellation_and_equal_rewards_are_exact() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare(&database_url, "lifecycle");
    let session = format!("m28-lifecycle-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    assert_eq!(
        start(&mut persistence, &session, "start-1", &fixtures)
            .expect("start should apply")
            .disposition,
        MutationDisposition::Applied
    );
    assert_eq!(
        start(&mut persistence, &session, "start-1", &fixtures)
            .expect("start should replay")
            .disposition,
        MutationDisposition::Replayed
    );
    assert_eq!(
        persistence
            .arrive_cooperation_anchor(&session, 100)
            .expect("anchor should apply")
            .disposition,
        MutationDisposition::Applied
    );
    assert_eq!(
        persistence
            .arrive_cooperation_anchor(&session, 100)
            .expect("anchor should replay")
            .disposition,
        MutationDisposition::Replayed
    );
    assert!(persistence
        .arrive_cooperation_anchor(&session, 101)
        .is_err());
    persistence
        .ping_cooperation_operation(&session, "ping-1", 200)
        .expect("ping should apply");
    assert_eq!(
        persistence
            .ping_cooperation_operation(&session, "ping-1", 300)
            .expect("ping should replay")
            .disposition,
        MutationDisposition::Replayed
    );
    assert!(persistence
        .ping_cooperation_operation(&session, "ping-2", 300)
        .is_err());
    persistence
        .down_cooperation_runner(&session, 400)
        .expect("runner down should apply");
    assert_eq!(
        persistence
            .down_cooperation_runner(&session, 400)
            .expect("runner down should replay")
            .disposition,
        MutationDisposition::Replayed
    );
    persistence
        .start_cooperation_revive(&session, "revive-1", 4, 500)
        .expect("revive should start");
    assert_eq!(
        persistence
            .observe_cooperation_revive(&session, "revive-1", 4, 600)
            .expect("early observation should remain pending")
            .disposition,
        MutationDisposition::Pending
    );
    assert_eq!(
        persistence
            .observe_cooperation_revive(&session, "revive-1", 5, 700)
            .expect("out-of-range observation should cancel")
            .disposition,
        MutationDisposition::Cancelled
    );
    persistence
        .start_cooperation_revive(&session, "revive-1", 4, 800)
        .expect("cancelled identifier should be reusable");
    let revived = persistence
        .observe_cooperation_revive(&session, "revive-1", 4, 2_800)
        .expect("revive should complete");
    assert_eq!(revived.state.phase(), Phase::EncounterActive);
    assert_eq!(
        revived.state.participants()[1].current_health,
        REVIVE_HEALTH
    );
    let completed = persistence
        .complete_cooperation_operation(&session, 3_000)
        .expect("success should commit");
    assert_eq!(completed.state.terminal(), Some(TerminalOutcome::Succeeded));
    assert_eq!(completed.state.reward(), Some([SUCCESS_REWARD; 2]));
    assert_eq!(
        persistence
            .complete_cooperation_operation(&session, 3_000)
            .expect("success should replay")
            .disposition,
        MutationDisposition::Replayed
    );

    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    let counts = cooperation_counts(&mut client, &session);
    assert_eq!(counts, vec![1, 2, 2, 2, 2, 0]);
    let rewards = client
        .query(
            "SELECT p.character_id, i.quantity, g.experience FROM \
             cooperation_operation_participants p \
             JOIN inventory_reward_grants i USING (session_id, character_id) \
             JOIN progression_reward_grants g USING (session_id, character_id) \
             WHERE p.session_id = $1 ORDER BY p.participant_index",
            &[&session],
        )
        .expect("reward rows should query");
    assert_eq!(rewards.len(), 2);
    for row in rewards {
        assert_eq!(row.get::<_, i32>(1), 2);
        assert_eq!(row.get::<_, i64>(2), 125);
    }
}

#[test]
fn every_failure_family_is_terminal_and_reward_free() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare(&database_url, "failure");
    let mut persistence = test_persistence(&database_url);
    let mut cases = Vec::new();

    let ping = format!("m28-ping-timeout-{}", unique_suffix());
    start(&mut persistence, &ping, "start", &fixtures).expect("start");
    persistence
        .arrive_cooperation_anchor(&ping, 0)
        .expect("anchor");
    persistence
        .ping_cooperation_operation(&ping, "ping", 100)
        .expect("ping");
    cases.push((
        ping.clone(),
        persistence
            .timeout_cooperation_operation(&ping, 5_101)
            .expect("ping timeout")
            .state
            .terminal(),
        TerminalOutcome::FailedPingTimeout,
    ));

    let revive = format!("m28-revive-timeout-{}", unique_suffix());
    start(&mut persistence, &revive, "start", &fixtures).expect("start");
    persistence
        .arrive_cooperation_anchor(&revive, 0)
        .expect("anchor");
    persistence
        .ping_cooperation_operation(&revive, "ping", 100)
        .expect("ping");
    persistence
        .down_cooperation_runner(&revive, 100)
        .expect("down");
    cases.push((
        revive.clone(),
        persistence
            .timeout_cooperation_operation(&revive, 15_101)
            .expect("revive timeout")
            .state
            .terminal(),
        TerminalOutcome::FailedReviveTimeout,
    ));

    let operation = format!("m28-operation-timeout-{}", unique_suffix());
    start(&mut persistence, &operation, "start", &fixtures).expect("start");
    cases.push((
        operation.clone(),
        persistence
            .timeout_cooperation_operation(&operation, 60_001)
            .expect("operation timeout")
            .state
            .terminal(),
        TerminalOutcome::FailedOperationTimeout,
    ));

    let defeat = format!("m28-defeat-{}", unique_suffix());
    start(&mut persistence, &defeat, "start", &fixtures).expect("start");
    drive_to_encounter(&mut persistence, &defeat).expect("encounter");
    cases.push((
        defeat.clone(),
        persistence
            .defeat_cooperation_participant(&defeat, Role::Runner, 3_000)
            .expect("defeat")
            .state
            .terminal(),
        TerminalOutcome::FailedParticipantDefeated,
    ));

    let abandon = format!("m28-abandon-{}", unique_suffix());
    start(&mut persistence, &abandon, "start", &fixtures).expect("start");
    cases.push((
        abandon.clone(),
        persistence
            .abandon_cooperation_operation(&abandon, Role::Anchor, 100)
            .expect("abandon")
            .state
            .terminal(),
        TerminalOutcome::AbandonedDisconnect,
    ));

    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    for (session, actual, expected) in cases {
        assert_eq!(actual, Some(expected));
        assert_eq!(
            cooperation_counts(&mut client, &session),
            vec![1, 2, 0, 0, 0, 0]
        );
    }
}

#[test]
fn replayed_arrivals_observe_active_deadlines_first() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare(&database_url, "replay_deadline");
    let mut persistence = test_persistence(&database_url);

    let anchor_session = format!("m28-anchor-replay-deadline-{}", unique_suffix());
    start(&mut persistence, &anchor_session, "start", &fixtures).expect("start");
    persistence
        .arrive_cooperation_anchor(&anchor_session, 0)
        .expect("anchor");
    let anchor_expired = persistence
        .arrive_cooperation_anchor(&anchor_session, 60_001)
        .expect("late anchor retry should persist the operation timeout");
    assert_eq!(
        anchor_expired.state.terminal(),
        Some(TerminalOutcome::FailedOperationTimeout)
    );

    let runner_session = format!("m28-runner-replay-deadline-{}", unique_suffix());
    start(&mut persistence, &runner_session, "start", &fixtures).expect("start");
    persistence
        .arrive_cooperation_anchor(&runner_session, 0)
        .expect("anchor");
    persistence
        .ping_cooperation_operation(&runner_session, "ping", 100)
        .expect("ping");
    persistence
        .down_cooperation_runner(&runner_session, 100)
        .expect("down");
    let runner_expired = persistence
        .down_cooperation_runner(&runner_session, 15_101)
        .expect("late down retry should persist the revive timeout");
    assert_eq!(
        runner_expired.state.terminal(),
        Some(TerminalOutcome::FailedReviveTimeout)
    );
}

#[test]
fn same_start_and_success_converge_across_connections() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare(&database_url, "concurrent");
    let session = format!("m28-concurrent-{}", unique_suffix());
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let database_url = database_url.clone();
            let fixtures = fixtures.clone();
            let session = session.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut persistence = test_persistence(&database_url);
                barrier.wait();
                start(&mut persistence, &session, "start", &fixtures)
                    .expect("concurrent start should converge")
            })
        })
        .collect::<Vec<_>>();
    let receipts = handles
        .into_iter()
        .map(|handle| handle.join().expect("start thread should join"))
        .collect::<Vec<_>>();
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.disposition == MutationDisposition::Applied)
            .count(),
        1
    );
    assert_eq!(receipts[0].state, receipts[1].state);

    let mut persistence = test_persistence(&database_url);
    drive_to_encounter(&mut persistence, &session).expect("encounter should persist");
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let database_url = database_url.clone();
            let session = session.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut persistence = test_persistence(&database_url);
                barrier.wait();
                persistence
                    .complete_cooperation_operation(&session, 3_000)
                    .expect("concurrent success should converge")
            })
        })
        .collect::<Vec<_>>();
    let receipts = handles
        .into_iter()
        .map(|handle| handle.join().expect("success thread should join"))
        .collect::<Vec<_>>();
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.disposition == MutationDisposition::Applied)
            .count(),
        1
    );
    assert_eq!(receipts[0].state, receipts[1].state);
    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    assert_eq!(
        cooperation_counts(&mut client, &session),
        vec![1, 2, 2, 2, 2, 0]
    );
}

#[test]
fn sql_constraints_reject_constant_phase_profile_and_terminal_drift() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare(&database_url, "constraints");
    let session = format!("m28-constraints-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    start(&mut persistence, &session, "start", &fixtures).expect("start should persist");
    for statement in [
        "UPDATE cooperation_operations SET reward_quantity = 3 WHERE session_id = $1",
        "UPDATE cooperation_operations SET phase = 'awaiting_runner' WHERE session_id = $1",
        "UPDATE cooperation_operations SET phase = 'awaiting_ping', anchor_arrived = TRUE, \
         anchor_elapsed_ms = 60001 WHERE session_id = $1",
        "UPDATE cooperation_operations SET terminal_outcome = 'succeeded', \
         terminal_elapsed_ms = 1, terminal_at = NOW() WHERE session_id = $1",
        "UPDATE cooperation_operation_participants SET role = 'runner' \
         WHERE session_id = $1 AND participant_index = 0",
        "UPDATE cooperation_operation_participants SET damage = 17 \
         WHERE session_id = $1 AND participant_index = 1",
    ] {
        let mut client = Client::connect(&database_url, NoTls).expect("constraint connection");
        assert!(
            client.execute(statement, &[&session]).is_err(),
            "constraint drift should reject: {statement}"
        );
    }
    assert_eq!(
        start(&mut persistence, &session, "start", &fixtures)
            .expect("canonical start should remain readable")
            .disposition,
        MutationDisposition::Replayed
    );

    persistence
        .arrive_cooperation_anchor(&session, 100)
        .expect("anchor should persist");
    assert_sql_rejected(
        &database_url,
        &session,
        "UPDATE cooperation_operations SET terminal_outcome = 'abandoned_disconnect', \
         terminal_elapsed_ms = 99, terminal_at = NOW() WHERE session_id = $1",
    );

    let precedence_session = format!("m28-constraints-precedence-{}", unique_suffix());
    start(&mut persistence, &precedence_session, "start", &fixtures)
        .expect("precedence start should persist");
    persistence
        .arrive_cooperation_anchor(&precedence_session, 0)
        .expect("anchor should persist");
    persistence
        .ping_cooperation_operation(&precedence_session, "ping", 100)
        .expect("ping should persist");
    assert_sql_rejected(
        &database_url,
        &precedence_session,
        "UPDATE cooperation_operations SET terminal_outcome = 'failed_operation_timeout', \
         terminal_elapsed_ms = 60001, terminal_at = NOW() WHERE session_id = $1",
    );

    let warden_session = format!("m28-constraints-warden-{}", unique_suffix());
    start(&mut persistence, &warden_session, "start", &fixtures)
        .expect("Warden constraint start should persist");
    drive_to_encounter(&mut persistence, &warden_session).expect("encounter should persist");
    assert_sql_rejected(
        &database_url,
        &warden_session,
        "UPDATE cooperation_operations SET warden_completed = TRUE, warden_elapsed_ms = 3000 \
         WHERE session_id = $1",
    );
}

fn assert_sql_rejected(database_url: &str, session: &str, statement: &str) {
    let mut client = Client::connect(database_url, NoTls).expect("constraint connection");
    assert!(
        client.execute(statement, &[&session]).is_err(),
        "constraint drift should reject: {statement}"
    );
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
        let trigger = format!("m28_fail_trigger_{label}_{suffix}");
        let function = format!("m28_fail_function_{label}_{suffix}");
        let mut client = Client::connect(database_url, NoTls).expect("failure connection");
        client
            .batch_execute(&format!(
                "CREATE FUNCTION {function}() RETURNS TRIGGER LANGUAGE plpgsql AS $m28$ \
                 BEGIN IF {predicate} THEN RAISE EXCEPTION 'm28 injected failure'; \
                 END IF; RETURN NEW; END; $m28$; \
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

fn install_commit_failure(database_url: &str, session: &str) -> FailureTrigger {
    let suffix = unique_suffix();
    let trigger = format!("m28_fail_trigger_commit_{suffix}");
    let function = format!("m28_fail_function_commit_{suffix}");
    let mut client = Client::connect(database_url, NoTls).expect("commit failure connection");
    client
        .batch_execute(&format!(
            "CREATE FUNCTION {function}() RETURNS TRIGGER LANGUAGE plpgsql AS $m28$ \
             BEGIN IF NEW.session_id = '{session}' THEN \
             RAISE EXCEPTION 'm28 injected deferred commit failure'; \
             END IF; RETURN NEW; END; $m28$; \
             CREATE CONSTRAINT TRIGGER {trigger} AFTER UPDATE ON cooperation_operations \
             DEFERRABLE INITIALLY DEFERRED FOR EACH ROW \
             EXECUTE FUNCTION {function}();"
        ))
        .expect("commit failure trigger should install");
    FailureTrigger {
        database_url: database_url.to_owned(),
        table: "cooperation_operations",
        trigger,
        function,
    }
}

#[test]
fn start_and_transition_write_boundaries_roll_back() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    for (label, table, predicate_suffix) in [
        ("start_parent", "cooperation_operations", ""),
        (
            "start_participant_zero",
            "cooperation_operation_participants",
            " AND NEW.participant_index = 0",
        ),
        (
            "start_participant_one",
            "cooperation_operation_participants",
            " AND NEW.participant_index = 1",
        ),
    ] {
        let fixtures = prepare(&database_url, label);
        let session = format!("m28-{label}-{}", unique_suffix());
        let predicate = format!("NEW.session_id = '{session}'{predicate_suffix}");
        let mut persistence = test_persistence(&database_url);
        {
            let _trigger =
                FailureTrigger::install(&database_url, label, table, "INSERT", &predicate);
            assert!(matches!(
                start(&mut persistence, &session, "start", &fixtures),
                Err(CooperationPersistenceError::Database(_))
            ));
        }
        let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
        assert_eq!(
            cooperation_counts(&mut client, &session),
            vec![0, 0, 0, 0, 0, 0]
        );
        start(&mut persistence, &session, "start", &fixtures)
            .expect("rolled-back start should remain available");
    }

    for boundary in [
        "anchor",
        "ping",
        "down_parent",
        "down_participant",
        "revive_start",
        "revive_cancel",
        "revive_complete_parent",
        "revive_complete_participant",
    ] {
        assert_transition_rollback(&database_url, boundary);
    }
}

fn assert_transition_rollback(database_url: &str, boundary: &'static str) {
    let fixtures = prepare(database_url, boundary);
    let session = format!("m28-{boundary}-{}", unique_suffix());
    let mut persistence = test_persistence(database_url);
    start(&mut persistence, &session, "start", &fixtures).expect("start should persist");
    match boundary {
        "anchor" => {}
        "ping" => {
            persistence
                .arrive_cooperation_anchor(&session, 100)
                .expect("anchor");
        }
        "down_parent" | "down_participant" => {
            persistence
                .arrive_cooperation_anchor(&session, 100)
                .expect("anchor");
            persistence
                .ping_cooperation_operation(&session, "ping", 200)
                .expect("ping");
        }
        "revive_start" => drive_to_downed(&mut persistence, &session),
        "revive_cancel" | "revive_complete_parent" | "revive_complete_participant" => {
            drive_to_downed(&mut persistence, &session);
            persistence
                .start_cooperation_revive(&session, "revive", 4, 400)
                .expect("revive start");
        }
        _ => panic!("unknown transition boundary"),
    }
    let (table, predicate) = if boundary.ends_with("participant") {
        (
            "cooperation_operation_participants",
            format!("NEW.session_id = '{session}' AND NEW.participant_index = 1"),
        )
    } else {
        (
            "cooperation_operations",
            format!("NEW.session_id = '{session}'"),
        )
    };
    let before = cooperation_state_json(database_url, &session);
    {
        let _trigger = FailureTrigger::install(database_url, boundary, table, "UPDATE", &predicate);
        let result = match boundary {
            "anchor" => persistence.arrive_cooperation_anchor(&session, 100),
            "ping" => persistence.ping_cooperation_operation(&session, "ping", 200),
            "down_parent" | "down_participant" => {
                persistence.down_cooperation_runner(&session, 300)
            }
            "revive_start" => persistence.start_cooperation_revive(&session, "revive", 4, 400),
            "revive_cancel" => persistence.observe_cooperation_revive(&session, "revive", 5, 500),
            "revive_complete_parent" | "revive_complete_participant" => {
                persistence.observe_cooperation_revive(&session, "revive", 4, 2_400)
            }
            _ => unreachable!(),
        };
        assert!(matches!(
            result,
            Err(CooperationPersistenceError::Database(_))
        ));
    }
    assert_eq!(cooperation_state_json(database_url, &session), before);
}

fn drive_to_downed(persistence: &mut Persistence, session: &str) {
    persistence
        .arrive_cooperation_anchor(session, 100)
        .expect("anchor");
    persistence
        .ping_cooperation_operation(session, "ping", 200)
        .expect("ping");
    persistence
        .down_cooperation_runner(session, 300)
        .expect("down");
}

#[test]
fn success_write_boundaries_roll_back_rewards_history_and_terminal() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    for (label, table, event, predicate_kind) in [
        (
            "reward_grant",
            "inventory_reward_grants",
            "INSERT",
            "session",
        ),
        ("inventory", "inventory", "INSERT OR UPDATE", "character"),
        (
            "progression_grant",
            "progression_reward_grants",
            "INSERT",
            "session",
        ),
        ("progression", "progression", "UPDATE", "character"),
        ("character", "characters", "UPDATE", "character_id"),
        ("history", "activity_history", "INSERT", "character"),
        ("terminal", "cooperation_operations", "UPDATE", "session"),
    ] {
        let fixtures = prepare(&database_url, label);
        let session = format!("m28-{label}-{}", unique_suffix());
        let mut persistence = test_persistence(&database_url);
        start(&mut persistence, &session, "start", &fixtures).expect("start");
        drive_to_encounter(&mut persistence, &session).expect("encounter");
        let before = cooperation_and_reward_json(&database_url, &session, &fixtures);
        let predicate = match predicate_kind {
            "session" => format!("NEW.session_id = '{session}'"),
            "character" => format!("NEW.character_id = '{}'", fixtures[0].character),
            "character_id" => format!("NEW.id = '{}'", fixtures[0].character),
            _ => unreachable!(),
        };
        {
            let _trigger = FailureTrigger::install(&database_url, label, table, event, &predicate);
            assert!(matches!(
                persistence.complete_cooperation_operation(&session, 3_000),
                Err(CooperationPersistenceError::Database(_))
            ));
        }
        assert_eq!(
            cooperation_and_reward_json(&database_url, &session, &fixtures),
            before
        );
        persistence
            .complete_cooperation_operation(&session, 3_000)
            .expect("rolled-back success should remain available");
    }

    let fixtures = prepare(&database_url, "commit");
    let session = format!("m28-commit-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    start(&mut persistence, &session, "start", &fixtures).expect("start");
    drive_to_encounter(&mut persistence, &session).expect("encounter");
    let before = cooperation_and_reward_json(&database_url, &session, &fixtures);
    {
        let _trigger = install_commit_failure(&database_url, &session);
        assert!(matches!(
            persistence.complete_cooperation_operation(&session, 3_000),
            Err(CooperationPersistenceError::Database(_))
        ));
    }
    assert_eq!(
        cooperation_and_reward_json(&database_url, &session, &fixtures),
        before
    );
    persistence
        .complete_cooperation_operation(&session, 3_000)
        .expect("commit-failed success should remain available");
}

#[test]
fn same_ping_revive_and_conflicting_start_are_serialized() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let fixtures = prepare(&database_url, "concurrent_transitions");
    let session = format!("m28-concurrent-transitions-{}", unique_suffix());
    let mut persistence = test_persistence(&database_url);
    start(&mut persistence, &session, "start", &fixtures).expect("start");
    persistence
        .arrive_cooperation_anchor(&session, 100)
        .expect("anchor");
    let ping_receipts = concurrent_transition(&database_url, &session, |persistence, session| {
        persistence.ping_cooperation_operation(session, "ping", 200)
    });
    assert_one_apply_one_replay(&ping_receipts);
    persistence
        .down_cooperation_runner(&session, 300)
        .expect("down");
    let revive_receipts = concurrent_transition(&database_url, &session, |persistence, session| {
        persistence.start_cooperation_revive(session, "revive", 4, 400)
    });
    assert_one_apply_one_replay(&revive_receipts);
    let complete_receipts =
        concurrent_transition(&database_url, &session, |persistence, session| {
            persistence.observe_cooperation_revive(session, "revive", 4, 2_400)
        });
    assert_one_apply_one_replay(&complete_receipts);

    let conflict_fixtures = prepare(&database_url, "concurrent_conflict");
    let conflict_session = format!("m28-concurrent-conflict-{}", unique_suffix());
    let barrier = Arc::new(Barrier::new(2));
    let handles = ["start-a", "start-b"].map(|operation_id| {
        let database_url = database_url.clone();
        let fixtures = conflict_fixtures.clone();
        let session = conflict_session.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            let mut persistence = test_persistence(&database_url);
            barrier.wait();
            start(&mut persistence, &session, operation_id, &fixtures)
        })
    });
    let results = handles.map(|handle| handle.join().expect("conflict thread should join"));
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
}

fn concurrent_transition<F>(
    database_url: &str,
    session: &str,
    operation: F,
) -> [revenant_persistence::CooperationReceipt; 2]
where
    F: Fn(
            &mut Persistence,
            &str,
        ) -> Result<revenant_persistence::CooperationReceipt, CooperationPersistenceError>
        + Send
        + Sync
        + 'static,
{
    let barrier = Arc::new(Barrier::new(2));
    let operation = Arc::new(operation);
    let handles = (0..2)
        .map(|_| {
            let database_url = database_url.to_owned();
            let session = session.to_owned();
            let barrier = Arc::clone(&barrier);
            let operation = Arc::clone(&operation);
            thread::spawn(move || {
                let mut persistence = test_persistence(&database_url);
                barrier.wait();
                operation(&mut persistence, &session)
                    .expect("concurrent transition should converge")
            })
        })
        .collect::<Vec<_>>();
    let receipts = handles
        .into_iter()
        .map(|handle| handle.join().expect("transition thread should join"))
        .collect::<Vec<_>>();
    receipts.try_into().expect("exactly two receipts")
}

fn assert_one_apply_one_replay(receipts: &[revenant_persistence::CooperationReceipt; 2]) {
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.disposition == MutationDisposition::Applied)
            .count(),
        1
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.disposition == MutationDisposition::Replayed)
            .count(),
        1
    );
    assert_eq!(receipts[0].state, receipts[1].state);
}

fn cooperation_state_json(database_url: &str, session: &str) -> String {
    let mut client = Client::connect(database_url, NoTls).expect("state connection");
    client
        .query_one(
            "SELECT jsonb_build_object(\
                'operation', (SELECT to_jsonb(o) - 'accepted_at' - 'terminal_at' \
                    FROM cooperation_operations o WHERE session_id = $1), \
                'participants', (SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_index) \
                    FROM cooperation_operation_participants p WHERE session_id = $1)\
             )::TEXT",
            &[&session],
        )
        .expect("state snapshot should query")
        .get(0)
}

fn cooperation_and_reward_json(
    database_url: &str,
    session: &str,
    fixtures: &[Fixture; 2],
) -> String {
    let mut client = Client::connect(database_url, NoTls).expect("reward connection");
    client
        .query_one(
            "SELECT jsonb_build_object(\
                'cooperation', (SELECT to_jsonb(o) - 'accepted_at' - 'terminal_at' \
                    FROM cooperation_operations o WHERE session_id = $1), \
                'participants', (SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_index) \
                    FROM cooperation_operation_participants p WHERE session_id = $1), \
                'inventory', (SELECT jsonb_agg(to_jsonb(i) ORDER BY character_id, item_id) \
                    FROM inventory i WHERE character_id IN ($2, $3)), \
                'progression', (SELECT jsonb_agg(to_jsonb(p) ORDER BY character_id) \
                    FROM progression p WHERE character_id IN ($2, $3)), \
                'levels', (SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM characters c \
                    WHERE id IN ($2, $3)), \
                'inventory_grants', (SELECT jsonb_agg(to_jsonb(g) - 'granted_at' \
                    ORDER BY character_id) FROM inventory_reward_grants g WHERE session_id = $1), \
                'progression_grants', (SELECT jsonb_agg(to_jsonb(g) - 'granted_at' \
                    ORDER BY character_id) FROM progression_reward_grants g WHERE session_id = $1), \
                'history', (SELECT jsonb_agg(to_jsonb(h) - 'completed_at' ORDER BY id) \
                    FROM activity_history h WHERE character_id IN ($2, $3))\
             )::TEXT",
            &[&session, &fixtures[0].character, &fixtures[1].character],
        )
        .expect("reward snapshot should query")
        .get(0)
}

fn cooperation_counts(client: &mut Client, session: &str) -> Vec<i64> {
    let row = client
        .query_one(
            "SELECT \
             (SELECT COUNT(*) FROM cooperation_operations WHERE session_id = $1), \
             (SELECT COUNT(*) FROM cooperation_operation_participants WHERE session_id = $1), \
             (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), \
             (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1), \
             (SELECT COUNT(*) FROM activity_history WHERE character_id IN (\
                 SELECT character_id FROM cooperation_operation_participants WHERE session_id = $1)), \
             (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 AND event_type LIKE 'cooperation_%')",
            &[&session],
        )
        .expect("cooperation counts should query");
    (0..6).map(|index| row.get(index)).collect()
}

#[test]
fn frozen_reward_identity_remains_the_existing_fragment() {
    assert_eq!(RELAY_CORE_FRAGMENT, "relay_core_fragment");
}
