# M28 Block 2 — Pure cooperation domain and laboratory audit

Date: 2026-09-06  
Decision: **Gate 2 approved**  
Accepted evidence: `/mnt/c/Users/Ian/revenant-local-evidence/m28-block2-0906g2c`

## Reviewed boundary

Block 2 adds only the workspace members `revenant-cooperation` and
`revenant-cooperation-lab`, plus the corresponding workspace/lock entries. The
runtime crate depends on `serde` only (`serde_json` is test-only). The lab reads
the accepted M25/M26 combat inputs from AI, inventory, and modules, but no M28
code depends on persistence, replay, protocol, gateway, filesystem, network,
client, or a clock.

The external M27/Gate-1 checkpoint was verified before the first implementation
change:

- path: `/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m27-closure-45718926-20260906.tar.gz`;
- entries: 352 source / 352 archive, with zero path differences;
- size: 10,684,040 bytes; and
- SHA-256: `81bb1fafaa397b27db7fc505767c09202d68b77e13355952b49ad9d994280d70`.

A current-to-checkpoint comparison after implementation reports exactly four
new Block-2 files, no missing path, and changes only to `Cargo.toml`,
`Cargo.lock`, `AGENTS.md`, the M28 milestone, and the two roadmap ledgers. No
M27 source, Lua, migration, replay, protocol, gateway, Godot, or frozen-V1 file
changed after the checkpoint.

## Pure domain review

`runtime/cooperation` freezes catalog revision `m28-v1` and represents the
complete finite state in typed values:

- immutable admission-order roles `anchor` and `runner` with exact combat
  profile, admitted health, current health, and life state;
- phases `awaiting_anchor`, `awaiting_ping`, `awaiting_runner`,
  `runner_downed`, `revive_channel`, `encounter_active`, and terminal success
  or failure;
- fixed semantic targets, contribution flags, one ping record, one active or
  completed revive record, revive count, terminal reason, and success-only
  reward;
- 1-32-byte ASCII alphanumeric/hyphen start, ping, and revive operation IDs,
  with same-input replay and conflict/second-operation rejection;
- caller-supplied elapsed milliseconds only, with no system-clock access;
- exact inclusive boundaries of 5,000 ms ping TTL, 15,000 ms revive window,
  2,000 ms revive channel, 60,000 ms operation budget, and squared revive
  distance at most four; and
- checked health/count/deadline/cardinality arithmetic and fixed-size
  participant/reward storage.

The scripted hazard subtracts the runner's exact admitted current health to
zero rather than assuming maximum health. The one successful revive restores
exactly 50 health, bounded by the admitted maximum. Any later defeat is
terminal; there is no second downing or revive. Failure, abandonment, and
incomplete state expose no reward. Success requires all five distinct
contribution facts and returns two fragments plus 125 XP to each of exactly two
participants.

The crate's 18 tests cover the success path, immutable order/profile/health,
operation-ID and profile/health bounds, wrong role/target/order, exact and
plus-one timer boundaries, timeout precedence, exact hazard arithmetic, revive
authority/range/channel/pending/cancellation/retry/conflict, second
ping/down/revive rejection, both participant defeats, both disconnect
identities at every nonterminal phase, terminal replay/post-terminal rejection,
and a serializable nonterminal crash snapshot.

## Generated matrix

`matrix-a.json` and `matrix-b.json` are byte-identical. Each uses schema
`RevenantCooperationLabV1`, cooperation catalog `m28-v1`, and the unchanged
module catalog `m26-v1`.

- Exact cardinality: `2 × 15 × 2 × 15 = 900` unique ordered
  anchor/runner weapon/build pairs.
- Every row retains the exact admitted profile and maximum health (100 or 120),
  all five required contributions, hazard damage equal to current health,
  downed health zero, revived health 50, revive count one, and terminal success.
- Every row retains the existing two-participant baseline enemy values: Drone
  health 210, Warden health 360, and Warden damage 15.
- The unchanged Warden pressure ceiling is two counterattacks × 15 damage = 30,
  strictly below revived health 50 for every row.
- Every row has a null M27 route effect and the same two-fragment/125-XP reward
  for each participant.
- Matrix JSON size: 1,376,849 bytes.
- Matrix SHA-256:
  `513de2fb226458d28d46d945f650ae482d18ad388d4f1ab8d30d2e8ae830ceb5`.

