# M25 — Combat pleasure and fairness

Status: complete; Gate M25 approved with bounded residual risk (green).

## Authorization and boundary

The owner explicitly authorized continued sequential execution of every
already-sketched solo roadmap milestone. M25 is the first active milestone.
M26-M32 remain ordered behind their preceding green gates; authorization does
not permit skipping a gate, involving another person, exposing a service,
publishing an artifact, or creating a commit, push, merge, version change, tag,
or release.

M25 is creator-only. Deterministic metrics and creator A/B notes may guide the
work, but neither is evidence of population preference. The server remains
authoritative for targets, range, cooldown, damage, health, death, activity
state, rewards, and replay. Client presentation may acknowledge an attempted
input locally, but hit, miss, damage, defeat, and reward confirmation must
remain consequences of authoritative messages.

Protocol V2, the replay vocabulary, frozen V1 artifacts, and `VERSION=0.2.0`
remain unchanged unless a separately reviewed need proves that M25 cannot be
completed without changing one of them. The initial design requires no such
change.

## Preserved M24 checkpoint

Before the first M25 change, the complete 273-file M24 closure tree was stored
outside the repository at:

`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m24-solo-closure-45718926-20260830.tar.gz`

Its SHA-256 is
`e7bca1305adda8ad0267e5e8d80b475016c9cc9d11308f6f650612d15b610aa9`
and its size is 9,471,062 bytes. This local source checkpoint supplements the
unchanged frozen M24 Windows package; it is not a commit, release, or new
package.

## Objective

Turn the technically stable `relay_awakening` combat into a measured,
server-authoritative baseline with two genuinely distinct weapons, bounded
enemy pressure, honest audiovisual confirmation, repeatable solo and
two-client behavior, and explicit creator A/B evidence.

M25 does not add another activity, progression system, module taxonomy,
cooperation mechanic, public networking, or content-mass pass.

## Baseline facts before M25 tuning

| Property | Current value | Consequence |
| --- | --- | --- |
| Pulse rifle | 40 damage, range 6, 250 ms cooldown | 3 hits and 500 ms theoretical TTK against both current enemies |
| Arc sidearm | 25 damage, range 8, 150 ms cooldown | 4 hits/450 ms against the drone; 5 hits/600 ms against the Warden |
| Client input gate | Fixed 260 ms for both weapons | Suppresses the sidearm's authoritative cadence and makes its effective TTK 780/1,040 ms |
| Relay drone | 100 HP, starts at `[4,0,2]`, chases to attack range, deals one 10-damage hit | Both weapons are in range; pressure occurs once |
| Warden | 120 HP, spawns at `[8,0,0]` after the player reaches `[6,0,0]` | Both weapons are in range; no Warden attack currently runs |
| Multiplayer | Same enemy health regardless of participant count | Correctness is covered, but two-active-attacker difficulty is not measured |
| Confirmation | Damage and defeat VFX/audio follow `DamageApplied`/`ActorDestroy` | Authoritative hit confirmation exists; local cooldown cue is fixed at 260 ms |

Theoretical TTK is measured from the first accepted hit to the lethal accepted
hit and therefore equals `(required_hits - 1) * cooldown`. Runtime encounter
duration also includes input, transport, scheduling, movement, presentation,
and stage transitions and is recorded separately.

## Hypotheses

1. **Authoritative cadence:** the local input gate can use the equipped
   authoritative weapon profile without predicting acceptance or allowing a
   profile switch to bypass the server cooldown.
2. **Weapon identity:** the rifle can remain the larger, slower, shorter-range
   hit while the sidearm remains the smaller, faster, longer-range hit, with no
   weapon dominating damage, cadence, range, and measured encounter time at
   once.
3. **Bounded encounter time:** optimal local solo TTK can remain between
   600-1,200 ms for the teaching drone and 1,200-2,600 ms for the Warden after
   tuning, excluding pre-combat movement and stage transitions.
