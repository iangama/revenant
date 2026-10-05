CREATE TABLE IF NOT EXISTS route_operations (
    session_id TEXT PRIMARY KEY
        CHECK (session_id ~ '^[A-Za-z0-9:_-]{1,64}$'),
    activity_id TEXT NOT NULL
        CHECK (activity_id ~ '^[a-z0-9_]{1,32}$'),
    operation_id TEXT NOT NULL
        CHECK (operation_id ~ '^[A-Za-z0-9-]{1,32}$'),
    catalog_revision TEXT NOT NULL CHECK (catalog_revision = 'm27-v1'),
    resolver_revision TEXT NOT NULL
        CHECK (resolver_revision = 'm27-permute63-v1'),
    leader_account_id TEXT NOT NULL REFERENCES accounts(id),
    participant_count SMALLINT NOT NULL
        CHECK (participant_count >= 1 AND participant_count <= 2),
    route_id TEXT NOT NULL CHECK (route_id IN ('breach', 'stabilize')),
    seed BIGINT NOT NULL CHECK (seed >= 0),
    event_id TEXT NOT NULL CHECK (
        event_id IN (
            'overcharged_armor',
            'arc_surge',
            'shielded_channel',
            'residual_feedback'
        )
    ),
    warden_health_basis_points INTEGER NOT NULL
        CHECK (warden_health_basis_points >= 10000
            AND warden_health_basis_points <= 12000),
    warden_counter_damage INTEGER NOT NULL
        CHECK (warden_counter_damage >= 10 AND warden_counter_damage <= 20),
    objective_path JSONB NOT NULL
        CHECK (jsonb_typeof(objective_path) = 'array')
        CHECK (jsonb_array_length(objective_path) >= 3
            AND jsonb_array_length(objective_path) <= 4)
        CHECK (octet_length(objective_path::text) <= 1024),
    duration_budget_ms BIGINT NOT NULL CHECK (duration_budget_ms = 90000),
    reward_item_id TEXT NOT NULL CHECK (reward_item_id = 'relay_core_fragment'),
    reward_quantity INTEGER NOT NULL CHECK (reward_quantity IN (1, 2)),
    experience BIGINT NOT NULL CHECK (experience IN (100, 150)),
    terminal_outcome TEXT
        CHECK (terminal_outcome IN ('succeeded', 'failed_timeout')),
    elapsed_ms BIGINT CHECK (elapsed_ms >= 0),
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    terminal_at TIMESTAMPTZ,
    CHECK (
        (route_id = 'breach'
            AND event_id IN ('overcharged_armor', 'arc_surge'))
        OR
        (route_id = 'stabilize'
            AND event_id IN ('shielded_channel', 'residual_feedback'))
    ),
    CHECK (
        (event_id = 'overcharged_armor'
            AND warden_health_basis_points = 12000
            AND warden_counter_damage = 15)
        OR
        (event_id = 'arc_surge'
            AND warden_health_basis_points = 10000
            AND warden_counter_damage = 20)
        OR
        (event_id = 'shielded_channel'
            AND warden_health_basis_points = 10000
            AND warden_counter_damage = 10)
        OR
        (event_id = 'residual_feedback'
            AND warden_health_basis_points = 11000
            AND warden_counter_damage = 15)
    ),
    CHECK (
        (route_id = 'breach' AND reward_quantity = 2 AND experience = 100)
        OR
        (route_id = 'stabilize' AND reward_quantity = 1 AND experience = 150)
    ),
    CHECK (
        (route_id = 'breach'
            AND objective_path =
                '["clear_drone_group", "reach_relay_door", "defeat_warden"]'::JSONB)
        OR
        (route_id = 'stabilize'
            AND objective_path =
                '["clear_drone_group", "reach_relay_stabilizer", "reach_relay_door", "defeat_warden"]'::JSONB)
    ),
    CHECK (
        (terminal_outcome IS NULL AND elapsed_ms IS NULL AND terminal_at IS NULL)
        OR
        (terminal_outcome IS NOT NULL AND elapsed_ms IS NOT NULL AND terminal_at IS NOT NULL)
    ),
    CHECK (
        terminal_outcome IS NULL
        OR (terminal_outcome = 'succeeded' AND elapsed_ms <= duration_budget_ms)
        OR (terminal_outcome = 'failed_timeout' AND elapsed_ms > duration_budget_ms)
    )
);

CREATE INDEX IF NOT EXISTS route_operations_leader_idx
    ON route_operations(leader_account_id, accepted_at DESC);

CREATE TABLE IF NOT EXISTS route_operation_participants (
    session_id TEXT NOT NULL REFERENCES route_operations(session_id),
    participant_index SMALLINT NOT NULL
        CHECK (participant_index >= 0 AND participant_index < 2),
    account_id TEXT NOT NULL REFERENCES accounts(id),
    character_id TEXT NOT NULL REFERENCES characters(id),
    actor_id BIGINT NOT NULL CHECK (actor_id >= 0),
    PRIMARY KEY (session_id, participant_index),
    UNIQUE (session_id, account_id),
    UNIQUE (session_id, character_id),
    UNIQUE (session_id, actor_id)
);
