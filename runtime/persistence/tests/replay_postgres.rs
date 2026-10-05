use std::env;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use revenant_persistence::{
    ActivityCompletion, ActivityParticipantCompletion, NewReplayEvent, Persistence,
};
use revenant_progression::ExperienceReward;

fn test_persistence(database_url: &str) -> Persistence {
    static SCHEMA_READY: OnceLock<()> = OnceLock::new();
    SCHEMA_READY.get_or_init(|| {
        drop(Persistence::connect(database_url).expect("PostgreSQL schema should initialize"));
    });
    Persistence::connect_existing(database_url).expect("PostgreSQL should connect")
}

#[test]
fn field_excursions_remain_readable_in_the_authoritative_inspector_summary() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; field summary integration skipped");
        return;
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let account_id = format!("local:field-summary-{suffix}");
    let mut persistence = test_persistence(&database_url);
    persistence
        .ensure_local_account(&account_id, "field-summary")
        .unwrap();
    for (index, payloads) in [
        vec!["coolant_run:started", "coolant_run:expired"],
        vec![
            "meridian-v1:started",
            "meridian-v1:meridian_arrival",
            "meridian-v1:meridian_lens",
        ],
    ]
    .iter()
    .enumerate()
    {
        let session_id = format!("session-field-{suffix}-{index}");
        for (event_type, payload) in [
            ("player_joined", "player joined"),
            ("activity_started", "activity started"),
        ]
        .into_iter()
        .chain(payloads.iter().map(|payload| ("field_activity", *payload)))
        {
            persistence
                .append_replay_event(&NewReplayEvent {
                    event_type,
                    session_id: &session_id,
                    account_id: &account_id,
                    activity_id: Some("relay_awakening"),
                    actor_id: Some(42),
                    payload,
                })
                .unwrap();
        }
        // Reopen the database boundary: derived evidence is durable, not cached.
        let mut reopened = Persistence::connect_existing(&database_url).unwrap();
        let summary = reopened
            .authoritative_session_summary(&session_id)
            .unwrap()
            .unwrap();
        assert!(!summary.completed);
        assert_eq!(summary.loot_grant_count, 0);
        assert_eq!(summary.progression_grant_count, 0);
        assert_eq!(summary.participant_count, 1);
    }
}

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
    let mut persistence = test_persistence(&database_url);
    persistence
        .ensure_local_account(&account_id, "replay-test")
        .expect("test account should persist");
    for event_type in [
        "player_joined",
        "activity_started",
        "enemy_spawned",
        "enemy_died",
        "boss_spawned",
        "equipment_changed",
        "activity_completed",
        "loot_granted",
        "progression_granted",
    ] {
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
    assert_eq!(events.len(), 9);
    assert_eq!(events[0].event_type, "player_joined");
    assert_eq!(events[6].event_type, "activity_completed");
    let sessions = persistence
        .replay_sessions(100)
        .expect("sessions should load");
    let session = sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .expect("inserted session should be listed");
    assert_eq!(session.event_count, 9);
    assert_eq!(session.participant_count, 1);
    assert!(session.completed);
    let summary = persistence
        .authoritative_session_summary(&session_id)
        .expect("summary should derive")
        .expect("summary should exist");
    assert_eq!(summary.session_id, session_id);
    assert_eq!(summary.activity_id.as_deref(), Some("relay_awakening"));
    assert!(summary.activity_started_at.is_some());
    assert!(summary.join_to_start_ms.is_some_and(|elapsed| elapsed >= 0));
    assert!(summary
        .activity_duration_ms
        .is_some_and(|elapsed| elapsed >= 0));
    assert_eq!(summary.participant_count, 1);
    assert!(summary.completed);
    assert_eq!(summary.enemy_spawn_count, 1);
    assert_eq!(summary.enemy_defeat_count, 1);
    assert!(summary.boss_spawned);
    assert_eq!(summary.equipment_change_count, 1);
    assert_eq!(summary.loot_grant_count, 1);
    assert_eq!(summary.progression_grant_count, 1);
    assert_eq!(summary.event_count, 9);
    assert_eq!(
        persistence
            .latest_completed_session_id(&account_id)
            .expect("latest session should query")
            .as_deref(),
        Some(session_id.as_str())
    );
}

