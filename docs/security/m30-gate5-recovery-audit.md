# M30 Gate 5 — disposable fault and application recovery

Date: 2026-09-09  
Decision at this gate: **Gate 5 approved**.

Subsequent owner revision (2026-09-09): M30 closed using the accepted results
below; the additional Gate 6 repetitions were removed, not executed. See the
[current closure](../roadmap-2026-08-30.md#lean-closure-and-next-scope--2026-09-09).
The earlier next-step statements below are historical.

## Boundary and implementation

This continues the accepted [backup foundation](m30-gate5-backup-foundation-audit.md)
under the [frozen recovery contract](m30-gate5-backup-recovery-contract.md).
`tests/m30-recovery-matrix.py` executes R01–R30 in two freshly named Compose
projects with synthetic accounts, generated private secrets, and disposable
data/socket volumes. All published ports use IPv4 loopback; the normal data
volume is independently fingerprinted and never restored, stopped, or removed.
Every fixture teardown checks both project and random ownership labels.

The existing V2 bot has a bounded `recovery-hold` diagnostic mode. It enters
an activity, acknowledges a file rendezvous, and submits a persistence-required
intent only after the driver interrupts its service. It refuses a projected
completion/reward/equipment result and requires actual connection closure.
Normal bot and Gateway paths retain their existing gameplay behavior.

An ignored-by-default Rust integration test repeats an already-committed
cooperation terminal through the existing persistence API. It requires the
synthetic database prefix, runtime role, loopback host, and fixture port 15451.
The matrix invokes it explicitly and independently compares every raw row and
sequence before/after both retries. Ordinary `make check` leaves this one test
ignored; its real database execution belongs to R20.

Eight additional Python safety tests cover foreign ownership, working-volume
and restore-target refusal, public-port refusal, preservation of reward and
sequence changes in normalization, ordered payload arrays, and mandatory raw
equality even when normalized observations match. All twenty-six backup and
recovery safety tests run in `make test` and CI.

## Recovery evidence

R01–R12 check health/identity, create a sealed custom archive, use the real
backup verifier to reject a checksum mismatch and a resealed truncated archive,
restore only into a checked-absent database, and compare every table, sequence,
and schema. The decoded archive and retained evidence are scanned for secret
material. The working database is checked independently of fixture state.

R13–R18 kill/restart Gateway, stop/restart PostgreSQL while an activity is
active, observe persistence reconnect in the same Gateway process, and
kill/restart Inspector. Interrupted sessions retain no terminal or reward;
fresh sessions on the same accounts complete normally. R15's durable
post-state is explicitly observed after PostgreSQL returns in R16.

R19 injects a trigger failure at the second participant's progression replay
insert and verifies that the actual fault was reached in PostgreSQL logs.
Neither participant retains inventory fragments, XP, reward grants, completion
history, or terminal reward replay from the rolled-back transaction. Removing
the fixture trigger permits a fresh successful retry. R20's two exact committed
terminal retries return `Replayed` and change zero rows or sequence values.

R21–R27 reject corrupt archive data, detect deliberately corrupted disposable
progression through the acceptance comparison, preserve a sealed pre-migration
snapshot, run all eight existing migrations twice, and restore that original
snapshot into another checked-absent database. Legacy exports remain exactly
equal; no new migration or gameplay schema is introduced.

R28 serves the restored sessions through Gateway/Inspector, verifies terminal
rewards and replay, and completes fresh ordinary and cooperative activities.
R29 measures from R08 restore start through this reconciliation, including
the intervening fault proofs, against the 600-second ceiling. R30 removes
only generated fixture resources and verifies the retained working database.

Both final fresh matrices passed all thirty cases, for 60/60 passing rows.
Their normalized reports are byte-identical with SHA-256
`d75285453caf319921e2e64127f61bd37b006e85189dfbbc69b8e9d410dcfad5`.

| Run | Complete matrix | Restore through application reconciliation |
| --- | ---: | ---: |
| 1 | 182,669 ms | 118,072 ms |
| 2 | 181,849 ms | 117,726 ms |

Both meet the 600,000 ms ceiling and preserve the working data/schema exactly.
Every raw stable-case comparison passed, including zero mutations for the
committed terminal retries. All generated containers, networks, and volumes
were removed. Runtime resource limits matched the reviewed configuration and
neither fixture reported an OOM kill.

The retained private working-data backup was also reverified with exact
sixteen-table/two-sequence/schema equality and two migration passes in 9,474 ms.
The backup command now explicitly reports that its own invocation does not
exercise application recovery; application RTO belongs to the matrix above.

Final evidence is under
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate5-recovery-20260909b`.
Its `SHA256SUMS` seals 63 payload files and has SHA-256
`3628993687f7d178afeaa2f6d96dc1bfcd266227f944c415e374613bf7f10bdc`.
Every entry verifies and every retained payload passes the final secret scan.

## Canonical quality and final disposition

The final uninterrupted `make check` passed: version consistency, Rust
formatting and strict all-feature Clippy, 229 Rust tests, 26 Python safety
tests, workspace build, Inspector TypeScript/build, secret audit, the complete
Godot/gameplay smoke, replay/Inspector, frozen V1, and isolated reconstruction.
Python formatting/lint pass separately. The one ignored Rust test is the
explicit fixture-only terminal retry already executed successfully by R20
in each matrix; it is not counted among the 229 canonical passing tests.

All 127 protected Gate 4 inputs and thirteen implementation inputs verify.
The accepted backup foundation manifest remains valid. Normal PostgreSQL,
Gateway, and Inspector are healthy; all listeners are loopback and PostgreSQL
maintenance is closed. No recovery fixture container, network, or volume
remains. The retained working volume is `infra_revenant-postgres`.
Canonical checks append their established synthetic test records separately
from the matrices' strict working-data preservation proof; the final normal
database contains 52,151 rows across sixteen tables and both sequences.
Its post-check state digest is
`f2322ec56264cccfdba821689d6c6f940324b86cc0a46f244da678230926d435`.

Gate 5 is approved within the frozen local scope. The next authorized action
is Gate 6's three consecutive complete 120-vector repetitions, with canonical
quality and protected checks for each repetition. Gate M30 remains pending.

## Review history and limits

Development evidence remains separate under
`revenant-local-evidence/m30-gate5-recovery-20260909-dev`. Initial setup probes
identified an unused image-declared anonymous PostgreSQL volume in the
provisioner and mismatched diagnostic bot labels. The fixture now mounts that
unused directory as tmpfs, uses the actual bot identities/output, and records
scanned client/service logs. Only verified created resources were cleaned up.

The first complete candidate in `m30-gate5-recovery-20260909a` passed all live
cases. Comparison identified the observed `expires_elapsed_ms` timestamp as
missing from the explicit elapsed-time allowlist; its second run was stopped
and retained as incomplete. Admission order is now fixed by observing the
first player's durable join before admitting the second. Actor IDs, event
order, rewards, sequence state, and gameplay timing constants remain protected.
Final acceptance uses fresh runs after these harness corrections; no failed
candidate is overwritten or counted as a passing repetition.

This remains a creator-operated local laboratory. The small-fixture result
does not establish recovery for arbitrary database sizes, production uptime,
off-host/scheduled backups, PITR, signatures, or untrusted archives. Changes
after the sealed snapshot are outside its recovery point. Active database
replacement remains outside these commands. SIGKILL or Docker daemon failure
can require ownership-verified cleanup of the invocation's temporary resources.

M30 Gate 6's three full 120-vector repetitions remain a separate acceptance
boundary. This audit cannot approve Gate M30 or open M31 by itself. No release,
version change, commit, public service, external API, or gameplay expansion
is part of Gate 5.
