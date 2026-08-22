use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read, Write};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 2;
pub const MAX_FRAME_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    ClientHello(ClientHello),
    AuthRequest(AuthRequest),
    CharacterListRequest(CharacterListRequest),
    WorldJoinRequest(WorldJoinRequest),
    AttackIntent(AttackIntent),
    MoveIntent(MoveIntent),
    EquipIntent(EquipIntent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    ServerHello(ServerHello),
    AuthResponse(AuthResponse),
    CharacterListResponse(CharacterListResponse),
    WorldJoinResponse(WorldJoinResponse),
    InventorySnapshot(InventorySnapshot),
    ProgressionSnapshot(ProgressionSnapshot),
    EquipmentSnapshot(EquipmentSnapshot),
    ActorSpawn(ActorSpawn),
    ActorUpdate(ActorUpdate),
    ActorDestroy(ActorDestroy),
    DamageApplied(DamageApplied),
    ActivityStart(ActivityStart),
    ObjectiveUpdate(ObjectiveUpdate),
    DoorState(DoorState),
    ActivityComplete(ActivityComplete),
    LootGranted(LootGranted),
    ProgressionGranted(ProgressionGranted),
    EquipmentChanged(EquipmentChanged),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub protocol_version: u16,
    pub client_name: String,
    pub client_build: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerHello {
    pub protocol_version: u16,
    pub server_name: String,
    pub accepted: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub authenticated: bool,
    pub account_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterListRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterListResponse {
    pub characters: Vec<CharacterSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub character_id: String,
    pub display_name: String,
    pub class_name: String,
    pub level: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldJoinRequest {
    pub character_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldJoinResponse {
    pub accepted: bool,
    pub world_id: String,
    pub player_actor_id: u64,
    pub spawn_position: [i32; 3],
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySnapshot {
    pub items: Vec<InventoryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionSnapshot {
    pub level: u32,
    pub experience: u64,
    pub experience_to_next_level: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentSnapshot {
    pub equipped_weapon_item_id: String,
    pub weapons: Vec<WeaponProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponProfile {
    pub item_id: String,
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSpawn {
    pub actor_id: u64,
    pub actor_kind: String,
    pub archetype: String,
    pub position: [i32; 3],
    pub health: u32,
    pub max_health: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorUpdate {
    pub actor_id: u64,
    pub position: [i32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorDestroy {
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackIntent {
    pub target_actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipIntent {
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageApplied {
    pub source_actor_id: u64,
    pub target_actor_id: u64,
    pub damage: u32,
    pub remaining_health: u32,
    pub killed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityStart {
    pub activity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveUpdate {
    pub objective_id: String,
    pub objective_type: String,
    pub state: String,
    pub progress: u32,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveIntent {
    pub position: [i32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorState {
    pub door_id: String,
    pub open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityComplete {
    pub activity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootGranted {
    pub activity_id: String,
    pub item_id: String,
    pub quantity: u32,
    pub resulting_quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionGranted {
    pub activity_id: String,
    pub experience_granted: u64,
    pub experience: u64,
    pub previous_level: u32,
    pub level: u32,
    pub experience_to_next_level: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentChanged {
    pub accepted: bool,
    pub message: String,
    pub actor_id: u64,
    pub equipped_weapon_item_id: String,
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    Encode(rmp_serde::encode::Error),
    Decode(rmp_serde::decode::Error),
    FrameTooLarge(usize),
}

impl Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "protocol I/O failed: {error}"),
            Self::Encode(error) => write!(formatter, "MessagePack encoding failed: {error}"),
            Self::Decode(error) => write!(formatter, "MessagePack decoding failed: {error}"),
            Self::FrameTooLarge(size) => write!(
                formatter,
                "protocol frame is {size} bytes; maximum is {MAX_FRAME_SIZE}"
            ),
        }
    }
}

impl Error for ProtocolError {}

impl From<io::Error> for ProtocolError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rmp_serde::encode::Error> for ProtocolError {
    fn from(error: rmp_serde::encode::Error) -> Self {
        Self::Encode(error)
    }
}

impl From<rmp_serde::decode::Error> for ProtocolError {
    fn from(error: rmp_serde::decode::Error) -> Self {
        Self::Decode(error)
    }
}

/// Writes one length-prefixed `MessagePack` message.
///
/// # Errors
///
/// Returns an error if serialization fails, the payload exceeds the frame limit,
/// or the writer cannot accept the complete frame.
pub fn write_message<W, T>(writer: &mut W, message: &T) -> Result<(), ProtocolError>
where
    W: Write,
    T: Serialize,
{
    let payload = rmp_serde::to_vec_named(message)?;
    if payload.len() > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(payload.len()));
    }

    let size =
        u32::try_from(payload.len()).map_err(|_| ProtocolError::FrameTooLarge(payload.len()))?;
    writer.write_all(&size.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

/// Reads one length-prefixed `MessagePack` message.
///
/// # Errors
///
/// Returns an error if the frame is incomplete, exceeds the frame limit, or does
/// not contain a valid message of the requested type.
pub fn read_message<R, T>(reader: &mut R) -> Result<T, ProtocolError>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut size_bytes = [0_u8; 4];
    reader.read_exact(&mut size_bytes)?;
    let size = usize::try_from(u32::from_be_bytes(size_bytes))
        .map_err(|_| ProtocolError::FrameTooLarge(usize::MAX))?;
    if size > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(size));
    }

    let mut payload = vec![0_u8; size];
    reader.read_exact(&mut payload)?;
    Ok(rmp_serde::from_slice(&payload)?)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        read_message, write_message, CharacterListRequest, ClientHello, ClientMessage,
        EquipmentSnapshot, InventoryItem, InventorySnapshot, ProgressionSnapshot, ServerMessage,
        WeaponProfile, PROTOCOL_VERSION,
    };

    #[test]
    fn client_hello_round_trips_through_framing() {
        let expected = ClientMessage::ClientHello(ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "protocol-test".to_owned(),
            client_build: "0.1.0".to_owned(),
        });
        let mut frame = Vec::new();

        write_message(&mut frame, &expected).expect("client hello should encode");
        let actual: ClientMessage =
            read_message(&mut Cursor::new(frame)).expect("client hello should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn oversized_frame_is_rejected_before_allocation() {
        let announced_size = u32::try_from(super::MAX_FRAME_SIZE + 1)
            .expect("frame limit should fit in a u32")
            .to_be_bytes();

        let result = read_message::<_, ClientMessage>(&mut Cursor::new(announced_size));

        assert!(matches!(
            result,
            Err(super::ProtocolError::FrameTooLarge(_))
        ));
    }

    #[test]
    fn empty_character_list_request_round_trips() {
        let expected = ClientMessage::CharacterListRequest(CharacterListRequest {});
        let mut frame = Vec::new();

        write_message(&mut frame, &expected).expect("request should encode");
        let actual: ClientMessage =
            read_message(&mut Cursor::new(frame)).expect("request should decode");

        assert_eq!(actual, expected);
    }

    #[test]
    fn inventory_snapshot_round_trips() {
        let expected = ServerMessage::InventorySnapshot(InventorySnapshot {
            items: vec![InventoryItem {
                item_id: "pulse_rifle".to_owned(),
                quantity: 1,
            }],
        });
        let mut frame = Vec::new();
        write_message(&mut frame, &expected).expect("inventory should encode");
        let actual: ServerMessage =
            read_message(&mut Cursor::new(frame)).expect("inventory should decode");
        assert_eq!(actual, expected);
    }

    #[test]
    fn progression_snapshot_round_trips() {
        let expected = ServerMessage::ProgressionSnapshot(ProgressionSnapshot {
            level: 2,
            experience: 500,
            experience_to_next_level: 500,
        });
        let mut frame = Vec::new();
        write_message(&mut frame, &expected).expect("progression should encode");
        let actual: ServerMessage =
            read_message(&mut Cursor::new(frame)).expect("progression should decode");
        assert_eq!(actual, expected);
    }

    #[test]
    fn equipment_snapshot_round_trips() {
        let expected = ServerMessage::EquipmentSnapshot(EquipmentSnapshot {
            equipped_weapon_item_id: "arc_sidearm".to_owned(),
            weapons: vec![WeaponProfile {
                item_id: "arc_sidearm".to_owned(),
                damage: 25,
                range: 8,
                cooldown_ms: 150,
            }],
        });
        let mut frame = Vec::new();
        write_message(&mut frame, &expected).expect("equipment should encode");
        let actual: ServerMessage =
            read_message(&mut Cursor::new(frame)).expect("equipment should decode");
        assert_eq!(actual, expected);
    }
}
