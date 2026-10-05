# M30 Gate 3 — transport and secrets audit

Date: 2026-09-08  
Decision: **Gate 3 approved; hardened local infrastructure retained**  
Accepted evidence:
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate3-transport-secrets-20260908h`

The accepted `SHA256SUMS` covers 30 evidence payload files totaling 155,036
bytes. Its SHA-256 is
`5673b1542bb5c4398813970110aa48c804e1f2b770a7187386abf8f9db9fcf9e`,
and every entry verifies.

## Backup and reviewed database update

The pre-change evidence at
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate3-prechange-20260908b`
contains the healthy PostgreSQL 16.11 working database as a custom-format
dump. The dump SHA-256 is
`c9d00d082e11059f724c21e3341b9865a39117c9182a5c328ae7a7afb4b8e894`.
It restored into a network-isolated, tmpfs-backed PostgreSQL 16.15 container
with byte-identical table inventory, ordered rows, counts, sequences, schema
catalog, schema text, and extensions. The disposable instance was removed and
the working database was never a restore target.

The retained service stays on major 16 and is pinned to the reviewed Docker
Official Image `postgres:16.15-alpine` repository digest
`sha256:cf78e76683b9ca8c5733cbbdce6c9262b45b6767934dd0a95e671f9a0fc20685`.
The live server reports 16.15.

## Retained boundary

Normal Compose has no PostgreSQL host publication. Gateway ports 7000/8080
and Inspector port 4173 publish only on literal IPv4 loopback. The separately
named maintenance override is the sole PostgreSQL publication and binds only
`127.0.0.1`. Rendered-config validation rejects wildcard, empty, hostname,
IPv6, LAN, public, normal-PostgreSQL, and Inspector-secret fixtures before a
listener starts.

The repository contains secret names and validation rules, never values.
Current and previous generations each contain exactly five owner-only files
under owner-only Linux directories. The exact Compose grants are:

- PostgreSQL: administrator password only;
- migration job: administrator database URL only;
- provisioner: runtime password only, with administrator access through the
  shared local PostgreSQL Unix socket;
- Gateway: runtime database URL only; and
- Inspector: no secret and no database-bearing environment entry.

The Gateway entrypoint stages its mounted URL at mode `0400` and then replaces
itself with the runtime process as UID 100/GID 101. The runtime database login
is non-superuser, cannot create roles or databases, cannot replicate or bypass
row security, owns no object, has no public database/schema creation right and
has no grant option. Its grants match the frozen sixteen-table write map, all
table reads, and the two required sequence usages exactly. Gateway uses
`connect_existing` and cannot run migrations or DDL.

Generation is non-overwriting and uses 256 random bits per password. Rotation
and rollback both require the verified backup, change both database logins in
one transaction through the local socket, recreate affected jobs/services,
accept the installed values, reject the displaced values over SCRAM network
connections, preserve the normalized working-data digest, and emit no value.

## Exact matrix and compatibility result

Two independent invocations emitted schema
`revenant.m30.transport-secrets-gate.v1` and exactly `T01` through `T30` in
order:

- report A: 30/30 passing;
- report B: 30/30 passing;
- combined: 60/60 passing;
- redaction failures: zero;
- undeclared stable-case state changes: zero;
- declared mutating cases per run: only `T27` and `T28`; and
- normalized decisions: byte-identical.

The isolated TLS cases use a fresh temporary local CA and loopback endpoint.
They accept only the trusted `localhost` certificate and reject plaintext,
untrusted, wrong-host, expired, not-yet-valid, missing-key, mismatched-key, and
downgrade cases. Key material is destroyed; scans find no private key,
current/previous database secret, or authenticated database URL in the
repository, image, dump, logs, or accepted evidence.

After the final dependency-ordered startup, frozen Protocol V1 completed
through the compatibility adapter, current Protocol V2 completed the full
activity, and requested Protocol V3 was refused without negotiation. The
normal repository contains no Protocol V3 or default-TLS symbol. PostgreSQL,
Gateway, and Inspector are healthy; migration and provisioner exited 0; only
ports 7000, 8080, and 4173 listen on host loopback.

The uninterrupted canonical `make check` passed version consistency, Rust
formatting, strict all-feature Clippy, 220 Rust tests, workspace build,
Inspector TypeScript/build, the 410-file secret audit, PostgreSQL persistence
and replay, Godot 4.7.1 validation, current gameplay smoke, frozen V1, and the
isolated reconstruction flow.

## Review corrections

Candidate `m30-gate3-transport-secrets-20260908f` is rejected because its
matrix harness had a false-positive scan, argument handling defect, and stale
local binary. Candidate `...20260908g` is rejected because final review found
that the provisioner received both administrator and runtime secrets instead
of the exact single grant. Both directories remain labeled rejected and are
not inputs to this decision.

During the final review, closing the maintenance override by recreating only
PostgreSQL left an already-running Gateway connection pool stale for one
ad-hoc V1 attempt. Recreating migration, provisioner, Gateway, and Inspector
in dependency order after database health returned produced an immediate clean
V1/V2/V3 result and is now the documented close procedure. The accepted
twice-repeated matrix itself had no failed run. Persistent reconnection under
injected container faults remains a Gate 5 claim and is not inferred here.

## Decision and residuals

Gate 3 is green and its local infrastructure hardening is retained. It meets
the frozen transport, secret, database-role, rotation, rollback, isolated TLS,
redaction, compatibility, and working-data boundaries without Protocol V3,
normal TLS, LAN/public exposure, gameplay changes, or version change.

The normal V1/V2 wire still uses username-only local development identity and
plaintext loopback transport. The Gate 2 identity/session model remains
default-disabled and is not integrated. TLS is laboratory-only. The
administrator role remains intentionally powerful for migrations; current and
previous secret generations remain same-host material. Inspector disclosure,
per-connection abuse limits, bounded outbound queues, structured redacted
security events, resource ceilings, and log rotation remain unresolved until
Gate 4. Integrated backup/reconnect/crash recovery remains unresolved until
Gate 5.

Only Gate 4 observability/abuse may begin next. Its implementation contract
must first measure legitimate M28 traffic and resource peaks, then freeze
finite queue and container ceilings before changing the runtime.
