# M28 Block 5 — Protocol V2 and gateway audit

Date: 2026-09-07  
Decision: **Gate 5 approved with bounded residual risk**

## Reviewed boundary

This gate reviews only the additive current-Protocol-V2 cooperation surface
and its gateway composition. It does not authorize or claim deterministic role
bots, Godot cooperation presentation, human teamwork, public networking,
Protocol V3, a version change, a release, or a commit.

The implementation keeps `PROTOCOL_VERSION=2` and the 65,536-byte frame
ceiling. It adds exactly four client variants and six server variants from the
accepted contract. All request bodies deny unknown fields; operation IDs are
server-validated as 1–32 ASCII alphanumeric/hyphen bytes; malformed frames and
unknown shapes fail before a session command; and every maximum cooperation
message remains below the private 8 KiB ceiling.

Current V2 canonicalizes all four requests. Frozen V1 rejects them through the
compatibility adapter and its archived source and manifest hashes remain
unchanged. Ordinary V2, mixed capability, solo, both M27 routes, the baseline
door flow, current V1, frozen V1, and reconstruction remain silent about M28
unless a current-V2 participant explicitly opts in.

## Gateway authority and composition

- Capability remains per-live-participant and does not imply peer consent.
- The first admitted participant alone can start; durable append order fixes
  `anchor` then `runner`, and clients supply no role, target, coordinate,
  timing, health, reward, terminal, or peer decision.
- The existing serialized coordinator makes M27 route choice and M28 start
  mutually exclusive. A cooperation commit durably locks the baseline without
  creating a route row or replay kind.
- Start, ping, downing, revive, every timeout/defeat/disconnect failure, and
  success publish only after their persistence/replay transaction succeeds.
- The scripted feedback downing uses the runner's exact current health. The
  revive channel derives source, target, distance, duration, deadline, and
  50-health restoration from server state. A downed participant cannot move,
  attack, equip, mutate modules, choose a route, ping, or revive.
- Ping/revive subdeadlines precede the 60,000 ms operation deadline. Exact
  5,000/15,000/2,000/60,000 ms inclusivity is covered through injected elapsed
  seams.
- Success atomically grants both ordered characters two existing
  `relay_core_fragment` and 125 XP. All five failure families grant no reward,
  write no activity history, and emit no generic completion.
- Reset clears capability, operation clocks, channel/life sidecars, and route
  coordination without changing durable evidence or adding resume/rejoin.

`CooperationLifeState.source_actor_id` is nullable only for the server-owned
`relay_feedback` hazard, which has no actor source. Gateway revive and Warden
projections always provide the authoritative source actor. This makes the
wire evidence honest without giving the client any additional authority or
adding a message variant.

## Closure review corrections

Final review and the first unaccepted Block 6 exploratory run found and
corrected three composition defects before the final gate decision:

1. Generic broadcast previously removed a participant immediately when its
   outbound receiver had closed. A later socket `Disconnect` could then lose
   the immutable role and fail to commit `abandoned_disconnect`. Broadcast now
   leaves admission ownership to the serialized disconnect command. The new
   regression proves that a failed broadcast retains the role, the late
   disconnect commits exactly one abandonment, and the writable peer receives
   the typed no-reward summary.
2. A late capability rejection previously returned an empty capable set even
   when the peer had already opted in. It now projects the actual current
   capable actor set while still rejecting the late requester, reserving no
   operation, and exposing no pre-start role assignment.
3. A same-ID ping retry observed later than the original acceptance rebuilt
   replay payload timing from the retry observation. That disagreed with the
   already-durable event and aborted the session. A replayed ping now rebuilds
   the payload only from the durable accepted operation ID and elapsed time;
   a PostgreSQL regression applies at 200 ms and replays the same ping at
   250 ms without extending its lifetime or changing replay evidence.

After the third correction, all 15 real PostgreSQL cooperation/replay tests
and all 39 gateway tests passed. Format and Clippy with warnings denied are
green. Block 6 evidence remained unaccepted until this complete Gate 5
revalidation finished.

