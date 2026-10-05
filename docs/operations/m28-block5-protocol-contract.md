# M28 Block 5 — additive Protocol V2 and gateway contract

Date: 2026-09-06  
Review state: **accepted before Block 5 Protocol/gameplay source edits**  
Scope: explicit cooperation capability, bounded intents/results, immutable
roles, authoritative ping/down/revive/life/timer projection, route exclusion,
transactional terminal truth, and ordinary-V2/frozen-V1 compatibility.

## Additive wire vocabulary

`PROTOCOL_VERSION` remains 2 and the existing 65,536-byte frame ceiling is
unchanged. Block 5 adds exactly four `ClientMessage` variants:

| Variant | Exact client authority |
| --- | --- |
| `CooperationStateRequest(CooperationStateRequest {})` | Explicitly opts the already-admitted current-V2 participant into M28 capability and requests current server truth. |
| `CooperationStartIntent(CooperationStartIntent)` | Requests the one leader-owned cooperation start using only an operation ID. |
| `CooperationPingIntent(CooperationPingIntent)` | Requests the one fixed `relay_console` ping using only an operation ID. |
| `CooperationReviveIntent(CooperationReviveIntent)` | Requests/observes the one server-fixed anchor-to-runner revive using only an operation ID. |

It adds exactly six `ServerMessage` variants:

| Variant | Purpose |
| --- | --- |
| `CooperationState(CooperationState)` | Capability, eligibility, phase, participant/capability, immutable operation, contribution, and last-observed timer truth. |
| `CooperationStartResult(CooperationStartResult)` | Targeted rejection or committed/replayed start with the complete immutable operation snapshot. |
| `CooperationPingResult(CooperationPingResult)` | Targeted rejection/retry or shared committed fixed-target ping evidence. |
| `CooperationLifeState(CooperationLifeState)` | Shared committed downed, revived, or defeated health/life transition. |
| `CooperationReviveResult(CooperationReviveResult)` | Targeted rejection/pending/retry or shared started/cancelled/completed channel evidence. |
| `CooperationOperationSummary(CooperationOperationSummary)` | Shared persisted success/failure, exact final participants/contributions, grants, or typed no-reward reason. |

Negotiating V2, using a current build name, authenticating, joining, moving,
or using M26/M27 never implies M28 capability. An ordinary V2 participant
that never sends `CooperationStateRequest` receives none of the six variants.
Frozen V1 cannot canonicalize any of the four requests and receives no M28
variant. No archived V1 byte changes.

## Wire constants and parsing bounds

The Protocol crate exports:

```text
COOPERATION_WIRE_SCHEMA_VERSION = 1
MAX_COOPERATION_OPERATION_ID_BYTES = 32
MAX_COOPERATION_MESSAGE_BYTES = 128
MAX_COOPERATION_PARTICIPANTS = 2
MAX_COOPERATION_TARGETS = 2
MAX_COOPERATION_GRANTS = 2
MAX_COOPERATION_WIRE_PAYLOAD_SIZE = 8 * 1024
```

Every new struct denies unknown fields. Every new enum uses exact snake-case
serialization. All four client operation IDs use 1-32 ASCII alphanumeric or
hyphen bytes. No client M28 request contains a role, actor/participant target,
coordinate, text, icon, color, duration, elapsed time, distance, health,
damage, life state, revive amount, contribution, participant list, capability
for a peer, reward, terminal, replay event, route state, or peer decision.

Gateway output validation retains existing bounded session/activity/catalog/
item identifiers, exactly two post-start participants and targets, zero or two
success grants, and at most 128 UTF-8 bytes of display/result text. Every
maximum-shaped new client/server variant must serialize below the private 8
KiB M28 ceiling and therefore below the unchanged frame ceiling.

Unknown variants, unknown top-level/nested fields, invalid enums, wrong
MessagePack shapes, and oversized frames fail during decoding before a session
command. Decoded but invalid operation IDs receive a bounded targeted
rejection with an empty returned operation ID and reserve/mutate nothing.

