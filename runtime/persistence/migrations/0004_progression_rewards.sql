CREATE TABLE IF NOT EXISTS progression_reward_grants (
    session_id TEXT NOT NULL,
    character_id TEXT NOT NULL REFERENCES characters(id),
    experience BIGINT NOT NULL CHECK (experience > 0),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (session_id, character_id)
);

ALTER TABLE progression_reward_grants
    ALTER COLUMN experience TYPE BIGINT;