The lab has four additional tests proving exact cardinality, uniqueness of all
900 ordered pairs, per-row hazard/revive/reward/pressure invariants, and stable
serialization.

## Lifecycle report

`domain-a.json` and `domain-b.json` are byte-identical under schema
`RevenantCooperationDomainLabV1`. The accepted report contains 50 named vectors
(35,213 bytes):

- start, ping, revive, and terminal same-input replay plus identifier/profile/
  admitted-health conflict;
- role, target, distance, order, second ping/down/revive, active-channel, and
  time-reversal rejection with unchanged state;
- exact and plus-one ping, revive-window, revive-channel, and operation
  boundaries, including revive completion exactly at its window;
- in-range pending/completion and out-of-range cancellation followed by reuse
  of the unreserved revive ID;
- both participant-defeat terminals and success replay;
- one reconstructibly incomplete revive-channel snapshot; and
- 12/12 abandonment terminals: both immutable identities at all six
  nonterminal phases.

The three plus-one vectors commit exactly `failed_ping_timeout`,
`failed_revive_timeout`, and `failed_operation_timeout`. All 17 rejected input
vectors leave state unchanged; deadline terminals are separately and honestly
classified as applied mutations.

- Domain SHA-256:
  `34fa219abb14a98e642a622615d17b0f5d7c71f7a7c796853b751cde3c06c261`.
- Evidence-manifest SHA-256:
  `f51cf6a76e37e654d65670c109c46e08976f2202f6cb71a393aebdde991f9fcf`.
- `sha256sum -c SHA256SUMS` passes for both independent matrix and domain
  copies.

## Rejected and superseded evidence

- `m28-block2-0906g2` is rejected. Its disconnect vector used 10,000 ms for
  every phase, so the two `awaiting_runner` cases correctly timed out the live
  ping instead of testing abandonment. This was found by querying terminal
  distribution, not hidden by case names.
- `m28-block2-0906g2b` corrects all 12 disconnect instants and is internally
  valid, but is superseded because the pure domain was subsequently tightened
  to preserve exact admitted current health, start-profile/health idempotency,
  and an explicit exact revive-window-completion vector.
- Only `m28-block2-0906g2c` is accepted for Gate 2.

## Canonical quality gate and protected invariants

The uninterrupted canonical command

```text
CARGO_HOME="$PWD/.tooling/cargo" \
RUSTUP_HOME="$PWD/.tooling/rustup" \
PATH="$PWD/.tooling/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" \
GODOT_BIN="$PWD/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64" \
make check
```

passed after the accepted report was generated: version consistency at 0.2.0,
formatter, workspace/all-target/all-feature Clippy with warnings denied, all
184 Rust tests, all-target build, Inspector TypeScript/build, secret audit over
356 candidates, complete multiplayer/Godot/persistence/replay/Inspector smoke,
current Protocol V2, frozen V1 compatibility, and standalone V1
reconstruction.

Protected hashes remain:

- frozen V1 main:
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`;
- frozen V1 manifest:
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`;
- routed Lua:
  `89b8d48ca4906ff484c3c60f1719e93f99a67fe77fae3f3b34073739031265a5`;
  and
- M27 migration 0007:
  `8f5a3bcf9d22e6b9b3bcfd7175a47365ed2b27ef59003d84586d781e35f55b6a`.

## Decision and residuals

Gate 2 is green. The evidence proves the finite pure proposition, exact ordered
state space, distinct contributions, nonlethal retained pressure, fixed
success-only reward, timing/authority/idempotency bounds, deterministic hashes,
and zero M27 mutation.

The residual is intentionally large but bounded: this is a pure model and a
generated laboratory, not durable or live-wire proof. Database constraints,
transaction rollback/concurrency, replay reconstruction, protocol parsing,
gateway clocks, two sockets, bots, Godot projection, recovery, and resources
remain unproven and locked behind Blocks 3-8. No human cooperation, social,
anti-griefing, comprehension, preference, accessibility, or enjoyment claim is
made.

Only Block 3's exact additive persistence contract review is now active. No DDL
may be applied until table/constraint/transaction/rollback/idempotency shapes
are reviewed; replay vocabulary remains locked until Gate 3 is green.
