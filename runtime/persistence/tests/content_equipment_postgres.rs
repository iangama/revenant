use revenant_modules::{ActivityPhase, ModuleId, OperationDisposition};
use revenant_persistence::Persistence;

#[test]
fn content_equipment_is_opted_in_idempotent_and_survives_reconnect() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; content equipment integration test skipped");
        return;
    };
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let account = format!("local:m31-equipment-{suffix}");
    let character = format!("{account}:operator");
    let mut database = Persistence::connect(&url).unwrap();
    database
        .ensure_local_account(&account, "content-equipment")
        .unwrap();
    assert_eq!(database.inventory_for(&character).unwrap().len(), 2);
    database.ensure_content_equipment(&character).unwrap();
    database.ensure_content_equipment(&character).unwrap();
    drop(database);
    let mut database = Persistence::connect_existing(&url).unwrap();
    database
        .ensure_local_account(&account, "content-equipment")
        .unwrap();
    database.ensure_content_equipment(&character).unwrap();
    let inventory = database.inventory_for(&character).unwrap();
    assert_eq!(inventory.len(), 3);
    assert_eq!(
        inventory
            .iter()
            .find(|item| item.item_id == "coil_lance")
            .unwrap()
            .quantity,
        1
    );
    assert_eq!(
        database.equipped_weapon_for(&character).unwrap().as_deref(),
        Some("pulse_rifle")
    );
    assert!(database
        .ensure_content_equipment("nonexistent-character")
        .is_err());

    // Disposable test character: exercise the new catalog identifiers through
    // the existing atomic fragment/receipt and optimistic-loadout paths.
    let mut fixture = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    fixture.execute("INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, 'relay_core_fragment', 4)", &[&character]).unwrap();
    for (operation, module) in [
        ("focus", ModuleId::FocusLens),
        ("cycle", ModuleId::CycleBypass),
    ] {
        let first = database
            .combine_module(&character, ActivityPhase::Complete, operation, module)
            .unwrap();
        let retry = database
            .combine_module(&character, ActivityPhase::Active, operation, module)
            .unwrap();
        assert_eq!(retry.disposition, OperationDisposition::Replayed);
        assert_eq!(first.outcome, retry.outcome);
    }
    database
        .set_module_loadout(
            &character,
            ActivityPhase::Complete,
            "content-loadout",
            0,
            &[ModuleId::FocusLens, ModuleId::CycleBypass],
        )
        .unwrap();
    assert!(database
        .set_module_loadout(&character, ActivityPhase::Complete, "stale-loadout", 0, &[])
        .is_err());
    drop(database);
    let mut database = Persistence::connect_existing(&url).unwrap();
    let state = database.module_state_for(&character).unwrap();
    assert_eq!(state.fragments, 0);
    assert_eq!(
        state.owned_modules,
        [ModuleId::FocusLens, ModuleId::CycleBypass]
    );
    assert_eq!(state.loadout, state.owned_modules);
    assert_eq!(state.revision, 1);
}
