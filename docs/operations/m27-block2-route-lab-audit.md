# M27 Block 2 route/event laboratory audit

Date: 2026-09-01  
Gate: 2 — pure route/event domain and deterministic laboratory.  
Decision: **approved**.

## Preserved input checkpoint

Before the first M27 implementation edit, the Gate 1 tree was archived outside
the repository at:

`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m26-closure-45718926-20260901.tar.gz`

It contains 315 source/evidence files, is 10,229,929 bytes, and has SHA-256
`066b9251f47c7b916b298d453a9c11b95c63ea305247e3cbf8010e0340256d82`.
The accepted source list and archive entry count match. No M27 implementation
preceded it.

## Implemented pure boundary

- `revenant-operations` owns typed route/event/objective identifiers, the exact
  finite catalog, checked Warden-health arithmetic, a stable 63-bit seed
  resolver, leader/capability/phase authority, one accepted session operation,
  idempotent retry/conflict, explicit baseline lock, deadline success/failure,
  no-reward failure, and reconstructibly incomplete state.
- `revenant-operation-lab` composes that API with the accepted M25 combat and
  M26 module APIs. It emits deterministic JSON or a human-readable table for
  the complete route/event/participant/weapon/build space and a separate
  operation-transition report.
- The workspace manifest and lockfile add only the pure crate and laboratory.
  The domain depends only on `serde`; neither new target has a socket,
  database, Lua, filesystem authoring, gateway, replay, protocol, client, or
  clock dependency.

Block 2 contains 2,287 new Rust source lines. That size is explicit rather than
hidden in the gateway: 1,274 lines own the typed domain plus 15 unit tests, and
1,013 lines own report generation plus eight tests.

## Frozen Gate 2 catalog

Catalog revision is `m27-v1`. Both explicit routes use a 90,000 ms monotonic
operation budget.

| Route | Objective path | Per-character reward |
| --- | --- | --- |
| `breach` | clear drones → relay door → Warden | 2 fragments, 100 XP |
| `stabilize` | clear drones → stabilizer → relay door → Warden | 1 fragment, 150 XP |

The event catalog contains exactly two candidates per route and resolves
exactly one candidate after accepted selection:

| Route/event | Warden health | Counter damage |
| --- | ---: | ---: |
| `breach/overcharged_armor` | 12,000 bp | 15 |
| `breach/arc_surge` | 10,000 bp | 20 |
| `stabilize/shielded_channel` | 10,000 bp | 10 |
| `stabilize/residual_feedback` | 11,000 bp | 15 |

Warden health is participant-scaled first, then the event multiplier is applied
once with checked round-half-up integer arithmetic. No event changes the drone,
player, weapon/module profile, movement, reward, seed, or later session state.

The compatibility baseline is deliberately outside this catalog. It retains
the original three objectives, 1-fragment/100-XP reward, 140/210 drone health,
240/360 Warden health, 15-damage Warden counters, and exact 40 total optimal
incoming damage for all 60 weapon/build/participant rows.

## Frozen seed resolver

Resolver revision is `m27-permute63-v1`:

- the input is one non-negative 63-bit server-owned seed;
- route-specific salts precede a fixed xor-shift/odd-multiply permutation in
  the 63-bit domain;
- the low output bit selects one of the route's two fixed candidates;
- odd multiplication modulo `2^63` and xor-shift steps are bijective, so the
  complete seed domain partitions evenly between the two indices;
- route salts create distinct mapping vectors;
- zero, one, two, three, `i64::MAX-1`, and `i64::MAX` vectors are embedded in
  the report; and
- a 65,536-seed focused fixture reaches both candidates per route and remains
  inside its reviewed balance band.

A same-operation retry ignores a newly supplied candidate seed and returns the
stored result. The seed type rejects `i64::MAX+1`. Block 2 creates no production
seed generator or network seed field; those remain later integration decisions.

## Accepted deterministic reports

The accepted encounter report is:

`/tmp/revenant-m27-block2-route-lab.json`

It is 421,893 bytes and has SHA-256
`d90c61d5cc734a98049cb16f73a0500b0bbe418ffd76f4bb9f811515284922a0`.
A fresh second run was byte-identical.

It contains exactly:

- 240 routed rows: two routes × two events × two participant counts × two
  weapons × 15 M26 builds;
- 60 unrouted compatibility rows;
- 60 route-pair tradeoff summaries; and
- 12 recorded seed-boundary vectors.

The complete routed envelope is:

| Event | Rows | Warden health | Incoming | Combat lower bound |
| --- | ---: | ---: | ---: | ---: |
| `overcharged_armor` | 60 | 288-432 | 40 | 1,491-2,970 ms |
| `arc_surge` | 60 | 240-360 | 50 | 1,278-2,640 ms |
| `shielded_channel` | 60 | 240-360 | 30 | 1,278-2,640 ms |
| `residual_feedback` | 60 | 264-396 | 40 | 1,491-2,856 ms |

All 240 rows are below each admitted 100-130 maximum health, at or below the
50-damage bound, and far below the 90,000 ms operation budget. The duration is
explicitly a combat-only lower bound; it does not claim movement, presentation,
network, or real operation duration.

Every one of the 60 comparisons is non-dominated. Breach always yields one
more fragment but less XP and greater aggregate/worst pressure; Stabilize
yields more XP and less pressure but one fewer fragment. No weight or exchange
rate is invented between XP, fragments, pressure, and time.