## Exact client structs

```text
CooperationStateRequest {}

CooperationStartIntent {
  operation_id
}

CooperationPingIntent {
  operation_id
}

CooperationReviveIntent {
  operation_id
}
```

The ping target is always the server-owned `relay_console`. Revive source and
target are always the immutable anchor and runner. Squared distance comes only
from current authoritative actor positions. A request cannot select or
impersonate either role.

## Shared state vocabulary

```text
CooperationPhase =
  waiting | drone | eligible | awaiting_anchor | awaiting_ping |
  awaiting_runner | runner_downed | revive_channel | encounter_active |
  succeeded | failed | unavailable

CooperationRole = anchor | runner
CooperationTarget = relay_anchor | relay_console
CooperationLife = active | downed | defeated
CooperationTerminalOutcome =
  succeeded | failed_ping_timeout | failed_revive_timeout |
  failed_operation_timeout | failed_participant_defeated |
  abandoned_disconnect

CooperationParticipantState {
  actor_id,
  role,
  current_health,
  max_health,
  life
}

CooperationContributions {
  anchor_arrived,
  pinged,
  runner_arrived,
  revived,
  warden_completed,
  revive_count
}

CooperationTargetState {
  target,
  position
}

CooperationTiming {
  operation_duration_ms,
  ping_ttl_ms,
  revive_window_ms,
  revive_channel_ms,
  revive_health,
  maximum_distance_squared
}

CooperationReward {
  item_id,
  item_quantity,
  experience
}
```

The targets are exactly `relay_anchor [3,0,3]` then
`relay_console [4,0,3]`. Timing is exactly
60,000/5,000/15,000/2,000 ms, 50 health, and squared distance four. Reward is
exactly two `relay_core_fragment` and 125 XP per participant. No request or
current mutable catalog can replace these values.

Before durable start, role-bearing `participants` and the operation snapshot
are absent. `participant_actor_ids` and `capable_actor_ids` disclose only the
current admission/capability set. This preserves the rule that complete role
assignment is exposed only after start commits.

## Exact operation, state, and start shapes

```text
CooperationPingState {
  operation_id,
  source_actor_id,
  target: relay_console,
  accepted_elapsed_ms,
  expires_elapsed_ms,
  active
}

CooperationReviveStatus =
  rejected | started | replayed | pending | cancelled | completed

CooperationReviveState {
  operation_id,
  source_actor_id,
  target_actor_id,
  started_elapsed_ms,
  observed_elapsed_ms,
  required_duration_ms,
  maximum_distance_squared,
  distance_squared,
  status
}

CooperationOperationState {
  catalog_revision,
  start_operation_id,
  phase,
  participants: [CooperationParticipantState; 2],
  targets: [CooperationTargetState; 2],
  timing: CooperationTiming,
  reward: CooperationReward,
  contributions: CooperationContributions,
  observed_elapsed_ms,
  ping: CooperationPingState | null,
  revive: CooperationReviveState | null,
  terminal_outcome: CooperationTerminalOutcome | null
}

CooperationState {
  schema_version,
  accepted,
  message,
  session_id,
  activity_id,
  phase,
  participant_actor_ids,
  capable_actor_ids,
  all_capable,
  operation: CooperationOperationState | null
}

CooperationStartResult {
  schema_version,
  accepted,
  replayed,
  message,
  session_id,
  operation_id,
  operation: CooperationOperationState | null
}
```

`waiting` means the expected pair is not yet admitted; `drone` is the existing
opening encounter; `eligible` is the post-Drone choice window;
`unavailable` means solo/mixed generation, insufficient capability, baseline
lock, accepted M27 route, terminal/reset, or another ineligible state. The
remaining values map the pure M28 phases exactly.

