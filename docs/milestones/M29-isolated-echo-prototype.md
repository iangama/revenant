# M29 — Isolated Echo prototype

Status: **complete by evidence-based early removal; Gate M29 green**. Gate 1
froze the finite proposition and Gate 2 executed the exact pure laboratory.
The numeric keep threshold failed, so the isolated implementation was removed
before persistence, replay, protocol, gateway, bot, Godot, or working-database
integration. The decision is audited in
`docs/operations/m29-block2-echo-lab-audit.md`.

## Authorization and prototype boundary

The owner authorized sequential work through the existing M25–M32 roadmap.
M28 is complete and Gate M28 is green, so M29 may proceed one gate at a time.
M29 is deliberately a removable creator-only prototype. A green outcome may
be either retaining the isolated feature with bounded residuals or deleting
all M29 implementation and proving exact M28 restoration.

M29 does not authorize a production feature, a new activity or area, an enemy
or item catalog expansion, a Protocol V3, a default-enabled wire surface, a
working-database migration, public networking, matchmaking, accounts, cloud
storage, input telemetry, commit, push, merge, version change, tag, release,
or distribution. `VERSION=0.2.0`, the frozen V1 tree, the M24 package, M25/M26
combat/module arithmetic, M27 routes/events/Lua revision, and the complete M28
cooperation contract remain protected.

Before this specification, the exact M28 source set was archived at
`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m28-closure-45718926-20260907.tar.gz`:

- source/archive entries: `386` / `386`;
- path/content differences after restore: zero;
- archive size: `10,846,955` bytes; and
- SHA-256: `8077c650455b61214ef7c843e6e5767f7a1a85c473bd210c87ba296b19edb4d9`.

The sidecar checksum and `gzip -t` pass. No M29 implementation source changed
before Gate 1.

## Research basis

The design uses external references only as boundary guidance, not as copied
gameplay or engine code:

