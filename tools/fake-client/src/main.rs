use std::env;
use std::net::TcpStream;
use std::process::ExitCode;
use std::time::Duration;

use revenant_protocol::{
    read_message, write_message, AuthRequest, CharacterListRequest, ClientHello, ClientMessage,
    ServerMessage, PROTOCOL_VERSION,
};
use revenant_protocol::{AttackIntent, MoveIntent, WorldJoinRequest};

fn main() -> ExitCode {
    match handshake() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("revenant-bot handshake failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
fn handshake() -> Result<(), Box<dyn std::error::Error>> {
    let game_addr = env::var("REVENANT_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7000".to_owned());
    let username = env::var("REVENANT_BOT_USERNAME").unwrap_or_else(|_| "revenant-bot".to_owned());
    let role = env::var("REVENANT_BOT_ROLE").unwrap_or_else(|_| "driver".to_owned());
    let expected_players = env::var("REVENANT_EXPECTED_PLAYERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let mut stream = TcpStream::connect(&game_addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    write_message(
        &mut stream,
        &ClientMessage::ClientHello(ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "revenant-bot".to_owned(),
            client_build: env!("CARGO_PKG_VERSION").to_owned(),
        }),
    )?;

    let ServerMessage::ServerHello(hello) = read_message(&mut stream)? else {
        return Err("expected ServerHello".into());
    };
    if !hello.accepted || hello.protocol_version != PROTOCOL_VERSION {
        return Err(format!(
            "server rejected protocol {}: {}",
            PROTOCOL_VERSION, hello.message
        )
        .into());
    }

    println!(
        "handshake accepted by {} using protocol v{}",
        hello.server_name, hello.protocol_version
    );

    write_message(
        &mut stream,
        &ClientMessage::AuthRequest(AuthRequest { username }),
    )?;
    let ServerMessage::AuthResponse(auth) = read_message(&mut stream)? else {
        return Err("expected AuthResponse".into());
    };
    if !auth.authenticated {
        return Err(format!("local authentication rejected: {}", auth.message).into());
    }
    println!("authenticated local account {}", auth.account_id);

    write_message(
        &mut stream,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )?;
    let ServerMessage::CharacterListResponse(character_list) = read_message(&mut stream)? else {
        return Err("expected CharacterListResponse".into());
    };
    let character = character_list
        .characters
        .first()
        .ok_or("server returned no local character")?;
    println!(
        "received character {} ({}, level {})",
        character.display_name, character.class_name, character.level
    );
    let character_id = character.character_id.clone();
    write_message(
        &mut stream,
        &ClientMessage::WorldJoinRequest(WorldJoinRequest { character_id }),
    )?;
    let ServerMessage::WorldJoinResponse(world) = read_message(&mut stream)? else {
        return Err("expected WorldJoinResponse".into());
    };
    if !world.accepted {
        return Err(format!("world join rejected: {}", world.message).into());
    }
    println!(
        "joined world {} as player actor {} at {:?}",
        world.world_id, world.player_actor_id, world.spawn_position
    );
    let ServerMessage::InventorySnapshot(inventory) = read_message(&mut stream)? else {
        return Err("expected InventorySnapshot".into());
    };
    if !inventory
        .items
        .iter()
        .any(|item| item.item_id == "pulse_rifle" && item.quantity == 1)
    {
        return Err("starter inventory snapshot is missing pulse_rifle".into());
    }
    println!("received authoritative inventory snapshot");
    let ServerMessage::ProgressionSnapshot(progression) = read_message(&mut stream)? else {
        return Err("expected ProgressionSnapshot".into());
    };
    if progression.level == 0 || progression.experience_to_next_level == 0 {
        return Err("authoritative progression snapshot is invalid".into());
    }
    println!(
        "received authoritative progression: level {} with {} XP",
        progression.level, progression.experience
    );
    let ServerMessage::EquipmentSnapshot(equipment) = read_message(&mut stream)? else {
        return Err("expected EquipmentSnapshot".into());
    };
    if equipment.weapons.len() != 2
        || !equipment
            .weapons
            .iter()
            .any(|weapon| weapon.item_id == "pulse_rifle" && weapon.damage == 40)
        || !equipment
            .weapons
            .iter()
            .any(|weapon| weapon.item_id == "arc_sidearm" && weapon.damage == 25)
    {
        return Err("authoritative equipment profiles are incomplete".into());
    }
    println!(
        "received authoritative loadout: {}",
        equipment.equipped_weapon_item_id
    );
    if role != "observer" {
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: "relay_core_fragment".to_owned(),
            }),
        )?;
        write_message(
            &mut stream,
            &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
                item_id: "arc_sidearm".to_owned(),
            }),
        )?;
    }
    let ServerMessage::ActivityStart(activity) = read_message(&mut stream)? else {
        return Err("expected ActivityStart".into());
    };
    let ServerMessage::ObjectiveUpdate(initial_objective) = read_message(&mut stream)? else {
        return Err("expected initial ObjectiveUpdate".into());
    };
    println!(
        "activity {} started with {} objective {}",
        activity.activity_id, initial_objective.state, initial_objective.objective_id
    );
    let mut enemy_id = None;
    let mut player_count = 0;
    for _ in 0..=expected_players {
        let ServerMessage::ActorSpawn(actor) = read_message(&mut stream)? else {
            return Err("expected ActorSpawn".into());
        };
        println!(
            "actor {} spawned as {} ({}) at {:?}",
            actor.actor_id, actor.actor_kind, actor.archetype, actor.position
        );
        if actor.actor_kind == "enemy" {
            enemy_id = Some(actor.actor_id);
        } else if actor.actor_kind == "player" {
            player_count += 1;
        }
    }
    if player_count != expected_players {
        return Err(
            format!("expected {expected_players} replicated players, got {player_count}").into(),
        );
    }
    let enemy_id = enemy_id.ok_or("server did not spawn the first enemy actor")?;
    if role == "observer" {
        return observe_shared_activity(&mut stream, enemy_id, expected_players);
    }
    observe_enemy_ai(&mut stream, enemy_id)?;
    let ServerMessage::EquipmentChanged(rejected) = read_message(&mut stream)? else {
        return Err("expected rejected EquipmentChanged".into());
    };
    if rejected.accepted || !rejected.message.contains("not an equipable weapon") {
        return Err("non-weapon equipment intent was not rejected".into());
    }
    let ServerMessage::EquipmentChanged(equipment) = read_message(&mut stream)? else {
        return Err("expected EquipmentChanged".into());
    };
    if !equipment.accepted || equipment.equipped_weapon_item_id != "arc_sidearm" {
        return Err(format!("sidearm equip rejected: {}", equipment.message).into());
    }
    println!("equipped authoritative weapon arc_sidearm");
    defeat_enemy(&mut stream, enemy_id)?;
    verify_objective_progression(&mut stream)?;
    complete_activity(&mut stream, world.player_actor_id)?;
    Ok(())
}

