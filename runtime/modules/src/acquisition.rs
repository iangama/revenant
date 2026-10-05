//! Fixed acquisition policy. Callers must persist server-verified milestone
//! proofs and commit the grant and replay atomically before projecting it.
use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const REVISION: &str = "m35-acquisition-v1";
pub const REWARD_FRAGMENTS: u32 = 2;
pub const MAX_FRAGMENTS: u32 = i32::MAX as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Milestone {
    MeridianRecovered,
    BreachCompleted,
    StabilizeCompleted,
    PrismCompleted,
}

impl Milestone {
    pub const ALL: [Self; 4] = [
        Self::MeridianRecovered,
        Self::BreachCompleted,
        Self::StabilizeCompleted,
        Self::PrismCompleted,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MeridianRecovered => "meridian_recovered",
            Self::BreachCompleted => "breach_completed",
            Self::StabilizeCompleted => "stabilize_completed",
            Self::PrismCompleted => "prism_completed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcId {
    Meridian,
    Routes,
    Prism,
}

impl ArcId {
    pub const ALL: [Self; 3] = [Self::Meridian, Self::Routes, Self::Prism];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Meridian => "meridian",
            Self::Routes => "routes",
            Self::Prism => "prism",
        }
    }

    #[must_use]
    pub const fn requirements(self) -> &'static [Milestone] {
        match self {
            Self::Meridian => &[Milestone::MeridianRecovered],
            Self::Routes => &[Milestone::BreachCompleted, Milestone::StabilizeCompleted],
            Self::Prism => &[Milestone::PrismCompleted],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcStatus {
    Locked,
    Ready,
    Claimed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AcquisitionState {
    milestones: BTreeSet<Milestone>,
    claimed: BTreeSet<ArcId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grant {
    pub arc_id: ArcId,
    pub replayed: bool,
    pub granted_fragments: u32,
    pub resulting_fragments: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionError {
    MissingRequirement,
    InvalidClaimedState,
    FragmentOverflow,
}

impl fmt::Display for AcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingRequirement => "commission requirements are incomplete",
            Self::InvalidClaimedState => "claimed commission has no milestone proof",
            Self::FragmentOverflow => "fragment balance exceeds persistence bounds",
        })
    }
}

impl std::error::Error for AcquisitionError {}

impl AcquisitionState {
    /// Restores the finite state after persistence validates each proof's owner.
    ///
    /// # Errors
    /// Rejects a claimed arc without its required milestones.
    pub fn new(milestones: &[Milestone], claimed: &[ArcId]) -> Result<Self, AcquisitionError> {
        let state = Self {
            milestones: milestones.iter().copied().collect(),
            claimed: claimed.iter().copied().collect(),
        };
        if state.claimed.iter().any(|arc| !state.eligible(*arc)) {
            return Err(AcquisitionError::InvalidClaimedState);
        }
        Ok(state)
    }

    /// Records one milestone from an already validated completed session.
    /// Reprocessing that session or another success cannot increase progress.
    pub fn record(&mut self, milestone: Milestone) -> bool {
        self.milestones.insert(milestone)
    }

    #[must_use]
    pub fn status(&self, arc: ArcId) -> ArcStatus {
        if self.claimed.contains(&arc) {
            ArcStatus::Claimed
        } else if self.eligible(arc) {
            ArcStatus::Ready
        } else {
            ArcStatus::Locked
        }
    }

    fn eligible(&self, arc: ArcId) -> bool {
        arc.requirements()
            .iter()
            .all(|id| self.milestones.contains(id))
    }

    /// Calculates a one-time grant against the locked inventory balance.
    /// A retry credits zero and preserves the current balance, including spending
    /// since the original claim. Persist the natural character/arc identity.
    ///
    /// # Errors
    /// Incomplete requirements or overflow leave this state unchanged.
    pub fn claim(&mut self, arc: ArcId, fragments: u32) -> Result<Grant, AcquisitionError> {
        if fragments > MAX_FRAGMENTS {
            return Err(AcquisitionError::FragmentOverflow);
        }
        if !self.eligible(arc) {
            return Err(AcquisitionError::MissingRequirement);
        }
        let replayed = self.claimed.contains(&arc);
        let granted_fragments = if replayed { 0 } else { REWARD_FRAGMENTS };
        let resulting_fragments = fragments
            .checked_add(granted_fragments)
            .filter(|value| *value <= MAX_FRAGMENTS)
            .ok_or(AcquisitionError::FragmentOverflow)?;
        self.claimed.insert(arc);
        Ok(Grant {
            arc_id: arc,
            replayed,
            granted_fragments,
            resulting_fragments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_need_two_distinct_successes_and_retries_never_grant_twice() {
        let mut state = AcquisitionState::default();
        assert!(state.record(Milestone::BreachCompleted));
        assert!(!state.record(Milestone::BreachCompleted));
        assert_eq!(
            state.claim(ArcId::Routes, 3),
            Err(AcquisitionError::MissingRequirement)
        );
        assert_eq!(state.status(ArcId::Routes), ArcStatus::Locked);
        state.record(Milestone::StabilizeCompleted);
        assert_eq!(state.status(ArcId::Routes), ArcStatus::Ready);
        let grant = state.claim(ArcId::Routes, 3).unwrap();
        assert_eq!(grant.resulting_fragments, 5);
        assert!(!grant.replayed);
        // Simulate spending after the first grant, then retrying its request.
        let retry = state.claim(ArcId::Routes, 1).unwrap();
        assert!(retry.replayed);
        assert_eq!(retry.granted_fragments, 0);
        assert_eq!(retry.resulting_fragments, 1);
    }

    #[test]
    fn restoration_and_overflow_cannot_fabricate_or_consume_rewards() {
        assert_eq!(
            AcquisitionState::new(&[], &[ArcId::Prism]),
            Err(AcquisitionError::InvalidClaimedState)
        );
        let mut state = AcquisitionState::new(&[Milestone::PrismCompleted], &[]).unwrap();
        let before = state.clone();
        assert_eq!(
            state.claim(ArcId::Prism, MAX_FRAGMENTS - 1),
            Err(AcquisitionError::FragmentOverflow)
        );
        assert_eq!(state, before);
        assert_eq!(state.claim(ArcId::Prism, 0).unwrap().resulting_fragments, 2);
        let mut restored =
            AcquisitionState::new(&[Milestone::PrismCompleted], &[ArcId::Prism]).unwrap();
        assert_eq!(restored.status(ArcId::Prism), ArcStatus::Claimed);
        assert_eq!(
            restored.claim(ArcId::Prism, 2).unwrap().granted_fragments,
            0
        );
    }
}
