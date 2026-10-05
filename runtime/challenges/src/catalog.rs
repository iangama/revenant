use serde::{Deserialize, Serialize};

/// Initial two-contract policy. New contracts or gameplay rules require a new
/// explicit revision; presentation settings never belong in record identity.
pub const REVISION: &str = "m37-challenges-v1";
pub const EXPANDED_REVISION: &str = "m37-challenges-v2";
/// Elite contract policy, negotiated separately from the first three contracts.
pub const ELITE_REVISION: &str = "m37-challenges-v3";
pub const PRISM_REVISION: &str = "m37-challenges-v4";
pub const SIGNAL_REVISION: &str = "m37-challenges-v5";
pub const SURVIVAL_REVISION: &str = "m37-challenges-v6";
pub const GAUNTLET_REVISION: &str = "m37-challenges-v7";
/// Domain identity reserved for M37-C; no gateway capability publishes it yet.
pub const MODIFIER_REVISION: &str = "m37-challenges-v8";
pub const GAMEPLAY_REVISION: &str = "m37-baseline-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractId {
    CloseQuarters,
    MeridianCircuit,
    CoolantRecovery,
    BastionLink,
    PrismDiscipline,
    DistantSignal,
    LastReserve,
    RelayGauntlet,
}

impl ContractId {
    pub const INITIAL: [Self; 2] = [Self::CloseQuarters, Self::MeridianCircuit];
    pub const RECOVERY: [Self; 3] = [
        Self::CloseQuarters,
        Self::MeridianCircuit,
        Self::CoolantRecovery,
    ];
    pub const ELITE: [Self; 4] = [
        Self::CloseQuarters,
        Self::MeridianCircuit,
        Self::CoolantRecovery,
        Self::BastionLink,
    ];

    pub const PRISM: [Self; 5] = [
        Self::CloseQuarters,
        Self::MeridianCircuit,
        Self::CoolantRecovery,
        Self::BastionLink,
        Self::PrismDiscipline,
    ];

    pub const SIGNAL: [Self; 6] = [
        Self::CloseQuarters,
        Self::MeridianCircuit,
        Self::CoolantRecovery,
        Self::BastionLink,
        Self::PrismDiscipline,
        Self::DistantSignal,
    ];

    pub const SURVIVAL: [Self; 7] = [
        Self::CloseQuarters,
        Self::MeridianCircuit,
        Self::CoolantRecovery,
        Self::BastionLink,
        Self::PrismDiscipline,
        Self::DistantSignal,
        Self::LastReserve,
    ];

    pub const ALL: [Self; 8] = [
        Self::CloseQuarters,
        Self::MeridianCircuit,
        Self::CoolantRecovery,
        Self::BastionLink,
        Self::PrismDiscipline,
        Self::DistantSignal,
        Self::LastReserve,
        Self::RelayGauntlet,
    ];

    #[must_use]
    pub fn available_in(self, policy: &str) -> bool {
        match policy {
            REVISION => Self::INITIAL.contains(&self),
            EXPANDED_REVISION => Self::RECOVERY.contains(&self),
            ELITE_REVISION => Self::ELITE.contains(&self),
            PRISM_REVISION => Self::PRISM.contains(&self),
            SIGNAL_REVISION => Self::SIGNAL.contains(&self),
            SURVIVAL_REVISION => Self::SURVIVAL.contains(&self),
            GAUNTLET_REVISION | MODIFIER_REVISION => true,
            _ => false,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CloseQuarters => "close_quarters",
            Self::MeridianCircuit => "meridian_circuit",
            Self::CoolantRecovery => "coolant_recovery",
            Self::BastionLink => "bastion_link",
            Self::PrismDiscipline => "prism_discipline",
            Self::DistantSignal => "distant_signal",
            Self::LastReserve => "last_reserve",
            Self::RelayGauntlet => "relay_gauntlet",
        }
    }

