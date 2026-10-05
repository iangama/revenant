# Safe resume roadmap — 2026-08-31

## Owner request and authority

The owner asked to continue the complete already-sketched solo roadmap. M25
through M32 are sequentially authorized only after the preceding milestone has
a green gate. This does not authorize another person, public exposure, commit,
push, merge, version change, tag, release, or deletion of existing evidence.

The working tree intentionally contains the reviewed M24-M26 work without a
commit. Preserve it. Baseline `HEAD` and `origin/main` are both:

`4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`

`VERSION` remains `0.2.0`. Frozen V1 files and the frozen M24 package remain
unchanged. Replay has the separately reviewed Gate 5 vocabulary and Protocol
V2 has the separately reviewed additive Gate 6 module messages.

## Exact continuation point

R1 and R2 completed on 2026-08-31. M25 implementation, validation, and formal
closure are complete. M26 Gates 1-8 and Gate M26 are green under
`docs/milestones/M26-fragments-modules-horizontal-progression.md`. R3 is
complete. M27 Gates 1-5 are green under
`docs/milestones/M27-routes-risks-events-replayable-operations.md`. Gate 5's
accepted implementation/audit is
`docs/operations/m27-block5-replay-audit.md`. The active continuation point is
R4 / M27 Block 6 implementation under the reviewed exact additive opt-in
Protocol V2 and gateway lifecycle contract in
`docs/operations/m27-block6-protocol-contract.md`. The pre-implementation
checkpoint remains at
`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m26-closure-45718926-20260901.tar.gz`
with SHA-256
`066b9251f47c7b916b298d453a9c11b95c63ea305247e3cbf8010e0340256d82`.
Gate 2's accepted reports have SHA-256
`d90c61d5cc734a98049cb16f73a0500b0bbe418ffd76f4bb9f811515284922a0`
and `111a5dc8a12c137108a182627d53721c13530078139d6f59c7367bceb256a85d`;
the complete audit is `docs/operations/m27-block2-route-lab-audit.md`.
Gate 3's stable authoring JSON has SHA-256
`2efd469f456b19a4d51d84cdcb1cd812ec31b03f0e5aab487cc20a4e1ac800ef`;
the complete audit is `docs/operations/m27-block3-lua-authoring-audit.md`.
Gate 4's accepted post-migration evidence manifest has SHA-256
`4c61ee012408683641001038f9a6c78c212f24458bd15670614be67fd848d82e`;
its recoverable pre-migration dump has SHA-256
`fe573546dd6e700604ef43ebef9ad8eabc84c007f19936733ff158d899e1623d`.
The complete audit is `docs/operations/m27-block4-persistence-audit.md`.
Block 5's exact pre-implementation replay contract is
`docs/operations/m27-block5-replay-contract.md`. Its replay/Inspector action is
complete and green under `docs/operations/m27-block5-replay-audit.md`: exact
atomic replay, independent corruption-rejecting reconstruction, legacy
fallback, GET-only Inspector, 15 replay-mode failure boundaries, 149 tests,
and 102 byte-identical protected files passed. Protocol/gateway integration
and client presentation remain behind their later reviewed gates. Block 6's
contract review is now recorded and its bounded Protocol/compatibility/gateway/
fake-client implementation is active; Godot remains locked behind Block 7.
The checkpoint is now complete at
`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m25-closure-45718926-20260831.tar.gz`
with SHA-256
`25e5eed51c11534569f0daf550af05990f927060c6c5b6766bc62fc22777ea83`;
the pure laboratory, domain, persistence, replay/Inspector, and protocol/gateway
blocks subsequently passed, so Gates 2-6 are green.
Block 2's accepted report has SHA-256
`b837554d10a90a134cf76e207c586d1cffde357c7594cd429ae434aecb80cd04`;
Block 3's 34-case report has SHA-256
`a2788dae84a5ae6cad607fe7b234ff3ddcf2c87fd75b5d711a6bd2dd22acd17e`.
Block 4's accepted post-migration evidence has manifest SHA-256
`b6e23447f48cce8a195f7c209f798e6d103c6d46b16026c3736aaca4d93f052e`;
its pre-migration dump remains recoverable with SHA-256
`5a849d3aa5bb006adf14a4551e14102d9f148d678f86c218ad028bbb6f24b95a`.
Block 5's accepted Inspector/reconstruction evidence has manifest SHA-256
`9c0f76e2e7fa8e7492f4efbfa7611b2469fa82865cf49c5a1b5f5627a00517cc`.
Block 6's accepted gateway evidence has manifest SHA-256
`7d92807cc61867bacaf0c1378b1a7b523eb0cd08b479ee1e0163b43b58973487`.
Block 7's accepted capture hashes are
`f008963aef72428be2142e7e848bfe8363434545d5579cfb84ac6aa514a8182b`
and `f929692882cbe52181386d3bef8d0f09005f2e361ff5b03f46ce16b31afe8839`;
its accepted real session is `session-1788260431884308379`. The active action
was completed by Block 8 runtime evidence. Block 8's accepted 125-file evidence
is `/mnt/c/Users/Ian/revenant-local-evidence/m26-runtime-0901b8d`, whose
manifest SHA-256 is
`803571b7d9664b3f60b46d924485abeabd06c47260488c76e67eefa9d5212a5b`.
It records 27 sessions, 35 active-client completions, 35 exact rewards, four
combinations, 21 loadout changes, both weapons across empty plus three builds,
distinct-build two-active sessions, V1 compatibility, and bounded resources.
The Block 9 closure review is recorded in
`docs/progression/m26-closure-audit.md`; Gate M26 is green. Taxonomy, Protocol,
schema, replay vocabulary, gateway lifecycle, and client presentation remain
frozen entering M27 specification.

