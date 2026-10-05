# M28 — Local cooperation simulation

Status: complete. Gates 1-8 and Gate M28 are green with bounded residual risk
under `docs/operations/m28-closure-audit.md`. M29 may begin only with its Gate
1 specification/removal review.

An owner-directed, bounded Relay Hub enrichment pass is green under
`docs/art/m28/relay-hub-enrichment-audit.md`. It improves the existing region's
visibility, hierarchy, cooperation landmarks, and recording presentation but
does not satisfy or broaden the active Block 7 cooperation contract.

A second owner-directed gameplay-depth presentation pass is green under
`docs/art/m28/relay-hub-gameplay-depth-audit.md`. It improves the existing
Drone and Warden silhouettes, stages Arrival Deck to Relay Core on accepted
server state, and visualizes the already-authoritative fragment grant. It adds
no new world, activity, enemy archetype, item authority, or M31 claim, and it
also does not satisfy or broaden the active Block 7 cooperation contract.

## Authorization and evidence boundary

The owner authorized sequential execution of the already-sketched solo
milestones M25-M32. M27 is complete and Gate M27 is green, so M28 may proceed
one reviewed block at a time. This milestone is creator-only local engineering
work using two loopback clients and deterministic role bots. It does not
involve another person and cannot establish social quality, communication
quality, anti-griefing effectiveness, outside-player comprehension,
accessibility, preference, enjoyment, or willingness to cooperate.

M28 does not authorize matchmaking, public lobbies, discovery, invites,
presence, chat, voice, text entry, emotes, friends, moderation, public
networking, accounts beyond the existing local identity, a Protocol V3,
commit, push, merge, version change, tag, release, distribution, or deletion
of prior evidence. `VERSION=0.2.0`, frozen V1 artifacts, the M24 package, all
M25/M26 catalogs, and the complete M27 route/event system remain protected.

Every M28 mutation remains server-authoritative. Clients may explicitly opt
in, start an eligible operation, submit one bounded ping, request a revive,
and present results. They may never assign roles, select a peer's action,
choose health/reward/timing values, complete a channel locally, invent a life
state, or decide terminal outcome.

## Single starting proposition

> A loopback Relay slice with exactly two admitted participants, one
> server-authoritative downed/revive cycle, one bounded contextual ping, and
> one complementary two-role objective can require distinct recorded
> participant contributions while anti-loop, disconnect/abandonment, reward,
> persistence, replay, Protocol V2 compatibility, and honest projection remain
> finite, deterministic, and independently reconstructible.

This is the only M28 proposition. “Require distinct contributions” means only
that accepted server evidence contains the exact required action from each
immutable role. It does not mean that human players would understand, enjoy,
coordinate around, or consider the interaction cooperative.

## Audited baseline before M28

- `Actor` has kind, archetype, position, current health, and maximum health.
  Damage saturates at zero. There is no life-state enum, downed ownership,
  revive count, revive health, incapacity reason, or bleed-out state.
- `CombatRuntime` accepts only a living Player attacking a living Enemy.
  A zero-health Player cannot attack, but zero player health has no distinct
  session transition, terminal effect, replay record, or recovery path.
- Relay Drone pressure targets the first admitted participant and deals one
  10-damage hit. Warden pressure targets the participant whose accepted hit
  reaches each counter threshold and deals at most two 15-damage counters.
  AI cannot select a replacement target or reason about downed/revived state.
- Player maximum health is an immutable admitted M26 value from 100 through
  130. Weapons, module arithmetic, cooldown, range, enemy health scaling, and
  the M25/M26 pressure envelope are frozen.
- Objectives have only `KillActors`, `ReachArea`, and `Boss`; states are
  Pending, Active, Completed, and Failed; triggers are actor-group death and
  area reached. No cooperative responsibility or participant-owned step
  exists.
- The gateway admits exactly one or two participants, starts when the configured
  count is present, keeps durable admission order, and treats the first account
  as owner/M27 leader. A disconnect removes that actor and resets only when the
  participant set becomes empty; there is no post-start abandonment terminal
  for a remaining peer and no resume/rejoin flow.
- A two-participant session shares enemies, objectives, door, terminal, and
  rewards. Each participant retains its own immutable admitted weapon/module
  profile. The established smoke proves both participants see one shared
  completion and one per-character reward.
