# M25 untuned combat baseline

Date: 2026-08-30  
Schema: `RevenantCombatLabV1`  
Baseline: `m25_pre_tuning`

## Reproduction

```bash
cargo run --quiet -p revenant-combat-lab
cargo run --quiet -p revenant-combat-lab -- --json
```

The CLI builds 24 rows from two authoritative weapon profiles, two current
enemies, one/two active attackers, and simulated round-trip delays of 0, 75,
and 150 ms. The pure analysis API rejects invalid damage, range, cooldown,
health, distance, attacker count, hostile pressure, and arithmetic overflow.

The model includes the current 260 ms client input gate. Effective interval is
the maximum of server cooldown, client input interval, and simulated round-trip
delay. The relay-drone range measurement uses squared distance 4 because the
coordinator finishes its chase and first hostile hit before processing player
attack commands; squared spawn distance 20 is not the accepted-attack range.

## Key solo rows at zero simulated delay

| Weapon | Enemy | Hits | Server TTK | Effective TTK | DPS | Overkill | Range margin² | Incoming |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Pulse rifle | Relay drone | 3 | 500 ms | 520 ms | 160.000 | 20 | 32 | 10 |
| Pulse rifle | Warden | 3 | 500 ms | 520 ms | 160.000 | 0 | 32 | 0 |
| Arc sidearm | Relay drone | 4 | 450 ms | 780 ms | 166.666 | 0 | 60 | 10 |
| Arc sidearm | Warden | 5 | 600 ms | 1,040 ms | 166.666 | 5 | 60 | 0 |

RTT 0, 75, and 150 ms produce the same effective rows because each value is at
or below the current client gate/server cadence. That is a deterministic timing
result, not runtime network-integrity evidence.

## Two-active-attacker projection

| Weapon | Enemy | Solo effective TTK | Two-attacker effective TTK | Ratio |
| --- | --- | ---: | ---: | ---: |
| Pulse rifle | Relay drone | 520 ms | 260 ms | 50.0% |
| Pulse rifle | Warden | 520 ms | 260 ms | 50.0% |
| Arc sidearm | Relay drone | 780 ms | 260 ms | 33.3% |
| Arc sidearm | Warden | 1,040 ms | 520 ms | 50.0% |

The 33.3% sidearm/drone case fails the M25 45% floor. The other values are
mathematical projections and still require two-active-client runtime evidence.

## Hypothesis result before tuning

| M25 hypothesis | Baseline result | Evidence |
| --- | --- | --- |
| Authoritative cadence | Fails | Client uses 260 ms for both announced 250/150 ms profiles |
| Weapon identity | Structurally distinct, operationally blurred | Rifle has larger hit; sidearm has faster server cadence and longer range, but the client suppresses the cadence and both encounters occur at distance 2 |
| Drone solo TTK 600-1,200 ms | Mixed | Rifle 520 ms fails; sidearm 780 ms passes only because of the mismatched client gate |
| Warden solo TTK 1,200-2,600 ms | Fails | 520/1,040 ms effective |
| Incoming damage 10-70 | Mixed | Drone contributes 10; Warden contributes 0 |
| Two-attacker ratio at least 45% | Mixed | Sidearm/drone is 33.3% |
| Honest confirmation | Mostly passes with one mismatch | Hit/death follow server messages; local cooldown duration is fixed rather than profile-derived |

## First tuning candidate

Gate 1 selects this bounded candidate for Block 2/3 evaluation:

1. Derive the client input/cooldown cue interval from the equipped
   authoritative profile; retain the server's shared cooldown across equipment
   changes.
2. Preserve the current weapon profiles initially. Their catalog already
   expresses a large-hit/slow/short-range rifle and a small-hit/fast/long-range
   sidearm. Revisit numbers only if the candidate matrix fails.
3. Evaluate base health 140 for the relay drone and 240 for the Warden. With
   profile-derived cadence this projects 750/750 ms for the drone and
   1,250/1,350 ms for the Warden.
4. Evaluate 150% enemy health for two participants. Projected two-active TTK is
   500-600 ms for the drone and 1,000-1,050 ms for the Warden, or roughly
   67-80% of the solo baseline.
5. Evaluate deterministic pressure of one 10-damage drone hit and two
   15-damage Warden hits in an optimal solo completion. Exact server scheduling
   remains a Block 3 design problem and will not be hidden inside the static
   model.

These values are a falsifiable engineering candidate, not a claim that they
feel good. Runtime evidence may reject them.

## Gate 1 decision

Gate 1 is **approved**. The lab describes the current constants, client gate,
accepted-attack distance, multiplayer volley arithmetic, and current pressure
without changing gameplay. Five combat tests, one CLI matrix test, format, and
Clippy with warnings denied pass. Block 2 may implement the selected candidate
while preserving every stop condition in the M25 milestone.
