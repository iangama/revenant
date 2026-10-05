# M24 Block 4 — failure and recovery audit

Status: complete and reviewed; Gate A approved with bounded residual risk.

## Audit boundary

This audit records the behavior found at baseline commit
`4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6`, the uncommitted Block 4
correction, and the evidence collected against the rebuilt local compose stack.
It does not authorize reconnect, resume, a protocol or replay-vocabulary
change, participant recruitment, public exposure, Block 6, a version change,
commit, push, merge, tag, or release.

The accepted recovery contract is deliberately narrow:

- an operation that fails is never repeated automatically;
- a broken PostgreSQL connection is discarded;
- the next independent operation opens a new connection;
- a persistence failure after admission aborts the affected activity, closes
  participant sockets, and resets one fresh coordinator session;
- Retry always begins a new activity session;
- durable state committed before an unrelated disconnect, such as equipment,
  remains durable;
- no reconnect or resume is implemented or presented to the participant.

Risk labels are `recoverable`, `requires operator action`, `loss risk`, and
`pilot blocker`. “Loss risk” here means loss of in-progress activity or final
local observation, not loss of an already committed authoritative reward.

## Executive result

Before correction, PostgreSQL failures could leave memory, client projection,
completion replay, reward state, and reward replay disagreeing. A stale
coordinator connection also required a gateway restart. Immediate connection
refusal was mislabeled as timeout, a server disconnect could be overwritten by
`failed`, and generic guidance incorrectly described unrelated failures as an
occupied session. These were pilot blockers.

After correction:

- required replay is persisted before related in-memory stage promotion and
  client projection;
- equipment and its replay event are one transaction;
- completion, all participant rewards, history, idempotency grants, and reward
  replay are one transaction;
- failed persistence closes the participants and discards the defective
  session;
- the next operation reconnects without replaying the failed operation and
  without restarting the gateway;
- immediate refusal, timeout, incompatible handshake, and occupied admission
  have distinct allow-listed outcomes;
- the first terminal local outcome is monotonic;
- participant guidance consistently says **Retry from start**.

The real selective completion failure rolled back every completion and reward
write. A later fresh session on the same unchanged gateway completed exactly
once. A real immediate gateway termination preserved
`terminal_outcome=disconnected`; after service restoration, a new session
completed exactly once. No remaining Block 4 integrity or duplication pilot
blocker is demonstrated. The final uninterrupted `make check` and formal diff
gate passed, so Block 4 is closed with the residual risks listed below accepted
only for the supervised local pilot.

## Failure matrix

