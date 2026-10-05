use std::env;
use std::sync::{Arc, Barrier, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use postgres::{Client, NoTls};
use revenant_modules::{ActivityPhase, ModuleId, OperationDisposition};
use revenant_persistence::{
    ModuleJoinReplayContext, ModuleMutationReplayContext, NewReplayEvent, Persistence,
};
use revenant_replay::{
    reconstruct, ReplayEvent, ReplayEventKind, ReplayProtocolGeneration,
    MAX_MODULE_REPLAY_PAYLOAD_BYTES,
};

const ACTIVITY_ID: &str = "relay_awakening";

#[test]
fn older_arsenal_retains_new_inventory_while_recording_its_own_catalog() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    let (account, character) = prepare_character(&url, "m35-older", 4);
    let mut database = test_persistence(&url);
    database
        .combine_module(
            &character,
            ActivityPhase::Complete,
            "new-module",
            ModuleId::BreachShunt,
        )
        .unwrap();
    let session = session_id("m35-older");
    database
        .append_player_joined_with_module_snapshot(
            &character,
            ModuleJoinReplayContext {
                catalog_revision: Some("m35-v1"),
                ..join_context(&session, &account, ReplayProtocolGeneration::V2)
            },
        )
        .unwrap();
    append_completion(&mut database, &session, &account);
    let replay = ModuleMutationReplayContext {
        catalog_revision: Some("m35-v1"),
        ..mutation_context(&session, &account)
    };
    database
        .combine_module_with_replay(
            &character,
            ActivityPhase::Complete,
            "old-module",
            ModuleId::ForceMatrix,
            replay,
        )
        .unwrap();
    database
        .set_module_loadout_with_replay(
            &character,
            ActivityPhase::Complete,
            "old-loadout",
            0,
            &[ModuleId::ForceMatrix],
            replay,
        )
        .unwrap();
    let reconstructed = reconstructed_session(&mut database, &session);
    let evidence = &reconstructed.module_participants[0].state;
    assert_eq!(evidence.catalog_revision, "m35-v1");
    assert_eq!(evidence.owned_modules, [ModuleId::ForceMatrix]);
    assert_eq!(evidence.persisted_loadout, [ModuleId::ForceMatrix]);
    let saved = database.module_state_for(&character).unwrap();
    assert_eq!(
        saved.owned_modules,
        [ModuleId::ForceMatrix, ModuleId::BreachShunt]
    );
    assert_eq!(saved.fragments, 1);
    let frozen = session_id("m35-frozen");
    database
        .append_player_joined_with_module_snapshot(
            &character,
            join_context(&frozen, &account, ReplayProtocolGeneration::V1),
        )
        .unwrap();
    let reconstructed = reconstructed_session(&mut database, &frozen);
    let evidence = &reconstructed.module_participants[0].state;
    assert!(!evidence.module_effects_active);
    assert!(evidence.applied_loadout.is_empty());
    assert_eq!(evidence.owned_modules, saved.owned_modules);
    assert_eq!(evidence.weapons[0].effective.damage, 40);
}

