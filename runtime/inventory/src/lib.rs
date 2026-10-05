use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const RELAY_CORE_FRAGMENT: &str = "relay_core_fragment";
pub const PULSE_RIFLE: &str = "pulse_rifle";
pub const ARC_SIDEARM: &str = "arc_sidearm";
pub const COIL_LANCE: &str = "coil_lance";
pub const SCATTER_CASTER: &str = "scatter_caster";
pub const RAIL_DRIVER: &str = "rail_driver";
pub const ARSENAL_WEAPONS: [&str; 2] = [SCATTER_CASTER, RAIL_DRIVER];

pub const SCATTER_CASTER_PROFILE: WeaponProfile = WeaponProfile {
    item_id: SCATTER_CASTER,
    damage: 56,
    range: 6,
    cooldown_ms: 400,
};
pub const RAIL_DRIVER_PROFILE: WeaponProfile = WeaponProfile {
    item_id: RAIL_DRIVER,
    damage: 56,
    range: 10,
    cooldown_ms: 400,
};

/// Resolves fixed weapon geometry from authoritative positions, before defenses.
/// The caller validates maximum range, life, faction and shared cooldown.
#[must_use]
pub fn positioned_damage(item_id: &str, damage: u32, distance_squared: i64) -> Option<u32> {
    if distance_squared < 0 || (item_id == RAIL_DRIVER && distance_squared < 16) {
        return None;
    }
    Some(if item_id == SCATTER_CASTER && distance_squared > 9 {
        damage / 2
    } else {
        damage
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponProfile {
    pub item_id: &'static str,
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
}

pub const PULSE_RIFLE_PROFILE: WeaponProfile = WeaponProfile {
    item_id: PULSE_RIFLE,
    damage: 40,
    range: 6,
    cooldown_ms: 250,
};

pub const ARC_SIDEARM_PROFILE: WeaponProfile = WeaponProfile {
    item_id: ARC_SIDEARM,
    damage: 25,
    range: 8,
    cooldown_ms: 150,
};

pub const COIL_LANCE_PROFILE: WeaponProfile = WeaponProfile {
    item_id: COIL_LANCE,
    damage: 48,
    range: 9,
    cooldown_ms: 350,
};

#[must_use]
pub fn weapon_profile(item_id: &str) -> Option<WeaponProfile> {
    match item_id {
        PULSE_RIFLE => Some(PULSE_RIFLE_PROFILE),
        ARC_SIDEARM => Some(ARC_SIDEARM_PROFILE),
        COIL_LANCE => Some(COIL_LANCE_PROFILE),
        SCATTER_CASTER => Some(SCATTER_CASTER_PROFILE),
        RAIL_DRIVER => Some(RAIL_DRIVER_PROFILE),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reward {
    pub item_id: String,
    pub quantity: u32,
}

impl Reward {
    /// Validates an activity-authored reward against the server-owned item catalog.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown items or a zero quantity.
    pub fn validated(item_id: impl Into<String>, quantity: u32) -> Result<Self, InventoryError> {
        let item_id = item_id.into();
        if !matches!(
            item_id.as_str(),
            PULSE_RIFLE
                | ARC_SIDEARM
                | COIL_LANCE
                | SCATTER_CASTER
                | RAIL_DRIVER
                | RELAY_CORE_FRAGMENT
        ) {
            return Err(InventoryError::UnknownItem(item_id));
        }
        if quantity == 0 {
            return Err(InventoryError::ZeroQuantity);
        }
        Ok(Self { item_id, quantity })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryError {
    UnknownItem(String),
    ZeroQuantity,
}

impl Display for InventoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownItem(item_id) => write!(formatter, "unknown inventory item: {item_id}"),
            Self::ZeroQuantity => formatter.write_str("inventory quantity must be positive"),
        }
    }
}

impl Error for InventoryError {}

#[cfg(test)]
mod tests {
    use super::{weapon_profile, InventoryError, Reward, ARC_SIDEARM, RELAY_CORE_FRAGMENT};

    #[test]
    fn arsenal_geometry_has_exact_boundaries_and_scales_effective_damage() {
        use super::{positioned_damage, RAIL_DRIVER, SCATTER_CASTER};
        assert_eq!(positioned_damage(SCATTER_CASTER, 67, 9), Some(67));
        assert_eq!(positioned_damage(SCATTER_CASTER, 67, 10), Some(33));
        assert_eq!(positioned_damage(RAIL_DRIVER, 56, 15), None);
        assert_eq!(positioned_damage(RAIL_DRIVER, 56, 16), Some(56));
        assert_eq!(positioned_damage(RAIL_DRIVER, 56, -1), None);
        assert_eq!(positioned_damage("pulse_rifle", 40, 1), Some(40));
    }

    #[test]
    fn validates_catalog_reward() {
        assert_eq!(
            Reward::validated(RELAY_CORE_FRAGMENT, 1).expect("catalog reward should validate"),
            Reward {
                item_id: RELAY_CORE_FRAGMENT.to_owned(),
                quantity: 1,
            }
        );
        assert_eq!(
            Reward::validated("unauthorized_item", 1),
            Err(InventoryError::UnknownItem("unauthorized_item".to_owned()))
        );
        assert_eq!(
            Reward::validated(RELAY_CORE_FRAGMENT, 0),
            Err(InventoryError::ZeroQuantity)
        );
    }

    #[test]
    fn only_weapons_have_combat_profiles() {
        let sidearm = weapon_profile(ARC_SIDEARM).expect("sidearm should be equipable");
        assert_eq!(sidearm.damage, 25);
        assert!(weapon_profile(RELAY_CORE_FRAGMENT).is_none());
    }
}
