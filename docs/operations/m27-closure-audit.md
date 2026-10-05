# M27 closure audit

Date: 2026-09-06  
Milestone: M27 — routes, risks, events, and replayable operations.  
Decision: **Gate M27 approved with bounded residual risk (green)**.

## Starting proposition result

M27 began with one engineering proposition: a Relay operation with exactly two
server-authoritative routes and at most one bounded deterministically seeded
event per route could expose distinct measured risk/reward outcomes while
choice, objectives, terminal summary, rewards, persistence, replay, protocol,
and presentation remained finite, transactional, compatibility-preserving,
and independently verifiable.

The proposition is supported inside its stated engineering boundary. Breach
and Stabilize retain distinct fragment/XP and measured-pressure tradeoffs; all
240 routed laboratory rows are bounded and every one of the 60 paired
comparisons is non-dominated. The final runtime matrix sampled both routes,
all four events, both participant counts, both weapons, four admitted build
profiles, success, timeout, disconnect, incomplete capability, ordinary V2,
current V1, and frozen V1. No result is evidence that another person
understands, prefers, enjoys, or would replay either route.

## Gate-by-gate closure

| Gate | Accepted result |
| --- | --- |
| 1 — specification | One falsifiable proposition; exactly two routes and two event candidates per route; explicit compatibility, seed, lifecycle, deadline, authority, evidence, and stop boundaries. |
| 2 — pure domain | Fixed `m27-v1` catalog and `m27-permute63-v1` resolver; 240 routed plus 60 baseline rows; all events reachable; checked arithmetic; non-dominance and finite lifecycle proof. |
| 3 — Lua authoring | Restricted declarative schema with source/instruction/memory/graph bounds, stable validator output, exact routed revision, and byte-preserved M26 golden projection. |
| 4 — persistence | Additive idempotent `0007`, recoverable pre-migration backup, disposable restore/apply-twice proof, immutable selection, participant order, terminal/reward transactions, and rollback boundaries. |
| 5 — replay/Inspector | Three bounded route event kinds, atomic replay coupling, independent success/failure/incomplete reconstruction, corruption rejection, CLI parity, and GET-only Inspector facts. |
| 6 — Protocol/gateway | Two opt-in client and three server V2 variants; server-only seed; leader/capability/choice/deadline/reset lifecycle; shared multiplayer truth; ordinary-V2 and frozen-V1 non-interference. |
| 7 — Godot | Honest server-only route projection, pending/rejection/replay/disconnect/terminal states, keyboard/reduced-flash semantics, two live routes, and three checksum-verifiable captures. |
| 8 — runtime/resources | 23 sessions/28 joins, eight routed successes/12 routed clients, compatibility/failure/interruption proof, exact SQL/replay/Inspector reconciliation, deterministic reports, and bounded resources. |

No block was implemented before its exact preceding review and no gate was
broadened retroactively to justify work from a later block.

## Complete evidence review

The accepted closure inputs were reread and reverified after Block 8:

| Evidence | Closure verification |
| --- | --- |
| M26 closure checkpoint | 315 entries, 10,229,929 bytes, SHA-256 `066b9251f47c7b916b298d453a9c11b95c63ea305247e3cbf8010e0340256d82`. |
| Gate 2 operation report | Regenerated before/after final runtime work; 240 routed rows, 60 baseline rows, four events, zero dominance; SHA-256 `d90c61d5cc734a98049cb16f73a0500b0bbe418ffd76f4bb9f811515284922a0`. |
| Gate 2 domain report | Eighteen lifecycle/authority cases, byte-identical before/after runtime; SHA-256 `111a5dc8a12c137108a182627d53721c13530078139d6f59c7367bceb256a85d`. |
| Gate 3 authoring | Active Lua validation SHA-256 `2efd469f456b19a4d51d84cdcb1cd812ec31b03f0e5aab487cc20a4e1ac800ef`; M26 golden source remains `460c895c2bb99090cbcf59d9238717fb3a868c9037dcb46fca5a88fc93e6b040`. |
| Gate 4 pre-migration | Recoverable dump SHA-256 `fe573546dd6e700604ef43ebef9ad8eabc84c007f19936733ff158d899e1623d`; verified evidence manifest `7638979ef377c6197f9e41f46080cfd7d14c68a0b5645625caa3305f6aa39c2c`. |
| Gate 4 accepted migration | Fourteen tables, empty initial route tables, byte-identical legacy exports; verified manifest `4c61ee012408683641001038f9a6c78c212f24458bd15670614be67fd848d82e`. |
| Gate 5 replay/Inspector | Atomic 15-boundary failure matrix, independently reconstructed success/failure/incomplete flows, current-source GET-only Inspector and protected-boundary audit. |
| Gate 6 gateway | Both live routes, complete opt-in/leader/retry/conflict/deadline/shared-state proof and uninterrupted 162-test gate. |
| Gate 7 captures | Three 1280×720 PNGs and `docs/art/m27/captures/SHA256SUMS` verify. |
| Gate 8 runtime | 117 files/1,561,467 bytes; verified manifest SHA-256 `322867bfa3f5ee977743e66c738f844ca1708e694777dd8785c2a0b2c25116ee`. |