- M27 owns an optional route operation after the drone objective. Both
  participants must explicitly opt in, the first admitted leader chooses, and
  an accepted route is immutable. Unopted sessions retain the exact baseline.
- Protocol is V2 with a 64 KiB frame ceiling. It has 13 client and 25 server
  variants, including M26 module and M27 route messages, but no cooperation,
  ping, downed, revive, role, abandonment, or cooperation-summary message.
- Replay recognizes exactly 15 event kinds: nine legacy activity/combat kinds,
  three M26 module kinds, and three M27 route kinds. It contains no
  cooperation vocabulary or life-state evidence.
- PostgreSQL has fourteen public tables. Existing activity completion grants
  inventory/XP/history per participant atomically; M27 selection/terminal
  rows and replay are idempotent and transactional. There is no cooperation
  operation, role, ping, life-state, or revive table.
- Inspector has only GET surfaces and reconstructs legacy, module, and route
  evidence. Godot projects actor health and generic confirmed damage/defeat but
  has no peer-life affordance, fixed ping marker, revive channel, role label,
  or cooperation summary. The fake client has deterministic driver/observer
  behavior but no immutable cooperation role.
- The mutable local engineering database currently has fourteen tables, 826
  synthetic accounts/characters, 1,059 activity histories, 9,321 replay rows,
  216 M27 route operations, and zero installed M27 failure-injection trigger or
  function. These are test fixtures, not product or player metrics.

M28 may add a bounded cooperation operation around this baseline. It may not
reinterpret zero health, route absence, admission order, ordinary completion,
or disconnect evidence retroactively.

## Compatibility and operation exclusivity

The M28 cooperation slice is explicit opt-in and available only when exactly
two admitted participants both use current Protocol V2 and both separately
request the M28 capability. Identity, client build, M27 capability, equipped
weapon, module loadout, movement, or timing never implies M28 capability.

Capability alone does not change the activity. After `clear_drone_group`
completes, the first-admitted leader may submit one bounded cooperation-start
operation while the M27 route/baseline choice window is still open. The
operation commits only if both participants are still admitted and capable.

M27 route selection and M28 cooperation are mutually exclusive for a session:

- an accepted M27 route or locked compatibility door makes M28 start a
  wrong-phase rejection;
- an accepted M28 cooperation start durably locks the unrouted route baseline,
  so later M27 choice rejects and no route row/event is created;
- rejected M28 input reserves no operation ID and does not lock routing; and
- the shared coordinator serializes start and route intents. The first valid
  leader operation committed by the server wins; clients cannot vote, race a
  seed, or select for a peer.

An ordinary unopted V2 session, a mixed capable/unopted V2 session, any V1
participant, a solo session, and every existing M27 routed session preserve
their exact current flows and receive no unsolicited M28 variant. M28 is not a
third route, does not resolve an M27 event, and cannot combine with Breach or
Stabilize in the same session.

## Fixed roles and complementary objective

An accepted operation has exactly two immutable roles derived from durable
join append order:

- participant index 0 is `anchor` and remains the leader;
- participant index 1 is `runner`.

No client supplies, swaps, votes on, rerolls, or inherits a role. Reconnect is
not available. The server exposes the complete role assignment to both capable
participants only after durable operation commit.

The cooperation objective is a sidecar operation between Drone and Door. It
does not add a Lua objective kind or modify the frozen M27 authoring schema.
It uses exactly these fixed semantic targets inside the existing movement
boundary:

| Target | Coordinate | Owner |
| --- | --- | --- |
| `relay_anchor` | `[3, 0, 3]` | anchor |
| `relay_console` | `[4, 0, 3]` | runner |

The only successful sequence is:

1. The anchor reaches `relay_anchor`.
2. The anchor emits the one accepted `relay_console` contextual ping.
3. The runner reaches `relay_console` while that ping is live.
4. The server applies one fixed Relay-feedback hazard that downs the runner.
5. The anchor starts and completes the one revive channel while in range.
6. The existing relay door and unchanged baseline Warden encounter complete.
7. One atomic cooperation success grants the fixed equal group reward.

Runner movement before the anchor step and anchor ping is ordinary movement
but cannot advance the objective. Reaching the console without a live accepted
ping does not advance or down the runner. Wrong-role, wrong-target, duplicate,
out-of-order, disconnected, incapacitated, or post-terminal actions reject
without changing state.