- Godot's official
  [secure multiplayer guidance](https://docs.godotengine.org/en/4.7/tutorials/networking/high_level_multiplayer.html)
  says gameplay-critical position, combat, inventory, and outcome must remain
  server-authoritative and client input must be validated. Therefore Echo
  intents contain no position, target, damage, timing, or recorded sequence.
- Epic's official
  [Replay System documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-the-replay-system-in-unreal-engine)
  reconstructs from replicated network truth. The bounded inference here is to
  store canonical accepted outcomes, never raw local key/mouse events.
- Godot's official
  [physics-interpolation guidance](https://docs.godotengine.org/en/stable/tutorials/physics/interpolation/physics_interpolation_introduction.html)
  notes that server/network timing may not align with local render ticks.
  Therefore Godot may smooth confirmed Echo motion but cannot advance its
  gameplay clock or predict its attack.
- Rust documents
  [`Instant`](https://doc.rust-lang.org/stable/std/time/struct.Instant.html) as
  a monotonically nondecreasing duration clock. The isolated gateway uses the
  same monotonic-observation pattern already accepted for M27/M28 and never a
  wall timestamp for gameplay decisions.

## Audited M28 baseline

- The shared gateway admits one or two participants. Admission index zero is
  the owner/leader. Sessions start only at the configured participant count;
  they retain one server-owned `SessionStage` and reset after the last
  participant leaves.
- `MoveIntent` contains only an integer `[x,y,z]`. The server accepts `y=0`
  and `|x|,|z|<=12`, updates the actor registry, and broadcasts `ActorUpdate`.
  Movement is not currently persisted or represented in replay.
- `AttackIntent` contains only the target actor ID. The gateway validates the
  active stage/enemy, participant/life, admitted weapon profile, range, and
  cooldown before `CombatRuntime` applies damage. Accepted attacks are not
  independently replayed; durable evidence records encounter spawn/death and
  terminal/reward facts.
- The baseline Warden spawns at `[8,0,0]` with 240 solo or 360 two-player
  health. It counters every second accepted nonlethal hit for 15 damage, at
  most twice, targeting the participant who caused the threshold hit. M27 can
  alter this only inside a selected route; M29 excludes every route.
- There are exactly 15 canonical M26 loadouts and two weapons, yielding 30
  admitted effective profiles with damage 18–52, range 4–10, cooldown
  120–350 ms, and maximum health 100–130. Active-session module state is
  immutable; weapon equip can otherwise change the admitted active weapon.
- `ActorKind` has only Player and Enemy. Godot renders server `ActorSpawn`,
  `ActorUpdate`, `DamageApplied`, and `ActorDestroy`; it stores no authoritative
  actor simulation and currently recognizes no Echo kind.
- Protocol V2 has 17 client and 31 server variants after M28, a 64 KiB frame
  ceiling, and private 8 KiB route/cooperation ceilings. Frozen V1 is adapted
  but cannot express modules, routes, cooperation, or an Echo capability.
- Replay recognizes 21 event kinds: nine original activity/combat kinds,
  three module kinds, three route kinds, and six cooperation kinds. It has no
  raw movement, attack-intent, damage, or Echo event.
- PostgreSQL has sixteen public tables. The default migration list applies
  through additive `0008`; the working database has no prototype schema.
  Generic activity completion grants the unchanged baseline one fragment and
  100 XP per current participant atomically.
- M27 selection and M28 cooperation both occur between Drone and Door. Their
  immutable balance, objectives, rewards, replay, and client projections may
  not absorb or depend on an Echo.
- The current Godot presentation already has an Operator, two weapon forms,
  server-confirmed effects, distinct Drone/Warden silhouettes, Arrival Deck to
  Relay Core staging, and authoritative fragment collection. An Echo can reuse
  those materials/effects but cannot imply a new world, enemy, item, or NPC AI.

The critical architectural gap is explicit: replaying a local input buffer
would bypass authoritative acceptance, while replaying the mutable current
world would not be independently reconstructible. The prototype must capture
one tiny canonical sequence only after the gateway has accepted its ordinary
movement and attack.

## Single falsifiable proposition

> A removable, server-authoritative Echo that records exactly one accepted
> owner movement followed by one accepted nonlethal Warden attack and can
> execute that recorded attack once after a bounded server-timed playback can
> create a measurable early-damage versus held-finisher decision without
> owning rewards, objectives, enemy decisions, cooperation roles, or raw
> input, while authority, lifetime, persistence/replay evidence, compatibility,
> exploit limits, and total removal remain finite and independently
> reconstructible.

This is the only M29 proposition. “Create a decision” means deterministic
analysis must expose both a faster early-use outcome and a lower/differently
assigned pressure held-use outcome in a material subset of two-player profile
pairs. It does not mean that another person would notice, understand, prefer,
or enjoy the mechanic.

## Exact keep/remove criterion

Block 2 must generate three strategies for every allowed encounter profile:
baseline without Echo, execute immediately when ready, and hold the Echo as a
finisher. The profile space is exactly:

- 30 solo owner profiles;
- 900 ordered two-player owner/peer profile pairs; and
- three strategies for each, for exactly 2,790 report rows.

The prototype proceeds beyond pure code only if all of these are true:

1. every successful Echo execution applies exactly one duplicate of the
   recorded accepted owner damage and reduces direct participant attacks to
   defeat the baseline Warden by exactly one;
2. no row changes weapon/module arithmetic, Warden health, the two-counter
   ceiling, activity reward, objective path, or produces a lethal optimal
   participant-pressure path;
3. immediate use improves modeled completion time by at least one admitted
   owner cooldown in at least 24 of the 30 solo profiles; and
4. immediate and held-finisher use are both non-dominated on the vector
   `(completion_ms, owner_hostile_damage, peer_hostile_damage)` in at least 60
   of the 900 ordered two-player pairs.

Any arithmetic/invariant failure, fewer than 24 solo improvements, fewer than
60 two-player tradeoff pairs, or a result where “use immediately” simply
dominates every valid context ends implementation after Gate 2. The pure M29
files are then removed, the M28 checkpoint is compared byte-for-byte, and
early removal is a successful Gate M29 result.

If the pure threshold passes, later creator dogfood still retains the prototype
only if the complete mechanic can be read without local gameplay inference and
at least two bounded use timings remain observable across the frozen scenario
set. Ambiguous presentation, default-surface leakage, working-database change,
or complexity outside this contract requires removal rather than scope growth.

## Isolation and removability contract

All implementation remains behind the explicit Cargo feature
`m29-echo-prototype`, disabled by default. The feature may compose a separately
named prototype gateway, bot, lab, persistence seam, replay decoder, and wire
variants. Default `revenant-gateway`, permanent Compose services, default
Protocol V2 builds, current working PostgreSQL, and ordinary Godot startup must
contain no active Echo behavior.

Prototype persistence uses only a checked-absent disposable database named
with a bounded run token. Its separately invoked prototype migration is not
added to the default `MIGRATIONS` list and never runs against the working
database. The database is backed up/evidenced, destroyed after accepted
testing, and reproducibly recreated. If M29 is removed, every feature-gated
source/composition seam is deleted and the repository is compared to the M28
checkpoint; no empty working-schema remnant is tolerated.

The default release Gateway must exclude Echo environment names, message/event
names, banners, and migration strings under static `strings` inspection. The
prototype binary must refuse startup unless its feature, explicit opt-in
environment flag, disposable database name, and loopback addresses are all
present. No network listener may bind beyond loopback.

Godot uses an explicit local prototype launch flag and otherwise exposes no
Echo entry, key, actor, state, or copy. Its files remain grouped behind one
small main/session/projection composition seam so removal is mechanical and
diff-verifiable.

## Eligibility and coexistence

Echo capability is explicit and session-local. One participant in a solo
session or both participants in a two-player session must separately request
prototype state using current Protocol V2. Identity, build string, weapon,
module loadout, movement, route/cooperation capability, or prior use never
implies Echo capability.

Exactly one Echo operation may exist per baseline session. Its owner is always
admission index zero. It can be armed only when:

- the current enemy is the baseline Warden in `SessionStage::Boss`;
- no accepted Warden hit has occurred yet;
- every current participant is Echo-capable current V2;
- no M27 route was selected and its route baseline is already locked;
- no M28 cooperation operation exists; and
- the owner is admitted, active, and supplies a valid operation ID.

Solo and ordinary two-player baseline activities are eligible. Any V1/mixed,
partly capable, routed M27, M28 cooperation, Drone/Door/Stabilizer,
complete/failed, late join, or second-operation state is ineligible and
receives no optimistic mutation. M27 and M28 behavior is not changed to make
Echo fit. The two-player peer observes the state but cannot arm or use it.

If a nonowner disconnects, current baseline session behavior and the owner's
Echo continue. Owner disconnect in any nonterminal Echo phase durably produces
`abandoned_owner_disconnect` before actor removal. Target loss produces
`invalidated_target`. The last participant leaving performs the existing clean
session reset. There is no reconnect, replacement owner, ownership transfer,
cross-session Echo, or persisted ability unlock.

## Canonical record and playback

The client supplies only an arm operation ID and later a use operation ID,
each 1–32 ASCII alphanumeric/hyphen bytes. It never supplies the Echo actor,
sequence, position, target, weapon, profile, damage, clock, phase, terminal, or
visual path.

An accepted arm snapshots from authoritative state:

- session/activity and owner account/character/actor identity;
- server-allocated Echo actor ID;
- owner origin position;
- current baseline Warden actor ID and health;
- participant admission order; and
- arm monotonic elapsed time.

While recording, the owner explicitly accepts a temporary action constraint:

1. in `recording_move`, only the owner's next otherwise-valid `MoveIntent` may
   mutate owner position; it is applied normally and captured as the one Echo
   destination;
2. in `recording_attack`, only the owner's next otherwise-valid attack against
   that same Warden may mutate combat; it is applied through unchanged combat
   validation and captured only if nonlethal; and
3. owner equip, additional movement, out-of-order attack, route, cooperation,
   module mutation, arm, and use attempts reject without base or Echo mutation.

Peer ordinary movement/attack remains unchanged. The accepted recorded attack
still damages the live Warden, consumes the owner's normal cooldown, advances
the existing Warden accepted-hit counter, and may cause its existing counter
against the owner. Echo recording adds no damage by itself. A lethal would-be
record attack completes ordinary combat but invalidates the Echo; it cannot be
retroactively duplicated.

Once ready, owner and peer gameplay is ordinary again. Use revalidates the
same live Warden, recorded destination/range/profile, owner presence/life,
phase, deadline, and unused use ID. It creates one ephemeral `ActorKind::Echo`
with archetype `operator-echo`, health/max health one, no participant identity,
no collision/AI/target/reward/history rights, and no input channel.

At the server-owned playback boundaries the Echo:

1. spawns at the recorded origin on accepted use;
2. moves to the recorded destination at exactly 250 ms; and
3. at exactly 500 ms, if the Warden is still valid, applies the exact recorded
   damage once, advances the unchanged Warden hit counter once, directs any
   resulting existing counterattack to the Echo owner, and is destroyed.

If the Echo damage kills the Warden, the existing enemy death, objective,
activity completion, and equal baseline rewards run normally after durable
Echo execution. Echo has no bespoke reward and cannot change who receives the
activity reward. If another accepted attack defeats the Warden before 500 ms,
the Echo is destroyed as `invalidated_target` with zero Echo damage.

The Echo cannot receive or deal Drone damage, move the owner, reset player
cooldown, bypass range, retarget, copy a peer, repeat, split damage, critical
hit, heal, revive, collect an item, open a door, advance M27/M28, or persist as
an autonomous companion.

## Fixed lifecycle and clocks

The only nonterminal phases are:

`recording_move → recording_attack → ready → playing`

The only terminals are:

`executed`, `expired_recording`, `expired_ready`, `invalidated_target`, and
`abandoned_owner_disconnect`.

Arm begins a 5,000 ms monotonic recording deadline. Both the movement and
recorded attack must be accepted at or before 5,000 ms; the first observation
after 5,000 ms produces `expired_recording`. Ready begins a separate 15,000 ms
deadline; use at or before 15,000 ms is eligible and the first later
observation produces `expired_ready`. Playback applies movement at use+250 ms
and damage/destruction at use+500 ms. At most 20,500 ms can elapse from arm to
successful terminal.

All comparisons use checked integer milliseconds. Exact boundary is accepted;
plus one millisecond expires. Gameplay commands/state requests observe clocks
before their requested mutation, matching M27/M28 ordering. Godot never owns a
countdown. System wall timestamps are audit metadata only.

Same arm/use ID with identical canonical input returns the durable receipt.
Same ID with conflicting input, a new ID after accepted arm/use, or any
post-terminal operation rejects without extending a deadline, respawning the
Echo, or applying damage. Recording actions themselves are identified by their
unique position in the finite lifecycle and can apply only once.

## Prototype persistence and replay

If Gate 2 passes, Block 3 may review an exact separate migration for one
`echo_operations` table. It stores the immutable identities/profile, origin,
destination, target, arm/record/use/playback elapsed times, operation IDs,
phase, one damage, and terminal. It stores no raw input stream, arbitrary
array, client timestamp, inventory, reward, key code, mouse coordinate, or
cross-session payload. SQL constraints must encode all constants, prefix
validity, monotonic ordering, terminal evidence, and one-use cardinality.

Exactly three feature-gated replay kinds are permitted:

- `echo_armed` — immutable authority, origin, target, constants, and profile;
- `echo_recorded` — the one accepted movement destination and nonlethal attack
  evidence; and
- `echo_terminal` — typed terminal, optional use/playback/one-damage evidence,
  owner/target/Echo identities, and zero bespoke reward.

Each payload has schema version one and a 4,096-byte private bound. Operation
transition and replay append are one transaction. `executed` must precede any
generic Warden death/completion/reward events caused by Echo damage. Failures
have no Echo damage or generic completion/reward. Reconstruction reads replay
only, accepts incomplete prefixes, rejects skipped/duplicate/post-terminal or
route/cooperation coexistence, and never consults the prototype table, current
actors, modules, inventory, Lua, or clock.

The feature-gated CLI/Inspector may expose GET-only summary/events for the
disposable database. Default Inspector is not required to understand
prototype-only evidence and must never point at that database.

## Prototype protocol, gateway, bot, and Godot

If Gate 2 passes and Block 3's wire review is green, the feature may add only
three client variants—state request, arm intent, use intent—and three server
variants—state, arm result, use result. Existing ActorSpawn/Update/
DamageApplied/Destroy project the ephemeral entity and confirmed attack.
Feature-off serialization remains byte-identical. Maximum-shaped messages
must fit a private 8 KiB ceiling and the unchanged 64 KiB frame.

Every result includes schema/session correlation and bounded accepted,
replayed, rejected, pending, or terminal language. State exposes complete
server facts only after commit. Malformed/oversized/unknown data rejects before
allocation or mutation. V1 and unopted V2 receive no Echo variant or Echo actor.

The isolated gateway composes the sidecar around existing accepted movement,
combat, AI, completion, reset, and persistence seams. It cannot copy client
messages before validation or call client presentation logic. Persistence
failure exposes no actor, phase, attack, terminal, or reward and uses the
existing fail-closed session abort.

One bounded deterministic Echo bot may request state, arm, submit the exact
ordinary move/attack, choose early or held use, observe clocks, retry IDs, and
verify shared truth. It cannot set a sequence, actor, damage, target, time,
terminal, or reward.

Godot may expose a neutral `ECHO [E]` surface only in an explicit prototype
launch. It shows capability/owner, recording instructions, authoritative
deadlines, ready/use/pending/terminal state, and the confirmed translucent
Operator path. Color is redundant with text/shape; Reduced Flash removes no
facts. Animation interpolates confirmed origin/destination/timestamps, while
damage, counter, target death, reward, and terminal remain server events. A
malformed or contradictory server sequence clears partial truth and displays
`INVALID SERVER DATA`.

## Exact pure and runtime evidence

Block 2 implements only a pure `revenant-echo-prototype` domain and lab. The
2,790-row value report is generated twice and byte-compared. A separate exact
50-case lifecycle report contains:

| Category | Cases |
| --- | ---: |
| Valid execution, arm/use retry/conflict/second/post-terminal behavior | 10 |
| Capability, owner, V1, stage, M27, and M28 eligibility/exclusion | 10 |
| Action ordering, peer isolation, target, lethality, and profile immutability | 10 |
| Recording/ready/playback exact and plus-one timing boundaries | 10 |
| Target loss in four phases, owner disconnect in four phases, peer disconnect, reset | 10 |

Every row has a stable input/output schema, complete fields, and checked
arithmetic. The lab must retain existing M25/M26/M27/M28 report hashes.

Later accepted runtime evidence, if authorized, must include all 30 solo owner
profiles and a deterministic bounded subset covering each owner/peer weapon,
Empty/Force/Ward/Force+Ward builds, both early/held strategies, solo/two-player,
same/conflict/second IDs, every terminal, peer/owner disconnect, target loss,
ordinary V2, M27, M28, current V1, frozen V1, replay/Inspector/SQL parity,
reset, repetition, resources, default-binary string exclusion, and disposable
database destruction. Exact sample counts are frozen before the harness.

All evidence is external, uniquely named, checksum-verifiable, loopback-only,
and contains synthetic identities only. No existing target is overwritten or
promoted.

## Block plan and gates

### Block 1 — specification and removal gate

- Audit current actor/movement/combat/AI/session/disconnect, M26 profiles,
  M27/M28 coexistence, persistence/replay/Inspector, Protocol V2/V1, Godot,
  default builds, working database, and protected checkpoint.
- Freeze the single proposition, exact mechanic, value threshold, isolation,
  lifetime, authority, evidence, removal, and stop boundaries.

Gate 1 requires one finite falsifiable prototype and zero M29 implementation
source change.

### Block 2 — pure prototype and value laboratory

- Implement the side-effect-free lifecycle and checked combat scheduler.
- Generate the exact 2,790-row strategy report and 50-case lifecycle report.

Gate 2 requires every invariant and all four numeric value thresholds. Failure
removes the pure prototype and closes M29 as an evidence-based removal.

### Block 3 — isolated integration

- Review exact disposable schema, three replay payloads, six wire variants,
  feature graph, and composition seam before implementation.
- Integrate only the feature-gated prototype binary/database/client and prove
  transactions, reconstruction, compatibility, failure withholding, and total
  default exclusion.

Gate 3 requires no working-schema/default-binary drift and a mechanically
complete removal path.

### Block 4 — presentation and creator dogfood

- Add feature-launched honest Godot presentation and deterministic Echo bots.
- Execute the frozen solo/two-client strategy, terminal, compatibility,
  repetition, and resource matrix with creator-reviewed captures/runs.

Gate 4 requires readable confirmed states, no client-owned outcome, exact
application parity, and the retained value signal without human claims.

### Block 5 — keep or remove

- Review every M29 diff and artifact against this contract and the M28
  checkpoint.
- Either retain the isolated off-by-default prototype with bounded residuals,
  or delete all M29 implementation, destroy disposable state, and prove exact
  M28 restoration.

Gate M29 requires an explicit keep/remove decision, uninterrupted canonical
quality gate, protected-invariant audit, and exactly one bounded M30 starting
proposition. Neither outcome promotes Echo to the default product.

## Stop conditions

- More than one Echo/session, more than one recorded movement/attack, more than
  one execution/damage, copying a peer, ownership transfer, autonomy, AI,
  targeting, health, collision, inventory, item collection, revive, objective,
  reward, cross-session reuse, upgrade, cooldown reset, or repeatability.
- Any client-authored sequence, coordinate, target, actor, damage, profile,
  timer, phase, terminal, reward, raw input/key/mouse log, wall-clock gameplay
  decision, arbitrary string/array, unbounded payload, or local Godot outcome.
- M27 route or M28 cooperation coexistence, changed Warden/weapon/module
  arithmetic, new Lua objective, new activity/world/enemy/item, Protocol V3,
  unsolicited old-V2/V1 message, or frozen-V1 change.
- Default-enabled feature, default migration, working-database DDL/data,
  permanent-service replacement, nonloopback listener, external account/
  service, proprietary asset/code, or incomplete removal seam.
- Partial operation/replay/damage state, damage without durable execution,
  duplicate/post-terminal mutation, reward on Echo failure, target retargeting,
  persistence optimism, replay from mutable state, or resource growth outside
  a frozen later bound.
- Lowering the numeric keep threshold, adding cost/reward/content to manufacture
  value, widening the mechanic after a weak lab result, or describing creator
  automation as human comprehension, enjoyment, preference, accessibility, or
  social behavior.

## Gate 1 decision

Gate 1 is **approved**. The M28 checkpoint is complete, the audited baseline
exposes the exact acceptance/replay gap, and the proposition, one-move/
one-attack mechanic, five lifecycle terminals, 5,000/15,000/250/500 ms clocks,
one-damage authority, M27/M28 exclusion, feature/disposable-database isolation,
2,790/50-case evidence, numeric keep threshold, complete-removal path, block
gates, and stop conditions are finite and falsifiable.

That decision authorized Block 2 alone to implement the pure prototype and
deterministic lab. Persistence, replay, protocol, gateway, bot, Godot, default
build, working database, and product content remained locked behind the value
threshold and were never opened.

## Gate 2 and Gate M29 decision

The side-effect-free prototype and laboratory generated the exact 2,790 value
rows and 50 lifecycle cases twice with byte-identical output. All lifecycle
cases and all 2,790 baseline/pressure invariants passed. Immediate use improved
all 30 solo profiles by the required cooldown.

The keep proposition nevertheless failed:

- only 1,560 of 1,805 successful executions reduced direct participant attacks
  by exactly one;
- held use did not execute in 55 encounter shapes;
- immediate use dominated held use in 845 of 900 ordered two-player pairs;
- held use dominated immediate use in zero pairs; and
- zero pairs met the required two-strategy non-dominated tradeoff, below the
  frozen threshold of 60.

Gate 2 is therefore rejected as a keep gate. The predeclared removal path was
executed without widening or retuning the mechanic. All temporary source and
workspace/lock entries were removed. The 235-file implementation source set,
`Cargo.toml`, `Cargo.lock`, `VERSION`, `README.md`, and `Makefile` compare
byte-identically to the M28 closure checkpoint, with zero Echo path/string
remaining. The uninterrupted restored `make check` passes all 217 tests and
the complete Rust, PostgreSQL, Inspector, secret, gameplay, Godot, replay, and
frozen/reconstructed-V1 gate.

Gate M29 is **green by complete removal**. Blocks 3 and 4 were never
authorized or executed. M30 Gate 1 may begin only from the single bounded
security/recovery proposition recorded in the Block 2 audit; M31 content
expansion remains locked behind M30.
