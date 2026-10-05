# M28 Block 7 — honest Godot cooperation-presentation contract

Date: 2026-09-07  
Review state: **accepted before Block 7 source edits**  
Scope: Godot projection, cooperation surface, keyboard/input integration,
semantic harness, live creator-operated validation, and captures only.

## Authority and capability boundary

Godot exposes one neutral `CO-OP [C]` entry action. Opening it sends only
`CooperationStateRequest`. Before an accepted `CooperationState`, the surface
shows capability pending and no role, target, timing, contribution, life,
reward, or terminal fact.

An accepted state is the sole source for participant/capable actor lists,
eligibility, operation phase, immutable ordered roles, target coordinates,
timing constants, fixed reward plan, contributions, observed elapsed time,
ping, revive, life, and any terminal field. The first participant may submit a
start intent only when the server says `eligible` and all admitted participants
are capable. The client assigns no role and does not infer leadership beyond
the server's ordered admitted actor list.

Start, ping, and revive controls create one bounded ASCII operation ID and send
only that ID. Their local effect is a neutral pending label. Pending cannot
advance a phase, mark a contribution, create a ping, down or heal an actor,
start/advance/complete a channel, open the door, complete an objective, or
show a reward. Accepted/replayed/rejected wording comes only from the matching
typed result. Broadcast state and life transitions remain authoritative even
when they were caused by the peer or server clock.

## Exact gameplay projection

The code-native `CooperationConsole` fits within the existing 1280×720 logical
canvas and follows the established graphite, blue-petrol, cyan, amber,
magenta, and neutral visual system. It contains:

- `COOPERATION LINK • SERVER AUTHORITY`, capability/phase/session text, and
  explicit local role/actor text;
- two equally weighted role cards showing actor, life/health, and the exact
  required contribution: anchor arrival/ping/revive and runner console step;
- fixed target positions, operation/ping/revive budgets, maximum revive
  distance, restored health, and equal reward plan copied from operation state;
- six labelled contribution rows, so color is never the only status channel;
- lifecycle detail for empty/unavailable, capability pending, eligible, each
  request pending/rejected/accepted/replayed, anchor step, live/expired ping,
  runner step/downed, revive started/pending/cancelled/completed, encounter,
  each typed terminal outcome, connection loss, invalid server, and reset; and
- `START OPERATION`, `PING RELAY CONSOLE`, `REVIVE RUNNER`, `REFRESH`, and
  `CLOSE` controls enabled only from authoritative eligibility/role/phase.

The console displays received integer milliseconds; it has no local operation,
ping, or revive countdown and never promotes elapsed wall time into authority.
`CooperationLifeState` alone updates immediate downed/revived/defeated
feedback. The next valid state may reconcile the complete participant and
contribution snapshot. `CooperationOperationSummary` alone establishes
success, typed failure/abandonment, terminal elapsed time, exact per-actor
grants, or the typed no-reward reason.

No new asset, sound family, voice/text chat, marker system, screen shake,
flashing, particle, local resolver, or inferred teamwork/fairness claim is
added. Reduced Flash changes no cooperation information. Labels and symbols
repeat every semantic color distinction.

## Validation and invalid-server behavior

Projection accepts cooperation data only when schema version is 1, text and
operation identities remain inside reviewed byte/ASCII bounds, identities are
positive and unique, actor subsets and admission order agree, phases/enums are
finite, and operation truth satisfies the frozen M28 contract:

- catalog `m28-v1`, exactly two ordered anchor/runner participants, two unique
  fixed targets, and positive bounded health;
- 60,000/5,000/15,000/2,000 ms timing, 50 revive health, maximum squared
  distance four, and reward `relay_core_fragment ×2 / 125 XP`;
- contributions consistent with phase, a maximum revive count of one, exact
  ping source/target/TTL, and exact revive source/target/duration/distance;
- life transitions consistent with participant identity, health and cause;
  nullable source only for relay feedback; and
- success containing both ordered grants and all contributions, while every
  failure has no reward/grant and its exact no-reward reason/optional subject.

Immutable session, start ID, participants, targets, timing, and reward may not
change between accepted projections. A malformed or contradictory message
clears pending state into explicit `INVALID SERVER DATA`, retains no partial
incoming truth, and never silently merges it. Reconnect calls `join_world` and
resets all cooperation projection; it does not claim resume or fabricate an
abandonment summary. A transport loss during nonterminal cooperation displays
`CONNECTION LOST • OPERATION NOT RESUMED` separately from server rejection and
typed `abandoned_disconnect`.

## Keyboard and focus contract

- `C` opens/closes the surface while gameplay HUD is visible.
- Escape closes and restores focus to `CO-OP [C]`.
- Opening focuses the first enabled authoritative action, otherwise `REFRESH`.
- Tab/Shift+Tab reaches Start, Ping, Revive, Refresh, and Close; disabled
  actions remain readable and cannot emit an intent.
- While open, movement, attack, equipment, module, route, and world mouse
  input are blocked. Module/route/cooperation surfaces are mutually exclusive.

## Required Gate 7 evidence

The semantic harness must cover every required empty, capability, eligible,
pending, accepted/replayed/rejected, phase, life, ping, revive, encounter,
success, each failure family, invalid-server, disconnect, and reset state. It
must prove exact copied values, no optimistic mutation, role-only enablement,
non-color labels, full keyboard focus, Escape/focus restoration, Reduced Flash
retention, bounded IDs, and 1280×720 containment. All M17–M27 harnesses remain
green.

A live source gateway, Godot anchor, and standalone deterministic runner must
explicitly opt in, clear the drone, start, reach anchor, ping, receive the
runner downing, revive for an observed server channel, open the existing door,
defeat the Warden, and reconcile both generic grants/completion and the typed
summary. Creator-reviewable captures cover eligible roles, live ping/downed,
revive channel or completion, encounter, and terminal success. Inspector/SQL
reconciliation and failure/runtime repetition remain bounded by the next gate.

Automated playback and captures prove semantic correctness and renderability,
not external comprehension, preference, teamwork quality, accessibility
experience, or enjoyment. If implementation would require a protocol/server
change, client-owned timing/outcome, new communication system, new audiovisual
family, or broader scope, Block 7 stops.
