# M26 closure audit

Date: 2026-09-01  
Milestone: M26 — fragments, modules, and horizontal progression.  
Decision: **Gate M26 approved with bounded residual risk (green)**.

## Starting hypothesis result

M26 began with one engineering hypothesis: a bounded three-slot/four-family
module loadout could provide at least three non-dominated builds while all
inventory, mutation, persistence, replay, and combat arithmetic remained
authoritative, idempotent, and reconstructible.

The hypothesis is supported inside its stated engineering boundary. The fixed
catalog produces exactly 15 canonical loadouts, all 15 are Pareto
non-dominated for both weapons, all resolved profiles remain inside the frozen
combat/pressure envelope, and the final runtime sample admitted Empty, Force,
Ward, and Force+Ward exactly for both weapons. No result is evidence that a
person prefers, understands, or will retain interest in those builds.

## Gate-by-gate closure

| Gate | Accepted result |
| --- | --- |
| 1 — specification | One falsifiable hypothesis; four entries, three slots, 15 builds, finite arithmetic/lifecycle/growth bounds, explicit later change gates. |
| 2 — arithmetic | Deterministic 30-build report; fixed `m26-v1` catalog; all 15 builds per weapon non-dominated; non-lethal bounded pressure. |
| 3 — domain | Atomic fixed-recipe combination, canonical whole-loadout replacement, revision, lifecycle lock, retry/conflict, and 128-operation bound. |
| 4 — persistence | Additive apply-twice migration, pre-migration backup/restore, byte-identical legacy preservation, durable concurrency/idempotency, and rollback at every write boundary. |
| 5 — replay/Inspector | Three bounded structured events, independent V1/V2/legacy reconstruction, corruption rejection, mutation/replay atomicity, CLI parity, and GET-only Inspector decoding. |
| 6 — Protocol/gateway | Eight opt-in V2 variants, bounded frames, immutable admitted state, complete-only mutation, next-session activation, old-V2 sequencing, and frozen-V1 base projection. |
| 7 — Godot workshop | Server-only preview/arithmetic, honest pending/result/reconnect state, keyboard focus, bounded identifiers, semantic fixtures, and real graphical evidence. |
| 8 — runtime/resources | 27 sessions/35 active completions, fresh/reused accounts, both weapons/four sampled builds, distinct two-active builds, exact state/reward/replay/Inspector/V1/reset/resource reconciliation. |

The detailed decisions remain in the Block 2-8 audits under
`docs/progression`. No gate was retroactively broadened to justify work that
belonged to a later block.

## Complete evidence review

The accepted closure inputs were re-read and reverified after Block 8:

| Evidence | Closure verification |
| --- | --- |
| M25 pre-M26 checkpoint | 9,841,452-byte archive; SHA-256 `25e5eed51c11534569f0daf550af05990f927060c6c5b6766bc62fc22777ea83`. |
| Gate 2 report | Regenerated twice from final source; SHA-256 `b837554d10a90a134cf76e207c586d1cffde357c7594cd429ae434aecb80cd04`. |
| Gate 3 report | Regenerated twice from final source; SHA-256 `a2788dae84a5ae6cad607fe7b234ff3ddcf2c87fd75b5d711a6bd2dd22acd17e`. |
| Gate 4 pre-migration | 13-file manifest `802434155be962061e9747ca0a4e1676f1836a786fcd1546cda6d9d6cf1c7922`; recoverable dump `5a849d3aa5bb006adf14a4551e14102d9f148d678f86c218ad028bbb6f24b95a`. |
| Gate 4 accepted migration | 15-file manifest `b6e23447f48cce8a195f7c209f798e6d103c6d46b16026c3736aaca4d93f052e`. |
| Gate 5 Inspector/replay | Seven-file manifest `9c0f76e2e7fa8e7492f4efbfa7611b2469fa82865cf49c5a1b5f5627a00517cc`. |
| Gate 6 gateway | Four-file manifest `7d92807cc61867bacaf0c1378b1a7b523eb0cd08b479ee1e0163b43b58973487`. |
| Gate 7 captures | Both PNG hashes and `docs/art/m26/captures/SHA256SUMS` pass. |
| Gate 8 runtime | 125 files/697,770 bytes; manifest `803571b7d9664b3f60b46d924485abeabd06c47260488c76e67eefa9d5212a5b`. |

The ephemeral original Gate 2/3 `/tmp` report files no longer existed, which
is expected for temporary output. Regeneration from the final binary was
byte-deterministic including the trailing newline and reproduced both accepted
digests exactly.

## Complete implementation and authority review

The final M26 implementation remains composed through explicit boundaries:

- `revenant-modules` owns the fixed catalog, checked arithmetic, canonical
  loadouts, ownership/revision/idempotency domain, and growth limits without a
  database, network, client, or clock dependency.
- PostgreSQL owns durable inventory, three bounded slots, monotonic revision,
  accepted operation identity, and replay-coupled transactions. The working
  database has zero negative revisions, invalid slot positions, malformed
  operation IDs, oversized operation payloads, or non-unit module quantities.
- Replay evidence embeds its fixed catalog and before/after arithmetic;
  reconstruction does not trust current mutable state. Inspector exposes only
  the three existing GET routes and no mutation surface.
- Protocol remains version 2 with the existing 64 KiB frame ceiling. Module
  requests are opt-in and bounded before domain/persistence mutation. Ordinary
  old-V2 admission remains sequence-compatible.
- Gateway admission copies a resolved immutable profile. Active/completed
  actors are never rewritten by a later loadout, and each multiplayer actor
  attacks from its own admitted state.
