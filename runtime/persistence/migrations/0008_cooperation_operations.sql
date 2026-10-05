CREATE TABLE IF NOT EXISTS cooperation_operations (
    session_id TEXT PRIMARY KEY
        CHECK (session_id ~ '^[A-Za-z0-9:_-]{1,64}$'),
    activity_id TEXT NOT NULL CHECK (activity_id = 'relay_awakening'),
    start_operation_id TEXT NOT NULL
        CHECK (start_operation_id ~ '^[A-Za-z0-9-]{1,32}$'),
    catalog_revision TEXT NOT NULL CHECK (catalog_revision = 'm28-v1'),
    anchor_account_id TEXT NOT NULL REFERENCES accounts(id),
    participant_count SMALLINT NOT NULL CHECK (participant_count = 2),
    phase TEXT NOT NULL CHECK (phase IN (
        'awaiting_anchor', 'awaiting_ping', 'awaiting_runner',
        'runner_downed', 'revive_channel', 'encounter_active'
    )),
    operation_duration_ms BIGINT NOT NULL CHECK (operation_duration_ms = 60000),
    ping_ttl_ms BIGINT NOT NULL CHECK (ping_ttl_ms = 5000),
    revive_window_ms BIGINT NOT NULL CHECK (revive_window_ms = 15000),
    revive_channel_ms BIGINT NOT NULL CHECK (revive_channel_ms = 2000),
    revive_health INTEGER NOT NULL CHECK (revive_health = 50),
    max_revive_distance_squared BIGINT NOT NULL
        CHECK (max_revive_distance_squared = 4),
    reward_item_id TEXT NOT NULL CHECK (reward_item_id = 'relay_core_fragment'),
    reward_quantity INTEGER NOT NULL CHECK (reward_quantity = 2),
    reward_experience BIGINT NOT NULL CHECK (reward_experience = 125),
    anchor_arrived BOOLEAN NOT NULL DEFAULT FALSE,
    anchor_elapsed_ms BIGINT CHECK (anchor_elapsed_ms >= 0),
    pinged BOOLEAN NOT NULL DEFAULT FALSE,
    ping_operation_id TEXT
        CHECK (ping_operation_id ~ '^[A-Za-z0-9-]{1,32}$'),
    ping_target TEXT CHECK (ping_target = 'relay_console'),
    ping_elapsed_ms BIGINT CHECK (ping_elapsed_ms >= 0),
    runner_arrived BOOLEAN NOT NULL DEFAULT FALSE,
    runner_elapsed_ms BIGINT CHECK (runner_elapsed_ms >= 0),
    downed_elapsed_ms BIGINT CHECK (downed_elapsed_ms >= 0),
    revive_operation_id TEXT
        CHECK (revive_operation_id ~ '^[A-Za-z0-9-]{1,32}$'),
    revive_target TEXT CHECK (revive_target = 'runner'),
    revive_started_elapsed_ms BIGINT CHECK (revive_started_elapsed_ms >= 0),
    revived BOOLEAN NOT NULL DEFAULT FALSE,
    revive_completed_elapsed_ms BIGINT CHECK (revive_completed_elapsed_ms >= 0),
    revive_count SMALLINT NOT NULL DEFAULT 0 CHECK (revive_count IN (0, 1)),
    warden_completed BOOLEAN NOT NULL DEFAULT FALSE,
    warden_elapsed_ms BIGINT CHECK (warden_elapsed_ms >= 0),
    terminal_outcome TEXT CHECK (terminal_outcome IN (
        'succeeded', 'failed_ping_timeout', 'failed_revive_timeout',
        'failed_operation_timeout', 'failed_participant_defeated',
        'abandoned_disconnect'
    )),
    terminal_elapsed_ms BIGINT CHECK (terminal_elapsed_ms >= 0),
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    terminal_at TIMESTAMPTZ,
    CHECK (anchor_arrived = (anchor_elapsed_ms IS NOT NULL)),
    CHECK (pinged = (ping_operation_id IS NOT NULL)
        AND pinged = (ping_target IS NOT NULL)
        AND pinged = (ping_elapsed_ms IS NOT NULL)),
    CHECK (runner_arrived = (runner_elapsed_ms IS NOT NULL)
        AND runner_arrived = (downed_elapsed_ms IS NOT NULL)),
    CHECK ((revive_operation_id IS NULL) = (revive_target IS NULL)
        AND (revive_operation_id IS NULL) = (revive_started_elapsed_ms IS NULL)),
    CHECK (revived = (revive_completed_elapsed_ms IS NOT NULL)
        AND revived = (revive_count = 1)),
    CHECK (warden_completed = (warden_elapsed_ms IS NOT NULL)),
    CHECK ((terminal_outcome IS NULL) = (terminal_elapsed_ms IS NULL)
        AND (terminal_outcome IS NULL) = (terminal_at IS NULL)),
    CHECK (
        (phase = 'awaiting_anchor'
            AND NOT anchor_arrived AND NOT pinged AND NOT runner_arrived
            AND revive_operation_id IS NULL AND NOT revived AND NOT warden_completed)
        OR
        (phase = 'awaiting_ping'
            AND anchor_arrived AND NOT pinged AND NOT runner_arrived
            AND revive_operation_id IS NULL AND NOT revived AND NOT warden_completed)
        OR
        (phase = 'awaiting_runner'
            AND anchor_arrived AND pinged AND NOT runner_arrived
            AND revive_operation_id IS NULL AND NOT revived AND NOT warden_completed)
        OR
        (phase = 'runner_downed'
            AND anchor_arrived AND pinged AND runner_arrived
            AND revive_operation_id IS NULL AND NOT revived AND NOT warden_completed)
        OR
        (phase = 'revive_channel'
            AND anchor_arrived AND pinged AND runner_arrived
            AND revive_operation_id IS NOT NULL AND NOT revived AND NOT warden_completed)
        OR
        (phase = 'encounter_active'
            AND anchor_arrived AND pinged AND runner_arrived
            AND revive_operation_id IS NOT NULL AND revived)
    ),
    CHECK (NOT pinged OR ping_elapsed_ms >= anchor_elapsed_ms),
    CHECK (NOT runner_arrived OR (
        runner_elapsed_ms >= ping_elapsed_ms
        AND runner_elapsed_ms <= ping_elapsed_ms + ping_ttl_ms
        AND downed_elapsed_ms = runner_elapsed_ms
    )),
    CHECK (revive_operation_id IS NULL OR (
        revive_started_elapsed_ms >= downed_elapsed_ms
        AND revive_started_elapsed_ms <= downed_elapsed_ms + revive_window_ms
    )),
    CHECK (NOT revived OR (
        revive_completed_elapsed_ms >= revive_started_elapsed_ms + revive_channel_ms
        AND revive_completed_elapsed_ms <= downed_elapsed_ms + revive_window_ms
    )),
    CHECK (NOT warden_completed OR (
        revived AND warden_elapsed_ms >= revive_completed_elapsed_ms
        AND warden_elapsed_ms <= operation_duration_ms
    )),
    CHECK (
        terminal_outcome IS NULL
        OR (terminal_outcome = 'succeeded'
            AND phase = 'encounter_active' AND warden_completed
            AND terminal_elapsed_ms = warden_elapsed_ms)
        OR (terminal_outcome = 'failed_ping_timeout'
            AND phase = 'awaiting_runner'
            AND terminal_elapsed_ms > ping_elapsed_ms + ping_ttl_ms)
        OR (terminal_outcome = 'failed_revive_timeout'
            AND phase IN ('runner_downed', 'revive_channel')
            AND terminal_elapsed_ms > downed_elapsed_ms + revive_window_ms)
        OR (terminal_outcome = 'failed_operation_timeout'
            AND terminal_elapsed_ms > operation_duration_ms)
        OR (terminal_outcome = 'failed_participant_defeated'
            AND phase = 'encounter_active'
            AND terminal_elapsed_ms <= operation_duration_ms)
        OR (terminal_outcome = 'abandoned_disconnect'
            AND terminal_elapsed_ms <= operation_duration_ms
            AND (phase <> 'awaiting_runner'
                OR terminal_elapsed_ms <= ping_elapsed_ms + ping_ttl_ms)
            AND (phase NOT IN ('runner_downed', 'revive_channel')
                OR terminal_elapsed_ms <= downed_elapsed_ms + revive_window_ms))
    )
);

