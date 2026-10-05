# M26 Block 4 migration and transactional-persistence audit

Date: 2026-08-31  
Gate: 4 — additive schema, migration, durable module state, and rollback.  
Decision: **approved for the persistence boundary**.

## Additive schema decision

`runtime/persistence/migrations/0006_module_progression.sql` adds exactly three
tables and does not alter, delete, rename, backfill, or reinterpret an existing
row:

- `module_states` owns one non-negative monotonic revision per materialized
  character state;
- `module_loadout_slots` owns at most three indexed, unique, inventory-backed
  module entries per character; and
- `module_operations` owns bounded structured request/result JSON for accepted
  `combine` and `loadout` operation identifiers.

Legacy characters materialize as empty revision-zero state when first needed.
No migration row, module, fragment, slot, or operation was synthesized. The
migration remains inside the existing advisory schema transaction, and the
additive tables are inert for a pre-M26 gateway.

The accepted source hashes are recorded in the Block 4 evidence manifest. The
migration itself has SHA-256
`f7e0799db89c50b7d4e17d859fceb8ba1fd9c154eab542863785e379aeab19d2`.

## Pre-migration and disposable proof

Before applying `0006` anywhere, a PostgreSQL custom-format dump, schema,
table counts, and stable ordered JSONL exports of all nine legacy tables were
written to:

`/mnt/c/Users/Ian/revenant-local-evidence/m26-block4-pre-migration-20260831`

The directory is 1,909,293 bytes. Its `SHA256SUMS` file has SHA-256
`802434155be962061e9747ca0a4e1676f1836a786fcd1546cda6d9d6cf1c7922`,
and every entry verifies. The custom dump has SHA-256
`5a849d3aa5bb006adf14a4551e14102d9f148d678f86c218ad028bbb6f24b95a`
and passed `pg_restore --list` before use.

The dump was restored into the exact disposable database
`revenant_m26_disposable_0831`. All nine ordered table exports compared byte
identical with their sources before DDL. The complete schema was then applied
twice successfully. It contained 12 tables, zero module rows, and byte-identical
legacy exports after both applications.

Only after that proof, the nine working-database legacy exports were compared
again with the pre-migration evidence, the same schema was applied twice to the
working database, and all nine exports remained byte identical. Immediately
after migration, its new-table counts were exactly zero. The accepted
post-migration evidence is:

`/mnt/c/Users/Ian/revenant-local-evidence/m26-block4-accepted-20260831`

It is 1,806,752 bytes. Its `SHA256SUMS` file has SHA-256
`b6e23447f48cce8a195f7c209f798e6d103c6d46b16026c3736aaca4d93f052e`,
and every entry verifies. It includes the resulting schema, table counts,
`\d` descriptions of all three new tables, all nine post-migration ordered
exports, disposable final counts, and accepted source hashes.

After all accepted evidence was preserved, the disposable database was
removed by its literal name and its absence was queried. This deletion affected
only the generated test database and is recoverable from the retained dump; it
did not remove the working database, PostgreSQL volume, or any evidence.

## Durable transition behavior

`revenant-persistence` now loads and validates fragments, unique fixed-catalog
module ownership, contiguous canonical slots, revision, and accepted-operation
counts into the pure `ModuleState`. It exposes typed state, combination, and
whole-loadout operations without admitting protocol or gateway policy.

- Combination locks/materializes character module state, loads the existing
  accepted operation before lifecycle evaluation, validates with the pure
  domain, subtracts the exact fragment cost, grants one inventory-backed
  module, and stores the accepted result in one transaction.
- Loadout replacement canonicalizes before identity comparison, validates
  ownership/revision/lifecycle, deletes the old whole set, inserts the new
  contiguous set, advances revision by one compare-and-set update, and stores
  the accepted result in one transaction.
- Same kind/ID/canonical input returns the stored result after a reconnect
  without a second consume, grant, slot write, or revision. Different input is
  a conflict.
- Business rejection rolls back lazy state materialization and writes no
  operation. The still-unreserved ID may later succeed after its preconditions
  change.
- Each operation kind is capped at 128 accepted rows. A new 129th loadout is
  refused while an older accepted identifier still replays exactly.
- Persisted rows outside the four-entry catalog, module quantity one,
  contiguous/canonical slot contract, signed revision range, or operation
  bounds fail closed as invalid stored state.

## PostgreSQL integration and failure evidence

Six real PostgreSQL tests passed against the restored disposable database and
again in the final workspace gate:

1. complete combination/loadout transitions, rejection reuse, durable retry,
   conflict, canonicalization, revision, and reconnect equality;
2. two concurrent connections using the same combination operation, yielding
   exactly one apply, one replay, one consume, and one grant;
3. 128 accepted loadout operations, 129th rejection, and preserved old replay;
4. combination rollback at state insert, fragment update, module inventory
   insert, and operation insert;
5. loadout rollback at slot insert, revision update, and operation insert; and
6. loadout rollback at old-slot deletion.

Each injected database failure left the complete observed module state equal
to its pre-request state. Conditional failure triggers/functions were removed
by fixture cleanup. The final working database query found zero such trigger
or function. Its module counts after the workspace gate were seven states, two
slots, and 141 accepted synthetic test operations; the zero-row
immediately-post-migration state is preserved separately in accepted evidence.

Using the temporary Rust 1.98.0 toolchain, the final block passed:

- `cargo fmt --all -- --check`;
- workspace Clippy for all targets with warnings denied;
- all 80 workspace unit, PostgreSQL integration, compatibility, CLI, and doc
  tests; and
- `cargo build --workspace --all-targets`.

## Rejected pre-gate attempts

1. The first persistence implementation passed focused tests, but Clippy
   rejected an overlong persisted-state loader. It was split by inventory,
   loadout, and operation-count responsibility before integration evidence.
2. The first real disposable integration run found a PostgreSQL driver type
   mismatch: serialized JSON text had been bound directly to a `JSONB`
   parameter. Four tests failed before their transactions committed; the one
   earlier rollback fixture passed. A database query confirmed zero committed
   module state, slots, operations, or module inventory and zero remaining
   failure fixtures. The insert now casts bounded text explicitly with
   `TEXT::JSONB`.
3. Subsequent tests passed, but Clippy rejected an overlong integration-test
   body and one needless pass-by-value helper. The test was split and the
   helper narrowed before accepted evidence.
4. A host-side cleanup command failed before execution because `psql` was not
   installed on the host. No database changed. The target was resolved and
   removed through the healthy local PostgreSQL container instead.

No rejected attempt changed replay vocabulary, Protocol V2, frozen V1,
gateway, Inspector, Godot, `VERSION`, or the frozen M24 package.

## Deliberate boundary entering Block 5

Block 4 proves atomic durable module mutations and operation identity, but it
does **not** yet claim mutation/replay atomicity. No new replay kind or replay
row was added here. The milestone deliberately reserves exact event names,
payloads, independent reconstruction, legacy fallback, corruption rejection,
and transaction coupling for Block 5. Until Gate 5 is green, module persistence
is not exposed through the gateway or client and is not a complete runtime
feature.

## Gate 4 decision

Gate 4 is **approved**. Additive DDL, pre-migration backup, disposable restore,
apply-twice behavior, byte-exact legacy preservation, lazy legacy state,
durable accepted-operation identity, concurrent retry, bounded growth, and
rollback at every current persistence write boundary satisfy Block 4. Block 5
may begin by reviewing exact replay vocabulary and bounded payload structs.
Protocol, gateway, Inspector implementation, and client work remain locked
behind their ordered gates.
