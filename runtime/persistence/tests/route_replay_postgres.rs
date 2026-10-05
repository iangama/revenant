use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use postgres::{Client, NoTls};
use revenant_operations::{
    apply_warden_health_effect, resolve_event, route_definition, OperationDisposition, RouteId,
    RouteOperationSummary, RouteSeed, TerminalOutcome, CATALOG_REVISION, RESOLVER_REVISION,
};
use revenant_persistence::{
    ModuleJoinReplayContext, Persistence, RouteOperationSelection, RouteParticipant,
    RoutePersistenceError,
};
use revenant_replay::{
    module_state_evidence, reconstruct, ReconstructedRouteState, ReplayEvent, ReplayEventKind,
    ReplayObjectiveTransition, ReplayProtocolGeneration, ReplayRouteEncounterEvidence,
    ReplayRouteGrantEvidence, ReplayRouteObjectiveEvidence, ReplayRouteObjectiveKind,
    ReplayRouteObjectiveState, ReplayRouteParticipantEvidence, RouteSelectedPayloadV1,
    RouteTerminalPayloadV1, AUTHORING_REVISION, ROUTE_REPLAY_SCHEMA_VERSION,
};

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos()
}

struct Fixture {
    account: String,
    character: String,
    actor: u64,
    module_state: revenant_replay::ReplayModuleStateEvidence,
}

fn prepare(database_url: &str, label: &str, session: &str) -> Fixture {
    let suffix = unique_suffix();
    let account = format!("m27_replay_{label}_{suffix}");
    let character = format!("{account}:operator");
    let actor =
        u64::try_from(suffix % 9_000_000_000_000_000_000).expect("bounded actor should fit") + 1;
    let mut persistence =
        Persistence::connect_existing(database_url).expect("test persistence should open");
    persistence
        .ensure_local_account(&account, "M27 Replay")
        .expect("test account should persist");
    let (_, _, persisted) = persistence
        .append_player_joined_with_module_snapshot(
            &character,
            ModuleJoinReplayContext {
                catalog_revision: None,
                session_id: session,
                account_id: &account,
                activity_id: "relay_awakening",
                actor_id: i64::try_from(actor).expect("actor should fit BIGINT"),
                player_joined_payload: "player joined",
                protocol_generation: ReplayProtocolGeneration::V2,
            },
        )
        .expect("admission snapshot should persist");
    let module_state = module_state_evidence(
        persisted.fragments,
        &persisted.owned_modules,
        &persisted.loadout,
        persisted.revision,
        ReplayProtocolGeneration::V2,
    )
    .expect("persisted module state should resolve");
    Fixture {
        account,
        character,
        actor,
        module_state,
    }
}

fn selection(fixture: &Fixture, route_id: RouteId, seed: u64) -> RouteSelectedPayloadV1 {
    let seed = RouteSeed::new(seed).expect("test seed should validate");
    let definition = route_definition(route_id);
    let event = resolve_event(route_id, seed);
    RouteSelectedPayloadV1 {
        schema_version: ROUTE_REPLAY_SCHEMA_VERSION,
        activity_id: "relay_awakening".to_owned(),
        authoring_revision: AUTHORING_REVISION.to_owned(),
        catalog_revision: CATALOG_REVISION.to_owned(),
        resolver_revision: RESOLVER_REVISION.to_owned(),
        operation_id: "route-replay-operation".to_owned(),
        leader_id: fixture.account.clone(),
        participants: vec![ReplayRouteParticipantEvidence {
            participant_id: fixture.account.clone(),
            character_id: fixture.character.clone(),
            actor_id: fixture.actor,
            weapon_item_id: "pulse_rifle".to_owned(),
            module_state: fixture.module_state.clone(),
        }],
        route_id,
        seed,
        event_id: event.event_id,
        effect: event.effect,
        objectives: definition
            .objective_path
            .iter()
            .enumerate()
            .map(|(index, objective_id)| ReplayRouteObjectiveEvidence {
                objective_id: *objective_id,
                kind: match objective_id {
                    revenant_operations::RouteObjectiveId::ClearDroneGroup => {
                        ReplayRouteObjectiveKind::KillActors
                    }
                    revenant_operations::RouteObjectiveId::ReachRelayStabilizer
                    | revenant_operations::RouteObjectiveId::ReachRelayDoor => {
                        ReplayRouteObjectiveKind::ReachArea
                    }
                    revenant_operations::RouteObjectiveId::DefeatWarden => {
                        ReplayRouteObjectiveKind::Boss
                    }
                },
                initial_state: if index == 0 {
                    ReplayRouteObjectiveState::Active
                } else {
                    ReplayRouteObjectiveState::Pending
                },
                target: 1,
            })
            .collect(),
        duration_budget_ms: definition.duration_ms,
        reward: definition.reward,
    }
}

