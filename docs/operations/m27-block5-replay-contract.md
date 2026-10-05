# M27 Block 5 — replay vocabulary and reconstruction contract

Date: 2026-09-01  
Review state: **accepted before replay implementation**  
Scope: exact replay vocabulary/payloads, atomic persistence coupling,
independent reconstruction, corruption rejection, and GET-only Inspector.

## Frozen vocabulary and cardinality

Block 5 adds exactly three `ReplayEventKind` values:

| Event kind | Cardinality and purpose |
| --- | --- |
| `route_selected` | Exactly one for a newly committed explicit route selection; records the complete immutable selection, participant/combat inputs, graph, seed resolution, effects, budget, and reward plan. |
| `route_operation_succeeded` | Exactly one for a newly committed successful terminal transaction; records transitions, encounter result, duration, and exact committed participant grants. |
| `route_operation_failed` | Exactly one for a newly committed timeout failure; records transitions, partial encounter result, duration, and an empty reward list. |

A same-input selection or terminal retry adds no event. A routed session has
one selection followed by at most one terminal event; success and failure are
mutually exclusive. An incomplete routed session has a selection and no
terminal. The compatibility baseline has none of these three events and is not
silently reclassified as a route.

All route replay rows use the leader account/actor identity. The payload still
contains the complete ordered participant evidence. No existing event name or
payload changes, and unknown kinds—including the 17 deliberate `future_event`
fixtures—continue to fail closed.

## Shared bounds and serialization

Every new payload is deterministic compact UTF-8 JSON with
`schema_version: 1`, snake-case enum values, denied unknown fields, and a hard
8,192-byte encoded/decoded maximum. Rust struct field order is canonical
emission order. Bounds are:

- session and participant IDs: 1-64 bounded ASCII bytes;
- activity, objective, event, weapon, and item IDs: `[a-z0-9_]{1,32}`;
- operation ID: 1-32 ASCII alphanumeric/hyphen bytes;
- participants and reward grants: one or two;
- selected objectives: three or four;
- objective transitions: at most eight; and
- every revision must equal `m27-v1` or `m27-permute63-v1` as applicable.

Route, event, effect, path, duration, seed, and reward validation uses embedded
immutable evidence plus the exact revision contract. Reconstruction never
reads current Lua, PostgreSQL route rows, inventory, loadout, gateway memory,
clock, or randomness.

## Exact selection payload

`route_selected` encodes:

```text
RouteSelectedPayloadV1 {
  schema_version,
  activity_id,
  authoring_revision,
  catalog_revision,
  resolver_revision,
  operation_id,
  leader_id,
  participants: [ReplayRouteParticipantEvidence],
  route_id,
  seed,
  event_id,
  effect,
  objectives: [ReplayRouteObjectiveEvidence],
  duration_budget_ms,
  reward
}

ReplayRouteParticipantEvidence {
  participant_id,
  character_id,
  actor_id,
  weapon_item_id,
  module_state: ReplayModuleStateEvidence
}

ReplayRouteObjectiveEvidence {
  objective_id,
  kind: kill_actors | reach_area | boss,
  initial_state: active | pending,
  target
}
```

Participant order is admission order; element zero equals `leader_id` and the
event row's account/actor. Account, character, and actor identities are unique.
Each `module_state` is the complete independently validated M26 snapshot
evidence. Its selected `weapon_item_id` must identify exactly one embedded
weapon profile; that profile is the immutable admitted combat input. Frozen V1
module evidence cannot enter an explicitly routed selection because later
routing capability is V2 opt-in only.

Objectives must equal the selected route path in canonical order, all with
target one. `clear_drone_group` is `kill_actors` and initially active;
`reach_relay_stabilizer`/`reach_relay_door` are `reach_area`,
`defeat_warden` is `boss`, and every later objective is initially pending.

## Exact terminal payloads

Both terminal kinds use the same shape; their event kind must agree with
`summary.outcome`:

```text
RouteTerminalPayloadV1 {
  schema_version,
  authoring_revision,
  catalog_revision,
  resolver_revision,
  operation_id,
  route_id,
  event_id,
  transitions: [ReplayObjectiveTransition],
  encounter: ReplayRouteEncounterEvidence,
  summary: RouteOperationSummary,
  grants: [ReplayRouteGrantEvidence]
}

ReplayObjectiveTransition {
  ordinal,
  objective_id,
  from: pending | active,
  to: active | completed | failed,
  progress,
  target
}

ReplayRouteEncounterEvidence {
  participant_count,
  drone_health,
  drone_damage,
  drone_defeated,
  warden_spawned,
  warden_base_health,
  warden_effective_health,
  warden_remaining_health,
  accepted_player_hits,
  warden_counter_count,
  warden_counter_damage,
  total_hostile_damage
}

ReplayRouteGrantEvidence {
  participant_id,
  character_id,
  actor_id,
  item_id,
  item_quantity,
  experience
}
```

Transitions are consecutive from ordinal zero, operate only on selection
objectives, and must form a legal activation/completion path. Success completes
the complete selected path exactly once with no failure and occurs at or before
90,000 ms. Failure occurs strictly after 90,000 ms, contains exactly one
active-to-failed transition for the current objective, no transition after
failure, and no completion/reward claim. At most one objective is active at
each reconstructed step.

Encounter values are finite and checked against the selected event, participant
count, recorded admitted profiles, M25/M26 enemy inputs, and Gate 2 bounds.
The Warden effective health uses the recorded basis points and checked
round-half-up arithmetic. Remaining health never exceeds effective health;
counter damage equals the event effect; total hostile damage is at most 50.
Unspawned Warden evidence has zero Warden result counters/health remaining.

A success grant list matches every selection participant exactly once in
admission order and equals the route reward. It is cross-checked against the
generic `loot_granted`/`progression_granted` rows committed in the same
transaction. A failure grant list is empty and the session has no M27 activity
completion/history or reward grant.

## Transaction and append-order contract

- New selection inserts the route parent, participant rows, and
  `route_selected` replay row in one transaction. A replay insert failure rolls
  back the entire selection and leaves the operation ID free.
- Success inserts/retains exactly one generic `activity_completed`, applies
  both existing participant grants/histories and their generic replay rows,
  updates the route terminal, then appends `route_operation_succeeded`, all in
  one transaction.
- Failure updates the route terminal and appends `route_operation_failed` in
  one transaction with no generic completion or reward write.
- Replay append IDs, not timestamps, define order. Selection precedes all route
  transitions and the terminal route event is last among the route operation's
  transactional evidence.
- Every existing Block 4 failure boundary is repeated with replay failure
  injection. No optimistic in-memory result is exposed before commit.

## Independent reconstruction and corruption rejection

`ReconstructedSession` gains explicit legacy/routed/incomplete/succeeded/failed
state plus the validated selection, transitions, encounter, summary, and grant
evidence. It rejects:

- missing/duplicate selection, terminal before selection, both terminal kinds,
  duplicate terminal, or any route event after terminal;
- mixed session/activity/revision/operation/route/event identity;
- participant count/order/leader/actor/character mismatch, missing/invalid M26
  snapshots, inactive V1 module evidence, or selected weapon/profile mismatch;
- seed/event disagreement, effect/catalog/path/duration/reward drift;
- duplicate, skipped, unknown, impossible, or post-terminal objective
  transitions and terminal state inconsistent with the graph;
- elapsed deadline contradiction, impossible Warden arithmetic/counters,
  negative/overflowing or out-of-envelope encounter values;
- success with missing/partial/duplicate/wrong generic grants, failure with any
  route grant/history/completion, or summary/grant disagreement;
- unknown fields/schema/kinds and any payload over 8,192 bytes.

An old session with no route event reconstructs as
`route_replay_legacy=true`, with no fabricated route, seed, transition, event,
summary, or reward. Existing M26 reconstruction remains independently valid.

## GET-only Inspector projection

Existing Inspector routes remain the only surface. Event responses may expose
a validated decoded route payload. Session summary adds only:

```text
route_replay_legacy
route_id
route_event_id
route_terminal_outcome
route_elapsed_ms
route_transition_count
route_reward_participant_count
```

The frontend may render those exact persisted facts and label route payloads
as validated structured evidence. It gains no POST/PUT/PATCH/DELETE route, raw
SQL, reroll, mutation, packet injection, secret, or client-calculated result.

Protocol V2, gateway route lifecycle, fake client, Godot, frozen V1, and
`VERSION` remain unchanged in Block 5. They are separately locked behind Gates
6 and 7.
