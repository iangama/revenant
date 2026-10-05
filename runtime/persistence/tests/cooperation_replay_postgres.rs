use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use postgres::{Client, NoTls};
use revenant_cooperation::{
    CombatProfile, MutationDisposition, Phase, Role, TerminalOutcome, MAX_REVIVE_DISTANCE_SQUARED,
};
use revenant_inventory::weapon_profile;
use revenant_modules::{resolve_build, BaseCombatProfile, ModuleId, CATALOG_REVISION};
use revenant_persistence::{
    CooperationOperationInput, CooperationParticipant, ModuleJoinReplayContext, NewReplayEvent,
    Persistence,
};
use revenant_replay::{
    reconstruct, ReconstructedCooperationState, ReplayEvent, ReplayEventKind,
    ReplayProtocolGeneration,
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

fn persistence(database_url: &str) -> Persistence {
    Persistence::connect_existing(database_url).expect("test persistence should connect")
}

fn prepare(database_url: &str, session_id: &str, label: &str) -> [Fixture; 2] {
    let suffix = unique_suffix();
    let weapons = ["pulse_rifle", "arc_sidearm"];
    let mut persistence = persistence(database_url);
    let fixtures = std::array::from_fn(|index| {
        let account = format!("m28_replay_{label}_{index}_{suffix}");
        let character = format!("{account}:operator");
        let actor = u64::try_from(
            (suffix % 8_000_000_000_000_000_000) + u128::try_from(index).expect("small index") + 1,
        )
        .expect("actor should fit BIGINT");
        persistence
            .ensure_local_account(&account, &format!("M28 Replay {index}"))
            .expect("fixture account should persist");
        let weapon = weapon_profile(weapons[index]).expect("fixture weapon should exist");
        let profile = resolve_build(
            CATALOG_REVISION,
            BaseCombatProfile {
                damage: weapon.damage,
                range: weapon.range,
                cooldown_ms: weapon.cooldown_ms,
                max_health: 100,
            },
            &[],
        )
        .expect("fixture build should resolve")
        .profile;
        persistence
            .append_player_joined_with_module_snapshot(
                &character,
                ModuleJoinReplayContext {
                    catalog_revision: None,
                    session_id,
                    account_id: &account,
                    activity_id: "relay_awakening",
                    actor_id: i64::try_from(actor).expect("actor should fit BIGINT"),
                    player_joined_payload: "player joined",
                    protocol_generation: ReplayProtocolGeneration::V2,
                },
            )
            .expect("admission replay should persist");
        Fixture {
            account,
            character,
            actor,
            weapon: weapons[index],
            modules: Vec::new(),
            profile: CombatProfile {
                damage: profile.damage,
                range: profile.range,
                cooldown_ms: profile.cooldown_ms,
                max_health: profile.max_health,
            },
            health: if index == 0 { 100 } else { 90 },
        }
    });
    for (kind, payload) in [
        ("activity_started", "activity started"),
        ("enemy_died", "enemy died: relay_drone"),
    ] {
        persistence
            .append_replay_event(&NewReplayEvent {
                event_type: kind,
                session_id,
                account_id: &fixtures[0].account,
                activity_id: Some("relay_awakening"),
                actor_id: Some(i64::try_from(fixtures[0].actor).expect("actor should fit")),
                payload,
            })
            .expect("boundary replay should persist");
    }
    fixtures
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

fn start_with_replay(
    persistence: &mut Persistence,
    session_id: &str,
    fixtures: &[Fixture; 2],
) -> revenant_persistence::CooperationReceipt {
    let participants = participants(fixtures);
    persistence
        .start_cooperation_operation_with_replay(&CooperationOperationInput {
            session_id,
            activity_id: "relay_awakening",
            requester_account_id: &fixtures[0].account,
            operation_id: "coop-start-1",
            participants: &participants,
        })
        .expect("cooperation replay start should persist")
}

fn drive_to_encounter(persistence: &mut Persistence, session_id: &str) {
    persistence
        .arrive_cooperation_anchor(session_id, 100)
        .expect("anchor should arrive");
    assert_eq!(
        persistence
            .ping_cooperation_operation_with_replay(session_id, "ping-1", 200)
            .expect("ping replay should persist")
            .disposition,
        MutationDisposition::Applied
    );
    assert_eq!(
        persistence
            .ping_cooperation_operation_with_replay(session_id, "ping-1", 250)
            .expect("later ping retry should validate the durable acceptance time")
            .disposition,
        MutationDisposition::Replayed
    );
    persistence
        .down_cooperation_runner_with_replay(session_id, 300)
        .expect("downing replay should persist");
    persistence
        .start_cooperation_revive(session_id, "revive-1", MAX_REVIVE_DISTANCE_SQUARED, 400)
        .expect("revive should start");
    persistence
        .observe_cooperation_revive_with_replay(
            session_id,
            "revive-1",
            MAX_REVIVE_DISTANCE_SQUARED,
            2_400,
        )
        .expect("revive replay should persist");
}

fn append_warden_evidence(persistence: &mut Persistence, session_id: &str, anchor: &Fixture) {
    for (kind, payload) in [
        ("boss_spawned", "boss spawned: warden"),
        ("enemy_died", "enemy died: warden"),
    ] {
        persistence
            .append_replay_event(&NewReplayEvent {
                event_type: kind,
                session_id,
                account_id: &anchor.account,
                activity_id: Some("relay_awakening"),
                actor_id: Some(i64::try_from(anchor.actor).expect("actor should fit")),
                payload,
            })
            .expect("Warden evidence should persist");
    }
}

fn reconstruct_session(
    persistence: &mut Persistence,
    session_id: &str,
) -> revenant_replay::ReconstructedSession {
    let events = persistence
        .replay_events(session_id)
        .expect("replay should load")
        .into_iter()
        .map(|event| ReplayEvent {
            id: event.id,
            kind: event
                .event_type
                .parse::<ReplayEventKind>()
                .expect("known replay kind"),
            timestamp: event.timestamp,
            session_id: event.session_id,
            account_id: event.account_id,
            activity_id: event.activity_id,
            actor_id: event
                .actor_id
                .map(u64::try_from)
                .transpose()
                .expect("actor should be nonnegative"),
            payload: event.payload,
        })
        .collect::<Vec<_>>();
    reconstruct(&events).expect("cooperation replay should reconstruct")
}

#[test]
fn success_retries_reconstructs_and_projects_exactly() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let session_id = format!("m28-replay-success-{}", unique_suffix());
    let fixtures = prepare(&database_url, &session_id, "success");
    let mut persistence = persistence(&database_url);
    assert_eq!(
        start_with_replay(&mut persistence, &session_id, &fixtures).disposition,
        MutationDisposition::Applied
    );
    assert_eq!(
        start_with_replay(&mut persistence, &session_id, &fixtures).disposition,
        MutationDisposition::Replayed
    );
    drive_to_encounter(&mut persistence, &session_id);
    append_warden_evidence(&mut persistence, &session_id, &fixtures[0]);
    assert_eq!(
        persistence
            .complete_cooperation_operation_with_replay(&session_id, 3_000)
            .expect("success replay should persist")
            .disposition,
        MutationDisposition::Applied
    );
    assert_eq!(
        persistence
            .complete_cooperation_operation_with_replay(&session_id, 3_000)
            .expect("success retry should replay")
            .disposition,
        MutationDisposition::Replayed
    );

    let reconstructed = reconstruct_session(&mut persistence, &session_id);
    assert_eq!(
        reconstructed.cooperation.state,
        ReconstructedCooperationState::Succeeded
    );
    assert_eq!(reconstructed.cooperation.contribution_count(), 5);
    assert_eq!(reconstructed.loot_grants, 2);
    assert_eq!(reconstructed.progression_grants, 2);
    let summary = persistence
        .authoritative_session_summary(&session_id)
        .expect("summary should validate")
        .expect("summary should exist");
    assert!(!summary.cooperation_replay_legacy);
    assert_eq!(
        summary.cooperation_replay_state,
        ReconstructedCooperationState::Succeeded
    );
    assert_eq!(summary.cooperation_last_phase, Some(Phase::EncounterActive));
    assert_eq!(
        summary.cooperation_terminal_outcome,
        Some(TerminalOutcome::Succeeded)
    );
    assert_eq!(summary.cooperation_terminal_elapsed_ms, Some(3_000));
    assert_eq!(summary.cooperation_participant_count, 2);
    assert_eq!(summary.cooperation_contribution_count, 5);
    assert_eq!(summary.cooperation_revive_count, 1);
    assert_eq!(summary.cooperation_reward_participant_count, 2);

    let cooperation_kinds = persistence
        .replay_events(&session_id)
        .expect("events should load")
        .into_iter()
        .filter_map(|event| {
            let kind = event.event_type.parse::<ReplayEventKind>().ok()?;
            kind.is_cooperation_event().then_some(event.event_type)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cooperation_kinds,
        [
            "cooperation_started",
            "cooperation_pinged",
            "player_downed",
            "player_revived",
            "cooperation_succeeded",
        ]
    );
}

#[test]
fn timeout_is_terminal_reward_free_and_retry_exact() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let session_id = format!("m28-replay-timeout-{}", unique_suffix());
    let fixtures = prepare(&database_url, &session_id, "timeout");
    let mut persistence = persistence(&database_url);
    start_with_replay(&mut persistence, &session_id, &fixtures);
    persistence
        .arrive_cooperation_anchor(&session_id, 100)
        .expect("anchor should arrive");
    persistence
        .ping_cooperation_operation_with_replay(&session_id, "ping-1", 200)
        .expect("ping should persist");
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "cooperation_failed");
        assert!(persistence
            .timeout_cooperation_operation_with_replay(&session_id, 5_201)
            .is_err());
    }
    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    let terminal_count = client
        .query_one(
            "SELECT COUNT(*) FROM cooperation_operations WHERE session_id = $1 \
             AND terminal_outcome IS NOT NULL",
            &[&session_id],
        )
        .expect("terminal count should query")
        .get::<_, i64>(0);
    assert_eq!(terminal_count, 0);
    assert_eq!(
        persistence
            .timeout_cooperation_operation_with_replay(&session_id, 5_201)
            .expect("ping timeout should persist")
            .state
            .terminal(),
        Some(TerminalOutcome::FailedPingTimeout)
    );
    assert_eq!(
        persistence
            .timeout_cooperation_operation_with_replay(&session_id, 5_201)
            .expect("timeout retry should replay")
            .disposition,
        MutationDisposition::Replayed
    );
    let reconstructed = reconstruct_session(&mut persistence, &session_id);
    assert_eq!(
        reconstructed.cooperation.state,
        ReconstructedCooperationState::Failed
    );
    assert!(reconstructed
        .cooperation
        .terminal
        .expect("terminal should exist")
        .grants
        .is_empty());
    let reward_count = client
        .query_one(
            "SELECT (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1) + \
                    (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1)",
            &[&session_id],
        )
        .expect("reward count should query")
        .get::<_, i64>(0);
    assert_eq!(reward_count, 0);
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_failure_family_reconstructs_with_exact_subject_and_zero_rewards() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let mut cases = Vec::new();

    let ping_session = format!("m28-replay-fail-ping-{}", unique_suffix());
    let ping_fixture = prepare(&database_url, &ping_session, "fail_ping");
    let mut persistence = persistence(&database_url);
    start_with_replay(&mut persistence, &ping_session, &ping_fixture);
    persistence
        .arrive_cooperation_anchor(&ping_session, 0)
        .expect("anchor should arrive");
    persistence
        .ping_cooperation_operation_with_replay(&ping_session, "ping-1", 100)
        .expect("ping should persist");
    persistence
        .timeout_cooperation_operation_with_replay(&ping_session, 5_101)
        .expect("ping timeout should persist");
    cases.push((ping_session, TerminalOutcome::FailedPingTimeout, None));

    let revive_session = format!("m28-replay-fail-revive-{}", unique_suffix());
    let revive_fixture = prepare(&database_url, &revive_session, "fail_revive");
    start_with_replay(&mut persistence, &revive_session, &revive_fixture);
    persistence
        .arrive_cooperation_anchor(&revive_session, 0)
        .expect("anchor should arrive");
    persistence
        .ping_cooperation_operation_with_replay(&revive_session, "ping-1", 100)
        .expect("ping should persist");
    persistence
        .down_cooperation_runner_with_replay(&revive_session, 100)
        .expect("downing should persist");
    persistence
        .timeout_cooperation_operation_with_replay(&revive_session, 15_101)
        .expect("revive timeout should persist");
    cases.push((revive_session, TerminalOutcome::FailedReviveTimeout, None));

    let operation_session = format!("m28-replay-fail-operation-{}", unique_suffix());
    let operation_fixture = prepare(&database_url, &operation_session, "fail_operation");
    start_with_replay(&mut persistence, &operation_session, &operation_fixture);
    persistence
        .timeout_cooperation_operation_with_replay(&operation_session, 60_001)
        .expect("operation timeout should persist");
    cases.push((
        operation_session,
        TerminalOutcome::FailedOperationTimeout,
        None,
    ));

    let defeat_session = format!("m28-replay-fail-defeat-{}", unique_suffix());
    let defeat_fixture = prepare(&database_url, &defeat_session, "fail_defeat");
    start_with_replay(&mut persistence, &defeat_session, &defeat_fixture);
    drive_to_encounter(&mut persistence, &defeat_session);
    persistence
        .defeat_cooperation_participant_with_replay(&defeat_session, Role::Runner, 3_000)
        .expect("defeat should persist");
    cases.push((
        defeat_session,
        TerminalOutcome::FailedParticipantDefeated,
        Some(Role::Runner),
    ));

    let abandon_session = format!("m28-replay-fail-abandon-{}", unique_suffix());
    let abandon_fixture = prepare(&database_url, &abandon_session, "fail_abandon");
    start_with_replay(&mut persistence, &abandon_session, &abandon_fixture);
    persistence
        .abandon_cooperation_operation_with_replay(&abandon_session, Role::Runner, 100)
        .expect("abandonment should persist");
    cases.push((
        abandon_session,
        TerminalOutcome::AbandonedDisconnect,
        Some(Role::Runner),
    ));

    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    for (session_id, expected_outcome, expected_subject) in cases {
        let reconstructed = reconstruct_session(&mut persistence, &session_id);
        let terminal = reconstructed
            .cooperation
            .terminal
            .expect("failure terminal should reconstruct");
        assert_eq!(
            reconstructed.cooperation.state,
            ReconstructedCooperationState::Failed
        );
        assert_eq!(terminal.outcome, expected_outcome);
        assert_eq!(terminal.subject_role, expected_subject);
        assert!(terminal.grants.is_empty());
        let counts = client
            .query_one(
                "SELECT \
                   (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
                        AND event_type = 'cooperation_failed'), \
                   (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
                        AND event_type IN ('activity_completed', 'loot_granted', \
                                           'progression_granted')), \
                   (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), \
                   (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1)",
                &[&session_id],
            )
            .expect("failure counts should query");
        assert_eq!(
            (0..4)
                .map(|index| counts.get::<_, i64>(index))
                .collect::<Vec<_>>(),
            vec![1, 0, 0, 0]
        );
    }
}