4. **Fair pressure:** each enemy can create observable, non-lethal pressure in
   the deterministic solo path; expected optimal total incoming damage remains
   between 10 and 70 of the player's 100 HP, and no damage is confirmed before
   the corresponding server event.
5. **Multiplayer scaling:** two active local attackers must not create partial
   rewards or projection divergence, and their TTK must remain at least 45% of
   the equivalent solo TTK rather than collapsing to an unmeasured instant
   completion.
6. **Latency tolerance:** at simulated 0, 75, and 150 ms round-trip delay,
   every accepted hit remains ordered and exact; added delay must not produce a
   duplicate hit, cooldown bypass, false confirmation, or reward change.
7. **Presentation clarity:** attempted input, authoritative hit, hostile hit,
   cooldown, defeat, and target-unavailable states remain visually and
   acoustically distinguishable with mute and Reduced Flash respected.

The timing ranges are engineering budgets for this small local slice, not
claims about what other players prefer.

## Metric contract

The deterministic combat laboratory records, for each weapon, enemy,
participant count, and simulated round-trip delay:

- required accepted hits;
- per-hit damage, target health, overkill, and range margin;
- authoritative cooldown and effective client interval;
- theoretical and delay-adjusted TTK;
- sustained damage per second;
- cooldown duty cycle;
- expected hostile attacks and incoming damage;
- solo/two-attacker TTK ratio;
- whether every active acceptance budget passes.

Runtime drivers separately record wall-clock encounter and activity duration,
accepted damage sequence, player health sequence, equipment profile, session
identity, participant count, completion/reward counts, and replay/Inspector
reconciliation. Presentation fixtures record semantic cue counts and bounded
effect/audio pools rather than using screenshots as authority.

## Block plan

### Block 1 — Baseline and deterministic laboratory

- Add a pure combat-analysis API without clock, socket, database, or Godot
  dependencies.
- Add a local CLI that emits deterministic JSON and a human-readable matrix.
- Encode the untuned baseline before changing profiles or health.
- Prove invalid profiles, arithmetic overflow, cooldown boundaries, overkill,
  range margin, participant scaling, and simulated-delay calculations.
- Record a baseline report and decide the exact first tuning candidate.

Gate 1 requires reproducible metrics that describe the current game exactly.
It authorizes no tuning by itself.

Gate 1 is approved. `docs/combat/m25-untuned-baseline.md` records all 24
untuned rows, the current failed budgets, and the bounded first tuning
candidate. The pure API and CLI pass format, tests, and Clippy with warnings
denied. No gameplay value changed in Block 1.

### Block 2 — Authoritative cadence and weapon identity

- Remove the fixed client attack gate and derive it from the authoritative
  equipped profile.
- Keep the server cooldown shared across profile switches.
- Tune damage, cooldown, range, and encounter health only through catalogued,
  tested constants.
- Assert neither weapon dominates all measured dimensions.
- Update automatic/manual/keyboard drivers to terminate from authoritative
  state instead of fixed hit counts.

Gate 2 requires both weapons to satisfy the timing and identity budgets with
all existing authority and compatibility tests green.

Gate 2 is approved with bounded residuals assigned to Blocks 3-5.
`docs/combat/m25-block2-cadence-audit.md` records the exact candidate matrix,
clean Godot fixture, full smoke, accepted rifle/sidearm runtime sessions, and
rejected harness attempts. The Warden pressure, two-active-client runtime, and
experiential range value remain explicitly open.

### Block 3 — Fair enemy pressure

- Define explicit server-owned drone and Warden pressure profiles.
- Exercise hostile cadence, player-health bounds, target loss, death boundary,
  and two-participant target selection deterministically.
- Ensure pressure cannot continue after enemy death, session completion,
  disconnect reset, or persistence abort.
- Avoid a Protocol V2 expansion unless this block is formally stopped and
  replanned.

Gate 3 requires non-lethal optimal pressure, deterministic lethal fixtures,
and no unavoidable or post-terminal damage.

