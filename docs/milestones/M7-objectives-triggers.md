# M7 — Objectives and Triggers

Status: complete and validated locally.

The generic objective runtime supports `KillActors` and `ReachArea` with `Pending`, `Active`, `Completed`, and `Failed` states. Killing the `relay_drones` group emits `ActorGroupDead`, completes `clear_drone_group`, and activates `reach_relay_door`. Reaching `relay_door` is implemented as a trigger transition but is not exercised until the activity flow expands.
