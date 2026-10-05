# M27 Block 8 — runtime, repetition, and resource audit

Date: 2026-09-06  
Gate: 8 — bounded runtime/repetition/resource matrix.  
Decision: **approved with bounded residuals assigned to M27 closure**.

## Executed surface

`tests/m27-runtime-matrix.sh` now drives the exact reviewed Block 8 contract
through freshly built local binaries and isolated loopback listeners. Product
gateway input still has no seed field or seed configuration. A separately
named `revenant-m27-matrix-gateway`, available only with the non-default
`m27-runtime-matrix` Cargo feature, accepts one finite process-local seed
queue, validates every entry as a non-negative 63-bit `RouteSeed`, and consumes
an entry only at the established server-owned selection boundary.

The normal package explicitly keeps `revenant-gateway` as its default run
target. The matrix binary prints its instrumentation banner and shares the
production listener, session, persistence, replay, protocol, and reset logic.
The fake-client additions only assert received server truth. They cannot
provide a seed, resolve an event, choose for another participant, or invent a
reward.

All fragment and module state was earned through normal activity completions
and accepted combination/loadout operations. SQL inspected state but created
no fixture rows. The matrix used accepted post-completion whole-loadout
changes, retried each with the same operation identifier, and admitted the new
profile only in the following session.

## Accepted evidence

The accepted checksum-verifiable record is:

`/mnt/c/Users/Ian/revenant-local-evidence/m27-runtime-0906b8e`

It contains 117 files and 1,561,467 bytes. `SHA256SUMS` verifies from inside
the directory and has SHA-256
`322867bfa3f5ee977743e66c738f844ca1708e694777dd8785c2a0b2c25116ee`.
The run took 213 seconds on isolated `127.0.0.1:17029` and
`127.0.0.1:18029` listeners. Both matrix gateways stopped afterward.

The fresh prefix produced exactly:

| State | Accepted total |
| --- | ---: |
| Accounts | 9 |
| Distinct sessions | 23 |
| Participant joins | 28 |
| Completed sessions / per-character histories | 21 / 26 |
| Routed successes / routed client projections | 8 / 12 |
| Routed timeout failures / incomplete selections | 1 / 1 |
| Compatibility and bootstrap sessions | 14 |
| Replay events | 300 |
| Route rows / route participants | 10 / 14 |

Every one of the 23 sessions was reconciled against direct SQL, independent
current CLI reconstruction, and current-source GET-only Inspector summary and
event responses. The accepted record contains no assertion mismatch.

## Exact successful runtime sample

The eight required routed sessions completed with these authoritative terminal
values:

| Case | Route/event | Participants | Warden health | Accepted hits | Counter damage | Elapsed |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Solo rifle Empty | Breach / `overcharged_armor` / seed 0 | 1 | 288 | 8 | 15 | 2,802 ms |
| Solo sidearm Force+Ward | Breach / `arc_surge` / seed 1 | 1 | 240 | 8 | 20 | 2,430 ms |
| Solo rifle Force | Stabilize / `shielded_channel` / seed 0 | 1 | 240 | 5 | 10 | 2,220 ms |
| Solo sidearm Ward | Stabilize / `residual_feedback` / seed 2 | 1 | 264 | 11 | 15 | 2,929 ms |
| Two-client rifle Empty + Force+Ward | Breach / `overcharged_armor` / seed 0 | 2 | 432 | 10 | 15 | 2,059 ms |
| Two-client sidearm Force+Ward + Ward | Breach / `arc_surge` / seed 1 | 2 | 360 | 13 | 20 | 1,952 ms |
| Two-client rifle Force + Force+Ward | Stabilize / `shielded_channel` / seed 0 | 2 | 360 | 8 | 10 | 1,884 ms |
| Two-client sidearm Ward + Force+Ward | Stabilize / `residual_feedback` / seed 2 | 2 | 396 | 15 | 15 | 2,211 ms |

Every primary observed its exact admitted damage, cooldown, maximum health,
route options, selection, event/effect, objective path, enemy health, accepted
hits, counter pressure, elapsed budget, transitions, reward, and terminal
grants. Each secondary retained its distinct admitted profile while matching
the primary's complete shared truth. All four non-leader attempts were
rejected before the leader selection; all eight leader selections proved first
acceptance, byte-equivalent same-input replay, different-route conflict, one
immutable resolution, and normal completion.

This finite runtime sample does not replace the Gate 2 exhaustive proof. The
operation laboratory was regenerated before and after the runtime work and
was byte-identical at SHA-256
`d90c61d5cc734a98049cb16f73a0500b0bbe418ffd76f4bb9f811515284922a0`.
It retains 240 routed rows, 60 compatibility rows, all four reachable events,
and zero route dominance. The 18-case domain report was also byte-identical at
SHA-256
`111a5dc8a12c137108a182627d53721c13530078139d6f59c7367bceb256a85d`.

## Failure, interruption, and compatibility proof

The real monotonic timeout selected Breach with seed 1 and `arc_surge`, then
failed `reach_relay_door` at 90,412 ms against the exact 90,000 ms budget. It
persisted one `failed_timeout` terminal with three ordered transitions and
`deadline_exceeded`, and emitted no activity completion, history, loot, XP, or
route grant.

The disconnect case durably selected Stabilize with seed 0 and
`shielded_channel`, then ended without a terminal. Replay and Inspector expose
the operation as reconstructibly incomplete with no history or reward. Neither
case could later complete in the same session, and the gateway reset before
the next admission.