fn complete_activity(
    stream: &mut TcpStream,
    player_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    write_message(
        stream,
        &ClientMessage::MoveIntent(MoveIntent {
            position: [6, 0, 0],
        }),
    )?;
    let ServerMessage::ActorUpdate(player_update) = read_message(stream)? else {
        return Err("expected player ActorUpdate".into());
    };
    if player_update.actor_id != player_id {
        return Err("server updated the wrong player actor".into());
    }
    let ServerMessage::ObjectiveUpdate(reach) = read_message(stream)? else {
        return Err("expected completed ReachArea".into());
    };
    let ServerMessage::ObjectiveUpdate(boss_objective) = read_message(stream)? else {
        return Err("expected active Boss objective".into());
    };
    let ServerMessage::DoorState(door) = read_message(stream)? else {
        return Err("expected open door".into());
    };
    if reach.state != "Completed" || boss_objective.state != "Active" || !door.open {
        return Err("activity did not open the boss stage".into());
    }
    let ServerMessage::ActorSpawn(boss) = read_message(stream)? else {
        return Err("expected boss ActorSpawn".into());
    };
    if boss.archetype != "warden" {
        return Err("unexpected boss archetype".into());
    }
    println!(
        "door {} opened; boss {} spawned",
        door.door_id, boss.archetype
    );
    defeat_enemy(stream, boss.actor_id)?;
    let ServerMessage::ObjectiveUpdate(completed_boss) = read_message(stream)? else {
        return Err("expected completed Boss objective".into());
    };
    let ServerMessage::LootGranted(loot) = read_message(stream)? else {
        return Err("expected LootGranted".into());
    };
    if loot.item_id != "relay_core_fragment" || loot.quantity != 1 {
        return Err("activity granted unexpected loot".into());
    }
    let ServerMessage::ProgressionGranted(progression) = read_message(stream)? else {
        return Err("expected ProgressionGranted".into());
    };
    if progression.experience_granted != 100 || progression.level < progression.previous_level {
        return Err("activity granted unexpected progression".into());
    }
    let ServerMessage::ActivityComplete(completed) = read_message(stream)? else {
        return Err("expected ActivityComplete".into());
    };
    if completed_boss.state != "Completed" {
        return Err("boss objective did not complete".into());
    }
    println!(
        "activity {} completed; received {} x{} (total {})",
        completed.activity_id, loot.item_id, loot.quantity, loot.resulting_quantity
    );
    println!(
        "received {} XP (total {}, level {})",
        progression.experience_granted, progression.experience, progression.level
    );
    Ok(())
}

