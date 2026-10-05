# M27 Block 5 — replay, reconstruction, and Inspector audit

Date: 2026-09-01  
Decision: **Gate 5 green**  
Scope: the reviewed contract in
`docs/operations/m27-block5-replay-contract.md`.

## Accepted implementation

Block 5 adds exactly three recognized replay kinds:
`route_selected`, `route_operation_succeeded`, and
`route_operation_failed`. Their schema-version-1 payloads are compact JSON,
bounded to 8,192 UTF-8 bytes, deny unknown fields at the top level and in
nested route effect, reward, and terminal-summary structures, and retain the
complete immutable evidence needed to reconstruct without consulting current
Lua, route tables, inventory, loadout, time, or randomness.

The selection evidence contains the exact activity/authoring/catalog/resolver
revisions, operation, leader, admission-ordered participants, M26 module
snapshots, selected immutable weapon profiles, route, server seed, resolved
event/effect, objective graph, duration, and planned reward. Reconstruction
requires exactly the same complete V2 snapshot set in the same admission order
before selection and rejects a late admission snapshot.

The shared terminal payload records the consecutive objective transitions,
bounded encounter evidence, exact immutable operation summary, and ordered
participant grants. Reconstruction validates:

- success, timeout failure, selected-but-incomplete, and explicit legacy
  states;
- route/event/revision/operation/activity, account/actor/character, seed,
  effect, path, duration, deadline, pressure, combat-profile, and reward
  agreement;
- legal objective activation/completion/failure order with at most one active
  objective and no post-failure transition;
- exact generic completion, loot, and progression rows for success, including
  their full textual shape, activity, participant, actor, quantity, and XP;
- no completion or reward before or after a failed terminal, no generic
  completion/reward after either route terminal, and no route event after a
  terminal; and
- malformed/oversized JSON, nested unknown fields, unknown replay kinds,
  mixed sessions/activities, duplicate/missing/out-of-order evidence, wrong
  admission order, inactive V1 module evidence, and impossible encounter
  arithmetic.

An event stream without any route kind still reconstructs as
`route_replay_legacy=true` and fabricates no route fact. Existing M26 module
reconstruction continues independently.

## Atomic persistence and retry evidence

The replay-aware persistence entry points preserve the Block 4 APIs and add
atomic selection, success, and timeout variants. A new selection commits the
route parent, ordered participants, and `route_selected` together. A success
commits the existing generic completion, every inventory/progression/history
write and generic replay row, the durable terminal, and
`route_operation_succeeded` in one transaction. A timeout commits only the
durable failed terminal and `route_operation_failed`.

Same-input retries compare stored replay evidence byte-for-byte and append
nothing. Conflicting retries fail. The PostgreSQL test repeats failure
injection across all 15 replay-mode boundaries:

- three selection boundaries: parent, participant, and selection replay;
- seven existing durable success boundaries: inventory grant/inventory,
  progression grant/progression, character level, history, and terminal;
- four success replay boundaries: activity completion, loot, progression, and
  route terminal; and
- the failed-terminal replay boundary.

Every injected failure leaves the operation/reward state exactly at its prior
committed boundary; after the trigger is removed, the same operation remains
reusable and reconstructs exactly. No failure-injection trigger or function
remained installed after the final gate.

## GET-only Inspector evidence

The existing Inspector summary adds only the seven reviewed facts:

- `route_replay_legacy`;
- `route_id` and `route_event_id`;
- `route_terminal_outcome` and `route_elapsed_ms`;
- `route_transition_count`; and
- `route_reward_participant_count`.

The existing event route returns account identity and a decoded structured
payload only after full-session reconstruction succeeds. The frontend labels
that value as validated persisted evidence. No route mutation, raw SQL,
packet injection, seed/reroll, or client-derived result was added.

The final rebuilt source gateway ran only on isolated loopback ports
`127.0.0.1:18081` and `127.0.0.1:17001`. For durable session
`m27-replay-success-1788294245016609696`, GET summary returned:

```json
{
  "route_replay_legacy": false,
  "route_id": "breach",
  "route_event_id": "overcharged_armor",
  "route_terminal_outcome": "succeeded",
  "route_elapsed_ms": 90000,
  "route_transition_count": 5,
  "route_reward_participant_count": 1,
  "event_count": 7
}
```