#[test]
fn m35_build_modules_retry_reconnect_and_reconstruct_without_double_spend() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    let (account, character) = prepare_character(&url, "m35-build", 4);
    let session = session_id("m35-build");
    let mut database = test_persistence(&url);
    database
        .append_player_joined_with_module_snapshot(
            &character,
            ModuleJoinReplayContext {
                catalog_revision: Some("m35-v2"),
                ..join_context(&session, &account, ReplayProtocolGeneration::V2)
            },
        )
        .unwrap();
    append_completion(&mut database, &session, &account);
    let replay = ModuleMutationReplayContext {
        catalog_revision: Some("m35-v2"),
        ..mutation_context(&session, &account)
    };
    for (index, module) in [
        ModuleId::BreachShunt,
        ModuleId::StandoffOptic,
        ModuleId::SkirmishDrive,
        ModuleId::AblativeShell,
    ]
    .into_iter()
    .enumerate()
    {
        let operation = format!("build-combine-{index}");
        let receipt = database
            .combine_module_with_replay(
                &character,
                ActivityPhase::Complete,
                &operation,
                module,
                replay,
            )
            .unwrap();
        assert_eq!(receipt.disposition, OperationDisposition::Applied);
        let retry = database
            .combine_module_with_replay(
                &character,
                ActivityPhase::Active,
                &operation,
                module,
                replay,
            )
            .unwrap();
        assert_eq!(retry.disposition, OperationDisposition::Replayed);
        assert_eq!(receipt.outcome, retry.outcome);
    }
    let modules = [ModuleId::StandoffOptic, ModuleId::SkirmishDrive];
    database
        .set_module_loadout_with_replay(
            &character,
            ActivityPhase::Complete,
            "build-loadout",
            0,
            &modules,
            replay,
        )
        .unwrap();
    assert!(database
        .set_module_loadout_with_replay(
            &character,
            ActivityPhase::Complete,
            "build-stale",
            0,
            &[],
            replay
        )
        .is_err());
    assert_eq!(
        database
            .set_module_loadout_with_replay(
                &character,
                ActivityPhase::Active,
                "build-loadout",
                0,
                &modules,
                replay
            )
            .unwrap()
            .disposition,
        OperationDisposition::Replayed
    );
    drop(database);
    assert_m35_saved_build(&url, &character, &session, &modules);
}

fn assert_m35_saved_build(url: &str, character: &str, session: &str, modules: &[ModuleId]) {
    let mut database = Persistence::connect_existing(url).unwrap();
    let saved = database.module_state_for(character).unwrap();
    assert_eq!(saved.fragments, 0);
    assert_eq!(saved.owned_modules.len(), 4);
    assert_eq!(saved.loadout, modules);
    assert_eq!(saved.revision, 1);
    let reconstructed = reconstructed_session(&mut database, session);
    assert_eq!(reconstructed.module_combinations, 4);
    assert_eq!(reconstructed.module_loadout_changes, 1);
    let state = &reconstructed.module_participants[0].state;
    assert_eq!(state.catalog_revision, "m35-v2");
    assert_eq!(state.persisted_loadout, modules);
    assert_eq!(state.weapons.len(), 5);
    assert_eq!(
        state
            .weapons
            .iter()
            .find(|w| w.item_id == "rail_driver")
            .unwrap()
            .effective
            .range,
        13
    );
}

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
    let account_id = format!("local:m26-replay-{label}-{}", unique_suffix());
    let character_id = format!("{account_id}:operator");
    let mut persistence = test_persistence(database_url);
    persistence
        .ensure_local_account(&account_id, label)
        .expect("module replay account should persist");
    let mut client = Client::connect(database_url, NoTls).expect("fixture connection should open");
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

fn session_id(label: &str) -> String {
    format!("session-m26-{label}-{}", unique_suffix())
}

fn join_context<'a>(
    session_id: &'a str,
    account_id: &'a str,
    generation: ReplayProtocolGeneration,
) -> ModuleJoinReplayContext<'a> {
    ModuleJoinReplayContext {
        catalog_revision: None,
        session_id,
        account_id,
        activity_id: ACTIVITY_ID,
        actor_id: 41,
        player_joined_payload: "player joined as actor 41",
        protocol_generation: generation,
    }
}

fn mutation_context<'a>(
    session_id: &'a str,
    account_id: &'a str,
) -> ModuleMutationReplayContext<'a> {
    ModuleMutationReplayContext {
        catalog_revision: None,
        session_id,
        account_id,
        activity_id: ACTIVITY_ID,
        actor_id: 41,
    }
}

fn append_completion(persistence: &mut Persistence, session_id: &str, account_id: &str) {
    persistence
        .append_replay_event(&NewReplayEvent {
            event_type: "activity_completed",
            session_id,
            account_id,
            activity_id: Some(ACTIVITY_ID),
            actor_id: None,
            payload: "activity completed",
        })
        .expect("completion replay should persist");
}