## Real two-client evidence

The local-only live flow used two separately opted current-V2 clients and
session `session-1788743676421160150`. Durable admission fixed actor 1 as
anchor and actor 2 as runner. The accepted sequence was:

- start at durable elapsed 0;
- anchor arrival at 86 ms;
- fixed `relay_console` ping at 182 ms;
- runner arrival and exact 100-to-0 feedback downing at 278 ms;
- revive start at 334 ms and completion at 2,530 ms;
- Warden completion and cooperation success at 5,213 ms.

SQL contains one cooperation operation, exactly two immutable participants,
zero route operations, two activity-history rows, two fragment grants, two XP
grants, and one replay event for each accepted cooperation transition. Replay,
CLI, Inspector event projection, Inspector summary, and SQL agree. Inspector
returned `200` only for GET; HEAD, POST, PUT, PATCH, DELETE, and OPTIONS each
returned `405`. No public failure-injection trigger or function remained.

The first live success run preceded the three closure corrections above. The
failed-broadcast and late-capability fixes do not alter that success path, and
the ping issue requires a later same-ID observation that run did not perform.
The final source is covered by the focused regressions, the real PostgreSQL
suites, a later bot success that performs the retry, and the uninterrupted
complete quality gate.

## Complete quality and protected boundaries

The final post-retry-fix `make check` passed version validation, formatter,
workspace Clippy with `-D warnings`, 217 tests, all-target builds, Inspector
typecheck and production build, secret audit, multiplayer smoke, current Godot
M17–M27 validation, current V1, frozen V1, and V1 reconstruction.

Final invariant inspection records:

- branch `main`, HEAD and `origin/main`
  `4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`;
- `VERSION=0.2.0`;
- `git diff --check` green;
- M27 checkpoint SHA-256
  `81bb1fafaa397b27db7fc505767c09202d68b77e13355952b49ad9d994280d70`;
- archived V1, Godot file boundary, and fake client equal to that checkpoint;
- frozen V1 source hashes
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`
  and
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`;
- healthy PostgreSQL, gateway, and Inspector listeners only on loopback;
- zero public non-internal trigger and zero public function; and
- no retained validation process.

The earlier `protected-boundaries.txt` reported a Godot directory mismatch
caused only by four empty editor-created directories. They were removed after
exact target inspection. `final-invariants.txt` supersedes that directory-only
result and proves the complete Godot file boundary equal.

Accepted evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block5-protocol-20260906`.
Its final `SHA256SUMS` file has SHA-256
`bf2d3206b0a807655f84aa7db41ef6bd3c3b4af7fb7579c33034516d681cce79`.
The final post-retry-fix gate log has SHA-256
`53be466cdd62f9effa974a96e9aaa4a1d8322f429d446766c24b7e1dcb335241`;
the focused real-PostgreSQL and gateway logs have SHA-256
`ed81ac3aec1183e58084db0b0b5cdb37ddc674b7ec24ee11ad4d0d46a503b74d`
and
`7cb57d4869d89f4827fad11d7b4a960de024bbfc0c05c273845310c0c5e6d79a`.

## Gate decision and residuals

Gate 5 is green. No reproducible protocol-size, parsing, authority, timing,
transaction, projection, route-exclusion, reward, Inspector, compatibility,
privacy, reset, or disconnect blocker remains in Block 5 scope.

Bounded residuals are the already-accepted loss of an in-progress operation on
process failure, no resume/rejoin, runtime observation occurring only when a
command is received, local-only evidence on the current host, and the absence
of a cooperation-aware durable client harness or presentation. The last item
is the explicit subject of Blocks 6 and 7, not evidence manufactured by this
gate.

Block 6 alone is now authorized to add deterministic anchor/runner automation
and its bounded positive, negative, timing, failure, compatibility, and reset
fixtures. Godot cooperation presentation remains locked until Gate 6 is green.
