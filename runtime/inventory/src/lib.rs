use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const RELAY_CORE_FRAGMENT: &str = "relay_core_fragment";
pub const PULSE_RIFLE: &str = "pulse_rifle";

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
        if !matches!(item_id.as_str(), PULSE_RIFLE | RELAY_CORE_FRAGMENT) {
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
    use super::{InventoryError, Reward, RELAY_CORE_FRAGMENT};

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
}
