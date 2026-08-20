# M8 — First Activity

Status: complete and validated locally.

The hardcoded `relay_awakening` activity now runs end to end: enter the world, clear the drone, activate `ReachArea`, move to `relay_door`, open `relay_core`, spawn the 120 HP `warden`, complete the boss objective, and emit `ActivityComplete`. The flow is compiled Rust by design; conversion to Lua belongs to M9.
