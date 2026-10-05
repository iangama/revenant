# M30 — Local identity, lobby, security, and recovery laboratory

Status: **M30 complete under the owner-revised process — 2026-09-09**.
Gates 1-5 retain their accepted evidence. Gate 6 closed by reviewing those
results, the existing final quality pass, and residuals; its extra 360-row
repetition was removed at the owner's request, not executed. M31 content is
next. The proportional policy in `AGENTS.md` supersedes earlier repetition and
blanket validation requirements. Gate decisions below retain their historical
context; the current closure is recorded at the end of this document.

## Authorization and honest boundary

The owner authorized sequential work through the existing M25–M32 roadmap.
M29 is complete by evidence-based removal, so M30 may proceed one reviewed
gate at a time. This milestone protects one creator-operated hobby deployment
on one host. It does not turn Revenant into a public or production service.

M30 excludes Internet exposure, LAN hosting, matchmaking, public discovery,
email or telephone identity, social login, password support, certificate-
authority operations, support staffing, moderation, telemetry, remote backup,
continuous archiving, production availability, compliance certification,
another person, commit, push, merge, version change, tag, release, or
distribution. No security claim extends beyond the exact local host and finite
matrix in this document.

The following remain protected throughout M30:

- `VERSION=0.2.0`, Protocol V2, and both frozen V1 source hashes;
- all M25–M28 combat, module, route, cooperation, reward, persistence, replay,
  Inspector, and Godot behavior;
- the sixteen-table working database and all non-test records; the canonical
  gate may append only its established synthetic smoke identities through
  normal authoritative flows;
- the final-video presentation pass and repository-owned assets; and
- all external checkpoints and accepted evidence.

No live credential, invite, session token, recovery code, private key,
database password, or unredacted connection string may enter the repository,
logs, Inspector, test reports, command transcript, or accepted evidence.

## Research basis

External references are design constraints, not a claim of certification:

- OWASP's
  [threat-modeling guidance](https://cheatsheetseries.owasp.org/cheatsheets/Threat_Modeling_Cheat_Sheet.html)
  organizes the review around system modeling, what can go wrong, mitigations,
  and verification. M30 uses a data-flow/trust-zone inventory and a bounded
  STRIDE register for that purpose.
- Docker's
  [port-publishing documentation](https://docs.docker.com/engine/network/port-publishing/)
  distinguishes unspecified-host publication from explicit localhost
  publication. Every normal Revenant host mapping remains explicit loopback;
  a public or LAN bind is a milestone stop, not a test variant.
- Docker's
  [Compose secrets guidance](https://docs.docker.com/compose/how-tos/use-secrets/)
  explains why credentials in environment variables can leak and how
  per-service file mounts narrow access. Gate 3 must replace source/default
  database secrets without placing generated values in Git.
- PostgreSQL documents that the active
  [database role determines its initial privileges](https://www.postgresql.org/docs/16/database-roles.html)
  and that client identity must be restricted by
  [authentication configuration](https://www.postgresql.org/docs/16/client-authentication.html).
  The current all-powerful runtime role is therefore a finding, not an
  acceptable consequence of local-only operation.
- PostgreSQL's
  [backup and restore chapter](https://www.postgresql.org/docs/16/backup.html)
  distinguishes SQL dumps, filesystem backups, and continuous archiving. M30
  proves only a custom-format dump and disposable restore; it makes no PITR or
  off-host durability claim.
- OWASP recommends unpredictable session identifiers, server-enforced idle
  and absolute expiry, lifecycle logging without raw token values, and
  resource bounds in its
  [session-management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
  and
  [persistent-connection security](https://cheatsheetseries.owasp.org/cheatsheets/WebSocket_Security_Cheat_Sheet.html)
  guidance. Although Revenant uses framed TCP rather than WebSocket, the frame,
  connection, idle, rate, and backpressure principles apply by analogy.
- NIST SP 800-63B recognizes saved recovery codes and requires documented,
  risk-based recovery methods in its
  [account-recovery guidance](https://pages.nist.gov/800-63-4/sp800-63b.html#account-recovery).
  Revenant intentionally models one local recovery code and does not claim a
  NIST assurance level.
- Godot's
  [TLS certificate guidance](https://docs.godotengine.org/en/4.7/tutorials/networking/ssl_certificates.html)
  requires the server private key to remain server-only and describes
  development self-signed certificates. Gate 3 is an isolated provisioning
  experiment, never a reason to ship a private key or disable hostname checks.
- NIST SP 800-184 treats recovery as a planned, measured capability in its
  [cybersecurity event recovery guide](https://csrc.nist.gov/pubs/sp/800/184/final).
  M30 therefore measures restore and reconciliation rather than treating a
  successful dump command as recovery proof.

## Audited starting state

The read-only Gate 1 audit found the following exact baseline on 2026-09-07:

- Docker Engine and client are 29.7.2. Gateway, Inspector, PostgreSQL, and all
  host-published listeners are healthy and bound only to `127.0.0.1` on ports
  7000, 8080, 4173, and 5432.
- The Gateway container runs as `revenant`, is read-only, and has
  `no-new-privileges`. Inspector and PostgreSQL also have
  `no-new-privileges`, but are not read-only; Inspector has no explicit
  nonroot user. No service has a configured memory or PID bound.
- Protocol framing already rejects payloads beyond 64 KiB. The Gateway has a
  15-second handshake timeout, five-minute gameplay idle timeout, 64 global
  game-connection limit, 4,096-byte HTTP request-line bound, and five-second
  HTTP timeout. It has no per-connection message-rate or outbound-queue bound.
- `LocalIdentityService::authenticate` validates only a 1–32-byte ASCII
  username and derives `local:<lowercase-name>`. It proves no secret or
  possession. Any same-host process that knows a name can request that
  identity.
- The V2 wire `AuthRequest` contains only `username`. Frozen V1 is adapted to
  the same canonical request. Character ownership is correctly checked after
  authentication, but that check inherits the unproven account identity.
- Inspector session, event, and summary reads require no credential and return
  `Access-Control-Allow-Origin: *`. Every tested non-GET method returns 405,
  and route identifiers are character constrained. Read-only semantics reduce
  tampering risk but do not prevent disclosure to another same-host process or
  browser origin.
- Compose publishes a known development database credential through defaults
  and `DATABASE_URL`. PostgreSQL 16.11 uses SCRAM for nonlocal host entries,
  but the sole `revenant` login role is superuser with role/database creation,
  replication, and row-security bypass privileges. It has 112 public-table
  grants across sixteen public tables.
- The running PostgreSQL 16.11 image is below the then-current fixed 16.x
  security minors listed by PostgreSQL. Updating or replacing it is deferred
  to Gate 3 so the working volume is backed up and compatibility-proven first.
- The repository has a manual custom-format `pg_dump`/checksum/disposable-
  restore runbook and prior milestone restore evidence. It has no automated
  sealed backup command, retention policy, scheduled backup, failure-injection
  recovery harness, measured restore target, or default migration rollback
  path.
- Gateway logging is line-oriented JSON for some events and unstructured
  errors for others. There is no common connection correlation identifier,
  explicit redaction contract, Docker log rotation, or bounded local security
  event set.

These findings do not imply an observed compromise. They define what the
current evidence can and cannot support.

## Trust zones and data flow

| Zone | Contents | Trust and permitted flow |
| --- | --- | --- |
| Z0 — owner/operator | Host login, repository, Docker control, generated secrets, backup custody | Trusted administrator. A compromised administrator or Docker daemon is out of scope; ordinary operator error is in scope. |
| Z1 — untrusted local client | Godot, fake client, frozen V1, arbitrary same-host TCP process | May submit bytes and intents only. Never trusted for identity, authorization, state, time, damage, reward, or recovery. |
| Z2 — loopback edge | Host ports 7000/8080/4173 and optional isolated TLS lab | Localhost narrows reachability but is not authentication. Non-loopback publication is forbidden. |
| Z3 — Inspector browser | Static UI and read-only HTTP proxy | Untrusted origin until explicitly allowed. May receive minimum read-only projections after Gate 4 controls. |
| Z4 — Gateway authority | Parser, protocol adapter, admission, session coordinator, domain composition | Trusted decision boundary. Must validate all data from Z1/Z2 and use least-privileged dependencies. |
| Z5 — container network | Gateway, Inspector, PostgreSQL bridge traffic and mounted secrets | Host-controlled but not assumed confidential from a compromised container. Access is per-service and minimized. |
| Z6 — PostgreSQL authority | Accounts, characters, inventory, progression, operations, replay | Integrity-critical. Runtime, migration, and backup duties must not share unrestricted privilege. |
| Z7 — recovery/evidence | Host-side dumps, checksums, manifests, redacted logs, checkpoints | Owner-readable, never web-served, never contains live authenticators. Restore targets are checked-absent disposable databases. |
| Z8 — compatibility archive | Frozen V1 client and reconstruction source | Immutable input to compatibility proof; it receives no silent authentication retrofit. |

The only normal authoritative flow is:

`Z1 client → Z2 loopback → Z4 parser/authority → Z6 persistence → Z4 projection → Z1 client`

Inspector flow is read-only:

`Z3 UI → Z2 loopback proxy → Z4 bounded Inspector query → Z6 → Z3 minimum projection`

Recovery flow is operator-only:

`Z0 command → Z6 consistent dump → Z7 checksum/archive → checked-absent Z6 disposable restore → reconciliation`

No client, Inspector, backup, or replay path may write authoritative gameplay
state except through an existing Gateway transaction.

## Protected assets and security objectives

| Asset | Primary objective | Gate-M30 requirement |
| --- | --- | --- |
| A1 account/character ownership | Integrity, authorization | A local credential lab proves possession, collision handling, rotation, revocation, and recovery without claiming wire integration. |
| A2 admission/lobby/session state | Integrity, confidentiality | One owner, at most one invited peer, no discovery, opaque bounded-lifetime values, server-owned lifecycle. |
| A3 combat/items/progression/reward | Integrity, replayability | Unauthorized, duplicate, stale, malformed, and faulted flows make zero protected mutation or duplicate grant. |
| A4 PostgreSQL schema/data | Integrity, recoverability | Least-privileged roles, verified pre-change backup, transactional failure behavior, disposable restore. |
| A5 replay/Inspector evidence | Integrity, minimum disclosure | Replay agrees with persistence; Inspector stays read-only, origin-restricted, bounded, and credential-free in logs. |
| A6 credentials/keys/secrets | Confidentiality, revocability | Generated outside Git, service-scoped, no raw storage/log/evidence, old value rejected after rotation. |
| A7 local logs | Availability, privacy, auditability | Fixed vocabulary, pseudonymous correlation, explicit redaction, 10 MiB maximum retained per service. |
| A8 local service availability | Bounded availability | Finite connections, frames, rates, queues, timeouts, memory/PID configuration, and recovery observations. |
| A9 V2/V1 compatibility | Integrity, removability | Byte-identical frozen V1, unchanged default V2 wire, no V3 symbol or negotiation in default artifacts. |

## Actors and excluded attacker power

In-scope actors are the trusted owner/operator, an ordinary malformed or
malicious same-host client process, a malicious browser origin trying to read
localhost Inspector data, a stale legitimate V1/V2 client, an interrupted
Gateway/Inspector/PostgreSQL process, corrupted disposable recovery material,
and accidental operator misuse of a bounded runbook.

A hostile host administrator, compromised kernel, Docker daemon control,
physical disk extraction, firmware compromise, malware able to read every
owner file/process, supply-chain compromise, and remote Internet attacker are
outside the claim. M30 may reduce consequences of some of these conditions but
cannot use that incidental resistance as acceptance evidence.

## Frozen 30-threat register

Likelihood and impact are each scored 1–3 inside the local scope; risk is their
product (`high=6–9`, `medium=3–4`, `low=1–2`). Existing mitigations are credited
only when observed. “Gate” is the first gate allowed to change the condition.

| ID | STRIDE class and abuse case | Risk | Starting control/finding | Gate |
| --- | --- | ---: | --- | ---: |
| S01 | Claim another username | 9 | Valid syntax only; no proof of possession | 2 |
| S02 | Redeem a copied/replayed invite | 6 | No invite model exists | 2 |
| S03 | Guess, steal, fix, or replay a session value | 6 | No authenticated session model exists | 2 |
| S04 | Treat V1/V2 build text as identity | 4 | Negotiation is compatibility, not identity | 2 |
| S05 | Use recovery to take over an account | 6 | No credential recovery model exists | 2 |
| T01 | Submit unauthorized character/gameplay intent | 6 | Ownership/domain checks exist but inherit S01 | 2/4 |
| T02 | Replay/conflict an operation identifier | 4 | M26–M28 domain idempotency exists | 4 |
| T03 | Mutate all data through the runtime DB role | 6 | Runtime role is superuser/BYPASSRLS | 3 |
| T04 | Leave partial state during migration/transaction failure | 6 | Transaction tests exist; no M30 rollback drill | 5 |
| T05 | Mutate through Inspector HTTP | 2 | Tested non-GET methods return 405 | 4 |
| T06 | Restore a tampered/truncated dump | 6 | Manual checksum/list instructions only | 5 |
| R01 | Deny which local principal performed an auth action | 4 | No proof or correlation ID | 2/4 |
| R02 | Lose event causality across connection/session/database logs | 4 | Mixed structured/unstructured logging | 4 |
| R03 | Rotate/recover without attributable lifecycle evidence | 4 | No rotation/recovery event model | 2/4 |
| I01 | Read Inspector history from another local origin | 6 | Unauthenticated reads and CORS `*` | 4 |
| I02 | Leak DB password through defaults/environment/diagnostics | 6 | Known default and environment URL | 3 |
| I03 | Leak invite/session/recovery values through logs | 6 | Values do not exist; no redaction contract | 2/4 |
| I04 | Expose identities in durable evidence beyond need | 4 | Account/character IDs appear in replay projections | 4 |
| I05 | Disclose a host-side database dump | 4 | Manual owner custody; no permissions/retention proof | 5 |
| D01 | Exhaust 64 global connections | 6 | Global cap exists; no unauthenticated sub-cap | 4 |
| D02 | Flood valid small messages | 6 | Frame limit only; no message-rate bound | 4 |
| D03 | Send oversized, malformed, or incomplete frames | 3 | 64 KiB cap and read timeout exist | 4 |
| D04 | Hold slow handshakes/gameplay sockets | 4 | 15 s handshake and 5 min idle timeouts exist | 4 |
| D05 | Flood HTTP threads/queries | 6 | 4,096-byte line and 5 s timeout; no request rate | 4 |
| D06 | Interrupt Gateway/PostgreSQL around terminal reward | 6 | Transactions/replay exist; combined fault proof absent | 5 |
| D07 | Grow logs, processes, memory, or outbound queues without bound | 6 | No rotation/resource/queue caps configured | 4 |
| E01 | Escalate from Gateway compromise to cluster superuser | 6 | Runtime DB role is cluster superuser | 3 |
| E02 | Reuse one claimed identity on conflicting sockets | 6 | Admission limits actors, not proven principal sessions | 2 |
| E03 | Confuse protocol generation to gain newer authority | 3 | Adapter gates V1 capability; adversarial matrix pending | 4 |
| E04 | Turn a local configuration change into public exposure | 6 | Explicit loopback today; no fail-closed config guard | 3 |

Gate 1 does not mark these threats mitigated. It accepts a complete, bounded
model and assigns every threat to one finite proof or an explicit residual.

## Identity, lobby, and session laboratory contract

Gate 2 is a pure, separately invoked, off-by-default laboratory. It may add
new crates/tools behind one explicit Cargo feature, but it may not alter
`AuthRequest`, the default Gateway, Compose, Godot startup, PostgreSQL, replay,
Inspector, or either archived client.

The model contains exactly one owner and at most one invited peer, one active
lobby, and no lobby discovery. Each local credential, invite, authenticated
session, and saved recovery code is an opaque 32-byte value generated by the
operating system CSPRNG. Reports contain only stable labeled fixture digests,
never generated bearer values. The server model stores only one-way verifiers
for bearer material and uses constant-time verifier comparison.

Fixed lifecycle:

- enrollment creates one account credential and one saved recovery code;
- a credential proof may issue one session scoped to account and lobby;
- sessions are valid through a ten-minute idle deadline and a thirty-minute
  absolute deadline; the first observation after either deadline invalidates
  them server-side;
- logout, credential rotation, or successful recovery invalidates every prior
  session for that account;
- the owner may create one two-seat lobby and one invite valid through ten
  minutes; the first observation after that deadline invalidates it;
- exactly one peer may redeem the invite once; replay, wrong-lobby use, a
  second peer, and concurrent loser all reject without mutation; and
- join/leave presence is server-owned, contains no chat/status text, and is not
  persisted across laboratory restart.

Authentication failures disclose neither account existence nor which field
failed. At most five failed proofs per normalized account in any rolling
60,000 ms window are evaluated; the sixth rejects as rate-limited. The first
new attempt is eligible at the exact expiry of the oldest retained failure.
Injected monotonic time proves every exact and plus-one boundary.

Gate 2 keeps the laboratory only if all 30 `I` vectors below pass twice with
byte-identical redacted reports, all rejected cases preserve account,
credential, lobby, membership, and session state, no raw secret is recoverable
from reports/process output, and default artifacts contain no laboratory
symbol. A failed credential proof may update only its bounded rate-limit
metadata; reports count that separately from protected mutation. Otherwise
every Gate 2 implementation file is removed and the Gate 1 checkpoint is
restored.

## Protocol V3 decision boundary

Gate 1 decides **not to create Protocol V3 in M30**. Proof of possession cannot
be added honestly to the frozen username-only V1/V2 authentication message
without changing its semantics and every client handshake. Build strings,
loopback source address, character ownership, or an optional field are not
accepted substitutes for authentication.

Consequently:

- Gate 2 proves the identity/lobby/session design only in isolation;
- Gate 3 may test TLS provisioning only in an opt-in isolated harness;
- normal V2 and frozen V1 continue to be labeled development-only username
  identity throughout M30; and
- any request to integrate credential proof, lobby admission, authenticated
  Inspector sessions, or mandatory TLS into the default game wire stops M30,
  preserves evidence, and requires a separately estimated Protocol V3
  milestone with explicit owner approval.

M30 can still harden loopback enforcement, database privilege, secrets,
containers, logging, Inspector disclosure, abuse limits, and recovery without
making that protocol change. Gate M30 must retain S01 as an explicit residual;
it may not claim account security for normal V1/V2 clients.

## Transport, secrets, and database contract

Gate 3 may change normal infrastructure only after a verified pre-change dump
and disposable restore. Its target is:

- Gateway and Inspector remain explicit IPv4 loopback publications; the
  normal PostgreSQL service has no host-published port. A separately named
  operator profile may expose PostgreSQL on `127.0.0.1` for bounded
  maintenance only.
- Empty, wildcard, LAN, hostname-resolved non-loopback, and public bind values
  fail before a listener starts. Container-internal listeners may remain
  `0.0.0.0` only behind explicit loopback host mappings.
- The known password and credential-bearing `DATABASE_URL` disappear from
  committed defaults. Owner-generated database material is mounted only to
  PostgreSQL and Gateway through service-scoped secret files. Inspector never
  receives a database credential.
- Separate cluster administrator/migration and runtime login roles are used.
  The runtime role has `LOGIN`, schema usage, and only the exact table/sequence
  rights needed by existing prepared operations; it has no superuser,
  role/database creation, replication, row-security bypass, ownership, DDL,
  arbitrary public create, or grant-option privilege.
- PostgreSQL moves from 16.11 only after backup/restore and release-note
  review, remains on major 16 for M30, and proves all migrations/tests against
  the candidate minor before the working volume is touched.
- The isolated TLS lab provisions a generated server-only private key, trusted
  public certificate, hostname verification, refusal of untrusted/mismatched/
  expired material, and no plaintext downgrade when TLS is required. It
  neither edits frozen V1 nor enables TLS in the normal Gateway.
- Secret rotation starts new connections with the new value, rejects the old
  value, does not print either, preserves working data, and has a documented
  rollback using the still-protected previous value. No secret is baked into
  an image layer or repository file.

Any working-volume change, V1/V2 failure, secret emission, non-loopback
listener, inability to restore the pre-change snapshot, or need for Protocol
V3 rejects Gate 3 and invokes the bounded rollback.

## Observability, Inspector, and abuse contract

Gate 4 may add only controls that preserve accepted gameplay timing and
authority:

- The 64 KiB frame, 64 global connection, 15-second handshake, five-minute
  gameplay idle, 4,096-byte HTTP line, and five-second HTTP limits remain.
- At most eight connections may be simultaneously unauthenticated. The ninth
  receives a bounded rejection while established participants remain live.
- Each admitted game connection accepts at most 32 frames in a one-second
  burst and 600 frames in a rolling minute. This exceeds the current client's
  approximately 8.34 movement intents/second plus combat/UI traffic. The first
  excess frame closes only that connection; authoritative state stays valid.
- Outbound projection queues are finite. A slow reader is disconnected rather
  than permitting unbounded retained messages. Exact capacity is frozen in
  the Gate 4 implementation contract after measuring the largest M28 success
  burst, with at least 2× that observed burst and at most 256 queued messages.
- Compose configures per-service memory and PID ceilings after baseline
  measurement, with at least 2× accepted M28 peaks and explicit absolute caps.
  It also retains at most five 2 MiB local log files per service (10 MiB).
- Structured events use a fixed allow-list and an opaque per-connection
  correlation digest. No raw credential, invite, session/recovery value,
  database URL/password, private key, arbitrary payload, or stack dump is
  logged. Account/character values are pseudonymized for security events.
- Session lifecycle logging records only creation/use/renewal/revocation/
  expiry outcome and correlation digest. Rejected input logs category and
  bounded lengths, never input bodies.
- Inspector remains GET-only and finite. CORS permits only the configured
  loopback Inspector origin, not `*` or reflected input. API responses use
  `Cache-Control: no-store`; methods other than GET remain 405; malformed or
  oversized paths remain bounded; failure bodies expose no database detail.

The TCP service is not WebSocket, so browser-specific cookie controls do not
apply to its game framing. Browser origin and caching controls do apply to the
Inspector HTTP projection.

## Backup, fault, and recovery contract

Gate 5 creates a non-destructive owner command that:

1. checks service/database identity and refuses an unexpected target;
2. writes a custom-format dump to a newly created owner-only host directory;
3. writes a SHA-256 sidecar and a redacted metadata manifest;
4. validates both checksum and `pg_restore --list`;
5. restores into a checked-absent, tokenized disposable database;
6. applies the full existing migration list twice;
7. compares schema cardinality plus canonical account, character, inventory,
   progression, operation, reward, and replay exports to the sealed fixture;
8. exercises Gateway and Inspector against the disposable target; and
9. removes only the exact disposable database after recording reconciliation.

The accepted small-fixture recovery objective is zero difference from the
state sealed at dump time and at most ten minutes from restore start through
healthy Gateway/Inspector reconciliation on this machine. This is a measured
laboratory RPO/RTO, not a time-based production promise: changes after the dump
are outside that snapshot, backups are not scheduled/off-host, and no PITR is
claimed.

Fault tests stop/restart only named local containers or isolated processes.
They must prove that interrupted nonterminal work remains incomplete, a
transaction failure grants nothing, a committed terminal retains exactly one
reward/replay result, retry never duplicates it, corrupt/truncated material is
rejected, and the working database is never a destructive restore target.

## Exact 120-vector acceptance matrix

Every report row has `case_id`, layer, initial-state digest, input category,
expected disposition, observed disposition, post-state digest, mutation count,
redaction result, duration, and resource sample. A reject passes only when its
protected post-state digest equals pre-state unless the contract explicitly
requires an expiry/revocation terminal. Failed-proof rate metadata is excluded
from that protected digest, remains finite, and is reported in the resource
sample.

### Gate 2 — identity/session (`I01`–`I30`)

| IDs | Exact cases |
| --- | --- |
| I01–I05 | owner enrollment; canonical lowercase account ID; invalid username; case-fold collision; identical enrollment retry |
| I06–I10 | conflicting enrollment; valid credential proof; wrong credential; unknown account indistinguishable from wrong credential; malformed proof |
| I11–I14 | fifth failed proof allowed; sixth rate-limited; exact rolling-window expiry; plus-one retry after expiry |
| I15–I18 | session issuance; opaque/256-bit value properties; valid scoped use; wrong account/lobby scope rejection |
| I19–I22 | exact idle boundary; idle plus-one expiry; exact absolute boundary; absolute plus-one expiry |
| I23–I26 | logout revocation; credential rotation revokes old sessions; recovery rotates credential/recovery code; recovery replay rejects |
| I27–I30 | one lobby/two-seat creation; one-time invite redemption; server-owned peer join/leave presence; concurrent invite redemption admits exactly one |

### Gate 3 — transport/secrets (`T01`–`T30`)

| IDs | Exact cases |
| --- | --- |
| T01–T06 | Gateway loopback; Inspector loopback; PostgreSQL absent from normal host listeners; wildcard bind refusal; non-loopback bind refusal; rendered Compose listener audit |
| T07–T11 | TLS-required plaintext refusal; trusted certificate success; untrusted certificate refusal; hostname mismatch refusal; expired/not-yet-valid refusal |
| T12–T15 | missing/wrong private key refusal; private key absent from client/image/evidence; downgrade refusal; fresh TLS reconnect after clean close |
| T16–T20 | no committed default DB secret; per-service secret grants; Gateway runtime secret only; Inspector receives no DB secret; diagnostics/logs redact environment and URLs |
| T21–T26 | new DB secret accepted; old secret rejected; restart uses new secret; owner-only secret permissions; secret scanner rejects key/credential fixtures; dump/image context excludes secrets |
| T27–T30 | frozen V1 hashes/flow; current V2 flow; unsupported generation refusal; no Protocol V3/default TLS symbols in ordinary artifacts |

### Gate 4 — abuse/observability (`A01`–`A30`)

| IDs | Exact cases |
| --- | --- |
| A01–A06 | 64 KiB frame accepted; plus-one frame rejected; incomplete size prefix timeout; incomplete body timeout; invalid MessagePack rejected; wrong first message rejected |
| A07–A11 | unsupported version; 4,096-byte HTTP line accepted; plus-one line rejected; every non-GET method 405; invalid/unsafe Inspector path 404 |
| A12–A16 | 64th global connection bounded; 65th rejected; eighth unauthenticated connection bounded; ninth rejected; exact 15-second handshake boundary |
| A17–A21 | handshake plus-one closes; exact five-minute idle boundary; idle plus-one closes; exact frame-rate ceiling; first excess frame closes only offender |
| A22–A24 | fresh connection after throttle window; slow-reader outbound-queue bound; measured memory/PID caps under abuse matrix |
| A25–A30 | correlation lifecycle; bearer/recovery redaction; DB URL/error redaction; five-by-2-MiB log rotation; allowed Inspector origin/no-store; disallowed/missing origin plus all non-GET methods fail closed |

### Gate 5 — backup/recovery (`R01`–`R30`)

| IDs | Exact cases |
| --- | --- |
| R01–R06 | target/health preflight; custom dump; checksum verification; restore-list verification; wrong checksum refusal; truncated archive refusal |
| R07–R12 | checked-absent disposable target; restore success; schema/object cardinality; canonical row/replay equality; archive contains no secret material; active database untouched |
| R13–R18 | Gateway killed during incomplete operation; Gateway restart; PostgreSQL stopped during active operation; PostgreSQL restart; Gateway persistence reconnect; Inspector kill/restart |
| R19–R22 | forced transaction rollback has zero partial grant; committed terminal retry has no duplicate; corrupt disposable archive rejected; corrupt disposable database is detected before acceptance |
| R23–R27 | pre-migration sealed snapshot; candidate migration once; candidate migration twice; pre-migration disposable restore; legacy exports byte-identical |
| R28–R30 | current session/reward/replay reconciliation; full restore-to-health within ten minutes; exact disposable teardown with working volume retained |

The original Gate 6 plan required three full repetitions of these 120 vectors
(360 additional rows). The owner removed that requirement on 2026-09-09.
Closure now reviews the accepted Gates 1-5 and existing final quality result;
rerun only a relevant case if changed code, a failure, or an unresolved concern
invalidates its evidence. No extra repetition is claimed or required.

## Evidence, privacy, and retention

Each gate writes to a new host-side directory under
`/mnt/c/Users/Ian/revenant-local-evidence/`. A candidate directory is immutable
after manifesting. Superseded candidates remain labeled rejected; they are not
silently overwritten. Accepted directories contain `SHA256SUMS`, a manifest
hash recorded in the gate audit, exact command/version context, redacted
reports, and protected-boundary hashes.

Security evidence is retained through M32 archival closure unless the owner
explicitly requests earlier removal. It contains no live authenticator or
unnecessary personal data. Test identities are fixed synthetic labels.
Generated secrets and private keys live only in owner-only temporary locations
and are destroyed after their test; reports prove absence rather than archive
the value. Database dumps remain owner-controlled outside the repository and
are never copied into a browser-accessible or release directory.

## Gates and rollback/removal rules

1. **Gate 1 — threat model/specification:** complete this finite document,
   current-state audit, evidence manifest, and checkpoint. No implementation.
2. **Gate 2 — identity/session lab:** execute `I01`–`I30` twice. Keep only a
   pure removable laboratory that passes exactly; otherwise delete it and
   prove Gate 1 restoration.
3. **Gate 3 — transport/secrets:** execute `T01`–`T30` after verified backup.
   Keep only V1/V2-compatible infrastructure changes with clean rollback.
4. **Gate 4 — observability/abuse:** execute `A01`–`A30`; retain only limits
   above measured legitimate peaks with no authority/timing regression.
5. **Gate 5 — backup/recovery:** execute `R01`–`R30` using checked-absent
   disposable targets and meet the measured recovery objective.
6. **Gate 6 — closure (revised):** review accepted Gates 1-5, the existing
   quality/compatibility/protected-input results, and residuals. Reuse valid
   evidence; no additional 360-row campaign or full-suite rerun is required.

Stop, preserve evidence, and re-estimate instead of expanding scope if a gate
requires Protocol V3, non-loopback networking, an external identity/certificate
or storage service, a destructive working-data operation, a new public API,
human participation, or a changed gameplay/content contract.

Any laboratory is removed if it cannot pass its frozen matrix, leaks a secret,
cannot be cleanly disabled, changes a protected default artifact, or requires
weakening a threshold. Any normal infrastructure change is rolled back if it
breaks V1/V2, accepted gameplay, working data, or loopback-only service health.
Early evidence-based removal is a valid gate outcome; an unproven retained
security surface is not.

## Gate 1 decision and residuals

Gate 1 is **green** because the current surface was audited without mutation;
trust zones, assets, actors, 30 threats, privacy/retention, recovery objectives,
resource targets, exact 120-vector matrix, removability, and Protocol V3/public
exposure boundaries are explicit and finite.

No threat is claimed mitigated by this decision. The highest current residuals
remain username impersonation, unrestricted runtime database privilege, known
environment/default database credentials, same-host Inspector disclosure,
unbounded per-connection traffic/resources/log retention, and unmeasured
integrated recovery. PostgreSQL 16.11 is also behind the then-current fixed
16.x security minors. Normal services remain local development services.

Gate 2 subsequently created and verified that checkpoint and is green under
`docs/security/m30-gate2-identity-session-audit.md`. Only Gate 3
transport/secrets may begin next, after a new verified pre-change database
backup and disposable restore. Protocol, Godot, and gameplay changes remain
locked.

## Gate 2 decision and residuals

Gate 2 is **green**. The retained `revenant-local-security-lab` is a separately
invoked, default-disabled pure package behind the explicit
`m30-identity-session-lab` feature. Its two byte-identical reports execute
`I01`–`I30` in order with 30/30 passes, zero raw bearer values, no default
Gateway symbol, and exact maximum state of two accounts, one stored session,
two lobby members, and five retained failures.

The laboratory uses OS-generated 256-bit values, domain-separated SHA-256
one-way verifiers, constant-time comparison, canonical identities, a shared
bounded unknown-principal rate bucket, single-active-session replacement,
server-owned expiry/revocation, one two-seat lobby, a one-time invite, and an
actual two-thread redemption race with exactly one winner. Failed proofs alter
only bounded rate metadata; every rejected identity/lobby/session case retains
the protected-state digest unless expiry or revocation is its declared result.

Gate 2 changes no default V1/V2 message, identity service, Gateway, Godot,
Compose service, migration, replay, Inspector, or working-database object. It
does not authenticate normal game clients and makes no production, human,
public-service, or NIST assurance claim.

The residual is deliberate: these are pure state transitions with injected
time and synthetic values, not durable credentials, wire authentication,
transport encryption, process limits, or recovery. Gate 3 may now address only
the frozen `T01`–`T30` transport/secrets boundary. It must first seal and
restore a pre-change database backup; any Protocol V3 need or compatibility,
secret, public-bind, or restore failure stops the gate.

## Gate 3 decision and residuals

Gate 3 is **green** under
`docs/security/m30-gate3-transport-secrets-audit.md`. Its two independent
reports execute exactly `T01`–`T30` with 60/60 combined passes and
byte-identical normalized decisions. The retained normal topology is
loopback-only, PostgreSQL has no host publication, service secrets and database
roles are separated with exact least-privilege grants, rotation and rollback
preserve data while rejecting displaced values, and PostgreSQL is pinned to
the reviewed 16.15 digest after a verified disposable restore.

Frozen V1 and current V2 remain live; V3 is refused, and normal TLS remains
disabled. The TLS proof is isolated and destroys its private material. The
accepted evidence contains no live or previous secret, private key, or
authenticated database URL. The canonical gate and final dependency-ordered
normal startup both pass.

This does not authenticate normal V1/V2 clients or claim encrypted normal
transport. Inspector disclosure, structured redacted security events,
per-connection abuse limits, outbound queue bounds, process/resource ceilings,
and log rotation remain Gate 4 work. Integrated backup, fault, reconnect, and
recovery claims remain Gate 5 work. Only the frozen `A01`–`A30` Gate 4 boundary
may proceed next.

## Gate 4 decision and residuals

Gate 4 is **green** under
`docs/security/m30-gate4-abuse-observability-audit.md`. Two fresh isolated
stacks passed 60/60 A01–A30 cases with identical normalized reports and no
undeclared state mutation. Retained frame/admission/rate/queue/deadline limits,
redacted correlation logs, exact Inspector origin/no-store behavior, and
container memory/PID/log ceilings preserve the accepted local runtime.

The uninterrupted 229-test canonical gate, all 48 Gateway tests, two M28
cooperation successes, real-browser proofs, and final normal-image V1/V2 and
unsupported-generation checks pass. The working volume is retained, temporary
fixtures are removed, normal services are healthy, and host port 5432 is closed.

This does not integrate credential authentication or TLS into V1/V2. Native
same-host processes can spoof Origin; bounded resources do not guarantee
availability against a persistent local attacker. Long exact timeout and
slow-reader queue boundaries use shared production code with controlled
fixtures; resource peaks are sampled on this host. Backup, crash/reconnect,
transaction-fault recovery, and the full 360-row closure remain unproven M30
work. Only Gate 5's frozen R01–R30 boundary may proceed next.

## Gate 5 closure — 2026-09-09

Gate 5 is green under `docs/security/m30-gate5-recovery-audit.md`. Two fresh
R01–R30 matrices passed 60/60 cases with byte-identical normalized reports.
Custom backup, corrupt/truncated rejection, exact restoration and two migration
passes, process interruption, persistence reconnect, transaction rollback,
committed retry, and restored Gateway/Inspector/session reconciliation passed.
Full application recovery measured 118,072/117,726 ms against the 600-second
ceiling. Working data remained unchanged throughout both matrices, and every
generated fixture resource was removed.

The final uninterrupted canonical gate passed 229 Rust tests plus 26 Python
safety tests, builds/lints, Godot/gameplay/replay/Inspector, frozen V1 and
reconstruction. R20 explicitly ran the additional ignored terminal-retry test
in both disposable sources. All 127 protected inputs verify. Normal services
are healthy, PostgreSQL maintenance is closed, and version 0.2.0 is retained.
At Gate 5 acceptance, Gate 6 was still pending. The subsequent owner revision
below replaces that final repetition requirement.

## Revised Gate 6 and M30 closure — 2026-09-09

The owner explicitly requested less verification and process overhead. Review
of the accepted Gates 1-5, the final 229-Rust/26-Python-test quality pass,
compatibility/protected-input proofs, and completed fixture cleanup closes
Gate 6 and M30 for the existing local scope. This reuses prior results; the
removed 360-row repetition was not run. No new implementation is accepted
without evidence by this process revision.

Normal identity remains username-only, transport is plaintext loopback, and
identity/TLS laboratories are not integrated. Same-host availability and
small-fixture backup/recovery limits remain accepted residuals. Public hosting,
production authentication, off-host backup/PITR, and Protocol V3 remain outside
scope. Continue with a small M31 playable content batch under `AGENTS.md`;
record its scope/result briefly in the durable roadmap.