The operation owns the following finite phases:

`awaiting_anchor → awaiting_ping → awaiting_runner → runner_downed → revive_channel → encounter_active → succeeded`

Terminal alternatives are exactly `failed_ping_timeout`,
`failed_revive_timeout`, `failed_operation_timeout`,
`failed_participant_defeated`, and `abandoned_disconnect`. There is no skip,
reset-to-baseline, role switch, second objective, difficulty tier, procedural
step, or continuation after a terminal.

## Contextual ping contract

- Exactly one accepted ping exists per cooperation operation.
- Only the anchor may submit it, only in `awaiting_ping`, and only after the
  anchor step is durably observed.
- The target is the enum value `relay_console`; no coordinate, text, icon,
  color, duration, participant identity, payload, or arbitrary string comes
  from the client.
- The ping becomes live at server monotonic acceptance and expires after
  exactly 5,000 ms. Runner arrival at or before 5,000 ms succeeds; the first
  authoritative observation after 5,000 ms fails the operation.
- The ping operation identifier uses 1-32 ASCII alphanumeric/hyphen
  characters. Same-input retry returns the stored result. Identifier conflict,
  a second identifier, and any later ping reject without extending the TTL.
- Both participants receive the same source actor, target, accepted monotonic
  duration, and active/expired state. The client does not start or renew the
  authoritative timer.

This is a fixed contextual signal, not chat or a general ping wheel. It cannot
target actors, locations, inventory, routes, or user-authored content.

## Downed and revive contract

The Runner reaching the live-ping console triggers exactly one scripted
Relay-feedback hazard. The server records the runner's current and maximum
health, applies damage equal to current health, sets health to zero, and enters
`runner_downed`. This hazard changes no maximum health, module state, weapon,
cooldown, range, inventory, route state, enemy health, or M27 event effect.

A downed actor remains present but cannot move, attack, equip, mutate modules,
submit route/cooperation actions, advance an objective, or receive another
downing. It is not destroyed, dead, respawned, teleported, invulnerable, or
controllable. Only the server may mark it downed.

The anchor may start one revive when all conditions hold:

- the target is the admitted runner actor;
- the source is the admitted anchor actor and is not incapacitated;
- squared distance between them is at most four;
- the operation is in `runner_downed`;
- the 15,000 ms revive window has not expired; and
- a valid 1-32 byte ASCII alphanumeric/hyphen revive operation ID is unused.

The accepted channel lasts exactly 2,000 monotonic milliseconds. A completion
observation before 2,000 ms remains pending and mutates no life state;
completion at or after 2,000 ms succeeds only if still within the 15,000 ms
revive window and the anchor has remained in range. Moving out of range cancels
the in-memory channel without reserving a new operation ID or extending the
revive deadline. Same-input retry after success returns the stored result;
conflicting target/identity and a new revive after success reject.

Successful revive restores exactly 50 health, never more than the runner's
immutable admitted maximum, changes the runner to active, and increments the
session revive count from zero to one. No heal-over-time, shield, immunity,
second downing, second revive, self-revive, item consumption, cooldown
reduction, revive module, or per-build revive effect exists.

The unchanged baseline Warden can deal at most 30 later damage to one
participant. A correctly revived 50-health runner therefore remains nonlethal
on the bounded optimal path. If either participant nevertheless reaches zero
after the one revive, the server records `failed_participant_defeated` and no
reward rather than allowing another down/revive cycle.

## Timing and terminal rules

The complete cooperation operation has a 60,000 ms monotonic budget starting
only after durable start commit. Wall-clock timestamps are display/audit facts
and decide no phase or ordering.

- Every mutating/observing cooperation, movement, combat, equipment, module,
  route, and disconnect command observes the active deadline first.
- Completion at or before 60,000 ms remains eligible for success; the first
  observation after 60,000 ms commits `failed_operation_timeout`.
- Ping and revive sub-deadlines are checked before the overall deadline when
  their phase is active and produce their specific failure reason.
- A failure marks every active cooperation step failed, closes the activity,
  and emits no generic `ActivityComplete`, history, loot, XP, route terminal,
  or cooperation success.
