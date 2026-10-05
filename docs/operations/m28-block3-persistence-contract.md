# M28 Block 3 — additive persistence contract review

Date: 2026-09-06  
Status: **exact schema and transaction shapes reviewed; DDL not yet applied**

## Boundary decision

Migration `0008_cooperation_operations.sql` must be additive and idempotent. It
may create exactly two tables and one account/time index. It may not alter,
rename, delete, backfill, reinterpret, or add a trigger/function to any of the
fourteen M27 tables. No replay event kind or payload is authorized in Block 3.

`cooperation_operations` owns at most one accepted cooperation operation per
session. Its primary key is `session_id`; `start_operation_id`, the two exact
ordered participant snapshots, and every fixed contract value form the
canonical start input/result. The parent row stores:

- exact activity `relay_awakening`, catalog `m28-v1`, anchor account, and
  participant count two;
- exact fixed values: operation 60,000 ms, ping 5,000 ms, revive window 15,000
  ms, channel 2,000 ms, revive health 50, distance-squared limit four,
  `relay_core_fragment` quantity two, and 125 XP per participant;
- last nonterminal phase, five contribution flags, and paired nullable elapsed
  fields for anchor arrival, ping, runner arrival/downing, revive start/
  completion, and Warden completion;
- the one optional ping ID and live target `relay_console`;
- the one optional active/completed revive ID, revive count zero/one, and fixed
  revive target `runner`; and
- an optional terminal outcome/elapsed/audit timestamp that is all-null or
  all-present.

The stored `phase` is deliberately the last durable nonterminal phase. A row
with no terminal is active/incomplete. A terminal row retains that phase and
derives domain `succeeded`/`failed` from `terminal_outcome`; it therefore
preserves the exact phase reached before timeout, defeat, abandonment, or
crash.

`cooperation_operation_participants` contains exactly two admission-order
rows. Index zero must be role `anchor` and match the parent's anchor account;
index one must be `runner`. Each row freezes unique account, character, actor,
weapon, canonical module-loadout JSON, effective damage/range/cooldown/maximum
health, admitted current health, mutable current health, and active/downed/
defeated life state. Application validation must additionally prove character
ownership, exact 0/1 row cardinality, role/index pairing, canonical M26 module
resolution, and parent/participant state parity while rows are locked.

## SQL constraint shape

The migration must enforce all locally expressible facts:

- bounded storage IDs and 1-32-byte ASCII alphanumeric/hyphen start/ping/revive
  operation IDs;
- exactly two participants, exact revisions/constants/reward, fixed targets,
  finite phase/outcome/life enums, boolean flags, and revive count 0-1;
- paired nullability for each contribution and its elapsed/operation evidence;
- a strict lifecycle prefix: awaiting-anchor has no contribution;
  awaiting-ping has anchor only; awaiting-runner adds ping; runner-downed adds
  runner/downing; revive-channel adds a live revive start; encounter-active adds
  completed revive, while Warden completion exists only with success;
- elapsed values non-negative and ordered; runner arrival is at or before the
  ping's inclusive 5,000 ms boundary; revive starts/completes at or before the
  inclusive 15,000 ms window, completes at least 2,000 ms after channel start,
  every nonterminal transition is at or before 60,000 ms, and success/Warden
  completion is at or before 60,000 ms;
- success requires every contribution, active participants, revive count one,
  Warden evidence may exist only with success, and terminal elapsed is ordered
  after every retained transition and equals Warden completion on success;
- ping/revive/operation timeout outcomes are strictly past their matching
  deadline and retain the matching last phase; overall timeout is allowed only
  when the active ping/revive subdeadline has not already won precedence;
  participant defeat and abandonment occur only while the relevant overall/
  subdeadline is still active; and
- per-session account, character, and actor uniqueness plus health within
  zero..maximum and admitted health within one..maximum.

Cross-row role/cardinality/ownership/profile/life checks cannot be expressed
without triggers, so they remain fail-closed application validation under row
locks. Triggers and stored functions are explicitly forbidden.

## Transaction shapes

Every mutation uses one PostgreSQL transaction plus the existing
transaction-scoped advisory lock derived from bounded `session_id`. Nothing is
projected into gateway memory before commit.

### Start

1. Validate the exact two input snapshots through the pure `m28-v1` domain and
   canonical M26 build resolver.
