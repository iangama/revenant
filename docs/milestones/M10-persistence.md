# M10 — Persistence

PostgreSQL now preserves local accounts, characters, inventory, progression, and activity completion history. `revenant-persistence` owns the idempotent schema and typed database operations; domain crates remain independent of SQL.

Authentication ensures one initial Operator and its `pulse_rifle` in a transaction. Character selection is loaded from PostgreSQL, world join loads inventory and progression, and completing `relay_awakening` appends an activity-history record.

The canonical smoke flow runs the full bot and optional Godot clients, stops the gateway, starts a new gateway process against the same database, and verifies the stored character state and activity completion with the diagnostic `persistence-check` binary.

M10 intentionally does not implement event sourcing or replay; those belong to M11.