- The revive completes the complementary objective but not the operation.
  Success commits only after the unchanged door/Warden flow completes within
  the overall deadline.
- An incomplete crash without a terminal remains reconstructibly incomplete.
  Startup does not invent elapsed time, resume a channel, fail an operation,
  or grant a reward.

The exact 5,000, 15,000, 2,000, and 60,000 ms boundaries require pure,
gateway-clock, persistence, and replay tests. Runtime evidence may observe a
real value strictly past a deadline; it does not replace exact injected-clock
boundary proof.

## Reward and fairness boundary

Successful cooperation grants each of the two admitted characters exactly:

- two `relay_core_fragment`; and
- 125 XP.

No individual score, last-hit, speed, role, ping, revive, health, weapon,
module, damage, or survival bonus exists. Both participants receive the same
reward only after both required role contributions and the shared Warden
completion are durably proven. Existing per-session/per-character idempotency
keys remain mandatory.

Failure, abandonment, incomplete operation, duplicate terminal, partial
participant state, or replay failure grants nothing and writes no activity
history. Success commits both participants' inventory, progression, history,
cooperation terminal, generic completion/grants, and replay as one transaction.
Retry returns the original result and appends/grants nothing.

The fixed reward creates no new item, currency, module, rarity, roll, recipe,
exchange rate, streak, pity, daily/weekly schedule, or metaprogression layer.
Its relationship to Breach/Stabilize is an engineering tradeoff only and makes
no preference or fairness claim about human valuation.

## Disconnect and abandonment rules

- Before cooperation start commits, disconnect behavior remains the current
  M27 behavior and creates no M28 row/event.
- After commit and before terminal success/failure, either participant
  disconnecting commits `abandoned_disconnect`, records the last durable
  phase/contributions/life state, and grants no reward or history.
- The remaining participant receives one capable-client terminal summary if
  its channel is writable, but cannot continue, route, replace the peer, or
  recruit a bot into the same session.
- A simultaneous/broken broadcast cannot produce two abandonment terminals.
  Durable append order, not socket timing or wall time, decides the first
  terminal.
- M28 adds no resume, rejoin, takeover, backfill, reconnect grace period,
  offline continuation, vote-to-abandon, kick, or penalty.
- When all connections leave, normal reset clears in-memory M28 capability,
  role, ping, life, timer, and operation state for the next session without
  deleting durable evidence.

These rules mechanically bound abandonment. They do not establish resistance
to real human griefing or social misuse.

## Pure domain and generated matrix contract

Block 2 must isolate a database/network/client/clock-free cooperation domain
and a deterministic local laboratory before any persistence or wire change.
The domain owns typed role, phase, contribution, life, ping, revive,
deadline, terminal, reward, and idempotency state.

The full admitted build/weapon pair space is exactly:

`2 anchor weapons × 15 anchor builds × 2 runner weapons × 15 runner builds = 900 rows`

Every row must prove:

- exact immutable admitted maximum health and combat profiles;
- one anchor step/ping/revive and one runner console contribution;
- scripted downing from the runner's exact current health to zero;
- one 50-health revive within maximum health;
- unchanged baseline enemy health/damage and no M27 effect;
- nonlethal bounded optimal post-revive Warden pressure;
- equal two-fragment/125-XP reward only on success; and
- checked arithmetic with no state-space or collection overflow.

Separate exhaustive lifecycle vectors must cover every phase, exact timer
boundary, both disconnect identities at every nonterminal phase, role/target/
distance/order rejection, same-input retry, conflict, channel cancellation,
terminal replay, crash-incomplete representation, and second ping/down/revive
rejection. Gate 2 must freeze stable JSON schemas and deterministic hashes.

M28 does not require a Pareto or human-fairness claim. The generated report
must instead prove that no build/weapon pair removes either role's required
contribution, changes the fixed hazard/revive/reward, or exceeds the retained
health/pressure bounds.

## Persistence and replay boundary

Block 3 must review exact additive idempotent DDL before applying it. At most
one cooperation operation exists per session with exactly two ordered
participants/roles, three bounded mutation operation IDs, fixed target/timing/
reward values, contribution/life state, at most one revive, and one optional
terminal. Old rows need no backfill. No existing table/column/meaning may be
deleted, renamed, or rewritten.

