use std::env;
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const FROZEN_PROTOCOL_VERSION: u16 = 1;
const FROZEN_BUILD: &str = "0.1.0-frozen";
const MAX_FRAME_SIZE: usize = 64 * 1024;

fn main() -> ExitCode {
    match connect() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("frozen Revenant client V1 failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn connect() -> Result<(), Box<dyn Error>> {
    let game_addr = env::var("REVENANT_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7000".to_owned());
    let mut stream = TcpStream::connect(game_addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    write_message(
        &mut stream,
        &ClientMessage::ClientHello(ClientHello {
            protocol_version: FROZEN_PROTOCOL_VERSION,
            client_name: "revenant-client-v1".to_owned(),
            client_build: FROZEN_BUILD.to_owned(),
        }),
    )?;
    let ServerMessage::ServerHello(hello) = read_message(&mut stream)? else {
        return Err("expected ServerHello".into());
    };
    if !hello.accepted || hello.protocol_version != FROZEN_PROTOCOL_VERSION {
        return Err(format!("V1 compatibility rejected: {}", hello.message).into());
    }

    write_message(
        &mut stream,
        &ClientMessage::AuthRequest(AuthRequest {
            username: "revenant-frozen-v1".to_owned(),
        }),
    )?;
    let ServerMessage::AuthResponse(auth) = read_message(&mut stream)? else {
        return Err("expected AuthResponse".into());
    };
    if !auth.authenticated {
        return Err(format!("authentication rejected: {}", auth.message).into());
    }

    write_message(
        &mut stream,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )?;
    let ServerMessage::CharacterListResponse(characters) = read_message(&mut stream)? else {
        return Err("expected CharacterListResponse".into());
    };
    let character = characters
        .characters
        .first()
        .ok_or("frozen account has no character")?;
    write_message(
        &mut stream,
        &ClientMessage::WorldJoinRequest(WorldJoinRequest {
            character_id: character.character_id.clone(),
        }),
    )?;
    let ServerMessage::WorldJoinResponse(world) = read_message(&mut stream)? else {
        return Err("expected WorldJoinResponse".into());
    };
    if !world.accepted {
        return Err(format!("world join rejected: {}", world.message).into());
    }
    println!(
        "frozen client V1 build {FROZEN_BUILD} joined {} as actor {} through compatibility adapter",
        world.world_id, world.player_actor_id
    );
    Ok(())
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ClientMessage {
    ClientHello(ClientHello),
    AuthRequest(AuthRequest),
    CharacterListRequest(CharacterListRequest),
    WorldJoinRequest(WorldJoinRequest),
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ServerMessage {
    ServerHello(ServerHello),
    AuthResponse(AuthResponse),
    CharacterListResponse(CharacterListResponse),
    WorldJoinResponse(WorldJoinResponse),
}

#[derive(Serialize)]
struct ClientHello {
    protocol_version: u16,
    client_name: String,
    client_build: String,
}

#[derive(Deserialize)]
struct ServerHello {
    protocol_version: u16,
    #[allow(dead_code)]
    server_name: String,
    accepted: bool,
    message: String,
}

#[derive(Serialize)]
struct AuthRequest {
    username: String,
}

#[derive(Deserialize)]
struct AuthResponse {
    authenticated: bool,
    #[allow(dead_code)]
    account_id: String,
    message: String,
}

#[derive(Serialize)]
struct CharacterListRequest {}

#[derive(Deserialize)]
struct CharacterListResponse {
    characters: Vec<CharacterSummary>,
}

#[derive(Deserialize)]
struct CharacterSummary {
    character_id: String,
    #[allow(dead_code)]
    display_name: String,
    #[allow(dead_code)]
    class_name: String,
    #[allow(dead_code)]
    level: u16,
}

#[derive(Serialize)]
struct WorldJoinRequest {
    character_id: String,
}

#[derive(Deserialize)]
struct WorldJoinResponse {
    accepted: bool,
    world_id: String,
    player_actor_id: u64,
    #[allow(dead_code)]
    spawn_position: [i32; 3],
    message: String,
}

fn write_message<T: Serialize>(stream: &mut TcpStream, message: &T) -> Result<(), Box<dyn Error>> {
    let payload = rmp_serde::to_vec_named(message)?;
    if payload.len() > MAX_FRAME_SIZE {
        return Err("frozen client frame exceeds V1 limit".into());
    }
    let size = u32::try_from(payload.len())?;
    stream.write_all(&size.to_be_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    Ok(())
}

fn read_message<T: DeserializeOwned>(stream: &mut TcpStream) -> Result<T, Box<dyn Error>> {
    let mut size = [0_u8; 4];
    stream.read_exact(&mut size)?;
    let size = usize::try_from(u32::from_be_bytes(size))?;
    if size > MAX_FRAME_SIZE {
        return Err("server frame exceeds frozen V1 limit".into());
    }
    let mut payload = vec![0_u8; size];
    stream.read_exact(&mut payload)?;
    Ok(rmp_serde::from_slice(&payload)?)
}
