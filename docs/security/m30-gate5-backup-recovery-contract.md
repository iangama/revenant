# M30 Gate 5 — backup and recovery implementation contract

Date: 2026-09-09  
Status: **Gate 5 approved under `m30-gate5-recovery-audit.md`**

The first implementation slice is validated under
[`m30-gate5-backup-foundation-audit.md`](m30-gate5-backup-foundation-audit.md).
The full fault/application-recovery matrix below is accepted under
[`m30-gate5-recovery-audit.md`](m30-gate5-recovery-audit.md).

Gate 4's accepted manifest was reverified before this continuation. The normal
containers initially could not start because Docker Desktop retained missing
WSL bind-mount paths after host restart. Recreating only the normal Compose
containers with the existing secret files and named database volume restored
service health. No volume was removed or restored.

## First implementation slice: sealed backup and disposable restoration

`scripts/m30-backup.py` provides `create` and `verify` commands. The source is
the healthy `infra-postgres-1` container, identified by its Compose project and
service labels, pinned image, mounted `infra_revenant-postgres` data volume,
database `revenant`, and administrator role `revenant`. It exposes no SQL,
connection-string, active restore target, or teardown target argument.

Backup directories are newly created outside the repository on a filesystem
that enforces POSIX ownership and modes: directory 0700, files 0600. The
Windows workspace is not an acceptable private dump destination. Contents are
one custom archive, an independent canonical export of all sixteen tables and
both sequences, the schema export, checksums, and redacted metadata. Raw rows
stay in the owner's private backup; gate evidence retains only counts and
digests. Existing directories, symlinks, and permissive paths are refused.

The command uses read-only queries and `pg_dump` against the source. Independent
source exports before and after dumping must agree; concurrent activity that
changes those exports rejects this candidate. This is a deliberately quiet
local backup window, without stopping services or locking gameplay writes.
The restored data must independently equal the sealed export, so a checksum
or a readable archive catalog alone cannot approve a backup.

Verification uses a freshly named, checked-absent container and database with
an unpredictable per-run token. The container has no network, published port,
host bind mount, secret mount, or persistent data volume. PostgreSQL listens
only on its internal Unix socket; data is tmpfs-backed. The exact pinned
PostgreSQL 16.15 image, 256 MiB memory and 64 PID limits, no-new-privileges,
and disabled Docker logging bound the disposable environment. Cleanup uses
only the exact container ID created by this invocation; it never runs Compose
`down`, a volume removal, or `DROP DATABASE` against the working container.

Checksum validation precedes archive processing. Verification fully decodes
the archive, scans for credential material without reporting matching bytes,
restores with `--exit-on-error --single-transaction --no-owner --no-privileges`,
and compares schema, every canonical row, and sequence value/called state.
The eight existing migrations then run transactionally twice; each pass must
preserve the same exports. No candidate migration or gameplay schema is added.
An incomplete operation never receives a successful seal or report.

PostgreSQL documents the custom format and consistent dumps in
[pg_dump](https://www.postgresql.org/docs/16/app-pgdump.html), and transactional,
error-stopping restores in
[pg_restore](https://www.postgresql.org/docs/16/app-pgrestore.html). A checksum
detects accidental corruption; it is not a signature or permission to restore
untrusted archives. Database role credentials and cluster-global roles are
not archived. Normal secret provisioning remains the Gate 3 operator path.

## Complete Gate 5 matrix

### Frozen runtime matrix continuation — 2026-09-09

The R01–R30 driver uses two newly named `m30g5` Docker Compose projects with
fresh owner-only secrets and project-owned disposable volumes. Persistent
fixture volumes are necessary to test PostgreSQL container stop/start;
neither project may reference `infra_revenant-postgres`. The source database
is `m30f_<token>`, and the checked-absent restored database is `m30r_<token>`.
Only the source fixture publishes PostgreSQL, on IPv4 loopback port 15451,
for a separately invoked, ignored-by-default persistence retry fixture.
Source game/health/Inspector use 17451/18451/41451; restored application uses
17452/18452/41452. The normal database maintenance port stays closed.

Both projects reuse the reviewed normal images and security/resource limits.
Automatic restart is disabled only in these fixtures so each explicit
stop/start is observable. Every destructive operation verifies the generated
project and random ownership label; cleanup removes only its created
containers, network, and named volumes. Secret files and raw synthetic dumps
remain in a private temporary Linux directory and are removed after the run.
The one-shot provisioning client's unused image-declared PostgreSQL data
directory is explicitly tmpfs-backed, preventing an anonymous data volume.
Admission of the first diagnostic client is observed before starting its
partner, so generated actor allocation is repeatable without weakening game
state comparisons.

Two ordinary V2 clients and the M28 cooperation bot seed authoritative state.
A bounded diagnostic bot mode holds an already-started activity at an
explicit file rendezvous. After Gateway kill or PostgreSQL stop, releasing
that rendezvous submits an equipment intent and verifies connection closure
without completion/reward/equipment-success projection. PostgreSQL restart
must retain prior committed data, and the same Gateway process must reconnect
on a fresh operation. A selective fixture-only trigger fails the second
participant's progression replay insert, proving both participants' complete
reward transaction rolls back. A separate Rust test retries the exact
already-committed participant tuple through the existing persistence API;
it must return only replayed/no-new-grant results and preserve every row and
sequence. No default protocol, Gateway, or gameplay implementation changes.

Each case records exact fixture pre/post digests, changed protected entries,
disposition, redaction, timing, and resource observations. R15's durable
post-state is explicitly observed after R16 restarts PostgreSQL; the driver
does not pretend to read a stopped database. Rejected archive/target cases
must preserve the exact fixture state. Normalized comparison may replace only
generated fixture/session identifiers, wall timestamps, and observed elapsed
durations; raw equality checks are always performed first. The working
database is independently fingerprinted over the whole matrix.
The synthetic-only normalized state exports and scanned client/service logs
are retained beside each report for independent comparison. No working-database
rows, archives, secret bytes, or connection URLs enter those artifacts.
`tests/m30-recovery-compare.py FIRST SECOND` requires two complete passing
reports and byte-identical rows after removing only row timing/resource samples.

R23–R27 use the existing eight migrations as the candidate revision, applied
twice to the restored fixture and followed by a second checked-absent restore
of the pre-migration snapshot. R28 compares restored replay/Inspector rewards
and completes fresh ordinary and cooperative sessions. R29 measures from
R08 restore start through healthy Gateway/Inspector plus reconciliation,
including intervening fault proofs, against the frozen 600-second ceiling.
R30 proves exact teardown and working-volume retention. Two fresh matrix
runs preceded the Gate 5 decision. The owner subsequently removed Gate 6's
extra repetitions in favor of reviewing the already accepted evidence.

The first slice supplied the backup/restore mechanisms; final acceptance now
includes every R01–R30 case in two fresh matrices plus the uninterrupted
canonical quality gate and protected hash checks. Reports are byte-identical
after the declared observation normalization, with 118,072/117,726 ms complete
application recovery and unchanged working data. Gate 5 is approved within
this local scope. The owner-revised Gate 6 review also closes M30 without
additional repetitions; M31 content is next under the proportional policy in
`AGENTS.md`. The removed 360-row campaign is not claimed as executed.