- Godot contains no module stat formula or local success authority. It formats
  server snapshots/previews/results and explicitly labels future-session
  activation/current-actor immutability.
- Runtime matrix support is confined to the fake client and reproducible test
  script. It creates accepted state through activities/protocol, not direct SQL
  fixture mutation.

No unresolved TODO/FIXME/unimplemented marker exists in the reviewed M26
surface. The only reviewed `panic!` matches are explicit test-fixture failure
branches. No taxonomy, protocol generation, schema table, replay kind, package,
or version was added outside its approved block.

## Final uninterrupted gate

After the last implementation source change and the accepted Gate 8 run, the
integrated smoke passed:

- shared two-active completion and bounded hostile pressure;
- automatic/reused, manual, and keyboard-only Godot activities;
- every M17-M26 semantic marker;
- exact persistence, replay, and current Inspector reconciliation; and
- the byte-frozen V1 client through both the current adapter and standalone
  reconstructed V1 backend.

The final technical command passed formatting, workspace/all-target/all-feature
Clippy with warnings denied, all 98 Rust/PostgreSQL tests, every-target build,
Inspector TypeScript check, and production Vite build. Post-gate review then
passed:

- `VERSION=0.2.0` consistency, the 312-file implementation-source secret
  audit, and the final docs-only 314-file recheck;
- M21/M22/M24/M25/M26 capture manifests;
- exact frozen-V1 hashes and no diff under `archive/clients/v1`;
- unchanged M24 unpublished archive size/hash and adjacent checksum;
- no diff under `VERSION` or pre-M26 migrations;
- zero `m24_fail_%` PostgreSQL trigger/function;
- loopback-only host exposure and healthy canonical services;
- no validation process, isolated listener, or generated untracked Godot UID;
- no tag at HEAD, no commit/release/public artifact, `git diff --check`, and
  `HEAD == origin/main == 4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`.

The long-lived Compose gateway image predates M26. It remains healthy and
host-loopback-only but was not used as current M26 evidence; every accepted M26
runtime/Inspector claim used the freshly built isolated binary.

## Rejected evidence synthesis

Rejected attempts across M26 fall into bounded categories and are retained in
their block audits:

- compile/lint/test-structure corrections before evidence generation;
- mismatched serialized module identity and invalid global replay-ID
  adjacency assumptions;
- PostgreSQL JSONB binding and injected-write rollback fixtures;
- an admission-time fragment projection defect found after a valid reward;
- a Godot signed-integer/string-array MessagePack defect found on the real
  wire;
- superseded graphical captures after normal-loop, keyboard, reconnect, and
  operation-ID corrections;
- the reproduced WSL wall-clock step rejected for Inspector timing evidence;
  and
- three Gate 8 harness expectations corrected before a fresh accepted prefix.

No rejected attempt is silently included in an accepted manifest or presented
as success. Where gameplay completed before a harness rejected its own
evidence, the atomic synthetic reward remains valid database state rather than
being deleted or misreported as rollback.

## Bounded residual risk

- The four runtime builds are a finite sample of the exhaustive 15-build pure
  matrix. Deterministic tests, not runtime sampling, prove the complete state
  space and Pareto result.
- Engineering non-dominance and creator-operated automation prove no human
  preference, onboarding comprehension, accessibility across bodies/hardware,
  economy longevity, or retention.
- The 128 accepted-loadout-operation lifetime cap is explicit prototype growth
  control. Any pruning, expansion, or new catalog entry requires a reviewed
  future schema/domain decision; silent eviction remains forbidden.
- M26 has no dismantle, refund, trade, transfer, random roll, tier, rarity,
  second currency, or public service. Those omissions are intentional, not
  implied future commitments.
- The WSL/PostgreSQL wall-clock-step residual remains possible. Append IDs are
  authoritative for replay order, while Inspector correctly fails impossible
  elapsed projections. All accepted Gate 8 timings were monotonic.
- Hardware audio perception and external-machine presentation remain untested;
  mute, Reduced Flash, fixed-pool, semantic, capture, and current-host behavior
  are the bounded evidence actually held.
- The frozen M24 package remains an unpublished 0.2.0 artifact and does not
  contain M25/M26. No M26 release/package claim is made.

None of these residuals is a reproducible integrity, authority,
dominance-bound, transaction, reconstruction, compatibility, privacy,
resource, or recovery blocker for the local solo M26 scope.

## Gate M26 decision

Gate M26 is **approved with bounded residual risk (green)**. The exact
four-entry catalog and 15-build bound are frozen; at least three builds per
weapon are non-dominated; fragment/module state and combat remain
server-authoritative; accepted operations are transactional, bounded, and
idempotent; migration and rollback evidence preserve legacy state; replay and
Inspector reconstruct independently; Godot presents server truth; runtime,
compatibility, reset, and resource matrices pass; and every protected artifact
remains intact.

M26 is complete. This decision authorizes only the already-sequenced M27
specification step. It does not authorize M27 implementation, another person,
public networking, commit, push, merge, version change, tag, release,
distribution, or deletion of historical evidence.

## Single M27 starting proposition

> A Relay operation with exactly two server-authoritative route choices and at
> most one bounded deterministically seeded event per route can expose distinct
> risk/reward outcomes while every choice, objective transition, operation
> summary, reward, and replay reconstruction remains finite, transactional,
> compatibility-preserving, and independently verifiable.

This proposition is only the input to a future M27 specification gate. No M27
schema, replay event, protocol message, Lua behavior, authoring tool, bot,
client surface, or runtime implementation was created during M26 closure.
