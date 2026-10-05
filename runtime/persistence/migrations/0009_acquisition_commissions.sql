CREATE TABLE IF NOT EXISTS acquisition_milestones (
    character_id TEXT NOT NULL REFERENCES characters(id),
    milestone TEXT NOT NULL CHECK (milestone IN (
        'meridian_recovered', 'breach_completed', 'stabilize_completed', 'prism_completed'
    )),
    proof_session_id TEXT NOT NULL,
    proof_event_id BIGINT NOT NULL REFERENCES replay_events(id),
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (character_id, milestone)
);

CREATE TABLE IF NOT EXISTS acquisition_claims (
    character_id TEXT NOT NULL REFERENCES characters(id),
    arc_id TEXT NOT NULL CHECK (arc_id IN ('meridian', 'routes', 'prism')),
    replay_event_id BIGINT NOT NULL UNIQUE REFERENCES replay_events(id),
    quantity INTEGER NOT NULL CHECK (quantity = 2),
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (character_id, arc_id)
);
