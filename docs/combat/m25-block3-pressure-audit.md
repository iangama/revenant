# M25 Block 3 fair-pressure audit

Date: 2026-08-31  
Gate: 3 — bounded server-owned enemy pressure.

## Implemented rule

- `relay-drone`: one initial 10-damage attack after its authoritative chase,
  with no repetition.
- `warden`: no spawn attack; one 15-damage counterattack after each second
  accepted nonlethal player hit, capped at two counterattacks per encounter.
- The Warden is armed against the player who opens the relay door. A
  counterattack targets the player whose accepted hit crosses the two-hit
  boundary. In two-participant fixtures this rule is deterministic and does
  not redirect damage to another player after target loss.
- The accepted player `DamageApplied` message is broadcast before a possible
  hostile `DamageApplied`. A lethal hit takes the enemy-death/completion path
  directly and cannot trigger pressure.
- Missing, zero-health, wrong-kind, or destroyed actors cannot deal or receive
  hostile damage. A zero-health player can no longer submit an accepted combat
  attack.
- Encounter death, completion, disconnect reset, and persistence-abort reset
  discard the AI controller and its hit counters.

The runtime and deterministic lab share the same two pressure-profile
constants. No Protocol V2 field, replay event, persistence schema, reward,
activity script, V1 artifact, or version changed.

## Deterministic evidence

The candidate lab continues to emit 24 weapon/enemy/participant/RTT rows. For
both weapon profiles and both one/two-attacker cases it projects exactly 10
incoming damage from the drone and 30 from the Warden. The solo optimal total
is 40 of 100 HP, inside the approved 10-70 budget.

Domain fixtures prove:

- the drone reaches attack range, attacks once, and cannot repeat;
- the Warden attacks on accepted hits 2 and 4 only, then stops;
- the player whose hit crosses the boundary receives the counterattack in a
  two-player sequence;
- a 15-HP player reaches exactly zero with `killed=true` and receives no later
  damage;
- enemy death and target loss suppress damage while consuming the bounded
  opportunity;
- a defeated player cannot damage an enemy;
- the coordinator emits the exact rifle sequence
  `P40, P40, W15, P40, P40, W15, P40, P40(lethal)` and ends at 70 player HP;
- coordinator reset clears pressure state.

The relevant automated result is 3 AI tests, 6 combat tests, 2 combat-lab
tests, and 20 gateway tests, all green. Relevant Rust formatting and Clippy
with warnings denied also pass.

## Runtime evidence

The canonical smoke now asserts the gateway log instead of inferring pressure
from client completion. Each completed activity must contain exactly three
`enemy_attack_applied` records: one 10-damage drone hit, two 15-damage Warden
hits, and zero hostile `killed=true` records.

The two-client driver/observer activity passed with a 210-HP drone and 360-HP
Warden. The sidearm driver delivered 9 and 15 accepted hits respectively. The
owner/observer received the drone hit; the active driver received both Warden
counterattacks and ended that encounter at 70 HP. Both accounts received one
fragment and 100 XP in the same completion transaction.

Four clean-source Godot activities then passed the same exact pressure-count
assertion: automatic, reused-session, on-screen manual driver, and
keyboard-only driver. The solo path ends at 60 HP after the combined 10 + 30
incoming damage. M17-M25 validation markers, replay/Inspector reconciliation,
frozen V1 compatibility, and standalone V1 reconstruction all remained green.

The first smoke attempt exposed that Warden activation selected the first
participant rather than the door opener, leaving chase updates queued for the
active driver. The coordinator was corrected to arm against the door opener
and the full smoke passed. A separate clean-project parse rejected the new
Godot receive helper until its infinite loop gained an explicit terminal
return; that correction was also retested by the full smoke. Neither rejected
run is accepted as gate evidence.

## Residuals entering Block 4

- The pressure schedule is intentionally hit-count based, not a free-running
  wall-clock loop. It is deterministic and bounded but is not a claim of
  external-player preference.
- Deliberately lethal pressure leaves the player unable to attack until the
  existing disconnect/Retry-from-start path; a new death/respawn system is
  outside M25.
- Two active attackers, simulated delay, repetition, and resource bounds
  remain Block 5 runtime evidence.
- Existing presentation already follows authoritative hostile messages, but
  the semantic attempted/cooling/unavailable/confirmed cue fixture and A/B
  capture path remain Block 4.

## Gate 3 decision

Gate 3 is **approved with bounded residuals assigned to Blocks 4-5**. Optimal
pressure is nonlethal and inside budget, deliberately lethal cases stop at the
death boundary, targeting is deterministic, terminal/reset paths discard
pressure state, and no post-terminal or unavoidable damage was observed.
