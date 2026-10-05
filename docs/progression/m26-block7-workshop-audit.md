# M26 Block 7 honest Godot workshop audit

Date: 2026-09-01  
Gate: 7 — honest Godot module presentation.

## Implemented surface and truth boundary

- The authored 1280×720 HUD exposes `MODULES [M]` and a bounded overlay with
  the four server-supplied catalog entries, exact fragment/ownership/loadout
  state, revision, three-slot limit, current effective weapon profiles, and
  maximum health.
- Candidate selection remains request input only. `PREVIEW` displays only a
  decoded `ModulePreview` and labels it `SERVER PREVIEW • NOT ACTIVE`.
  Combination and whole-loadout mutation remain disabled until the
  authoritative activity is complete.
- Pending requests change no inventory, ownership, loadout, revision, weapon
  profile, or actor health. Accepted/replayed/rejected/connection-lost results
  remain distinct, and accepted mutation states disclose both `APPLIES NEXT
  SESSION` and `CURRENT ACTOR UNCHANGED`.
- The normal post-completion message pump remains active for workshop traffic;
  the validation driver does not own a second socket reader. A new connection
  clears workshop state, candidate selection, results, and visibility.
- `M`, Escape, mouse, Tab/forward focus, and button activation coexist without
  leaking movement or attack input through the overlay. Deselecting a module
  also clears or retargets the singular combination candidate, including after
  a rejected fourth selection.
- Mutation operation identifiers use 64 bits of local OS entropy encoded as a
  24-character ASCII alphanumeric/hyphen token. The harness proves bounded
  grammar and non-repetition within a process; the server remains the authority
  for idempotency and conflict detection.
- Mute and Reduced Flash reuse the existing fixed audio/effect pools. The
  workshop adds no dynamic audiovisual resource and no local success cue.

No module arithmetic is implemented in Godot. The accepted state, preview,
fragment quantity, catalog modifiers, effective profiles, and maximum health
are formatted from server messages. Block 7 changed no Rust Protocol V2 type,
catalog/domain rule, schema, migration, replay vocabulary, gateway lifecycle,
frozen V1 artifact, package, or product version.

## Real-wire defect found and corrected

The first graphical current-gateway attempt completed the authoritative
activity but decoded `ModuleSnapshot` as an empty message. The existing Godot
MessagePack subset did not decode signed integer markers used by the accepted
Tempo/Reach modifiers and did not encode string arrays required by preview and
loadout requests.

The client codec now supports negative fixints and signed 8/16/32/64-bit
integers, bounded string-array values, booleans, and nested request values while
preserving the exact older movement bytes. The boundary harness adds an exact
`[-1500, -1000, -12]` signed fixture and a two-module preview-request round
trip. This is implementation support for the already accepted Protocol V2
wire contract, not a protocol expansion.

The failed attempt reserved no module operation and changed no loadout. Its
activity had already completed atomically, so its one fragment and 100 XP are
valid persisted rewards rather than partial state.

## Deterministic semantic evidence

The isolated Godot 4.7.1 import and full M17–M26 semantic run pass without a
script error. The M26 fixture proves:

- empty/non-empty snapshots and exact current/preview server values;
- preview of unowned candidates without mutation;
- neutral pending projection and unchanged admitted combat fields;
- accepted, replayed, eight named rejection classes, timeout/connection loss,
  and reconnect reset;
- active mutation lock and complete-state availability;
- next-session/current-actor disclosure;
- actual forward-focus reachability for all four module choices and Preview,
  Combine, Apply, and Close;
- 1280×720 containment, mute/Reduced Flash truth equivalence, bounded operation
  identifiers, and no stale reconnect selection.

The same run retains every prior display, presentation, combat, observation,
transport, keyboard, and authority marker through M25.

## Accepted real gateway evidence

The final accepted graphical run used the current compiled gateway only on
`127.0.0.1:17026` and `127.0.0.1:18026`, PostgreSQL on loopback, a fresh
isolated Godot import/data directory, and fixture character
`local:m26g6-221024-60274:operator`.

Accepted session: `session-1788260431884308379`.

- Admission applied persisted Force+Ward: rifle `48 / range 6 / 325 ms`,
  sidearm `30 / range 8 / 195 ms`, and 120 maximum health.
- The completed encounter granted exactly one fragment and 100 XP. The player
  ended at 80/120 HP; the post-completion mutation did not heal or rewrite that
  admitted actor.
- The server previewed Force-only as rifle `48 / 6 / 300 ms`, sidearm
  `30 / 8 / 180 ms`, and 100 maximum health. Godot displayed those values as
  not active.
- Operation `godot-l-4a94087ad6098177` accepted exactly once with revision
  `3 → 4`, persisted loadout Force+Ward → Force, and unchanged ownership.