fn verify_objective_progression(stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let ServerMessage::ObjectiveUpdate(kill) = read_message(stream)? else {
        return Err("expected completed KillActors objective".into());
    };
    let ServerMessage::ObjectiveUpdate(reach) = read_message(stream)? else {
        return Err("expected active ReachArea objective".into());
    };
    if kill.objective_type != "KillActors" || kill.state != "Completed" {
        return Err("KillActors objective did not complete".into());
    }
    if reach.objective_type != "ReachArea" || reach.state != "Active" {
        return Err("ReachArea objective did not open".into());
    }
    println!(
        "objective {} completed; next objective {} is active",
        kill.objective_id, reach.objective_id
    );
    Ok(())
}

fn observe_enemy_ai(
    stream: &mut TcpStream,
    enemy_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut moved = false;
    loop {
        match read_message(stream)? {
            ServerMessage::ActorUpdate(update) if update.actor_id == enemy_id => {
                moved = true;
                println!("enemy actor {enemy_id} chased to {:?}", update.position);
            }
            ServerMessage::DamageApplied(damage) if damage.source_actor_id == enemy_id => {
                if !moved {
                    return Err("enemy attacked without a chase update".into());
                }
                println!(
                    "enemy actor {enemy_id} dealt {} damage; player has {} HP",
                    damage.damage, damage.remaining_health
                );
                return Ok(());
            }
            _ => return Err("unexpected message while observing enemy AI".into()),
        }
    }
}

fn observe_shared_activity(
    stream: &mut TcpStream,
    first_enemy_id: u64,
    expected_players: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut enemy_destroyed = false;
    let mut boss_spawned = false;
    let mut boss_destroyed = false;
    let mut objective_updates = 1;
    let mut loot_received = false;
    let mut progression_received = false;
    let mut equipment_change_received = false;
    loop {
        match read_message(stream)? {
            ServerMessage::ActorDestroy(destroy) if destroy.actor_id == first_enemy_id => {
                enemy_destroyed = true;
            }
            ServerMessage::ActorSpawn(actor) if actor.archetype == "warden" => {
                boss_spawned = true;
            }
            ServerMessage::ActorDestroy(_) if boss_spawned => boss_destroyed = true,
            ServerMessage::ObjectiveUpdate(_) => objective_updates += 1,
            ServerMessage::LootGranted(loot) if loot.item_id == "relay_core_fragment" => {
                loot_received = true;
            }
            ServerMessage::ProgressionGranted(progression)
                if progression.experience_granted == 100 =>
            {
                progression_received = true;
            }
            ServerMessage::EquipmentChanged(equipment) if equipment.accepted => {
                equipment_change_received = true;
            }
            ServerMessage::ActivityComplete(activity) => {
                if !enemy_destroyed
                    || !boss_spawned
                    || !boss_destroyed
                    || objective_updates < 5
                    || !loot_received
                    || !progression_received
                    || !equipment_change_received
                {
                    return Err("observer missed shared activity state".into());
                }
                println!(
                    "observer received {expected_players} players and shared completion for {}",
                    activity.activity_id
                );
                return Ok(());
            }
            ServerMessage::ActorUpdate(_)
            | ServerMessage::DamageApplied(_)
            | ServerMessage::DoorState(_) => {}
            _ => return Err("observer received an unexpected shared message".into()),
        }
    }
}

fn defeat_enemy(stream: &mut TcpStream, enemy_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        write_message(
            stream,
            &ClientMessage::AttackIntent(AttackIntent {
                target_actor_id: enemy_id,
            }),
        )?;
        let ServerMessage::DamageApplied(damage) = read_message(stream)? else {
            return Err("expected DamageApplied".into());
        };
        println!(
            "dealt {} damage to actor {}; {} HP remains",
            damage.damage, damage.target_actor_id, damage.remaining_health
        );
        if damage.killed {
            break;
        }
        std::thread::sleep(Duration::from_millis(260));
    }
    let ServerMessage::ActorDestroy(destroy) = read_message(stream)? else {
        return Err("expected ActorDestroy".into());
    };
    println!("enemy actor {} destroyed", destroy.actor_id);
    Ok(())
}