Selection, ping, downing, revive, success, and failure transitions must be
transactional and retry-safe. A pre-migration external backup, disposable
restore/apply-twice proof, legacy logical hashes, every write-boundary rollback
fixture, concurrent same-operation fixture, and working-database migration
audit are required before the persistence gate is green.

Block 4 may add exactly six reviewed replay kinds:

1. `cooperation_started`;
2. `cooperation_pinged`;
3. `player_downed`;
4. `player_revived`;
5. `cooperation_succeeded`; and
6. `cooperation_failed`.

Their structured payloads must embed schema/catalog revision, session and
operation identities, ordered roles, admitted profiles, fixed objective and
timing values, every accepted contribution/transition, health/life evidence,
terminal reason, and exact grants/no-reward reason. Reconstruction must not
consult current actors, Lua, operation rows, inventory, modules, gateway
memory, randomness, or wall time.

An event stream with no M28 kind remains cooperation-legacy and fabricates no
M28 fact. Unknown, duplicate, missing, reordered, oversized, cross-session,
wrong-role, wrong-health, wrong-timing, partial-grant, post-terminal, and
impossible lifecycle payloads reject. Inspector may expose only bounded
reconstructed facts on its existing GET surfaces.

## Protocol and gateway boundary

Protocol remains V2. Before editing it, Block 5 must review exact additive
capability, start, ping, revive, state/result, life-state, and summary shapes.
There may be at most four new client variants and six new server variants, all
behind explicit M28 capability and inside a private 8 KiB cooperation-message
budget plus the existing 64 KiB frame ceiling.

No request may contain a role, coordinate, duration, health, hazard damage,
revive amount, reward, participant list, terminal, replay event, route state,
or peer decision. Nested unknown fields and invalid enum/string/collection
bounds reject before mutation. An ordinary old V2 or frozen V1 participant
receives no new variant.

The gateway must compose the accepted domain with existing immutable
admission, single-threaded shared coordinator, actor registry, baseline combat,
transactional persistence/replay, and reset. It may not put protocol parsing
inside the cooperation domain or use wall time for phase decisions.

## Deterministic bots and honest Godot presentation

Deterministic bots must exercise both immutable roles, every valid step,
wrong-role/order/distance/target input, exact timing boundaries, retry/conflict,
disconnect at each phase, ping/revive/overall timeout, post-revive combat,
success, no-reward failure, reset, ordinary V2, M27 routes, and frozen V1. Bots
provide no human-behavior evidence.

Godot may show only server-authored role, phase, fixed target, ping source/TTL,
downed health, revive source/progress/result, contributions, terminal, and
reward evidence. A local attempt may show neutral pending state, but cannot
place a confirmed ping, down a peer, advance a channel, restore health,
complete an objective, or show a reward before authoritative confirmation.

Required semantic states are empty/unavailable, capability pending, eligible,
start pending/rejected/accepted/replayed, anchor step, ping pending/live/expired,
runner step, downed, revive pending/channel/cancelled/rejected/completed,
encounter active, disconnect/abandonment, success, failure, invalid-server,
and reset. Keyboard focus, non-color text/icon redundancy, reduced flash,
bounded identifiers, 1280×720 containment, and disconnect recovery fixtures are
mandatory. No voice/text communication or additional audiovisual asset family
is allowed.

## Runtime and evidence contract

Block 8 must freeze an exact finite execution contract before writing its
harness. At minimum it samples all required roles, both weapons in each role,
Empty plus Force/Ward/Force+Ward builds, success, each terminal failure family,
both disconnect identities, retry/conflict, channel cancellation, fresh/reused
progression, ordinary V2, a complete M27 route session, current V1, frozen V1,
replay/Inspector/SQL parity, reset, repetition, and resource bounds.

All state must be earned through normal activities and accepted operations;
SQL may inspect but not create fixture state. Evidence remains external,
prefix-isolated, bounded, checksum-verifiable, and free of unnecessary personal
data. All listeners are loopback and every isolated process stops afterward.

## Block plan and gates

### Block 1 — Specification and baseline

- Audit actors, life/damage, AI target ownership, objectives, two-client
  session/disconnect, reward persistence, replay/Inspector, Protocol V2/V1,
  Godot/fake-client, live schema, and M27 coexistence.
