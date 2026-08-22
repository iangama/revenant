# M18 — Authoritative Loot and Inventory

Status: complete and validated locally.

M18 turns the persisted inventory introduced in M10 into a small playable system. Protocol V2 clients receive an authoritative inventory snapshot after world join. Completing `relay_awakening` grants each participating character one `relay_core_fragment`, persists the resulting quantity, emits a replay event, and updates the Godot inventory HUD before activity completion is presented.

The activity script declares the reward identifier and quantity, while `revenant-inventory` validates both against the server-owned catalog. PostgreSQL grants the reward and records activity history in one transaction. A `(session_id, character_id, item_id)` key makes retries idempotent without preventing rewards from later legitimate sessions.

The V1 wire contract and all files under `archive/clients/v1` remain frozen. Inventory messages are projected only to negotiated V2 connections; V1 clients retain their established compatibility and reconstruction flows.

M18 intentionally adds no world pickups, random drops, equipment effects, capacity, discard, trading, crafting, new activity, class, weapon, map, or level progression.