The accepted domain report is:

`/tmp/revenant-m27-block2-domain-lab.json`

It is 7,806 bytes and has SHA-256
`111a5dc8a12c137108a182627d53721c13530078139d6f59c7367bceb256a85d`.
A fresh second run was byte-identical.

Its 18 cases contain seven acceptances and 11 rejections. They cover first
selection, retry with a different candidate seed, conflicting route, second
operation, completion/retry, terminal conflict, incomplete capability,
non-leader choice, rejected-ID reuse after preconditions change, exact deadline,
post-deadline completion/timeout, explicit baseline lock, pre-choice phase,
unknown participant, and malformed operation ID. Every rejected case preserves
the complete state exactly. Timeout has no reward; a selected operation can
remain durably representable without a terminal outcome.

## Authority and arithmetic review

- The first participant in supplied durable join order is the only leader.
  There are one or two unique bounded participants, matching the current local
  session ceiling.
- Selection opens only after the drone phase. Locking the compatibility door
  changes to a distinct `BaselineLocked` phase and makes later choice invalid.
- Every participant must explicitly become capable before selection. No
  username, build string, protocol equality, or timing implies capability.
- Operation IDs are 1-32 ASCII alphanumeric/hyphen characters. Exactly one
  accepted operation exists per state. Business/validation rejection reserves
  nothing.
- Same ID/route/leader retries replay the accepted outcome even after the phase
  advances and cannot reroll. Same ID/different route conflicts; a new ID after
  selection is rejected.
- Completion at exactly 90,000 ms succeeds. Completion at 90,001 ms and timeout
  at exactly 90,000 ms reject without mutation. Timeout at 90,001 ms fails with
  no reward.
- All arithmetic uses checked multiplication/addition/conversion. Invalid
  catalog cardinality, reward, duration, event membership, inert/out-of-bound
  effect, objective path, seed, participant, and identifier fixtures reject.

## Rejected implementation/evidence attempts

1. The first formatting invocation found the workspace member for the lab
   before its manifest had been added. No test ran and no evidence was accepted;
   the bounded lab target was then created.
2. The first focused test run passed but reported two unused imports. They were
   removed before strict Clippy and report generation.
3. Strict Clippy then rejected one 143-line report builder. It was decomposed
   into four bounded case-group functions; no lint suppression was added.
4. Adding explicit invalid-catalog fixtures initially placed a local const after
   statements, which strict Clippy rejected. The const was moved before the
   statements and the complete focused gate was rerun.
5. The first protected-boundary comparison command was rejected before process
   creation because it contained recursive temporary-directory cleanup. No file
   was removed. The comparison was repeated without a destructive command and
   is the accepted evidence below.

No rejected report hash is presented as Gate 2 evidence.

## Quality and protected-boundary evidence

The final Block 2 source passed:

- `cargo fmt --all -- --check`;
- workspace/all-target/all-feature Clippy with warnings denied;
- all 121 workspace unit and PostgreSQL integration tests, including 23 focused
  route/domain/lab tests;
- `cargo build --workspace --all-targets`; and
- `git diff --check`.

A file-list and byte comparison against the external pre-implementation
checkpoint covered 122 protected files under activity/Lua, persistence,
replay, protocol, gateway, Godot, frozen V1, and `VERSION`. Every file and the
complete list matched exactly. In particular:

- activity Lua SHA-256 remains
  `460c895c2bb99090cbcf59d9238717fb3a868c9037dcb46fca5a88fc93e6b040`;
- frozen V1 source SHA-256 remains
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`;
- frozen V1 manifest SHA-256 remains
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`;
- `VERSION` remains `0.2.0`; and
- `HEAD == origin/main == 4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`.

The full smoke, Godot, live route runtime, migration, replay/Inspector, protocol,
and V1 gameplay gates are intentionally not claimed here; Block 2 did not alter
those surfaces.

## Residuals entering Block 3

- The exact catalog and resolver are pure values only. Lua cannot author them,
  the gateway cannot select them, and nothing persists or reconstructs them yet.
- The 90-second budget is a frozen authoring/domain value, but Block 2 has no
  monotonic runtime clock. Real timeout enforcement belongs to Block 6.
- The matrix assumes all participants use the same weapon/build row. Later
  runtime evidence must also sample distinct admitted M26 builds.
- The deterministic balance and non-dominance result is engineering evidence,
  not a human risk preference, reward valuation, enjoyment, or replayability
  claim.
- `/tmp` reports are ephemeral. Their schemas, generation command, tests, and
  hashes make them reproducible; final M27 runtime evidence will use a retained
  external manifest.

None is a Gate 2 blocker because Lua, persistence, replay, gateway, client, and
runtime proof are explicitly assigned to later blocks.

## Gate 2 decision

Gate 2 is **approved**. `m27-v1`, `m27-permute63-v1`, the 90-second budget,
two route definitions, four event definitions, reward/objective/effect values,
63-bit seed envelope, 32-byte operation ID, one/two participant bound,
240 routed rows, 60 compatibility rows, operation lifecycle, and checked
arithmetic are frozen for M27.

Block 3 is authorized only to implement restricted declarative Lua authoring,
resource/graph validation, a stable validator CLI, and the golden M26 baseline.
It may not change this catalog, add a route/event/effect/objective kind, or edit
schema, persistence, replay vocabulary, protocol, gateway, Godot, frozen V1,
package, or version.
