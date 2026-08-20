use std::error::Error;
use std::fmt::{self, Display, Formatter};

use revenant_protocol::{
    AttackIntent, AuthRequest, CharacterListRequest, ClientMessage, MoveIntent, WorldJoinRequest,
    PROTOCOL_VERSION,
};

pub const FROZEN_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolGeneration {
    FrozenV1,
    CurrentV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolAdapter {
    generation: ProtocolGeneration,
}

impl ProtocolAdapter {
    /// Negotiates a supported wire version before any domain message is handled.
    ///
    /// # Errors
    ///
    /// Returns an error when the client version has no authorized adapter.
    pub fn negotiate(client_version: u16) -> Result<Self, CompatibilityError> {
        let generation = match client_version {
            FROZEN_V1 => ProtocolGeneration::FrozenV1,
            PROTOCOL_VERSION => ProtocolGeneration::CurrentV2,
            version => return Err(CompatibilityError::UnsupportedVersion(version)),
        };
        Ok(Self { generation })
    }

    #[must_use]
    pub fn wire_version(self) -> u16 {
        match self.generation {
            ProtocolGeneration::FrozenV1 => FROZEN_V1,
            ProtocolGeneration::CurrentV2 => PROTOCOL_VERSION,
        }
    }

    #[must_use]
    pub fn generation(self) -> ProtocolGeneration {
        self.generation
    }

    /// Converts a versioned wire message into the gateway's canonical input vocabulary.
    /// V1 and V2 intentionally share field layouts in M14, but the mapping boundary is
    /// explicit so later evolution does not leak version checks into domain systems.
    ///
    /// # Errors
    ///
    /// Returns an error if another handshake appears after negotiation.
    pub fn canonicalize(
        self,
        message: ClientMessage,
    ) -> Result<CanonicalClientMessage, CompatibilityError> {
        match message {
            ClientMessage::AuthRequest(message) => Ok(CanonicalClientMessage::AuthRequest(message)),
            ClientMessage::CharacterListRequest(message) => {
                Ok(CanonicalClientMessage::CharacterListRequest(message))
            }
            ClientMessage::WorldJoinRequest(message) => {
                Ok(CanonicalClientMessage::WorldJoinRequest(message))
            }
            ClientMessage::AttackIntent(message) => {
                Ok(CanonicalClientMessage::AttackIntent(message))
            }
            ClientMessage::MoveIntent(message) => Ok(CanonicalClientMessage::MoveIntent(message)),
            ClientMessage::ClientHello(_) => Err(CompatibilityError::RepeatedHandshake),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalClientMessage {
    AuthRequest(AuthRequest),
    CharacterListRequest(CharacterListRequest),
    WorldJoinRequest(WorldJoinRequest),
    AttackIntent(AttackIntent),
    MoveIntent(MoveIntent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityError {
    UnsupportedVersion(u16),
    RepeatedHandshake,
}

impl Display for CompatibilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "protocol version {version} has no compatibility adapter"
                )
            }
            Self::RepeatedHandshake => formatter.write_str("handshake cannot be repeated"),
        }
    }
}

impl Error for CompatibilityError {}

#[cfg(test)]
mod tests {
    use revenant_protocol::{AuthRequest, ClientMessage, PROTOCOL_VERSION};

    use super::{CanonicalClientMessage, CompatibilityError, ProtocolAdapter, FROZEN_V1};

    #[test]
    fn negotiates_frozen_and_current_protocols() {
        assert_eq!(
            ProtocolAdapter::negotiate(FROZEN_V1)
                .expect("V1 should remain supported")
                .wire_version(),
            FROZEN_V1
        );
        assert_eq!(
            ProtocolAdapter::negotiate(PROTOCOL_VERSION)
                .expect("current version should be supported")
                .wire_version(),
            PROTOCOL_VERSION
        );
        assert_eq!(
            ProtocolAdapter::negotiate(99),
            Err(CompatibilityError::UnsupportedVersion(99))
        );
    }

    #[test]
    fn maps_v1_wire_message_to_canonical_input() {
        let adapter = ProtocolAdapter::negotiate(FROZEN_V1).expect("V1 should negotiate");
        let canonical = adapter
            .canonicalize(ClientMessage::AuthRequest(AuthRequest {
                username: "frozen-client".to_owned(),
            }))
            .expect("auth should canonicalize");
        assert!(matches!(
            canonical,
            CanonicalClientMessage::AuthRequest(AuthRequest { username })
                if username == "frozen-client"
        ));
    }
}