fn select_with_replay(
    persistence: &mut Persistence,
    session: &str,
    fixture: &Fixture,
    replay: &RouteSelectedPayloadV1,
) -> Result<
    revenant_operations::OperationReceipt<revenant_operations::RouteSelectionOutcome>,
    RoutePersistenceError,
> {
    let participants = [RouteParticipant {
        account_id: &fixture.account,
        character_id: &fixture.character,
        actor_id: fixture.actor,
    }];
    persistence.select_route_operation_with_replay(
        &RouteOperationSelection {
            session_id: session,
            activity_id: "relay_awakening",
            requester_account_id: &fixture.account,
            operation_id: &replay.operation_id,
            route_id: replay.route_id,
            candidate_seed: replay.seed,
            participants: &participants,
        },
        replay,
    )
}

fn select_two_with_replay(
    persistence: &mut Persistence,
    session: &str,
    fixtures: [&Fixture; 2],
    replay: &RouteSelectedPayloadV1,
) -> Result<
    revenant_operations::OperationReceipt<revenant_operations::RouteSelectionOutcome>,
    RoutePersistenceError,
> {
    let participants = fixtures.map(|fixture| RouteParticipant {
        account_id: &fixture.account,
        character_id: &fixture.character,
        actor_id: fixture.actor,
    });
    persistence.select_route_operation_with_replay(
        &RouteOperationSelection {
            session_id: session,
            activity_id: "relay_awakening",
            requester_account_id: &fixtures[0].account,
            operation_id: &replay.operation_id,
            route_id: replay.route_id,
            candidate_seed: replay.seed,
            participants: &participants,
        },
        replay,
    )
}

fn success_terminal(selection: &RouteSelectedPayloadV1) -> RouteTerminalPayloadV1 {
    let mut transitions = Vec::new();
    let mut ordinal = 0_u8;
    for (index, objective) in selection.objectives.iter().enumerate() {
        if index > 0 {
            transitions.push(ReplayObjectiveTransition {
                ordinal,
                objective_id: objective.objective_id,
                from: ReplayRouteObjectiveState::Pending,
                to: ReplayRouteObjectiveState::Active,
                progress: 0,
                target: 1,
            });
            ordinal += 1;
        }
        transitions.push(ReplayObjectiveTransition {
            ordinal,
            objective_id: objective.objective_id,
            from: ReplayRouteObjectiveState::Active,
            to: ReplayRouteObjectiveState::Completed,
            progress: 1,
            target: 1,
        });
        ordinal += 1;
    }
    terminal(selection, TerminalOutcome::Succeeded, 90_000, transitions)
}

fn failure_terminal(selection: &RouteSelectedPayloadV1) -> RouteTerminalPayloadV1 {
    terminal(
        selection,
        TerminalOutcome::FailedTimeout,
        90_001,
        vec![ReplayObjectiveTransition {
            ordinal: 0,
            objective_id: selection.objectives[0].objective_id,
            from: ReplayRouteObjectiveState::Active,
            to: ReplayRouteObjectiveState::Failed,
            progress: 0,
            target: 1,
        }],
    )
}

