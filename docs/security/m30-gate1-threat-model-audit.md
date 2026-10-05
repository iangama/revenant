# M30 Gate 1 — threat-model and specification audit

Date: 2026-09-07  
Decision: **Gate 1 approved**  
Accepted evidence:
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate1-security-audit-20260907a`

The accepted directory contains 16 manifested files (18,717 bytes).
`sha256sum --check SHA256SUMS` passes, and the manifest SHA-256 is
`ab6c3c13df4543d2ee0150fa023d12ebeacfd9929693e7c59b1827c9ea045674`.
The credential-bearing source lines in the preliminary snapshot were redacted
before acceptance; a fixed-value and credential-bearing-URL scan of the final
directory returns zero matches.

## Decision

The exact M30 local security/recovery contract is frozen in
`docs/milestones/M30-local-security-recovery-laboratory.md`. It defines nine
trust zones, nine protected assets, seven in-scope actor/fault classes, 30
ranked STRIDE threats, four 30-vector block matrices, a three-repetition
360-row closure, explicit privacy/log/resource/recovery limits, removal rules,
and a separate Protocol V3/public-exposure stop boundary.

Gate 1 changed no runtime, protocol, client, container, credential, role, or
schema. Its inventory probes were read-only. The required canonical regression
then exercised only its established synthetic smoke identities through normal
authoritative flows; it did not directly edit or delete a pre-existing record.
The green decision means the laboratory is sufficiently bounded to begin; it
does not mean the audited system is secure.

## Evidence reviewed

The accepted evidence records:

- exact date, repository, version, and read-only scope;
- Docker 29.7.2, PostgreSQL 16.11, service health, explicit loopback listeners,
  and container privilege/resource configuration;
- the `revenant` database role's superuser, role/database creation,
  replication, and row-security bypass flags;
- sixteen public tables, 112 table grants, zero public triggers, and zero
  public functions;
- Inspector GET success/CORS behavior and 405 for HEAD, POST, PUT, PATCH,
  DELETE, and OPTIONS;
- source constants for 64 KiB frames, 15-second handshake, five-minute idle,
  64 game connections, 4,096-byte HTTP request lines, loopback defaults,
  username-only identity, and known development database defaults; and
- hashes for the version, infrastructure, identity, protocol, Gateway, backup
  runbook, and frozen V1 boundaries.

The uninterrupted canonical `make check` passed after the specification and
ledger update: version/format/Clippy, all 217 Rust and PostgreSQL tests,
all-target build, Inspector checks/build, secret audit, current multiplayer and
Godot flows, persistence/replay/Inspector reconciliation, frozen-V1
compatibility, and standalone V1 reconstruction.

No credential value was added to the evidence. The known development default
is described as a source finding rather than copied into this audit.

## Key findings

### Controls already present

- All four published service ports were observed on `127.0.0.1` only. Docker's
  current documentation treats explicit localhost publishing as host-only in
  the normal NAT configuration.
- Gateway framing, request-line, connection, handshake, HTTP, and gameplay-idle
  bounds already prevent several unlimited-input cases.
- Inspector exposes only bounded GET routes; all tested mutation methods
  return 405.
- Character ownership, server-authoritative intent validation, transactional
  terminal rewards, replay reconciliation, operation idempotency, and V1/V2
  capability separation are established by prior green milestones.
- Gateway runs nonroot with a read-only filesystem and
  `no-new-privileges`.

### Material open risks

- A syntactically valid username is treated as authenticated without any
  secret or proof of possession. Loopback narrows reachability but does not
  distinguish local processes.
- The application runtime connects as the sole PostgreSQL cluster superuser.
  A Gateway/database injection or process compromise therefore has far more
  privilege than existing prepared operations require.
- A committed development password and credential-bearing environment URL are
  normal defaults. Docker recommends per-service secret mounts over broadly
  exposed environment variables.
- Inspector history is unauthenticated and permits any browser origin through
  CORS `*`. It is read-only, but account/activity/replay disclosure remains
  possible to a same-host process or malicious browser origin.
- The Gateway has no per-connection message-rate or outbound-queue bound;
  services have no configured memory/PID caps; Docker log retention is
  unbounded.
- Backup instructions exist, but there is no single automated sealed command,
  corrupt-input matrix, exact active-data nonmutation proof, or measured
  restore-to-service objective.
- The running PostgreSQL 16.11 image is behind the then-current fixed 16.x
  minors listed by PostgreSQL's security page. Gate 1 deliberately performs no
  image/volume update before backup and compatibility proof.

## Research reconciliation

The specification uses authoritative current guidance without claiming formal
conformance:

- OWASP's four threat-model questions justify separating inventory, threats,
  mitigations, and verification, while STRIDE supplies only the organizing
  labels.
- OWASP's minimum session entropy is exceeded by the laboratory's 256-bit
  opaque values. Idle/absolute expiry, server-side invalidation, rotation, and
  digest-only log correlation are explicit test cases.
- OWASP's persistent-connection guidance supports the existing 64 KiB bound
  and the new rate, idle, backpressure, and resource vectors. Its WebSocket
  examples are not misrepresented as Revenant's TCP implementation.
- PostgreSQL role/authentication and backup documentation support least
  privilege, constrained login, verified dumps, and real restore drills.
- Docker's network and secrets documentation supports explicit loopback
  publishing and service-scoped secret files.
- NIST recovery material supports a documented saved recovery code, but the
  pure lab intentionally makes no AAL claim. NIST event-recovery guidance
  supports measuring restoration and reconciliation.
- Godot's TLS documentation supports a server-only key and strict certificate
  validation. Mandatory TLS is not integrated because that would strand
  frozen V1 and change the normal transport contract.

## Protocol and product decision

The Gate rejects a silent authentication retrofit to Protocol V2. Its exact
wire request has only a username, and frozen V1 uses the same canonical
boundary. Adding credential proof or authenticated lobby admission would be a
semantic protocol change even if an optional field could deserialize.

M30 therefore keeps Gate 2's identity/lobby/session design pure and removable.
Default V1/V2 remain explicitly development-only username identity. A product
integration requires a separately estimated and owner-approved Protocol V3;
public or LAN exposure requires a separate deployment/security milestone.

This is the honest tradeoff: M30 may reduce local infrastructure, disclosure,
abuse, and recovery risk without falsely claiming that the normal game wire
authenticates accounts.

## Gate result and next boundary

Gate 1 is green with the open risks above. The accepted specification is
finite, testable, local-only, evidence-bounded, and removable. Public service,
external accounts, human testing, Protocol V3, and working-data mutation are
not authorized.

Only Gate 2 may open next. It must checkpoint the exact Gate 1 repository,
implement an off-by-default pure identity/session laboratory, execute
`I01`–`I30` twice with byte-identical redacted reports, prove default artifact
exclusion, and either retain that isolated lab or remove it completely. Gate 3
transport/secrets and every normal-service change remain locked until Gate 2
is decided.
