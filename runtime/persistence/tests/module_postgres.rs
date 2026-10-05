use std::env;
use std::sync::{Arc, Barrier, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use postgres::{Client, NoTls};
use revenant_modules::{ActivityPhase, ModuleId, ModuleMutationError, OperationDisposition};
use revenant_persistence::{ModulePersistenceError, PersistedModuleState, Persistence};

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

fn prepare_character(database_url: &str, label: &str, fragments: i32) -> (String, String) {
    let account_id = format!("local:m26-{label}-{}", unique_suffix());
    let character_id = format!("{account_id}:operator");
    let mut persistence = test_persistence(database_url);
    persistence
        .ensure_local_account(&account_id, label)
        .expect("module test account should persist");
    let mut client = Client::connect(database_url, NoTls).expect("admin connection should open");
    client
        .execute(
            "INSERT INTO inventory (character_id, item_id, quantity) \
             VALUES ($1, 'relay_core_fragment', $2) \
             ON CONFLICT (character_id, item_id) DO UPDATE SET quantity = EXCLUDED.quantity",
            &[&character_id, &fragments],
        )
        .expect("fragment fixture should persist");
    (account_id, character_id)
}

#[test]
fn module_transitions_persist_retry_and_reconnect_exactly() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (account_id, character_id) = prepare_character(&database_url, "transitions", 8);
    let mut persistence = test_persistence(&database_url);
    assert_eq!(
        persistence
            .module_state_for(&character_id)
            .expect("legacy-compatible state should load"),
        PersistedModuleState {
            fragments: 8,
            owned_modules: Vec::new(),
            loadout: Vec::new(),
            revision: 0,
            combination_operations: 0,
            loadout_operations: 0,
        }
    );

    exercise_persisted_combinations(&mut persistence, &character_id);
    exercise_persisted_loadouts(&mut persistence, &character_id);
    drop(persistence);
    assert_reconnected_module_state(&database_url, &account_id, &character_id);
}

fn exercise_persisted_combinations(persistence: &mut Persistence, character_id: &str) {
    let locked = persistence
        .combine_module(
            character_id,
            ActivityPhase::Active,
            "force-op",
            ModuleId::ForceMatrix,
        )
        .expect_err("active combination should be rejected");
    assert!(matches!(
        locked,
        ModulePersistenceError::Domain(ModuleMutationError::MutationLocked(ActivityPhase::Active))
    ));
    let applied = persistence
        .combine_module(
            character_id,
            ActivityPhase::Complete,
            "force-op",
            ModuleId::ForceMatrix,
        )
        .expect("rejected operation identifier should remain unreserved");
    assert_eq!(applied.disposition, OperationDisposition::Applied);
    assert_eq!(
        (
            applied.outcome.previous_fragments,
            applied.outcome.resulting_fragments,
        ),
        (8, 6)
    );
    let replayed = persistence
        .combine_module(
            character_id,
            ActivityPhase::Active,
            "force-op",
            ModuleId::ForceMatrix,
        )
        .expect("accepted retry should load its durable result");
    assert_eq!(replayed.disposition, OperationDisposition::Replayed);
    assert_eq!(replayed.outcome, applied.outcome);
    assert!(matches!(
        persistence.combine_module(
            character_id,
            ActivityPhase::Complete,
            "force-op",
            ModuleId::TempoRegulator,
        ),
        Err(ModulePersistenceError::Domain(
            ModuleMutationError::IdempotencyConflict
        ))
    ));

    for (operation_id, module_id) in [
        ("tempo-op", ModuleId::TempoRegulator),
        ("reach-op", ModuleId::ReachLattice),
        ("ward-op", ModuleId::WardCapacitor),
    ] {
        let receipt = persistence
            .combine_module(
                character_id,
                ActivityPhase::Complete,
                operation_id,
                module_id,
            )
            .expect("remaining module should combine");
        assert_eq!(receipt.disposition, OperationDisposition::Applied);
    }
}

