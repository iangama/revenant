# M26 Block 3 authoritative module-domain audit

Date: 2026-08-31  
Gate: 3 — pure ownership, combination, whole-loadout, revision, and idempotency.  
Decision: **approved**.

## Domain transition contract

`revenant-modules` now owns a pure `ModuleState` containing fragment quantity,
unique module ownership, canonical zero-to-three loadout, monotonic revision,
and separate bounded accepted-operation ledgers for combination and loadout.
It has no SQL, socket, replay, gateway, protocol, clock, or client dependency.

- Operation IDs contain 1-32 ASCII alphanumeric/hyphen characters.
- Only a participant in authoritative `Complete` may create a new mutation.
- A combination subtracts the frozen two-fragment recipe and adds one unique
  unlock as one state replacement.
- A loadout request replaces the entire canonical set and increments revision
  exactly once; individual slot writes do not exist.
- Once accepted, the same kind/ID/canonical input returns the original outcome
  as `replayed`, even if lifecycle has since changed. Different input under the
  reserved ID is a conflict.
- Rejections are stateless: they reserve no ID and may be evaluated again after
  fragments, ownership, lifecycle, or revision changes.
- Each accepted-operation ledger is capped at 128 rows per character/kind. The
  fixed combination catalog can naturally accept at most four unique grants;
  loadout mutation explicitly refuses a 129th accepted operation.
- Every error returns before assigning a cloned candidate state. Fragment,
  ownership, loadout, revision, and both ledgers therefore remain identical.

Persistence must hydrate/check durable accepted-operation records in Block 4;
the current in-memory ledger proves the transition semantics but is not
misrepresented as cross-process durability.

## Generated evidence

The accepted deterministic domain report is
`/tmp/revenant-m26-block3.json`, schema `RevenantModuleDomainLabV1`. It is
17,510 bytes and has SHA-256
`a2788dae84a5ae6cad607fe7b234ff3ddcf2c87fd75b5d711a6bd2dd22acd17e`.
A fresh second run was byte-identical.

The 34 cases comprise:

- 15 of 15 canonical loadout replacements from fully owned state;
- exact combination apply, same-operation replay, conflict, insufficient
  fragments, already-owned, active-state lock, and malformed ID;
- non-canonical whole-loadout apply, reordered canonical replay, and
  same-operation conflict;
- active lock, stale revision, unowned module, duplicate, fourth entry,
  malformed ID, unchanged state, revision overflow, and operation-limit
  rejection; and
- 128 alternating accepted loadout changes followed by one atomic limit
  rejection at revision/ledger count 128.

Aggregate assertions pass:

| Assertion | Result |
| --- | ---: |
| Total generated cases | 34 |
| Accepted cases | 19 |
| Rejected cases | 15 |
| Same-operation replays | 2 |
| Valid canonical loadouts applied | 15 |
| Rejections with complete state equality | 15/15 |
| Accepted-operation limit exercised | 128 |

The Block 2 `--json` output remained byte-identical after the domain extension,
retaining SHA-256
`b837554d10a90a134cf76e207c586d1cffde357c7594cd429ae434aecb80cd04`.
Block 3 evidence uses a separate `--domain-json` mode so later transition
coverage does not silently rewrite the accepted arithmetic artifact.

## Focused and workspace quality evidence

The focused suite passes 12 module-domain tests and five CLI/laboratory tests.
It covers complete state-space construction, exact arithmetic, catalog
validation/order, request-order invariance, malformed identity, phase lock,
insufficient/already-owned, same-ID replay/conflict, whole-loadout canonical
replacement, all 15 subsets, stale/unowned/duplicate/capacity rejection,
revision overflow, 128-row bound, full-state equality on error, and stable JSON.

Using the official temporary Rust 1.98.0 toolchain, the final block passed:

- `cargo fmt --all -- --check`;
- workspace Clippy for all targets with warnings denied;
- all 74 workspace unit, PostgreSQL integration, compatibility, CLI, and doc
  tests; and
- `cargo build --workspace --all-targets`.

`git diff --check` is green. Block 3 did not edit Protocol, replay,
persistence, gateway, Inspector, client, frozen V1, `VERSION`, or the M24
package.

## Rejected pre-gate attempts

1. All 12 domain tests first passed, but Clippy rejected assignment from
   `modules.clone()` where `clone_from` reuses the allocation. The mechanical
   correction was applied before evidence generation.
2. All 17 focused tests then passed, but Clippy rejected the 228-line domain
   report builder. It was split into bounded combination, valid-loadout,
   idempotency, invalid-loadout, and limit generators. The accepted 34-case
   output was generated only after that refactor and clean Clippy.

Neither rejected command changed schema, persistence, replay, protocol, or
runtime state.

## Residuals entering Block 4

- Accepted operation ledgers are in-memory domain state only. PostgreSQL must
  become durable authority and return stored results across reconnect/restart.
- Rejections intentionally write no operation row. A later request using that
  still-unreserved ID is evaluated against current authoritative preconditions.
- The 128 accepted-loadout lifetime cap is explicit prototype growth control.
  Raising/pruning it requires a future reviewed migration; silent eviction is
  forbidden because it could weaken retry identity.
- No module has been granted to a real account, persisted, replayed, sent over
  Protocol V2, applied in gateway combat, or presented by Godot.

## Gate 3 decision

Gate 3 is **approved**. Ownership, fixed-recipe combination, complete canonical
loadout replacement, lifecycle lock, monotonic revision, accepted-operation
idempotency/conflict, bounded growth, and atomic rejection behavior satisfy the
pure-domain contract. Block 4 may begin with exact DDL review and disposable
backup/restore proof. No working-database migration is authorized before that
proof passes, and replay/protocol/gateway/client work remains behind later
gates.
