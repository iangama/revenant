# PostgreSQL backup and restore

Revenant stores accounts, characters, inventory, progression, activity history, and replay events in the `revenant` PostgreSQL database. Backups use PostgreSQL's custom archive format and never require deleting the Compose volume.

## Backup

Create a host-side backup directory and stream `pg_dump` from the healthy container:

```bash
mkdir -p backups
docker compose -f infra/docker-compose.yml exec -T postgres \
  pg_dump --username revenant --dbname revenant --format=custom \
  > backups/revenant-0.1.0.pgdump
sha256sum backups/revenant-0.1.0.pgdump \
  > backups/revenant-0.1.0.pgdump.sha256
```

Verify the file before relying on it:

```bash
sha256sum --check backups/revenant-0.1.0.pgdump.sha256
docker compose -f infra/docker-compose.yml exec -T postgres \
  pg_restore --list < backups/revenant-0.1.0.pgdump
```

## Restore drill

Restoration overwrites database objects, so perform the first drill into a separate database rather than the active `revenant` database:

```bash
docker compose -f infra/docker-compose.yml exec -T postgres \
  createdb --username revenant revenant_restore_test
docker compose -f infra/docker-compose.yml exec -T postgres \
  pg_restore --username revenant --dbname revenant_restore_test \
  --clean --if-exists < backups/revenant-0.1.0.pgdump
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
