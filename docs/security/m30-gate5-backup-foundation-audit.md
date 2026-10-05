# M30 Gate 5 — first backup and restore slice

Date: 2026-09-09  
Decision: **first implementation slice validated; Gate 5 remains open**

Historical first-slice decision. The subsequent complete Gate 5 acceptance is
recorded in [the recovery audit](m30-gate5-recovery-audit.md).

## Review and correction

The execution ledger and Gate 4 audit identify backup/fault/recovery as the
next authorized work. The README still described M23/M24, and the roadmap's
final continuation paragraph still pointed at Gate 4. Both now agree with
the active Gate 5 contract. Existing uncommitted milestone work is preserved;
`main` and `origin/main` remain at
`4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`.

At resumption, PostgreSQL and Gateway could not start because Docker Desktop
referenced WSL bind mounts lost after host restart; Inspector consequently
restarted without its upstream. Recreating the normal containers using the
existing owner-only secret files and named database volume restored service
health. No database volume was deleted, replaced, or used as a restore target.

The old backup runbook relied on a checksum and archive listing followed by
a manually named restore in the working cluster. The new `scripts/m30-backup.py`
command requires the expected source labels/image/volume/database/health,
private POSIX file modes, a new destination, a complete custom archive, and
independent canonical data/schema exports. It seals only after an actual
isolated restore and two passes through all eight existing migrations agree.

`verify` checks the exact file inventory and SHA-256 values before processing
the retained bytes. Restore containers have no network, port publications,
host mounts, secrets, or persistent volumes. Their random ownership labels
and exact created IDs bound cleanup, including interrupted Docker creation.
No command accepts an active restore or teardown target. Diagnostics contain
fixed categories; credential scanning honors both the default and configured
secret directory. The [operator runbook](../operations/postgresql-backup.md)
documents commands, retention, failure handling, and limitations.

## Measured results

The reviewed backup is retained privately at
`/home/an/.local/state/revenant/backups/m30-g5-20260909b`.
Its `SHA256SUMS` hash is
`7833b3dc13f9de8f6dcca6ab4f101f32fab3c82edcdee565c092fe5242381093`.
No raw archive or canonical row export is included in repository/evidence
files. The earlier successful `...20260909a` backup remains separate.

- All 45,797 rows across sixteen tables and both sequence value/called states
  match after restoration and after each migration pass.
- The custom archive is 875,726 bytes. The private canonical export is
  16,913,774 bytes; the schema export is 38,091 bytes.
- Backup creation measured 8,132 ms for the database restore portion;
  final-source explicit verification measured 9,111 ms. These timings include
  the disposable container lifecycle and comparison, not application health.
- The five-case real smoke passes normal restoration and rejects an altered
  checksum, a truncated archive whose checksum matches, and independent
  schema/row mismatches whose checksums and manifests match their fixtures.
- Working source data/schema remain identical throughout backup creation and
  the focused corruption smoke. Every disposable restore container is removed.
- Eighteen deterministic safety tests pass and are wired into `make test`
  and CI. They include collision/ownership, interrupted-create cleanup,
  symlink/mode/non-overwrite, checksums/format/migration revision, size bounds,
  source identity, and secret redaction with custom directories.

## Quality and protected boundaries

The baseline full `make check` passed. The final source also passed an
uninterrupted full run after the secret-directory override correction:
version consistency, Rust formatting/strict Clippy, 229 Rust/PostgreSQL tests,
the eighteen Python tests, workspace build, Inspector TypeScript/build,
secret audit, multiplayer/gameplay and Godot M17–M28 checks, replay/Inspector,
frozen V1 compatibility, and isolated reconstruction. Python formatting and
lint pass separately with Ruff.

All 127 protected Gate 4 gameplay, Godot, activity, protocol, compatibility,
archive, and version inputs remain byte-identical. Gate 4's evidence manifest
and the external M28 closure/M30 Gate 1 checkpoint hashes were reverified.
The maintenance port is closed after checks; normal PostgreSQL, Gateway, and
Inspector are healthy. Canonical checks append their established synthetic
test records; those expected test writes are separate from the non-mutating
backup and corruption smoke. Product version remains 0.2.0.

Evidence is retained under
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate5-backup-foundation-20260909a`.
Its `SHA256SUMS` covers fifteen payload files and has SHA-256
`5318fe191b14975679e4ac26aa48606075e9d97f22490f4a12836e18293a1125`.
Every entry verifies; the final redaction scan finds no available current or
previous secret material. Final database inspection reports sixteen public
tables, zero temporary triggers, and zero public functions.

## Remaining work and decision

This is not R01–R30 acceptance or Gate 5 approval. The next slice must provide
disposable Gateway/PostgreSQL/Inspector kill/restart, persistence reconnect,
forced transaction rollback, committed-terminal idempotency, explicit
database-corruption detection, live reward/replay/Inspector reconciliation,
and the complete restore-to-healthy-application objective. It must then run
the exact thirty-vector matrix and decide Gate 5 before the 360-row Gate 6
closure. M31 and M32 remain behind those milestone gates.

The backup command requires a quiet local interval and the current eight-
migration revision. It excludes cluster-global roles/passwords and normal
runtime-role provisioning, signatures, untrusted archives, scheduled/off-host
backup, PITR, and active-database replacement. SIGKILL or Docker daemon
failure can require cleanup of that invocation's tmpfs container. These
limits remain explicit; the database-only timing is not an application RTO.