fn exercise_persisted_loadouts(persistence: &mut Persistence, character_id: &str) {
    let noncanonical = [
        ModuleId::ReachLattice,
        ModuleId::ForceMatrix,
        ModuleId::TempoRegulator,
    ];
    let canonical_retry = [
        ModuleId::TempoRegulator,
        ModuleId::ReachLattice,
        ModuleId::ForceMatrix,
    ];
    let loadout = persistence
        .set_module_loadout(
            character_id,
            ActivityPhase::Complete,
            "loadout-op",
            0,
            &noncanonical,
        )
        .expect("whole loadout should persist");
    assert_eq!(loadout.disposition, OperationDisposition::Applied);
    assert_eq!(loadout.outcome.resulting_revision, 1);
    let loadout_retry = persistence
        .set_module_loadout(
            character_id,
            ActivityPhase::Active,
            "loadout-op",
            0,
            &canonical_retry,
        )
        .expect("reordered accepted loadout should replay");
    assert_eq!(loadout_retry.disposition, OperationDisposition::Replayed);
    assert_eq!(loadout_retry.outcome, loadout.outcome);
    assert!(matches!(
        persistence.set_module_loadout(
            character_id,
            ActivityPhase::Complete,
            "loadout-op",
            0,
            &[ModuleId::WardCapacitor],
        ),
        Err(ModulePersistenceError::Domain(
            ModuleMutationError::IdempotencyConflict
        ))
    ));

    assert!(matches!(
        persistence.set_module_loadout(
            character_id,
            ActivityPhase::Complete,
            "stale-op",
            0,
            &[ModuleId::WardCapacitor],
        ),
        Err(ModulePersistenceError::Domain(
            ModuleMutationError::StaleRevision {
                expected: 0,
                actual: 1
            }
        ))
    ));
    let changed = persistence
        .set_module_loadout(
            character_id,
            ActivityPhase::Complete,
            "stale-op",
            1,
            &[ModuleId::WardCapacitor],
        )
        .expect("rejected operation identifier should remain unreserved");
    assert_eq!(changed.outcome.resulting_revision, 2);
}

fn assert_reconnected_module_state(database_url: &str, account_id: &str, character_id: &str) {
    let mut reconnected = Persistence::connect_existing(database_url)
        .expect("fresh persistence connection should open");
    let state = reconnected
        .module_state_for(character_id)
        .expect("module state should survive reconnect");
    assert_eq!(state.fragments, 0);
    assert_eq!(state.owned_modules, ModuleId::LEGACY);
    assert_eq!(state.loadout, vec![ModuleId::WardCapacitor]);
    assert_eq!(state.revision, 2);
    assert_eq!(state.combination_operations, 4);
    assert_eq!(state.loadout_operations, 2);

    let mut client =
        Client::connect(database_url, NoTls).expect("assertion connection should open");
    let counts = client
        .query_one(
            "SELECT \
                 (SELECT COUNT(*) FROM inventory WHERE character_id = $1 AND item_id LIKE 'module_%' AND quantity = 1), \
                 (SELECT COUNT(*) FROM module_loadout_slots WHERE character_id = $1), \
                 (SELECT COUNT(*) FROM module_operations WHERE character_id = $1), \
                 (SELECT COUNT(*) FROM replay_events WHERE account_id = $2)",
            &[&character_id, &account_id],
        )
        .expect("module counts should query");
    assert_eq!(
        (
            counts.get::<_, i64>(0),
            counts.get::<_, i64>(1),
            counts.get::<_, i64>(2),
            counts.get::<_, i64>(3),
        ),
        (4, 1, 6, 0)
    );
}

#[test]
fn concurrent_same_operation_consumes_and_grants_once() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (_, character_id) = prepare_character(&database_url, "concurrent", 2);
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let database_url = database_url.clone();
            let character_id = character_id.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut persistence = Persistence::connect_existing(&database_url)
                    .expect("concurrent persistence should connect");
                barrier.wait();
                persistence
                    .combine_module(
                        &character_id,
                        ActivityPhase::Complete,
                        "same-operation",
                        ModuleId::ForceMatrix,
                    )
                    .expect("same operation should apply or replay")
            })
        })
        .collect::<Vec<_>>();
    let receipts = handles
        .into_iter()
        .map(|handle| handle.join().expect("concurrent thread should finish"))
        .collect::<Vec<_>>();
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.disposition == OperationDisposition::Applied)
            .count(),
        1
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| receipt.disposition == OperationDisposition::Replayed)
            .count(),
        1
    );
    assert_eq!(receipts[0].outcome, receipts[1].outcome);
    let mut persistence =
        Persistence::connect_existing(&database_url).expect("assertion persistence should connect");
    let state = persistence
        .module_state_for(&character_id)
        .expect("concurrent state should load");
    assert_eq!(state.fragments, 0);
    assert_eq!(state.owned_modules, vec![ModuleId::ForceMatrix]);
    assert_eq!(state.combination_operations, 1);
}