The event route returned `route_selected` at append ID 6907 and
`route_operation_succeeded` at append ID 6911 with the same leader account,
actor, decoded route, and event-kind labels. `POST`, `PUT`, `PATCH`, and
`DELETE` each returned HTTP 405. The isolated process was then stopped and
neither port retained a listener.

## Complete validation

The final uninterrupted canonical command was:

```bash
DATABASE_URL_FILE=/owner-only/path/operator_database_url \
CARGO_HOME="$PWD/.tooling/cargo" \
RUSTUP_HOME="$PWD/.tooling/rustup" \
PATH="$PWD/.tooling/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" \
make check
```

It passed with `VERSION=0.2.0`, formatting, workspace/all-target/all-feature
Clippy with warnings denied, **149 tests**, complete workspace build,
TypeScript check, production Inspector build, secret audit, multiplayer smoke,
frozen-V1 compatibility, reconstructed-V1, and M17-M20 markers. Focused replay
evidence includes 11 pure reconstruction tests and four PostgreSQL route
replay tests; the failure-matrix test internally exercises the 15 transaction
boundaries above.

The post-gate working database contained mutable local engineering fixtures:
163 route rows, 79 selection replay rows, 47 success route terminal rows, and
16 failed route terminal rows. Block 4 fixtures intentionally account for
route rows without replay. Among rows that contain any route replay evidence,
zero lacked exactly one selection, zero replay terminals lacked a durable
terminal, and zero timeout rows had generic completion/reward evidence. The 23
unknown rows were all deliberate `future_event` fixtures and remain rejected,
not accepted vocabulary. These counts are test-environment state, not product
or player metrics.

## Protected boundaries

The external M26 checkpoint remains:

`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m26-closure-45718926-20260901.tar.gz`

SHA-256:
`066b9251f47c7b916b298d453a9c11b95c63ea305247e3cbf8010e0340256d82`.

The final comparison covered 102 files under `VERSION`, frozen V1,
`runtime/protocol`, and `client/game`. Archive and worktree path lists both
hashed to
`fc20a632833643b9fe1e734071e4fd6059a3f871f13f6982408deeccbbdb3b42`,
with no path-set difference and zero byte mismatch. The gateway diff against
that checkpoint is confined to the existing Inspector read path, decoded
evidence, exact route summary fields, GET method enforcement, and its unit
test. No gameplay lifecycle or Protocol message changed. No generated `.gd.uid`
sidecar remained, and `git diff --check` passed.

One localized `clippy::struct_excessive_bools` allowance remains on the
pre-existing flat Inspector summary DTO because the required
`route_replay_legacy` field crosses the lint's presentation threshold. It
changes no validation or authority rule.

## Rejected attempts and corrections

- The first manual method probe showed that the old path-only HTTP parser
  answered non-GET methods with 200. The request line now validates method,
  path, and HTTP version; the four mutation verbs return 405 and have a unit
  assertion.
- The first full integration run let one new PostgreSQL fixture call migration
  DDL in parallel and encountered a DDL deadlock. Block 5 fixtures now use the
  already-migrated `connect_existing` path; the default-parallel target and
  full gate pass.
- The final audit review found missing explicit checks for admission order,
  nested unknown fields, and generic evidence after terminal. Those checks and
  adversarial assertions were added before the accepted gate.
- The expanded failure matrix first used an over-64-byte temporary session ID;
  the fixture was shortened without relaxing the production bound.
- Its generalized trigger guard initially remembered only the old
  `replay_events` cleanup table. The one exact temporary trigger/function was
  removed, the guard now records its exact table, the complete matrix passed,
  and the final database audit found zero failure objects.

## Residuals and gate decision

Block 5 intentionally does not send route messages, mutate the gateway route
lifecycle, or render route choice in Godot. Those are Gates 6 and 7. The
working database retains bounded test fixtures by design; no claim relies on
their aggregate counts. Replay payload schema 1 and the three new names must
remain stable unless a later explicit replay gate reviews a change.

Gate 5 is green. Block 6 may now review and freeze the exact additive opt-in
Protocol V2 and gateway contract. No Block 6 implementation is approved until
that pre-implementation review is recorded; frozen V1, ordinary unopted V2,
Godot, `VERSION`, local-only networking, and no-commit/no-release boundaries
remain in force.
