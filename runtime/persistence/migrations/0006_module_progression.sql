CREATE TABLE IF NOT EXISTS module_states (
    character_id TEXT PRIMARY KEY REFERENCES characters(id),
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0)
);

CREATE TABLE IF NOT EXISTS module_loadout_slots (
    character_id TEXT NOT NULL REFERENCES module_states(character_id),
    slot_index SMALLINT NOT NULL CHECK (slot_index >= 0 AND slot_index < 3),
    module_item_id TEXT NOT NULL,
    PRIMARY KEY (character_id, slot_index),
    UNIQUE (character_id, module_item_id),
    FOREIGN KEY (character_id, module_item_id)
        REFERENCES inventory(character_id, item_id)
);

CREATE TABLE IF NOT EXISTS module_operations (
    character_id TEXT NOT NULL REFERENCES module_states(character_id),
    operation_kind TEXT NOT NULL
        CHECK (operation_kind IN ('combine', 'loadout')),
    operation_id TEXT NOT NULL
        CHECK (operation_id ~ '^[A-Za-z0-9-]{1,32}$'),
    request_payload JSONB NOT NULL
        CHECK (jsonb_typeof(request_payload) = 'object')
        CHECK (octet_length(request_payload::text) <= 4096),
    result_payload JSONB NOT NULL
        CHECK (jsonb_typeof(result_payload) = 'object')
        CHECK (octet_length(result_payload::text) <= 8192),
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (character_id, operation_kind, operation_id)
);
