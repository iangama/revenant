# M25 Block 5 runtime-matrix audit

Date: 2026-08-31  
Gate: 5 — solo, two-active-client, delay, repetition, and reconciliation.

## Accepted evidence set

The accepted matrix is stored outside the repository at:

`/mnt/c/Users/Ian/revenant-local-evidence/m25-runtime-162628-25628`

It contains 151,724 bytes across the two gateway logs, 30 client logs, 20
reconstructed replay logs, exact client and Inspector JSONL, resource/database
tables, aggregate TTK JSON, summary, and `SHA256SUMS`. The SHA-256 of the
manifest itself is
`eaa52e634ffd7c86709ae1a4234da681bdd46d84616c6e1569d816f57f4254c7`.
Every entry in that manifest passes `sha256sum --check`.

`tests/m25-runtime-matrix.sh` creates the set from fresh bounded account names.
It runs five sessions for each weapon at each participant count, using RTT
cases `0, 75, 150, 0, 0`. The result is:

- 20 completed sessions: 10 solo and 10 with two active attackers;
- 10 completed activities per final weapon profile;
- 30 fresh active client accounts and 30 participant completions;
- both weapons at 0, 75, and 150 ms simulated round trip;
- 20 exact disconnect resets across two gateway processes;
- 92 seconds total local matrix duration.

The bounded delay harness waits half the configured RTT before the first
intent, then uses `max(authoritative cooldown, RTT)` as each client's accepted
attack interval. It does not claim to be an operating-system network emulator.
All selected RTTs are at or below the authoritative 150/250 ms cooldowns, so
the expected accepted cadence remains unchanged while ordering, duplication,
projection, and reward behavior are still exercised with the delay envelope.

For two attackers, the primary and secondary use half-interval phase offsets.
Each retains its own full authoritative cooldown, both produce accepted damage
against both enemies, and the stagger removes a deliberately avoidable stale
post-lethal intent race.

## Exact combat results

Every active client observed and independently validated the same full target
health sequence. Required accepted hits were:

| Weapon | Players | Drone HP / hits | Warden HP / hits |
| --- | ---: | ---: | ---: |
| Pulse rifle, 40 damage | 1 | 140 / 4 | 240 / 6 |
| Arc sidearm, 25 damage | 1 | 140 / 6 | 240 / 10 |
| Pulse rifle, 40 damage | 2 | 210 / 6 | 360 / 9 |
| Arc sidearm, 25 damage | 2 | 210 / 9 | 360 / 15 |

Each sequence ended at exactly zero, preserved catalogued per-hit damage, and
contained no duplicate or skipped remaining-health value. In every two-player
session both actor IDs appear in the accepted-hit map for the drone and the
Warden, and the two clients emitted byte-equivalent shared combat evidence
after removing only account/role identity.

Every session contained exactly one 10-damage drone attack and two 15-damage
Warden attacks, with no lethal hostile hit. Solo health therefore followed
`100, 90, 75, 60`. In the deterministic staggered two-player path, the owner
received the drone hit and the other active attacker received the two Warden
counters, leaving both alive at 90 and 70 HP respectively.

## Measured runtime timing

TTK is measured from receipt of the first accepted authoritative hit through
receipt of the lethal accepted hit. The table reports the accepted minimum to
maximum across all RTT and repeat cases:

| Weapon | Players | Drone TTK | Warden TTK |
| --- | ---: | ---: | ---: |
| Pulse rifle | 1 | 877–881 ms | 1,469–1,483 ms |
| Arc sidearm | 1 | 961–966 ms | 1,736–1,744 ms |
| Pulse rifle | 2 | 710–713 ms | 1,181–1,186 ms |
| Arc sidearm | 2 | 813–817 ms | 1,445–1,453 ms |

All raw solo runs remain inside the approved 600–1,200 ms drone and
1,200–2,600 ms Warden budgets. The measured two/solo average ratios across all
weapon/RTT/enemy combinations range from 79.77% to 85.01%, above the 45%
anti-collapse floor. The 0/75/150 ms cases retain exact hit counts and remain
within a few milliseconds of their same-profile group; no delay-dependent
duplicate, cooldown bypass, reordering, health divergence, or reward change
occurred.