#[test]
fn persisted_operation_limit_rejects_new_but_replays_old() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (_, character_id) = prepare_character(&database_url, "operation-limit", 2);
    let mut persistence = test_persistence(&database_url);
    persistence
        .combine_module(
            &character_id,
            ActivityPhase::Complete,
            "prepare-module",
            ModuleId::ForceMatrix,
        )
        .expect("operation-limit module should combine");
    for operation in 0..128 {
        let modules = if operation % 2 == 0 {
            vec![ModuleId::ForceMatrix]
        } else {
            Vec::new()
        };
        persistence
            .set_module_loadout(
                &character_id,
                ActivityPhase::Complete,
                &format!("limit-{operation}"),
                operation,
                &modules,
            )
            .expect("bounded persisted loadout should apply");
    }
    let before = persistence
        .module_state_for(&character_id)
        .expect("bounded persisted state should load");
    assert_eq!((before.revision, before.loadout_operations), (128, 128));
    assert!(matches!(
        persistence.set_module_loadout(
            &character_id,
            ActivityPhase::Complete,
            "limit-new",
            128,
            &[ModuleId::ForceMatrix],
        ),
        Err(ModulePersistenceError::Domain(
            ModuleMutationError::OperationLimitReached
        ))
    ));
    assert_eq!(
        persistence
            .module_state_for(&character_id)
            .expect("limit rejection should preserve state"),
        before
    );
    let replayed = persistence
        .set_module_loadout(
            &character_id,
            ActivityPhase::Active,
            "limit-0",
            0,
            &[ModuleId::ForceMatrix],
        )
        .expect("stored operation should replay after the ledger reaches its limit");
    assert_eq!(replayed.disposition, OperationDisposition::Replayed);
    assert_eq!(replayed.outcome.resulting_revision, 1);
}

struct FailureTrigger {
    database_url: String,
    table: &'static str,
    trigger_name: String,
    function_name: String,
}

impl FailureTrigger {
    fn install(
        database_url: &str,
        label: &str,
        table: &'static str,
        event: &'static str,
        predicate: &str,
        row_reference: &'static str,
    ) -> Self {
        let suffix = unique_suffix();
        let safe_label = label.replace('-', "_");
        let trigger_name = format!("m26_fail_trigger_{safe_label}_{suffix}");
        let function_name = format!("m26_fail_function_{safe_label}_{suffix}");
        let mut client =
            Client::connect(database_url, NoTls).expect("failure trigger connection should open");
        client
            .batch_execute(&format!(
                "CREATE FUNCTION {function_name}() RETURNS TRIGGER LANGUAGE plpgsql AS $m26$ \
                 BEGIN IF {predicate} THEN RAISE EXCEPTION 'm26 injected {label} failure'; \
                 END IF; RETURN {row_reference}; END; $m26$; \
                 CREATE TRIGGER {trigger_name} BEFORE {event} ON {table} \
                 FOR EACH ROW EXECUTE FUNCTION {function_name}();"
            ))
            .expect("failure trigger should install");
        Self {
            database_url: database_url.to_owned(),
            table,
            trigger_name,
            function_name,
        }
    }
}

impl Drop for FailureTrigger {
    fn drop(&mut self) {
        if let Ok(mut client) = Client::connect(&self.database_url, NoTls) {
            let _ = client.batch_execute(&format!(
                "DROP TRIGGER IF EXISTS {} ON {}; DROP FUNCTION IF EXISTS {}();",
                self.trigger_name, self.table, self.function_name
            ));
        }
    }
}

fn assert_database_failure(error: &ModulePersistenceError) {
    assert!(matches!(error, ModulePersistenceError::Database(_)));
}

