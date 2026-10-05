# M28 Block 4 — replay, reconstruction, and Inspector audit

Date: 2026-09-06  
Decision: **Gate 4 approved**  
Scope: the reviewed contract in
`docs/operations/m28-block4-replay-contract.md`.

## Accepted replay contract

Block 4 adds exactly six recognized replay kinds:

1. `cooperation_started`;
2. `cooperation_pinged`;
3. `player_downed`;
4. `player_revived`;
5. `cooperation_succeeded`; and
6. `cooperation_failed`.

Their schema-version-1 payloads are canonical compact JSON, deny unknown
fields at every M28-owned level, and enforce the reviewed 8,192-byte ceiling.
They embed the complete immutable evidence required for reconstruction:
ordered identities/roles, admitted M26 module and selected-weapon profiles,
fixed objective targets, exact timing/revive/reward constants, contribution
and health/life transitions, terminal outcome/subject, and success grants or
an empty failure grant list.

The start evidence must follow exactly two active-V2 module snapshots in
durable admission order. The successful M28 subsequence is exactly start,
ping, down, revive, and success. A failed stream contains the legal prefix and
one failure terminal. A crash prefix remains explicitly active/incomplete.
Append IDs define order; timestamps and current mutable rows do not.

`ReconstructedSession` now owns an explicit cooperation projection with
legacy, active, succeeded, and failed states. Reconstruction reads replay rows
only. It does not consult current operation/participant rows, actors, Lua,
inventory, module/loadout tables, gateway memory, randomness, wall time, or a
current catalog not frozen in the events.

Six focused replay tests cover complete success, every incomplete prefix, the
exact ping timeout, skipped transitions and timeout precedence, route
exclusion, size/schema/unknown-field rejection, identity/profile/health/life
drift, duplicate or post-terminal evidence, and missing/partial/wrong rewards.
An event stream without an M28 kind remains cooperation-legacy and fabricates
no role, phase, contribution, life, outcome, or reward.

## Atomic persistence and failure evidence

Replay-aware persistence entry points compose the accepted Gate-3 state
machine and transaction locks. Start commits the parent, both participants,
and `cooperation_started` together. Ping, scripted downing, successful revive,
success, and every failure append their canonical event in the same
transaction as the durable transition. Pending/cancelled revive observations
append nothing.

Success orders one generic completion, both participants' existing durable
inventory/progression/history grants and generic replay facts, then the M28
terminal as the final transaction event. Failure appends only the M28 failure
terminal and never writes generic completion, history, loot, or XP. Exact
retry validates the stored reconstructed stream and appends/grants nothing;
missing, corrupt, conflicting, or partial companions fail closed.

Six focused PostgreSQL cases cover:

- exact success, retry, independent reconstruction, and summary projection;
- terminal reward-free timeout, exact retry, and failure-append rollback;
- all five failure families with exact subjects and zero reward;
- persisted corruption rejection by both summary and retry;
- all eleven success reward/history/terminal/generic-append boundaries; and
- start, ping, down, revive, and success replay-append rollback.

The pre-existing nine-case Gate-3 PostgreSQL suite also ran in the canonical
gate, so every base lifecycle, concurrency, constraint, and transaction
boundary remained green with the replay layer enabled. Every injected error
left the complete unit at its prior committed state and the same operation
remained usable after removing the injection.

## CLI and GET-only Inspector parity

The authoritative session summary adds only the nine reviewed cooperation
facts. The existing Inspector event route validates the complete structured
stream before exposing a decoded cooperation payload. The frontend renders
those persisted facts, while the CLI reports the same reconstruction. No
Inspector mutation surface, raw SQL, packet injection, timer, life-state
calculation, or gameplay command was added.

For durable disposable session
`m28-replay-success-1788739730919979658`, CLI and Inspector agreed on:

```text
legacy=false
state=succeeded
phase=encounter_active
outcome=succeeded
terminal_elapsed_ms=3000
participant_count=2
contribution_count=5
revive_count=1
reward_participant_count=2
```

The event endpoint decoded start at append ID 9936, ping at 9937, down at
9938, revive at 9939, and success at 9947. Generic completion and both ordered
loot/XP pairs occur before the success terminal. `POST`, `PUT`, `PATCH`, and
`DELETE` each returned HTTP 405. The rebuilt gateway listened only on the
isolated loopback ports `127.0.0.1:18080` and `127.0.0.1:17000`, was stopped,
and neither port retained a listener.

## Disposable database and legacy preservation

Gate-4 evidence used the explicitly checked-absent disposable database
`revenant_m28_replay_0906a`, restored from the accepted Gate-3 pre-migration
dump. The dump SHA-256 remained
`644e160dd06ccb4e86851472bb2962e94e5e5a74bfa87636dff36328fa9a47b7`,
and migration `0008` remained
`56409c65fda362eec25b79da5548224f4752c3ea9626dc96737e77e9f633b81e`.
Two direct idempotent applications retained exactly sixteen tables, one named
temporal constraint, and initially zero cooperation rows/events.