#[test]
fn corruption_rejects_summary_and_exact_retry() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let session_id = format!("m28-replay-corrupt-{}", unique_suffix());
    let fixtures = prepare(&database_url, &session_id, "corrupt");
    let mut persistence = persistence(&database_url);
    start_with_replay(&mut persistence, &session_id, &fixtures);
    persistence
        .arrive_cooperation_anchor(&session_id, 100)
        .expect("anchor should arrive");
    persistence
        .ping_cooperation_operation_with_replay(&session_id, "ping-1", 200)
        .expect("ping should persist");
    let mut client = Client::connect(&database_url, NoTls).expect("corruption connection");
    client
        .execute(
            "UPDATE replay_events SET payload = \
             regexp_replace(payload, '^\\{', '{\"unknown\":true,') \
             WHERE session_id = $1 AND event_type = 'cooperation_pinged'",
            &[&session_id],
        )
        .expect("test corruption should apply");
    assert!(persistence
        .authoritative_session_summary(&session_id)
        .is_err());
    assert!(persistence
        .ping_cooperation_operation_with_replay(&session_id, "ping-1", 200)
        .is_err());
}

#[test]
#[allow(clippy::too_many_lines)]
fn replay_success_repeats_every_reward_terminal_and_append_boundary() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let cases = [
        ("inventory_grant", "inventory_reward_grants", "INSERT", None),
        ("inventory", "inventory", "INSERT", None),
        (
            "progression_grant",
            "progression_reward_grants",
            "INSERT",
            None,
        ),
        ("progression", "progression", "UPDATE", None),
        ("character", "characters", "UPDATE", None),
        ("history", "activity_history", "INSERT", None),
        ("terminal", "cooperation_operations", "UPDATE", None),
        (
            "completed_replay",
            "replay_events",
            "INSERT",
            Some("activity_completed"),
        ),
        (
            "loot_replay",
            "replay_events",
            "INSERT",
            Some("loot_granted"),
        ),
        (
            "progression_replay",
            "replay_events",
            "INSERT",
            Some("progression_granted"),
        ),
        (
            "terminal_replay",
            "replay_events",
            "INSERT",
            Some("cooperation_succeeded"),
        ),
    ];
    for (label, table, event, replay_event_type) in cases {
        let session_id = format!("m28-rb-{}", unique_suffix());
        let fixtures = prepare(&database_url, &session_id, label);
        let mut persistence = persistence(&database_url);
        start_with_replay(&mut persistence, &session_id, &fixtures);
        drive_to_encounter(&mut persistence, &session_id);
        append_warden_evidence(&mut persistence, &session_id, &fixtures[0]);
        let predicate = if let Some(event_type) = replay_event_type {
            format!("NEW.session_id = '{session_id}' AND NEW.event_type = '{event_type}'")
        } else if matches!(
            table,
            "inventory_reward_grants" | "progression_reward_grants" | "cooperation_operations"
        ) {
            format!("NEW.session_id = '{session_id}'")
        } else if table == "characters" {
            format!("NEW.id = '{}'", fixtures[0].character)
        } else {
            format!("NEW.character_id = '{}'", fixtures[0].character)
        };
        {
            let _trigger = FailureTrigger::install_on(&database_url, table, event, &predicate);
            assert!(persistence
                .complete_cooperation_operation_with_replay(&session_id, 3_000)
                .is_err());
        }
        let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
        let rollback = client
            .query_one(
                "SELECT \
                   (SELECT COUNT(*) FROM cooperation_operations WHERE session_id = $1 \
                        AND terminal_outcome IS NOT NULL), \
                   (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), \
                   (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1), \
                   (SELECT COUNT(*) FROM activity_history WHERE character_id = $2), \
                   (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
                        AND event_type IN ('activity_completed', 'loot_granted', \
                                           'progression_granted', 'cooperation_succeeded')), \
                   (SELECT COUNT(*) FROM inventory WHERE character_id = $2 \
                        AND item_id = 'relay_core_fragment'), \
                   (SELECT experience FROM progression WHERE character_id = $2), \
                   (SELECT level FROM characters WHERE id = $2)",
                &[&session_id, &fixtures[0].character],
            )
            .expect("rollback state should query");
        assert_eq!(
            (0..7)
                .map(|index| rollback.get::<_, i64>(index))
                .collect::<Vec<_>>(),
            vec![0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(rollback.get::<_, i32>(7), 1);
        persistence
            .complete_cooperation_operation_with_replay(&session_id, 3_000)
            .expect("rolled-back success should remain reusable");
        assert_eq!(
            reconstruct_session(&mut persistence, &session_id)
                .cooperation
                .state,
            ReconstructedCooperationState::Succeeded
        );
    }
}

