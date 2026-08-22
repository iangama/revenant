use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const EXPERIENCE_PER_LEVEL: u64 = 500;
const MAX_ACTIVITY_EXPERIENCE: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperienceReward(u64);

impl ExperienceReward {
    /// Validates activity-authored experience against server limits.
    ///
    /// # Errors
    ///
    /// Returns an error for zero or unreasonably large rewards.
    pub fn validated(experience: u64) -> Result<Self, ProgressionError> {
        if experience == 0 {
            return Err(ProgressionError::ZeroExperience);
        }
        if experience > MAX_ACTIVITY_EXPERIENCE {
            return Err(ProgressionError::ExcessiveExperience(experience));
        }
        Ok(Self(experience))
    }

    #[must_use]
    pub fn experience(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progression {
    pub level: u32,
    pub experience: u64,
}

impl Progression {
    #[must_use]
    pub fn from_experience(experience: u64) -> Self {
        let level = u32::try_from(1 + experience / EXPERIENCE_PER_LEVEL).unwrap_or(u32::MAX);
        Self { level, experience }
    }

    #[must_use]
    pub fn grant(self, reward: ExperienceReward) -> Self {
        Self::from_experience(self.experience.saturating_add(reward.experience()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressionError {
    ZeroExperience,
    ExcessiveExperience(u64),
}

impl Display for ProgressionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExperience => formatter.write_str("experience reward must be positive"),
            Self::ExcessiveExperience(value) => {
                write!(formatter, "experience reward exceeds server limit: {value}")
            }
        }
    }
}

impl Error for ProgressionError {}

#[cfg(test)]
mod tests {
    use super::{ExperienceReward, Progression, ProgressionError};

    #[test]
    fn level_is_derived_from_total_experience() {
        let reward = ExperienceReward::validated(100).expect("reward should validate");
        let mut progression = Progression::from_experience(0);
        for _ in 0..5 {
            progression = progression.grant(reward);
        }
        assert_eq!(progression.level, 2);
        assert_eq!(progression.experience, 500);
    }

    #[test]
    fn rejects_invalid_activity_rewards() {
        assert_eq!(
            ExperienceReward::validated(0),
            Err(ProgressionError::ZeroExperience)
        );
        assert_eq!(
            ExperienceReward::validated(10_001),
            Err(ProgressionError::ExcessiveExperience(10_001))
        );
    }
}
