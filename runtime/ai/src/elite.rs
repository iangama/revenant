//! Alternating danger windows for the planned Bulwark + Lancer elite pair.
//! Clone before proposing a transition; publish it only after durable evidence.

use crate::{bulwark, lancer, support};
use revenant_actors::{Actor, ActorKind};

pub const BREATHER_MS: u64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Warning {
    actor_id: u64,
    resolves_at_ms: u64,
    duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EliteCadence {
    actors: [u64; 2],
    alive: [bool; 2],
    next: usize,
    available_at_ms: u64,
    warning: Option<Warning>,
}

impl EliteCadence {
    /// Bulwark warns first; Lancer takes the next turn while the shield recovers.
    #[must_use]
    pub fn new(bulwark_id: u64, lancer_id: u64) -> Option<Self> {
        if bulwark_id == 0 || lancer_id == 0 || bulwark_id == lancer_id {
            return None;
        }
        Some(Self {
            actors: [bulwark_id, lancer_id],
            alive: [true, true],
            next: 0,
            available_at_ms: 0,
            warning: None,
        })
    }

    #[must_use]
    pub fn ready_actor(&self, now_ms: u64) -> Option<u64> {
        if self.warning.is_some() || now_ms < self.available_at_ms {
            return None;
        }
        [self.next, 1 - self.next]
            .into_iter()
            .find(|index| self.alive[*index])
            .map(|index| self.actors[index])
    }

    pub fn begin_warning(&mut self, actor_id: u64, now_ms: u64) -> bool {
        if self.ready_actor(now_ms) != Some(actor_id) {
            return false;
        }
        let duration_ms = if actor_id == self.actors[0] {
            bulwark::WINDUP_MS
        } else {
            lancer::WINDUP_MS
        };
        self.warning = Some(Warning {
            actor_id,
            resolves_at_ms: now_ms.saturating_add(duration_ms),
            duration_ms,
        });
        true
    }

    pub fn resolve_warning(&mut self, actor_id: u64, now_ms: u64) -> bool {
        let Some(warning) = self.warning else {
            return false;
        };
        if warning.actor_id != actor_id || now_ms < warning.resolves_at_ms {
            return false;
        }
        self.warning = None;
        self.next = usize::from(actor_id == self.actors[0]);
        self.available_at_ms = now_ms.saturating_add(BREATHER_MS);
        true
    }

    /// Apply alongside the persisted death; an obsolete warning cannot hold the pair.
    pub fn retire(&mut self, actor_id: u64, now_ms: u64) {
        if let Some(index) = self.actors.iter().position(|id| *id == actor_id) {
            self.alive[index] = false;
            if self
                .warning
                .is_some_and(|warning| warning.actor_id == actor_id)
            {
                self.warning = None;
                self.next = 1 - index;
                self.available_at_ms = now_ms.saturating_add(BREATHER_MS);
            }
        }
    }

    /// A slow write extends the visible warning or the gap after its resolution.
    pub fn confirm_at(&mut self, now_ms: u64) {
        if let Some(warning) = &mut self.warning {
            warning.resolves_at_ms = warning
                .resolves_at_ms
                .max(now_ms.saturating_add(warning.duration_ms));
        } else {
            self.available_at_ms = self.available_at_ms.max(now_ms.saturating_add(BREATHER_MS));
        }
    }
}

/// Run the existing locked charge in the north pad without changing solo Lancer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NorthPadLancer {
    ai: lancer::GlassLancer,
}

impl NorthPadLancer {
    pub fn tick(
        &mut self,
        now_ms: u64,
        origin: [i32; 3],
        player: [i32; 3],
    ) -> Option<lancer::ChargeAction> {
        if !bulwark::arena_contains(origin) || !bulwark::arena_contains(player) {
            return None;
        }
        match self.ai.tick(now_ms, to_south(origin), to_south(player))? {
            lancer::ChargeAction::Windup { origin, target } => Some(lancer::ChargeAction::Windup {
                origin: to_north(origin),
                target: to_north(target),
            }),
            lancer::ChargeAction::Resolved {
                origin,
                target,
                damage,
            } => Some(lancer::ChargeAction::Resolved {
                origin: to_north(origin),
                target: to_north(target),
                damage,
            }),
        }
    }

    pub fn confirm_at(&mut self, now_ms: u64) {
        self.ai.confirm_at(now_ms);
    }
}

#[must_use]
pub fn north_charge_hits(origin: [i32; 3], target: [i32; 3], player: [i32; 3]) -> bool {
    bulwark::arena_contains(origin)
        && bulwark::arena_contains(target)
        && bulwark::arena_contains(player)
        && lancer::charge_hits(to_south(origin), to_south(target), to_south(player))
}

