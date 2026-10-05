# M26 Block 2 module-laboratory audit

Date: 2026-08-31  
Gate: 2 — pure arithmetic, finite catalog, and non-dominance.  
Decision: **approved**.

## Preserved input checkpoint

Before the first implementation edit, the 290-file M25 closure tree was
archived outside the repository at:

`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m25-closure-45718926-20260831.tar.gz`

It is 9,841,452 bytes and has SHA-256
`25e5eed51c11534569f0daf550af05990f927060c6c5b6766bc62fc22777ea83`.
The rejected positional-option attempt and exact partial-file replacement are
recorded in the M26 milestone. No M26 implementation preceded the accepted
checkpoint.

## Implemented pure boundary

- `revenant-modules` owns the fixed catalog, typed identifiers/families,
  catalog validation, canonical subset generation, checked basis-point
  arithmetic, effective bounds, and Pareto dominance predicate.
- `revenant-module-lab` composes that API with the existing M25 weapon,
  pressure, and combat-analysis APIs. It emits deterministic JSON or a compact
  human-readable table without a clock, socket, database, gateway, replay, or
  Godot dependency.
- The workspace manifest and lockfile include only the new pure crate and CLI
  for this block. Block 2 made no protocol, replay, persistence, gateway,
  client, Inspector, V1, version, or package change.

All percentages are signed basis points, summed before one round-half-up
resolution against the immutable M25 base profile. Range and maximum health
use checked addition. Request order is canonicalized; invalid revision,
catalog, capacity, duplicate, overflow, or effective bound returns an explicit
error rather than saturation.

## Frozen Gate 2 catalog

Every module is a fixed character unlock, costs exactly two existing
`relay_core_fragment` items, and may occur at most once in a canonical
three-entry loadout.

| Item ID | Family | Damage | Cooldown | Range | Max health | Cost |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `module_force_matrix` | Force | +2,000 bp | +2,000 bp | 0 | 0 | 2 |
| `module_tempo_regulator` | Tempo | -1,000 bp | -1,500 bp | 0 | 0 | 2 |
| `module_reach_lattice` | Reach | -1,000 bp | 0 | +2 | 0 | 2 |
| `module_ward_capacitor` | Ward | 0 | +1,000 bp | 0 | +20 | 2 |

Equal recipe cost avoids suggesting a tier order. The modules change different
axes and none is an unconditional upgrade: Force/Ward slow attacks, while
Tempo/Reach reduce damage. No random value, tier, affix, level, instance,
rarity, durability, or second currency exists.

## Accepted deterministic report

The accepted report is `/tmp/revenant-m26-block2.json`, schema
`RevenantModuleLabV1`, catalog revision `m26-v1`. It is 33,626 bytes and has
SHA-256
`b837554d10a90a134cf76e207c586d1cffde357c7594cd429ae434aecb80cd04`.
A byte comparison against a fresh second run passed.

The report contains four catalog rows, 30 build rows, two frontier summaries,
and 28 recipe-boundary rows. Recipe rows produce eight insufficient, eight
eligible, four already-owned, and eight explicitly deferred operation-key
cases; retry/conflict transitions belong to Gate 3 and are not falsely claimed
as implemented here.

The following table is the complete resolved build matrix. `F`, `T`, `R`, and
`W` mean Force, Tempo, Reach, and Ward. TTK starts at the first accepted hit.
Incoming is the combined optimal drone/Warden pressure. Every row is Pareto
non-dominated under the approved damage/range/health/cooldown vector.

