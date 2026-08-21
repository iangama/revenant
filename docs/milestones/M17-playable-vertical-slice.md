# M17 — Playable Vertical Slice

Status: complete and validated locally.

M17 makes the existing `relay_awakening` activity manually playable without introducing a new gameplay system. A normal Godot launch presents a top-down relay arena, camera, player/enemy meshes, sealed core door, cursor-following crosshair, player/enemy HP, objective progress, position/door distance, controls, encounter guidance, and a compact input monitor.

Controls:

- WASD or arrow keys submit bounded `MoveIntent` steps;
- an on-screen directional pad provides the same movement path when desktop focus forwarding is unavailable;
- cursor proximity selects the active enemy;
- left mouse, Space, or the on-screen Attack button submits `AttackIntent`;
- reaching `x=6` after the drone dies opens the relay core.

Godot renders and presents feedback but does not author actor position, HP, damage, objectives, door state, boss lifecycle, or completion. The server validates movement within `[-12,12]` on the ground plane and broadcasts the accepted position. Existing combat range, cooldown, damage, and activity logic remain authoritative.

The client encodes both positive and negative relay-hub coordinates and refuses to transmit a partially encoded frame. Successful movement and damage remain visible only after the authoritative server response. Session admission is confirmed by the shared-session coordinator before `WorldJoinResponse` reports success; a late client therefore receives an actionable rejection instead of waiting indefinitely for `ActivityStart`. After all participants disconnect, the gateway reloads the activity and creates a fresh session without restarting the process or database.

The original automated Godot path remains active only with `REVENANT_EXIT_AFTER_FLOW=1`, preserving every earlier acceptance test. `REVENANT_VALIDATE_MANUAL_FLOW=1` drives the normal manual loop through the on-screen Attack and direction controls, killing the drone, moving incrementally to the door, defeating the Warden, and completing the activity without desktop input forwarding. `REVENANT_VALIDATE_SLICE=1` performs a headless composition and signed-coordinate encoding check for the camera, HUD, door, movement actions, aiming, and attack action. The canonical smoke runs all three modes, completes consecutive activities through one gateway process to verify session reuse, and then continues through persistence, replay, V1 compatibility, and reconstruction.

M17 intentionally adds no loot, inventory UI, new activity, class, weapon, map, graphical replay, or M18 work.
