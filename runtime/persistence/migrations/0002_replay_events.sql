
CREATE TABLE IF NOT EXISTS replay_events (
    id BIGSERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    session_id TEXT NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    activity_id TEXT,
    actor_id BIGINT,
    payload TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS replay_events_session_order_idx
    ON replay_events(session_id, id);

CREATE INDEX IF NOT EXISTS replay_events_account_completion_idx
    ON replay_events(account_id, event_type, id DESC);