| Failure | Behavior before correction | Correction | Evidence after correction | Residual risk and recovery |
| --- | --- | --- | --- | --- |
| Gateway unavailable before Connect | Immediate refusal was described as a five-second timeout and shown with occupied-session guidance. | Transport status at deadline is classified structurally; failure presentation is neutral and says Retry from start. | Real refusal returned in about 6 ms with `connection_outcome=transport_failure`, `terminal_outcome=failed`, no session ID, and an unchanged allow-list. Deterministic client fixtures distinguish refusal and timeout. | **Recoverable.** Operator restores health; participant retries from the beginning. |
| Handshake incompatible | Rejection was terminal but presentation depended partly on parsing error text. | Session controller returns a structured `rejected` outcome for rejected or unexpectedly versioned handshakes. | Deterministic classification fixture and existing compatibility negotiation tests pass. | **Requires operator action.** Install the compatible build; retrying the same incompatible build remains rejected. |
| Session occupied | Correct gateway rejection existed, but generic client fallback could confuse other failures with occupancy. | Only the canonical “already active” world-join rejection maps to `session_unavailable`. | Deterministic classification and `active_or_full_sessions_reject_late_players` pass; rejected admission appends no join event. | **Recoverable.** Retry only after the active participants leave. |
| Mid-activity gateway loss | Activity memory disappears, replay remains incomplete, and a later `_fail()` overwrote `disconnected` with `failed`. | The local report accepts only the first terminal outcome; all failure guidance states that Retry begins from the start. | Immediate container termination produced `disconnect_observed=512` and `terminal_outcome=disconnected`; a later driver failure did not overwrite it. A post-health Retry created a distinct completed session and exactly one reward. | **Loss risk** for in-progress activity by design. No resume; preserve incomplete replay and start again. |
| Abrupt client termination | Coordinator reset was correct, but no final callback could complete the local report. | Server behavior remains reset-on-last-disconnect; no unsafe inference is added to the local report. | Existing real capture retains incomplete replay. The last atomic local snapshot can remain `running`; retention-declined temporary data still needs operator cleanup after an abrupt kill. | **Recoverable** activity, with local evidence and privacy cleanup risk. Follow the deletion runbook in Block 6. |
| PostgreSQL unavailable at startup | Gateway failed closed and restarted under compose policy. | Startup remains the only schema migration gate; runtime connections use the initialized schema. | Existing real startup injection recovered after PostgreSQL health returned. Parallel integration tests no longer compete to reapply DDL. | **Requires operator action** if database health does not return. No gameplay listener should be accepted before the startup gate. |
| PostgreSQL unavailable during authentication or character loading | Client transport could authenticate partially, then close before admission. | This boundary remains fail closed; no replay session is manufactured. Client reports `transport_failure`. | Existing pre-join injection created no authoritative session or reward. | **Recoverable.** Restore PostgreSQL and Retry from start. |
| PostgreSQL fails during an admitted activity | Fatal damage, actor removal, door/objective state, or stage promotion could outrun replay. The coordinator connection stayed unusable after PostgreSQL returned. | Required events precede projection; any session-owned persistence error aborts participants and resets. The broken connection is discarded and renewed only on the next operation. | Deterministic start, spawn, death, boss, equipment, completion, abort, and renewal fixtures pass. A real outage withheld fatal projection, aborted the activity, then a fresh operation reconnected and completed without restarting the gateway. | **Loss risk** for the interrupted activity. Persisted prefix remains for reconciliation; no operation retry or resume. |
| Failure inside multiplayer completion | Completion replay, reward transaction, and reward replay were separate. Historical injections produced completed replay without reward or committed reward absent from replay/client. | `activity_completed`, every participant reward, inventory, progression, history, idempotency grants, `loot_granted`, and `progression_granted` now share one PostgreSQL transaction. Projection occurs only after commit. | PostgreSQL rollback fixture fails the second participant and proves the first also rolls back. A real selective `progression_granted` fault produced no completion, loot, XP, history, grants, or reward projection. | **Recoverable.** Abort and Retry in a fresh session. Transaction state is no longer ambiguous. |
| Duplicate completion tuple | Persistence already suppressed duplicate inventory reward, but orchestration-level evidence could be partial. | Atomic completion inserts replay reward events only when a participant reward grant is newly applied. | Repeating the same multiplayer completion returns `[None, None]`; inventory, XP, history, grants, and replay counts remain one per participant. Runtime Retry uses a new session and grants exactly once. | **Recoverable.** Idempotency is per `(session_id, character_id, item_id)`; operator still reconciles incomplete old sessions separately. |
| Local report through failure and Retry | Raw error wording drove classification and terminal state could be overwritten. | Structured outcome, first-terminal monotonicity, neutral guidance, and the original allow-list are enforced. | Godot harness and real refusal/disconnect reports pass. No username, endpoint, raw error, payload, or invented session ID is stored. | **Loss risk** on abrupt process kill. One report preserves its first terminal attempt; later attempts use separately retained evidence. |

## Historical pre-correction evidence

The following local-only injections established the original blocker. Their
temporary triggers were removed after use and never became repository schema.

| Session | Selective fault | Durable result before correction |
| --- | --- | --- |
| `session-1788003936670073670` | PostgreSQL stopped before fatal drone `enemy_died` | Client had already seen fatal damage and actor removal; replay had no defeat and the stale coordinator required restart. |
| `session-1788020738124311635` | `boss_spawned` rejected | Door objectives had advanced visibly, but replay had no boss spawn. |
| `session-1788030723660784757` | `activity_completed` rejected | Both defeats existed; summary remained incomplete and no reward was granted. |
| `session-1788030741281219957` | reward transaction rejected | Completion replay existed, but reward/history/grants rolled back. |
| `session-1788031266198779812` | `loot_granted` rejected after reward commit | Reward was durable but absent from reward replay and client projection. |
| `session-1788031442826391255` | `progression_granted` rejected after loot replay | Durable reward had only partial replay and no client reward projection. |

The corresponding real Godot captures PT-G419 through PT-G422 fixed the
participant-visible contradiction: an open door without Warden, or a visibly
completed Warden objective without completion and reward messages. Those
states motivated the atomic and fail-closed correction; they are not the
current behavior.

## Correction architecture

### Startup and runtime connections

`Persistence::connect` remains the startup migration gate and serializes schema
application with a PostgreSQL advisory transaction lock.
`Persistence::connect_existing` opens a connection to the already initialized
schema for gameplay, Inspector, and replacement connections. This prevents DDL
from competing with active gameplay transactions.

