# M25 closure audit

Date: 2026-08-31  
Gate: M25 — combat pleasure and fairness closure.  
Decision: **approved with bounded residual risk (green)**.

## Scope and evidence reviewed

The closure review covered the complete M25 implementation diff, the
milestone contract, and every accepted block audit:

- `m25-untuned-baseline.md`: exact 24-row baseline and Gate 1 laboratory;
- `m25-block2-cadence-audit.md`: authoritative cadence, weapon identity, and
  Gate 2 runtime evidence;
- `m25-block3-pressure-audit.md`: server-owned pressure, terminal boundaries,
  and Gate 3 fixtures;
- `m25-block4-presentation-audit.md`: honest local/authoritative presentation,
  bounded audiovisual pools, and Gate 4 captures;
- `m25-block5-runtime-matrix-audit.md`: 20-session solo/two-active/delay
  matrix, exact persistence/replay reconciliation, and Gate 5 resources.

The final combat-lab output at `/tmp/revenant-m25-final-lab.json` has SHA-256
`d03fe0ce9b5999a77f7ab05dafa7b68f56f82e37258e91b0740b2bc03874bb7e`.
Its final candidate agrees with runtime evidence: rifle `40 / range 6 /
250 ms`, sidearm `25 / range 8 / 150 ms`, solo enemy health `140/240`,
two-active health `210/360`, one 10-damage drone attack, and two bounded
15-damage Warden counterattacks.

## Gate results

| Gate | Accepted result |
| --- | --- |
| 1 — baseline | Pure deterministic analysis reproduced the untuned game, exposed the fixed client-cadence defect, and bounded the first candidate without changing gameplay. |
| 2 — cadence and identity | The local gate follows the authoritative profile; switching cannot reset an active server cooldown; both weapons retain damage/cadence/range tradeoffs. |
| 3 — enemy pressure | Pressure is server-owned, ordered after accepted hits, non-lethal on the optimal path, capped, and absent after death, reset, completion, or target loss. |
| 4 — presentation | Attempt, cooling, unavailable target, confirmed player hit, hostile hit, and defeat are distinct; confirmation remains authoritative; mute, Reduced Flash, and fixed pools pass. |
| 5 — runtime matrix | 20 fresh sessions and 30 active accounts passed exact combat, two-attacker participation, 0/75/150 ms timing envelopes, rewards, replay, Inspector, reset, and bounded-resource assertions. |

The accepted matrix remains at
`/mnt/c/Users/Ian/revenant-local-evidence/m25-runtime-162628-25628`.
It contains 151,724 bytes of accepted evidence. The SHA-256 of its
`SHA256SUMS` is
`eaa52e634ffd7c86709ae1a4234da681bdd46d84616c6e1569d816f57f4254c7`,
and every manifest entry passed `sha256sum --check` during this closure.

Measured solo TTK was 877-966 ms for the drone and 1,469-1,744 ms for the
Warden. Two-active TTK was 710-817 ms and 1,181-1,453 ms respectively, with
two/solo ratios of 79.77%-85.01% against the 45% floor. All 30 participants
received exactly one fragment and 100 XP; the database contains 30 histories,
30 inventory grants, 30 progression grants, 20 replay sessions, and zero
integrity mismatch. Idle RSS deltas were +512 KiB solo and +1,220 KiB
two-active, with descriptors, threads, and database connections returning
exactly to baseline.

## Complete quality gate

The uninterrupted canonical `make check` recorded after Gate 5 passed:

```bash
PATH=/tmp/revenant-rust.K64VMq/cargo/bin:$PATH \
CARGO_HOME=/tmp/revenant-rust.K64VMq/cargo \
RUSTUP_HOME=/tmp/revenant-rust.K64VMq/rustup \
DATABASE_URL_FILE=/owner-only/path/operator_database_url \
GODOT_BIN="$PWD/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64" \
make check
```

That run passed version consistency, Rust formatting, workspace Clippy with
warnings denied, 57 Rust/PostgreSQL tests, all-target build, Inspector
TypeScript/build, the 287-file secret audit, multiplayer smoke, automatic,
reused, manual and keyboard Godot paths, all M17-M25 markers, persistence,
replay, Inspector, frozen V1 compatibility, and standalone V1 reconstruction.