struct FailureTrigger {
    database_url: String,
    trigger: String,
    function: String,
    table: String,
}

impl FailureTrigger {
    fn install(database_url: &str, session_id: &str, event_type: &str) -> Self {
        Self::install_on(
            database_url,
            "replay_events",
            "INSERT",
            &format!("NEW.session_id = '{session_id}' AND NEW.event_type = '{event_type}'"),
        )
    }

    fn install_on(database_url: &str, table: &str, event: &str, predicate: &str) -> Self {
        let suffix = unique_suffix();
        let trigger = format!("m28_replay_trigger_{suffix}");
        let function = format!("m28_replay_function_{suffix}");
        let mut client = Client::connect(database_url, NoTls).expect("trigger connection");
        client
            .batch_execute(&format!(
                "CREATE FUNCTION {function}() RETURNS TRIGGER LANGUAGE plpgsql AS $m28$ \
                 BEGIN IF {predicate} \
                 THEN RAISE EXCEPTION 'm28 replay injected failure'; END IF; \
                 RETURN NEW; END; $m28$; \
                 CREATE TRIGGER {trigger} BEFORE {event} ON {table} \
                 FOR EACH ROW EXECUTE FUNCTION {function}();"
            ))
            .expect("failure trigger should install");
        Self {
            database_url: database_url.to_owned(),
            trigger,
            function,
            table: table.to_owned(),
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
#[allow(clippy::too_many_lines)]
fn replay_append_failures_roll_back_start_ping_and_success_units() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let session_id = format!("m28-replay-rollback-{}", unique_suffix());
    let fixtures = prepare(&database_url, &session_id, "rollback");
    let mut persistence = persistence(&database_url);
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "cooperation_started");
        let participants = participants(&fixtures);
        assert!(persistence
            .start_cooperation_operation_with_replay(&CooperationOperationInput {
                session_id: &session_id,
                activity_id: "relay_awakening",
                requester_account_id: &fixtures[0].account,
                operation_id: "coop-start-1",
                participants: &participants,
            })
            .is_err());
    }
    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    assert_eq!(
        client
            .query_one(
                "SELECT COUNT(*) FROM cooperation_operations WHERE session_id = $1",
                &[&session_id],
            )
            .expect("start count should query")
            .get::<_, i64>(0),
        0
    );
    start_with_replay(&mut persistence, &session_id, &fixtures);
    persistence
        .arrive_cooperation_anchor(&session_id, 100)
        .expect("anchor should persist");
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "cooperation_pinged");
        assert!(persistence
            .ping_cooperation_operation_with_replay(&session_id, "ping-1", 200)
            .is_err());
    }
    let pinged = client
        .query_one(
            "SELECT pinged FROM cooperation_operations WHERE session_id = $1",
            &[&session_id],
        )
        .expect("ping state should query")
        .get::<_, bool>(0);
    assert!(!pinged);
    persistence
        .ping_cooperation_operation_with_replay(&session_id, "ping-1", 200)
        .expect("rolled-back ping should remain reusable");
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "player_downed");
        assert!(persistence
            .down_cooperation_runner_with_replay(&session_id, 300)
            .is_err());
    }
    let downing = client
        .query_one(
            "SELECT c.runner_arrived, p.current_health, p.life_state \
             FROM cooperation_operations c JOIN cooperation_operation_participants p \
               USING (session_id) WHERE c.session_id = $1 AND p.participant_index = 1",
            &[&session_id],
        )
        .expect("downing rollback state should query");
    assert!(!downing.get::<_, bool>(0));
    assert_eq!(
        downing.get::<_, i32>(1),
        i32::try_from(fixtures[1].health).expect("health fits")
    );
    assert_eq!(downing.get::<_, String>(2), "active");
    persistence
        .down_cooperation_runner_with_replay(&session_id, 300)
        .expect("downing should persist");
    persistence
        .start_cooperation_revive(&session_id, "revive-1", MAX_REVIVE_DISTANCE_SQUARED, 400)
        .expect("revive should start");
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "player_revived");
        assert!(persistence
            .observe_cooperation_revive_with_replay(
                &session_id,
                "revive-1",
                MAX_REVIVE_DISTANCE_SQUARED,
                2_400,
            )
            .is_err());
    }
    let revive = client
        .query_one(
            "SELECT c.phase, c.revived, p.current_health, p.life_state \
             FROM cooperation_operations c JOIN cooperation_operation_participants p \
               USING (session_id) WHERE c.session_id = $1 AND p.participant_index = 1",
            &[&session_id],
        )
        .expect("revive rollback state should query");
    assert_eq!(revive.get::<_, String>(0), "revive_channel");
    assert!(!revive.get::<_, bool>(1));
    assert_eq!(revive.get::<_, i32>(2), 0);
    assert_eq!(revive.get::<_, String>(3), "downed");
    persistence
        .observe_cooperation_revive_with_replay(
            &session_id,
            "revive-1",
            MAX_REVIVE_DISTANCE_SQUARED,
            2_400,
        )
        .expect("revive should persist");
    append_warden_evidence(&mut persistence, &session_id, &fixtures[0]);
    {
        let _trigger = FailureTrigger::install(&database_url, &session_id, "cooperation_succeeded");
        assert!(persistence
            .complete_cooperation_operation_with_replay(&session_id, 3_000)
            .is_err());
    }
    let rollback = client
        .query_one(
            "SELECT \
               (SELECT COUNT(*) FROM cooperation_operations WHERE session_id = $1 \
                    AND terminal_outcome IS NOT NULL), \
               (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), \
               (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1), \
               (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 AND event_type IN (\
                    'activity_completed', 'loot_granted', 'progression_granted', \
                    'cooperation_succeeded'))",
            &[&session_id],
        )
        .expect("rollback counts should query");
    assert_eq!(
        (0..4)
            .map(|index| rollback.get::<_, i64>(index))
            .collect::<Vec<_>>(),
        vec![0, 0, 0, 0]
    );
    persistence
        .complete_cooperation_operation_with_replay(&session_id, 3_000)
        .expect("rolled-back success should remain reusable");
}