fn terminal(
    selection: &RouteSelectedPayloadV1,
    outcome: TerminalOutcome,
    elapsed_ms: u64,
    transitions: Vec<ReplayObjectiveTransition>,
) -> RouteTerminalPayloadV1 {
    let (drone_health, warden_health) = if selection.participants.len() == 1 {
        (140, 240)
    } else {
        (210, 360)
    };
    let effective_health = apply_warden_health_effect(warden_health, selection.effect)
        .expect("event effect should apply");
    let succeeded = outcome == TerminalOutcome::Succeeded;
    let weapon_damage = selection
        .participants
        .iter()
        .map(|participant| {
            participant
                .module_state
                .weapons
                .iter()
                .find(|weapon| weapon.item_id == participant.weapon_item_id)
                .expect("selected weapon should exist")
                .effective
                .damage
        })
        .max()
        .expect("route should have participants");
    RouteTerminalPayloadV1 {
        schema_version: ROUTE_REPLAY_SCHEMA_VERSION,
        authoring_revision: AUTHORING_REVISION.to_owned(),
        catalog_revision: CATALOG_REVISION.to_owned(),
        resolver_revision: RESOLVER_REVISION.to_owned(),
        operation_id: selection.operation_id.clone(),
        route_id: selection.route_id,
        event_id: selection.event_id,
        transitions,
        encounter: ReplayRouteEncounterEvidence {
            participant_count: u8::try_from(selection.participants.len())
                .expect("participant count should fit"),
            drone_health,
            drone_damage: 10,
            drone_defeated: succeeded,
            warden_spawned: succeeded,
            warden_base_health: warden_health,
            warden_effective_health: effective_health,
            warden_remaining_health: 0,
            accepted_player_hits: if succeeded {
                effective_health.div_ceil(weapon_damage)
            } else {
                0
            },
            warden_counter_count: if succeeded { 2 } else { 0 },
            warden_counter_damage: selection.effect.warden_counter_damage,
            total_hostile_damage: if succeeded {
                10 + 2 * selection.effect.warden_counter_damage
            } else {
                0
            },
        },
        summary: RouteOperationSummary {
            catalog_revision: CATALOG_REVISION.to_owned(),
            resolver_revision: RESOLVER_REVISION.to_owned(),
            operation_id: selection.operation_id.clone(),
            leader_id: selection.leader_id.clone(),
            participant_ids: selection
                .participants
                .iter()
                .map(|participant| participant.participant_id.clone())
                .collect(),
            route_id: selection.route_id,
            seed: selection.seed,
            event_id: selection.event_id,
            effect: selection.effect,
            objective_path: selection
                .objectives
                .iter()
                .map(|objective| objective.objective_id)
                .collect(),
            duration_budget_ms: selection.duration_budget_ms,
            elapsed_ms,
            outcome,
            reward: succeeded.then_some(selection.reward),
        },
        grants: if succeeded {
            selection
                .participants
                .iter()
                .map(|participant| ReplayRouteGrantEvidence {
                    participant_id: participant.participant_id.clone(),
                    character_id: participant.character_id.clone(),
                    actor_id: participant.actor_id,
                    item_id: "relay_core_fragment".to_owned(),
                    item_quantity: selection.reward.fragments,
                    experience: selection.reward.experience,
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn reconstruct_session(
    persistence: &mut Persistence,
    session: &str,
) -> revenant_replay::ReconstructedSession {
    let events = persistence
        .replay_events(session)
        .expect("route replay should load")
        .into_iter()
        .map(|event| ReplayEvent {
            id: event.id,
            kind: event
                .event_type
                .parse::<ReplayEventKind>()
                .expect("known event kind"),
            timestamp: event.timestamp,
            session_id: event.session_id,
            account_id: event.account_id,
            activity_id: event.activity_id,
            actor_id: event
                .actor_id
                .map(u64::try_from)
                .transpose()
                .expect("positive actor"),
            payload: event.payload,
        })
        .collect::<Vec<_>>();
    reconstruct(&events).expect("persisted route replay should reconstruct")
}

#[test]
fn selection_success_retries_and_independent_reconstruction_are_exact() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let session = format!("m27-replay-success-{}", unique_suffix());
    let fixture = prepare(&database_url, "success", &session);
    let selection = selection(&fixture, RouteId::Breach, 0);
    let mut persistence =
        Persistence::connect_existing(&database_url).expect("test persistence should reconnect");
    let selected = select_with_replay(&mut persistence, &session, &fixture, &selection)
        .expect("route selection replay should commit");
    assert_eq!(selected.disposition, OperationDisposition::Applied);
    assert_eq!(
        select_with_replay(&mut persistence, &session, &fixture, &selection)
            .expect("selection retry should replay")
            .disposition,
        OperationDisposition::Replayed
    );
    let terminal = success_terminal(&selection);
    let completed = persistence
        .complete_route_operation_with_replay(&session, 90_000, &terminal)
        .expect("route success replay should commit");
    assert_eq!(completed.disposition, OperationDisposition::Applied);
    assert_eq!(
        persistence
            .complete_route_operation_with_replay(&session, 90_000, &terminal)
            .expect("terminal retry should replay")
            .disposition,
        OperationDisposition::Replayed
    );
    let reconstructed = reconstruct_session(&mut persistence, &session);
    assert_eq!(
        reconstructed.route.state,
        ReconstructedRouteState::Succeeded
    );
    assert_eq!(reconstructed.loot_grants, 1);
    assert_eq!(reconstructed.progression_grants, 1);
    let summary = persistence
        .authoritative_session_summary(&session)
        .expect("route summary should validate")
        .expect("route summary should exist");
    assert!(!summary.route_replay_legacy);
    assert_eq!(summary.route_id, Some(RouteId::Breach));
    assert_eq!(summary.route_event_id, Some(selection.event_id));
    assert_eq!(
        summary.route_terminal_outcome,
        Some(TerminalOutcome::Succeeded)
    );
    assert_eq!(summary.route_elapsed_ms, Some(90_000));
    assert_eq!(summary.route_transition_count, 5);
    assert_eq!(summary.route_reward_participant_count, 1);
    let event_types = persistence
        .replay_events(&session)
        .expect("events should load")
        .into_iter()
        .map(|event| event.event_type)
        .collect::<Vec<_>>();
    assert_eq!(
        event_types,
        vec![
            "player_joined",
            "module_state_snapshot",
            "route_selected",
            "activity_completed",
            "loot_granted",
            "progression_granted",
            "route_operation_succeeded",
        ]
    );
}

#[test]
fn two_participant_success_preserves_admission_and_grant_order() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let session = format!("m27-replay-pair-{}", unique_suffix());
    let leader = prepare(&database_url, "pair_leader", &session);
    let partner = prepare(&database_url, "pair_partner", &session);
    let mut selection = selection(&leader, RouteId::Stabilize, 3);
    selection.participants.push(ReplayRouteParticipantEvidence {
        participant_id: partner.account.clone(),
        character_id: partner.character.clone(),
        actor_id: partner.actor,
        weapon_item_id: "pulse_rifle".to_owned(),
        module_state: partner.module_state.clone(),
    });
    let mut persistence =
        Persistence::connect_existing(&database_url).expect("test persistence should reconnect");
    select_two_with_replay(&mut persistence, &session, [&leader, &partner], &selection)
        .expect("two-participant selection should commit");
    let terminal = success_terminal(&selection);
    persistence
        .complete_route_operation_with_replay(&session, 90_000, &terminal)
        .expect("two-participant success should commit");
    let reconstructed = reconstruct_session(&mut persistence, &session);
    assert_eq!(
        reconstructed.route.state,
        ReconstructedRouteState::Succeeded
    );
    assert_eq!(
        reconstructed.route.terminal.expect("terminal").grants.len(),
        2
    );
    assert_eq!(reconstructed.loot_grants, 2);
    assert_eq!(reconstructed.progression_grants, 2);
}

#[test]
fn timeout_replay_is_terminal_without_completion_or_rewards() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let session = format!("m27-replay-timeout-{}", unique_suffix());
    let fixture = prepare(&database_url, "timeout", &session);
    let selection = selection(&fixture, RouteId::Stabilize, 3);
    let mut persistence =
        Persistence::connect_existing(&database_url).expect("test persistence should reconnect");
    select_with_replay(&mut persistence, &session, &fixture, &selection)
        .expect("route selection replay should commit");
    let terminal = failure_terminal(&selection);
    persistence
        .fail_route_operation_timeout_with_replay(&session, 90_001, &terminal)
        .expect("route timeout replay should commit");
    let reconstructed = reconstruct_session(&mut persistence, &session);
    assert_eq!(reconstructed.route.state, ReconstructedRouteState::Failed);
    assert!(!reconstructed.completed);
    assert_eq!(reconstructed.loot_grants, 0);
    assert_eq!(reconstructed.progression_grants, 0);
}

struct FailureTrigger {
    database_url: String,
    trigger: String,
    function: String,
    table: String,
}

impl FailureTrigger {
    fn install(database_url: &str, session: &str, event_type: &str) -> Self {
        let predicate = format!("NEW.session_id = '{session}' AND NEW.event_type = '{event_type}'");
        Self::install_on(
            database_url,
            event_type,
            "replay_events",
            "INSERT",
            &predicate,
        )
    }

    fn install_on(
        database_url: &str,
        label: &str,
        table: &str,
        event: &str,
        predicate: &str,
    ) -> Self {
        let suffix = unique_suffix();
        let trigger = format!("m27_replay_trigger_{label}_{suffix}");
        let function = format!("m27_replay_function_{label}_{suffix}");
        let mut client = Client::connect(database_url, NoTls).expect("trigger connection");
        client
            .batch_execute(&format!(
                "CREATE FUNCTION {function}() RETURNS TRIGGER LANGUAGE plpgsql AS $m27$ \
                 BEGIN IF {predicate} THEN RAISE EXCEPTION 'm27 replay injected failure'; END IF; \
                 RETURN NEW; END; $m27$; \
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

fn assert_replay_selection_write_failure(database_url: &str, label: &str, table: &str) {
    let session = format!("m27-rs-{}", unique_suffix());
    let fixture = prepare(database_url, label, &session);
    let selection = selection(&fixture, RouteId::Breach, 0);
    let mut persistence =
        Persistence::connect_existing(database_url).expect("test persistence should reconnect");
    let predicate = if table == "replay_events" {
        format!("NEW.session_id = '{session}' AND NEW.event_type = 'route_selected'")
    } else {
        format!("NEW.session_id = '{session}'")
    };
    {
        let _trigger = FailureTrigger::install_on(database_url, label, table, "INSERT", &predicate);
        assert!(matches!(
            select_with_replay(&mut persistence, &session, &fixture, &selection),
            Err(RoutePersistenceError::Database(_))
        ));
    }
    let mut client = Client::connect(database_url, NoTls).expect("assertion connection");
    let counts = client
        .query_one(
            "SELECT \
               (SELECT COUNT(*) FROM route_operations WHERE session_id = $1), \
               (SELECT COUNT(*) FROM route_operation_participants WHERE session_id = $1), \
               (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
                    AND event_type = 'route_selected')",
            &[&session],
        )
        .expect("selection rollback counts should query");
    assert_eq!(
        (0..3)
            .map(|index| counts.get::<_, i64>(index))
            .collect::<Vec<_>>(),
        vec![0, 0, 0]
    );
    assert_eq!(
        select_with_replay(&mut persistence, &session, &fixture, &selection)
            .expect("rolled-back selection should remain reusable")
            .disposition,
        OperationDisposition::Applied
    );
}

fn assert_replay_terminal_write_failure(
    database_url: &str,
    label: &str,
    table: &str,
    event: &str,
    replay_event_type: Option<&str>,
) {
    let session = format!("m27-rt-{}", unique_suffix());
    let fixture = prepare(database_url, label, &session);
    let selection = selection(&fixture, RouteId::Breach, 0);
    let terminal = success_terminal(&selection);
    let mut persistence =
        Persistence::connect_existing(database_url).expect("test persistence should reconnect");
    select_with_replay(&mut persistence, &session, &fixture, &selection)
        .expect("failure-matrix selection should persist");
    let predicate = if let Some(event_type) = replay_event_type {
        format!("NEW.session_id = '{session}' AND NEW.event_type = '{event_type}'")
    } else if matches!(
        table,
        "route_operations" | "inventory_reward_grants" | "progression_reward_grants"
    ) {
        format!("NEW.session_id = '{session}'")
    } else {
        format!("NEW.character_id = '{}'", fixture.character)
    };
    {
        let _trigger = FailureTrigger::install_on(database_url, label, table, event, &predicate);
        assert!(matches!(
            persistence.complete_route_operation_with_replay(&session, 90_000, &terminal),
            Err(RoutePersistenceError::Database(_))
        ));
    }
    let mut client = Client::connect(database_url, NoTls).expect("assertion connection");
    let rollback = client
        .query_one(
            "SELECT \
               (SELECT COUNT(*) FROM route_operations WHERE session_id = $1 \
                    AND terminal_outcome IS NOT NULL), \
               (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), \
               (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1), \
               (SELECT COUNT(*) FROM activity_history WHERE character_id = $2), \
               (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
                    AND event_type IN ('activity_completed', 'loot_granted', \
                                       'progression_granted', 'route_operation_succeeded')), \
               (SELECT COUNT(*) FROM inventory WHERE character_id = $2 \
                    AND item_id = 'relay_core_fragment'), \
               (SELECT experience FROM progression WHERE character_id = $2), \
               (SELECT level FROM characters WHERE id = $2), \
               (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
                    AND event_type = 'route_selected')",
            &[&session, &fixture.character],
        )
        .expect("terminal rollback state should query");
    assert_eq!(
        (0..7)
            .map(|index| rollback.get::<_, i64>(index))
            .collect::<Vec<_>>(),
        vec![0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(rollback.get::<_, i32>(7), 1);
    assert_eq!(rollback.get::<_, i64>(8), 1);
    assert_eq!(
        persistence
            .complete_route_operation_with_replay(&session, 90_000, &terminal)
            .expect("rolled-back terminal should remain reusable")
            .disposition,
        OperationDisposition::Applied
    );
    assert_eq!(
        reconstruct_session(&mut persistence, &session).route.state,
        ReconstructedRouteState::Succeeded
    );
}

#[test]
fn replay_mode_failures_roll_back_every_selection_and_terminal_write() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    for (label, table) in [
        ("selection_parent", "route_operations"),
        ("selection_participant", "route_operation_participants"),
        ("selection_replay", "replay_events"),
    ] {
        assert_replay_selection_write_failure(&database_url, label, table);
    }
    for (label, table, event, replay_event_type) in [
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
        ("terminal", "route_operations", "UPDATE", None),
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
            Some("route_operation_succeeded"),
        ),
    ] {
        assert_replay_terminal_write_failure(&database_url, label, table, event, replay_event_type);
    }

    let failed_session = format!("m27-replay-failed-terminal-{}", unique_suffix());
    let fixture = prepare(&database_url, "failed_terminal", &failed_session);
    let selection = selection(&fixture, RouteId::Stabilize, 3);
    let terminal = failure_terminal(&selection);
    let mut persistence =
        Persistence::connect_existing(&database_url).expect("test persistence should reconnect");
    select_with_replay(&mut persistence, &failed_session, &fixture, &selection)
        .expect("timeout failure selection should persist");
    {
        let _trigger =
            FailureTrigger::install(&database_url, &failed_session, "route_operation_failed");
        assert!(matches!(
            persistence.fail_route_operation_timeout_with_replay(
                &failed_session,
                90_001,
                &terminal
            ),
            Err(RoutePersistenceError::Database(_))
        ));
    }
    let mut client = Client::connect(&database_url, NoTls).expect("assertion connection");
    let rollback = client
        .query_one(
            "SELECT \
               (SELECT COUNT(*) FROM route_operations WHERE session_id = $1 \
                    AND terminal_outcome IS NOT NULL), \
               (SELECT COUNT(*) FROM replay_events WHERE session_id = $1 \
                    AND event_type = 'route_operation_failed')",
            &[&failed_session],
        )
        .expect("failed-terminal rollback should query");
    assert_eq!(rollback.get::<_, i64>(0), 0);
    assert_eq!(rollback.get::<_, i64>(1), 0);
    assert_eq!(
        persistence
            .fail_route_operation_timeout_with_replay(&failed_session, 90_001, &terminal)
            .expect("rolled-back timeout should remain reusable")
            .disposition,
        OperationDisposition::Applied
    );
    assert_eq!(
        reconstruct_session(&mut persistence, &failed_session)
            .route
            .state,
        ReconstructedRouteState::Failed
    );
}
