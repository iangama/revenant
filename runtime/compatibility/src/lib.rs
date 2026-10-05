use std::error::Error;
use std::fmt::{self, Display, Formatter};

use revenant_protocol::{
    AttackIntent, AuthRequest, CharacterListRequest, ClientMessage, CooperationPingIntent,
    CooperationReviveIntent, CooperationStartIntent, CooperationStateRequest, EquipIntent,
    ModuleCombineIntent, ModuleLoadoutIntent, ModulePreviewRequest, ModuleStateRequest, MoveIntent,
    RouteChoiceIntent, RouteStateRequest, WorldJoinRequest, PROTOCOL_VERSION,
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
            ClientMessage::CampaignStateRequest(message) => {
                self.current_v2(CanonicalClientMessage::CampaignStateRequest(message))
            }
            ClientMessage::ChallengeStateRequest(message) => {
                self.current_v2(CanonicalClientMessage::ChallengeStateRequest(message))
            }
            ClientMessage::ChallengeJoinRequest(message) => {
                self.current_v2(CanonicalClientMessage::ChallengeJoinRequest(message))
            }
            ClientMessage::ChallengeAbandonIntent(message) => {
                self.current_v2(CanonicalClientMessage::ChallengeAbandonIntent(message))
            }
            ClientMessage::CampaignJoinRequest(message) => {
                self.current_v2(CanonicalClientMessage::CampaignJoinRequest(message))
            }
            ClientMessage::CampaignStoryIntent(message) => {
                self.current_v2(CanonicalClientMessage::CampaignStoryIntent(message))
            }
            ClientMessage::AttackIntent(message) => {
                Ok(CanonicalClientMessage::AttackIntent(message))
            }
            ClientMessage::MoveIntent(message) => Ok(CanonicalClientMessage::MoveIntent(message)),
            ClientMessage::EquipIntent(message) => {
                if self.generation == ProtocolGeneration::FrozenV1 {
                    return Err(CompatibilityError::UnsupportedV1Message);
                }
                Ok(CanonicalClientMessage::EquipIntent(message))
            }
            ClientMessage::ModuleStateRequest(message) => {
                self.current_v2(CanonicalClientMessage::ModuleStateRequest(message))
            }
            ClientMessage::AcquisitionStateRequest(message) => {
                self.current_v2(CanonicalClientMessage::AcquisitionStateRequest(message))
            }
            ClientMessage::AcquisitionClaimIntent(message) => {
                self.current_v2(CanonicalClientMessage::AcquisitionClaimIntent(message))
            }
            ClientMessage::ModulePreviewRequest(message) => {
                self.current_v2(CanonicalClientMessage::ModulePreviewRequest(message))
            }
            ClientMessage::ModuleCombineIntent(message) => {
                self.current_v2(CanonicalClientMessage::ModuleCombineIntent(message))
            }
            ClientMessage::ModuleLoadoutIntent(message) => {
                self.current_v2(CanonicalClientMessage::ModuleLoadoutIntent(message))
            }
            ClientMessage::RouteStateRequest(message) => {
                self.current_v2(CanonicalClientMessage::RouteStateRequest(message))
            }
            ClientMessage::RouteChoiceIntent(message) => {
                self.current_v2(CanonicalClientMessage::RouteChoiceIntent(message))
            }
            ClientMessage::CooperationStateRequest(message) => {
                self.current_v2(CanonicalClientMessage::CooperationStateRequest(message))
            }
            ClientMessage::CooperationStartIntent(message) => {
                self.current_v2(CanonicalClientMessage::CooperationStartIntent(message))
            }
            ClientMessage::CooperationPingIntent(message) => {
                self.current_v2(CanonicalClientMessage::CooperationPingIntent(message))
            }
            ClientMessage::CooperationReviveIntent(message) => {
                self.current_v2(CanonicalClientMessage::CooperationReviveIntent(message))
            }
            ClientMessage::ClientHello(_) => Err(CompatibilityError::RepeatedHandshake),
        }
    }

    fn current_v2(
        self,
        message: CanonicalClientMessage,
    ) -> Result<CanonicalClientMessage, CompatibilityError> {
        if self.generation == ProtocolGeneration::FrozenV1 {
            Err(CompatibilityError::UnsupportedV1Message)
        } else {
            Ok(message)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalClientMessage {
    AuthRequest(AuthRequest),
    CharacterListRequest(CharacterListRequest),
    WorldJoinRequest(WorldJoinRequest),
    CampaignStateRequest(revenant_protocol::CampaignStateRequest),
    ChallengeStateRequest(revenant_protocol::ChallengeStateRequest),
    ChallengeJoinRequest(revenant_protocol::ChallengeJoinRequest),
    ChallengeAbandonIntent(revenant_protocol::ChallengeAbandonIntent),
    CampaignJoinRequest(revenant_protocol::CampaignJoinRequest),
    CampaignStoryIntent(revenant_protocol::CampaignStoryIntent),
    AttackIntent(AttackIntent),
    MoveIntent(MoveIntent),
    EquipIntent(EquipIntent),
    ModuleStateRequest(ModuleStateRequest),
    AcquisitionStateRequest(revenant_protocol::AcquisitionStateRequest),
    AcquisitionClaimIntent(revenant_protocol::AcquisitionClaimIntent),
    ModulePreviewRequest(ModulePreviewRequest),
    ModuleCombineIntent(ModuleCombineIntent),
    ModuleLoadoutIntent(ModuleLoadoutIntent),
    RouteStateRequest(RouteStateRequest),
    RouteChoiceIntent(RouteChoiceIntent),
    CooperationStateRequest(CooperationStateRequest),
    CooperationStartIntent(CooperationStartIntent),
    CooperationPingIntent(CooperationPingIntent),
    CooperationReviveIntent(CooperationReviveIntent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityError {
    UnsupportedVersion(u16),
    RepeatedHandshake,
    UnsupportedV1Message,
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
            Self::UnsupportedV1Message => {
                formatter.write_str("message is not available in frozen protocol V1")
            }
        }
    }
}

impl Error for CompatibilityError {}

#[cfg(test)]
mod tests {
    use revenant_protocol::{
        AuthRequest, ClientMessage, CooperationPingIntent, CooperationReviveIntent,
        CooperationStartIntent, CooperationStateRequest, EquipIntent, ModuleStateRequest,
        RouteChoiceIntent, RouteStateRequest, PROTOCOL_VERSION,
    };

    use super::{CanonicalClientMessage, CompatibilityError, ProtocolAdapter, FROZEN_V1};

    #[test]
    fn challenge_board_request_is_v2_only() {
        let message =
            ClientMessage::ChallengeStateRequest(revenant_protocol::ChallengeStateRequest {
                character_id: "operator".to_owned(),
            });
        let mut frame = Vec::new();
        revenant_protocol::write_message(&mut frame, &message).unwrap();
        let decoded: ClientMessage =
            revenant_protocol::read_message(&mut frame.as_slice()).unwrap();
        assert_eq!(decoded, message);
        assert!(ProtocolAdapter::negotiate(PROTOCOL_VERSION)
            .unwrap()
            .canonicalize(decoded.clone())
            .is_ok());
        assert_eq!(
            ProtocolAdapter::negotiate(FROZEN_V1)
                .unwrap()
                .canonicalize(decoded),
            Err(CompatibilityError::UnsupportedV1Message)
        );
    }

    #[test]
    fn challenge_entry_and_abandon_are_v2_only() {
        for message in [
            ClientMessage::ChallengeJoinRequest(revenant_protocol::ChallengeJoinRequest {
                preset_id: None,
                character_id: "operator".into(),
                operation_id: "entry".into(),
                expected_revision: 0,
                contract_id: "close_quarters".into(),
                retry_run_id: None,
            }),
            ClientMessage::ChallengeAbandonIntent(revenant_protocol::ChallengeAbandonIntent {
                operation_id: "abandon".into(),
                expected_revision: 1,
                run_id: 1,
            }),
        ] {
            let mut frame = Vec::new();
            revenant_protocol::write_message(&mut frame, &message).unwrap();
            let decoded: ClientMessage =
                revenant_protocol::read_message(&mut frame.as_slice()).unwrap();
            assert_eq!(decoded, message);
            assert!(ProtocolAdapter::negotiate(PROTOCOL_VERSION)
                .unwrap()
                .canonicalize(decoded.clone())
                .is_ok());
            assert_eq!(
                ProtocolAdapter::negotiate(FROZEN_V1)
                    .unwrap()
                    .canonicalize(decoded),
                Err(CompatibilityError::UnsupportedV1Message)
            );
        }
    }

    #[test]
    fn campaign_entry_is_v2_only_and_preserves_explicit_mode_and_revision() {
        let requests = [
            ClientMessage::CampaignStoryIntent(revenant_protocol::CampaignStoryIntent {
                operation_id: "story-one".to_owned(),
                run_id: "run-one".to_owned(),
                expected_revision: 5,
                action: revenant_protocol::CampaignStoryAction::SupplyService,
            }),
            ClientMessage::CampaignStateRequest(revenant_protocol::CampaignStateRequest {
                character_id: "operator".to_owned(),
            }),
            ClientMessage::CampaignJoinRequest(revenant_protocol::CampaignJoinRequest {
                character_id: "operator".to_owned(),
                chapter_id: "meridian_readings".to_owned(),
                mode: revenant_protocol::CampaignEntryMode::Resume,
                expected_revision: 5,
            }),
        ];
        for message in requests {
            let mut frame = Vec::new();
            revenant_protocol::write_message(&mut frame, &message).unwrap();
            let decoded: ClientMessage =
                revenant_protocol::read_message(&mut frame.as_slice()).unwrap();
            assert_eq!(decoded, message);
            assert!(ProtocolAdapter::negotiate(PROTOCOL_VERSION)
                .unwrap()
                .canonicalize(decoded.clone())
                .is_ok());
            assert_eq!(
                ProtocolAdapter::negotiate(FROZEN_V1)
                    .unwrap()
                    .canonicalize(decoded),
                Err(CompatibilityError::UnsupportedV1Message)
            );
        }
    }

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

    #[test]
    fn frozen_v1_cannot_submit_equipment_intents() {
        let adapter = ProtocolAdapter::negotiate(FROZEN_V1).expect("V1 should negotiate");
        for message in [
            ClientMessage::AcquisitionStateRequest(revenant_protocol::AcquisitionStateRequest {}),
            ClientMessage::AcquisitionClaimIntent(revenant_protocol::AcquisitionClaimIntent {
                arc_id: "meridian".to_owned(),
            }),
        ] {
            assert_eq!(
                adapter.canonicalize(message),
                Err(CompatibilityError::UnsupportedV1Message)
            );
        }
        assert_eq!(
            adapter.canonicalize(ClientMessage::EquipIntent(EquipIntent {
                item_id: "arc_sidearm".to_owned(),
            })),
            Err(CompatibilityError::UnsupportedV1Message)
        );
        assert_eq!(
            adapter.canonicalize(ClientMessage::ModuleStateRequest(ModuleStateRequest {})),
            Err(CompatibilityError::UnsupportedV1Message)
        );
        assert_eq!(
            adapter.canonicalize(ClientMessage::RouteStateRequest(RouteStateRequest {})),
            Err(CompatibilityError::UnsupportedV1Message)
        );
        assert_eq!(
            adapter.canonicalize(ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                operation_id: "route-1".to_owned(),
                route_id: "breach".to_owned(),
            })),
            Err(CompatibilityError::UnsupportedV1Message)
        );
        for message in [
            ClientMessage::CooperationStateRequest(CooperationStateRequest {}),
            ClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id: "start-1".to_owned(),
            }),
            ClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id: "ping-1".to_owned(),
            }),
            ClientMessage::CooperationReviveIntent(CooperationReviveIntent {
                operation_id: "revive-1".to_owned(),
            }),
        ] {
            assert_eq!(
                adapter.canonicalize(message),
                Err(CompatibilityError::UnsupportedV1Message)
            );
        }
    }

    #[test]
    fn maps_current_route_requests_to_canonical_input() {
        let adapter =
            ProtocolAdapter::negotiate(PROTOCOL_VERSION).expect("current V2 should negotiate");
        assert!(matches!(
            adapter
                .canonicalize(ClientMessage::RouteStateRequest(RouteStateRequest {}))
                .expect("route state request should canonicalize"),
            CanonicalClientMessage::RouteStateRequest(RouteStateRequest {})
        ));
        assert!(matches!(
            adapter
                .canonicalize(ClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                    operation_id: "route-1".to_owned(),
                    route_id: "breach".to_owned(),
                }))
                .expect("route choice should canonicalize"),
            CanonicalClientMessage::RouteChoiceIntent(RouteChoiceIntent {
                operation_id,
                route_id,
            }) if operation_id == "route-1" && route_id == "breach"
        ));
    }

    #[test]
    fn maps_current_cooperation_requests_to_canonical_input() {
        let adapter =
            ProtocolAdapter::negotiate(PROTOCOL_VERSION).expect("current V2 should negotiate");
        assert!(matches!(
            adapter
                .canonicalize(ClientMessage::CooperationStateRequest(
                    CooperationStateRequest {}
                ))
                .expect("cooperation state request should canonicalize"),
            CanonicalClientMessage::CooperationStateRequest(CooperationStateRequest {})
        ));
        assert!(matches!(
            adapter
                .canonicalize(ClientMessage::CooperationStartIntent(
                    CooperationStartIntent {
                        operation_id: "start-1".to_owned(),
                    }
                ))
                .expect("cooperation start should canonicalize"),
            CanonicalClientMessage::CooperationStartIntent(CooperationStartIntent {
                operation_id
            }) if operation_id == "start-1"
        ));
        assert!(matches!(
            adapter
                .canonicalize(ClientMessage::CooperationPingIntent(
                    CooperationPingIntent {
                        operation_id: "ping-1".to_owned(),
                    }
                ))
                .expect("cooperation ping should canonicalize"),
            CanonicalClientMessage::CooperationPingIntent(CooperationPingIntent {
                operation_id
            }) if operation_id == "ping-1"
        ));
        assert!(matches!(
            adapter
                .canonicalize(ClientMessage::CooperationReviveIntent(
                    CooperationReviveIntent {
                        operation_id: "revive-1".to_owned(),
                    }
                ))
                .expect("cooperation revive should canonicalize"),
            CanonicalClientMessage::CooperationReviveIntent(CooperationReviveIntent {
                operation_id
            }) if operation_id == "revive-1"
        ));
    }
}
