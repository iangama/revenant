# Relay Hub final-video presentation audit

Date: 2026-09-07  
Result: green within the final-video presentation boundary  
Contract: `docs/art/m28/relay-hub-final-video-brief.md`

## Delivered result

The existing Relay Hub now has a clearer visual and gameplay rhythm:

- brighter graphite, blue-petrol, and damaged-metal surfaces plus higher ambient
  exposure make actors and floor routes readable without shadows;
- five phase-responsive existing lights distinguish Arrival, Breach, Core, and
  Secured states without changing world or objective truth;
- the Breach Threshold adds approach markers, while the Relay Core Chamber adds
  a dais, reactor pylons, a pulsing core, and two counter-rotating rings behind
  the existing Warden encounter;
- the Drone has moving arms and independently bobbing signal pods; the Warden
  has magenta containment plates, an amber halo/crown, and distinct confirmed
  attack/hit motion, with no AI or combat-value change;
- the recording camera follows the accepted encounter phase, keeps the Warden
  out from behind the opened door, and places the crosshair on the active target
  only in the deterministic creator showcase;
- recording-only diagnostic controls are hidden while telemetry, mission,
  inventory, health, weapon, and server-confirmation text remain visible; and
- accepted `LootGranted` reveals three fragments for 2.03 seconds, labels the
  reward in world space, and carries it from the last confirmed Warden position
  to the authoritative Operator position before the delayed Secured banner.

`ARRIVAL DECK`, `BREACH CORRIDOR`, and `RELAY CORE` remain named subspaces of
one existing approximately 24x24 activity room. No new map, activity, enemy,
item, drop, collision, protocol, persistence, replay, or balance content was
created.

## Validation

The Godot semantic harness passed after both visual review iterations. Its
representative combat peak is:

- 163 meshes out of 170;
- 26 material identities out of 32;
- five lights, all with shadows disabled;
- zero particles; and
- 14 audio nodes.

The Warden is exactly 18 meshes and remains inside four material identities;
the Drone remains inside its prior 12-mesh/three-material cap. The harness also
proved the four restorable lighting phases, rejection of an unknown phase, the
bounded reward, the relocated non-color sector banner, and Reduced Flash/input
pass-through behavior.

The uninterrupted canonical `make check` passed after the final implementation.
It included version consistency, Rust formatting, Clippy with warnings denied,
all workspace unit and PostgreSQL integration tests, all-target builds,
Inspector TypeScript/build, secret audit, smoke flows, real Godot manual and
keyboard flows, persistence/replay inspection, frozen V1 compatibility, and V1
reconstruction.

## Live capture and reconciliation

The accepted take used the source-built gateway on loopback and a fresh local
identity. Its live log proves join, Drone chase and defeat, authoritative weapon
change, six accepted movement steps, open-door Warden spawn, two bounded Warden
counterattacks, Warden defeat, exactly one fragment grant, exactly one 100 XP
grant, and terminal completion.

Inspector session `session-1788827815977407773` independently reports one
participant, one enemy spawn, two enemy defeats, one boss spawn, one equipment
change, one loot grant, one progression grant, 11 replay events, and completed
truth. The source capture gateway, deployed loopback gateway, Inspector, and
PostgreSQL were all healthy when the evidence was sealed.

The accepted movie contains 649 Motion JPEG frames at 1280x720 and 30 FPS
(21.63 seconds), with PCM stereo audio at 48 kHz. Godot's software-rendered
capture log measured 15.04 ms average CPU render time and 14.31 ms average GPU
render time per frame; encoding overhead is recorded separately and is not
claimed as ordinary gameplay performance.

Evidence is retained at
`/mnt/c/Users/Ian/revenant-local-evidence/relay-hub-final-video-20260907b`.
Its manifest SHA-256 is
`ddc332d57e8a9915f7d0a0feacf7b92a3ed0fcdf6bda03e3c3b5e8ccfd80999c`.
Representative reviewed frames cover Arrival, late Drone combat, traversal,
Core transition, Warden combat, named collection, and Secured completion.

The owner-facing movie is:

`/mnt/c/Users/Ian/Downloads/Revenant-Gameplay-Final-Relay-Hub-2026-09-07.avi`

It is 63,995,040 bytes and has SHA-256
`562d388ef66e4e3770b465d2644d5cb07126db5108274c92f72c1e4ff9c4897f`.

## Continuation

This interlude is complete. M30 Gate 1 remains paused rather than green and is
the next roadmap continuation point if the owner resumes development. Actual
new activities, areas, enemy/item catalog entries, and broader world content
remain M31 work behind a future explicit finite gate.