CREATE INDEX IF NOT EXISTS cooperation_operations_anchor_idx
    ON cooperation_operations(anchor_account_id, accepted_at DESC);

CREATE TABLE IF NOT EXISTS cooperation_operation_participants (
    session_id TEXT NOT NULL REFERENCES cooperation_operations(session_id),
    participant_index SMALLINT NOT NULL CHECK (participant_index IN (0, 1)),
    role TEXT NOT NULL CHECK (
        (participant_index = 0 AND role = 'anchor')
        OR (participant_index = 1 AND role = 'runner')
    ),
    account_id TEXT NOT NULL REFERENCES accounts(id),
    character_id TEXT NOT NULL REFERENCES characters(id),
    actor_id BIGINT NOT NULL CHECK (actor_id >= 0),
    weapon_item_id TEXT NOT NULL CHECK (weapon_item_id IN ('pulse_rifle', 'arc_sidearm')),
    module_loadout JSONB NOT NULL
        CHECK (jsonb_typeof(module_loadout) = 'array')
        CHECK (jsonb_array_length(module_loadout) <= 3)
        CHECK (octet_length(module_loadout::TEXT) <= 512),
    damage INTEGER NOT NULL CHECK (damage >= 18 AND damage <= 52),
    range INTEGER NOT NULL CHECK (range >= 4 AND range <= 10),
    cooldown_ms BIGINT NOT NULL CHECK (cooldown_ms >= 120 AND cooldown_ms <= 350),
    max_health INTEGER NOT NULL CHECK (max_health >= 100 AND max_health <= 130),
    admitted_health INTEGER NOT NULL
        CHECK (admitted_health > 0 AND admitted_health <= max_health),
    current_health INTEGER NOT NULL
        CHECK (current_health >= 0 AND current_health <= max_health),
    life_state TEXT NOT NULL CHECK (life_state IN ('active', 'downed', 'defeated')),
    PRIMARY KEY (session_id, participant_index),
    UNIQUE (session_id, account_id),
    UNIQUE (session_id, character_id),
    UNIQUE (session_id, actor_id),
    CHECK ((life_state = 'active' AND current_health > 0)
        OR (life_state IN ('downed', 'defeated') AND current_health = 0))
);

