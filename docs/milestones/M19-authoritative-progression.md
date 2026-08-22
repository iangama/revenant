# M19 — Authoritative Progression

Status: complete and validated locally.

M19 activates the persisted progression boundary introduced in M10. Protocol V2 clients receive an authoritative level and experience snapshot after world join. Completing `relay_awakening` grants each participating character 100 experience in the same idempotent PostgreSQL transaction as M18 loot and activity history.

`revenant-progression` owns reward validation and the deterministic level formula: characters begin at level 1 and gain one level per 500 total experience. Lua declares the experience reward but cannot author the resulting level. Replay records the grant, and Godot presents the server-confirmed total and level.

The V1 contract and `archive/clients/v1` remain frozen. M19 intentionally adds no skills, attributes, level-based combat effects, equipment, crafting, new activity, class, weapon, or map.
