# PostgreSQL backup and restore

Revenant stores accounts, characters, inventory, progression, activity history, and replay events in the `revenant` PostgreSQL database. Backups use PostgreSQL's custom archive format and never require deleting the Compose volume.

## Backup

Create a host-side backup directory and stream `pg_dump` from the healthy container:

```bash
mkdir -p backups
backup_label=0.2.0
docker compose -f infra/docker-compose.yml exec -T postgres \
  pg_dump --username revenant --dbname revenant --format=custom \
  > "backups/revenant-$backup_label.pgdump"
sha256sum "backups/revenant-$backup_label.pgdump" \
  > "backups/revenant-$backup_label.pgdump.sha256"
```

Verify the file before relying on it:

```bash
backup_label=0.2.0
sha256sum --check "backups/revenant-$backup_label.pgdump.sha256"
docker compose -f infra/docker-compose.yml exec -T postgres \
  pg_restore --list < "backups/revenant-$backup_label.pgdump"
```

## Restore drill

Restoration overwrites database objects, so perform the first drill into a separate database rather than the active `revenant` database:

```bash
backup_label=0.2.0
docker compose -f infra/docker-compose.yml exec -T postgres \
  createdb --username revenant revenant_restore_test
docker compose -f infra/docker-compose.yml exec -T postgres \
  pg_restore --username revenant --dbname revenant_restore_test \
  --clean --if-exists < "backups/revenant-$backup_label.pgdump"
docker compose -f infra/docker-compose.yml exec -T postgres \
  psql --username revenant --dbname revenant_restore_test \
  --command='SELECT COUNT(*) FROM characters;'
```

Drop only the explicitly named drill database after verification:

```bash
docker compose -f infra/docker-compose.yml exec -T postgres \
  dropdb --username revenant revenant_restore_test
```

Never use `docker compose down -v` as a backup or restore operation. A production restore into `revenant` requires a maintenance window, a verified backup, and explicit operator approval.
