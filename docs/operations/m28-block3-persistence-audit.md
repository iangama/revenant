# M28 Block 3 — migration and transactional persistence audit

Date: 2026-09-06  
Decision: **Gate 3 approved for the persistence boundary**

## Additive schema decision

`runtime/persistence/migrations/0008_cooperation_operations.sql` has SHA-256
`56409c65fda362eec25b79da5548224f4752c3ea9626dc96737e77e9f633b81e`.
It creates exactly `cooperation_operations`,
`cooperation_operation_participants`, and one anchor/time index. It changes no
M27 table, column, meaning, or row and introduces no replay kind, trigger, or
stored function.

The parent primary key permits at most one operation per session. SQL freezes
the activity/catalog, exact participant count, role order, objective targets,
timing constants, revive values, reward, lifecycle prefix, contribution
nullability, health/life bounds, success/failure outcomes, monotonic temporal
order, subdeadline precedence, and success-only Warden evidence. The named
temporal constraint is applied idempotently through an anonymous migration
block so the already-empty reviewed M28 table and a clean restore converge on
the same strengthened contract without altering legacy tables or retaining a
database function.

Application loading locks and reconstructs the complete row set through the
pure `m28-v1` domain. It additionally validates exact two-row cardinality,
anchor/runner order, account/character ownership, unique identities, canonical
M26 weapon/module profile resolution, admitted/current health, life state, and
parent/participant parity. The reviewed design is recorded in
`docs/operations/m28-block3-persistence-contract.md`.

## Pre-migration backup and restore

Before the first `0008` application, all fourteen M27 tables were preserved at
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block3-pre-migration-20260906` as
a custom-format dump, schema, inventory/counts, restore list, and stable ordered
JSONL:

- dump size: 380,548 bytes;
- dump SHA-256:
  `644e160dd06ccb4e86851472bb2962e94e5e5a74bfa87636dff36328fa9a47b7`;
- verified manifest SHA-256:
  `8e0f5cd6b3424ea89f3449ce4451957ed569c1499b1be512965f73e8e70fe57e`;
- 826 accounts and characters, 2,409 inventory rows, 826 progression rows,
  1,065 histories, 9,382 replay rows, 991 inventory grants, 980 progression
  grants, 802 equipment loadouts, 165 module states, 99 module slots, 2,044
  module operations, 216 route operations, and 254 route participants; and
- `pg_restore --list` and every manifest entry verified.

The dump restored into the explicitly checked-absent disposable database
`revenant_m28_disposable_0906c`. All fourteen restored JSONL files matched the
backup byte-for-byte before DDL. The complete schema then ran twice through
`Persistence::connect` under the existing migration advisory lock. Both runs
produced exactly sixteen tables, zero cooperation rows, and byte-identical
legacy exports.

## Durable operation behavior

Every mutation uses one PostgreSQL transaction and the session advisory lock.
Nothing is returned as applied before commit.

- start validates exact authority, identities, canonical profiles, admitted
  health, and two participants; exact retry replays and conflict writes
  nothing;
- anchor, ping, scripted downing, revive start/cancellation/completion, every
  timeout, participant defeat, and disconnect reconstruct and mutate one
  locked state;
- repeated arrival/downing observations check the active deadline before
  replaying, so a late retry commits the deterministic timeout;
- downing changes the runner to exact zero/downed and revive restores exact
  50/active in the same transactions as their parent transitions;
- cancellation clears the active revive identifier and reserves no retry
  budget;
- success grants both participants exactly two `relay_core_fragment` and 125
  XP, with both inventories, progression rows, levels, grants, histories, and
  cooperation terminal in one transaction; and
- failure, abandonment, incomplete state, conflicting terminal, or partial
  pre-existing grant produces no reward or history.

Two independent connections proved one apply/one replay convergence for the
same start, ping, revive start, revive completion, and success. Conflicting
concurrent start input produced one winner and one fail-closed conflict.

## Rollback and constraint evidence

Nine focused cooperation PostgreSQL cases passed. Selective failure triggers
covered parent and both participant start inserts; anchor, ping, parent and
participant downing; revive start, cancellation, parent and participant revive
completion; every existing reward/grant/progression/level/history boundary;
terminal update; and deferred commit. Every injected failure left operation,
participant, inventory, progression, grants, history, and replay observations
equal to the pre-request snapshot, after which the same input remained usable.

Direct SQL drift attempts rejected constants, lifecycle prefixes, role/profile
bounds, transition past 60,000 ms, terminal time reversal, wrong timeout
precedence, Warden evidence without success, and invalid success evidence. The
accepted disposable run ended with 66 operations and 132 participant rows: 20
successes, two ping timeouts, four revive timeouts, four operation timeouts, two
participant defeats, two abandonments, and 32 intentionally incomplete rows.
Its 20 successes produced exactly 40 inventory and 40 progression grants, and
Block 3 retained zero cooperation replay rows. No M28 failure trigger/function
remained.

Accepted disposable evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block3-disposable-20260906c`;
its verified manifest SHA-256 is
`6009de322d6c32e9aa5dcc8373f9f60221f9ca125b9d452d6731138a05efa039`.
After final counts were recorded, the literal disposable database was removed
and its absence verified. The backup and evidence directories were preserved.