fn reconstructed_session(
    persistence: &mut Persistence,
    session_id: &str,
) -> revenant_replay::ReconstructedSession {
    let events = persistence
        .replay_events(session_id)
        .expect("module replay events should load")
        .into_iter()
        .map(|event| ReplayEvent {
            id: event.id,
            kind: event
                .event_type
                .parse::<ReplayEventKind>()
                .expect("persisted event kind should be known"),
            timestamp: event.timestamp,
            session_id: event.session_id,
            account_id: event.account_id,
            activity_id: event.activity_id,
            actor_id: event
                .actor_id
                .map(u64::try_from)
                .transpose()
                .expect("test actor should be positive"),
            payload: event.payload,
        })
        .collect::<Vec<_>>();
    reconstruct(&events).expect("persisted M26 session should reconstruct")
}

#[test]
fn snapshot_mutations_retries_and_reconnect_reconstruct_exactly() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (account_id, character_id) = prepare_character(&database_url, "exact", 4);
    let session_id = session_id("exact");
    let mut persistence = test_persistence(&database_url);
    let (joined_id, snapshot_id, snapshotted_state) = persistence
        .append_player_joined_with_module_snapshot(
            &character_id,
            join_context(&session_id, &account_id, ReplayProtocolGeneration::V2),
        )
        .expect("join and snapshot should commit");
    assert!(snapshot_id > joined_id);
    assert_eq!(snapshotted_state.fragments, 4);
    append_completion(&mut persistence, &session_id, &account_id);

    let replay = mutation_context(&session_id, &account_id);
    let combined = persistence
        .combine_module_with_replay(
            &character_id,
            ActivityPhase::Complete,
            "combine-exact",
            ModuleId::ForceMatrix,
            replay,
        )
        .expect("combination and replay should commit");
    assert_eq!(combined.disposition, OperationDisposition::Applied);
    let loadout = persistence
        .set_module_loadout_with_replay(
            &character_id,
            ActivityPhase::Complete,
            "loadout-exact",
            0,
            &[ModuleId::ForceMatrix],
            replay,
        )
        .expect("loadout and replay should commit");
    assert_eq!(loadout.disposition, OperationDisposition::Applied);
    assert_eq!(
        persistence
            .combine_module_with_replay(
                &character_id,
                ActivityPhase::Active,
                "combine-exact",
                ModuleId::ForceMatrix,
                replay,
            )
            .expect("durable combination should replay")
            .disposition,
        OperationDisposition::Replayed
    );
    assert_eq!(
        persistence
            .set_module_loadout_with_replay(
                &character_id,
                ActivityPhase::Active,
                "loadout-exact",
                0,
                &[ModuleId::ForceMatrix],
                replay,
            )
            .expect("durable loadout should replay")
            .disposition,
        OperationDisposition::Replayed
    );
    drop(persistence);

    assert_exact_replay(&database_url, &session_id);
}

fn assert_exact_replay(database_url: &str, session_id: &str) {
    let mut reconnected = Persistence::connect_existing(database_url)
        .expect("fresh replay assertion connection should open");
    let events = reconnected
        .replay_events(session_id)
        .expect("exact replay should load");
    assert_eq!(events.len(), 5);
    assert_eq!(
        events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "player_joined",
            "module_state_snapshot",
            "activity_completed",
            "module_combined",
            "module_loadout_changed",
        ]
    );
    assert!(events
        .iter()
        .filter(|event| event.event_type.starts_with("module_"))
        .all(|event| event.payload.len() <= MAX_MODULE_REPLAY_PAYLOAD_BYTES));
    let reconstructed = reconstructed_session(&mut reconnected, session_id);
    assert_eq!(reconstructed.module_snapshots, 1);
    assert_eq!(reconstructed.module_combinations, 1);
    assert_eq!(reconstructed.module_loadout_changes, 1);
    assert_eq!(
        reconstructed.module_participants[0].state.persisted_loadout,
        [ModuleId::ForceMatrix]
    );
    let summary = reconnected
        .authoritative_session_summary(session_id)
        .expect("module summary should validate")
        .expect("module summary should exist");
    assert!(!summary.module_replay_legacy);
    assert_eq!(summary.module_participant_count, 1);
    assert_eq!(summary.module_snapshot_count, 1);
    assert_eq!(summary.module_combination_count, 1);
    assert_eq!(summary.module_loadout_change_count, 1);
}

