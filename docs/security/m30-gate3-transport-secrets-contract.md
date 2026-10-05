# M30 Gate 3 — frozen transport and secrets implementation contract

Date: 2026-09-08  
Status: **fulfilled; Gate 3 approved under
`docs/security/m30-gate3-transport-secrets-audit.md`**

## Preconditions and reviewed update

The accepted pre-change backup candidate is outside the repository at
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate3-prechange-20260908b`.
It contains a custom-format dump from the healthy PostgreSQL 16.11 working
database and a complete restore into a network-isolated, tmpfs-backed
PostgreSQL 16.15 container. All sixteen tables, every ordered row, both
sequences, the normalized schema catalog, and extensions matched byte for
byte. The disposable container was removed and the working database was never
a restore target.

PostgreSQL's current supported-version table identifies 16.15 as the current
16.x minor and recommends the current minor. Its 16.15 release notes say that
a 16.x dump/restore is not required, while calling out security-related
configuration/data review. Revenant uses no logical replication slot,
non-core decoding plugin, `pgcrypto`, `btree_gist`, or `ltree`, so none of the
listed special remediation paths applies. The candidate is the Docker Official
Image `postgres:16.15-alpine` at immutable repository digest
`sha256:cf78e76683b9ca8c5733cbbdce6c9262b45b6767934dd0a95e671f9a0fc20685`.

Primary references:

- <https://www.postgresql.org/support/versioning/>
- <https://www.postgresql.org/docs/16/release-16-15.html>
- <https://hub.docker.com/_/postgres>
- <https://docs.docker.com/compose/how-tos/use-secrets/>

## Exact normal topology

- PostgreSQL uses the immutable 16.15 Alpine digest and publishes no host
  port in `infra/docker-compose.yml`.
- Gateway and Inspector retain only literal `127.0.0.1` host publications.
  Container-internal Gateway listeners remain `0.0.0.0` behind those mappings.
- `infra/docker-compose.maintenance.yml` is the only checked-in operator
  override allowed to publish PostgreSQL, always as
  `127.0.0.1:${POSTGRES_PORT:-5432}:5432`.
- A rendered-config validator fails closed on wildcard, empty, hostname, IPv6,
  LAN, or public host publications; on PostgreSQL publication in the normal
  file; and on any Inspector secret grant.
- No normal configuration enables Gateway TLS, changes Protocol V2, adds
  Protocol V3, or changes frozen V1.

## Exact secret files and grants

The default secret root is
`${XDG_STATE_HOME:-<account-home>/.local/state}/revenant/m30-secrets/current`.
It must be on a filesystem that enforces POSIX owner modes; the Windows DrvFS
workspace is deliberately rejected because it reports generated files as
world-accessible. `REVENANT_SECRETS_DIR` may select another absolute
owner-controlled Linux directory. The repository contains names and
validation rules only, never values.

| File | Shape | Granted service |
| --- | --- | --- |
| `postgres_admin_password` | 43-character base64url, 256 random bits | PostgreSQL only |
| `postgres_runtime_password` | 43-character base64url, 256 random bits | database provisioner only |
| `postgres_admin_database_url` | credential URL for role `revenant` on host `postgres` | migration job only |
| `gateway_database_url` | credential URL for role `revenant_runtime` on host `postgres` | Gateway only |
| `operator_database_url` | credential URL for `revenant_runtime` on `127.0.0.1` | host operator only; never a Compose secret |

The host directory is mode `0700` and files are mode `0600`. Generation uses
the operating-system random source through OpenSSL. Existing files are never
overwritten. Compose mounts each listed service secret read-only; Inspector
gets none. PostgreSQL consumes `POSTGRES_PASSWORD_FILE`; Gateway and the
migration job consume `DATABASE_URL_FILE`. No credential-bearing URL or
password is a committed default or a configured container environment value.

## Migration and runtime roles

The existing `revenant` owner becomes the cluster-administrator/migration
login and receives the generated administrator password. The separately
created `revenant_runtime` role is `LOGIN`, `NOSUPERUSER`, `NOCREATEDB`,
`NOCREATEROLE`, `NOREPLICATION`, and `NOBYPASSRLS`, owns no object, and has no
grant option. `PUBLIC` loses database/schema creation.

The runtime role receives `CONNECT` on database `revenant`, `USAGE` on schema
`public`, `SELECT` on all sixteen authoritative tables, and only these writes:

| Privilege | Tables |
| --- | --- |
| `INSERT` | `accounts`, `characters`, `inventory`, `equipment_loadouts`, `activity_history`, `inventory_reward_grants`, `progression`, `progression_reward_grants`, `module_states`, `module_loadout_slots`, `module_operations`, `route_operations`, `route_operation_participants`, `replay_events`, `cooperation_operations`, `cooperation_operation_participants` |
| `UPDATE` | `accounts`, `characters`, `inventory`, `equipment_loadouts`, `progression`, `module_states`, `route_operations`, `cooperation_operations`, `cooperation_operation_participants` |
| `DELETE` | `module_loadout_slots` |
| sequence `USAGE` | `activity_history_id_seq`, `replay_events_id_seq` |

The migration job uses the administrator URL, runs existing idempotent
migrations, and exits before the provisioner applies the runtime grants.
Gateway starts only after both jobs succeed and opens the database with
`connect_existing`; it cannot execute DDL. Default host development still
supports an explicit administrator URL and migration mode, but has no fallback
credential.

## Rotation and rollback

Bootstrap/rotation is an explicit operator command with a verified backup. It:

1. validates a newly generated sibling secret directory and current health;
2. uses the trusted local PostgreSQL Unix socket, never an old value on a
   command line, to rotate both login passwords and reconcile exact grants in
   one transaction;
3. proves new administrator and runtime connections before installing the new
   directory as `current`;
4. retains the previous owner-only directory as one rollback generation;
5. recreates only migration/provisioner/Gateway containers, proves new
   connections and V1/V2 flows, and proves the old values fail; and
6. emits categories/digests only, never secret text.

Rollback uses the same local socket to restore the protected previous values,
restores the previous `current` directory, recreates affected containers, and
rechecks health. Any partial rotation, missing previous generation, data/hash
drift, or inability to reject an old value rejects Gate 3.

## Isolated TLS laboratory

`tests/m30-transport-secrets-matrix.sh` creates all key/certificate material in
an owner-only temporary directory, starts only loopback OpenSSL endpoints, and
deletes the directory on every exit. A temporary local CA signs the valid
`localhost` server certificate. Separate untrusted, wrong-host, not-yet-valid,
expired, missing-key, and mismatched-key fixtures exercise `T07`–`T15`.
Clients always use hostname verification and explicit trust; the required-TLS
path has no plaintext retry. Reports retain only categorical outcomes,
certificate public fingerprints, durations, and resource counts. No private
key or bearer value enters the repository, image context, or evidence.

## Exact `T01`–`T30` evidence mapping

- `T01`–`T06`: normal and maintenance rendered Compose JSON plus negative
  validator fixtures; no service is started from a rejected rendering.
- `T07`–`T15`: the isolated loopback TLS laboratory described above.
- `T16`–`T20`: repository scan, rendered service secret inventory, container
  environment/mount inspection, Inspector inspection, and redacted failure
  diagnostics.
- `T21`–`T26`: controlled working-role rotation after the accepted backup,
  old/new connection probes, owner-mode checks, deliberate secret-scanner
  fixtures outside Git, and build/dump-context inspection.
- `T27`–`T30`: frozen-V1 hashes and live flow, current-V2 live flow,
  unsupported-generation refusal, ordinary artifact string/config inspection,
  and exact absence of Protocol V3/default TLS.

The report schema is `revenant.m30.transport-secrets-gate.v1` with exactly
thirty ordered rows. Every row records its case ID, layer, input category,
expected/observed disposition, pre/post protected-state digest, mutation count,
redaction result, duration, resource sample, and pass boolean.

## Stop boundary

A non-loopback listener, secret output, key in an image/evidence/repository,
working-data mismatch, unrecoverable role rotation, V1/V2 regression, normal
TLS requirement, Protocol V3 symbol, or inability to return to healthy normal
services stops and rolls back Gate 3. Gate 4 remains locked until the exact
matrix passes twice and the canonical gate is green.
