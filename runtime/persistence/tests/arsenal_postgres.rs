use revenant_persistence::{ModuleJoinReplayContext, NewReplayEvent, Persistence};
use revenant_replay::{reconstruct, ReplayEvent, ReplayProtocolGeneration};

#[test]
fn arsenal_inventory_equipment_and_replay_are_atomic_and_survive_reconnect() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let account = format!("local:m35-{stamp}");
    let character = format!("{account}:operator");
    let session = format!("m35-{stamp}");
    let mut database = Persistence::connect(&url).unwrap();
    database.ensure_local_account(&account, "arsenal").unwrap();
    assert_eq!(database.inventory_for(&character).unwrap().len(), 2);
    database.ensure_content_equipment(&character).unwrap();
    database.ensure_arsenal_equipment(&character).unwrap();
    database.ensure_arsenal_equipment(&character).unwrap();
    database
        .append_player_joined_with_module_snapshot(
            &character,
            ModuleJoinReplayContext {
                catalog_revision: Some("m35-v1"),
                session_id: &session,
                account_id: &account,
                activity_id: "relay_awakening",
                actor_id: 35,
                player_joined_payload: "player joined",
                protocol_generation: ReplayProtocolGeneration::V2,
            },
        )
        .unwrap();
    let event = NewReplayEvent {
        event_type: "equipment_changed",
        session_id: &session,
        account_id: &account,
        activity_id: Some("relay_awakening"),
        actor_id: Some(35),
        payload: "weapon equipped: rail_driver",
    };
    assert!(database
        .equip_weapon_with_replay(&character, "rail_driver", &event)
        .unwrap());
    // PostgreSQL rejects an embedded NUL after the equipment UPDATE. Neither
    // the selection nor the event may survive the rolled-back transaction.
    assert!(database
        .equip_weapon_with_replay(
            &character,
            "scatter_caster",
            &NewReplayEvent {
                payload: "invalid\0event",
                ..event
            }
        )
        .is_err());
    drop(database);
    let mut database = Persistence::connect_existing(&url).unwrap();
    database.ensure_arsenal_equipment(&character).unwrap();
    assert_eq!(
        database.equipped_weapon_for(&character).unwrap().as_deref(),
        Some("rail_driver")
    );
    let inventory = database.inventory_for(&character).unwrap();
    assert_eq!(inventory.len(), 5);
    assert!(inventory.iter().all(|item| item.quantity == 1));
    assert!(database
        .ensure_arsenal_equipment("missing-m35-character")
        .is_err());
    let events = database
        .replay_events(&session)
        .unwrap()
        .into_iter()
        .map(|event| ReplayEvent {
            id: event.id,
            kind: event.event_type.parse().unwrap(),
            timestamp: event.timestamp,
            session_id: event.session_id,
            account_id: event.account_id,
            activity_id: event.activity_id,
            actor_id: event.actor_id.map(|id| u64::try_from(id).unwrap()),
            payload: event.payload,
        })
        .collect::<Vec<_>>();
    let replay = reconstruct(&events).unwrap();
    assert_eq!(replay.equipment_changes, 1);
    assert_eq!(
        replay.module_participants[0].state.catalog_revision,
        "m35-v1"
    );
    assert_eq!(replay.module_participants[0].state.weapons.len(), 5);
}