#[test]
fn frozen_v1_snapshot_keeps_persisted_module_inactive() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (account_id, character_id) = prepare_character(&database_url, "v1", 1);
    let mut client = Client::connect(&database_url, NoTls).expect("fixture connection should open");
    client
        .batch_execute(&format!(
            "INSERT INTO inventory (character_id, item_id, quantity) \
             VALUES ('{character_id}', 'module_force_matrix', 1); \
             INSERT INTO module_states (character_id, revision) VALUES ('{character_id}', 1); \
             INSERT INTO module_loadout_slots (character_id, slot_index, module_item_id) \
             VALUES ('{character_id}', 0, 'module_force_matrix');"
        ))
        .expect("persisted V2 module fixture should insert");
    let session_id = session_id("v1");
    let mut persistence = test_persistence(&database_url);
    persistence
        .append_player_joined_with_module_snapshot(
            &character_id,
            join_context(&session_id, &account_id, ReplayProtocolGeneration::V1),
        )
        .expect("V1 join snapshot should commit");
    let reconstructed = reconstructed_session(&mut persistence, &session_id);
    let participant = &reconstructed.module_participants[0];
    assert_eq!(
        participant.protocol_generation,
        ReplayProtocolGeneration::V1
    );
    assert_eq!(participant.state.persisted_loadout, [ModuleId::ForceMatrix]);
    assert!(participant.state.applied_loadout.is_empty());
    assert!(!participant.state.module_effects_active);
}

struct FailureTrigger {
    database_url: String,
    trigger_name: String,
    function_name: String,
}

impl FailureTrigger {
    fn install(database_url: &str, session_id: &str, event_type: &str) -> Self {
        let suffix = unique_suffix();
        let trigger_name = format!("m26_replay_fail_trigger_{suffix}");
        let function_name = format!("m26_replay_fail_function_{suffix}");
        let mut client =
            Client::connect(database_url, NoTls).expect("failure trigger connection should open");
        client
            .batch_execute(&format!(
                "CREATE FUNCTION {function_name}() RETURNS TRIGGER LANGUAGE plpgsql AS $m26$ \
                 BEGIN IF NEW.session_id = '{session_id}' AND NEW.event_type = '{event_type}' \
                 THEN RAISE EXCEPTION 'm26 injected replay failure'; END IF; RETURN NEW; END; $m26$; \
                 CREATE TRIGGER {trigger_name} BEFORE INSERT ON replay_events \
                 FOR EACH ROW EXECUTE FUNCTION {function_name}();"
            ))
            .expect("replay failure trigger should install");
        Self {
            database_url: database_url.to_owned(),
            trigger_name,
            function_name,
        }
    }
}

impl Drop for FailureTrigger {
    fn drop(&mut self) {
        if let Ok(mut client) = Client::connect(&self.database_url, NoTls) {
            let _ = client.batch_execute(&format!(
                "DROP TRIGGER IF EXISTS {} ON replay_events; DROP FUNCTION IF EXISTS {}();",
                self.trigger_name, self.function_name
            ));
        }
    }
}

#[test]
fn join_snapshot_failure_exposes_neither_event() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (account_id, character_id) = prepare_character(&database_url, "join-failure", 2);
    let session_id = session_id("join-failure");
    let mut persistence = test_persistence(&database_url);
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "module_state_snapshot");
        assert!(persistence
            .append_player_joined_with_module_snapshot(
                &character_id,
                join_context(&session_id, &account_id, ReplayProtocolGeneration::V2),
            )
            .is_err());
    }
    assert!(persistence
        .replay_events(&session_id)
        .expect("failed join replay should query")
        .is_empty());
    persistence
        .append_player_joined_with_module_snapshot(
            &character_id,
            join_context(&session_id, &account_id, ReplayProtocolGeneration::V2),
        )
        .expect("same join should succeed after failure fixture removal");
}