#[test]
fn derives_incomplete_and_multiplayer_session_summaries() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    let account_one = format!("local:summary-one-{suffix}");
    let account_two = format!("local:summary-two-{suffix}");
    let no_start_session = format!("summary-no-start-{suffix}");
    let disconnected_session = format!("summary-disconnected-{suffix}");
    let mut persistence = test_persistence(&database_url);
    for (account, username) in [(&account_one, "summary-one"), (&account_two, "summary-two")] {
        persistence
            .ensure_local_account(account, username)
            .expect("test account should persist");
    }
    for account in [&account_one, &account_two] {
        persistence
            .append_replay_event(&NewReplayEvent {
                event_type: "player_joined",
                session_id: &no_start_session,
                account_id: account,
                activity_id: Some("relay_awakening"),
                actor_id: None,
                payload: "player joined",
            })
            .expect("join should append");
    }
    let no_start = persistence
        .authoritative_session_summary(&no_start_session)
        .expect("summary should derive")
        .expect("summary should exist");
    assert_eq!(no_start.participant_count, 2);
    assert!(!no_start.completed);
    assert_eq!(no_start.activity_started_at, None);
    assert_eq!(no_start.join_to_start_ms, None);
    assert_eq!(no_start.activity_duration_ms, None);
    assert_eq!(no_start.event_count, 2);

    for event_type in ["player_joined", "activity_started", "enemy_spawned"] {
        persistence
            .append_replay_event(&NewReplayEvent {
                event_type,
                session_id: &disconnected_session,
                account_id: &account_one,
                activity_id: Some("relay_awakening"),
                actor_id: None,
                payload: event_type,
            })
            .expect("event should append");
    }
    let disconnected = persistence
        .authoritative_session_summary(&disconnected_session)
        .expect("summary should derive")
        .expect("summary should exist");
    assert!(disconnected.activity_started_at.is_some());
    assert!(disconnected.join_to_start_ms.is_some());
    assert_eq!(disconnected.activity_duration_ms, None);
    assert!(!disconnected.completed);
    let disconnected_events = persistence
        .replay_events(&disconnected_session)
        .expect("events should load");
    assert_eq!(
        disconnected.activity_ended_at,
        disconnected_events
            .last()
            .expect("event should exist")
            .timestamp
    );
    assert_eq!(disconnected.enemy_spawn_count, 1);
    assert_eq!(
        persistence
            .authoritative_session_summary(&format!("missing-{suffix}"))
            .expect("missing lookup should succeed"),
        None
    );

    let unknown_session = format!("summary-unknown-{suffix}");
    for event_type in ["player_joined", "future_event"] {
        persistence
            .append_replay_event(&NewReplayEvent {
                event_type,
                session_id: &unknown_session,
                account_id: &account_one,
                activity_id: Some("relay_awakening"),
                actor_id: None,
                payload: event_type,
            })
            .expect("event should append");
    }
    assert!(persistence
        .authoritative_session_summary(&unknown_session)
        .expect_err("unknown replay vocabulary should be rejected")
        .to_string()
        .contains("unknown replay event kind"));
}