- The final database state is 8 fragments, revision 4, owned Force+Ward, and
  equipped Force. The session contains exactly 12 append-ordered events, one
  completion, one loot grant, one progression grant, one module snapshot, and
  one loadout change.
- Current CLI reconstruction passed. The current isolated Inspector reported
  45 ms join-to-start, 4,236 ms activity duration, one participant, one module
  snapshot, zero combinations, and one decoded loadout change with the exact
  before/after revision and loadout.
- Gateway logs contained one clean reset and no command, connection, or abort
  failure.

The normal loop, rather than validation-only direct reads, received the
snapshot, preview, and mutation result. This is therefore real client/server
integration evidence for the shipped workshop path.

## Creator-reviewable captures

| Capture | Meaning | Size | SHA-256 |
| --- | --- | ---: | --- |
| `01-server-preview.png` | Current Force+Ward/120 HP versus server-previewed Force/100 HP, explicitly not active | 148,032 bytes | `f008963aef72428be2142e7e848bfe8363434545d5579cfb84ac6aa514a8182b` |
| `02-accepted-next-session.png` | Revision 4 Force state with accepted-next-session/current-actor-unchanged disclosure | 151,070 bytes | `f929692882cbe52181386d3bef8d0f09005f2e361ff5b03f46ce16b31afe8839` |

Both are 1280×720 RGBA PNGs and pass `docs/art/m26/captures/SHA256SUMS`.
Visual inspection confirmed that catalog rows, current/preview arithmetic,
lifecycle text, controls, result text, and the dimmed gameplay context are
legible without clipping or overlap. These are engineering review artifacts;
no creator preference or outside-player comprehension claim is inferred.

## Integrated and technical gates

The final integrated smoke passed after all Block 7 source changes:

- two-active-client completion and bounded hostile pressure;
- two automatic/reused-account Godot runs, on-screen manual control, and the
  keyboard-only activity;
- every M17–M26 semantic marker, including the workshop marker;
- persistence, replay, current Inspector summary/events, frozen V1
  compatibility, exact V1 hashes, and standalone V1 reconstruction.

The smoke first exposed a harness-only launch race: two concurrent `cargo run`
commands could hold the target lock while the first bot waited for its peer.
Those attempts stopped before valid multiplayer gameplay. The script now
builds `revenant-bot` once and launches the two built binaries; two subsequent
complete smokes passed, including the final post-review run.

The block technical gate also passed Rust formatting, workspace/all-target/all-
feature Clippy with warnings denied, all 98 Rust/PostgreSQL tests, all-target
build, Inspector TypeScript check, and production Inspector build.

## Rejected and bounded evidence

- `session-1788259463836578537` completed and rewarded correctly but reached no
  module mutation because of the client MessagePack defect. It is diagnostic,
  not Gate 7 presentation evidence.
- `session-1788259661470496504` first demonstrated real preview and revision
  `1 → 2`, but later review changed the normal message loop, keyboard handling,
  reconnect reset, and operation IDs. Its captures were replaced.
- `session-1788260318901015528` used the final source and preserved exact
  append-ID/reward/loadout integrity, but a reproduced WSL wall-clock step put
  `activity_started` about 4m20s before its earlier-ID `player_joined`. The
  Inspector correctly rejected that temporal summary. The fresh accepted run
  above was monotonic. This remains the already documented local clock
  residual; replay authority continues to use append IDs.
- The long-lived Compose gateway image predates M26 and therefore cannot
  summarize new module events. All accepted M26 evidence used the current
  isolated binary; the Compose service was neither rebuilt nor treated as
  evidence in this block.

## Residuals entering Block 8

- The accepted graphical path used Mesa llvmpipe and dummy audio because this
  WSL shell has no playback device. Hardware audio perception remains untested;
  mute and fixed-pool semantics are deterministic.
- Captures and automated focus prove bounded presentation behavior, not creator
  preference, outside-player comprehension, or accessibility on other bodies
  and hardware.
- Fresh/reused inventory combinations, insufficient/retry/conflict over the
  real wire, both weapons across selected builds, two-active distinct builds,
  repetition, reconnect, exact combat/reward reconciliation, and resource
  bounds remain the explicit Block 8 matrix.
- The WSL wall-clock step remains an environmental residual and must not be
  described as absent merely because the accepted diagnostic was monotonic.

## Gate 7 decision

Gate 7 is **approved with bounded residuals assigned to Block 8**. The Godot
workshop presents server truth without optimistic mutation, remains operable by
keyboard, resets honestly on reconnect, uses bounded unique operation tokens,
and has accepted semantic, graphical, real-wire, replay, Inspector, smoke, and
technical evidence. No authority, compatibility, privacy, or presentation
blocker remains in Block 7 scope.