fn assert_combination_failure_boundary(
    database_url: &str,
    label: &str,
    table: &'static str,
    event: &'static str,
    predicate_suffix: &str,
) {
    let (_, character_id) = prepare_character(database_url, label, 2);
    let mut persistence = test_persistence(database_url);
    let before = persistence
        .module_state_for(&character_id)
        .expect("pre-failure state should load");
    let predicate = format!("NEW.character_id = '{character_id}' {predicate_suffix}");
    let _trigger = FailureTrigger::install(database_url, label, table, event, &predicate, "NEW");
    let error = persistence
        .combine_module(
            &character_id,
            ActivityPhase::Complete,
            "failure-op",
            ModuleId::ForceMatrix,
        )
        .expect_err("injected combination should fail");
    assert_database_failure(&error);
    assert_eq!(
        persistence
            .module_state_for(&character_id)
            .expect("post-failure state should load"),
        before
    );
}

#[test]
fn combination_rolls_back_at_every_write_boundary() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    for (label, table, event, suffix) in [
        ("state-insert", "module_states", "INSERT", ""),
        (
            "fragment-update",
            "inventory",
            "UPDATE",
            "AND NEW.item_id = 'relay_core_fragment'",
        ),
        (
            "module-insert",
            "inventory",
            "INSERT",
            "AND NEW.item_id = 'module_force_matrix'",
        ),
        (
            "operation-insert",
            "module_operations",
            "INSERT",
            "AND NEW.operation_kind = 'combine'",
        ),
    ] {
        assert_combination_failure_boundary(&database_url, label, table, event, suffix);
    }
}

fn assert_loadout_failure_boundary(
    database_url: &str,
    label: &str,
    table: &'static str,
    event: &'static str,
    predicate_suffix: &str,
) {
    let (_, character_id) = prepare_character(database_url, label, 2);
    let mut persistence = test_persistence(database_url);
    persistence
        .combine_module(
            &character_id,
            ActivityPhase::Complete,
            "prepare-module",
            ModuleId::ForceMatrix,
        )
        .expect("loadout fixture module should combine");
    let before = persistence
        .module_state_for(&character_id)
        .expect("pre-failure loadout should load");
    let predicate = format!("NEW.character_id = '{character_id}' {predicate_suffix}");
    let _trigger = FailureTrigger::install(database_url, label, table, event, &predicate, "NEW");
    let error = persistence
        .set_module_loadout(
            &character_id,
            ActivityPhase::Complete,
            "failure-loadout",
            0,
            &[ModuleId::ForceMatrix],
        )
        .expect_err("injected loadout should fail");
    assert_database_failure(&error);
    assert_eq!(
        persistence
            .module_state_for(&character_id)
            .expect("post-failure loadout should load"),
        before
    );
}

#[test]
fn loadout_rolls_back_at_insert_update_and_operation_boundaries() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    for (label, table, event, suffix) in [
        ("slot-insert", "module_loadout_slots", "INSERT", ""),
        ("revision-update", "module_states", "UPDATE", ""),
        (
            "loadout-operation",
            "module_operations",
            "INSERT",
            "AND NEW.operation_kind = 'loadout'",
        ),
    ] {
        assert_loadout_failure_boundary(&database_url, label, table, event, suffix);
    }
}

#[test]
fn loadout_rolls_back_at_slot_delete_boundary() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (_, character_id) = prepare_character(&database_url, "slot-delete", 2);
    let mut persistence = test_persistence(&database_url);
    persistence
        .combine_module(
            &character_id,
            ActivityPhase::Complete,
            "prepare-module",
            ModuleId::ForceMatrix,
        )
        .expect("delete fixture module should combine");
    persistence
        .set_module_loadout(
            &character_id,
            ActivityPhase::Complete,
            "prepare-loadout",
            0,
            &[ModuleId::ForceMatrix],
        )
        .expect("delete fixture loadout should persist");
    let before = persistence
        .module_state_for(&character_id)
        .expect("pre-delete state should load");
    let predicate = format!("OLD.character_id = '{character_id}'");
    let _trigger = FailureTrigger::install(
        &database_url,
        "slot-delete",
        "module_loadout_slots",
        "DELETE",
        &predicate,
        "OLD",
    );
    let error = persistence
        .set_module_loadout(
            &character_id,
            ActivityPhase::Complete,
            "delete-failure",
            1,
            &[],
        )
        .expect_err("injected slot delete should fail");
    assert_database_failure(&error);
    assert_eq!(
        persistence
            .module_state_for(&character_id)
            .expect("post-delete state should load"),
        before
    );
}