`RecoveringSessionPersistence` owns one optional session connection. Any
operation error drops it. The failed closure is returned once and is never
replayed. The next operation invokes the connector and emits
`session_persistence_reconnected` after a connection opens.

### Fail-closed coordinator

Admission failure does not abort already admitted participants because the
rejected participant has not entered memory or replay. After admission, a
`SessionPersistenceFailure` clears participant senders, causing writer threads
to shut down both halves of their sockets, then resets exactly one fresh
session. Late disconnect commands for already-cleared participants cannot
cause a second reset.

Normal domain rejections remain ordinary command results. Invalid target,
cooldown, movement bounds, unknown weapon, unowned weapon, and equipment lock
do not become persistence failures and do not abort the coordinator.

### Ordering

- `player_joined` persists before the actor and participant enter memory.
- `activity_started` and `enemy_spawned` persist before ActivityStart,
  objectives, player spawns, enemy spawn, stage promotion, and AI projection.
- `enemy_died` persists before fatal damage, actor destruction, and objective
  projection.
- `boss_spawned` persists before Boss stage promotion, door/objective
  projection, and Warden spawn.
- equipment loadout and `equipment_changed` replay commit together before
  memory and client projection change.
- completion transaction commits before fatal boss projection, completion
  objective, participant reward messages, and ActivityComplete.

Some domain methods necessarily calculate candidate activity or actor state
before persistence. A persistence failure exposes none of that candidate state
to the client and the mandatory abort resets it. This is a bounded residual
implementation detail, not resumable state.

### Atomic multiplayer completion

One transaction contains:

1. at-most-once `activity_completed` for the session;
2. each participant's inventory idempotency grant;
3. inventory quantity;
4. progression idempotency grant;
5. progression and character level;
6. activity history;
7. participant `loot_granted`;
8. participant `progression_granted`;
9. commit.

If any participant or replay write fails, PostgreSQL rolls the entire unit back.
On exact repetition, existing participant grants return `None` and no duplicate
reward replay event is appended.

## Deterministic evidence

The gateway suite contains 17 passing tests, including:

- failure of `activity_started` and `enemy_spawned` with no projection;
- failure of `boss_spawned` with no door, objective, or actor projection;
- failure of `enemy_died` with no fatal projection;
- equipment failure with no persisted, in-memory, or projected change;
- completion failure retaining the prior stage and projecting no reward;
- successful two-participant completion with exact per-participant rewards and
  message order;
- invalid normal combat command followed by a successful command on the same
  session;
- persistence failure disconnecting participants and accepting a fresh join;
- late disconnect resetting an active session whose last participant was
  already removed by a failed broadcast, without a second reset;
- discarded connection renewed only on the following operation;
- one named failure receiving no silent fallback retry.

The PostgreSQL suite contains five passing tests. Its atomic multiplayer
fixture proves:

- second-participant failure rolls back the first participant;
- no completion or reward replay survives rollback;
- no inventory or XP survives rollback;
- successful participants receive their own resulting quantities and
  progression;
- repeating the transaction is harmless;
- equipment and its replay commit or roll back together.

The Godot M24 harness additionally proves structured timeout, refusal,
handshake, occupied, and other-join classifications; report allow-list and size
bounds; consent behavior; and first-terminal monotonicity.

The first final-gate smoke attempt exposed a test-harness scheduling limit: the
observer bot's five-second socket timeout could expire while the second
`cargo run` process was still starting, causing the observer to leave before
the two-player activity began. An instrumented repetition completed every
canonical path. The fake client's bounded I/O timeout was aligned to the
gateway's 15-second handshake window, after which the full uninterrupted gate
passed. This changes only the local smoke client and neither protocol behavior
nor gateway deadlines.

## Post-correction runtime evidence

### Real PostgreSQL outage and live recovery

PostgreSQL was stopped during combat without deleting or recreating its volume.

- Interrupted account/session:
  `local:m24-recovery-fail` /
  `session-1788055239968946748`.
- Durable prefix: join, start, spawn, and equipment.
- Fatal death was not projected after its required persistence failed.
- Loot, XP, history, and grants remained zero.
- Participant channels closed and the session reset.

After PostgreSQL returned, without restarting the gateway:

- the gateway emitted `session_persistence_reconnected`;
- `local:m24-recovery-success` entered fresh session
  `session-1788055439863213424`;
- the activity completed with one fragment, 100 XP, and one history row;
- the gateway container ID remained unchanged.

### Real selective failure inside the completion transaction

A temporary trigger rejected only `progression_granted` for
`local:m24-atomic-runtime-fail`.