## Working database and protected boundaries

The working database correction evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block3-working-20260906c`, with
verified manifest SHA-256
`4d4edfe400fe3f56e121e306cd4e7f6acf3404d1431dd80dac398c3d903e545d`.
Fresh before/after exports prove all fourteen then-current legacy tables were
byte-identical across two schema applications. The working database has sixteen
tables, the strengthened temporal constraint exactly once, zero cooperation
operations/participants, and zero M28 failure triggers/functions.

The uninterrupted canonical `make check` passed version 0.2.0, format,
workspace/all-target/all-feature Clippy with warnings denied, all 193 Rust
tests, all-target build, Inspector check/build, a 361-file secret audit, the
complete multiplayer/Godot/persistence/replay/Inspector smoke, current
Protocol V2, frozen V1 compatibility, and standalone V1 reconstruction. Its
log SHA-256 is
`feba8faa53c78aac18829b215b2d7f4a2f3d11d89e182c1708ebe94508c840e2`.

All 115 protected files under `VERSION`, frozen V1, Protocol, gateway, replay,
M27 operations, Godot, activity scripts, and migration `0007` are byte-identical
to the verified M27 closure checkpoint. The checkpoint remains SHA-256
`81bb1fafaa397b27db7fc505767c09202d68b77e13355952b49ad9d994280d70`.
Frozen V1 main/Cargo hashes and the routed Lua/`0007` hashes remain unchanged.

## Rejected and superseded evidence

`m28-block3-disposable-20260906`,
`m28-block3-disposable-20260906b`, and
`m28-block3-working-20260906` are preserved but rejected for Gate 3. Their
tests exposed no data loss, but final review found that their SQL admitted
Warden completion without an atomic success terminal and did not constrain all
transition/terminal temporal order or timeout precedence. Arrival/downing retry
shortcuts also checked stored elapsed input before observing a newly expired
deadline. The strengthened migration, persistence correction, new regression
vectors, clean restore, working-table reconciliation, and full gate in the
`20260906c` evidence eliminate those gaps.

## Gate decision and residuals

Gate 3 is green for additive migration, backup/restore, apply-twice behavior,
legacy preservation, durable lifecycle state, concurrency/idempotency, exact
success rewards, no-reward failure, rollback, and protected-boundary
compatibility.

The residual is intentionally bounded: Block 3 contains no cooperation replay
event, independent event reconstruction, Inspector projection, wire message,
gateway lifecycle, bot, or Godot behavior. Cross-row cardinality, ownership,
canonical profile, and life parity remain application validations under locks
rather than trigger-based SQL. Only Block 4's exact six-event replay/payload
review and implementation are now authorized; protocol, gateway, bots, and
Godot remain locked.