The resume audit found no implementation change after that run, so the full
gate was not repeated for ceremony. Documentation-only closure edits were
checked separately with `git diff --check` and final status inspection.

## Protected invariants

The closure audit reconfirmed:

- `HEAD` and `origin/main` both remain
  `4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`, with no tag on `HEAD`;
- `VERSION` remains `0.2.0`;
- no diff exists under `runtime/protocol`, `runtime/replay`,
  `archive/clients/v1`, or `VERSION`;
- frozen V1 hashes remain
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`
  and
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`;
- all M21, M22, M24, and M25 capture manifests pass;
- PostgreSQL, gateway, and Inspector are healthy and expose ports only on
  `127.0.0.1`;
- no `m24_fail_%` trigger/function, stale validator, old gateway/bot process,
  or untracked Godot UID remains;
- the frozen M24 checkpoint/package identities remain unchanged; and
- no commit, push, merge, version change, tag, release, publication, external
  participant, or public service exposure occurred.

The tracked `client/game/main.gd.uid`, briefly mistaken for generated capture
state, remains restored exactly as `uid://b2rg1den8l8rw` with no diff.

## Rejected evidence and corrections

Rejected attempts remain part of the engineering history, never part of the
accepted counts:

- an over-32-character synthetic identifier was correctly refused before
  gameplay, producing no session or reward;
- a cadence rehearsal invoked a stale plain bot binary after `cargo test` had
  rebuilt only a hashed test executable; an explicit build and fresh accounts
  produced the accepted rifle and sidearm evidence;
- early pressure runs exposed and corrected target-selection and parser
  defects before Gate 3;
- the first two-rifle matrix correctly rejected the secondary actor at range
  8 against rifle range 6; both actors now execute the authoritative door
  movement; and
- a later 20-session run passed gameplay but failed its final `jq` aggregation;
  the expression was corrected and the entire matrix was repeated with fresh
  accounts before acceptance.

Legitimate rewards from completed rejected harness runs were neither rewritten
nor deleted.

## Residual risks

- One final-smoke replay printed a `player_joined` wall-clock timestamp about
  four minutes earlier than its later insertion ID. Ordering, completion,
  rewards, and replay state were correct; no injection object existed. The
  accepted matrix had 1-102 ms join-to-start values, and a fresh two-active
  diagnostic produced 14 monotonic events, 100 ms join-to-start, 2,942 ms
  activity duration, two exact rewards, one reset, and zero failure. The
  earlier value is retained as a non-reproduced WSL/PostgreSQL wall-clock-step
  residual, not erased or generalized away.
- The RTT fixture is a deterministic loopback timing envelope, not packet
  loss, jitter, congestion, or remote-network evidence.
- Ten activities per weapon and the recorded idle deltas are bounded local
  engineering evidence, not a long-duration soak.
- No creator preference was supplied. Human feel, first-contact comprehension,
  external hardware, gamepad, accessibility experience, and population
  preference remain unvalidated and are not claimed.

None is a reproducible integrity, fairness, projection, compatibility,
privacy, resource, or authority blocker within M25 scope.

## Gate M25 decision and continuation

Gate M25 is **green, approved with bounded residual risk**. The M25 definition
of done is satisfied: laboratory/runtime arithmetic agrees, neither weapon
dominates every measured dimension, pressure and confirmation remain
authoritative, the solo/two-active/delay matrix passes, compatibility remains
green, and claims remain within the evidence.

Exactly one M26 starting hypothesis is recorded:

> A bounded three-slot module loadout with four tradeoff families can produce
> at least three non-dominated combat builds while combination, equip, grant,
> persistence, migration, and replay arithmetic remain server-authoritative,
> idempotent, and exactly reconstructible.

This decision permits the already authorized M26 specification block. It does
not itself authorize module implementation before that specification gate is
reviewed green, nor any commit, publication, version, protocol, schema, replay,
or public-exposure change.