    #[must_use]
    pub const fn activity_id(self) -> &'static str {
        match self {
            Self::CloseQuarters => "challenge_close_quarters",
            Self::MeridianCircuit => "challenge_meridian_circuit",
            Self::CoolantRecovery => "challenge_coolant_recovery",
            Self::BastionLink => "challenge_bastion_link",
            Self::PrismDiscipline => "challenge_prism_discipline",
            Self::DistantSignal => "challenge_distant_signal",
            Self::LastReserve => "challenge_last_reserve",
            Self::RelayGauntlet => "challenge_relay_gauntlet",
        }
    }

    #[must_use]
    pub fn rules(self) -> Rules {
        let (revision, seed) = match self {
            Self::CloseQuarters => ("close-quarters-v1", 37_001),
            Self::MeridianCircuit => ("meridian-circuit-v1", 37_002),
            Self::CoolantRecovery => (crate::recovery::REVISION, 37_003),
            Self::BastionLink => ("bastion-link-v1", 37_004),
            Self::PrismDiscipline => ("prism-discipline-v1", 37_005),
            Self::DistantSignal => (crate::signal::REVISION, 37_006),
            Self::LastReserve => (crate::survival::REVISION, 37_007),
            Self::RelayGauntlet => (crate::gauntlet::REVISION, 37_008),
        };
        Rules {
            contract: self,
            contract_revision: revision.to_owned(),
            gameplay_revision: match self {
                Self::CoolantRecovery => "m37-recovery-v1",
                Self::BastionLink => "m37-elite-v1",
                Self::PrismDiscipline => "m37-prism-v1",
                Self::DistantSignal => "m37-signal-v1",
                Self::LastReserve => "m37-survival-v1",
                Self::RelayGauntlet => "m37-gauntlet-v1",
                Self::CloseQuarters | Self::MeridianCircuit => GAMEPLAY_REVISION,
            }
            .to_owned(),
            seed,
            preset: crate::modifiers::Preset::Baseline,
        }
    }

    /// # Errors
    /// Rejects presets outside this contract's authored combinations.
    pub fn rules_with_preset(
        self,
        preset: crate::modifiers::Preset,
    ) -> Result<Rules, crate::ChallengeError> {
        if !preset.available_for(self) {
            return Err(crate::ChallengeError::InvalidModifiers);
        }
        let mut rules = self.rules();
        if !preset.is_baseline() {
            "m37-modifiers-v1".clone_into(&mut rules.gameplay_revision);
            rules.preset = preset;
        }
        Ok(rules)
    }

    pub(crate) const fn complete_mask(self) -> u8 {
        match self {
            Self::CloseQuarters | Self::BastionLink => 0b11,
            Self::MeridianCircuit => 0b1111,
            Self::CoolantRecovery | Self::LastReserve | Self::RelayGauntlet => 0b111,
            Self::PrismDiscipline | Self::DistantSignal => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub contract: ContractId,
    pub contract_revision: String,
    pub gameplay_revision: String,
    pub seed: u64,
    /// Omitted for all frozen baseline records and commands.
    #[serde(default, skip_serializing_if = "crate::modifiers::Preset::is_baseline")]
    pub preset: crate::modifiers::Preset,
}

impl Rules {
    #[must_use]
    pub fn visit_objective(&self, objectives: u8, position: [i32; 3]) -> Option<Objective> {
        if self.contract != ContractId::MeridianCircuit {
            return None;
        }
        [
            Objective::LensRead,
            Objective::GalleryRead,
            Objective::LogRead,
            Objective::Returned,
        ]
        .into_iter()
        .find(|o| {
            self.objective_position(*o) == Some(position)
                && self
                    .objective_bit(*o)
                    .is_some_and(|bit| objectives & bit == 0)
                && (*o != Objective::Returned || objectives == 7)
        })
    }

    /// Station identity belongs to the admitted rules; an alternate return must
    /// not silently keep the baseline's east-side destination.
    #[must_use]
    pub fn objective_position(&self, objective: Objective) -> Option<[i32; 3]> {
        if self.contract == ContractId::MeridianCircuit && objective == Objective::Returned {
            self.preset.meridian_entrance()
        } else {
            objective.position()
        }
    }

    #[must_use]
    pub fn available_in(&self, policy: &str) -> bool {
        self.contract.available_in(policy)
            && (self.preset.is_baseline() || policy == MODIFIER_REVISION)
    }

    #[must_use]
    pub fn objective_bit(&self, objective: Objective) -> Option<u8> {
        if self.contract == ContractId::RelayGauntlet
            && self.preset.has(crate::modifiers::Modifier::BastionFirst)
        {
            match objective {
                Objective::GauntletBastionCleared => return Some(1),
                Objective::GauntletSupportCleared => return Some(2),
                _ => {}
            }
        }
        objective.bit(self.contract)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Objective {
    LancerCleared,
    MenderCleared,
    LensRead,
    GalleryRead,
    LogRead,
    Returned,
    CoolantRetrieved,
    CoolantTransferred,
    CoolantDelivered,
    BulwarkCleared,
    PrismCleared,
    SentinelCleared,
    WestRelayCharged,
    EastRelayCharged,
    ReserveReturned,
    GauntletSupportCleared,
    GauntletBastionCleared,
    GauntletPrismCleared,
}

impl Objective {
    /// These positions are checked against the authoritative actor by the
    /// gateway. Replay must bind that position to this exact attempt/objective.
    #[must_use]
    pub const fn position(self) -> Option<[i32; 3]> {
        match self {
            Self::LensRead => Some([-28, 0, -7]),
            Self::GalleryRead => Some([-28, 0, 7]),
            Self::LogRead => Some([-30, 0, 0]),
            Self::Returned => Some([-16, 0, 0]),
            Self::CoolantRetrieved => Some(crate::recovery::INTAKE),
            Self::CoolantTransferred => Some(crate::recovery::TRANSFER),
            Self::CoolantDelivered => Some(crate::recovery::DELIVERY),
            Self::LancerCleared
            | Self::MenderCleared
            | Self::BulwarkCleared
            | Self::PrismCleared
            | Self::SentinelCleared
            | Self::WestRelayCharged
            | Self::EastRelayCharged
            | Self::ReserveReturned
            | Self::GauntletSupportCleared
            | Self::GauntletBastionCleared
            | Self::GauntletPrismCleared => None,
        }
    }

    pub(crate) const fn bit(self, contract: ContractId) -> Option<u8> {
        match (contract, self) {
            (ContractId::RelayGauntlet, Self::GauntletSupportCleared)
            | (ContractId::LastReserve, Self::WestRelayCharged)
            | (ContractId::DistantSignal, Self::SentinelCleared)
            | (ContractId::PrismDiscipline, Self::PrismCleared)
            | (ContractId::CloseQuarters, Self::LancerCleared)
            | (ContractId::BastionLink, Self::BulwarkCleared)
            | (ContractId::MeridianCircuit, Self::LensRead)
            | (ContractId::CoolantRecovery, Self::CoolantRetrieved) => Some(1),
            (ContractId::RelayGauntlet, Self::GauntletBastionCleared)
            | (ContractId::LastReserve, Self::EastRelayCharged)
            | (ContractId::CloseQuarters | ContractId::BastionLink, Self::MenderCleared)
            | (ContractId::MeridianCircuit, Self::GalleryRead)
            | (ContractId::CoolantRecovery, Self::CoolantTransferred) => Some(2),
            (ContractId::RelayGauntlet, Self::GauntletPrismCleared)
            | (ContractId::LastReserve, Self::ReserveReturned)
            | (ContractId::MeridianCircuit, Self::LogRead)
            | (ContractId::CoolantRecovery, Self::CoolantDelivered) => Some(4),
            (ContractId::MeridianCircuit, Self::Returned) => Some(8),
            _ => None,
        }
    }
}