Current durable milestone files:

- `docs/milestones/M25-combat-pleasure-fairness.md`
- `docs/combat/m25-untuned-baseline.md`
- `docs/combat/m25-block2-cadence-audit.md`
- `docs/combat/m25-block3-pressure-audit.md`
- `docs/combat/m25-block4-presentation-audit.md`
- `docs/combat/m25-block5-runtime-matrix-audit.md`
- `docs/progression/m26-block2-module-lab-audit.md`
- `docs/progression/m26-block3-domain-audit.md`
- `docs/progression/m26-block4-persistence-audit.md`
- `docs/progression/m26-block5-replay-contract.md`
- `docs/progression/m26-block5-replay-audit.md`
- `docs/progression/m26-block6-protocol-contract.md`
- `docs/progression/m26-block6-protocol-audit.md`
- `docs/progression/m26-block7-workshop-contract.md`
- `docs/progression/m26-block7-workshop-audit.md`
- `docs/progression/m26-block8-runtime-audit.md`
- `docs/progression/m26-closure-audit.md`
- `docs/milestones/M26-fragments-modules-horizontal-progression.md`
- `docs/milestones/M27-routes-risks-events-replayable-operations.md`
- `docs/operations/m27-block2-route-lab-audit.md`
- `docs/operations/m27-block3-lua-authoring-audit.md`
- `docs/operations/m27-block4-persistence-contract.md`
- `docs/operations/m27-block4-persistence-audit.md`
- `docs/operations/m27-block5-replay-contract.md`
- `docs/operations/m27-block5-replay-audit.md`
- `docs/operations/m27-block6-protocol-contract.md`
- `docs/roadmap-2026-08-30.md`
- `docs/roadmap-remaining-2026-09-01.md`

The main roadmap and M25 milestone now record Phase F complete and Gate M25
green. Preserve the closure audit as the authoritative transition evidence.

## Evidence already accepted

### Combat and presentation

- Candidate catalog: rifle `40 / range 6 / 250 ms`; sidearm
  `25 / range 8 / 150 ms`.
- Solo enemy health: drone 140, Warden 240. Two-player health: 210/360.
- Pressure: one 10-damage drone hit; Warden counters for 15 after accepted hits
  2 and 4, capped at two; optimal path ends at 60 HP.
- Local intent, cooling, unavailable target, confirmed player hit, hostile hit,
  and defeat are semantically distinct. Mute and Reduced Flash pass fixed-pool
  fixtures.
- Canonical A/B captures are in `docs/art/m25/captures` and their manifest
  passes. Current PNG hashes:
  - local attempt:
    `9802cdaa9fa16d14969dd7ff7bc4894ef5f0d60f893f30c2a7c22c8c0114c15f`
  - server-confirmed hit:
    `1c6b14d5b8ca1d67b42dfee30c6dfcaee2d81a78f49535d19610f6777d754c89`

### Accepted runtime matrix