A second clean restore isolated legacy preservation. Data-only dumps of all
fourteen M27 tables before and after two `0008` applications were
byte-identical after removing only PostgreSQL 16's independently randomized
`\\restrict`/`\\unrestrict` guard token. Both normalized dumps have SHA-256
`9caa5b58ed5c08a9c2e3851f9f6d6d4108f2be46e226810bbdbcd7072863178f`.
The migration added the two empty reviewed tables and changed no legacy row.

The accepted test run ended with the following disposable fixture facts:

- 40 starts, 36 pings, 30 downings, 28 revives, 26 replay successes, and 12
  replay failures;
- zero route/cooperation coexistence sessions;
- zero generic completion/reward rows attached to any failed or abandoned
  operation;
- maximum observed cooperation payload size 3,581 bytes; and
- two deliberately corrupted ping rows, one from each focused/canonical
  corruption run, retained solely to prove fail-closed loading.

These are engineering fixtures, not product or player metrics. Both literal
disposable databases were removed after evidence capture and their absence was
verified.

## Complete validation and protected boundaries

The uninterrupted canonical command used the disposable database and the
repository-pinned Rust/Godot tools:

```bash
DATABASE_URL_FILE=/owner-only/path/disposable_database_url \
CARGO_HOME="$PWD/.tooling/cargo" \
RUSTUP_HOME="$PWD/.tooling/rustup" \
PATH="$PWD/.tooling/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" \
GODOT_BIN="$PWD/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64" \
make check
```

It passed `VERSION=0.2.0`, formatting, workspace/all-target/all-feature
Clippy with warnings denied, all **205 Rust tests**, complete workspace build,
TypeScript check, production Inspector build, a 366-file secret audit, the
multiplayer and Godot smoke flows, persistence/replay reconciliation, current
Protocol V2, frozen-V1 compatibility, and standalone V1 reconstruction. The
canonical log SHA-256 is
`9ebe9e44bf14fab39e6412f198740ba68bdc6da7a9abcf890214338106a35898`;
`git diff --check` also passed.

Against the verified M27 closure checkpoint SHA-256
`81bb1fafaa397b27db7fc505767c09202d68b77e13355952b49ad9d994280d70`,
all checkpoint files under `runtime/protocol`, frozen V1,
`runtime/operations`, `client/game`, `scripts`, and `tests`, plus `VERSION` and
migration `0007`, are byte-identical. The six current extra directories in
the client/scripts comparison are empty. The 59-line gateway diff is confined
to replay imports and existing GET Inspector summary/event decoding; gameplay
lifecycle code is unchanged. Protocol, gateway gameplay, fake client, Godot,
frozen V1, M27 replay shapes, and version remain locked.

Accepted external evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block4-replay-20260906`.
Its verified manifest SHA-256 is
`f4ec22b93323121effb8196096e64606121b212a416e314c6a0282f89620565f`.

## Rejected diagnostics and cleanup

The first schema probe used `persistence-check`, which successfully opens the
application persistence path but then requires an unrelated historical
`arc_sidearm` fixture. The restored dump does not contain that expected state,
so the command exited with `persisted sidearm was not found`. Its output is
preserved and explicitly rejected as a schema-success proof. Gate 3 already
contains the accepted application-lock apply-twice proof; Gate 4 changed no
DDL, and its direct PostgreSQL apply-twice and legacy comparisons are green.

The final post-`make check` database sweep found one named M26 module-test
failure trigger/function whose best-effort test cleanup had not removed it.
The exact table, trigger, function, and empty argument signature were inspected
before removing only those two disposable objects. The accepted final sweep
contains zero non-internal triggers and zero public functions. No M28 failure
injection object was retained.

The two raw legacy `pg_dump` files initially had different hashes solely
because PostgreSQL generated a different safety token on their
`\\restrict`/`\\unrestrict` lines. The diff was inspected, those two
non-data lines were normalized, and the complete remaining files matched
byte-for-byte.

## Gate decision and residuals

Gate 4 is green for the exact six-event vocabulary, bounded canonical
payloads, transactional append/reward coupling, exact retry, independent
reconstruction, corruption rejection, failure precedence, legacy
preservation, CLI/Inspector parity, GET-only exposure, and protected-boundary
compatibility.

The residual is intentional and bounded: Block 4 does not add cooperation wire
messages, gateway gameplay integration, deterministic role bots, or Godot
presentation. Aggregate fixture counts have no player meaning, and automated
evidence makes no claim about human coordination, comprehension, preference,
accessibility, or enjoyment. Block 5 may now review the exact additive
Protocol V2 structs and gateway composition contract. Protocol and gameplay
implementation remain locked until that review is recorded.