#[test]
fn mutation_replay_failures_roll_back_and_leave_ids_unreserved() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (account_id, character_id) = prepare_character(&database_url, "mutation-failure", 4);
    let session_id = session_id("mutation-failure");
    let mut persistence = test_persistence(&database_url);
    persistence
        .append_player_joined_with_module_snapshot(
            &character_id,
            join_context(&session_id, &account_id, ReplayProtocolGeneration::V2),
        )
        .expect("mutation fixture snapshot should commit");
    append_completion(&mut persistence, &session_id, &account_id);
    let replay = mutation_context(&session_id, &account_id);
    let before = persistence
        .module_state_for(&character_id)
        .expect("pre-failure module state should load");
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "module_combined");
        assert!(persistence
            .combine_module_with_replay(
                &character_id,
                ActivityPhase::Complete,
                "combine-failure",
                ModuleId::ForceMatrix,
                replay,
            )
            .is_err());
    }
    assert_eq!(
        persistence
            .module_state_for(&character_id)
            .expect("combination rollback state should load"),
        before
    );
    persistence
        .combine_module_with_replay(
            &character_id,
            ActivityPhase::Complete,
            "combine-failure",
            ModuleId::ForceMatrix,
            replay,
        )
        .expect("failed combination ID should remain unreserved");
    let before_loadout = persistence
        .module_state_for(&character_id)
        .expect("pre-loadout state should load");
    {
        let _trigger =
            FailureTrigger::install(&database_url, &session_id, "module_loadout_changed");
        assert!(persistence
            .set_module_loadout_with_replay(
                &character_id,
                ActivityPhase::Complete,
                "loadout-failure",
                0,
                &[ModuleId::ForceMatrix],
                replay,
            )
            .is_err());
    }
    assert_eq!(
        persistence
            .module_state_for(&character_id)
            .expect("loadout rollback state should load"),
        before_loadout
    );
    persistence
        .set_module_loadout_with_replay(
            &character_id,
            ActivityPhase::Complete,
            "loadout-failure",
            0,
            &[ModuleId::ForceMatrix],
            replay,
        )
        .expect("failed loadout ID should remain unreserved");
    let reconstructed = reconstructed_session(&mut persistence, &session_id);
    assert_eq!(reconstructed.module_combinations, 1);
    assert_eq!(reconstructed.module_loadout_changes, 1);
}

#[test]
fn concurrent_retry_appends_exactly_one_combination_event() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let (account_id, character_id) = prepare_character(&database_url, "concurrent", 2);
    let session_id = session_id("concurrent");
    let mut setup = test_persistence(&database_url);
    setup
        .append_player_joined_with_module_snapshot(
            &character_id,
            join_context(&session_id, &account_id, ReplayProtocolGeneration::V2),
        )
        .expect("concurrent snapshot should commit");
    append_completion(&mut setup, &session_id, &account_id);
    drop(setup);

    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let database_url = database_url.clone();
            let account_id = account_id.clone();
            let character_id = character_id.clone();
            let session_id = session_id.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut persistence = Persistence::connect_existing(&database_url)
                    .expect("concurrent replay connection should open");
                barrier.wait();
                persistence
                    .combine_module_with_replay(
                        &character_id,
                        ActivityPhase::Complete,
                        "same-replay-operation",
                        ModuleId::ForceMatrix,
                        mutation_context(&session_id, &account_id),
                    )
                    .expect("concurrent mutation should apply or replay")
            })
        })
        .collect::<Vec<_>>();
    let receipts = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .expect("concurrent replay thread should finish")
        })
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
    let mut assertion = test_persistence(&database_url);
    let event_count = assertion
        .replay_events(&session_id)
        .expect("concurrent replay events should load")
        .into_iter()
        .filter(|event| event.event_type == "module_combined")
        .count();
    assert_eq!(event_count, 1);
}