- Failed session: `session-1788089644734997724`.
- Replay persisted only join, start, spawn, drone death, boss spawn, and boss
  death.
- `activity_completed`, `loot_granted`, and `progression_granted` were absent.
- Fragment, XP, history, inventory grants, and progression grants were zero.
- The client received no fatal boss confirmation, reward, or completion after
  the transaction failed.
- Gateway emitted one command failure, reset, and abort.
- The trigger and function were removed; `trigger_count=0`.

Without restarting the gateway, the same account retried:

- fresh session: `session-1788089759666834529`;
- the replacement connection opened on the new operation;
- replay contained one complete lifecycle and one reward pair;
- resulting state was one fragment, 100 XP, one history row, one inventory
  grant, and one progression grant;
- the failed session remained unchanged;
- the gateway container ID remained
  `b1cadaacdaa91cd3f90b7a04f19eadabe4f5fd783f7185c6334a2cf8c5451a90`.

### Real gateway termination and monotonic disconnect

An initial graceful-stop attempt was intentionally discarded as disconnect
evidence because the activity completed during the stop window. The valid
injection used immediate termination during the manual flow.

- Account: `local:m24-disconnect-monotonic-2`.
- Interrupted session: `session-1788090621746254249`.
- Replay: join, start, spawn, and the already committed equipment change.
- Report PT-H426: `connection_outcome=connected`,
  `disconnect_observed=512`, `completion_observed=null`, and
  `terminal_outcome=disconnected`.
- The later manual-driver `_fail()` did not overwrite the terminal outcome.
- Reward, XP, history, and grants remained zero.

After gateway health returned:

- Retry session: `session-1788090662243881295`;
- the activity restarted from its beginning;
- the previously committed sidearm selection correctly remained durable;
- the session completed with exactly one fragment, 100 XP, one history row,
  one inventory grant, and one progression grant;
- PT-H427 recorded `terminal_outcome=completed`;
- no activity resume or reward duplication occurred.

## Operational reconciliation

After an interrupted session, the operator must:

1. confirm PostgreSQL, gateway, and Inspector health;
2. confirm no temporary trigger or failure function remains;
3. preserve the incomplete replay prefix;
4. inspect summary, ordered replay, inventory, progression, history, and both
   grant tables;
5. classify the local report independently and preserve contradictions;
6. tell the participant to Retry from start only after health is restored;
7. verify the Retry creates a distinct session and grants exactly once;
8. never delete the PostgreSQL volume to make recovery appear successful.

## Residual risks

- No Protocol V2 session ID is projected to the local report, so correlation
  remains an operator task.
- No resume exists; an interrupted participant loses in-progress activity.
- Abrupt client kill can leave the last local report `running` and can leave a
  retention-declined temporary file for operator cleanup.
- Database loss during pre-admission account or character work closes that
  connection but is outside the shared-session recovery wrapper; Retry opens a
  new connection and creates no replay session for the failed attempt.
- The current activity reward is validated and small. A future content system
  that permits quantities outside PostgreSQL/wire conversion bounds needs its
  own domain limit before deployment.
- Reset failure after an abort would leave no participant socket open but could
  prevent new admission; operator restart remains the fallback.

These risks are acceptable only for a supervised, local, closed pilot with the
Block 6 runbook. They do not authorize public or unattended operation.

## Final gate result

Gate A is **approved with bounded residual risk**.

- Uninterrupted `make check` passed with the repository-local Godot 4.7.1
  binary.
- Version, formatting, workspace Clippy with warnings denied, workspace tests,
  build, Inspector typecheck/build, secret audit, and the canonical smoke all
  passed.
- The 17 gateway fixtures and five live PostgreSQL fixtures passed.
- The automatic, reusable-session, and manual Godot flows passed, followed by
  every M17–M24 deterministic marker.
- Replay, Inspector summary, frozen V1 compatibility, exact frozen hashes, and
  reconstruction without the current gateway passed.
- M21 and M22 capture manifests verify successfully.
- `VERSION=0.2.0`; protocol, replay vocabulary, and frozen V1 paths have no
  diff.
- Compose services remain healthy and bound only to `127.0.0.1`; the disposable
  smoke ports were released.
- No `m24_fail_%` trigger or function, old validator, commit, push, merge, tag,
  release, recruitment, or public exposure remains.
- `git diff --check` passed and the intentional uncommitted review tree was
  preserved.

No reproducible integrity, duplication, participant-projection, privacy, or
unrecoverable-session blocker remains. Block 5 may proceed under its separate
owner authorization; this result authorizes nothing beyond Block 5.
