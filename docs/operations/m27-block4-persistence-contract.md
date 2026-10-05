# M27 Block 4 — persistence contract review

Date: 2026-09-01  
Status: exact DDL frozen for pre-migration safety proof; not yet applied

## Boundary decision

Migration `0007_route_operations.sql` is additive and idempotent. It creates
exactly two tables plus one index and does not alter, rename, delete, backfill,
or reinterpret any of the twelve M26 tables or their rows.

`route_operations` owns exactly one accepted route resolution per bounded
session. Its primary key is `session_id`; the canonical request is the
`operation_id`/`route_id` pair, and the remaining immutable selection columns
are its canonical server result. The row freezes:

- activity, catalog, and resolver revisions;
- leader, participant count, route, 63-bit seed, event, and exact effects;
- exact three/four-step objective path and 90,000 ms duration budget;
- exact per-participant route reward plan; and
- an initially-null terminal outcome, elapsed duration, and terminal time.

SQL constraints enforce the exact Gate 2 route/event/effect/path/reward
combinations rather than only their broad numeric envelopes. Terminal columns
are all-null or all-present; success is at or before the budget and timeout
failure is strictly after it.

`route_operation_participants` stores one or two participants in authoritative
admission order with account, character, and actor foreign-key/evidence
identity. Per-session account, character, and actor identities are unique. The
application must additionally prove that each character belongs to its stored
account and that row count/order equals the parent participant count inside the
same transaction.

No replay event is introduced by this DDL. Block 5 still owns exact vocabulary
and bounded payload review. Block 4 must leave transaction seams that can add
those reviewed replay writes atomically; it must not invent event names early.

## Accepted-operation rules

- Selection locks by `session_id`. A same operation/leader/route/participant
  retry returns the stored resolution and ignores a new candidate seed.
- A different operation, route, leader, activity, or participant set for an
  occupied session is an idempotency conflict and writes nothing.
- Invalid identity, catalog, phase, capability, or business input is rejected
  before insertion and leaves the session/operation identity unreserved.
- The participant rows and parent selection commit together. No accepted row
  may be exposed in memory before commit succeeds.
- At most one accepted operation exists per session; no reroll or second event
  row exists.

## Terminal transaction rules

- Success locks the selected row, reconstructs and validates the exact stored
  selection, persists its summary, and applies existing per-session/per-
  character inventory/progression grants plus activity history in one
  transaction.
- Timeout failure locks and validates the same selection, persists only the
  no-reward terminal summary, and writes no activity history or reward grant.
- An exact terminal retry returns the stored summary without another grant or
  history row. A different outcome or elapsed duration is a conflict.
- Failure injection at participant insertion, terminal update, each reward
  boundary, history, and commit must leave selection/terminal/reward/history
  state equal to its pre-request state.

## Mandatory migration sequence

1. Preserve a custom-format dump, schema, table inventory/counts, and stable
   ordered JSONL for all twelve legacy tables outside the repository.
2. Verify the evidence manifest and `pg_restore --list`.
3. Restore into one explicitly named disposable database.
4. Compare every restored legacy export byte-for-byte before DDL.
5. Apply the complete schema twice in the disposable database; require exactly
   fourteen tables, zero route rows, and unchanged legacy exports.
6. Run all persistence/concurrency/failure tests against the disposable copy.
7. Only then re-export and compare the working database, apply the schema twice,
   and prove the twelve legacy exports remain byte-identical and both new
   tables start empty.

The working database, its volume, prior evidence, and old rows must never be
deleted. Disposable database cleanup is allowed only by its resolved literal
name after its recoverable evidence is accepted.
