# M27 Block 3 — restricted Lua authoring audit

Date: 2026-09-01  
Decision: **Gate 3 approved**

## Accepted scope

Block 3 changed only the activity authoring boundary and its local validation
tooling. It added no persistence schema, replay vocabulary, protocol message,
gateway route behavior, Godot behavior, frozen-V1 change, version change, or
working-database mutation.

The accepted implementation consists of:

- the dependency-light `revenant-operations` catalog consumed as the exact
  source of route/event values;
- bounded declarative parsing in `revenant-activities`;
- the extended `scripts/activities/relay_awakening.lua` authoring table;
- the byte-exact M26 golden fixture plus an explicit rejected fixture; and
- the local `revenant-activity-validator [--json] <activity.lua>` CLI.

## Contract evidence

The loader accepts valid UTF-8 source of at most 64 KiB, evaluates text only
with table/string libraries, caps Lua memory at 1 MiB and VM work at 100,000
instructions, then validates a static graph capped at 128-byte strings, 64
tables, 256 values, and depth eight. Functions, userdata, threads, metatables,
floats, bytecode/non-UTF-8 files, cycles, aliases, dynamic loading, unknown
fields, mixed/sparse sequences, and invalid types are rejected.

Schema limits are eight objectives, twelve triggers, two routes, two events
per route, four objectives per route path, and targets from 1 through 100.
Identifiers, duplicate semantic identities, objective references, initial
state, completion cardinality, reachability, cycles, terminal cardinality,
late terminal activation, and the exact `m27-v1` catalog are validated before
a manifest is returned. Route duration, rewards, progression, path, event
order, and effects must equal the frozen Gate 2 definitions; Lua supplies no
formula or callback.

The original M26 Lua was preserved before extending the active source:

- `tests/fixtures/m27/relay_awakening_m26.lua`: 1,071 bytes;
- SHA-256:
  `460c895c2bb99090cbcf59d9238717fb3a868c9037dcb46fca5a88fc93e6b040`.

It remains valid as `revision=compatibility`, with three objectives, three
triggers, no routes, one fragment, and 100 XP. A direct runtime projection test
proves that loading the extended table still produces the exact M26 objective,
trigger, reward, and progression state used by an unrouted session.

## Deterministic CLI evidence

Two independent validations of the active 2,986-byte Lua source produced
byte-identical 3,391-byte JSON files:

`/tmp/revenant-m27-block3-authoring-a.json`  
`/tmp/revenant-m27-block3-authoring-b.json`

Both have SHA-256:

`2efd469f456b19a4d51d84cdcb1cd812ec31b03f0e5aab487cc20a4e1ac800ef`

The human-readable mode accepted the M26 golden and reported its compatibility
projection. `tests/fixtures/m27/reject_unknown_activity_field.lua` exited with
status 1 and diagnosed `unknown activity field: unknown`.

## Quality and compatibility evidence

- focused activity/CLI tests: 14 passed, zero failed;
- workspace tests: 134 passed, zero failed;
- workspace formatting: passed;
- workspace Clippy over all targets with warnings denied: passed;
- workspace all-target build: passed; and
- workspace-test log SHA-256:
  `003f116d33d195a8ed0c6963cb4af7f1137e924e09407919974d1b7e031093bf`.

A checkpoint/current path-list and per-file SHA-256 comparison covered 119
files under `VERSION`, frozen V1, Godot, gateway, persistence/migrations,
Protocol, and replay. Names and contents were byte-identical. The stable
protected path list has SHA-256:

`307585e2238829b8285a8a23eccaf1801cc1e47cff227781bed99be329a3f333`

Ignored `.godot` editor/import caches were deliberately outside both the
source checkpoint and this comparison.

## Rejected evidence and corrections

The first focused run was rejected: five cases passed and six failed because
the frozen revision token was incorrectly checked as an underscore-only ID,
and two negative fixtures did not construct the intended sparse/cyclic graphs.
The validator exception was narrowed to the exact catalog revision and the
fixtures were corrected. A later run had all library cases green but exposed
that the CLI accepted an unknown option as a path; unknown options now reject.
The first strict Clippy run was also rejected and its type-complexity,
function-size, and redundant-return findings were refactored without an allow.
Only the clean reruns above are accepted evidence.

## Gate decision and residual boundary

Gate 3 is green. The activity file is validated and deterministic, but routing
is not yet durable or live in the gateway; this audit makes no runtime-route,
replay, client, preference, or human-usability claim.

Gate 3 authorizes only M27 Block 4: review exact additive `0007` DDL, create and
verify an external pre-migration backup, prove disposable restore/apply-twice
and legacy preservation, then implement the bounded transactional route
operation. The working database must not be migrated until those earlier
Block 4 proofs pass. Replay vocabulary, Protocol, gateway routing, and client
changes remain locked behind later gates.