Ten ordinary unopted V2 bootstrap completions, one fresh ordinary V2
completion, one current Protocol-V1 probe, and the byte-frozen V1 client all
retained the exact one-fragment/100-XP baseline and wrote no route row or route
event. In the two-client capability mismatch, only the leader opted in; the
route attempt was rejected without seed use or operation reservation, both
clients completed the baseline, and no route state was persisted.

Frozen V1 hashes remain exactly:

- `archive/clients/v1/src/main.rs`:
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`;
- `archive/clients/v1/Cargo.toml`:
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.

## Reward and operation reconciliation

The 26 successful per-character completions produced exactly 26 inventory
grants, 26 progression grants, 24 final fragments, and 2,900 total XP. The
eight routed successes contain 12 participant grants. Failure and incomplete
operations contain none.

The two reused progression characters finished with exactly two module states,
four owned-module rows, four accepted combinations/events, eleven accepted
loadout operations/events, three final equipped slots, and a summed loadout
revision of eleven. Account A finished with nine fragments, revision seven,
and Force+Ward; Account B finished with seven fragments, revision four, and
Force. Same-input retries changed no inventory, ownership, revision, operation,
or replay count.

Every accepted session has at most one terminal, no post-terminal event, and
no partial participant grant. The final integrity query found zero invalid
route/session combinations and the database contained zero installed M27
failure-injection trigger or function.

## Reset and resource bounds

Both gateways stayed well inside every reviewed ceiling:

| Gateway | RSS before/after | Delta | FDs | Threads | DB connections | Resets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Solo | 7,936 / 10,204 KiB | +2,268 KiB | 9 → 9 | 3 → 3 | 3 → 3 | 19 |
| Two-active | 7,808 / 10,368 KiB | +2,560 KiB | 9 → 9 | 3 → 3 | 3 → 3 | 5 |

The 64 MiB RSS ceiling was not approached. No descriptor, thread, or
PostgreSQL connection was retained. Logs contain both exact seed-instrumentation
banners, the expected reset counts, no seed exhaustion, and no unexpected
command, connection, or session-abort failure.

## Integrated and technical gate

After the final accepted matrix, the uninterrupted canonical `make check`
passed version consistency at `0.2.0`, formatting, workspace/all-target/
all-feature Clippy with warnings denied, all 162 Rust/PostgreSQL tests, every
target build, Inspector TypeScript check and production build, the 349-file
secret audit, multiplayer/current-client smoke, four Godot flows, every
M17-M27 semantic marker, frozen-V1 adapter compatibility, and independent V1
reconstruction.

All M21-M27 capture manifests verify. `git diff --check` passes, no untracked
`.gd.uid` exists, `HEAD == origin/main ==
4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`, and no commit, tag, release, public
listener, or public artifact was created.

## Rejected and superseded attempts

Five earlier records remain outside accepted evidence:

1. `0905b8a` stopped after the solo disconnect case and has no final summary or
   checksum manifest. Its already committed synthetic sessions remain valid
   local fixtures, but the incomplete evidence set is not promotable.
2. `0905b8b` reached the two-client group and exposed an admission/projection
   race in the validation client: one peer asserted route state before the
   complete admitted set and then attributed the other participant's accepted
   damage to itself. The harness was corrected to wait for complete shared
   admission and distinguish per-client from shared combat truth.
3. `0905b8c` and `0905b8d` completed their exact matrices, but were
   deliberately superseded after later validation-source and package
   integration adjustments. They remain immutable historical evidence and are
   not cited as the accepted Gate 8 run.
4. The first post-matrix canonical gate found that adding the required-feature
   binary made an existing unqualified `cargo run -p revenant-gateway`
   ambiguous. The product binary itself had not failed. Declaring the normal
   `revenant-gateway` as the package `default-run` restored the established CLI,
   capture, and smoke entry point without enabling matrix features. A fresh
   `0906b8e` matrix and complete uninterrupted gate then passed.

No rejected record was deleted, merged into the accepted manifest, or
presented as product rollback. Each accepted prefix began with zero matching
accounts.

## Bounded residuals entering M27 closure

- Runtime covers the exact eight reviewed successes rather than all 300 pure
  rows; the exhaustive deterministic laboratory remains authority for the
  complete state space.
- The real 90-second failure is one observation strictly beyond the deadline;
  unit/domain/persistence tests remain authority for the exact 90,000 ms
  success and 90,001 ms failure boundary.
- Timing and RSS values describe this local WSL environment only. They are not
  performance or reliability claims for other machines.
- Automation and creator operation prove no outside-player preference,
  comprehension, enjoyment, accessibility, or replay desire.
- The long-lived Compose gateway image predates M27 and was not used as
  current route evidence.

## Gate decision

Gate 8 is **approved with bounded residuals assigned to Block 9**. Both routes,
all four events, both weapons, the required builds, solo/two-active shared
truth, non-leader rejection, immutable retry/conflict, exact rewards,
failure/incomplete withholding, compatibility, replay/Inspector/SQL parity,
reset, repeatability, and resources are green. No integrity,
dominance-bound, projection, timeout, migration, compatibility, privacy, or
resource blocker remains. Block 9 may perform only the complete M27 diff,
evidence, invariant, residual, and closure review before deciding Gate M27.
