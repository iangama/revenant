# Relay Hub presentation enrichment audit

Date: 2026-09-07  
Decision: **green for the bounded visual pass**  
Contract: `docs/art/m28/relay-hub-enrichment-brief.md`

## Delivered result

The existing Relay Hub now has four explicit visual groups without adding a
new region or changing gameplay authority:

- a cyan/blue-petrol arrival platform grounds the Operator spawn;
- a cyan-to-amber floor lane connects arrival, cooperation anchor
  `[3,0,3]`, cooperation console `[4,0,3]`, and the Relay Core;
- a framed core threshold and three repeated service banks establish relay
  infrastructure; and
- three emissive veins plus bounded debris concentrate the damaged story in
  the existing corruption sector.

The gameplay camera is closer and targets the objective lane. The recording-
only `REVENANT_SHOWCASE_MODE=1` hides the input diagnostic panel and adds
readable pauses to the automated presentation flow. It changes neither manual
game speed nor the default playtest layout.

All additions are code-native, presentation-only, collision-free, and reuse
the existing graphite, blue-petrol, intact-cyan, objective-amber,
damaged-metal, and threat-magenta resources. The server still owns movement,
target arrival, objective progress, door state, enemy life, rewards, and every
M28 cooperation fact.

## Validation

The graphical semantic slice passed on Godot 4.7.1 GL Compatibility using Mesa
llvmpipe. Its representative combat peak measured:

| Metric | Measured | Contract |
| --- | ---: | ---: |
| Whole-scene meshes | 138 | at most 150 |
| Whole-scene material references | 26 | at most 32 |
| Environment material identities | 6 | at most 6 |
| Lights | 5 | exactly 5 |
| Shadow lights | 0 | exactly 0 |
| Permanent particles | 0 | exactly 0 |
| Audio nodes | 14 | exactly 14 |

The complete `make check` subsequently passed version consistency, Rust format,
Clippy with warnings denied, the full workspace test suite including real
PostgreSQL cooperation/replay coverage, all workspace builds, Inspector
TypeScript/build, secret audit, multiplayer smoke, repeated Godot activity,
manual controls, keyboard-only completion, presentation semantics, persistence,
replay, frozen V1 compatibility, and backend reconstruction.

## Review evidence

The creator-reviewable stills are retained outside the repository at:

`/mnt/c/Users/Ian/revenant-local-evidence/relay-hub-enrichment-20260907`

The still manifest SHA-256 is
`3c0682c226289b05462e9fbce5cd41fc21b9e659269bab7e51456034ca2b5350`.
Five selected frames were also extracted from the final AVI and visually
checked for initial combat, confirmed hits, door/transition, Warden combat, and
terminal reward readability.

The presentation recording is:

`/mnt/c/Users/Ian/Downloads/Revenant-Gameplay-Relay-Hub-Apresentacao-2026-09-07.avi`

It is a 34,724,812-byte AVI containing 383 Motion JPEG frames at 1280×720 and
30 FPS (12 seconds + 23 frames), with stereo 48 kHz 16-bit PCM audio. Its
SHA-256 is
`86b10c44da785eb4e650f0683fe412fc1e2a1235929192432800bd35402b6ab6`.

## Honest boundary

This pass makes the one current room more readable and intentional. It does
not turn the prototype into a broad world, add a second activity, establish
human comprehension or enjoyment, or complete M28 Gate 7. The authoritative
Godot cooperation console, role projection, ping, downed/revive presentation,
and live two-client creator evidence remain the active Block 7 work.
