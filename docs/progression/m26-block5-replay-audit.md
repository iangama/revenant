# M26 Block 5 replay reconstruction and Inspector audit

Date: 2026-08-31  
Gate: 5 — structured replay, atomic mutation evidence, reconstruction, and
read-only Inspector projection.  
Decision: **approved**.

## Reviewed vocabulary and payload boundary

`docs/progression/m26-block5-replay-contract.md` froze the exact contract
before implementation. Block 5 adds only:

- `module_state_snapshot`;
- `module_combined`; and
- `module_loadout_changed`.

Each payload is deterministic compact JSON with `schema_version: 1`, rejects
unknown fields, and is limited in application code to 8,192 UTF-8 bytes. The
snapshot records character/protocol identity and complete state. Mutations
record operation identity plus complete before/after state. Each state embeds
the exact `m26-v1` four-entry catalog, fragment/ownership/loadout/revision,
activation, aggregate modifiers, and base/effective profiles for both weapons.
Reconstruction therefore queries neither current inventory nor mutable catalog
tables.

Frozen V1 snapshots retain persisted V2 ownership/loadout for evidence but
record modules inactive, an empty applied loadout, and exact base profiles.
Current V2 snapshots apply the canonical persisted loadout. Sessions with no
module event retain the explicit legacy empty-revision-zero rule; their old
payloads and vocabulary were not rewritten.

## Independent reconstruction

`revenant-replay` now consumes strict increasing append IDs and validates each
M26 payload before changing reconstructed state. It fails closed on mixed
sessions, non-increasing IDs, missing/non-adjacent/duplicate participant
snapshots, actor disagreement, mutation before completion, frozen-V1 mutation,
unknown schema/catalog/module/field, wrong ordering/ownership/activation,
arithmetic mismatch, impossible recipe/balance, and impossible loadout/revision
transition.

For the existing Relay reward payload, the recorded authoritative fragment
total advances the participant state before a later combination. Mutation
before-state must then equal reconstructed state exactly. Every profile is
re-resolved with checked half-up arithmetic and compared with the embedded
effective values.

Six focused replay tests pass:

1. unchanged legacy reconstruction;
2. V2 snapshot, combination, and loadout reconstruction;
3. frozen V1 persisted-versus-applied separation;
4. missing adjacency and append-order rejection;
5. arithmetic and unknown-field JSON corruption rejection; and
6. exact fragment reward followed by combination.

## Atomic PostgreSQL evidence

`revenant-persistence` adds typed replay contexts and three atomic paths:

- player join plus module snapshot;
- combination inventory/operation plus `module_combined`; and
- whole-loadout slots/revision/operation plus `module_loadout_changed`.

Mutation replay context must match an account-owned character, prior
participant snapshot, same session/activity/actor, and a later persisted
`activity_completed`. An accepted durable retry returns its stored result
before any new replay insert. Rejection and replay-insert failure reserve no
new operation and expose no partial state.

Five new PostgreSQL integration tests pass in addition to the 12 prior
persistence tests. They prove:

- exact snapshot/mutation ordering, retry, reconnect, summary, and independent
  reconstruction;
- V1 inactive evidence for a character with persisted V2 modules;
- snapshot-insert failure rolls back the preceding player join;
- combination/loadout replay-insert failure rolls back inventory, operation,
  slots, and revision and leaves the ID reusable; and
- two concurrent connections produce one applied result, one replayed result,
  and exactly one `module_combined` row.

All selective failure triggers/functions were removed. The final database
query found zero remaining M26 failure fixture.

## Read-only Inspector and CLI evidence

The existing Inspector routes remain GET-only. The authoritative summary adds
legacy, participant, snapshot, combination, and module-loadout counts after
running the same independent reconstruction. The event endpoint returns a
validated `decoded_payload` for module events; malformed structured evidence
fails rather than falling back to raw trust. The React view shows the added
counts and pretty-prints exact decoded before/after objects. No route, query,
credential, raw SQL, or mutation capability was added.

