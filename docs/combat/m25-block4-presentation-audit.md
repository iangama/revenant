# M25 Block 4 honest-presentation audit

Date: 2026-08-31  
Gate: 4 — honest, bounded combat presentation.

## Implemented semantics

- A valid local attack input emits a small amber attempt flash and the existing
  cooldown ring, reports `ATTACK SENT • AWAITING SERVER`, and uses the current
  authoritative equipment cooldown. It does not change actor health, play a
  hit cue, or claim damage.
- An input during the active local deadline reports `WEAPON COOLING` and emits
  at most one acknowledgement until the state leaves cooling.
- An input with no aimed active target emits a red unavailable cue, reports
  `ATTACK NOT SENT`, and sends no `AttackIntent`.
- A player hit remains a consequence of the server `DamageApplied` message and
  uses the cyan trail, confirmed weapon/impact audio, exact damage, and
  authoritative remaining enemy health.
- A hostile hit remains a consequence of the server `DamageApplied` message
  and uses the magenta exchange, enemy/impact/player-damage audio, exact damage,
  and authoritative remaining player health.
- Defeat presentation follows the server `ActorDestroy` message and adds an
  explicit defeat flash, audio cue, target state, and status message.
- Attempt, cooling, and target-unavailable sounds reuse the bounded original
  cooldown PCM asset at semantic pitch scales 1.30, 1.00, and 0.72. No new
  unreviewed audio asset or player voice was introduced.

The presentation does not predict acceptance, miss, damage, death, completion,
or reward. Protocol V2, replay vocabulary, persistence schema, activity script,
V1 artifacts, and `VERSION=0.2.0` remain unchanged.

## Deterministic fixture

The M25 Godot combat harness now covers both cadence and presentation. Its
presentation fixture creates an Operator, relay drone, and Warden and proves
exactly one delta for each of these semantic counters:

- local attempt;
- local cooldown;
- target unavailable;
- confirmed player hit;
- confirmed hostile hit;
- confirmed defeat.

The fixture also proves that active effects never exceed the fixed limit of 24
and that no permanent particle emitter is introduced. Reduced Flash changes
the flash scale from 1.00 to 0.55 and shortens transient flash lifetime. Audio
requests cover attempt, cooldown, unavailable target, rifle, impact, Warden,
player damage, and defeat while the existing fixed pools remain bounded to 12
voices. Silent mode suppresses all three local semantic requests and leaves
zero active voices.

The pre-snapshot 260 ms safety default, exact 150/250 ms authoritative
equipment deadlines, invalid-value rejection, and no deadline rewrite across
profile changes continue to pass in the same harness. The older M21
presentation fixture was changed to compare counter deltas so that the added
M25 semantic fixture cannot invalidate its independent bounded-state check.

## Reviewable A/B evidence

`scripts/capture-m25.sh` copies the Godot project to a temporary directory,
rebuilds its import cache there, runs the complete graphical validation, writes
the pair below, and verifies `SHA256SUMS`. It leaves no generated `.gd.uid` or
`.godot` state in the working tree.

| Capture | Meaning | Size | SHA-256 |
| --- | --- | ---: | --- |
| `01-local-attempt.png` | Amber local input and cooldown; 100/100 player HP and unchanged authoritative target health | 167,266 bytes | `9802cdaa9fa16d14969dd7ff7bc4894ef5f0d60f893f30c2a7c22c8c0114c15f` |
| `02-server-confirmed-hit.png` | Cyan authoritative rifle trail and red impact; 40 damage and enemy health 100/140 | 160,262 bytes | `1c6b14d5b8ca1d67b42dfee30c6dfcaee2d81a78f49535d19610f6777d754c89` |

Both PNGs are 1280×720 RGBA. Visual review confirmed that the states are
legible and distinct. This is creator-reviewable engineering evidence only;
the owner supplied no A/B preference, so no preference or population claim is
recorded.

The first isolated-capture revision copied `.import` sidecars but not their
generated `.godot/imported` targets. Godot therefore rejected empty audio
streams and the run was not accepted. The script now performs an isolated
editor import before graphical validation; the fresh run passed and produced
the canonical hashes above. An earlier direct-project import created one
untracked script UID, which was removed before the accepted run. Neither issue
was a gameplay or authority defect.

## Integrated runtime evidence

The uninterrupted canonical smoke passed after the presentation changes:

- two-participant driver/observer completion with exact bounded pressure;
- automatic and reused-account Godot activities;
- on-screen manual and keyboard-only complete activities;
- every M17-M25 slice marker, including the new honest-presentation marker;
- exact persistence reward, replay timeline, and Inspector summary;
- frozen V1 compatibility and standalone V1 reconstruction.

The smoke observed exactly one 10-damage drone hit and two 15-damage Warden
counterattacks in each activity, with zero lethal hostile hit. Presentation
therefore consumed real interleaved hostile confirmations without changing
completion or reward behavior.

The first smoke invocation stopped before gateway readiness because this WSL
shell had no Rust toolchain in `PATH`; it created no accepted gameplay
evidence. As already documented for M24, an official Rust 1.98.0 toolchain with
rustfmt and Clippy was provisioned under `/tmp` only. The next unchanged smoke
passed. No repository or user-profile toolchain state was added.

## Residuals entering Block 5

- Screenshots demonstrate reproducibility and semantic contrast, not timing
  authority or external-player comprehension.
- Perceptual audio quality on real playback hardware and accessibility beyond
  mute/Reduced Flash remain outside this creator-only gate.
- Range value, two active attackers, 0/75/150 ms simulated RTT, repetition,
  resource bounds, and final runtime reconciliation remain Block 5 evidence.
- Creator preference remains absent and must not be invented.

## Gate 4 decision

Gate 4 is **approved with bounded residuals assigned to Block 5**. Local intent
and unavailable/cooling feedback remain visibly and acoustically distinct from
server-confirmed player hit, hostile hit, and defeat; mute, Reduced Flash,
effect/audio pools, keyboard flow, authority, compatibility, and reproducible
A/B evidence are green.