A first eligible state request records only that actor's session-local
capability. A new capability change sends current state only to opted actors;
an idempotent refresh responds to its requester. A late first request is a
targeted `accepted=false`, `phase=unavailable`, `operation=null` response and
does not opt the participant in.

Start is accepted only after Drone completion, with exactly two current-V2,
separately opted, still-admitted participants, from the first-admitted leader,
while the M27 route choice remains open. The gateway derives ordered roles,
profiles, health, and all constants. It first validates a cloned route lock,
then commits Gate-4 start persistence/replay, then publishes in-memory/wire
truth. A newly accepted start result is broadcast identically to both opted
participants; exact retry is targeted with `replayed=true`. Rejection has
`operation=null`. No pre-start location is credited retroactively.

## Ping, life, and revive shapes

```text
CooperationPingResult {
  schema_version,
  accepted,
  replayed,
  message,
  session_id,
  operation_id,
  ping: CooperationPingState | null
}

CooperationLifeCause = relay_feedback | revive | warden

CooperationLifeState {
  schema_version,
  session_id,
  actor_id,
  role,
  source_actor_id,
  cause,
  elapsed_ms,
  health_before,
  health_after,
  max_health,
  life_before,
  life_after
}

CooperationReviveResult {
  schema_version,
  accepted,
  replayed,
  message,
  session_id,
  operation_id,
  status,
  revive: CooperationReviveState | null
}
```

Only the anchor can ping, only in `awaiting_ping`. New accepted ping evidence
is broadcast after the persistence/replay commit; exact retry is targeted;
rejection has `ping=null`. It exposes the fixed source/target and authoritative
acceptance/expiry durations but creates no client-owned timer. Expiry produces
the terminal summary rather than a fabricated ping mutation.

Only a post-commit anchor movement to `[3,0,3]` records anchor arrival. Only a
post-ping runner movement to `[4,0,3]` while the ping is live records runner
arrival and scripted feedback. The gateway commits downing first, then changes
the actor from its exact current health/active state to zero/downed and emits
one matching life transition. A downed actor remains present but cannot move,
attack, equip, mutate modules, choose/start/ping/revive, or advance an
objective.

Only the active anchor can submit a revive ID. The target is the immutable
runner and distance is calculated by checked integer arithmetic from server
positions. Started, pending/replayed, cancelled, and completed evidence always
contains the same source/target, fixed duration/distance ceiling, and latest
authoritative observation. Out-of-range movement cancels before that movement
is published when the channel has not completed. At or after 2,000 ms while
still in range, persistence/replay commits before the actor changes from
zero/downed to 50/active and emits the life plus completed-channel evidence.

## Exact terminal summary

```text
CooperationGrant {
  actor_id,
  item_id,
  item_quantity,
  experience
}

CooperationNoRewardReason =
  ping_timeout | revive_timeout | operation_timeout |
  participant_defeated | participant_disconnected

CooperationOperationSummary {
  schema_version,
  session_id,
  catalog_revision,
  start_operation_id,
  last_nonterminal_phase,
  participants: [CooperationParticipantState; 2],
  contributions: CooperationContributions,
  outcome,
  subject_role,
  terminal_elapsed_ms,
  reward: CooperationReward | null,
  grants: [CooperationGrant],
  no_reward_reason: CooperationNoRewardReason | null
}
```

Success requires `encounter_active`, all five contributions, two active final
participants, one revive, null subject/reason, exact reward, and two ordered
actor grants. Failure has the exact pure-domain outcome, zero grants, null
reward, and its typed no-reward reason. Timeout subject is null; participant
defeat/disconnect names the exact immutable role. Summary is emitted only
after the matching Gate-4 terminal transaction commits.

On success the established Warden defeat/objective projection remains first,
then each current-V2 participant receives its committed generic loot/XP, then
generic activity completion is published, and the M28 summary is the final
cooperation message. On failure a committed defeated life transition, when
applicable, precedes the summary; no generic activity completion, history,
loot, XP, or route terminal is sent/written.

