CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS characters (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    display_name TEXT NOT NULL,
    class_name TEXT NOT NULL,
    level INTEGER NOT NULL CHECK (level > 0)
);

CREATE INDEX IF NOT EXISTS characters_account_id_idx ON characters(account_id);

CREATE TABLE IF NOT EXISTS inventory (
    character_id TEXT NOT NULL REFERENCES characters(id),
    item_id TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity >= 0),
    PRIMARY KEY (character_id, item_id)
);

CREATE TABLE IF NOT EXISTS progression (
    character_id TEXT PRIMARY KEY REFERENCES characters(id),
    level INTEGER NOT NULL CHECK (level > 0),
    experience BIGINT NOT NULL CHECK (experience >= 0)
);

CREATE TABLE IF NOT EXISTS activity_history (
    id BIGSERIAL PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    character_id TEXT NOT NULL REFERENCES characters(id),
    activity_id TEXT NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS activity_history_account_id_idx
    ON activity_history(account_id, activity_id);
