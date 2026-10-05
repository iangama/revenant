# M30 Gate 2 — identity, session, lobby, and invite laboratory audit

Date: 2026-09-08  
Decision: **Gate 2 approved; isolated laboratory retained**  
Accepted evidence:
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate2-identity-session-20260908b`

The accepted `SHA256SUMS` covers 18 evidence payload files totaling 105,137
bytes. Its SHA-256 is
`2a66740a0118cd7563c9516a850bf037f5d3b5d628fe2e1c78bec295d14f90ca`,
and every entry verifies.

## Reviewed boundary

Gate 2 adds one workspace package, `revenant-local-security-lab`, with four
source files. All implementation and its binary require the explicit
`m30-identity-session-lab` feature, which is disabled by default. The package
uses `getrandom` for 32-byte operating-system randomness, `sha2` for
domain-separated one-way verifiers, `subtle` for constant-time comparison, and
Serde only for deterministic redacted reports.

Before implementation, the complete Gate 1 checkpoint was verified against
the current repository:

- path:
  `/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m30-gate1-45718926-20260907.tar.gz`;
- 379 source/archive entries with zero path or content differences;
- 10,879,958 bytes; and
- SHA-256:
  `2a4b39b5602e073e18f446fc44cd6773722adb2559c2e5057598d717ef7ec9c1`.

The checkpoint includes the final-video interlude and Gate 1 specification.
Only generated Godot imports and the pre-existing accidental `NUL` path were
excluded, as recorded by its external file list.

## Pure state model

The retained model contains at most two canonical accounts, one stored session
per account, one lobby with two seats, one one-time invite, and one five-entry
failure window per known account plus one shared unknown-principal bucket.
There is no network, filesystem state, wall clock, database, protocol, replay,
Inspector, Godot, or normal-service dependency.

Security-sensitive values are fixed at 32 bytes. Production-shaped generation
uses the operating-system CSPRNG. The deterministic matrix injects synthetic
fixtures, but reports expose only protected-state digests and categorical
results. Raw tokens and verifier material are never serialized. Debug output
prints only a redacted size marker.

The model proves:

- exact username validation and lowercase canonical identity, including
  collision, replay, and conflicting enrollment;
- outwardly identical wrong-credential and unknown-account failures;
- five evaluated failures in a rolling 60,000 ms window, sixth-attempt
  throttling, exact expiry, and a single bounded unknown-principal bucket;
- credential proof only for an account already admitted to the named lobby;
- one active stored session per account, with replacement of the previous
  session, exact 600,000 ms idle and 1,800,000 ms absolute boundaries, logout,
  credential rotation, and saved-code recovery revocation;
- one owner-created two-seat lobby, inclusive 600,000 ms invite lifetime,
  one-time redemption, and server-owned leave/join presence; and
- an actual two-thread redemption race that commits exactly one peer.

One Gate 1 wording ambiguity was clarified before acceptance. Failed proofs
must update rate-limit metadata while preserving account, credential, lobby,
membership, and session state. Reports therefore count protected mutation and
bounded failure metadata separately. This narrows the mutation definition; it
does not remove or weaken a vector.

## Exact matrix result

Both independent invocations emitted schema
`revenant.m30.identity-session-gate.v1` and exactly `I01` through `I30` in
order. The JSON files are byte-identical:

- report size: 20,607 bytes;
- report SHA-256:
  `71bd75a1329ffd5473e0f9ef615b18022888c5e81d1145bc3e178d7544494407`;
- passing cases: 30/30;
- failed redaction checks: zero;
- total protected transitions across independent cases: 19;
- stable protected-state cases: 13; and
- maximum accounts/sessions/lobby members/retained failures: `2/1/2/5`.

All five package tests pass. They independently verify exact ordered case
coverage, deterministic/redacted serialization, fixed CSPRNG output size and
redacted debug behavior, unchanged protected digests for every applicable
rejection, and a bounded five-entry shared unknown-principal bucket after 100
distinct unknown names.

The uninterrupted canonical `make check` also passes with 217 default Rust
tests, strict all-feature Clippy, workspace builds, Inspector TypeScript/build,
the 398-file repository secret audit, PostgreSQL/Gateway smoke behavior, Godot
4.7.1 validation, and frozen-V1 compatibility. The five feature-gated Gate 2
tests are recorded separately and are not miscounted as part of the default
217-test baseline.

## Isolation and protected boundaries

A fresh default package build emits no laboratory binary. Cargo metadata shows
an empty default feature set and the binary's exact required feature. The
ordinary release Gateway contains zero M30/laboratory symbols under static
string inspection.

The Gate 1 hashes remain byte-identical for Compose, current identity,
Protocol V2, Gateway, Godot project settings, VERSION, and both frozen V1
files. The live services remain healthy. PostgreSQL still has sixteen public
tables and zero M30-named object. No normal client, service, migration,
credential, account, lobby, session, or working-database row was created.

The report writer uses create-new semantics. An attempted write to the accepted
existing path fails instead of overwriting evidence. The repository secret
audit passes, and explicit report scans find no synthetic credential/recovery
fixture value or serialized 32-byte array.

## Rejected candidate and review correction

`m30-gate2-identity-session-20260908a` is rejected. Its 30 vectors passed, but
review found that reauthentication revoked and retained every historical
session. The finite matrix observed two, yet repeated valid authentications
could grow storage without bound. The implementation was changed to replace
the account's stored session, enforce at most two accounts, and share unknown
identities in one bounded rate bucket. Candidate `...20260908b` was generated
from that corrected model and is the only accepted evidence.

The rejected directory also contains a superseded partial release-build log;
its cached continuation completed, but neither file is used by the accepted
decision.

## Gate decision and residuals

Gate 2 is green and the pure laboratory is retained. It meets every frozen
`I01`–`I30` requirement twice, is deterministic, bounded, redacted, concurrent
where required, default-disabled, and absent from protected runtime surfaces.

This does not fix the normal game's username-only identity. The lab has no
durability, game-wire proof, TLS, real secret provisioning, abuse/resource
limits, or recovery. Normal V1/V2 must continue to be described as local
development identity; integrating this model would require the separately
reviewed Protocol V3 decision that M30 explicitly rejected.

Only Gate 3 transport/secrets may begin next. It must first make and verify a
pre-change custom-format database backup plus disposable restore, then execute
only `T01`–`T30`. Gate 4 and all M31 content remain locked.
