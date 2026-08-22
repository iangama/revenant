use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayEventKind {
    PlayerJoined,
    ActivityStarted,
    EnemySpawned,
    EnemyDied,
    BossSpawned,
    ActivityCompleted,
    LootGranted,
}

impl Display for ReplayEventKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PlayerJoined => "player_joined",
            Self::ActivityStarted => "activity_started",
            Self::EnemySpawned => "enemy_spawned",
            Self::EnemyDied => "enemy_died",
            Self::BossSpawned => "boss_spawned",
            Self::ActivityCompleted => "activity_completed",
            Self::LootGranted => "loot_granted",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReplayEventKind(pub String);

impl Display for UnknownReplayEventKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown replay event kind: {}", self.0)
    }
}

impl Error for UnknownReplayEventKind {}

impl FromStr for ReplayEventKind {
    type Err = UnknownReplayEventKind;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "player_joined" => Ok(Self::PlayerJoined),
            "activity_started" => Ok(Self::ActivityStarted),
            "enemy_spawned" => Ok(Self::EnemySpawned),
            "enemy_died" => Ok(Self::EnemyDied),
            "boss_spawned" => Ok(Self::BossSpawned),
            "activity_completed" => Ok(Self::ActivityCompleted),
            "loot_granted" => Ok(Self::LootGranted),
            _ => Err(UnknownReplayEventKind(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEvent {
    pub id: i64,
    pub kind: ReplayEventKind,
    pub timestamp: String,
    pub session_id: String,
    pub activity_id: Option<String>,
    pub actor_id: Option<u64>,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedSession {
    pub session_id: String,
    pub activity_id: Option<String>,
    pub player_actor_id: Option<u64>,
    pub spawned_enemies: usize,
    pub defeated_enemies: usize,
    pub boss_spawned: bool,
    pub completed: bool,
    pub loot_grants: usize,
    pub timeline: Vec<String>,
}

/// Reconstructs the observable state and textual timeline of one session.
///
/// # Errors
///
/// Returns an error if the event list is empty or mixes session identifiers.
pub fn reconstruct(events: &[ReplayEvent]) -> Result<ReconstructedSession, ReplayError> {
    let first = events.first().ok_or(ReplayError::EmptySession)?;
    let mut state = ReconstructedSession {
        session_id: first.session_id.clone(),
        activity_id: None,
        player_actor_id: None,
        spawned_enemies: 0,
        defeated_enemies: 0,
        boss_spawned: false,
        completed: false,
        loot_grants: 0,
        timeline: Vec::with_capacity(events.len()),
    };
    for event in events {
        if event.session_id != state.session_id {
            return Err(ReplayError::MixedSessions);
        }
        if event.activity_id.is_some() {
            state.activity_id.clone_from(&event.activity_id);
        }
        match event.kind {
            ReplayEventKind::PlayerJoined => state.player_actor_id = event.actor_id,
            ReplayEventKind::EnemySpawned => state.spawned_enemies += 1,
            ReplayEventKind::EnemyDied => state.defeated_enemies += 1,
            ReplayEventKind::BossSpawned => state.boss_spawned = true,
            ReplayEventKind::ActivityCompleted => state.completed = true,
            ReplayEventKind::LootGranted => state.loot_grants += 1,
            ReplayEventKind::ActivityStarted => {}
        }
        state.timeline.push(format!(
            "{} | {} | {}",
            event.timestamp, event.kind, event.payload
        ));
    }
    Ok(state)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayError {
    EmptySession,
    MixedSessions,
}

impl Display for ReplayError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptySession => "session has no replay events",
            Self::MixedSessions => "replay events belong to different sessions",
        })
    }
}

impl Error for ReplayError {}

#[cfg(test)]
mod tests {
    use super::{reconstruct, ReplayEvent, ReplayEventKind};

    fn event(id: i64, kind: ReplayEventKind) -> ReplayEvent {
        ReplayEvent {
            id,
            kind,
            timestamp: format!("2026-01-01 00:00:0{id}+00"),
            session_id: "session-1".to_owned(),
            activity_id: Some("relay_awakening".to_owned()),
            actor_id: Some(u64::try_from(id).expect("positive test id")),
            payload: kind.to_string().replace('_', " "),
        }
    }

    #[test]
    fn reconstructs_completed_session_and_timeline() {
        let kinds = [
            ReplayEventKind::PlayerJoined,
            ReplayEventKind::ActivityStarted,
            ReplayEventKind::EnemySpawned,
            ReplayEventKind::EnemyDied,
            ReplayEventKind::BossSpawned,
            ReplayEventKind::EnemyDied,
            ReplayEventKind::ActivityCompleted,
            ReplayEventKind::LootGranted,
        ];
        let events = kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| event(i64::try_from(index + 1).expect("small index"), kind))
            .collect::<Vec<_>>();
        let state = reconstruct(&events).expect("valid session should reconstruct");
        assert_eq!(state.spawned_enemies, 1);
        assert_eq!(state.defeated_enemies, 2);
        assert!(state.boss_spawned);
        assert!(state.completed);
        assert_eq!(state.loot_grants, 1);
        assert_eq!(state.timeline.len(), events.len());
    }
}
