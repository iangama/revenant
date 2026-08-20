# M11 — Replay

Status: complete and validated locally.

The authoritative activity flow now appends the minimum durable event stream needed to reconstruct a session: player join, activity start, regular enemy spawn and death, boss spawn and death, and activity completion. Every event contains an ordered database ID, type, PostgreSQL timestamp, `session_id`, optional `activity_id`, optional `actor_id`, and payload.

`revenant-replay` is a storage-independent domain crate. It validates that events belong to one session and reconstructs the player, activity, enemy counts, boss presence, completion state, and textual timeline. `revenant-persistence` remains the only SQL boundary, and protocol V1 is unchanged.

Use `revenant replay <session-id>` to reconstruct an exact session. `revenant replay --latest <account-id>` is provided for local diagnostics and the canonical smoke test. The smoke flow completes the Lua activity with bot and Godot, restarts the gateway without replacing the PostgreSQL volume, verifies M10 state, and reconstructs the latest bot session from persisted M11 events.

M11 intentionally does not include multiplayer, a visual Inspector, graphical replay, compatibility adapters, microservices, or M12 work.
