# M28 closure audit

Date: 2026-09-07  
Milestone: M28 — local cooperation simulation.  
Decision: **Gate M28 approved with bounded residual risk (green)**.

## Starting proposition result

M28 began with one engineering proposition: a loopback Relay slice with
exactly two admitted participants, one server-authoritative downed/revive
cycle, one bounded contextual ping, and one complementary two-role objective
could require distinct recorded contributions while anti-loop, disconnect,
reward, persistence, replay, Protocol V2 compatibility, and honest projection
remained finite and independently reconstructible.

The proposition is supported inside that technical boundary. One immutable
anchor and one immutable runner make five distinct accepted contributions;
one fixed ping gates one scripted downing and one fixed revive; the unchanged
Warden closes the operation; and a single atomic success grants each
participant two fragments and 125 XP. Every failure, disconnect, retry,
conflict, timing boundary, reset, compatibility flow, and resource envelope is
bounded and reconstructed independently. This is not evidence that another
person understands, enjoys, prefers, or would cooperate around it.

## Gate-by-gate closure

| Gate | Accepted result |
| --- | --- |
| 1 — specification | One falsifiable proposition; exactly two immutable roles, one objective/ping/down/revive cycle, exact targets/timers/reward, M27 exclusivity, compatibility, evidence, and stop bounds. |
| 2 — pure domain | Fixed `m28-v1` lifecycle, exhaustive 900-row ordered weapon/build report, 50 authority/lifecycle cases, exact boundary arithmetic, nonlethal optimal pressure, and stable hashes. |
| 3 — persistence | Additive idempotent `0008`, recoverable backup/restore and apply-twice proof, immutable participants/profiles, transactional lifecycle/reward, concurrency, constraints, and complete rollback matrix. |
| 4 — replay/Inspector | Six bounded event kinds, atomic transition/replay coupling, independent legacy/active/success/failure reconstruction, corruption rejection, CLI parity, and GET-only Inspector state. |
| 5 — Protocol/gateway | Four additive client and six additive server current-V2 variants; explicit capability, authoritative clocks/life/roles/terminal, M27 serialization, shared truth, failure withholding, ordinary-V2 and V1 preservation. |
| 6 — role bots | One bounded anchor/runner bot and a 16-session matrix covering success, three real-clock timeouts, twelve disconnect identities/phases, invalid/retry/cancellation probes, exact SQL/replay/Inspector parity, and resets. |
| 7 — Godot | Strict server-only cooperation projection, neutral pending states, keyboard/non-color/Reduced Flash behavior, invalid-server fail-closed handling, real Godot-anchor/runner flow, and six 1280×720 captures. |
| 8 — runtime/resources | Normally earned module state, four ordered build/weapon successes, 16 failure sessions, participant defeat integration, four compatibility flows, exact ledgers, deterministic repetition, and bounded RSS/FD/thread/connection growth. |

No later block widened an earlier gate. Block 8 changed validation
instrumentation only and did not retune product state from wall-clock samples.

## Complete accepted evidence review

Every accepted manifest was reread and verified during closure:

| Evidence | Verified manifest SHA-256 |
| --- | --- |
| Gate 2 pure reports | `f51cf6a76e37e654d65670c109c46e08976f2202f6cb71a393aebdde991f9fcf` |
| Gate 3 pre-migration backup | `8e0f5cd6b3424ea89f3449ce4451957ed569c1499b1be512965f73e8e70fe57e` |
| Gate 3 disposable database | `6009de322d6c32e9aa5dcc8373f9f60221f9ca125b9d452d6731138a05efa039` |
| Gate 3 working database | `4d4edfe400fe3f56e121e306cd4e7f6acf3404d1431dd80dac398c3d903e545d` |
| Gate 4 replay/Inspector | `f4ec22b93323121effb8196096e64606121b212a416e314c6a0282f89620565f` |
| Gate 5 Protocol/gateway | `bf2d3206b0a807655f84aa7db41ef6bd3c3b4af7fb7579c33034516d681cce79` |
| Gate 6 role bots | `5acbb4351b28d3a8e86f2c694445ddd219f6b3cf1a9ccbc64a70b56d35d79a2b` |
| Gate 7 Godot | `1cbf0d832aafc3bd4b56c9a4520e0ad219abc381b63a8d3a461ea02b9ee4a10a` |
| Gate 8 runtime | `9dbf6fba76bd99cf6abdc10109e8d1ec5dd22271949e8ac732306a049f4f6a46` |
| Gate 8 canonical/boundaries | `36497f1b7d3467411910b994b379f1d33481e03643c2bf20aa54203e22596b3a` |