The ephemeral Gate 2 and Gate 3 `/tmp` outputs need not persist because their
schemas, commands, hashes, and deterministic regeneration are recorded. Gate
8 embeds final regenerated operation/domain reports in its immutable external
manifest.

## Complete implementation and authority review

The final M27 implementation remains composed through explicit boundaries:

- `revenant-operations` owns the finite catalog, typed identifiers, checked
  resolver/effect arithmetic, route lifecycle, retry/conflict, exact deadline,
  terminal summary, and baseline lock without a database, network, client,
  filesystem, or wall clock dependency.
- `revenant-activities` evaluates only a bounded declarative Lua table. Its
  current routed source has SHA-256
  `89b8d48ca4906ff484c3c60f1719e93f99a67fe77fae3f3b34073739031265a5`;
  the exact M26 source remains a golden compatibility fixture.
- PostgreSQL owns immutable accepted selection, admission-ordered participants,
  terminal outcome, participant rewards, idempotency, and replay-coupled
  transactions. Migration `0007` is additive and has SHA-256
  `8f5a3bcf9d22e6b9b3bcfd7175a47365ed2b27ef59003d84586d781e35f55b6a`.
- Replay owns the reviewed selected/succeeded/failed vocabulary and validates
  embedded revision, seed, event/effect, objectives, encounter, reward, grant,
  participant, order, and terminal facts without reading current Lua,
  operation rows, inventory, modules, randomness, or time.
- Protocol remains V2 and the 64 KiB ceiling remains fixed. Route capability
  is explicit; the intent contains only operation and route IDs. Unopted V2
  and frozen V1 receive no route-specific message.
- The gateway alone sources the production 63-bit seed from the operating
  system, commits selection before projection, uses monotonic deadline
  enforcement, applies event effects once, shares route truth, and withholds
  terminal/reward projection until durable commit.
- Godot contains no route resolver, seed generator, stat/reward formula,
  deadline authority, optimistic acceptance, or terminal inference. It
  validates and presents bounded server truth.
- Inspector retains only its existing GET routes. Mutation verbs reject and
  no raw SQL, reroll, seed injection, or operation mutation surface exists.
- Block 8 instrumentation is isolated behind a non-default required feature
  and separately named binary. The normal package explicitly defaults to
  `revenant-gateway`; its default release build contains none of the matrix
  environment variable, banner, or seed-queue strings.

There is no unresolved TODO, FIXME, TBD, `todo!`, or `unimplemented!` marker in
the reviewed M27 surface. Reviewed `panic!`, `expect`, and `unwrap` matches are
test-fixture assertions except for already-audited invariant accesses after
validated negotiation/session construction. No third route, event, random
reward, arbitrary effect, vote, reroll, public network, external account,
third-party proprietary asset, or human-research dependency was added.

## Complete checkpoint diff review

The final repository source set was compared against the external M26 closure
checkpoint. The 315 checkpoint paths remain present; none was deleted. M27
changes exactly 28 checkpoint members and adds 35 source/evidence files,
producing 350 current non-ignored source files.

The differences map to the eight reviewed gates: workspace/domain wiring,
restricted authoring, additive migration/persistence, route replay and
Inspector projection, additive compatibility/protocol/gateway behavior,
Godot route presentation/captures, matrix instrumentation, and their tests and
audits. `VERSION`, the frozen V1 tree, migrations `0001`-`0006`, the M24
package, and all pre-M27 evidence remain unchanged. There is no missing
checkpoint member or unexplained product surface.

## Final uninterrupted quality gate

After the final implementation correction and accepted Gate 8 matrix, the
canonical `make check` passed uninterrupted:

- version consistency at `0.2.0`;
- `cargo fmt --all --check`;
- workspace/all-target/all-feature Clippy with warnings denied;
- all 162 Rust and PostgreSQL tests;
- every-target workspace build;
- Inspector TypeScript check and production Vite build;
- secret audit over 349 candidate files;
- two-active fake-client smoke and authoritative pressure checks;
- automatic, reused, manual, and keyboard-only Godot flows;
- every retained M17-M27 semantic validation marker;
- persistence, independent replay, and current Inspector reconciliation; and
- frozen-V1 adapter compatibility plus independent reconstructed-V1 startup.

