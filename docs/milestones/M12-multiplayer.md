# M12 — Multiplayer

Status: complete and validated locally.

The gateway now accepts game connections concurrently and routes post-join intents to one server-authoritative shared-session coordinator. The coordinator owns the common actor registry, Lua activity, combat runtime, objective transitions, encounter stage, persistence, and replay session. Clients receive state through independent outbound channels, so both players observe the same player actors, enemies, movement, damage, deaths, objectives, boss, and activity completion.

`REVENANT_EXPECTED_PLAYERS` sets the join barrier and defaults to one for local compatibility. The M12 smoke starts the gateway with two expected players, connects an observer bot and a driver bot under distinct local identities, completes `relay_awakening`, and requires both clients to observe the shared completion. It then restarts in single-player mode to retain the complete Godot validation, restarts again, verifies both PostgreSQL completion history and the shared event replay, and never replaces the database volume.

Protocol V1 is unchanged: M12 defines broadcast and shared-state semantics for existing messages rather than introducing unnecessary multiplayer-specific packets. M12 intentionally does not implement matchmaking, parties, PvP, a visual Inspector, compatibility adapters, or M13 work.
