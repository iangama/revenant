# M28 Block 8 — runtime, recovery, and resource matrix audit

Date: 2026-09-07  
Decision: **Gate 8 green; M28 closure remains pending Block 9**  
Scope: the pre-harness contract in
`docs/operations/m28-block8-runtime-contract.md`.

## Accepted execution

The accepted current-source run is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block8-runtime-0907b8c`.
It completed in 399 seconds and its 144-file `SHA256SUMS` manifest has SHA-256
`9dbf6fba76bd99cf6abdc10109e8d1ec5dd22271949e8ac732306a049f4f6a46`.
The run used fresh synthetic local identities and normal activity, inventory,
module, loadout, route, and cooperation operations. SQL inspected the result
but created or repaired no fixture state.

Six reused characters earned their required modules through exactly 34
successful baseline sessions. The four accepted two-client cooperation
sessions then exercised these ordered profiles:

| Sample | Anchor | Runner | Terminal elapsed |
| --- | --- | --- | ---: |
| 1 | Empty pulse rifle | Force+Ward arc sidearm | 6,362 ms |
| 2 | Force arc sidearm | Ward pulse rifle | 7,303 ms |
| 3 | Ward pulse rifle | Force arc sidearm | 6,372 ms |
| 4 | Force+Ward arc sidearm | Empty pulse rifle | 7,316 ms |

Each session retained the admitted weapon/module arithmetic, immutable
anchor/runner order, same-input retry, conflict rejection, one fixed ping, one
scripted downing, cancelled-then-completed revive channel, second-revive
rejection, route exclusion, five ordered contributions, one success, and two
ordered two-fragment/125-XP grants. Inspector and replay each report two
participants, five contributions, one revive, success, two loot grants, and
two progression grants.

## Exact durable ledger

The build-only prefix reconciles to the complete frozen ledger:

- eight accounts, 38 distinct sessions, and 42 activity histories;
- 42 inventory-grant rows, 50 fragments awarded, 24 consumed, and 26 current;
- eight cooperation item grants of quantity two, eight 125-XP progression
  grants, 4,400 total XP, four succeeded cooperation rows, and eight immutable
  participant rows;
- six module states, twelve owned modules, eight equipped slots, revision sum
  ten, and 22 accepted module operations; and
- zero route/cooperation integrity discrepancy.

Including the bounded compatibility probes, the fresh current-prefix ledger
contains 12 accounts, 41 sessions, 46 joins, 46 completed histories, 55
fragments awarded, 31 current fragments, and 4,800 XP. Every asserted value
matches PostgreSQL exactly. The complete M28 cooperation rows and the M27
route row remain mutually exclusive.

## Failure, reset, and recovery coverage

The nested Gate 6 role matrix used a separate fresh prefix and completed all
16 two-client cases with 16 clean resets:

- one success;
- real-clock `failed_ping_timeout` at 5,386 ms,
  `failed_revive_timeout` at 15,437 ms, and
  `failed_operation_timeout` at 60,185 ms; and
- anchor and runner disconnects in each of the six nonterminal phases,
  producing twelve `abandoned_disconnect` terminals.

All fifteen nonsuccess sessions have zero completion and zero loot/progression
grant. SQL, replay, and Inspector prove one typed terminal, the expected phase
and subject evidence, no route row, no activity history, no reward, and no
post-terminal mutation before reset.

The intentionally unreachable-in-production `failed_participant_defeated`
branch was not fabricated by weakening health or pressure. The composed
gateway test and both real PostgreSQL/replay tests each executed one focused
case and passed. They prove fatal authoritative Warden damage, typed defeated
terminal evidence, transaction/reconstruction ordering, and zero reward.

## Compatibility and application boundaries

The same run passed an ordinary unopted two-client current-V2 completion, a
complete current-V2 M27 Breach session, a solo current Protocol-V1 completion,
and the byte-frozen V1 join through the compatibility adapter. Ordinary V2 and
both V1 clients received no unsolicited M28 variant; the M27 route retained no
cooperation state; and every M28 success rejected an M27 choice without
creating a route row.

All sampled Inspector summaries/events match current CLI reconstruction and
PostgreSQL. GET returns 200 while HEAD, POST, PUT, PATCH, DELETE, and OPTIONS
return 405. The before/after pure reports are byte-identical at 900 matrix rows
and 50 lifecycle cases, with their accepted hashes:

- matrix: `513de2fb226458d28d46d945f650ae482d18ad388d4f1ab8d30d2e8ae830ceb5`;
- domain: `34fa219abb14a98e642a622615d17b0f5d7c71f7a7c796853b751cde3c06c261`.

## Resource and quality evidence

No sampled gateway grew an FD, thread, or database connection. RSS growth was
2,616 KiB for the 37-session solo preparation/compatibility group, 3,124 KiB
for the build/ordinary-V2 group, and 3,564 KiB for the 16-session failure
matrix, all below the 65,536 KiB bound. Every isolated listener and process
stopped afterward.

The supporting quality evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block8-quality-0907c`; its
`SHA256SUMS` hash is
`36497f1b7d3467411910b994b379f1d33481e03643c2bf20aa54203e22596b3a`.
The uninterrupted `make check` passed version consistency at `0.2.0`, strict
formatting/Clippy, **217 Rust tests**, complete workspace and production web
builds, secret audit, current multiplayer smoke, all Godot M17–M28 semantic
markers, frozen-V1 compatibility, and independent V1 reconstruction. Gateway
and Inspector production Compose images also build.

Every retained M21–M28 capture manifest verifies. PostgreSQL, Gateway, and
Inspector remain healthy and bound only to `127.0.0.1:5432`, `:7000`, `:8080`,
and `:4173`. The public schema retains zero non-internal trigger and zero
function. `git diff --check` passes, no generated `.gd.uid` is untracked, and
the protected hashes remain:

- frozen V1 `src/main.rs`:
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`;
- frozen V1 `Cargo.toml`:
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`;
- M27 closure checkpoint:
  `81bb1fafaa397b27db7fc505767c09202d68b77e13355952b49ad9d994280d70`.

No schema, migration, protocol, replay vocabulary, cooperation behavior,
weapon/module/route catalog, Lua revision, Godot presentation, frozen V1,
package, version, commit, tag, release, public listener, or public artifact was
changed by Block 8. Its product mutation is limited to validation scripts and
the previously required complete-workspace Gateway Docker build context.

## Rejected evidence

`m28-block8-runtime-0907b8a` is preserved but rejected because the harness
started the ordinary-V2 driver before its observer. Bootstrap interleaving made
the validation client expect `ActivityStart` while the second admission was
still arriving. The four build successes before that harness failure were not
promoted.

`m28-block8-runtime-0907b8b` is also preserved but rejected. Its product
matrix completed, but final review found that the two focused persistence
commands had not received `DATABASE_URL` and therefore reported themselves as
skipped. The script was corrected, and the full matrix—not merely the omitted
commands—was rerun under the new immutable `0907b8c` target.

## Gate decision and residuals

Gate 8 is green. The finite build, weapon, success, failure, disconnect,
recovery, retry/conflict, cancellation, progression, compatibility, replay,
Inspector, SQL, repetition, reset, and resource contract is satisfied without
client-owned outcomes or fixture state injection.

This evidence is creator automation, not proof of another person's
comprehension, teamwork, accessibility experience, preference, enjoyment, or
anti-griefing quality. Hardware GPU/audio-device presentation and long-duration
public/network soak remain outside M28. Block 9 may review the complete M28
diff and evidence, run its own uninterrupted final gate, classify residuals,
and decide Gate M28. M29 implementation remains locked.