#[test]
#[allow(clippy::too_many_lines)]
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
    let mut persistence = test_persistence(&database_url);
    persistence
        .ensure_local_account(&account_id, "reward-test")
        .expect("test account should persist");
    assert_eq!(
        persistence
            .equipped_weapon_for(&character_id)
            .expect("loadout should query")
            .as_deref(),
        Some("pulse_rifle")
    );
    assert!(persistence
        .equip_weapon(&character_id, "arc_sidearm")
        .expect("owned sidearm should equip"));
    assert_eq!(
        persistence
            .equipped_weapon_for(&character_id)
            .expect("loadout should query")
            .as_deref(),
        Some("arc_sidearm")
    );

    let first = persistence
        .complete_activity_with_rewards(&ActivityCompletion {
            session_id: &session_id,
            account_id: &account_id,
            character_id: &character_id,
            activity_id: "relay_awakening",
            item_id: "relay_core_fragment",
            item_quantity: 1,
            experience_reward: ExperienceReward::validated(100)
                .expect("experience should validate"),
        })
        .expect("first reward should persist");
    let duplicate = persistence
        .complete_activity_with_rewards(&ActivityCompletion {
            session_id: &session_id,
            account_id: &account_id,
            character_id: &character_id,
            activity_id: "relay_awakening",
            item_id: "relay_core_fragment",
            item_quantity: 1,
            experience_reward: ExperienceReward::validated(100)
                .expect("experience should validate"),
        })
        .expect("duplicate reward should be harmless");

    let first = first.expect("first reward should be applied");
    assert_eq!(first.item_quantity, 1);
    assert_eq!(first.experience, 100);
    assert_eq!(first.level, 1);
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
    assert_eq!(
        persistence
            .progression_for(&character_id)
            .expect("progression should load")
            .expect("progression should exist"),
        revenant_persistence::Progression {
            level: 1,
            experience: 100,
        }
    );
    for index in 2..=5 {
        persistence
            .complete_activity_with_rewards(&ActivityCompletion {
                session_id: &format!("{session_id}-{index}"),
                account_id: &account_id,
                character_id: &character_id,
                activity_id: "relay_awakening",
                item_id: "relay_core_fragment",
                item_quantity: 1,
                experience_reward: ExperienceReward::validated(100)
                    .expect("experience should validate"),
            })
            .expect("later session reward should persist")
            .expect("later session should be unique");
    }
    assert_eq!(
        persistence
            .progression_for(&character_id)
            .expect("progression should load")
            .expect("progression should exist"),
        revenant_persistence::Progression {
            level: 2,
            experience: 500,
        }
    );
    assert_eq!(
        persistence
            .activity_completion_count(&account_id, "relay_awakening")
            .expect("completion count should load"),
        5
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn completion_replay_and_multiplayer_rewards_commit_as_one_idempotent_unit() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    let account_one = format!("local:atomic-one-{suffix}");
    let account_two = format!("local:atomic-two-{suffix}");
    let character_one = format!("{account_one}:operator");
    let character_two = format!("{account_two}:operator");
    let missing_character = format!("local:missing-{suffix}:operator");
    let session_id = format!("atomic-session-{suffix}");
    let mut persistence = test_persistence(&database_url);
    for (account, username) in [(&account_one, "atomic-one"), (&account_two, "atomic-two")] {
        persistence
            .ensure_local_account(account, username)
            .expect("test account should persist");
    }

    let completion_event = NewReplayEvent {
        event_type: "activity_completed",
        session_id: &session_id,
        account_id: &account_one,
        activity_id: Some("relay_awakening"),
        actor_id: None,
        payload: "activity completed",
    };
    let invalid_participants = [
        ActivityParticipantCompletion {
            completion: ActivityCompletion {
                session_id: &session_id,
                account_id: &account_one,
                character_id: &character_one,
                activity_id: "relay_awakening",
                item_id: "relay_core_fragment",
                item_quantity: 1,
                experience_reward: ExperienceReward::validated(100)
                    .expect("experience should validate"),
            },
            actor_id: 101,
        },
        ActivityParticipantCompletion {
            completion: ActivityCompletion {
                session_id: &session_id,
                account_id: &account_two,
                character_id: &missing_character,
                activity_id: "relay_awakening",
                item_id: "relay_core_fragment",
                item_quantity: 1,
                experience_reward: ExperienceReward::validated(100)
                    .expect("experience should validate"),
            },
            actor_id: 102,
        },
    ];
    assert!(persistence
        .complete_activity_with_rewards_and_replay(&completion_event, &invalid_participants)
        .is_err());
    assert!(persistence
        .replay_events(&session_id)
        .expect("rolled-back replay should query")
        .is_empty());
    assert_eq!(
        persistence
            .progression_for(&character_one)
            .expect("progression should query")
            .expect("progression should exist")
            .experience,
        0
    );
    assert!(!persistence
        .inventory_for(&character_one)
        .expect("inventory should query")
        .iter()
        .any(|item| item.item_id == "relay_core_fragment"));

    let participants = [
        ActivityParticipantCompletion {
            completion: ActivityCompletion {
                session_id: &session_id,
                account_id: &account_one,
                character_id: &character_one,
                activity_id: "relay_awakening",
                item_id: "relay_core_fragment",
                item_quantity: 1,
                experience_reward: ExperienceReward::validated(100)
                    .expect("experience should validate"),
            },
            actor_id: 101,
        },
        ActivityParticipantCompletion {
            completion: ActivityCompletion {
                session_id: &session_id,
                account_id: &account_two,
                character_id: &character_two,
                activity_id: "relay_awakening",
                item_id: "relay_core_fragment",
                item_quantity: 1,
                experience_reward: ExperienceReward::validated(100)
                    .expect("experience should validate"),
            },
            actor_id: 102,
        },
    ];
    let first = persistence
        .complete_activity_with_rewards_and_replay(&completion_event, &participants)
        .expect("atomic completion should persist");
    assert!(first.iter().all(Option::is_some));
    let duplicate = persistence
        .complete_activity_with_rewards_and_replay(&completion_event, &participants)
        .expect("duplicate completion should be harmless");
    assert_eq!(duplicate, vec![None, None]);
    let events = persistence
        .replay_events(&session_id)
        .expect("completion replay should load");
    assert_eq!(
        events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "activity_completed",
            "loot_granted",
            "progression_granted",
            "loot_granted",
            "progression_granted",
        ]
    );
    for (account, character) in [
        (&account_one, &character_one),
        (&account_two, &character_two),
    ] {
        assert_eq!(
            persistence
                .activity_completion_count(account, "relay_awakening")
                .expect("history should query"),
            1
        );
        assert_eq!(
            persistence
                .progression_for(character)
                .expect("progression should query")
                .expect("progression should exist")
                .experience,
            100
        );
    }
}