A separate locked optimized build of the default `revenant-gateway` release
target passed. Static inspection confirmed that it excludes the
`REVENANT_M27_MATRIX_SEEDS` input, matrix banner, and seed-queue failure text.

Post-gate invariant review also passed:

- all M21-M27 capture manifests;
- exact frozen V1 hashes and no diff under `archive/clients/v1`;
- `git diff --check` and zero untracked `.gd.uid` files;
- 14 public PostgreSQL tables and zero installed M27 failure-injection trigger
  or function;
- zero integrity failure among all working-database sessions containing route
  replay evidence;
- healthy permanent PostgreSQL, Inspector, and gateway services bound only to
  `127.0.0.1:5432`, `:4173`, `:7000`, and `:8080`;
- no leftover matrix listener on `:17029` or `:18029`; and
- `HEAD == origin/main == 4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`,
  no tag at HEAD, and no commit, push, release, or public artifact.

The mutable local database now contains 826 synthetic accounts/characters,
1,059 activity histories, 9,321 replay rows, 216 route operations, and 254
route participants. Of route operations, 95 are succeeded, 26 are
`failed_timeout`, and 95 remain incomplete, including deliberate Block 4 and
failure fixtures. Route replay has 132 selections, 89 success terminals, and
20 failed terminals. The 23 `future_event` rows remain deliberate unknown-kind
rejection fixtures. These are local engineering fixtures, not player or
product usage metrics.

## Rejected evidence synthesis

Rejected and superseded work is retained and classified in each block audit:

- Gate 2 manifest/import cleanup before the pure lab existed;
- Gate 3 revision-token, negative-fixture, CLI-option, and strict-Clippy
  corrections;
- Gate 4 disposable database naming/connection diagnostics and transactional
  fixture corrections before the working migration;
- Gate 5 HTTP-method, PostgreSQL DDL serialization, JSONB, replay-ID, and
  corruption/failure-matrix corrections;
- Gate 6 compile/test integration fixes before live evidence and the accepted
  canonical gate;
- Gate 7 a stale listener collision and an overlong synthetic identity before
  fresh live/capture evidence;
- Gate 8 one interrupted matrix, one two-client validation race, two
  superseded complete runs, and one ambiguous default binary discovered by
  the first closure smoke.

No rejected attempt was silently promoted, deleted to simulate rollback, or
merged into an accepted checksum manifest. Valid atomic synthetic database
effects from rejected harness runs remain honestly recorded under disjoint
local prefixes.

## Bounded residual risk

- The final runtime matrix samples eight routed successes from the exhaustive
  240-row space. Deterministic pure/domain tests remain authority for all 15
  builds, seed boundaries, and exact deadline edges.
- Runtime duration, RSS, rendering, and listener evidence comes from this one
  local WSL/software-rendered environment and cannot predict other hardware.
- The historical WSL/PostgreSQL wall-clock-step residual remains possible for
  display timestamps. Append order is replay authority and monotonic clocks
  own gameplay deadlines.
- The long-lived Compose gateway image predates M27. It remains healthy and
  loopback-only but was not treated as current route evidence.
- Creator-operated automation and visual review prove no outside-player
  preference, comprehension, enjoyment, replay desire, accessibility across
  bodies/hardware, or human cooperative behavior.

No residual is an integrity, dominance, authority, transaction, replay,
projection, compatibility, privacy, reset, or bounded-resource blocker inside
M27 scope.

## Single bounded M28 starting proposition

> A loopback Relay slice with exactly two admitted participants, one
> server-authoritative downed/revive cycle, one bounded contextual ping, and
> one complementary two-role objective can require distinct recorded
> participant contributions while anti-loop, disconnect/abandonment, reward,
> persistence, replay, Protocol V2 compatibility, and honest projection remain
> finite, deterministic, and independently reconstructible.

This is only the M28 Gate 1 starting proposition. Before any M28 source change,
Gate 1 must audit the current actor/combat/objective/session/protocol/client
baseline and freeze exact role, timing, target ownership, anti-loop,
disconnect, abandonment, reward, compatibility, evidence, and stop bounds.
It may reduce or reject the proposition. It may not claim human teamwork,
social quality, anti-griefing effectiveness, or outside-player comprehension.

## Gate decision

Gate M27 is **approved with bounded residual risk (green)**. Exactly two
opt-in routes and one server-resolved event per routed session are frozen;
non-dominance, compatibility, finite Lua authoring, authoritative immutable
selection, monotonic terminal state, atomic rewards/replay, independent
reconstruction, honest client projection, runtime repetition, failure
withholding, reset, and resources are green. M27 is complete.

The owner's standing sequential authorization permits M28 to begin only at
its Gate 1 specification and baseline audit. No M28 implementation, version
change, commit, tag, release, distribution, or public exposure is authorized
by this closure.
