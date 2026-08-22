use std::env;
use std::error::Error;
use std::io;

use revenant_persistence::Persistence;

const DEFAULT_DATABASE_URL: &str = "postgres://revenant:revenant_local@127.0.0.1:5432/revenant";

fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    let account_id =
        env::var("REVENANT_EXPECT_ACCOUNT").unwrap_or_else(|_| "local:revenant-bot".to_owned());
    let character_id = format!("{account_id}:operator");
    let mut persistence = Persistence::connect(&database_url)?;

    let characters = persistence.characters_for(&account_id)?;
    if !characters
        .iter()
        .any(|character| character.id == character_id)
    {
        return Err(io::Error::other("persisted operator was not found").into());
    }
    let inventory = persistence.inventory_for(&character_id)?;
    if !inventory
        .iter()
        .any(|entry| entry.item_id == "pulse_rifle" && entry.quantity == 1)
    {
        return Err(io::Error::other("persisted starter inventory was not found").into());
    }
    if !inventory
        .iter()
        .any(|entry| entry.item_id == "relay_core_fragment" && entry.quantity >= 1)
    {
        return Err(io::Error::other("persisted activity reward was not found").into());
    }
    let progression = persistence.progression_for(&character_id)?;
    if progression.is_none() {
        return Err(io::Error::other("persisted progression was not found").into());
    }
    let completions = persistence.activity_completion_count(&account_id, "relay_awakening")?;
    if completions < 1 {
        return Err(io::Error::other("persisted activity completion was not found").into());
    }

    println!(
        "persistence verified: account={account_id} character={character_id} completions={completions}"
    );
    Ok(())
}
