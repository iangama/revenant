# M27 Block 7 — honest Godot route-presentation contract

Date: 2026-09-05  
Review state: **accepted before Block 7 source edits**  
Scope: Godot projection, route surface, input integration, semantic harness,
live creator-operated validation, and review captures only.

## Authority and opt-in boundary

Godot does not expose candidate routes merely because it negotiated Protocol
V2, joined, recognized the activity, or contains a route-capable build. The
HUD exposes one neutral `ROUTES [R]` entry action. Opening it explicitly sends
`RouteStateRequest`; until the server answers, the surface says only that
server route state is pending and contains no candidate, selection, event,
effect, objective, reward, or timer projection.

An accepted `RouteState` is the sole source for capability, phase, session,
leader, admitted/capable actor lists, exact two options, possible event
effects, objective paths, duration budget, reward plans, and any immutable
selection. A rejection remains a rejection and exposes no locally cached
options. Reconnect starts from an empty route projection and does not claim to
resume an incomplete operation.

Only `leader_actor_id == player_actor_id`, `all_capable=true`,
`phase=choice_open`, no selection, and no pending request enable the two choice
buttons. Other participants see the server actor ID that decides and both
buttons disabled. There is no vote, recommendation, local default, automatic
choice, or hidden third route.

## Intent and lifecycle projection

Selecting a displayed route creates one bounded client operation ID and sends
only:

```text
RouteChoiceIntent {
  operation_id,
  route_id
}
```

The client may show `CHOICE PENDING`, the requested route label, and the fact
that no route has yet been accepted. Pending state may not select/highlight a
winner, resolve an event, alter objective guidance, change combat/reward
values, start a clock, or play success feedback.

`RouteChoiceResult` drives the distinct rejected, first-accepted, and accepted
replay dispositions. Only its server-supplied `selection` may display the
accepted route, seed, resolved event/effect, objective path, duration budget,
reward plan, leader, and participants. The following `RouteState` may refresh
that same immutable truth; disagreement is treated as invalid server data, not
silently merged.

`RouteOperationSummary` is the only route terminal authority. Success displays
server elapsed/budget, ordered transition count, exact reward, and exact grant
for the local actor. Timeout displays `FAILED — DEADLINE EXCEEDED`, failed
objective evidence, no reward/grant, and no generic completion claim.
Incomplete/disconnect is labelled `CONNECTION LOST — ROUTE NOT RESUMED` and is
never presented as a server rejection, timeout, or completion.

The route surface never contains the resolver, event catalog, health formula,
counter formula, reward arithmetic, local deadline timer, or inferred outcome.
It formats received integer facts only. The generic objective HUD continues to
consume `ObjectiveUpdate`; an active `reach_relay_stabilizer` explains the
server-authored `[3, 0, 3]` destination, while `reach_relay_door` preserves the
existing `[6, 0, 0]` guidance.

## Exact visual surface

One code-native `RouteConsole` control fits inside the existing logical
1280×720 canvas and uses the established graphite, blue-petrol, cyan, amber,
magenta, and neutral theme. It contains:

- title and explicit `SERVER AUTHORITY` label;
- capability/phase/leader/participant text;
- two fixed presentation columns populated only from the two server options,
  each listing route label, objective chain, 90,000 ms budget, fragment/XP
  plan, and both candidate event effect pairs;
- `BREACH` and `STABILIZE` buttons, plus `REFRESH` and `CLOSE`;
- lifecycle text for unavailable, pending, rejected, accepted, accepted replay,
  connection lost, succeeded, and failed-timeout states; and
- accepted/terminal detail copied from the server response.

The text must never say best, recommended, safer, fun, preferred, optimal, or
more valuable. Breach and Stabilize use the same visual weight. Color is
redundant with labels, icons/rails are not semantic authority, and no flashing,
particles, screen shake, or new audio asset is added. Reduced Flash is retained
as explicit state and causes no loss of information. Existing mute and fixed
audiovisual pools remain untouched.

## Keyboard and focus contract

- `R` opens or closes the route surface while the gameplay HUD is visible.
- Opening focuses `REFRESH` before authoritative options exist and the first
  enabled route button once an eligible state exists.
- Tab/Shift+Tab can reach both route buttons, `REFRESH`, and `CLOSE`; disabled
  choices remain visibly readable but cannot emit an intent.
- Enter/Space activates focused controls; Escape closes and restores focus to
  `ROUTES [R]`.
- While the route surface is visible, movement, attack, equipment, module
  opening, and world mouse input are blocked.

## Projection validation

The Godot authoritative projection accepts route data only when:

- schema version is 1; strings and lists stay inside reviewed wire bounds;
- phase/outcome/objective states are finite reviewed values;
- an accepted state has one or two unique participants, the leader first,
  capable actors as a participant subset, and exactly the two unique route IDs;
- each option has two events, three or four objectives, the fixed 90,000 ms
  budget, and a positive server reward;
- an accepted result has a valid pending/same operation and complete selection;
- selection identity/path/budget agrees with prior server state when present;
- terminal transition ordinals are consecutive and bounded, participants agree
  with the accepted selection, and success/failure reward shapes are distinct.

Malformed data clears pending state into an explicit invalid-server rejection;
it never partially replaces the last authoritative selection or terminal.

## Required Gate 7 evidence

The semantic harness must prove:

- no options before authoritative accepted state;
- neutral pending state changes no server-owned projection;
- exact two-option rendering with no preference language or local arithmetic;
- leader-only enablement and readable non-leader state;
- first acceptance, replay, conflict/rejection, timeout, success, connection
  loss, and reconnect reset are distinct;
- accepted data is copied exactly, with no event/seed/reward/objective inference;
- complete keyboard focus, Escape/focus restoration, bounded 1280×720 layout,
  Reduced Flash retention, and existing input suppression; and
- M17–M26 harnesses remain green.

A live source-gateway/Godot flow must opt in, render the server options, choose
at least one route through the route surface, traverse its server objective,
and reconcile accepted selection, generic objectives/rewards/completion, and
terminal summary. Creator-reviewable captures cover the open choice, accepted
event/path, and terminal summary. A second live route may be sampled here but
the complete bounded runtime matrix remains Block 8.

The full format/lint/test/build/frontend/secret/smoke gate, frozen-V1 hashes,
ordinary unopted Godot smoke, loopback-only listeners, and absence of new
untracked `.gd.uid` files are mandatory. Captures demonstrate renderability and
semantic states only; they do not prove outside comprehension, preference, or
enjoyment.

If honest presentation requires local authority, an unsolicited route request,
a protocol change, a new route/event, a new audiovisual pool, or an unbounded
payload, Block 7 stops instead of widening this contract.