Gate 3 is approved with bounded residuals assigned to Blocks 4-5.
`docs/combat/m25-block3-pressure-audit.md` records the shared runtime/lab
profiles, exact hostile ordering and bounds, lethal/target-loss/reset fixtures,
canonical multiplayer and four-flow Godot smoke evidence, and two rejected
pre-gate runs that exposed and corrected target-selection and parser defects.
Protocol V2 and replay remain unchanged.

### Block 4 — Honest combat presentation

- Align local cooldown duration and HUD profile with authoritative equipment.
- Make attempted input, cooling, unavailable target, confirmed player hit,
  confirmed hostile hit, and defeat semantically distinct.
- Preserve mute, Reduced Flash, bounded effects, keyboard control, and
  server-confirmed damage/death.
- Add an M25 Godot validation harness and reproducible A/B capture path.

Gate 4 requires semantic fixture evidence and creator-reviewable captures; it
does not manufacture a creator preference.

Gate 4 is approved with bounded residuals assigned to Block 5.
`docs/combat/m25-block4-presentation-audit.md` records distinct local-attempt,
cooling, unavailable, authoritative player-hit, hostile-hit, and defeat
semantics; fixed audiovisual bounds; mute and Reduced Flash fixtures; isolated
reproducible A/B captures; and the complete post-change smoke. No creator
preference was supplied or inferred.

### Block 5 — Runtime matrix

- Run both weapons in solo encounters and two active local clients.
- Exercise 0, 75, and 150 ms simulated round-trip delay in a local bounded
  harness.
- Reconcile exact health, hit count, TTK, completion, reward, replay, and
  Inspector state.
- Repeat at least ten activities per selected final profile and check resource
  bounds and session reset.
- Record creator A/B notes only if the owner actually supplies them.

Gate 5 requires no integrity, fairness-budget, projection, compatibility,
privacy, or resource blocker in the bounded matrix.

Gate 5 is approved with bounded residuals for the final M25 review.
`docs/combat/m25-block5-runtime-matrix-audit.md` records 20 fresh sessions,
30 active clients, ten activities per weapon, solo and two-attacker paths,
0/75/150 ms delay envelopes, exact target/player health and TTK, per-account
rewards, replay/Inspector agreement, 20 resets, and bounded idle resources.
No creator preference was supplied or inferred.

### Block 6 — M25 closure

- Review the entire M25 diff and metric report.
- Run the uninterrupted canonical `make check`.
- Reconfirm Protocol V2, replay vocabulary, frozen V1 hashes, M17-M24 flows,
  `VERSION=0.2.0`, local-only services, no temporary injection, secret audit,
  `git diff --check`, and final status.
- Classify residual risks and decide Gate M25.

A green Gate M25 permits sequential work to continue to the already authorized
M26. A blocked gate stops that progression until the blocker is resolved or
the owner changes scope.

Gate M25 is approved with bounded residual risk. The complete decision and
evidence reconciliation are recorded in
`docs/combat/m25-closure-audit.md`. The uninterrupted canonical quality gate
and resume invariant audit passed without a reproducible integrity, fairness,
projection, compatibility, privacy, resource, or authority blocker. M26 may
proceed to its specification gate; this decision does not authorize module
implementation before that gate is green.

## Stop conditions

- Any reproducible damage after death/completion, cooldown bypass, duplicate
  damage, partial multiplayer reward, client-confirmed unaccepted hit, or
  server/client health divergence.
- Any need to silently change Protocol V2, replay vocabulary, V1 artifacts,
  version, persistence authority, or the frozen M24 package.
- Unbounded effect, descriptor, thread, memory, connection, or report growth.
- A tuning decision represented as external-player evidence without such
  evidence.

## Definition of done

M25 is complete when the laboratory and runtime evidence agree, the two
weapons retain measurable tradeoffs, enemy pressure is bounded and
authoritative, presentation distinguishes attempts from confirmed outcomes,
solo/two-client/latency matrices pass, creator preference is labeled honestly,
all compatibility gates remain green, and exactly one M26 starting hypothesis
is recorded without silently beginning it.
