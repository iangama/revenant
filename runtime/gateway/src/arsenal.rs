use std::{error::Error, io};

use revenant_combat::AttackProfile;
use revenant_inventory::positioned_damage;

use super::SharedSession;

impl SharedSession {
    /// Weapon geometry is resolved before enemy shielding and phase damage floors.
    pub(super) fn positioned_attack_profile(
        &self,
        player_id: u64,
        target_id: u64,
    ) -> Result<AttackProfile, Box<dyn Error>> {
        let weapon = self
            .participants
            .iter()
            .find(|p| p.actor.id == player_id)
            .and_then(super::Participant::admitted_weapon)
            .ok_or_else(|| io::Error::other("no admitted weapon"))?;
        let player = self
            .actors
            .get(player_id)
            .ok_or_else(|| io::Error::other("attacker missing"))?;
        let target = self
            .actors
            .get(target_id)
            .ok_or_else(|| io::Error::other("target missing"))?;
        let distance_squared = player
            .position
            .iter()
            .zip(target.position)
            .map(|(left, right)| (i64::from(*left) - i64::from(right)).pow(2))
            .sum();
        let damage = positioned_damage(&weapon.item_id, weapon.effective_damage, distance_squared)
            .ok_or_else(|| {
                io::Error::other("Rail Driver needs at least four units of separation")
            })?;
        Ok(AttackProfile {
            damage,
            range: weapon.effective_range,
            cooldown_ms: weapon.effective_cooldown_ms,
        })
    }
}
