# M25 Block 2 cadence and weapon-identity audit

Date: 2026-08-30  
Gate: 2 — authoritative cadence and weapon identity.

## Implemented candidate

- The Godot intent controller retains a safe 260 ms pre-snapshot default, then
  accepts only bounded 1-5,000 ms authoritative cooldowns.
- Equipment snapshot and accepted equipment change synchronize the local input
  gate and local cooldown cue to the equipped server profile.
- Changing profile never rewrites an already active local deadline. The server
  continues to own the canonical per-attacker deadline and already proves that
  switching to a faster profile cannot bypass it.
- Automatic, on-screen, and keyboard drivers terminate from authoritative
  actor/completion state rather than a fixed hit count.
- The fake client accepts `REVENANT_BOT_WEAPON=pulse_rifle|arc_sidearm` and
  sleeps for the cooldown returned in `EquipmentChanged`.
- Solo relay-drone/Warden health is 140/240. Each additional participant adds
  50% of base health, producing 210/360 for the supported two-player path.
- Weapon catalog values remain unchanged: rifle 40/6/250 and sidearm 25/8/150.
  Protocol V2, replay vocabulary, rewards, persistence schema, and V1 are
  unchanged.

## Deterministic candidate matrix

`revenant-combat-lab --candidate` emits 24 rows. RTT 0, 75, and 150 ms are at
or below both authoritative cooldowns and therefore do not extend the
candidate interval.

| Weapon | Enemy | Players | Health | Effective TTK | Solo ratio |
| --- | --- | ---: | ---: | ---: | ---: |
| Pulse rifle | Relay drone | 1 | 140 | 750 ms | 100% |
| Pulse rifle | Relay drone | 2 | 210 | 500 ms | 66.7% |
| Arc sidearm | Relay drone | 1 | 140 | 750 ms | 100% |
| Arc sidearm | Relay drone | 2 | 210 | 600 ms | 80.0% |
| Pulse rifle | Warden | 1 | 240 | 1,250 ms | 100% |
| Pulse rifle | Warden | 2 | 360 | 1,000 ms | 80.0% |
| Arc sidearm | Warden | 1 | 240 | 1,350 ms | 100% |
| Arc sidearm | Warden | 2 | 360 | 1,050 ms | 77.8% |

All solo timing and 45% multiplayer-floor budgets pass. The rifle retains the
larger, slower hit; the sidearm retains the smaller, faster, longer-range hit.
Neither dominates every catalog dimension.

## Runtime evidence

The canonical smoke passed the two-player driver/observer path; automatic,
reused-session, manual, and keyboard Godot paths; every M17-M24 marker;
replay/Inspector reconciliation; frozen V1 compatibility; and V1
reconstruction. The two-player server spawned 210-HP drone and 360-HP Warden;
the solo server spawned 140/240. Manual and keyboard drivers completed from
authoritative state with the sidearm's 150 ms local gate.

A separate accepted two-session run used local gateway ports 17001/18081:

| Account | Weapon/cooldown | Accepted hits | Result |
| --- | --- | ---: | --- |
| `local:m25b2r-194358-68870` | Pulse rifle / 250 ms | 4 drone + 6 Warden | 1 completion, 1 fragment, 100 XP |
| `local:m25b2s-194358-68870` | Arc sidearm / 150 ms | 6 drone + 10 Warden | 1 completion, 1 fragment, 100 XP |

The emitted damage sequences ended exactly at zero. No extra hit, reward, or
history row was created.

The first evidence script used an overlong synthetic username and was rejected
by the existing identity boundary before gameplay. A second harness attempt
called a stale plain bot executable after `cargo test`; it correctly completed
two sidearm sessions but was rejected as rifle evidence. An explicit
`cargo build` and fresh accounts produced the accepted run above. Neither
rejected attempt exposed a product defect.

## Automated coverage

- Five `revenant-combat` tests pass.
- Two combat-lab tests pass, including all candidate timing and multiplayer
  budgets.
- Eighteen gateway tests pass, including health scaling and all M24 failure,
  atomicity, ordering, and recovery fixtures.
- The M25 Godot cadence fixture proves exact 150/250 ms boundaries, invalid
  cooldown rejection, and no deadline rewrite across profile changes.
- Relevant Rust formatting and Clippy with warnings denied pass.

## Residuals entering Block 3

- The Warden still applies zero incoming damage.
- The drone still applies exactly one pre-combat 10-damage hit.
- The two-player smoke has one active attacker and one observer; two-active
  runtime timing remains Block 5 evidence.
- Range remains a catalog tradeoff but both current accepted-attack positions
  are distance 2, so this activity does not exercise the sidearm's extra range.
- Creator preference is still absent.

## Gate 2 decision

Gate 2 is **approved with bounded residuals assigned to later M25 blocks**.
Cadence, health scaling, weapon dimensions, drivers, compatibility, and
authoritative reward behavior are green. Block 3 may add fair enemy pressure;
it must not hide pressure inside the static lab or expand Protocol V2.
