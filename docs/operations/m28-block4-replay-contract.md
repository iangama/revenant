# M28 Block 4 — replay, reconstruction, and Inspector contract

Date: 2026-09-06  
Review state: **accepted, implemented, and audited at Gate 4**  
Scope: exact six-kind vocabulary/payloads, atomic persistence coupling,
independent reconstruction, corruption rejection, and GET-only Inspector.

## Vocabulary and cardinality

Block 4 adds exactly six `ReplayEventKind` values:

| Event kind | Cardinality and purpose |
| --- | --- |
| `cooperation_started` | Exactly one for a newly committed M28 operation. It freezes the two admission-ordered roles/profiles, fixed targets, timing constants, and reward plan. |
| `cooperation_pinged` | At most one, after start. It records the anchor arrival and the one accepted fixed-target ping. |
| `player_downed` | At most one, after the live ping. It records runner arrival and exact current-health-to-zero Relay feedback. |
| `player_revived` | At most one, after downing. It records the successful channel start/completion and exact zero-to-50 restore. Cancelled or merely pending channels reserve no durable event. |
| `cooperation_succeeded` | At most one terminal and mutually exclusive with failure. It records the complete contribution/life snapshot and both committed grants. |
| `cooperation_failed` | At most one terminal and mutually exclusive with success. It records the last durable phase/contribution/life snapshot, exact bounded reason/subject, and an empty grant list. |

Same-input persistence retries append nothing. A successful stream has exactly
start, ping, down, revive, and success in that order. A failed stream has start,
the exact applicable prefix, then failure. A crash may leave any prefix without
a terminal; reconstruction reports only the last independently proven replay
state and never fabricates a pending channel or later contribution from current
database rows.

The start/ping/success/failure replay rows use the anchor account/actor.
`player_downed` and `player_revived` use the runner account/actor because their
facts describe the target life transition; both payloads also name the
authoritative source. No M28 stream may contain an M27 route kind, and a route
stream may contain no M28 kind.

## Shared bounds and fixed evidence

All six payloads are deterministic compact UTF-8 JSON with `schema_version: 1`,
snake-case enums, denied unknown fields at every M28-owned structure, and an
8,192-byte encoded/decoded maximum. Rust field order is canonical emission
order.

- session/account/character IDs use the existing 1-64-byte bounded storage
  grammar; start/ping/revive operation IDs use the frozen 1-32-byte ASCII
  alphanumeric/hyphen grammar;
- participant vectors contain exactly two unique account/character/actor rows,
  index zero `anchor` and index one `runner`;
- module evidence is a complete validated active-V2 `m26-v1` snapshot and the
  selected weapon identifies exactly one embedded effective profile;
- cooperation catalog is exactly `m28-v1`, activity exactly
  `relay_awakening`, targets exactly `relay_anchor [3,0,3]` and
  `relay_console [4,0,3]`, and Relay feedback is a fixed enum rather than text;
- operation/ping/revive-window/channel durations are exactly
  60,000/5,000/15,000/2,000 ms, revive health is 50, and maximum squared
  distance is four;
- reward is exactly two `relay_core_fragment` plus 125 XP for each participant;
  and
- elapsed arithmetic is checked, monotonic, and uses the same inclusive exact
  boundaries and phase-specific-before-overall precedence as the pure domain.

Reconstruction reads only append-ordered replay rows. It never reads current
actors, Lua, cooperation/route rows, inventory, module/loadout tables, gateway
memory, randomness, wall time, or current catalog behavior not embedded and
revision-validated in the events.

## Exact payloads

`cooperation_started`:

```text
CooperationStartedPayloadV1 {
  schema_version,
  activity_id,
  catalog_revision,
  start_operation_id,
  participants: [ReplayCooperationParticipantEvidence; 2],
  anchor_target: ReplayCooperationTargetEvidence,
  runner_target: ReplayCooperationTargetEvidence,
  timing: ReplayCooperationTimingEvidence,
  reward: ReplayCooperationRewardEvidence
}

ReplayCooperationParticipantEvidence {
  participant_id,
  character_id,
  actor_id,
  role: anchor | runner,
  weapon_item_id,
  module_state: ReplayModuleStateEvidence,
  admitted_health
}
```

The participant profile and maximum health are derived from the embedded
selected weapon/M26 evidence and must exactly equal the immutable persistence
snapshot. Admitted health is in `1..=maximum`; both life states start active.
The start event follows both admission module snapshots and agrees with them
byte-for-byte.

`cooperation_pinged`:

```text
CooperationPingedPayloadV1 {
  schema_version,
  catalog_revision,
  start_operation_id,
  anchor_elapsed_ms,
  ping_operation_id,
  source_participant_id,
  source_actor_id,
  target: relay_console,
  ping_elapsed_ms,
  expires_elapsed_ms
}
```

The source is the immutable anchor, anchor time is at or before ping time, and
expiry is the checked sum `ping_elapsed_ms + 5_000`.

`player_downed`:

```text
PlayerDownedPayloadV1 {
  schema_version,
  catalog_revision,
  start_operation_id,
  participant_id,
  actor_id,
  role: runner,
  target: relay_console,
  runner_elapsed_ms,
  hazard: relay_feedback,
  health_before,
  damage,
  health_after,
  life_before: active,
  life_after: downed
}
```