Evidence directory:

`/mnt/c/Users/Ian/revenant-local-evidence/m25-runtime-162628-25628`

- Manifest SHA-256:
  `eaa52e634ffd7c86709ae1a4234da681bdd46d84616c6e1569d816f57f4254c7`.
- 20 sessions, 30 active client accounts, 10 activities per weapon.
- 10 solo and 10 two-active-client sessions.
- Simulated RTT cases 0/75/150 ms; no hit, ordering, projection, or reward
  divergence.
- Exact totals: 30 histories, 30 fragments, 3,000 XP, 30 inventory grants,
  30 progression grants, 20 replay sessions, zero session-integrity mismatch.
- Solo TTK: drone 877-966 ms; Warden 1,469-1,744 ms.
- Two-active TTK: drone 710-817 ms; Warden 1,181-1,453 ms.
- Two/solo ratios are 79.77%-85.01%, above the 45% floor.
- RSS delta: +512 KiB solo, +1,220 KiB two-active. Descriptors, threads, and
  PostgreSQL connections returned exactly to baseline.
- Both gateway logs contain ten resets and zero command/connection/abort
  failure.

Rejected attempts are already explained in the Block 5 audit: the first
two-rifle run correctly exposed that the secondary at distance 8 was outside
rifle range 6; both clients now perform the door movement. A later run passed
all 20 sessions but stopped in the final `jq` aggregation; the expression was
fixed and the whole matrix was repeated with new accounts to produce the
accepted evidence above.

## Final gate already run

The uninterrupted canonical command passed after Block 5:

```bash
PATH=/tmp/revenant-rust.K64VMq/cargo/bin:$PATH \
CARGO_HOME=/tmp/revenant-rust.K64VMq/cargo \
RUSTUP_HOME=/tmp/revenant-rust.K64VMq/rustup \
DATABASE_URL_FILE=/owner-only/path/operator_database_url \
GODOT_BIN="$PWD/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64" \
make check
```

It passed version consistency, Rust formatting, workspace Clippy with warnings
denied, 57 Rust/PostgreSQL tests, all-target build, Inspector TypeScript/build,
secret audit over 287 candidate files, multiplayer smoke, automatic/reused,
manual and keyboard Godot flows, all M17-M25 markers, persistence, replay and
Inspector, frozen V1 compatibility, and standalone V1 reconstruction.

This WSL shell normally has no Rust toolchain. The official Rust 1.98.0
toolchain with rustfmt and Clippy was installed only under
`/tmp/revenant-rust.K64VMq`. If that directory no longer exists, provision a
new official temporary toolchain under `/tmp`; do not change the repository or
user profile merely to resume.

Post-gate checks also passed:

- all M21/M22/M24/M25 capture manifests;
- exact frozen V1 hashes;
- no diff under `runtime/protocol`, `runtime/replay`, `archive/clients/v1`, or
  `VERSION`;
- no `m24_fail_%` PostgreSQL trigger or function;
- healthy compose services bound only to `127.0.0.1`;
- no old gateway, bot, Godot validator, or matrix process;
- no untracked generated Godot UID;
- no tag on `HEAD`, equal local/upstream baseline, and `git diff --check`.

The tracked `client/game/main.gd.uid` was briefly mistaken for generated state
during capture cleanup and was restored exactly to
`uid://b2rg1den8l8rw`; it currently has no diff.

## Explicit clock residual

The final smoke replay printed one `player_joined` timestamp about four minutes
earlier than its later insertion ID. No trigger/function existed, append-ID
ordering, completion, rewards, and replay state were correct, and the accepted
20-session matrix had join-to-start values of 1-102 ms.

A fresh two-active diagnostic session then produced 14 monotonic events,
100 ms join-to-start, 2,942 ms activity duration, two exact rewards, one reset,
and zero gateway failure. Evidence is under
`/tmp/revenant-m25-clock-audit.Kv6EpP`. Treat the earlier line as a
non-reproduced WSL/PostgreSQL wall-clock step and record it as a bounded local
environment residual in the M25 closure; do not claim it did not happen.

## Resume sequence

### R1 — Formally close M25

Completed on 2026-08-31. The seven steps below remain as historical resume
instructions and closure provenance.

1. Re-read this file, the main roadmap, the M25 milestone, and all five M25
   combat audits.