- Freeze the single proposition, exact two roles, one objective/ping/down/revive
  cycle, timing/reward/compatibility/evidence bounds, and stop conditions.

Gate 1 requires one finite falsifiable design with no M28 implementation source
change.

### Block 2 — Pure cooperation domain and laboratory

- Implement typed lifecycle/authority/timing/retry/terminal arithmetic without
  database, network, client, filesystem, or clock dependencies.
- Generate the exact 900-row build/weapon report plus exhaustive lifecycle and
  timer-boundary vectors.

Gate 2 requires deterministic hashes, complete contribution invariants,
nonlethal pressure, exact reward, checked bounds, and no M27 mutation.

### Block 3 — Additive persistence

- Review exact schema and transaction shapes before DDL.
- Back up, restore, apply twice, preserve legacy rows, and prove selection,
  ping/down/revive/terminal idempotency and rollback.

Gate 3 requires recoverability, zero partial state/reward, concurrency proof,
and no replay vocabulary change.

### Block 4 — Replay, reconstruction, and Inspector

- Review exact six event payloads before editing replay.
- Couple operation transitions transactionally, reconstruct independently, and
  expose only GET evidence.

Gate 4 requires corruption/failure matrices, CLI/Inspector parity, legacy
preservation, and zero leftover injection mechanism.

### Block 5 — Additive Protocol V2 and gateway

- Review exact wire structs before protocol edits.
- Integrate explicit capability, roles, operation, ping/life/revive/timers,
  failure, success, and reset while preserving M27/V1.

Gate 5 requires max-frame, malformed, authority, exact-clock, persistence
failure, two-client shared truth, ordinary-V2, M27-route, and frozen-V1 proof.

### Block 6 — Deterministic role bots

- Add bounded anchor/runner automation and negative/failure fixtures.
- Prove distinct required contributions and shared terminal truth without a
  human-behavior claim.

Gate 6 requires repeatable valid/invalid flows and no client-owned outcome.

### Block 7 — Honest Godot cooperation presentation

- Add server-only role/ping/downed/revive/contribution/terminal presentation.
- Prove keyboard, non-color, reduced-flash, invalid-server, disconnect/reset,
  and 1280×720 states with creator-reviewable captures.

Gate 7 requires semantic, real-wire, replay/Inspector, visual, compatibility,
and canonical quality evidence.

### Block 8 — Runtime, recovery, and resource matrix

- Execute the reviewed finite two-client success/failure/build/compatibility
  matrix.
- Reconcile exact state, contribution, health, timing, rewards, persistence,
  replay, Inspector, reset, repetition, and resources.

Gate 8 requires no integrity, authority, projection, transaction,
compatibility, privacy, reset, or resource blocker.

### Block 9 — M28 closure

- Review the complete M28 diff and every accepted/rejected artifact.
- Run the uninterrupted canonical quality gate and protected-invariant audit.
- Classify residuals, decide Gate M28, and record exactly one bounded M29
  starting proposition without implementing M29.

Only a green Gate M28 permits M29 specification work.

## Stop conditions

- More or fewer than exactly two cooperation participants, mutable/client
  roles, role voting/swap/reroll, matchmaking, lobby, invite, presence, or
  replacement participant.
- More than one objective, ping, scripted downing, revive, or operation per
  session; arbitrary ping text/coordinate/target; heal, item, module, class,
  skill tree, or repeatable revive growth.
- Client-authored coordinate, role, timer, health, damage, revive amount,
  reward, contribution, peer action, terminal, or replay fact.
- Cooperation combined with an M27 route, implicit capability, unsolicited new
  message to old V2/V1, Protocol V3, or change to frozen V1.
- Changed M25/M26 weapon/module arithmetic, M27 route/event catalog, baseline
  enemy pressure, Lua authoring schema, version, package, or existing reward
  meaning outside a reviewed later gate.
- Partial operation/participant/life/reward/replay state, duplicate terminal,
  post-terminal mutation, reward on failure/abandonment/incomplete state,
  destructive migration, or reconstruction from current mutable state.
- Wall-clock gameplay decision, unbounded timer/string/collection, public
  listener, external account/service, another person, unnecessary personal
  data, or retained failure-injection mechanism.
- Any automation or creator evidence described as human teamwork, social
  quality, anti-griefing effectiveness, preference, comprehension, enjoyment,
  accessibility, or population behavior.

