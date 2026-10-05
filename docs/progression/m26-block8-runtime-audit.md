# M26 Block 8 runtime, repetition, and resource audit

Date: 2026-09-01  
Gate: 8 — bounded runtime/repetition/resource matrix.  
Decision: **approved with bounded residuals assigned to M26 closure**.

## Executed surface

`tests/m26-runtime-matrix.sh` runs a reproducible current-binary matrix on
isolated loopback gateway ports. It creates all accepted fragment/module state
through authoritative activities and protocol requests; it performs no SQL
fixture mutation. Two reused characters each complete five activities before
combining Force and Ward. The fifth completion exposes exactly five fragments,
then the established real-wire flow exercises preview, active lock, malformed
and unknown input, duplicate/fourth selection, insufficient fragments,
already-owned state, stale revision, retry, and operation conflict before
accepting two combinations and Force+Ward revision one.

The matrix then admits the empty baseline plus Force, Ward, and Force+Ward for
both weapons. Accepted post-completion loadout changes are retried with the
same operation identity, must return `replayed=true`, and become active only
after reconnect. No transition asks for its already-active build. The fake
client validates all four exact profile families, its own admitted maximum
health, and the peer's observed 100/120 maximum-health boundary. It retains the
M25 matrix schema marker so the prior M25 tooling remains compatible, with
additive M26 build/health/next-loadout evidence fields.

This instrumentation changes no production catalog, domain rule, persistence,
schema, replay vocabulary, Protocol V2 type, gateway lifecycle, Inspector,
Godot presentation, frozen V1 file, package, or version.

## Accepted evidence

The accepted checksum-verifiable record is:

`/mnt/c/Users/Ian/revenant-local-evidence/m26-runtime-0901b8d`

It contains 125 files and 697,770 bytes. `SHA256SUMS` verifies from inside the
directory and has SHA-256
`803571b7d9664b3f60b46d924485abeabd06c47260488c76e67eefa9d5212a5b`.
The run took 131 seconds and used only `127.0.0.1:17028`,
`127.0.0.1:18028`, and the existing loopback PostgreSQL endpoint. Both
temporary gateways stopped afterward.

The accepted run contains:

- 27 sessions and 35 active-client completions across four synthetic accounts;
- 19 solo sessions, including ten authoritative bootstrap activities, eight
  build/weapon matrix activities, and one V1 activity on a modular account;
- eight two-active sessions and 16 two-active client projections;
- 12 client projections per weapon: one empty, four Force, three Ward, and
  four Force+Ward; and
- two distinct-build sessions, one per weapon, in which Force and Force+Ward
  dealt the same authoritative damage while retaining different cooldown and
  maximum-health tradeoffs.

The two-active distinct sessions are
`session-1788262112817535269` for rifle and
`session-1788262130857511789` for sidearm. Both clients observed identical
enemy-health, hit-owner, hostile-pressure, player-health, reward, and terminal
state. The client-specific admitted fields remained honestly different.

## Exact combat projection

Every accepted damage event matched its admitted profile. The exact observed
hit counts were:

| Weapon/build damage | Solo drone/Warden | Two-active drone/Warden |
| --- | ---: | ---: |
| Rifle Empty/Ward, 40 | 4 / 6 | 6 / 9 |
| Rifle Force/Force+Ward, 48 | 3 / 5 | 5 / 8 |
| Sidearm Empty/Ward, 25 | 6 / 10 | 9 / 15 |
| Sidearm Force/Force+Ward, 30 | 5 / 8 | 7 / 12 |

The admitted cooldowns remained exactly rifle `250/300/275/325 ms` and
sidearm `150/180/165/195 ms` for Empty/Force/Ward/Force+Ward. Empty and Force
admitted 100 maximum health; Ward and Force+Ward admitted 120. Every encounter
retained one 10-damage drone hit and two 15-damage Warden hits, every active
actor contributed an accepted hit to both enemies, and no player died.

This runtime sample does not replace the Gate 2 exhaustive proof. The pure
matrix and final module tests continue to cover all 15 canonical builds per
weapon, full arithmetic bounds, non-lethal pressure, and the Pareto frontier.

## Inventory, operation, and database reconciliation

The accepted prefix did not exist before the run. Final exact totals were:

| State | Accepted total |
| --- | ---: |
| Accounts | 4 |
| Activity histories / inventory grants / progression grants | 35 / 35 / 35 |
| Distinct replay sessions | 27 |
| Fragment balance across the four accounts | 27 |
| Experience | 3,500 |
| Materialized module states / owned module rows | 2 / 4 |
| Final loadout slots | 3 |
| Accepted combination operations/events | 4 / 4 |
| Accepted loadout operations/events | 21 / 21 |
| Sum of loadout revisions | 21 |

Account A finished with 16 fragments, revision 13, and Force+Ward. Account B
finished with nine fragments, revision eight, and Force. Those values equal
all 35 committed rewards minus exactly four fragments consumed per modular
account. Same-input retries added no fragment consume, module grant, revision,
operation, or replay event.

