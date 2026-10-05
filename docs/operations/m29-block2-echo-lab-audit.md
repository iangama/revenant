# M29 Block 2 — Echo value laboratory and early-removal audit

Date: 2026-09-07  
Decision: **numeric keep gate failed; prototype removed; Gate M29 complete**  
Accepted evidence:
`/mnt/c/Users/Ian/revenant-local-evidence/m29-block2-echo-lab-20260907b`

The accepted directory contains 32 manifested files (6,082,139 bytes).
`sha256sum --check SHA256SUMS` passes, and the manifest SHA-256 is
`dc1b5b1cd090fce5367f2de2fdb95214bdd8860488d9518ca359abd3bb6cdee7`.

## Scope reviewed

Block 2 temporarily added only two off-by-default workspace members:
`revenant-echo-prototype` and `revenant-echo-lab`. The pure domain represented
one owner, one accepted movement, one accepted nonlethal Warden attack, one
playback use, five terminals, and checked 5,000/15,000/250/500 ms boundaries.
The lab read the frozen M25/M26 profiles and had no persistence, replay,
protocol, gateway, bot, Godot, network, wall-clock, or working-database
dependency.

The four temporary source files are preserved only in the external evidence
archive `pure-prototype-source.tar.gz`, with individual hashes in
`prototype-source.sha256`. They are deliberately absent from the repository
after this audit. Candidate `m29-block2-echo-lab-20260907a` is rejected: its
shell `tee` created the stdout log before the lab's empty-directory check, so
the lab refused to emit any report. Candidate `...20260907b` is the accepted
run. Inside the accepted directory, `restoration-before-docs.log` and
`restoration-accepted.log` are superseded directory-level comparisons that
encountered generated `.godot` and empty ignored directories;
`restoration-current-files.txt` likewise included ignored web build outputs.
The accepted Git-aware comparison is `restoration-source-proof.log` with its
two `restoration-*-source-files.txt` inventories.

## Deterministic reports

The accepted value report uses schema
`revenant.m29.echo-value-matrix.v1` and contains exactly 2,790 rows:

- 30 solo owner profiles and 900 ordered two-player owner/peer pairs;
- baseline, immediate-use, and held-finisher strategies for every encounter;
- all two weapons and all fifteen canonical M26 builds; and
- complete health, direct/Echo attack, completion-time, hostile-pressure, and
  counter evidence per row.

Its SHA-256 is
`35b4cd8e4cb0fdfbc97c7a70dd5addb254000ecb466b7fcb871d4fb8f6d5ea43`.
The 50-case lifecycle report has SHA-256
`cca53dc230e8561aa01750b3cd6746fccee09aead0ca69009d3bc7c330159f9b`.
The explicit gate decision has SHA-256
`6a642fee356c207754acf9f77e26b2c78aea435b43bf906183f71e74bc710269`.
A fresh second run reproduced all three files byte for byte.

All 50 lifecycle cases pass, with ten cases in each frozen category:
lifecycle/idempotency, eligibility, action ordering, timing boundaries, and
failure/disconnect/reset. The report observes all five terminals and never
accepts a second damage application.

The existing pure reports also reproduce at their accepted hashes:

| Baseline | SHA-256 |
| --- | --- |
| M25 final combat report | `d03fe0ce9b5999a77f7ab05dafa7b68f56f82e37258e91b0740b2bc03874bb7e` |
| M26 module report | `b837554d10a90a134cf76e207c586d1cffde357c7594cd429ae434aecb80cd04` |
| M27 route matrix | `d90c61d5cc734a98049cb16f73a0500b0bbe418ffd76f4bb9f811515284922a0` |
| M27 route domain | `111a5dc8a12c137108a182627d53721c13530078139d6f59c7367bceb256a85d` |
| M28 cooperation matrix | `513de2fb226458d28d46d945f650ae482d18ad388d4f1ab8d30d2e8ae830ceb5` |
| M28 cooperation domain | `34fa219abb14a98e642a622615d17b0f5d7c71f7a7c796853b751cde3c06c261` |