DO $migration$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'cooperation_operations_temporal_contract'
          AND conrelid = 'cooperation_operations'::REGCLASS
    ) THEN
        ALTER TABLE cooperation_operations
            ADD CONSTRAINT cooperation_operations_temporal_contract CHECK (
                (anchor_elapsed_ms IS NULL
                    OR anchor_elapsed_ms <= operation_duration_ms)
                AND (ping_elapsed_ms IS NULL
                    OR ping_elapsed_ms <= operation_duration_ms)
                AND (runner_elapsed_ms IS NULL
                    OR runner_elapsed_ms <= operation_duration_ms)
                AND (revive_started_elapsed_ms IS NULL
                    OR revive_started_elapsed_ms <= operation_duration_ms)
                AND (revive_completed_elapsed_ms IS NULL
                    OR revive_completed_elapsed_ms <= operation_duration_ms)
                AND warden_completed = COALESCE(
                    terminal_outcome = 'succeeded', FALSE
                )
                AND (
                    terminal_elapsed_ms IS NULL
                    OR (
                        (anchor_elapsed_ms IS NULL
                            OR terminal_elapsed_ms >= anchor_elapsed_ms)
                        AND (ping_elapsed_ms IS NULL
                            OR terminal_elapsed_ms >= ping_elapsed_ms)
                        AND (runner_elapsed_ms IS NULL
                            OR terminal_elapsed_ms >= runner_elapsed_ms)
                        AND (revive_started_elapsed_ms IS NULL
                            OR terminal_elapsed_ms >= revive_started_elapsed_ms)
                        AND (revive_completed_elapsed_ms IS NULL
                            OR terminal_elapsed_ms >= revive_completed_elapsed_ms)
                    )
                )
                AND (
                    terminal_outcome IS DISTINCT FROM 'failed_operation_timeout'
                    OR (
                        (phase <> 'awaiting_runner'
                            OR terminal_elapsed_ms <= ping_elapsed_ms + ping_ttl_ms)
                        AND (phase NOT IN ('runner_downed', 'revive_channel')
                            OR terminal_elapsed_ms <= downed_elapsed_ms + revive_window_ms)
                    )
                )
            );
    END IF;
END
$migration$;