## Gate 1 decision

Gate 1 is **approved**. The repository and live-schema audit confirms the
baseline above, and the single proposition is finite and falsifiable. The two
roles, one fixed complementary objective, one fixed contextual ping, one
scripted down/revive cycle, exact locations/timers/reward, route exclusivity,
compatibility boundary, 900-row pure matrix, evidence requirements, block
gates, and stop conditions are frozen.

No M28 implementation source was changed during Gate 1. The complete M27
closure plus this green specification is preserved in the external checkpoint
`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m27-closure-45718926-20260906.tar.gz`:

- source/archive entries: `352` / `352`, with zero path differences;
- archive size: `10,684,040` bytes; and
- SHA-256: `81bb1fafaa397b27db7fc505767c09202d68b77e13355952b49ad9d994280d70`.

The sidecar `.sha256` file and `gzip -t` validation are green. Block 2 alone was
then authorized to implement the pure cooperation domain and deterministic
laboratory; persistence, replay, protocol, gateway, bots, and Godot remain
locked behind their later gates.

## Gate 2 decision

Gate 2 is **approved** under
`docs/operations/m28-block2-domain-audit.md`. The pure `m28-v1` domain, 18
domain tests, four lab tests, byte-stable 900-row ordered build/weapon matrix,
50-vector lifecycle report, exact deadline/contribution/health/reward bounds,
and M27 non-mutation audit are green. The accepted evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block2-0906g2c`.

Block 3 may review the exact additive persistence schema and transaction
shapes. No DDL is authorized until that review is recorded, and replay,
protocol, gateway, bot, and Godot work remain locked behind later gates.

## Gate 3 decision

Gate 3 is **approved** under
`docs/operations/m28-block3-persistence-audit.md`. The reviewed two-table
additive schema, pre-migration dump, clean restore, apply-twice proof, fourteen
legacy byte comparisons, transactional lifecycle, exact equal rewards,
no-reward terminals, nine-case PostgreSQL suite, every reviewed rollback
boundary, concurrent convergence/conflict, working-database audit, 193-test
canonical gate, and 115-file M27 protected comparison are green.

The accepted disposable and working evidence are
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block3-disposable-20260906c` and
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block3-working-20260906c`, with
manifest SHA-256 values
`6009de322d6c32e9aa5dcc8373f9f60221f9ca125b9d452d6731138a05efa039`
and
`4d4edfe400fe3f56e121e306cd4e7f6acf3404d1431dd80dac398c3d903e545d`.
Earlier `20260906`/`20260906b` persistence evidence is preserved but rejected
because final review found and corrected temporal/terminal constraint and
deadline-retry gaps.

Block 4 may now review the exact six replay payloads, atomic append seams,
independent reconstruction, corruption matrix, and GET-only Inspector shape.
No replay implementation is authorized until that review is recorded, and
protocol, gateway, bots, and Godot remain locked.

## Gate 4 decision

Gate 4 is **approved** under
`docs/operations/m28-block4-replay-audit.md`. The exact six schema-version-1
payloads, 8,192-byte ceiling, atomic durable/replay/reward coupling, exact
retry, independent legacy/active/succeeded/failed reconstruction, corruption
and lifecycle rejection, all five reward-free failure families, CLI/Inspector
parity, GET-only method boundary, fourteen-table legacy preservation, and the
205-test canonical gate are green.

Accepted external evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block4-replay-20260906`, with
manifest SHA-256
`f4ec22b93323121effb8196096e64606121b212a416e314c6a0282f89620565f`.
The accepted final database sweep retained zero failure trigger/function, both
literal disposable databases were removed, and the M27 checkpoint, Protocol
V2, frozen V1, gateway gameplay, Godot, scripts, tests, migrations, and
`VERSION=0.2.0` protected boundaries remain intact.

Block 5 may now review the exact additive capability/start/ping/revive/state/
life/summary wire structs, private 8 KiB cooperation-message bound, malformed
input behavior, ordinary-V2/frozen-V1 silence, and gateway composition seams.
No Protocol or gateway gameplay implementation is authorized until that
pre-implementation review is recorded; bots and Godot remain locked.

## Gate 5 decision

