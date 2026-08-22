CREATE TABLE IF NOT EXISTS inventory_reward_grants (
    session_id TEXT NOT NULL,
    character_id TEXT NOT NULL REFERENCES characters(id),
    item_id TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (session_id, character_id, item_id)
);