A freshly built gateway ran temporarily on loopback ports `18080/17000`, read
one real synthetic M26 session, and was then stopped. Canonical services were
left running and healthy on their prior loopback ports. Accepted evidence is:

`/mnt/c/Users/Ian/revenant-local-evidence/m26-block5-inspector-20260831`

It is 25,316 bytes. Its `SHA256SUMS` has SHA-256
`9c0f76e2e7fa8e7492f4efbfa7611b2469fa82865cf49c5a1b5f5627a00517cc`,
and every entry verifies. Session
`session-m26-exact-1788223695994085227` returned exactly:

| Ordered event | Decoded | Payload bytes |
| --- | ---: | ---: |
| `player_joined` | no | 25 |
| `module_state_snapshot` | yes | 1,403 |
| `activity_completed` | no | 18 |
| `module_combined` | yes | 2,771 |
| `module_loadout_changed` | yes | 2,785 |

The summary reported one module participant, snapshot, combination, and
loadout change, five total events, completed true, and legacy false. The CLI
independently reconstructed the same counts and final state. The evidence
directory contains raw health/summary/events JSON, CLI output, selected session
ID, source hashes, and its verified manifest.

## Quality gate

Using the temporary Rust 1.98.0 toolchain, the final block passed:

- `cargo fmt --all -- --check`;
- workspace Clippy for all targets with warnings denied;
- all 90 workspace unit, PostgreSQL integration, compatibility, CLI, and doc
  tests;
- `cargo build --workspace --all-targets`;
- Inspector `npm run check` and production `npm run build`; and
- `git diff --check`.

The production Inspector bundle contains 16 transformed modules; its generated
JavaScript was 198.38 kB (62.40 kB gzip) and CSS 8.14 kB (2.52 kB gzip).
Protocol V2, frozen V1 source/artifacts, gameplay admission/routing, Godot,
`VERSION`, and the M24 package were not changed in this block.

## Rejected pre-gate attempts

1. The first replay compilation compared structurally equivalent base and
   effective profile types directly in one V1 fixture. Rust rejected the type
   mismatch before tests. The fixture now compares the four typed fields.
2. All five new PostgreSQL tests first passed, but Clippy rejected the two
   mutation methods at 104/110 lines. Typed replay insertion was extracted into
   focused helpers without changing transaction scope.
3. The combined persistence suite then rejected a test assertion that global
   replay IDs must be consecutive. Parallel sessions may legitimately consume
   an intervening global ID; the actual contract is increasing IDs and adjacent
   events within the filtered session stream. That exact reconstruction check
   remained and the invalid global assumption was removed.
4. All focused suites passed again, but Clippy rejected a 105-line integration
   test. Its read-only reconnect assertions were extracted into one helper.

No rejected attempt produced accepted evidence or changed schema, protocol,
frozen V1, gameplay routing, or client state.

## Residuals entering Block 6

- Gameplay still uses the pre-M26 join/equipment flow. Gate 6 must route V2
  admission and module mutations only through the new atomic replay methods.
- The new payloads intentionally repeat a small fixed catalog/profile snapshot;
  the measured maximum is far below 8,192 bytes. Any future catalog expansion
  requires a new reviewed replay schema rather than silently consuming headroom.
- Legacy Relay fragment continuity relies on its exact existing authoritative
  reward-total payload. Gate 6 runtime fixtures must prove that completion then
  combination reconstructs without divergence.
- Inspector validation is local engineering evidence, not an authorization to
  expose it publicly or add mutation controls.

## Gate 5 decision

Gate 5 is **approved**. Exact vocabulary, bounded payloads, V1/V2/legacy
semantics, independent reconstruction, corruption rejection, replay atomicity,
concurrent retry, CLI parity, and read-only Inspector projection satisfy the
block contract. Block 6 may begin with an exact additive Protocol V2 message
review. No protocol or gameplay gateway change is accepted until that review,
and Godot remains behind Gate 7.
