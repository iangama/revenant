# M27 Block 4 — migration and transactional persistence audit

Date: 2026-09-01  
Decision: **Gate 4 approved for the persistence boundary**

## Additive schema decision

`runtime/persistence/migrations/0007_route_operations.sql` has SHA-256
`8f5a3bcf9d22e6b9b3bcfd7175a47365ed2b27ef59003d84586d781e35f55b6a`.
It creates exactly `route_operations`, `route_operation_participants`, and one
leader/time index. It alters, deletes, renames, backfills, and reinterprets no
M26 object or row.

The parent primary key permits one accepted operation per session. SQL checks
freeze exact revision, route/event pairing, effect, reward, objective path,
90,000 ms budget, success boundary, and strictly-post-budget timeout. One or
two participant rows preserve admission order and unique account, character,
and actor identities with account/character foreign keys. Application loading
also verifies participant cardinality, leader-first order, character ownership,
the deterministic seed resolver, and every frozen catalog value.

The reviewed design and migration sequence are recorded in
`docs/operations/m27-block4-persistence-contract.md`.

## Pre-migration backup

Before applying `0007`, all twelve working legacy tables were preserved as a
PostgreSQL custom-format dump, schema, counts, and stable ordered JSONL at:

`/mnt/c/Users/Ian/revenant-local-evidence/m27-block4-pre-migration-20260901`

- size: 3,893,879 bytes;
- dump SHA-256:
  `fe573546dd6e700604ef43ebef9ad8eabc84c007f19936733ff158d899e1623d`;
- verified `SHA256SUMS` SHA-256:
  `7638979ef377c6197f9e41f46080cfd7d14c68a0b5645625caa3305f6aa39c2c`;
- `pg_restore --list`: passed.

The snapshot contains 431 accounts, 431 characters, 1,285 inventory rows, 431
progression rows, 790 histories, 6,725 replay rows, 716 inventory grants, 705
progression grants, 407 equipment loadouts, 89 module states, 52 module slots,
and 1,105 module operations.

## Disposable restore and apply-twice proof

The dump restored into the explicitly checked-absent database
`revenant_m27_disposable_0901`. All twelve restored JSONL files matched the
source byte-for-byte before DDL. The complete schema ran twice inside an
advisory-locked transaction and produced fourteen tables, zero route-operation
rows, zero participant rows, and byte-identical legacy exports after both
runs.

Disposable evidence is at:

`/mnt/c/Users/Ian/revenant-local-evidence/m27-block4-disposable-20260901`

Its verified manifest SHA-256 is
`3eba4c9ed9799eb406f497e9fd4f047e829a0029d26f3cc67fa2fe422a7106e9`.

## Durable operation behavior

Selection uses a transaction-scoped advisory lock derived from the bounded
session ID. It validates participant account/character ownership under row
locks, resolves the pure Gate 2 domain, inserts the parent and ordered
participants, and exposes success only after commit.

- same session/operation/leader/route/participants replays the stored result
  and ignores a different retry seed;
- any occupied-session input difference is a conflict with no write;
- leader/business/identity rejection leaves the session and operation ID free;
- two concurrent connections produce one apply, one replay, one row, one seed,
  and one event; and
- stored values are reconstructed and checked against today's frozen
  revision/resolver/catalog before use.

Terminal success locks and validates the selection, applies existing
per-session/per-character inventory and progression grants plus history for
all participants, and writes the summary in one transaction. Exact retry
returns the stored summary; conflicting terminal input rejects. Timeout at the
budget rejects; the first millisecond after it commits failure with no grant or
history. A pre-existing partial grant without terminal evidence fails closed.

Nine injected write boundaries covered parent selection, participant
selection, inventory grant/inventory, progression grant/progression, character
level, activity history, and terminal update. Each failure left all observed
selection, participant, reward, progression, history, and terminal state equal
to its pre-request state. Direct SQL attempts to drift event/route, reward,
objective path, or terminal deadline also failed their constraints and left
the canonical selection readable.

## Working database migration

Immediately before migration, a fresh working export again matched all twelve
backup JSONL files byte-for-byte. `Persistence::connect` then applied the
schema through the existing advisory transaction. Two accepted runner passes
proved idempotency. Final working evidence is at:

`/mnt/c/Users/Ian/revenant-local-evidence/m27-block4-accepted-20260901`

- size: 3,685,470 bytes;
- verified manifest SHA-256:
  `4c61ee012408683641001038f9a6c78c212f24458bd15670614be67fd848d82e`;
- final table count: fourteen;
- new-table counts: `0|0`;
- all twelve legacy exports: byte-identical to pre-migration; and
- leftover M27 failure triggers/functions: `0|0` in working and disposable
  databases.

After recording disposable final counts `52|59|0|0` (synthetic operations,
participants, failure triggers, and failure functions), the exact generated
database `revenant_m27_disposable_0901` was removed. Its absence was verified;
the recoverable custom dump, evidence directories, working `revenant` database,
and PostgreSQL volume were not removed.

## Quality and protected-boundary evidence

- six focused route-operation PostgreSQL cases passed;
- all 23 persistence unit/PostgreSQL cases passed;
- workspace format and all-target warnings-denied Clippy passed;
- all 140 workspace tests passed against the disposable database;
- workspace all-target build passed;
- accepted test-log SHA-256:
  `fcd8dda8ed58028b72046177984875ba47e1cf4e5354e4f6b939aa3c037c76cb`;
  and
- 107 files under `VERSION`, frozen V1, Godot, gateway, Protocol, and replay
  were byte-identical to the pre-M27 checkpoint. The protected path-list
  SHA-256 is
  `2307891c940d4bdf6099008ad13d05e85e5a09e1e7b9fb7066445d96e226f954`.

## Rejected attempts

The first compile rejected an optional-terminal ownership mistake. The first
strict lint passes then rejected field naming, one overlong loader, a needless
owned argument, and a duplicated test branch; each was refactored without an
allow. No database operation accompanied those failures.

The first working `persistence-check` did apply the already-proven schema, then
rejected because its default historical bot lacked the sidearm expected by the
diagnostic. The two new tables remained empty and no gameplay row changed. A
read-only query selected `local:m24-block5-abrupt`, which satisfies every
diagnostic invariant; two complete runner passes then succeeded. This rejected
diagnostic run is not accepted idempotency evidence.

## Gate decision and replay boundary

Gate 4 is green for additive migration, restore, legacy preservation, durable
selection, concurrent retry, exact terminal/reward transactions, and rollback.
No replay kind or payload was invented here because Block 5 separately owns
that vocabulary review. Consequently this gate does not yet claim
selection/replay or terminal/replay atomicity, Inspector reconstruction, or a
live gateway route. Block 5 must add its reviewed events inside these same
transactions and repeat every failure boundary before Gate 5 can pass.

Only M27 Block 5 exact replay vocabulary/payload review, atomic append,
independent reconstruction, corruption rejection, legacy fallback, and
read-only Inspector evidence are now authorized. Protocol, gateway, and client
work remain locked.