#[test]
fn equipment_and_replay_commit_or_roll_back_together() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is not set; PostgreSQL integration test skipped");
        return;
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    let account_id = format!("local:atomic-equipment-{suffix}");
    let character_id = format!("{account_id}:operator");
    let session_id = format!("atomic-equipment-session-{suffix}");
    let mut persistence = test_persistence(&database_url);
    persistence
        .ensure_local_account(&account_id, "atomic-equipment")
        .expect("test account should persist");

    let invalid_event = NewReplayEvent {
        event_type: "equipment_changed",
        session_id: &session_id,
        account_id: "local:account-does-not-exist",
        activity_id: Some("relay_awakening"),
        actor_id: Some(201),
        payload: "weapon equipped: arc_sidearm",
    };
    assert!(persistence
        .equip_weapon_with_replay(&character_id, "arc_sidearm", &invalid_event)
        .is_err());
    assert_eq!(
        persistence
            .equipped_weapon_for(&character_id)
            .expect("loadout should query")
            .as_deref(),
        Some("pulse_rifle")
    );

    let valid_event = NewReplayEvent {
        account_id: &account_id,
        ..invalid_event
    };
    assert!(persistence
        .equip_weapon_with_replay(&character_id, "arc_sidearm", &valid_event)
        .expect("equipment transaction should persist"));
    assert_eq!(
        persistence
            .equipped_weapon_for(&character_id)
            .expect("loadout should query")
            .as_deref(),
        Some("arc_sidearm")
    );
    let events = persistence
        .replay_events(&session_id)
        .expect("equipment replay should load");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "equipment_changed");
}

#[test]
fn support_pair_summary_counts_both_actors_from_one_durable_spawn() {
    use revenant_replay::{encode_support_evidence, SupportEvidence, SupportTransition};
    let Ok(database_url) = env::var("DATABASE_URL") else {
        return;
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let account = format!("local:support-summary-{suffix}");
    let session = format!("session-support-{suffix}");
    let mut persistence = test_persistence(&database_url);
    persistence
        .ensure_local_account(&account, "support-summary")
        .unwrap();
    let spawn = encode_support_evidence(&SupportEvidence {
        elapsed_ms: 0,
        transition: SupportTransition::Started {
            lancer_id: 2,
            mender_id: 3,
            player_health: 90,
        },
    })
    .unwrap();
    for (event_type, actor_id, payload) in [
        ("player_joined", 1, "player joined"),
        ("enemy_spawned", 2, spawn.as_str()),
    ] {
        persistence
            .append_replay_event(&NewReplayEvent {
                event_type,
                session_id: &session,
                account_id: &account,
                activity_id: Some("relay_awakening"),
                actor_id: Some(actor_id),
                payload,
            })
            .unwrap();
    }
    let mut reopened = Persistence::connect_existing(&database_url).unwrap();
    let summary = reopened
        .authoritative_session_summary(&session)
        .unwrap()
        .unwrap();
    assert_eq!(summary.enemy_spawn_count, 2);
    assert_eq!(summary.enemy_defeat_count, 0);
    assert_eq!(summary.loot_grant_count, 0);
    assert_eq!(summary.progression_grant_count, 0);
    assert!(!summary.completed);
}
