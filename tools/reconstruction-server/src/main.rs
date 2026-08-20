use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::ExitCode;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const FROZEN_PROTOCOL_VERSION: u16 = 1;
const FROZEN_CLIENT_NAME: &str = "revenant-client-v1";
const FROZEN_CLIENT_BUILD: &str = "0.1.0-frozen";
const MAX_FRAME_SIZE: usize = 64 * 1024;

fn main() -> ExitCode {
    match serve_once() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("reconstructed V1 backend failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn serve_once() -> Result<(), Box<dyn Error>> {
    let address =
        env::var("REVENANT_RECONSTRUCTION_ADDR").unwrap_or_else(|_| "127.0.0.1:17015".to_owned());
    let listener = TcpListener::bind(&address)?;
    println!("reconstructed V1 backend listening on {address}");
    let (mut stream, _) = listener.accept()?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    reconstruct_session(&mut stream)?;
    println!(
        "reconstruction complete: frozen V1 client entered relay-hub without revenant-gateway"
    );
    Ok(())
}

fn reconstruct_session(stream: &mut TcpStream) -> Result<(), ReconstructionError> {
    let ClientMessage::ClientHello(hello) = read_message(stream)? else {
        return Err(ReconstructionError::UnexpectedMessage("ClientHello"));
    };
    let recognized = hello.protocol_version == FROZEN_PROTOCOL_VERSION
        && hello.client_name == FROZEN_CLIENT_NAME
        && hello.client_build == FROZEN_CLIENT_BUILD;
    write_message(
        stream,
        &ServerMessage::ServerHello(ServerHello {
            protocol_version: FROZEN_PROTOCOL_VERSION,
            server_name: "revenant-reconstructed-v1".to_owned(),
            accepted: recognized,
            message: if recognized {
                "reconstructed V1 contract accepted"
            } else {
                "client does not match preserved V1 evidence"
            }
            .to_owned(),
        }),
    )?;
    if !recognized {
        return Ok(());
    }

    let ClientMessage::AuthRequest(auth) = read_message(stream)? else {
        return Err(ReconstructionError::UnexpectedMessage("AuthRequest"));
    };
    if auth.username.is_empty() {
        return Err(ReconstructionError::InvalidEvidence("empty username"));
    }
    let account_id = format!("reconstructed:{}", auth.username);
    write_message(
        stream,
        &ServerMessage::AuthResponse(AuthResponse {
            authenticated: true,
            account_id: account_id.clone(),
            message: "reconstructed local identity".to_owned(),
        }),
    )?;

    let ClientMessage::CharacterListRequest(_) = read_message(stream)? else {
        return Err(ReconstructionError::UnexpectedMessage(
            "CharacterListRequest",
        ));
    };
    let character_id = format!("{account_id}:operator");
    write_message(
        stream,
        &ServerMessage::CharacterListResponse(CharacterListResponse {
            characters: vec![CharacterSummary {
                character_id: character_id.clone(),
                display_name: "Reconstructed Operator".to_owned(),
                class_name: "Operator".to_owned(),
                level: 1,
            }],
        }),
    )?;

    let ClientMessage::WorldJoinRequest(join) = read_message(stream)? else {
        return Err(ReconstructionError::UnexpectedMessage("WorldJoinRequest"));
    };
    let accepted = join.character_id == character_id;
    write_message(
        stream,
        &ServerMessage::WorldJoinResponse(WorldJoinResponse {
            accepted,
            world_id: if accepted { "relay-hub" } else { "" }.to_owned(),
            player_actor_id: u64::from(accepted),
            spawn_position: [0, 0, 0],
            message: if accepted {
                "world reconstructed from preserved V1 evidence"
            } else {
                "unknown reconstructed character"
            }
            .to_owned(),
        }),
    )?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    ClientHello(ClientHello),
    AuthRequest(AuthRequest),
    CharacterListRequest(CharacterListRequest),
    WorldJoinRequest(WorldJoinRequest),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    ServerHello(ServerHello),
    AuthResponse(AuthResponse),
    CharacterListResponse(CharacterListResponse),
    WorldJoinResponse(WorldJoinResponse),
}

#[derive(Debug, Deserialize)]
struct ClientHello {
    protocol_version: u16,
    client_name: String,
    client_build: String,
}

#[derive(Debug, Serialize)]
struct ServerHello {
    protocol_version: u16,
    server_name: String,
    accepted: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct AuthRequest {
    username: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    authenticated: bool,
    account_id: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct CharacterListRequest {}

#[derive(Debug, Serialize)]
struct CharacterListResponse {
    characters: Vec<CharacterSummary>,
}

#[derive(Debug, Serialize)]
struct CharacterSummary {
    character_id: String,
    display_name: String,
    class_name: String,
    level: u16,
}

#[derive(Debug, Deserialize)]
struct WorldJoinRequest {
    character_id: String,
}

#[derive(Debug, Serialize)]
struct WorldJoinResponse {
    accepted: bool,
    world_id: String,
    player_actor_id: u64,
    spawn_position: [i32; 3],
    message: String,
}

#[derive(Debug)]
enum ReconstructionError {
    Io(std::io::Error),
    Encode(rmp_serde::encode::Error),
    Decode(rmp_serde::decode::Error),
    FrameTooLarge(usize),
    UnexpectedMessage(&'static str),
    InvalidEvidence(&'static str),
}

impl Display for ReconstructionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "V1 I/O failed: {error}"),
            Self::Encode(error) => write!(formatter, "V1 encoding failed: {error}"),
            Self::Decode(error) => write!(formatter, "V1 decoding failed: {error}"),
            Self::FrameTooLarge(size) => {
                write!(
                    formatter,
                    "V1 frame is {size} bytes; maximum is {MAX_FRAME_SIZE}"
                )
            }
            Self::UnexpectedMessage(expected) => write!(formatter, "expected {expected}"),
            Self::InvalidEvidence(reason) => {
                write!(formatter, "invalid preserved evidence: {reason}")
            }
        }
    }
}

impl Error for ReconstructionError {}

impl From<std::io::Error> for ReconstructionError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rmp_serde::encode::Error> for ReconstructionError {
    fn from(error: rmp_serde::encode::Error) -> Self {
        Self::Encode(error)
    }
}

impl From<rmp_serde::decode::Error> for ReconstructionError {
    fn from(error: rmp_serde::decode::Error) -> Self {
        Self::Decode(error)
    }
}

fn write_message<T: Serialize>(
    stream: &mut TcpStream,
    message: &T,
) -> Result<(), ReconstructionError> {
    let payload = rmp_serde::to_vec_named(message)?;
    if payload.len() > MAX_FRAME_SIZE {
        return Err(ReconstructionError::FrameTooLarge(payload.len()));
    }
    let size = u32::try_from(payload.len())
        .map_err(|_| ReconstructionError::FrameTooLarge(payload.len()))?;
    stream.write_all(&size.to_be_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    Ok(())
}

fn read_message<T: DeserializeOwned>(stream: &mut TcpStream) -> Result<T, ReconstructionError> {
    let mut size = [0_u8; 4];
    stream.read_exact(&mut size)?;
    let size = usize::try_from(u32::from_be_bytes(size))
        .map_err(|_| ReconstructionError::FrameTooLarge(usize::MAX))?;
    if size > MAX_FRAME_SIZE {
        return Err(ReconstructionError::FrameTooLarge(size));
    }
    let mut payload = vec![0_u8; size];
    stream.read_exact(&mut payload)?;
    Ok(rmp_serde::from_slice(&payload)?)
}

#[cfg(test)]
mod tests {
    use super::{
        ClientHello, ClientMessage, FROZEN_CLIENT_BUILD, FROZEN_CLIENT_NAME,
        FROZEN_PROTOCOL_VERSION,
    };

    #[test]
    fn preserved_client_identity_is_explicit() {
        let hello = ClientMessage::ClientHello(ClientHello {
            protocol_version: FROZEN_PROTOCOL_VERSION,
            client_name: FROZEN_CLIENT_NAME.to_owned(),
            client_build: FROZEN_CLIENT_BUILD.to_owned(),
        });
        let ClientMessage::ClientHello(hello) = hello else {
            panic!("expected hello");
        };
        assert_eq!(hello.protocol_version, 1);
        assert_eq!(hello.client_name, "revenant-client-v1");
        assert_eq!(hello.client_build, "0.1.0-frozen");
    }
}