The owner-directed Relay Hub readability and gameplay-depth passes also
verify at manifest hashes
`3c0682c226289b05462e9fbce5cd41fc21b9e659269bab7e51456034ca2b5350`
and
`4564a2afa6ef17714e4118bbbacafb3b0ef0eb2e085e156e7dc04af35b6c33c0`.
Their final presentation movies remain in Downloads with SHA-256 values
`86b10c44da785eb4e650f0683fe412fc1e2a1235929192432800bd35402b6ab6`
and
`b08ba9bd8b0cccb5c3a3b61d1b4c83461f2115fd8136c568d5ef2eace2e0263a`.
These passes improve lighting, spatial hierarchy, enemy silhouettes, staged
sector transition, and authoritative fragment presentation; they do not claim
a second map, new enemy archetype, new item authority, or M31 completion.

The closure verification ledger is retained at
`/mnt/c/Users/Ian/revenant-local-evidence/m28-closure-20260907a`. Rejected and
superseded targets remain separate and were inventoried rather than deleted or
promoted.

## Complete implementation and authority review

The final M28 implementation preserves explicit ownership:

- `revenant-cooperation` owns the pure fixed-role lifecycle, typed targets,
  life state, timers, retry/conflict behavior, contributions, terminal state,
  and reward contract without database, network, client, filesystem, or wall
  clock dependencies;
- PostgreSQL owns immutable operation/participant profiles, transition and
  terminal facts, equal reward/history grants, idempotency, serialization, and
  atomic rollback. Migration `0008` adds only the two reviewed tables;
- replay owns the reviewed start/ping/down/revive/success/failure vocabulary
  and reconstructs from immutable append evidence rather than current mutable
  operation, inventory, route, or clock state;
- Protocol remains V2 with its 64 KiB frame limit and private 8 KiB cooperation
  message bound. Capability is explicit and frozen V1 cannot express M28;
- the gateway alone derives roles from admission order, supplies monotonic
  time, validates range/phase/life, applies pressure, serializes against M27,
  commits before projection, and withholds reward/terminal truth on failure;
- the role bot submits bounded intents but owns no role, target coordinate,
  health, timer, contribution, reward, or outcome;
- Godot validates and presents server facts. It has no cooperation resolver,
  local countdown, healing/reward formula, optimistic success, or terminal
  inference; and
- Inspector retains only existing GET surfaces. There is no mutation endpoint,
  raw SQL surface, public listener, or retained failure-injection mechanism.

The Gateway Dockerfile now copies every current workspace member needed for a
reproducible production build. This packaging correction changes no runtime
contract. Block 8 scripts are bounded, uniquely prefixed, refuse evidence
overwrite, inspect rather than fabricate database state, stop isolated
processes, and fail resource or checksum drift.

There is no unresolved `TODO`, `FIXME`, `TBD`, `todo!`, or `unimplemented!` in
the reviewed M28 surface. `panic!`, `unwrap`, and `expect` matches are test
assertions except for bounded invariant accesses after validated negotiation,
persistence construction, replay sequencing, or reward cardinality. Strict
Clippy and all failure-path suites cover those seams.

## Complete checkpoint diff review

The final non-ignored source set was compared byte-for-byte with the external
M27 closure checkpoint. All 352 checkpoint paths remain present; none was
deleted. M28 changes 30 checkpoint members and adds 34 files, including this
closure audit, producing 386 current non-ignored source files.

The differences map exactly to workspace/domain wiring, additive persistence,
replay/Inspector, compatibility/protocol/gateway composition, bot and matrix
instrumentation, Godot cooperation projection, the two bounded presentation
passes, Docker build context, tests, audits, and roadmap state. `VERSION`, the
frozen V1 tree, migrations `0001`–`0007`, the M24 package, every pre-M28 capture,
and the M27 catalog/Lua revision remain unchanged. There is no missing
checkpoint member or unexplained product surface.

## Final uninterrupted quality and protected boundaries

After the complete implementation and Block 8 correction, the canonical
`make check` passed uninterrupted:

- version consistency at `0.2.0`;
- `cargo fmt --all --check` and workspace/all-target/all-feature Clippy with
  warnings denied;
- all **217 Rust and PostgreSQL tests** and every-target workspace build;
- Inspector TypeScript check and production Vite build;
- secret audit;
- current two-client multiplayer, persistence, replay, and Inspector smoke;
- automatic, repeated, manual, and keyboard-only Godot flows plus every
  retained semantic marker from M17 through M28; and
- frozen-V1 adapter compatibility and independent reconstructed-V1 startup.

Production Gateway and Inspector Compose images build. Every M21–M28 capture
manifest verifies. Permanent PostgreSQL, Gateway, and Inspector services are
healthy and bound only to `127.0.0.1:5432`, `:7000`, `:8080`, and `:4173`.
There is no isolated M28 listener, public-schema non-internal trigger, or
public-schema function. `git diff --check` passes and no generated `.gd.uid`
file is untracked.

