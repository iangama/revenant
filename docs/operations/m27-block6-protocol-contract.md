# M27 Block 6 — additive Protocol V2 and gateway contract

Date: 2026-09-01  
Review state: **accepted before Block 6 source edits**  
Scope: opt-in Protocol V2 messages, immutable route admission, authoritative
shared gateway lifecycle, monotonic deadline, persistence/replay coupling, and
ordinary-V2/frozen-V1 compatibility.

## Additive wire vocabulary

`PROTOCOL_VERSION` remains 2 and the 64 KiB frame limit remains unchanged.
Block 6 adds exactly two `ClientMessage` variants and three `ServerMessage`
variants:

| Direction | Variant | Purpose |
| --- | --- | --- |
| client | `RouteStateRequest(RouteStateRequest {})` | Explicitly opts the already-admitted current-V2 participant into M27 route capability and requests current server truth. |
| client | `RouteChoiceIntent(RouteChoiceIntent)` | Requests one leader-owned route operation using only an operation ID and route ID. |
| server | `RouteState(RouteState)` | Targeted/broadcast capability, phase, leader, participant, exact route-option, and optional selected-state truth for opted participants. |
| server | `RouteChoiceResult(RouteChoiceResult)` | Targeted rejection or shared accepted/replayed immutable selection. |
| server | `RouteOperationSummary(RouteOperationSummary)` | Shared terminal success or timeout-failure evidence for opted participants. |

No message is sent merely because a client negotiated V2, used a current
build name, authenticated, or joined. An ordinary V2 client that sends neither
new request receives the exact established sequence. Frozen V1 cannot
canonicalize either new request and receives no new response.

## Wire constants and bounds

The new structs use `schema_version: 1`, deny unknown fields, and use typed
snake-case enums for phase, objective kind/state, and terminal outcome.
Gateway validation enforces:

- operation ID: 1-32 ASCII alphanumeric/hyphen bytes;
- session/participant identity already admitted by the server: 1-64 bytes;
- route, event, objective, revision, activity, item, and reason IDs:
  `[a-z0-9_-]{1,32}` as appropriate;
- display/result message: at most 128 UTF-8 bytes;
- participants/capable actors/grants: one or two;
- route options: exactly two;
- event candidates: exactly two per route option;
- objective paths: three or four;
- transitions: at most eight; and
- the maximum shaped request and response must serialize below 8 KiB and
  therefore remain far below the existing 65,536-byte frame limit.

The protocol crate exports the corresponding explicit maxima. A frame over the
existing limit fails before allocation. Malformed, unknown-variant,
unknown-field, invalid-enum, or wrong-shaped MessagePack fails before a session
command reaches domain/persistence code.

## Exact request structs

```text
RouteStateRequest {}

RouteChoiceIntent {
  operation_id,
  route_id
}
```

`RouteChoiceIntent` contains no seed, event, effect, reward, leader,
participant, objective transition, duration result, wall-clock value, or
client confirmation. Invalid operation/route input is rejected without seed
generation, domain mutation, persistence, or identifier reservation.

## Exact state and option structs

```text
RouteState {
  schema_version,
  accepted,
  message,
  session_id,
  activity_id,
  phase: waiting | drone | choice_open | baseline_locked | routed | succeeded | failed,
  leader_actor_id,
  participant_actor_ids,
  capable_actor_ids,
  all_capable,
  routes: [RouteOption],
  selection: RouteSelection | null
}

RouteOption {
  route_id,
  objective_path,
  duration_budget_ms,
  reward: RouteReward,
  events: [RouteEventCandidate]
}

RouteEventCandidate {
  event_id,
  effect: RouteEffect
}

RouteEffect {
  warden_health_basis_points,
  warden_counter_damage
}

RouteReward {
  item_id,
  item_quantity,
  experience
}
```

An accepted state request sets only that participant's session-local
capability bit. Before all expected participants are present, it may report
`waiting`; activity start initializes the domain operation in authoritative
join order and imports only those explicit bits. A request during drone or an
open choice is idempotent. A first request after baseline lock, route terminal,
or other ineligible phase is a targeted rejection with empty routes/selection
and does not opt the participant in.

Exact route options are projected from the frozen `m27-v1` catalog. They expose
both possible server-authored event effects as tradeoff facts but resolve
neither. The leader is the first admitted actor. When capability or phase
changes, only opted participants receive the current state. No unopted client
receives this variant.

## Exact choice result and immutable selection

```text
RouteChoiceResult {
  schema_version,
  accepted,
  replayed,
  message,
  session_id,
  operation_id,
  selection: RouteSelection | null
}

RouteSelection {
  leader_actor_id,
  participant_actor_ids,
  route_id,
  seed,
  event_id,
  effect,
  objective_path,
  duration_budget_ms,
  reward
}
```