Runner time is within the accepted ping TTL, health before equals admitted
runner health, damage equals health before, and health after is zero.

`player_revived`:

```text
PlayerRevivedPayloadV1 {
  schema_version,
  catalog_revision,
  start_operation_id,
  revive_operation_id,
  source_participant_id,
  source_actor_id,
  target_participant_id,
  target_actor_id,
  target_role: runner,
  revive_started_elapsed_ms,
  revive_completed_elapsed_ms,
  channel_duration_ms,
  maximum_distance_squared,
  health_before,
  health_after,
  life_before: downed,
  life_after: active,
  revive_count
}
```

The source/target are the immutable anchor/runner, completion is at least 2,000
ms after start and no later than 15,000 ms after downing, health is exactly
zero to 50, and revive count is exactly one.

Both terminal kinds use one shape:

```text
CooperationTerminalPayloadV1 {
  schema_version,
  catalog_revision,
  start_operation_id,
  last_nonterminal_phase,
  contributions: ReplayCooperationContributionEvidence,
  participants: [ReplayCooperationTerminalParticipantEvidence; 2],
  outcome,
  subject_role,
  terminal_elapsed_ms,
  grants: [ReplayCooperationGrantEvidence]
}
```

Contribution evidence contains every retained optional elapsed field plus exact
booleans/revive count. Terminal participants repeat ordered identity/role and
record current health/life. Success requires encounter-active, all five
contributions, both active participants, one revive, Warden/terminal time at or
before 60,000 ms, a null subject, and two ordered exact grants. Failure requires
the pure-domain outcome and prefix, zero grants, no Warden contribution, and:

- null subject for ping/revive/operation timeout;
- exactly one defeated subject matching the zero/defeated participant for
  `failed_participant_defeated`; or
- the disconnecting anchor/runner for `abandoned_disconnect`, with participant
  life otherwise unchanged.

Ping/revive timeouts are strictly past their subdeadline; operation timeout is
strictly past 60,000 ms only when an active subdeadline has not already won;
defeat/abandonment is temporally ordered and no later than the applicable live
deadlines.

## Transaction and append order

- New start inserts the parent, both participant rows, and
  `cooperation_started` in one transaction.
- Accepted ping and scripted downing update their durable rows and append their
  respective events in the same transactions.
- Pending/cancelled/revive-start-only observations append nothing. Successful
  revive atomically updates parent/runner life and appends `player_revived`.
- Success inserts one generic `activity_completed`, both participants' existing
  inventory/progression/history grants and generic replay rows, updates the M28
  terminal, then appends `cooperation_succeeded`, all in one transaction.
- Every failure updates the durable terminal/defeated life when applicable and
  appends `cooperation_failed` in the same transaction, with no generic
  completion/reward write.
- Replay append IDs, not timestamps, define order. A cooperation terminal event
  is the final event in its M28 transaction and no M28/generic completion or
  reward event may follow it.

Every Gate-3 write boundary is repeated in replay mode, plus start/ping/down/
revive/success/failure replay append failures. Exact retry compares stored
payload evidence and appends nothing; a missing, duplicate, mismatched, or
partial replay companion fails closed.

## Reconstruction and corruption rejection

`ReconstructedSession` gains a `ReconstructedCooperationReplay` with explicit
legacy, active/incomplete, succeeded, and failed states plus the validated
start, optional ping/down/revive, and optional terminal payloads. It rejects:

- missing/duplicate/out-of-order start or transition; both terminal kinds,
  duplicate terminal, any M28 event after terminal, or a transition skipped
  from the required prefix;
- any coexistence with M27 route events;
- mixed session/activity/revision/operation or replay-row identity;
- participant cardinality/order/role/account/character/actor duplication,
  missing/late/mismatched M26 snapshots, inactive V1 module evidence, selected
  weapon/profile drift, or admitted-health drift;
- target/timing/reward constant drift, overflow, time reversal, wrong inclusive
  boundary, or wrong timeout precedence;
- wrong ping/downing/revive source or target, health/life arithmetic drift,
  second ping/down/revive, or impossible contribution prefix;
- success with missing/partial/duplicate/wrong generic completion/grants, and
  failure with any generic completion/grant or nonempty M28 grant list;
- terminal/participant/subject disagreement, post-terminal generic evidence,
  unknown nested/top-level fields, wrong schema, oversized payload, or unknown
  event kind.

A stream without an M28 kind remains `cooperation_replay_legacy=true` and
fabricates no role, contribution, life, terminal, or reward. Existing M26/M27
reconstruction remains independently valid.

## GET-only Inspector projection

Existing Inspector routes remain the only surface. Event responses expose a
decoded M28 payload only after complete-session reconstruction succeeds.
Session summary adds only:

```text
cooperation_replay_legacy
cooperation_replay_state
cooperation_last_phase
cooperation_terminal_outcome
cooperation_terminal_elapsed_ms
cooperation_participant_count
cooperation_contribution_count
cooperation_revive_count
cooperation_reward_participant_count
```

The frontend may label/render those persisted facts. No POST/PUT/PATCH/DELETE,
raw SQL, operation mutation, packet injection, client timer, or calculated life
state is added.

Protocol V2, gateway gameplay lifecycle, fake client, Godot, frozen V1,
`VERSION`, and the M27 catalog/replay shapes remain unchanged in Block 4.