## Gateway authority and route exclusion

- Capability is stored independently per live participant and cleared on
  reset. Exactly two current-V2 participants must opt in independently.
- The first admitted actor is the only start authority. After committed start,
  index zero is immutable anchor and index one immutable runner.
- M27 route selection and M28 start share the existing single-threaded session
  command coordinator. The first valid persistence-backed operation wins.
- A committed cooperation start changes the cloned/current route domain to
  `baseline_locked` only after the M28 commit; no route row/event is created.
  An already-selected/locked M27 route rejects M28 start. A newly committed
  M27 route makes M28 unavailable to opted participants.
- Door/Warden progression is unavailable before successful revive. After
  `encounter_active`, the exact existing baseline door, Warden health, pressure,
  weapon/module profiles, hit rules, and objective sequence continue.
- Cooperation-active equipment changes are rejected and M26 mutation remains
  phase-locked, preserving embedded admission profiles. M27 choice rejects
  after cooperation locks its baseline.
- Every gameplay/module/route/cooperation request observes the active
  cooperation clock first. Pending revive observes current server distance.
  A candidate anchor move that would leave range cancels before publication.
- The operation `Instant` starts only after durable start commit. Exact tests
  call elapsed-time seams directly. Ping/revive subdeadlines precede the
  overall deadline, and exact inclusive boundaries retain the pure-domain
  meaning.
- Warden pressure that reduces a post-revive participant to zero commits
  `failed_participant_defeated` before publishing defeated life/terminal truth.
- Disconnect before start remains the M27 baseline. Disconnect after start
  observes deadlines first; if still nonterminal it commits one
  `abandoned_disconnect` for the exact role before removal and sends the
  summary to any writable opted peer. A later disconnect cannot append a
  second terminal.
- Persistence/replay rejection or failure exposes no optimistic start, ping,
  down, revive, life, reward, or terminal. A database failure follows the
  existing fail-closed session abort/reset path.
- Reset clears capability, cooperation runtime/clock/channel, actor life
  sidecar, and route coordination while preserving durable evidence. There is
  no resume, rejoin, replacement, or inferred elapsed time after restart.

## Required Gate 5 evidence

- round trips for all four client and six server variants;
- maximum-shaped variants below 8 KiB and the existing frame limit;
- unknown nested/top-level field, invalid enum/ID/shape, and oversized-frame
  rejection before mutation;
- current-V2 canonicalization and explicit frozen-V1 rejection for all four
  requests, with unchanged archived V1 hashes;
- waiting/drone/eligible/late capability, two independent opt-ins, immutable
  leader/roles, start apply/retry/conflict, and no pre-start contribution;
- route-first/cooperation-first mutual exclusion with no mixed durable/replay
  stream;
- wrong-role/order/phase ping and revive rejection; exact post-commit target
  movement, downed incapacity, server distance, pending/cancel/completion,
  health/life projection, and second-action rejection;
- exact 5,000/15,000/2,000/60,000 ms boundaries and subdeadline precedence
  through injected elapsed seams;
- start/ping/down/revive/success/failure persistence faults with no optimistic
  in-memory or wire truth;
- two-client identical role/ping/life/revive/success/failure truth, exact equal
  rewards on success, and no rewards on all five failures;
- ordinary unopted V2, mixed opted/unopted V2, solo, M27 Breach/Stabilize,
  current V1, frozen V1, reset, and existing smoke behavior unchanged; and
- full format/Clippy/test/build/Inspector/secret/smoke gate, loopback-only
  listeners, zero retained injection objects, unchanged `VERSION=0.2.0`, and
  no commit/tag/release/public exposure.

Block 5 does not add deterministic role bots or Godot presentation. If this
contract requires Protocol V3, a fifth client/seventh server variant,
unsolicited old-client messages, client-owned authority/timing/life/reward, a
third route, non-atomic confirmation, or resume/rejoin behavior, implementation
stops rather than widening the gate.