Rejections are targeted to the requester and contain no selection. A newly
accepted result is broadcast identically to every opted participant only after
the selection and `route_selected` evidence commit. A same-input retry returns
the exact stored selection with `replayed=true` to the requester and appends
nothing. A conflict, new operation after lock, non-leader, incomplete
capability, wrong phase, malformed input, or persistence failure changes no
in-memory route state and exposes no accepted result.

Production creates a 63-bit seed from the operating system random source only
after every deterministic validation that can precede selection. Tests inject
a non-network deterministic seed source. The request, username, build,
timestamp text, movement, loadout, and request timing never derive the seed.
The gateway never sends a client-provided seed to persistence.

## Exact terminal summary

```text
RouteOperationSummary {
  schema_version,
  session_id,
  authoring_revision,
  catalog_revision,
  resolver_revision,
  operation_id,
  leader_actor_id,
  participant_actor_ids,
  route_id,
  seed,
  event_id,
  effect,
  objective_path,
  transitions: [RouteObjectiveTransition],
  duration_budget_ms,
  elapsed_ms,
  outcome: succeeded | failed_timeout,
  reward: RouteReward | null,
  grants: [RouteGrant],
  no_reward_reason: null | deadline_exceeded
}

RouteObjectiveTransition {
  ordinal,
  objective_id,
  from: pending | active,
  to: active | completed | failed,
  progress,
  target
}

RouteGrant {
  actor_id,
  item_id,
  item_quantity,
  experience
}
```

Success contains the complete ordered transition path and one exact grant per
participant; `no_reward_reason` is null. Timeout contains one terminal failed
transition, no grants/reward, and `deadline_exceeded`. The message is emitted
only after the corresponding Block 5 terminal transaction commits. It is sent
only to opted participants. Generic success objective/reward/completion
messages remain in their established order; failure emits an authoritative
Failed objective plus this summary and no generic completion/reward.

## Gateway lifecycle

- The route domain is created when the complete admitted participant set
  starts the activity. Account order is durable join append order; actor order
  follows it and element zero is the leader.
- Drone completion opens choice. Opted participants receive current state;
  unopted participants receive only the existing generic drone/objective
  sequence.
- Reaching the existing door at `[6, 0, 0]` without a selected route explicitly
  locks the compatibility baseline. No routed persistence/replay/protocol
  evidence is fabricated.
- Accepted `breach` keeps the door as the active next objective. Accepted
  `stabilize` makes `reach_relay_stabilizer` active and the door pending; the
  exact stabilizer trigger is `[3, 0, 3]`, after which the door becomes active.
- Selection commit starts an `Instant`-based 90,000 ms deadline. Wall-clock
  time remains display evidence only. The first gameplay/module/route command
  observed strictly after the budget commits timeout failure before applying
  that command. Disconnect/reset may leave a selected operation incomplete and
  never invents a terminal timestamp.
- The selected event applies Warden health scaling once after existing
  participant scaling and supplies the Warden counter damage. Each admitted
  participant retains the immutable selected weapon/module profile already
  embedded in selection replay.
- Objective transitions, drone/Warden health, accepted Warden hits, counters,
  hostile damage, duration, and grant evidence are accumulated from
  authoritative state and used to build the exact Block 5 terminal payload.
- Success uses `complete_route_operation_with_replay`; timeout uses
  `fail_route_operation_timeout_with_replay`. In-memory phase, objective,
  actors, rewards, and accepted wire projection change only after commit.
- Reset clears capability, route state, seed/deadline, transitions, and
  encounter tracking while preserving durable evidence.

The existing activity Lua, additive schema, replay vocabulary, Inspector,
Godot, frozen V1, and `VERSION=0.2.0` do not change in Block 6.

## Required Gate 6 evidence

- round trips for both client and all three server variants;
- maximum-shaped request/state/choice/summary frames below 8 KiB and the
  existing frame limit;
- malformed, unknown-field, invalid ID/route/list/enum, and oversized-frame
  rejection before mutation;
- explicit capability, first-admitted leader, non-leader/incomplete-capability
  rejection, accepted selection, same-result retry, conflict, and no reroll;
- solo and two-active shared selection/event/path/deadline/terminal truth;
- both route objective paths, all event effect axes through injected seeds,
  Warden health/counter application, success at the boundary, timeout after
  the boundary, no reward on failure, disconnect-incomplete, and reset;
- selection/terminal persistence failure with no optimistic in-memory or wire
  success;
- an ordinary current-V2 flow with no new request and no new response variant;
- frozen-V1 byte hashes, gameplay smoke, and reconstruction unchanged; and
- full format/Clippy/test/build/frontend/secret/smoke gate with loopback-only
  listeners and no protocol-version, Godot, tag, release, or public exposure
  change.

If any exact behavior requires Protocol V3, unsolicited old-client messages,
client-derived authority, non-atomic confirmation, or a third route/event,
Block 6 stops rather than widening this contract.