The frozen V1 hashes remain
`4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`
and
`c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.
The verified M27 closure checkpoint remains 10,684,040 bytes with 352 entries
and SHA-256
`81bb1fafaa397b27db7fc505767c09202d68b77e13355952b49ad9d994280d70`.

The closure pre-gate inventory of the mutable local engineering database had
16 public tables, 1,477 synthetic accounts/characters, 1,479 activity
histories, 13,880 replay rows, 283 route operations, 279 cooperation
operations, and 558 cooperation participants. Later canonical smoke sessions
may increase these fixture counts without changing the audited M28 prefix.
Cooperation outcomes are 100 successes, 21 ping timeouts, 17 revive timeouts,
17 operation timeouts, ten participant defeats, 59 abandonments, and 55
deliberately incomplete component/runtime fixtures. All 85 composed
`session-*` cooperation operations have exactly two roles, zero route overlap,
and zero success, failure, incomplete-terminal, or reward-integrity violation.
Lower-level persistence component fixtures intentionally omit replay and are
not misclassified as composed sessions or usage metrics.

No protocol generation, schema beyond reviewed additive `0008`, M25/M26
arithmetic, M27 catalog/event/Lua revision, version, frozen client, package,
commit, tag, release, public listener, or public artifact changed in closure.

## Rejected evidence synthesis

All rejected work is preserved and classified in its owning audit:

- Gate 2 rejected one wrong disconnect instant and superseded one valid report
  before admitted-health/revive-boundary tightening;
- Gate 3 rejected or superseded early temporal-constraint, deadline-retry, and
  working-database evidence before the accepted `c` targets;
- Gate 4 retained a diagnostic that used the wrong historical fixture as a
  schema-success probe;
- Gate 5 corrected late disconnect projection, capable-peer disclosure, and
  persisted-ping retry time before final evidence;
- Gate 6 retained two collector-only JSON/SQL query corrections before `g6c`;
- Gate 7 retained startup/admission, traversal, duplicate-action, and display
  ordering corrections before the accepted `h` live run; and
- Gate 8 rejected `b8a` for ordinary-V2 admission ordering and `b8b` because
  two focused PostgreSQL commands were skipped without `DATABASE_URL`; the
  entire matrix was rerun as accepted `b8c`.

No rejected target was deleted to simulate rollback or silently included in an
accepted checksum manifest. Durable synthetic effects use disjoint local
prefixes and remain honest engineering fixtures.

## Bounded residual risk

- The exhaustive 900-row/50-case pure reports remain mathematical authority;
  runtime samples four build pairs and one local WSL/software-rendered host.
- Three runtime timeouts exercise real monotonic clocks, but this is not a
  long-duration or lossy-network soak and predicts no public deployment.
- Hardware GPU, physical audio device, other platforms, and accessibility
  across bodies/hardware remain untested.
- Creator automation and visual inspection establish no outside-player
  comprehension, preference, enjoyment, teamwork, replay desire, or
  anti-griefing effectiveness.
- Revenant still has one approximately 24×24 Relay Hub activity. Arrival Deck
  and Relay Core are staged sectors within it; genuinely new enemies, areas,
  collectible types, and broader world content remain later M31 work.

No residual is an integrity, authority, transaction, replay, projection,
compatibility, privacy, reset, or bounded-resource blocker inside M28 scope.

## Single bounded M29 starting proposition

> A removable, server-authoritative Echo that records at most one short
> sequence of already accepted movement/combat intents from its owner and
> replays it once inside the same local activity can create one measurable
> tactical choice in a fixed encounter without owning rewards, objectives,
> enemy decisions, or cooperation roles, while creation, use, lifetime,
> persistence/replay evidence, exploit limits, compatibility, and total
> removal remain finite and independently reconstructible.

This is the only M29 starting proposition, not an implementation decision.
M29 Gate 1 must first audit what “Echo” can mean in the current architecture,
freeze exact state/lifetime/authority/isolation/removal bounds, and define a
falsifiable creator-only keep/remove measure. It may narrow or reject the
proposition. If the tactical signal cannot justify its complexity without
weakening server authority, compatibility, or complete removability, removal
is the successful result.

## Gate decision

Gate M28 is **approved with bounded residual risk (green)**. Exactly two local
participants, immutable roles, one fixed cooperation sequence, authoritative
life/timers/contributions, atomic equal reward, route exclusion, durable
failure/abandonment, independent replay, honest projection, compatibility,
runtime repetition, reset, and resources are green. M28 is complete.

The owner's standing sequential authorization permits M29 to begin only at
Gate 1 specification/removal review. No M29 implementation, version change,
commit, tag, release, distribution, or public exposure is authorized by this
closure.
