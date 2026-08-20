# M9 — Lua Activity Runtime

Status: complete and validated locally.

`relay_awakening` is defined externally in `scripts/activities/relay_awakening.lua`. The generic Rust runtime loads objective definitions and trigger transitions from a restricted Lua 5.4 VM. No compiled Rust code contains the activity's objective IDs, trigger subjects, door ID, boss archetype, or completion sequence.

The script path is configured through `REVENANT_ACTIVITY_SCRIPT`. Lua receives table and string libraries only; filesystem, operating-system, package, and debug libraries are unavailable.