All 27 sessions passed independent CLI reconstruction and current Inspector
summary/event decoding. Their 361 events contain exactly 35 module snapshots,
four combinations, 21 loadout changes, 35 loot grants, 35 progression grants,
and one completed activity per session. The Inspector reported join-to-start
values of 8-114 ms and activity durations of 2,647-6,520 ms. Every event-count,
participant, enemy, boss, equipment, reward, snapshot, combination, and
loadout count matched the independently queried database.

## Frozen V1 compatibility

After Account A had persisted Force+Ward revision seven, a real V1 gameplay
probe admitted it with base rifle damage 40, 250 ms cooldown, and 100 maximum
health. Session `session-1788262092314577456` completed with the exact base
damage sequence and one normal reward. Its replay snapshot explicitly records
`protocol_generation=v1`, the persisted Force+Ward loadout, empty applied
loadout, inactive module effects, and base effective profiles. Revision,
slots, ownership, and operation rows were equal before and after the V1 flow.

The final integrated smoke also passed the byte-frozen V1 client through the
current compatibility adapter and the independent reconstructed V1 backend.
The frozen source hashes remain
`4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`
and
`c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.

## Reset and resource bounds

Both isolated gateways returned descriptors, threads, and PostgreSQL
connections exactly to their pre-matrix values:

| Gateway | RSS before/after | Delta | FDs | Threads | DB connections | Resets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Solo | 6,656 / 7,928 KiB | +1,272 KiB | 9 → 9 | 3 → 3 | 3 → 3 | 19 |
| Two-active | 6,784 / 8,440 KiB | +1,656 KiB | 9 → 9 | 3 → 3 | 3 → 3 | 8 |

The accepted 64 MiB RSS-growth ceiling was not approached. Both logs contain
the exact reset count and zero command, connection, or abort failure. No
matrix, bot, Godot validator, or isolated listener remained after validation.

## Integrated and technical gates

After the final source and accepted runtime evidence, the integrated smoke
passed multiplayer, automatic/reused, manual, keyboard-only, all M17-M26
semantic markers, persistence, replay, current Inspector, frozen V1
compatibility, and standalone V1 reconstruction.

The technical gate then passed formatting, workspace/all-target/all-feature
Clippy with warnings denied, all 98 Rust/PostgreSQL tests, every-target build,
Inspector TypeScript check, and production Vite build. The final invariant
audit passed version `0.2.0`, the 312-file secret audit, M21/M22/M24/M25/M26
capture manifests, frozen V1 hashes, zero failure-injection trigger/function,
`git diff --check`, no generated untracked Godot UID, unchanged
`HEAD == origin/main == 4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`, and no
commit, tag, release, or public exposure.

## Rejected harness attempts

Three isolated prefixes were not promoted to Gate 8 evidence:

1. `0901b8a` reached a valid V1 completion, but the harness searched for an
   obsolete prose line instead of the emitted `M26_V1_PROBE` marker.
2. `0901b8b` assumed the V1 replay was legacy and snapshot-free. The actual
   approved behavior is the stronger explicit V1 inactive-module snapshot
   described above. A read-only current-Inspector probe confirmed the decoded
   shape before the assertion was corrected.
3. `0901b8c` completed all 27 runtime sessions and every exact database total,
   but its final generic integrity query expected one equipment event from the
   V1 default-rifle flow. The corrected query requires equipment events equal
   participant joins minus explicit V1 snapshots and also requires one module
   snapshot per join.

These were harness-expectation failures after valid atomic gameplay, not
product rollback or projection failures. Their synthetic rewards remain valid
local rows under disjoint prefixes. The accepted `0901b8d` run began with zero
accounts under its own prefix and repeated the complete matrix after all three
corrections.

## Residuals entering M26 closure

- Runtime samples four builds rather than all 15; exhaustive pure/domain tests
  remain the authority for the complete bounded state space.
- The matrix is deterministic creator-operated automation. It proves no human
  preference, comprehension, accessibility across hardware, or long-term
  enjoyment.
- The prior WSL/PostgreSQL wall-clock-step residual still exists as an
  environmental possibility. All accepted Gate 8 Inspector timings were
  monotonic, but this does not erase the earlier observation; append IDs remain
  replay authority.
- Hardware audio perception and creator presentation preference remain the
  bounded Gate 7 residuals. The long-lived Compose gateway image still
  predates M26 and was not treated as current evidence.

## Gate decision

Gate 8 is **approved with bounded residuals assigned to Block 9**. Fresh and
reused progression, real combination/preview/loadout boundaries, reconnect
activation, idempotent retry/conflict, empty plus three tradeoff builds per
weapon, solo and distinct-build two-active combat, exact rewards/state,
replay/Inspector parity, V1 compatibility, resets, and resources are green.
No integrity, dominance-bound, projection, migration, compatibility, privacy,
or resource blocker remains. Block 9 may perform only the complete M26 diff,
evidence, invariant, residual, and closure review before deciding Gate M26.
