# M30 Gate 4 — abuse and observability audit

Date: 2026-09-08 (creator timezone)  
Decision: **Gate 4 approved; bounded local hardening retained**  
Accepted evidence:
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate4-abuse-20260908b`

The sealed `SHA256SUMS` covers 54 payload files totaling 411,357 bytes. Its
SHA-256 is
`26ce09668ac6894e7a09e468f70c8df85030dfb3a6f68cc0a349b89123491b38`;
all entries verify.

## Contract and implementation

The pre-implementation M28 success baseline and exact ceilings are frozen in
`docs/security/m30-gate4-abuse-observability-contract.md`. The measured largest
synchronous participant batch is eight messages, against a retained queue of
32. No numeric ceiling was relaxed to pass validation.

The Gateway retains 65,536-byte frames, 64 global connections, a 15-second
handshake, and a five-minute gameplay idle limit. It now bounds unauthenticated
connections at eight, decoded frames at 32/second and 600/minute, coordinator
input at 64 commands, and each participant's outbound queue at 32 messages.
The first rate excess closes only the offender before domain dispatch. Queue
excess disconnects the shared sender without blocking domain execution.
Connection accounting includes writer lifetime, and every admitted
participant exit submits the existing disconnect transition.

A total monotonic read budget prevents trickled bytes from restarting the
frame/header deadline. Handshake database work is serialized briefly and
connections are released before network waits. Database initialization occurs
only after the next valid request. Movement coordinate comparisons also avoid
signed-minimum overflow while preserving the existing -12..12 bounds.

Gateway diagnostics use fixed JSON event/category fields and fresh random
128-bit correlation digests. Supplied identity/build text, replay session IDs,
request paths, bearer/recovery canaries, database URLs, and arbitrary error
text are not interpolated into Gateway logs. Internal HTTP errors return only
the generic error with the permitted origin and `no-store` where appropriate.

Inspector API reads require the exact configured IPv4-loopback origin and
emit `Cache-Control: no-store`; methods other than GET remain 405. Foreign,
missing, or empty Origin values are 403; malformed/duplicate headers are 400.
Request lines, individual headers, header count, and aggregate header bytes
are bounded. Nginx and the loopback-only Vite proxy supply a derived Origin
only for an absent Origin plus browser `Sec-Fetch-Site: same-origin`, and the
Gateway still exact-compares it. Request logging is disabled in Nginx.

All five Compose services have the exact reviewed memory/PID caps and Docker
`local` logging with five 2m segments and compression disabled. The normal
configuration validator rejects altered/removed caps, enlarged or changed
logging, and wildcard Inspector origin before startup.

## Repeated evidence

Fresh tmpfs-backed projects `m30g4b` and `m30g4c` each passed exactly A01–A30:

- report A: 30/30; report B: 30/30; combined: 60/60;
- normalized report bytes: identical;
- undeclared protected-state changes: zero;
- redaction failures: zero;
- working database changes during either matrix: zero; and
- OOM kills or failed migration/provisioner exits: zero.

Each rejection compares the exact raw contents of all sixteen fixture tables
and both sequences. Only A21's normal one-player waiting admission is setup,
captured before its rejection baseline. Separate-fixture report digests
canonicalize generated session IDs and timestamps; report comparison removes
only durations and resource samples. The working database has a separate
whole-run fingerprint, not that canonicalized fixture identity.

The maximum sampled memory/PID observations across both runs were:

| Service | Memory bytes | PIDs | Enforced memory / PIDs |
| --- | ---: | ---: | --- |
| Gateway | 5,398,528 | 68 | 128 MiB / 192 |
| PostgreSQL | 79,982,592 | 9 | 256 MiB / 64 |
| Inspector | 14,327,808 | 20 | 64 MiB / 32 |

Samples include the held 64-connection boundary. Migration and provisioner
each retain 128 MiB / 32 PIDs and exit 0. These are local samples, not a claim
of continuous worst-case profiling.

Each fresh fixture then completed the M28 two-role cooperation success with
one revive, two reward participants, two loot grants, two progression grants,
and matching reconstructed Inspector truth. Real Chromium proved that the
Inspector page's normal GET omits Origin yet succeeds with `no-store`, while
a foreign-origin page cannot read its response. Both disposable projects and
their socket volumes were removed; working data was never a teardown target.

## Canonical and normal-stack proof

The uninterrupted `make check` passed version consistency, formatting, strict
all-feature Clippy, 229 Rust tests including PostgreSQL integration, workspace
build, Inspector TypeScript/build, the 416-file secret audit, gameplay and
multiplayer smoke, canonical Godot 4.7.1/M17–M28 validation, frozen V1, and
isolated V1 reconstruction. All 48 Gateway unit tests also pass separately.

The normal image/configuration update preserved its pre/post working-data
fingerprint before the declared canonical smoke fixtures. Maintenance was
closed by recreating the full stack in dependency order. Final normal-image
V2 and frozen V1 flows pass, an unsupported generation is refused, and the
real-browser proof passes on port 4173. PostgreSQL, Gateway, and Inspector are
healthy; both one-shot jobs exit 0; only 7000/8080/4173 are published on IPv4
loopback, with 5432 and the disposable test ports absent.

Gateway runs effectively as UID 100 with its staged secret at 0400. PostgreSQL
remains the pinned 16.15 image, the runtime role remains nonprivileged, and
there are still sixteen public tables. Current/previous secret scans of the
repository, accepted evidence, and exported Gateway/Inspector root filesystems
find no live secret material. Frozen V1 hashes and version 0.2.0 remain exact.
All 127 protected gameplay, Godot, activity, protocol, compatibility, archive,
and version inputs are byte-identical to the checksum-verified Gate 1 source
checkpoint; the final documentation-inclusive secret audit passes 417 files.
No commit, push, version, tag, release, public listener, or Protocol V3 change
was made.

## Review history and residuals

The preliminary `...20260908a` directory is superseded, not silently rewritten.
It retains an initial clean-build failure from missing declared Node types,
an unsuccessful ad-hoc browser invocation, and a preliminary passing matrix.
Declaring the development-only Node types fixed the clean build; the final
build and browser proofs passed before both accepted fresh-fixture matrices.
The implementation review addendum explicitly records the browser Origin,
database lifetime, monotonic deadline, and coordinator backpressure corrections.

Normal V1/V2 identity is still username-only and transport remains plaintext
loopback; the identity and TLS laboratories are not integrated. Native local
processes can spoof Origin. Exact long timeout and slow-reader queue boundary
cases exercise shared production code with controlled clocks/receivers, not
five-minute wall-clock repetitions or saturated OS socket-buffer claims.
Serial HTTP handling and finite admission bound resources but do not promise
availability against a persistent hostile same-host process. Resource and
browser results apply to this creator host, not untested hardware or people.

Gate 4 is green within those bounds. The only next boundary is Gate 5's
checked-absent disposable backup/fault/recovery contract and R01–R30 proof.
Gate M30 and M31 remain closed pending Gate 5 and the 360-row closure.