## Frozen keep-threshold result

The threshold was applied without tuning it after observing the result:

| Requirement | Required | Observed | Result |
| --- | ---: | ---: | --- |
| Complete finite value rows | 2,790 | 2,790 | pass |
| Lifecycle cases | 50 | 50 | pass |
| Baseline/pressure-safe rows | 2,790 | 2,790 | pass |
| Solo immediate improvement by at least one owner cooldown | at least 24/30 | 30/30 | pass |
| Successful Echo uses with exact one-direct-attack reduction | every successful use | 1,560/1,805 | **fail** |
| Two-player immediate/held non-dominated tradeoffs | at least 60/900 | 0/900 | **fail** |

Every successful Echo applied exactly one copy of the owner's recorded damage,
but the tactical value was not stable. Of the 1,805 successful executions,
1,560 removed one direct attack, 180 removed two, ten removed three, and 55
removed none. Immediate use executed in all 930 eligible encounter shapes.
Held use executed in 875; in 55 shapes the ordinary participants defeated the
Warden before a valid held execution.

The decisive product failure is dominance, not lifecycle safety. Immediate use
dominates held use on completion/owner-pressure/peer-pressure in 845 of 900
ordered pairs; baseline also dominates held use in those same 845 pairs.
Held use dominates immediate use in zero pairs. The remaining 55 held cases do
not execute and therefore cannot establish the promised timing decision. The
mechanic is modeled free damage with a generally inferior delay, not a
meaningful early-versus-finisher tradeoff.

The threshold may not be lowered and cost, reward, content, cooldown, or
pressure may not be added merely to manufacture a favorable result. Gates 3
and 4 therefore never opened: no schema, replay, protocol, gateway, bot, Godot,
item, enemy, area, or working-service implementation was created.

## Isolation and complete removal

Before removal, formatter, Clippy with warnings denied, four pure-domain tests,
three lab tests, stable serialization, static source exclusion, and
`git diff --check` passed. The working database remained at sixteen public
tables with zero public object containing `echo`; the permanent PostgreSQL,
Gateway, and Inspector containers remained healthy and loopback-only.

After removal, the complete implementation source comparison against
`revenant-m28-closure-45718926-20260907.tar.gz` reports:

- 235 current and checkpoint implementation files;
- zero path differences and zero content differences;
- byte-identical `Cargo.toml`, `Cargo.lock`, `VERSION`, `README.md`, and
  `Makefile`;
- zero Echo paths or strings in product/build source; and
- unchanged frozen-V1 hashes
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`
  and
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.

The checkpoint SHA-256 remains
`8077c650455b61214ef7c843e6e5767f7a1a85c473bd210c87ba296b19edb4d9`.
The restored uninterrupted `make check` passed version/format/Clippy, all 217
Rust and PostgreSQL tests, all-target builds, Inspector checks/build, the
388-file secret audit, current multiplayer gameplay, Godot M17-M28 validation,
replay/Inspector reconciliation, frozen-V1 compatibility, and standalone V1
reconstruction.

## Gate decisions and next proposition

Gate 2 is **rejected as a keep decision**. Under the predeclared early-removal
path, Gate M29 is **complete and green by evidence-based removal**. There is no
retained Echo code, feature, database state, protocol vocabulary, client
surface, or product claim.

Exactly one bounded starting proposition is handed to M30:

> With the current Protocol V2 and loopback-only deployment, a local,
> opt-in security laboratory can reject every case in a finite matrix covering
> identity impersonation, cross-session mutation, malformed/oversized frames,
> operation replay/conflict, and bounded connection abuse with zero protected
> state mutation, then restore exact authoritative state after bounded
> connection, process, and database faults, without Protocol V3, public
> exposure, or default-service behavior changes.

M30 Gate 1 may audit and freeze that proposition. M31 content expansion remains
sequentially locked until M30 closes.
