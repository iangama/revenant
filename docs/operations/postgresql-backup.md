# PostgreSQL backup and restore

Use the owner command from the repository root. It seals a backup only after
an actual disposable restore matches all sixteen tables, both sequences, and
the schema, including two applications of the eight existing migrations.
M30 Gate 5 application crash/reconnect, transaction-fault, and complete
restore-to-application-health proofs are accepted under the
[recovery audit](../security/m30-gate5-recovery-audit.md). The small disposable
fixture recovered in about two minutes in both fresh acceptance runs.

## Prerequisites

- Python 3.10+, Docker, and the locally available pinned PostgreSQL image.
- Healthy normal `infra-postgres-1` with database `revenant`, administrator
  role `revenant`, and the existing `infra_revenant-postgres` volume.
- The normal stack running with its maintenance port closed. Use
  `scripts/m30-validate-compose.sh normal` to check its configuration.
- A quiet local interval: avoid playing or running validators until the
  command returns. Concurrent data changes reject the candidate; the command
  never stops the game or blocks its writes to force success.
- A private Linux backup location outside the repository. The Windows DrvFS
  workspace does not enforce the required 0700/0600 modes and is refused.

If Docker Desktop lost WSL secret mounts after a host restart, recreate the
normal containers using the existing secret generation and volume:

```bash
export REVENANT_SECRETS_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/revenant/m30-secrets/current"
docker compose -p infra -f infra/docker-compose.yml \
  up -d --no-build --force-recreate --wait --wait-timeout 90
```

This does not delete a volume or generate replacement credentials.

## Create a verified backup

Create the private parent directory once, then choose a new label for each
backup. An existing destination is always refused.

```bash
backup_root="${XDG_STATE_HOME:-$HOME/.local/state}/revenant/backups"
install -d -m 700 "$backup_root"
backup_dir="$backup_root/manual-$(date -u +%Y%m%dT%H%M%SZ)"
python3 -B scripts/m30-backup.py create "$backup_dir"
```

The final JSON reports `backup_sealed: true`, `source_unchanged: true`, exact
state/schema digests, migration passes, elapsed database-restore time, and
disposable cleanup. It does not print connection strings, credentials, rows,
or unfiltered PostgreSQL diagnostics. The directory contains:

| File | Contents |
| --- | --- |
| `archive.pgdump` | Custom-format database archive |
| `state.json` | Private canonical export of every table and sequence |
| `schema.sql` | Structural schema without ownership or access grants |
| `manifest.json` | Format/image/migration identity, counts, hashes, and restore result |
| `SHA256SUMS` | SHA-256 for the four exact payload files |

Treat the dump and canonical export as private gameplay data. They are not
release artifacts, Inspector files, or material to put in Git. The command
scans the decompressed archive for credential patterns and available current/
previous database secrets. Cluster-global roles and passwords are excluded;
restoring normal runtime privileges remains the separate Gate 3 provisioning
procedure.

## Recheck a retained backup

```bash
python3 -B scripts/m30-backup.py verify "$backup_dir"
```

Verification checks the exact file inventory, ownership, permissions,
checksums, format, and migration revision before restoring the validated
bytes. It runs without using the active database as a destination. A fresh
tokenized PostgreSQL container has no network or host mounts; its database is
checked absent and created from `template0`. Restore uses a single transaction
and stops on errors. Rows, sequence values/called state, and schema must match
before and after each migration pass. Only that invocation's container is
removed, including on handled failures and interruption.

`pg_restore --list` alone cannot detect all compressed-data corruption. The
command also fully decodes the archive and performs the real restore. A
checksum is not a signature: verify only trusted, owner-created backups.

## Failure and retention

A rejected command exits nonzero with a fixed diagnostic category. An
incomplete create directory has no valid seal; retain it separately as a
failed candidate, then retry using a new destination. Do not overwrite a
sealed backup or manually repair its checksums to make it pass. Preserve
accepted backups through M32 archival closure unless the owner chooses
earlier removal. There is no scheduled pruning, upload, off-host backup,
continuous archiving, or point-in-time recovery.

For a process killed with SIGKILL or Docker daemon failure, automatic cleanup
cannot be guaranteed. Inspect only containers named `m30g5restore-*` and their
`revenant.m30.restore` ownership labels before operator cleanup. They hold
only disposable tmpfs data; the command never mounts the working volume.

An active-database restore remains a separate maintenance operation requiring
explicit owner authorization. Never use `docker compose down -v` for backup
or recovery. Database-only elapsed time from this command is not the M30
application RTO; see the [Gate 5 contract](../security/m30-gate5-backup-recovery-contract.md).

## Developer validation

```bash
python3 -B -m unittest discover -s tests -p 'test_m30_*.py'
python3 -B tests/m30-backup-smoke.py "$backup_dir"
```

The first command also runs under `make test` and CI. The second performs a
real restore and rejects wrong checksums, a truncated archive with a matching
checksum, and independent schema/data mismatches. It checks that working data
is unchanged and that no disposable restore container remains. This focused
smoke is separate from the complete R01–R30 recovery matrix below.

To execute the complete disposable recovery matrix, build the normal Compose
images and the Rust workspace, place the repository's Rust toolchain on PATH,
and create a new evidence directory outside the repository. Then run:

```bash
python3 -B tests/m30-recovery-matrix.py \
  --report /absolute/new/evidence/recovery.json
```

Keep the working stack quiet and its maintenance port closed until completion.
The driver creates two isolated projects with fresh private secrets and
disposable data volumes. It publishes only loopback ports 15451, 17451/17452,
18451/18452, and 41451/41452; occupied ports reject the run before creation.
Only generated fixture containers are stopped or killed. Reports and scanned
synthetic client/service logs remain beside the requested report; raw backups
and secret files are removed from the private Linux temporary directory.
The ignored `m30_terminal_retry` Rust test is invoked only by this harness,
against its checked synthetic database and loopback port 15451.

Successful reports contain exactly R01–R30, the independently checked
working-data digest, exact raw fixture digests, normalized observations, and
the complete restore-to-application-health duration. Preserve failed reports
under separate names. A new acceptance run always creates new fixture projects.
