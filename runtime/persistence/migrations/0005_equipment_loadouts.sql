CREATE TABLE IF NOT EXISTS equipment_loadouts (
    character_id TEXT PRIMARY KEY REFERENCES characters(id),
    weapon_item_id TEXT NOT NULL,
    FOREIGN KEY (character_id, weapon_item_id)
        REFERENCES inventory(character_id, item_id)
);