2. Check `git status --short`, `git diff --check`, protected-boundary diffs,
   `HEAD`/`origin/main`, services, failure objects, and active validators.
3. If source did not change after the recorded `make check`, do not rerun it
   merely for ceremony. If any implementation source changed, rerun the full
   canonical command above before closing.
4. Create `docs/combat/m25-closure-audit.md` summarizing Gates 1-5, the full
   gate, protected invariants, rejected evidence, the clock residual, and final
   bounded risks.
5. Set the M25 milestone status to complete and record Gate M25 as green.
6. Check the last Phase F item in `docs/roadmap-2026-08-30.md` and change its
   continuation ledger from M25 Block 6 to M26 specification.
7. Mark the current working plan complete. Do not commit or publish.

Gate M25 should be green unless the resume audit finds a new reproducible
integrity, fairness, projection, compatibility, privacy, resource, or boundary
failure.

### R2 — Start M26 with specification only

Completed on 2026-08-31. The exact hypothesis and bounded specification were
written, structurally reviewed, and Gate 1 approved without implementation,
schema, protocol, or replay changes.

Record exactly this one starting hypothesis before design or code:

> A bounded three-slot module loadout with four tradeoff families can produce
> at least three non-dominated combat builds while combination, equip, grant,
> persistence, migration, and replay arithmetic remain server-authoritative,
> idempotent, and exactly reconstructible.

Then create `docs/milestones/M26-fragments-modules-horizontal-progression.md`
with:

- authority and non-goals;
- fragment/module taxonomy questions, not assumed answers;
- slot, compatibility, arithmetic, and growth bounds;
- persistence/migration/rollback/replay invariants;
- comparison-UI truth boundary;
- generated inventory/build matrix contract;
- block gates and stop conditions;
- an explicit rule that Protocol V2, schema, and replay changes require their
  own reviewed block rather than a silent expansion.

Do not implement modules until the first M26 specification gate is reviewed
green.

### R3 — Execute M26 sequentially

After its specification gate, use small reversible blocks:

1. pure module arithmetic and generated non-dominance matrix;
2. authoritative inventory/combination/loadout domain;
3. transactional persistence, migration, rollback, and idempotency;
4. replay reconstruction and Inspector evidence;
5. additive Protocol V2 and gateway integration;
6. honest Godot comparison/loadout UI;
7. solo and bot-driven runtime/repetition/resource matrix;
8. uninterrupted full gate and M26 closure.

Items 1-8 are complete and green. R3 is closed; resume at R4 with M27
specification only.

Stop immediately on a dominant build, unbounded combination growth, partial
transaction, duplicate grant, replay mismatch, dishonest UI projection,
Protocol/schema expansion outside its reviewed block, or compatibility loss.

### R4 — Continue M27-M32 only behind green gates

After M26 closure, return to `docs/roadmap-2026-08-30.md` and create a detailed
milestone contract before each next implementation. Preserve this order:

1. M27 routes, risks, events, and replayable operations;
2. M28 local cooperation simulation;
3. M29 isolated Echo prototype with an explicit removal criterion;
4. M30 security, observability, backup, and recovery hardening;
5. M31 creator dogfooding and vertical-slice synthesis;
6. M32 evidence-based decision on closed external validation.

No green engineering gate may be relabeled as human preference or external
playtest evidence.

M27 Gate 1 is approved. It freezes exactly two explicit opt-in routes, exactly
two candidate events per route with one resolved event, the unrouted M26
compatibility baseline, a 240-row routed/60-row baseline pure matrix, bounded
server-owned seed/operation/duration/Lua/transaction/replay rules, and separate
later change gates. Before the first M27 implementation edit, create and record
the checksum-verifiable external M26 closure checkpoint. That checkpoint is
complete with the path and digest recorded above. Continue only with Block 2's
dependency-light domain and laboratory; do not edit Lua,
persistence/schema, replay, Protocol/gateway, or Godot in that block.

## First command on resume

```bash
cd /mnt/c/Users/Ian/revenant
sed -n '1,260p' docs/roadmap-resume-2026-08-31.md
git status --short
git diff --check
```

Resume at the first incomplete resume block. The durable continuation point is
R4 / M27 Block 6 bounded implementation under the recorded exact additive
Protocol V2/gateway contract. Gates 1-5 and the external M26 closure checkpoint
are green; do not edit Godot before Gate 6.