The rifle retains the larger/slower hit while the sidearm retains the
smaller/faster/longer-range hit. A rejected pre-gate run left the secondary
rifle user at `[0,0,0]` when the Warden spawned at `[8,0,0]`. The server
correctly rejected the range-8 rifle intent because its range is 6, while the
earlier sidearm rehearsal could attack at that same distance with range 8.
The accepted matrix makes both active players perform the authoritative door
movement, exercises two real attackers in the intended encounter position,
and logs zero rejected command. The rejected run is useful range-boundary
diagnosis but is not included in the accepted evidence.

## Reward, replay, and Inspector reconciliation

Each of the 20 session IDs was resolved from the primary account's persisted
`player_joined` event; both accounts in every two-player run resolved the same
ID. The CLI reconstructed every session as:

- one activity start and completion;
- one drone spawn, two enemy deaths, and one Warden spawn;
- one equipment change, loot grant, and progression grant per participant;
- `completed=true` with the expected participant count.

The Inspector summary and event APIs independently matched those same counts
for every session, with non-null join-to-start and activity durations. Across
the accepted set, activity duration was 2,942–5,015 ms (3,475 ms average) and
join-to-start was 1–102 ms (50 ms average).

The final PostgreSQL aggregation matched every exact expected value:

| Assertion | Expected | Actual |
| --- | ---: | ---: |
| Fresh accounts | 30 | 30 |
| Activity histories | 30 | 30 |
| Resulting fragments | 30 | 30 |
| Resulting XP | 3,000 | 3,000 |
| Inventory reward grants | 30 | 30 |
| Progression reward grants | 30 | 30 |
| Distinct replay sessions | 20 | 20 |
| Sessions with any join/completion/reward/equipment/death/boss mismatch | 0 | 0 |

## Resource and reset bounds

After ten sessions at each participant count, the gateway returned to the
following idle measurements:

| Mode | RSS before / after | Descriptors | Threads | PostgreSQL connections |
| --- | ---: | ---: | ---: | ---: |
| Solo | 6,144 / 6,656 KiB | 9 / 9 | 3 / 3 | 3 / 3 |
| Two active | 6,144 / 7,364 KiB | 9 / 9 | 3 / 3 | 3 / 3 |

RSS growth was 512 KiB and 1,220 KiB, far below the 64 MiB bound. No descriptor,
thread, or database-connection growth remained. Both logs contain ten resets
and zero `session_command_failed`, `connection_failed`, or session-abort event.

## Rejected harness attempts

Two full or partial runs were deliberately excluded:

1. The first matrix reached its initial two-rifle session with the secondary
   actor still at the spawn point. The authoritative range rejection and the
   missing second-attacker hit caused the client fixture to fail. Both bots now
   execute the door movement, and a fresh two-rifle rehearsal passed with two
   moves, both hit maps populated, zero command failure, and one reset.
2. The next fresh matrix completed and reconciled all 20 sessions, but its
   final `jq` solo-budget reduction used the wrong scalar context and exited
   before creating the aggregate/manifest. The corrected expression passed
   against that raw JSONL, then the entire command was repeated with new
   accounts to produce the accepted set above.

These attempts added controlled local accounts and completed rewards where the
server had legitimately completed an activity; none was rewritten or deleted.
They are not counted as Gate 5 evidence.

## Residuals entering Block 6

- RTT is a deterministic client timing envelope over loopback, not a claim
  about packet loss, jitter, congestion, or remote networking.
- Ten sessions per profile and bounded idle-resource deltas are engineering
  evidence, not long-duration soak or population evidence.
- The creator supplied no A/B preference; weapon timing differences remain
  measured facts only.
- Real human comprehension, perceptual feel, external networking, gamepad, and
  non-creator hardware remain outside M25.

## Gate 5 decision

Gate 5 is **approved with bounded residuals for the final M25 review**. Exact
combat, pressure, participation, cadence, delay, completion, per-account
reward, replay, Inspector, reset, compatibility boundary, and idle-resource
invariants pass without an integrity or fairness blocker.
