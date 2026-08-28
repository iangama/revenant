use postgres::{Client, NoTls};
use revenant_progression::{ExperienceReward, Progression as DomainProgression};
use std::fmt;

const SCHEMA: &str = concat!(
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_replay_events.sql"),
    include_str!("../migrations/0003_inventory_rewards.sql"),
    include_str!("../migrations/0004_progression_rewards.sql"),
    include_str!("../migrations/0005_equipment_loadouts.sql")
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCharacter {
    pub id: String,
    pub display_name: String,
    pub class_name: String,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEntry {
    pub item_id: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progression {
    pub level: i32,
    pub experience: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionRewards {
    pub item_quantity: i32,
    pub experience_granted: i64,
    pub previous_level: i32,
    pub level: i32,
    pub experience: i64,
}

pub struct ActivityCompletion<'a> {
    pub session_id: &'a str,
    pub account_id: &'a str,
    pub character_id: &'a str,
    pub activity_id: &'a str,
    pub item_id: &'a str,
    pub item_quantity: i32,
    pub experience_reward: ExperienceReward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReplayEvent<'a> {
    pub event_type: &'a str,
    pub session_id: &'a str,
    pub account_id: &'a str,
    pub activity_id: Option<&'a str>,
    pub actor_id: Option<i64>,
    pub payload: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedReplayEvent {
    pub id: i64,
    pub event_type: String,
    pub timestamp: String,
    pub session_id: String,
    pub activity_id: Option<String>,
    pub actor_id: Option<i64>,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedReplaySession {
    pub session_id: String,
    pub activity_id: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub event_count: i64,
    pub participant_count: i64,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeSessionSummary {
    pub session_id: String,
    pub activity_id: Option<String>,
    pub first_joined_at: String,
    pub activity_started_at: Option<String>,
    pub activity_ended_at: String,
    pub join_to_start_ms: Option<i64>,
    pub activity_duration_ms: Option<i64>,
    pub participant_count: i64,
    pub completed: bool,
    pub enemy_spawn_count: i64,
    pub enemy_defeat_count: i64,
    pub boss_spawned: bool,
    pub equipment_change_count: i64,
    pub loot_grant_count: i64,
    pub progression_grant_count: i64,
    pub event_count: i64,
}

#[derive(Debug)]
pub enum SessionSummaryError {
    Database(postgres::Error),
    InvalidEvidence(&'static str),
}

impl fmt::Display for SessionSummaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => error.fmt(formatter),
            Self::InvalidEvidence(message) => {
                write!(formatter, "invalid replay evidence: {message}")
            }
        }
    }
}

impl std::error::Error for SessionSummaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::InvalidEvidence(_) => None,
        }
    }
}

impl From<postgres::Error> for SessionSummaryError {
    fn from(error: postgres::Error) -> Self {
        Self::Database(error)
    }
}

pub struct Persistence {
    client: Client,
}

impl Persistence {
    /// Connects to `PostgreSQL` and applies the idempotent Revenant schema.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the connection or migration fails.
    pub fn connect(database_url: &str) -> Result<Self, postgres::Error> {
        let mut client = Client::connect(database_url, NoTls)?;
        let mut transaction = client.transaction()?;
        transaction.query_one("SELECT pg_advisory_xact_lock(824_180_018)", &[])?;
        transaction.batch_execute(SCHEMA)?;
        transaction.commit()?;
        Ok(Self { client })
    }

    /// Creates the local account and its initial operator when they do not exist.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the transaction fails.
    pub fn ensure_local_account(
        &mut self,
        account_id: &str,
        username: &str,
    ) -> Result<(), postgres::Error> {
        let character_id = format!("{account_id}:operator");
        let mut transaction = self.client.transaction()?;
        transaction.execute(
            "INSERT INTO accounts (id, username) VALUES ($1, $2) \
             ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username",
            &[&account_id, &username],
        )?;
        transaction.execute(
            "INSERT INTO characters (id, account_id, display_name, class_name, level) \
             VALUES ($1, $2, $3, 'Operator', 1) ON CONFLICT (id) DO NOTHING",
            &[&character_id, &account_id, &username],
        )?;
        transaction.execute(
            "INSERT INTO progression (character_id, level, experience) VALUES ($1, 1, 0) \
             ON CONFLICT (character_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, 'pulse_rifle', 1) \
             ON CONFLICT (character_id, item_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, 'arc_sidearm', 1) \
             ON CONFLICT (character_id, item_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.execute(
            "INSERT INTO equipment_loadouts (character_id, weapon_item_id) \
             VALUES ($1, 'pulse_rifle') ON CONFLICT (character_id) DO NOTHING",
            &[&character_id],
        )?;
        transaction.commit()
    }

    /// Loads characters owned by an account in stable identifier order.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn characters_for(
        &mut self,
        account_id: &str,
    ) -> Result<Vec<PersistedCharacter>, postgres::Error> {
        self.client
            .query(
                "SELECT id, display_name, class_name, level FROM characters \
                 WHERE account_id = $1 ORDER BY id",
                &[&account_id],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| PersistedCharacter {
                        id: row.get(0),
                        display_name: row.get(1),
                        class_name: row.get(2),
                        level: row.get(3),
                    })
                    .collect()
            })
    }

    /// Loads the persisted inventory for a character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn inventory_for(
        &mut self,
        character_id: &str,
    ) -> Result<Vec<InventoryEntry>, postgres::Error> {
        self.client
            .query(
                "SELECT item_id, quantity FROM inventory WHERE character_id = $1 ORDER BY item_id",
                &[&character_id],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| InventoryEntry {
                        item_id: row.get(0),
                        quantity: row.get(1),
                    })
                    .collect()
            })
    }

    /// Loads progression for a character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn progression_for(
        &mut self,
        character_id: &str,
    ) -> Result<Option<Progression>, postgres::Error> {
        self.client
            .query_opt(
                "SELECT level, experience FROM progression WHERE character_id = $1",
                &[&character_id],
            )
            .map(|row| {
                row.map(|row| Progression {
                    level: row.get(0),
                    experience: row.get(1),
                })
            })
    }

    /// Loads the selected weapon for a character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn equipped_weapon_for(
        &mut self,
        character_id: &str,
    ) -> Result<Option<String>, postgres::Error> {
        self.client
            .query_opt(
                "SELECT weapon_item_id FROM equipment_loadouts WHERE character_id = $1",
                &[&character_id],
            )
            .map(|row| row.map(|row| row.get(0)))
    }

    /// Persists the authoritative weapon selection.
    ///
    /// The caller validates ownership and equipability before crossing this boundary.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the update fails.
    pub fn equip_weapon(
        &mut self,
        character_id: &str,
        item_id: &str,
    ) -> Result<bool, postgres::Error> {
        self.client
            .execute(
                "UPDATE equipment_loadouts SET weapon_item_id = $2 WHERE character_id = $1",
                &[&character_id, &item_id],
            )
            .map(|updated| updated == 1)
    }

    /// Records a completed activity for operational history.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the insert fails.
    pub fn record_activity_completion(
        &mut self,
        account_id: &str,
        character_id: &str,
        activity_id: &str,
    ) -> Result<(), postgres::Error> {
        self.client.execute(
            "INSERT INTO activity_history (account_id, character_id, activity_id) VALUES ($1, $2, $3)",
            &[&account_id, &character_id, &activity_id],
        )?;
        Ok(())
    }

    /// Atomically records completion and grants idempotent inventory and progression rewards.
    ///
    /// Returns the resulting authoritative state, or `None` when this session was
    /// already applied to the character.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the transaction fails.
    pub fn complete_activity_with_rewards(
        &mut self,
        completion: &ActivityCompletion<'_>,
    ) -> Result<Option<CompletionRewards>, postgres::Error> {
        let mut transaction = self.client.transaction()?;
        let inserted = transaction.query_opt(
            "INSERT INTO inventory_reward_grants (session_id, character_id, item_id, quantity) \
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING quantity",
            &[
                &completion.session_id,
                &completion.character_id,
                &completion.item_id,
                &completion.item_quantity,
            ],
        )?;
        if inserted.is_none() {
            transaction.commit()?;
            return Ok(None);
        }
        let resulting_quantity: i32 = transaction
            .query_one(
                "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, $2, $3) \
                 ON CONFLICT (character_id, item_id) DO UPDATE \
                 SET quantity = inventory.quantity + EXCLUDED.quantity RETURNING quantity",
                &[
                    &completion.character_id,
                    &completion.item_id,
                    &completion.item_quantity,
                ],
            )?
            .get(0);
        let progression_row = transaction.query_one(
            "SELECT level, experience FROM progression WHERE character_id = $1 FOR UPDATE",
            &[&completion.character_id],
        )?;
        let previous_level: i32 = progression_row.get(0);
        let previous_experience: i64 = progression_row.get(1);
        let current = DomainProgression::from_experience(previous_experience.unsigned_abs());
        let resulting = current.grant(completion.experience_reward);
        let experience = i64::try_from(resulting.experience).unwrap_or(i64::MAX);
        let level = i32::try_from(resulting.level).unwrap_or(i32::MAX);
        let experience_granted =
            i64::try_from(completion.experience_reward.experience()).unwrap_or(i64::MAX);
        transaction.execute(
            "INSERT INTO progression_reward_grants (session_id, character_id, experience) \
             VALUES ($1, $2, $3)",
            &[
                &completion.session_id,
                &completion.character_id,
                &experience_granted,
            ],
        )?;
        transaction.execute(
            "UPDATE progression SET level = $2, experience = $3 WHERE character_id = $1",
            &[&completion.character_id, &level, &experience],
        )?;
        transaction.execute(
            "UPDATE characters SET level = $2 WHERE id = $1",
            &[&completion.character_id, &level],
        )?;
        transaction.execute(
            "INSERT INTO activity_history (account_id, character_id, activity_id) VALUES ($1, $2, $3)",
            &[
                &completion.account_id,
                &completion.character_id,
                &completion.activity_id,
            ],
        )?;
        transaction.commit()?;
        Ok(Some(CompletionRewards {
            item_quantity: resulting_quantity,
            experience_granted,
            previous_level,
            level,
            experience,
        }))
    }

    /// Counts recorded completions for smoke and diagnostic tooling.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn activity_completion_count(
        &mut self,
        account_id: &str,
        activity_id: &str,
    ) -> Result<i64, postgres::Error> {
        self.client
            .query_one(
                "SELECT COUNT(*) FROM activity_history WHERE account_id = $1 AND activity_id = $2",
                &[&account_id, &activity_id],
            )
            .map(|row| row.get(0))
    }

    /// Appends one immutable event to a replay session.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the insert fails.
    pub fn append_replay_event(
        &mut self,
        event: &NewReplayEvent<'_>,
    ) -> Result<i64, postgres::Error> {
        self.client
            .query_one(
                "INSERT INTO replay_events \
                 (event_type, session_id, account_id, activity_id, actor_id, payload) \
                 VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
                &[
                    &event.event_type,
                    &event.session_id,
                    &event.account_id,
                    &event.activity_id,
                    &event.actor_id,
                    &event.payload,
                ],
            )
            .map(|row| row.get(0))
    }

    /// Loads one session in its authoritative append order.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn replay_events(
        &mut self,
        session_id: &str,
    ) -> Result<Vec<PersistedReplayEvent>, postgres::Error> {
        self.client
            .query(
                "SELECT id, event_type, occurred_at::TEXT, session_id, activity_id, actor_id, payload \
                 FROM replay_events WHERE session_id = $1 ORDER BY id",
                &[&session_id],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| PersistedReplayEvent {
                        id: row.get(0),
                        event_type: row.get(1),
                        timestamp: row.get(2),
                        session_id: row.get(3),
                        activity_id: row.get(4),
                        actor_id: row.get(5),
                        payload: row.get(6),
                    })
                    .collect()
            })
    }

    /// Lists the most recent replay sessions for inspection.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn replay_sessions(
        &mut self,
        limit: i64,
    ) -> Result<Vec<PersistedReplaySession>, postgres::Error> {
        self.client
            .query(
                "SELECT session_id, MAX(activity_id), MIN(occurred_at)::TEXT, \
                        MAX(occurred_at)::TEXT, COUNT(*), COUNT(DISTINCT account_id), \
                        BOOL_OR(event_type = 'activity_completed') \
                 FROM replay_events GROUP BY session_id \
                 ORDER BY MAX(id) DESC LIMIT $1",
                &[&limit.clamp(1, 100)],
            )
            .map(|rows| {
                rows.into_iter()
                    .map(|row| PersistedReplaySession {
                        session_id: row.get(0),
                        activity_id: row.get(1),
                        started_at: row.get(2),
                        ended_at: row.get(3),
                        event_count: row.get(4),
                        participant_count: row.get(5),
                        completed: row.get(6),
                    })
                    .collect()
            })
    }

    /// Derives the bounded authoritative playtest summary for one replay session.
    ///
    /// # Errors
    ///
    /// Returns an error when the query fails or persisted timestamps contradict replay order.
    pub fn authoritative_session_summary(
        &mut self,
        session_id: &str,
    ) -> Result<Option<AuthoritativeSessionSummary>, SessionSummaryError> {
        let row = self.client.query_opt(
            "WITH summary AS ( \
                 SELECT session_id, MAX(activity_id) AS activity_id, \
                        MIN(occurred_at) FILTER (WHERE event_type = 'player_joined') AS first_joined_at, \
                        MIN(occurred_at) FILTER (WHERE event_type = 'activity_started') AS activity_started_at, \
                        MIN(occurred_at) FILTER (WHERE event_type = 'activity_completed') AS completed_at, \
                        MAX(occurred_at) AS latest_at, COUNT(DISTINCT account_id) AS participant_count, \
                        COUNT(*) FILTER (WHERE event_type = 'enemy_spawned') AS enemy_spawn_count, \
                        COUNT(*) FILTER (WHERE event_type = 'enemy_died') AS enemy_defeat_count, \
                        BOOL_OR(event_type = 'boss_spawned') AS boss_spawned, \
                        COUNT(*) FILTER (WHERE event_type = 'equipment_changed') AS equipment_change_count, \
                        COUNT(*) FILTER (WHERE event_type = 'loot_granted') AS loot_grant_count, \
                        COUNT(*) FILTER (WHERE event_type = 'progression_granted') AS progression_grant_count, \
                        COUNT(*) AS event_count, \
                        COUNT(*) FILTER (WHERE event_type NOT IN ( \
                            'player_joined', 'activity_started', 'enemy_spawned', 'enemy_died', \
                            'boss_spawned', 'activity_completed', 'loot_granted', \
                            'progression_granted', 'equipment_changed' \
                        )) AS unknown_event_count \
                 FROM replay_events WHERE session_id = $1 GROUP BY session_id \
             ) \
             SELECT session_id, activity_id, first_joined_at::TEXT, activity_started_at::TEXT, \
                    COALESCE(completed_at, latest_at)::TEXT, \
                    CASE WHEN first_joined_at IS NULL OR activity_started_at IS NULL THEN NULL \
                         ELSE (EXTRACT(EPOCH FROM (activity_started_at - first_joined_at)) * 1000)::BIGINT END, \
                    CASE WHEN activity_started_at IS NULL OR completed_at IS NULL THEN NULL \
                         ELSE (EXTRACT(EPOCH FROM (completed_at - activity_started_at)) * 1000)::BIGINT END, \
                    participant_count, completed_at IS NOT NULL, enemy_spawn_count, enemy_defeat_count, \
                    boss_spawned, equipment_change_count, loot_grant_count, progression_grant_count, event_count, \
                    unknown_event_count \
             FROM summary",
            &[&session_id],
        )?;
        let Some(row) = row else {
            return Ok(None);
        };
        let first_joined_at =
            row.get::<_, Option<String>>(2)
                .ok_or(SessionSummaryError::InvalidEvidence(
                    "session has no player_joined event",
                ))?;
        let join_to_start_ms: Option<i64> = row.get(5);
        let activity_duration_ms: Option<i64> = row.get(6);
        if join_to_start_ms.is_some_and(|elapsed| elapsed < 0) {
            return Err(SessionSummaryError::InvalidEvidence(
                "activity_started precedes player_joined",
            ));
        }
        if activity_duration_ms.is_some_and(|elapsed| elapsed < 0) {
            return Err(SessionSummaryError::InvalidEvidence(
                "activity_completed precedes activity_started",
            ));
        }
        if row.get::<_, i64>(16) > 0 {
            return Err(SessionSummaryError::InvalidEvidence(
                "session contains an unknown replay event kind",
            ));
        }
        Ok(Some(AuthoritativeSessionSummary {
            session_id: row.get(0),
            activity_id: row.get(1),
            first_joined_at,
            activity_started_at: row.get(3),
            activity_ended_at: row.get(4),
            join_to_start_ms,
            activity_duration_ms,
            participant_count: row.get(7),
            completed: row.get(8),
            enemy_spawn_count: row.get(9),
            enemy_defeat_count: row.get(10),
            boss_spawned: row.get(11),
            equipment_change_count: row.get(12),
            loot_grant_count: row.get(13),
            progression_grant_count: row.get(14),
            event_count: row.get(15),
        }))
    }

    /// Finds the most recently completed replay session for an account.
    ///
    /// # Errors
    ///
    /// Returns the `PostgreSQL` error when the query fails.
    pub fn latest_completed_session_id(
        &mut self,
        account_id: &str,
    ) -> Result<Option<String>, postgres::Error> {
        self.client
            .query_opt(
                "SELECT completed.session_id FROM replay_events AS completed \
                 WHERE completed.event_type = 'activity_completed' \
                   AND EXISTS (SELECT 1 FROM replay_events AS participant \
                               WHERE participant.session_id = completed.session_id \
                                 AND participant.account_id = $1) \
                 ORDER BY completed.id DESC LIMIT 1",
                &[&account_id],
            )
            .map(|row| row.map(|row| row.get(0)))
    }
}

#[cfg(test)]
mod tests {
    use super::SCHEMA;

    #[test]
    fn schema_covers_persisted_state_and_replay() {
        for table in [
            "accounts",
            "characters",
            "inventory",
            "progression",
            "activity_history",
            "replay_events",
            "inventory_reward_grants",
            "progression_reward_grants",
            "equipment_loadouts",
        ] {
            assert!(SCHEMA.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")));
        }
    }
}