| Weapon | Loadout | Damage | Range | Cooldown | HP | Drone hits / TTK | Warden hits / TTK | Incoming | Pareto |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Rifle | empty | 40 | 6 | 250 | 100 | 4 / 750 | 6 / 1,250 | 40 | yes |
| Rifle | F | 48 | 6 | 300 | 100 | 3 / 600 | 5 / 1,200 | 40 | yes |
| Rifle | T | 36 | 6 | 213 | 100 | 4 / 639 | 7 / 1,278 | 40 | yes |
| Rifle | F+T | 44 | 6 | 263 | 100 | 4 / 789 | 6 / 1,315 | 40 | yes |
| Rifle | R | 36 | 8 | 250 | 100 | 4 / 750 | 7 / 1,500 | 40 | yes |
| Rifle | F+R | 44 | 8 | 300 | 100 | 4 / 900 | 6 / 1,500 | 40 | yes |
| Rifle | T+R | 32 | 8 | 213 | 100 | 5 / 852 | 8 / 1,491 | 40 | yes |
| Rifle | F+T+R | 40 | 8 | 263 | 100 | 4 / 789 | 6 / 1,315 | 40 | yes |
| Rifle | W | 40 | 6 | 275 | 120 | 4 / 825 | 6 / 1,375 | 40 | yes |
| Rifle | F+W | 48 | 6 | 325 | 120 | 3 / 650 | 5 / 1,300 | 40 | yes |
| Rifle | T+W | 36 | 6 | 238 | 120 | 4 / 714 | 7 / 1,428 | 40 | yes |
| Rifle | F+T+W | 44 | 6 | 288 | 120 | 4 / 864 | 6 / 1,440 | 40 | yes |
| Rifle | R+W | 36 | 8 | 275 | 120 | 4 / 825 | 7 / 1,650 | 40 | yes |
| Rifle | F+R+W | 44 | 8 | 325 | 120 | 4 / 975 | 6 / 1,625 | 40 | yes |
| Rifle | T+R+W | 32 | 8 | 238 | 120 | 5 / 952 | 8 / 1,666 | 40 | yes |
| Sidearm | empty | 25 | 8 | 150 | 100 | 6 / 750 | 10 / 1,350 | 40 | yes |
| Sidearm | F | 30 | 8 | 180 | 100 | 5 / 720 | 8 / 1,260 | 40 | yes |
| Sidearm | T | 23 | 8 | 128 | 100 | 7 / 768 | 11 / 1,280 | 40 | yes |
| Sidearm | F+T | 28 | 8 | 158 | 100 | 5 / 632 | 9 / 1,264 | 40 | yes |
| Sidearm | R | 23 | 10 | 150 | 100 | 7 / 900 | 11 / 1,500 | 40 | yes |
| Sidearm | F+R | 28 | 10 | 180 | 100 | 5 / 720 | 9 / 1,440 | 40 | yes |
| Sidearm | T+R | 20 | 10 | 128 | 100 | 7 / 768 | 12 / 1,408 | 40 | yes |
| Sidearm | F+T+R | 25 | 10 | 158 | 100 | 6 / 790 | 10 / 1,422 | 40 | yes |
| Sidearm | W | 25 | 8 | 165 | 120 | 6 / 825 | 10 / 1,485 | 40 | yes |
| Sidearm | F+W | 30 | 8 | 195 | 120 | 5 / 780 | 8 / 1,365 | 40 | yes |
| Sidearm | T+W | 23 | 8 | 143 | 120 | 7 / 858 | 11 / 1,430 | 40 | yes |
| Sidearm | F+T+W | 28 | 8 | 173 | 120 | 5 / 692 | 9 / 1,384 | 40 | yes |
| Sidearm | R+W | 23 | 10 | 165 | 120 | 7 / 990 | 11 / 1,650 | 40 | yes |
| Sidearm | F+R+W | 28 | 10 | 195 | 120 | 5 / 780 | 9 / 1,560 | 40 | yes |
| Sidearm | T+R+W | 20 | 10 | 143 | 120 | 7 / 858 | 12 / 1,573 | 40 | yes |

The complete envelope is:

- drone TTK 600-990 ms against the approved 600-1,200 ms budget;
- Warden TTK 1,200-1,666 ms against the approved 1,200-2,600 ms budget;
- exact combined optimal incoming damage 40 for all rows, below every
  100/120 effective maximum health;
- damage 20-48, range 6-10, cooldown 128-325 ms, and health 100-120, all inside
  the wider specification safety bounds; and
- 15 of 15 non-dominated builds for each weapon, above the required minimum of
  three, with no build dominating all alternatives.

The all-frontier result follows directly from explicit sacrifices on each axis.
It is a useful anti-dominance fact, not proof that all 15 choices feel equally
meaningful to a person.

## Rejected evidence

1. The first relevant command passed all nine new tests, but Clippy rejected a
   typed catalog lookup that used `expect` without a panic contract and found
   one unused CLI import. The lookup became an exhaustive enum match and the
   import was removed before evidence was regenerated.
2. The next deterministic JSON had SHA-256
   `a1b48bd70467a3b946f9279a690f7f59d5854c69ccc9128b46bc0bbaca52da29`.
   Arithmetic passed, but semantic inspection found that Serde emitted
   `force_matrix` while inventory/build identity was
   `module_force_matrix`. All four enum wire names were made identical to
   their catalog item IDs and an exact serialization fixture was added. The
   earlier JSON is rejected and not Gate 2 evidence.

## Quality evidence

Using the official temporary Rust 1.98.0 toolchain under
`/tmp/revenant-rust.K64VMq`, the accepted block passed:

- `cargo fmt --all -- --check`;
- workspace Clippy for all targets with warnings denied;
- all 66 workspace unit, PostgreSQL integration, compatibility, CLI, and doc
  tests, including the nine new module/lab tests; and
- `cargo build --workspace --all-targets`.

`git diff --check` is green. Protocol, replay, frozen V1, and `VERSION` remain
unchanged. The complete `make check`, Godot, secret, smoke, and standalone V1
gate remain mandatory for M26 closure rather than being misreported as part of
this pure block.

## Residuals entering Block 3

- Recipe eligibility is arithmetic-only here. Ownership mutation,
  idempotency-key retry/conflict, revision, and lifecycle transitions remain
  unimplemented until the authoritative pure domain in Block 3.
- Effective values have not entered gateway combat, persistence, replay,
  Protocol V2, Inspector, or Godot.
- Equal two-fragment recipes and Pareto results are engineering constraints,
  not an economy, preference, or retention claim.
- Runtime cadence, actor health, multiplayer, migration, reconnect, and UI
  evidence remain behind their explicit later gates.

## Gate 2 decision

Gate 2 is **approved**. The four-entry catalog, `m26-v1` revision, equal
two-fragment recipes, signed modifiers, round-half-up arithmetic, canonical
15-build state space, and effective safety envelope are frozen for M26. Block 3
may implement only pure authoritative ownership/combination/loadout/revision
semantics. Schema, persistence, replay, protocol, gateway, Inspector, and
client work remain unauthorized until their ordered blocks.