// Called only after validating the small arena, so these translations cannot overflow.
fn to_south(position: [i32; 3]) -> [i32; 3] {
    [position[0], position[1], position[2] + 15]
}

fn to_north(position: [i32; 3]) -> [i32; 3] {
    [position[0], position[1], position[2] - 15]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EliteComposition {
    BastionLink,
    CrossedGuard,
}

impl EliteComposition {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::BastionLink => "bastion_link",
            Self::CrossedGuard => "crossed_guard",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "bastion_link" => Some(Self::BastionLink),
            "crossed_guard" => Some(Self::CrossedGuard),
            _ => None,
        }
    }

    #[must_use]
    pub const fn entrance(self) -> [i32; 3] {
        match self {
            Self::BastionLink => [5, 0, -6],
            Self::CrossedGuard => [5, 0, -10],
        }
    }

    #[must_use]
    pub const fn partner(self) -> (&'static str, [i32; 3], u32) {
        match self {
            Self::BastionLink => ("relay-mender", [9, 0, -6], support::HEALTH),
            Self::CrossedGuard => ("glass-lancer", [9, 0, -9], lancer::HEALTH),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EliteAction {
    Defense(bulwark::BulwarkAction),
    Charge(lancer::ChargeAction),
    Repair(support::RepairAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EliteBrain {
    composition: EliteComposition,
    bulwark_id: u64,
    partner_id: u64,
    defense: bulwark::SteelBulwark,
    charge: NorthPadLancer,
    repair: support::RelayMender,
    cadence: EliteCadence,
}

impl EliteBrain {
    #[must_use]
    pub fn new(composition: EliteComposition, bulwark_id: u64, partner_id: u64) -> Option<Self> {
        let mut cadence = EliteCadence::new(bulwark_id, partner_id)?;
        // The spawn projection already contains the Bulwark's initial warning.
        cadence.begin_warning(bulwark_id, 0);
        Some(Self {
            composition,
            bulwark_id,
            partner_id,
            cadence,
            defense: bulwark::SteelBulwark::default(),
            charge: NorthPadLancer::default(),
            repair: support::RelayMender::default(),
        })
    }

    #[must_use]
    pub fn defense(&self) -> &bulwark::SteelBulwark {
        &self.defense
    }

    /// Propose at most one durable transition. The registry is never mutated here.
    pub fn tick(
        &mut self,
        now_ms: u64,
        guard: Option<&Actor>,
        partner: Option<&Actor>,
        player: &Actor,
    ) -> Option<EliteAction> {
        if player.kind != ActorKind::Player
            || player.health == 0
            || !bulwark::arena_contains(player.position)
        {
            return None;
        }
        let guard = guard.filter(|actor| living_enemy(actor, self.bulwark_id));
        let partner = partner.filter(|actor| living_enemy(actor, self.partner_id));
        if self.composition == EliteComposition::BastionLink {
            if let Some(guard) = guard {
                if let Some(action) = self.defense.tick(now_ms, guard.position, player.position) {
                    return Some(EliteAction::Defense(action));
                }
                if let Some(partner) = partner {
                    return self
                        .repair
                        .tick(now_ms, partner, std::slice::from_ref(guard))
                        .map(EliteAction::Repair);
                }
            }
            return None;
        }
        let selected = self
            .cadence
            .warning
            .map(|warning| warning.actor_id)
            .or_else(|| self.cadence.ready_actor(now_ms))?;
        if selected == self.bulwark_id {
            let guard = guard?;
            let mut proposed = self.defense.clone();
            let action = proposed.tick(now_ms, guard.position, player.position)?;
            let accepted = match action {
                bulwark::BulwarkAction::Brace { .. } => {
                    self.cadence.begin_warning(selected, now_ms)
                }
                bulwark::BulwarkAction::Slam { .. } => {
                    self.cadence.resolve_warning(selected, now_ms)
                }
            };
            if !accepted {
                return None;
            }
            self.defense = proposed;
            Some(EliteAction::Defense(action))
        } else {
            let partner = partner?;
            let mut proposed = self.charge.clone();
            let action = proposed.tick(now_ms, partner.position, player.position)?;
            let accepted = match action {
                lancer::ChargeAction::Windup { .. } => self.cadence.begin_warning(selected, now_ms),
                lancer::ChargeAction::Resolved { .. } => {
                    self.cadence.resolve_warning(selected, now_ms)
                }
            };
            if !accepted {
                return None;
            }
            self.charge = proposed;
            Some(EliteAction::Charge(action))
        }
    }

    pub fn confirm_at(&mut self, action: EliteAction, now_ms: u64) {
        match action {
            EliteAction::Defense(_) => self.defense.confirm_at(now_ms),
            EliteAction::Charge(_) => self.charge.confirm_at(now_ms),
            EliteAction::Repair(_) => {
                self.repair.confirm_at(now_ms);
                return;
            }
        }
        if self.composition == EliteComposition::CrossedGuard {
            self.cadence.confirm_at(now_ms);
        }
    }

    /// Use the death's confirmation time to preserve a full cancellation gap.
    pub fn retire(&mut self, actor_id: u64, confirmed_ms: u64) {
        self.cadence.retire(actor_id, confirmed_ms);
    }
}

fn living_enemy(actor: &Actor, id: u64) -> bool {
    actor.id == id
        && actor.kind == ActorKind::Enemy
        && actor.health > 0
        && bulwark::arena_contains(actor.position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elite_danger_alternates_with_complete_warnings_and_a_breather() {
        let mut cadence = EliteCadence::new(1, 2).unwrap();
        assert!(!cadence.begin_warning(2, 0));
        assert!(cadence.begin_warning(1, 0));
        assert!(!cadence.begin_warning(2, 700));
        assert!(!cadence.resolve_warning(1, 1599));
        assert!(cadence.resolve_warning(1, 1600));
        assert_eq!(cadence.ready_actor(2199), None);
        assert!(cadence.begin_warning(2, 2200));
        assert!(!cadence.resolve_warning(1, 4000));
        assert!(!cadence.resolve_warning(2, 3999));
        assert!(cadence.resolve_warning(2, 4000));
        assert_eq!(cadence.ready_actor(4600), Some(1));
    }

    #[test]
    fn elite_slow_confirmation_preserves_warning_and_recovery() {
        let mut cadence = EliteCadence::new(1, 2).unwrap();
        let mut proposed = cadence.clone();
        assert!(proposed.begin_warning(1, 0));
        assert_eq!(
            cadence.ready_actor(0),
            Some(1),
            "failed write keeps the proposal retryable"
        );
        proposed.confirm_at(5000);
        cadence = proposed;
        assert!(!cadence.resolve_warning(1, 6599));
        assert!(cadence.resolve_warning(1, 6600));
        cadence.confirm_at(9000);
        assert_eq!(cadence.ready_actor(9599), None);
        assert_eq!(cadence.ready_actor(9600), Some(2));
    }

    #[test]
    fn elite_dead_members_release_their_warning_and_never_take_another_turn() {
        assert!(EliteCadence::new(1, 1).is_none());
        assert!(EliteCadence::new(0, 2).is_none());
        for dead in [1, 2] {
            let mut cadence = EliteCadence::new(1, 2).unwrap();
            cadence.begin_warning(1, 0);
            cadence.retire(dead, 500);
            if dead == 1 {
                assert_eq!(cadence.ready_actor(1099), None);
                assert_eq!(cadence.ready_actor(1100), Some(2));
            } else {
                assert!(cadence.resolve_warning(1, 1600));
                assert_eq!(cadence.ready_actor(2200), Some(1));
            }
            cadence.retire(3 - dead, 2500);
            assert_eq!(cadence.ready_actor(u64::MAX), None);
        }
    }
}

#[cfg(test)]
mod north_pad_tests {
    use super::*;

    #[test]
    fn north_lancer_keeps_locked_target_damage_and_slow_warning_in_world_coordinates() {
        let mut ai = NorthPadLancer::default();
        let origin = [9, 0, -8];
        let target = [6, 0, -8];
        assert_eq!(
            ai.tick(700, origin, target),
            Some(lancer::ChargeAction::Windup { origin, target })
        );
        ai.confirm_at(2000);
        assert_eq!(ai.tick(3799, origin, [6, 0, -7]), None);
        assert_eq!(
            ai.tick(3800, origin, [6, 0, -7]),
            Some(lancer::ChargeAction::Resolved {
                origin,
                target,
                damage: 0
            })
        );
        assert!(north_charge_hits(origin, target, [8, 0, -8]));
        assert!(!north_charge_hits(origin, target, [8, 0, -7]));
        assert!(!north_charge_hits(origin, target, [i32::MAX, 0, i32::MIN]));
        assert_eq!(ai.tick(u64::MAX, origin, [i32::MAX, 0, i32::MIN]), None);
    }
}
