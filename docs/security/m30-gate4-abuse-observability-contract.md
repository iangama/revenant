# M30 Gate 4 — frozen abuse and observability implementation contract

Date: 2026-09-08  
Status: **frozen before Gate 4 runtime edits; Gate 4 now approved** under
`docs/security/m30-gate4-abuse-observability-audit.md`.

Implementation review addendum (2026-09-08, before live acceptance): the
original draft missed same-origin browser GET behavior, retained database
connections during socket waits, and coordinator input backpressure. The
review retains every numeric ceiling above/below and adds a 64-command
coordinator queue (one queued command per global connection, plus at most one
blocked producer per connection). Browser GETs normally omit Origin; Nginx
may derive it from the request's scheme/Host only when Origin is absent and
the browser's Sec-Fetch-Site is exactly same-origin. Gateway still compares
the resulting value to its exact configured origin. Missing Origin without
that browser metadata, cross-site requests, and foreign Host values fail
closed. Nginx request/access logging is disabled to avoid retaining URLs.
Duplicate Origin fields are malformed HTTP (400); a single disallowed or
empty Origin is an authorization rejection (403).

Handshake/database initialization now happens only after the next valid
protocol request, and database handles are released before network waits.
Every admitted participant submits disconnect on every exit path. Connection
slots include outstanding writer lifetime. A monotonic deadline reader shares
the tested inclusive comparison and bounds a whole frame/HTTP header read,
including trickled bytes, using the original 15-second/five-minute/five-second
budgets. These corrections address implementation gaps, not relaxed caps.
The same-origin behavior also applies to the loopback-only Vite development
proxy. Movement-boundary validation uses comparisons instead of signed `abs`
so `i32::MIN` cannot panic or bypass the existing -12..12 coordinate bounds.
The browser rationale follows the [Fetch Origin rules](https://fetch.spec.whatwg.org/#origin-header).

For repeatable evidence, each final run starts a fresh, named `m30g4` Compose
project with tmpfs PostgreSQL data. Its only setup mutation is the A21 normal
one-player waiting admission. Case fingerprints cover all sixteen tables and
both sequences; exact raw fingerprints must match within each rejection case.
Report fingerprints normalize only generated replay session IDs and event
timestamps so separate fresh fixtures compare. The separate working database
is fingerprinted before/after each complete run and must remain unchanged.

## Preconditions and protected boundary

M30 Gates 1–3 are green. The Gate 3 infrastructure, working database, normal
loopback topology, exact secret grants/roles, Protocol V2, frozen V1, Godot,
gameplay timing/content, version 0.2.0, and all accepted evidence remain
protected. Gate 4 adds no account protocol, Protocol V3, normal TLS, public or
LAN listener, mutation API, gameplay rule, database object, or external
service.

The pre-implementation baseline is sealed outside the repository at
`/mnt/c/Users/Ian/revenant-local-evidence/m30-gate4-baseline-20260908a`.
Its eight-entry `SHA256SUMS` has SHA-256
`a771166ad29450f14691557c4051bce69216672c1afb4426f96ed4813a3cda96`
and verifies. A repeated two-client M28 success emitted no failure. The
accepted source-built M28 Gateway peaked at 11,756 KiB RSS; the repeated
Compose run peaked at 1.160 MiB/10 PIDs for Gateway, 44.630 MiB/10 PIDs for
PostgreSQL, and 10.020 MiB/13 PIDs for Inspector.

## Frozen framing, admission, and time boundaries

- The protocol payload ceiling remains exactly 65,536 bytes. A complete valid
  frame of that size reaches decoding; a 65,537-byte announcement is rejected
  before payload allocation.
- The existing global game-connection maximum remains 64. The 64th is
  admitted; the 65th is closed without affecting established connections.
- At most eight connections may be awaiting successful local authentication.
  The eighth is bounded by the existing 15-second handshake timeout; the ninth
  is closed. The unauthenticated slot is released exactly once after a valid
  `AuthRequest` is accepted or when the connection closes.
- Handshake reads retain the 15-second OS socket timeout. The exact deadline is
  inclusive in the pure boundary model; an incomplete prefix or body observed
  strictly after it closes. Gameplay idle remains five minutes with the same
  inclusive/strict-plus-one model.
- Every successfully decoded client frame from `ClientHello` onward is
  admitted by one per-connection rolling limiter. Timestamps whose age is
  greater than or equal to the window are evicted. The first 32 frames in any
  one-second window and first 600 in any 60-second window are accepted. The
  33rd or 601st retained frame closes only that connection before its domain
  command is sent. A frame at exactly the expired timestamp opens capacity.
- Malformed MessagePack, wrong message order, unsupported version, incomplete
  frame, oversize frame, timeout, and rate excess never mutate authoritative
  state. Existing supported-version and order responses remain unchanged.

Exact long-duration comparisons use an injected monotonic-clock unit model
that shares the live constants and comparison functions. Live socket cases
cover framing, malformed/order/version behavior, admission, rate, isolation,
and bounded closure; the matrix does not weaken normal timeouts through an
environment override.

## Frozen outbound queue

Static enumeration of the retained M28 success finds an exact maximum
synchronous batch of eight messages per participant at activity start:
`ActivityStart`, initial `ObjectiveUpdate`, two player `ActorSpawn` messages,
one Relay Drone `ActorSpawn`, two chase `ActorUpdate` messages, and one initial
`DamageApplied`. The cooperation terminal emits seven. The baseline records
the source hashes and derivation.

Every admitted participant receives one nonblocking bounded queue of exactly
32 `ServerMessage` values: four times the observed maximum and below the
milestone ceiling of 256. A full or disconnected queue atomically closes its
sender. The writer drains only while socket writes succeed, retains the
existing 15-second write timeout, and shuts down the socket on queue close or
write failure. Domain execution never blocks on a slow reader; the connection
handler then submits the ordinary disconnect transition. Tests prove the
32nd enqueue, first excess close, and unaffected peer.

## Structured and redacted events

Gateway output remains newline-delimited JSON and uses only code-defined event
names and categorical fields. Existing fixed gameplay events may retain
bounded numeric actor/damage/position facts. Security/operational events are
limited to:

- `gateway_started`, `database_migration_complete`, and the feature-gated
  `m27_matrix_seed_injection_enabled`;
- `connection_admitted`, `connection_rejected`, `connection_closed`,
  `protocol_negotiated`, `authentication_result`, and `frame_rate_rejected`;
- `outbound_queue_closed`, `http_request_rejected`,
  `session_persistence_reconnected`, `session_command_failed`,
  `session_aborted_after_command_failure`, and `session_reset`; and
- the existing `attack_applied`, `enemy_attack_applied`, and
  `movement_applied` gameplay events.

Each accepted socket gets a fresh operating-system-random 128-bit lowercase
hex correlation digest. Connection/protocol/authentication/rate/close events
carry that digest; no peer address is logged. Failure fields are finite enum
categories such as `global_limit`, `unauthenticated_limit`, `timeout`,
`frame_too_large`, `invalid_message`, `unexpected_message`, `rate_second`,
`rate_minute`, `persistence`, `queue_full`, and `internal`. Arbitrary error
text or supplied input is never interpolated.

No log contains a raw username, account/character ID, client name/build,
session/recovery/bearer value, invite, database URL/password, private key,
request path/query/body, MessagePack body, stack dump, or environment value.
The former raw replay session ID is removed from `session_reset`. Gate 4 does
not add durable login sessions; synthetic bearer/recovery fixtures therefore
exercise redaction at input and diagnostic boundaries without integrating the
Gate 2 laboratory into Protocol V2.

## Inspector HTTP boundary

- The configured Inspector origin defaults to exactly
  `http://127.0.0.1:4173`. Compose renders the selected loopback Inspector port
  into `REVENANT_INSPECTOR_ORIGIN`. Startup rejects schemes other than `http`,
  hostnames, IPv6, credentials, paths, queries, fragments, port zero, and any
  host other than literal `127.0.0.1`.
- The request line remains at most 4,096 bytes including its newline. Header
  parsing accepts at most 32 lines and 16,384 total bytes, with at most one
  case-insensitive `Origin` field. All reads retain the five-second HTTP
  timeout.
- `/health` remains GET-accessible without an Origin for container health.
  Every `/api/inspector` GET requires the exact configured Origin; missing,
  empty, wildcard, `null`, hostname, IPv6, LAN, public, wrong scheme, wrong
  port, or path-bearing values receive 403 without an
  `Access-Control-Allow-Origin` header. Duplicate Origin fields or malformed
  HTTP headers receive 400 before route handling.
- An allowed response emits exactly that origin, `Vary: Origin`, and
  `Cache-Control: no-store`. All JSON responses emit `Cache-Control: no-store`.
  No wildcard or reflected input is emitted.
- Every non-GET method remains 405 before route/database work. Invalid or
  unsafe canonical paths remain 404. Malformed/oversized HTTP input receives a
  bounded generic 400 or closes on timeout. Database/internal failures receive
  `500` with only `{"error":"internal_error"}`; supplied/error text is not
  returned or logged.

The Nginx Inspector continues to proxy only read-only `/api/` requests and
passes the browser Origin. No cookie, WebSocket, write route, or new response
field is added.

## Compose resource and log ceilings

Every service uses Docker's `local` log driver with `max-size=2m`,
`max-file=5`, and compression disabled. Exact cgroup ceilings are:

| Service | Memory | PIDs | Basis |
| --- | ---: | ---: | --- |
| PostgreSQL | 256 MiB | 64 | More than 5.7× the measured 44.630 MiB and 6.4× 10 PIDs |
| migration | 128 MiB | 32 | Bounded no-listener Gateway-image one-shot |
| provisioner | 128 MiB | 32 | Bounded PostgreSQL-client-image one-shot |
| Gateway | 128 MiB | 192 | More than 10× the accepted 11.756 MiB peak; covers three base plus two tasks for 64 connections |
| Inspector | 64 MiB | 32 | More than 6× the measured 10.020 MiB and 2× 13 PIDs |

The matrix samples memory/PIDs under the full admission/rate/slow-reader set
and proves every service stays healthy within its cap. A cap kill, legitimate
M28 timing regression, or need to raise a ceiling rejects Gate 4 for review;
the matrix never silently loosens it.

## Exact `A01`–`A30` execution mapping

Each report uses schema `revenant.m30.abuse-observability-gate.v1` and contains
exactly 30 ordered rows with case ID, layer, input category,
expected/observed disposition, pre/post protected-state digest, protected
mutation count, redaction result, duration, resource sample, and pass boolean.

| IDs | Exact execution |
| --- | --- |
| A01–A02 | Exact 65,536-byte valid frame reaches decode; 65,537-byte announcement closes before allocation |
| A03–A04 | Incomplete length prefix and incomplete declared body close at the frozen handshake boundary |
| A05–A07 | Invalid MessagePack, wrong first message, and unsupported version reject without protected mutation |
| A08–A11 | Exact/plus-one HTTP line, all non-GET methods, and invalid/unsafe Inspector paths |
| A12–A15 | 64th/65th global and eighth/ninth unauthenticated connection boundaries with established peers retained |
| A16–A19 | Exact/plus-one handshake and exact/plus-one gameplay-idle monotonic comparisons |
| A20–A22 | Exact rate ceilings, first excess offender-only close, and fresh acceptance at the expired timestamp |
| A23–A24 | Exact queue capacity/slow-reader isolation and live cgroup memory/PID evidence |
| A25–A28 | Correlation lifecycle, bearer/recovery and database/error redaction, and exact log-driver rotation settings |
| A29–A30 | Allowed exact origin plus no-store, then missing/disallowed origins and every non-GET method fail closed |

The matrix runs twice in new external evidence files. Stable categorical rows
must be byte-identical after removing only durations, resource samples, fresh
correlation digests, and intentionally different synthetic identities. Only a
case whose contract explicitly establishes a fixture may change protected
state; abuse/rejection cases require identical pre/post digests.

## Stop and rollback boundary

Any V1/V2 or gameplay regression, normal non-loopback listener, Protocol V3,
normal TLS requirement, unbounded queue/thread/log growth, raw sensitive/input
material in output, reflected/wildcard CORS, missing no-store, Inspector write,
working-data damage, cap below a legitimate peak, or failure to isolate the
offender rejects Gate 4. Rollback removes only Gate 4 runtime/config/test
changes and restores the accepted Gate 3 source and normal services. Gate 5
remains locked until two exact matrices and the canonical quality gate pass.