Gate 5 is **approved with bounded residual risk** under
`docs/operations/m28-block5-protocol-audit.md`. The exact four-client/six-server
additive current-V2 vocabulary, private 8 KiB bound, malformed-input rejection,
frozen-V1 exclusion, immutable roles, route exclusion, post-commit shared
truth, exact clock seams, all persistence-failure boundaries, equal success
rewards, five reward-free failures, reset, Inspector, and ordinary compatibility
flows are green.

Final review corrected premature participant removal after a failed broadcast,
inaccurate capable-peer disclosure on a late capability rejection, and a
persisted ping retry that incorrectly used the later observation time instead
of its durable accepted time. The post-correction real PostgreSQL suites have
15 tests, the gateway suite has 39 tests, and the uninterrupted complete gate
has 217 tests. Accepted external evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block5-protocol-20260906`; its
`SHA256SUMS` hash is
`bf2d3206b0a807655f84aa7db41ef6bd3c3b4af7fb7579c33034516d681cce79`.

## Gate 6 decision

Gate 6 is **approved** under `docs/operations/m28-block6-bot-audit.md`. The
bounded 16-session matrix proves one complete distinct-role success, all three
clock timeouts, both roles disconnecting in every nonterminal phase, invalid
intent rejection, durable same-ID retries, route exclusion, exact rewards,
GET-only Inspector/SQL parity, and 16 clean resets without client-owned
outcome. Its accepted evidence manifest hash is
`5acbb4351b28d3a8e86f2c694445ddd219f6b3cf1a9ccbc64a70b56d35d79a2b`.

Block 7 alone is authorized to review and implement honest Godot cooperation
presentation and its semantic/live creator evidence.

## Gate 7 decision

Gate 7 is **approved with bounded Block 8 residuals** under
`docs/operations/m28-block7-godot-audit.md`. The bounded cooperation console,
strict server projection, keyboard/non-color/Reduced Flash behavior, complete
semantic fixtures, real Godot anchor plus standalone runner flow, six reviewed
captures, exact PostgreSQL/replay/Inspector parity, corrected Compose build,
217-test canonical gate, loopback services, and protected V1/M27 boundaries
are green.

Accepted external evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block7-godot-20260907h`; its
`SHA256SUMS` hash is
`1cbf0d832aafc3bd4b56c9a4520e0ad219abc381b63a8d3a461ea02b9ee4a10a`.
Block 8 may now freeze and execute the exact finite two-client success,
failure, build, compatibility, recovery, repetition, and resource matrix. M28
closure and M29 remain locked behind their respective green gates.

## Gate 8 decision

Gate 8 is **approved** under
`docs/operations/m28-block8-runtime-audit.md`. The accepted 399-second matrix
earned six module builds through normal play, completed all four exact
anchor/runner build pairs, sampled ordinary V2, a complete M27 route, current
V1 and frozen V1, repeated the 900-row/50-case pure reports, and reconciled
PostgreSQL, replay, Inspector, resets, resources, and protected boundaries.

Its accepted external evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block8-runtime-0907b8c`, with
manifest SHA-256
`9dbf6fba76bd99cf6abdc10109e8d1ec5dd22271949e8ac732306a049f4f6a46`.
The supporting canonical/build/invariant evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block8-quality-0907c`, with
manifest SHA-256
`36497f1b7d3467411910b994b379f1d33481e03643c2bf20aa54203e22596b3a`.
Earlier `0907b8a` and `0907b8b` targets remain preserved but rejected for an
ordinary-V2 harness admission-order flaw and skipped focused PostgreSQL tests,
respectively.

Block 9 alone is now authorized to review the complete M28 diff and accepted/
rejected evidence, run the uninterrupted closure gate, classify residuals,
decide Gate M28, and record one bounded M29 starting proposition without
implementing M29.

## Gate M28 decision

Gate M28 is **approved with bounded residual risk (green)** under
`docs/operations/m28-closure-audit.md`. All eight block gates, complete
checkpoint diff/evidence review, 217-test canonical gate, production builds,
capture manifests, working-database composed-session integrity, loopback
services, frozen V1, and M27 protected boundaries are green.

M28 is complete. Its automation and creator review make no claim about another
person's comprehension, cooperation, enjoyment, accessibility experience, or
preference. The one permitted M29 starting proposition is recorded in the
closure audit; no M29 implementation has begun.