2. Acquire the session advisory lock; lock both accounts/characters and verify
   ownership.
3. If a parent exists, lock/load both participants. Exact operation/activity/
   participant/profile/health retry returns the stored result; any difference
   is an idempotency conflict.
4. Otherwise insert the awaiting-anchor parent and both ordered participants.
5. Reload/validate the complete stored state and commit.

A rejected request reserves no session or operation ID. Parent and both
participants are never partially visible.

### Anchor, ping, and scripted downing

- Each transition locks and reconstructs the current state through the pure
  domain before mutation.
- A repeated anchor/runner observation checks the currently observed deadline
  before returning its stored transition, so a late retry commits the same
  deterministic timeout as any other active command.
- Anchor arrival updates only the phase, anchor flag, and elapsed field.
- Ping stores the one ID/target/time and advances phase. Same ID/input returns
  the stored result; a different or malformed ID writes nothing.
- Runner arrival stores the runner/downing flag/time, sets phase
  runner-downed, and atomically changes only the runner from its exact current
  health to zero/downed. Re-observation returns the stored transition only when
  its canonical elapsed/input matches; conflicting/order/identity input writes
  nothing.

### Revive start, cancellation, and completion

- Start stores the ID/time and revive-channel phase only after authority,
  target, life, range, deadline, and ID validation.
- Same active ID/input replays. A different active ID conflicts.
- Out-of-range channel observation atomically returns to runner-downed and
  clears the active ID/time. It therefore does not reserve that ID and permits
  a later valid reuse without increasing any counter.
- Completion locks/reconstructs the same channel, validates the inclusive
  window and 2,000 ms duration, changes the runner to exactly 50/active, sets
  revive count one and contribution true, stores completion time, and advances
  encounter-active atomically. Exact retry returns the stored result; any new
  revive rejects.

### Terminal

- Every request observes phase-specific deadlines before the overall deadline.
- Ping, revive, operation-timeout, participant-defeat, and disconnect failure
  update only the terminal tuple and, for defeat, the one participant's health/
  life. They write no history or reward.
- Success requires the complete stored prefix and locks both participants. It
  writes Warden/terminal evidence and applies the existing per-session/per-
  character inventory grant/inventory, progression grant/progression/level,
  and activity history for both participants in that same transaction.
- Exact terminal retry returns the stored outcome and grants/appends nothing.
  Any different outcome, elapsed value, participant, or incomplete/partial
  pre-existing grant is a conflict and writes nothing.

## Mandatory failure and concurrency proof

Failure injection must cover parent and each participant start insert; anchor,
ping, down-parent, down-participant, revive-start, cancellation,
revive-completion-parent, revive-completion-participant, terminal update, and
each existing reward/history write boundary. Every injected failure must leave
all cooperation, inventory, progression, grants, history, and replay state
equal to its pre-request snapshot, with no retained trigger/function.

Two independent connections must prove same-start, same-ping, same-revive, and
same-success convergence: exactly one applied receipt, one replayed receipt,
one canonical row/result, one revive, two equal participant grants, and no
duplicate history. Conflicting concurrent input must produce one winner and one
fail-closed conflict without mixed state.

## Mandatory migration sequence

1. Export a custom-format dump, schema, counts, and stable ordered JSONL for
   all fourteen M27 tables outside the repository; checksum and verify
   `pg_restore --list`.
2. Restore into one explicitly named and previously absent disposable database;
   byte-compare all fourteen exports before DDL.
3. Apply the complete schema twice under the migration advisory lock; require
   exactly sixteen tables, zero cooperation rows, and byte-identical legacy
   exports after each application.
4. Run the complete focused migration, constraint, idempotency, rollback, and
   concurrency suite only against the disposable database first.
5. Re-export and byte-compare the working database to the pre-migration
   evidence, then apply the schema twice and prove both new tables start empty
   without a trigger/function.
6. Preserve all evidence. Remove only the literal disposable database after
   its accepted final counts and recoverability evidence are recorded.

The working database, PostgreSQL volume, checkpoint, prior evidence, and any
legacy row may not be deleted.

## Review decision

The schema and transaction contract is finite and approved for implementation.
Only the exact additive migration, persistence API, disposable/working safety
sequence, and Gate-3 tests above are now authorized. Replay event names,
payloads, reconstruction, Inspector, protocol, gateway, bot, and Godot changes
remain locked.
