# Relay Hub gameplay-depth presentation audit

Date: 2026-09-07  
Result: green within the accepted presentation-only boundary  
Contract: `docs/art/m28/relay-hub-gameplay-depth-brief.md`

## Delivered increment

The existing Relay Hub slice now communicates a small encounter arc instead
of reading as one undifferentiated test room:

- the Relay Drone has a larger, brighter chassis and four corruption signal
  pods, while remaining the existing `relay-drone` actor with unchanged
  health, AI, damage, cadence, and spawn order;
- the Warden has a larger containment silhouette, split base, shoulder guards,
  and an amber crown, while remaining the existing `warden` actor with
  unchanged authoritative behavior and combat values;
- an accepted world join presents `SECTOR 01 / ARRIVAL DECK`;
- authoritative `DoorState(open=true)` alone presents `SECTOR 02 / RELAY CORE`
  and moves the camera toward the existing core encounter;
- authoritative `LootGranted(relay_core_fragment)` first updates inventory and
  then animates three bounded fragment meshes from the last confirmed defeated
  enemy toward the authoritative player position; and
- accepted completion presents `SECTOR SECURED / RELAY HUB ONLINE`.

The banner uses text and a sector number in addition to color, never captures
the mouse, and removes translation plus shortens its fade under Reduced Flash.
The pickup rejects unsupported items, non-positive quantities, and an
overlapping duplicate. It cannot create inventory or predict a reward.

## Semantic and resource proof

The Godot presentation harness completed successfully after the final visual
changes and measured the representative peak at:

- 147 meshes out of 150;
- 26 material identities out of 32;
- five lights, all without shadows;
- zero particles; and
- 14 audio nodes.

The Relay Drone remains at 12 meshes and three material identities. The Warden
remains at 17 meshes and four material identities. The reward effect permits
one active pickup and three transient meshes. The harness started one confirmed
pickup, rejected its overlapping duplicate, and verified the Relay Core banner
in Reduced Flash mode with its fixed 560x96 bounds and input pass-through.

## Integrated application proof

The uninterrupted canonical `make check` completed green after the change. It
covered formatting and Clippy with warnings denied, every Rust unit and
PostgreSQL integration test, all-target builds, Inspector TypeScript checking
and production build, secret audit, gateway/database smoke flows, Godot real
wire/manual/keyboard/semantic flows, persistence and replay inspection, and
frozen plus reconstructed V1 compatibility.

The deployed local applications were also checked before the final recording:
gateway, PostgreSQL, and Inspector were all healthy. A fresh local identity
then completed the real gateway flow in Godot: join, Drone defeat, authoritative
weapon change, movement to the door, Warden defeat, `LootGranted`,
`ProgressionGranted`, and `ActivityComplete`.

## Review evidence

Creator-review evidence is retained under
`/mnt/c/Users/Ian/revenant-local-evidence/relay-hub-gameplay-depth-20260907`.
Its live review movie is 1280x720 at 30 FPS and has SHA-256
`a9c0a27d32e68a84bfd602b46717864017b7245a720fb2814075f1631cd83622`.
Extracted review frames cover arrival, the damaged Drone silhouette, traversal,
the Relay Core transition, the Warden, confirmed fragment collection, and the
secured end state.

The owner-facing final movie is:

`/mnt/c/Users/Ian/Downloads/Revenant-Gameplay-Inimigos-Areas-Coleta-2026-09-07.avi`

It contains 381 Motion JPEG frames at 1280x720 and 30 FPS (12 seconds and 21
frames), is 35,052,010 bytes, and has SHA-256
`b08ba9bd8b0cccb5c3a3b61d1b4c83461f2115fd8136c568d5ef2eace2e0263a`.

## Honest boundary and next content tranche

`ARRIVAL DECK` and `RELAY CORE` are two staged sectors inside the same existing
approximately 24x24 Relay Hub. This pass does not claim a second map, world, or
activity. It adds no enemy wire archetype, AI rule, random/proximity loot,
pickup intent, persistence event, replay event, protocol change, balance
change, version change, or M31 content completion.

Actual new enemy variants, a new traversable area, and additional collectible
types remain M31 work after Gates M28, M29, and M30. The presentation hooks in
this pass are bounded preparation; active M28 Block 7 cooperation presentation
remains the current milestone work.
