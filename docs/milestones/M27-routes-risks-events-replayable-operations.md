# M27 — Routes, risks, events, and replayable operations

Status: complete; Gates 1-8 and Gate M27 are approved with bounded residual
risk.

## Authorization and boundary

The owner authorized sequential execution of the already-sketched solo
milestones M25-M32. M26 is complete and Gate M26 is green. M27 may therefore
proceed, but every block below remains behind its preceding gate.

M27 is creator-only local engineering work. It may use generated seeds,
deterministic bots, creator-operated Godot flows, and loopback services. It
does not authorize another person, public networking, a commit, push, merge,
version change, tag, release, distribution, or deletion of prior evidence.

The server owns route eligibility, leader identity, accepted choice,
idempotency, seed generation, event resolution, objective graph, encounter
effects, duration, success/failure, rewards, persistence, and replay. A client
may opt in, request, and present. It may never select for another participant,
provide the seed, resolve an event, advance an objective, calculate a reward,
or confirm an outcome locally.

Protocol V2, the 12 recognized replay event kinds, the 12-table PostgreSQL
schema, the current activity Lua, the frozen V1 artifacts, the M24 package,
and `VERSION=0.2.0` remain unchanged through Gate 1. Every later Lua/domain,
schema, replay, protocol/gateway, or client change has its own explicit block
and gate below. No such change may leak into an earlier block.

## Single starting proposition

> A Relay operation with exactly two server-authoritative route choices and at
> most one bounded deterministically seeded event per route can expose distinct
> risk/reward outcomes while every choice, objective transition, operation
> summary, reward, and replay reconstruction remains finite, transactional,
> compatibility-preserving, and independently verifiable.

This is the sole M27 engineering proposition. Generated matrices and local
creator runs may support or falsify it. They cannot show that another person
understands, enjoys, prefers, or would replay either route.

## Audited baseline before M27

- `relay_awakening` is a 1,071-byte static Lua table with SHA-256
  `460c895c2bb99090cbcf59d9238717fb3a868c9037dcb46fca5a88fc93e6b040`.
  It has one active drone objective followed by one door objective and one
  Warden objective. It grants one `relay_core_fragment` and 100 XP.
- `revenant-activities` enables only Lua table and string libraries and reads
  an activity table into Rust. It has no source, instruction, or memory budget;
  no unknown-field, duplicate-ID, graph, reachability, or cycle validation;
  and no route, seed, event, failure-summary, or duration concept.
- Objectives are limited to `KillActors`, `ReachArea`, and `Boss`, with
  Pending, Active, Completed, and Failed states. World triggers are actor-group
  death and area reached. No current activity path emits Failed.
- The gateway has one fixed Waiting/Drone/Door/Boss/Complete lifecycle. During
  Door, an authoritative move to `[6, 0, 0]` reaches `relay_door`, opens the
  relay, and requests the Warden. No route operation exists.
- M25 combat owns two weapons, 100-130 admitted player health, one 10-damage
  drone hit, and up to two 15-damage Warden counters. M26 owns exactly 15
  canonical loadouts and copies resolved profiles into immutable admission
  state. M27 must not mutate either catalog or active loadout arithmetic.
- Protocol remains V2 with a 64 KiB frame ceiling. Current V2 has attack,
  movement, equipment, and opt-in module messages plus generic activity,
  objective, door, completion, reward, and module responses. It has no route
  capability, choice, event, or operation-summary message.
- Replay recognizes exactly 12 kinds: player join, activity start, enemy
  spawn/death, boss spawn, activity completion, loot, progression, equipment,
  and three module kinds. The local database also contains 17 deliberately
  invalid `future_event` rows created by the PostgreSQL unknown-vocabulary
  rejection test; they are test fixtures, not accepted vocabulary or M27 data.
- Completion history, per-character item/XP reward grants, and replay writes
  are already participant-idempotent and transactional. There is no durable
  session route, seed, route operation, or failed-operation summary.
- The Inspector is GET-only. Godot and the fake client assume the current
  generic linear objective sequence. Frozen V1 and an ordinary old V2 client
  have no way to understand route-specific messages.
- The observed local engineering database has 12 public tables, 431
  accounts/characters, 790 activity-history rows, 6,725 replay rows, 89 module
  states, and 1,105 module operations. These mutable fixtures are not product
  usage metrics and may change as tests run.

The current activity, protocol, replay vocabulary, schema, gateway lifecycle,
and client sequence are a protected baseline, not an accidental starting
point to reinterpret.

## External M26 closure checkpoint

The required checkpoint was accepted before any M27 implementation source
edit:

`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m26-closure-45718926-20260901.tar.gz`

It contains 315 repository source/evidence files, excludes `.git`, ignored
build output, and dependency caches, is 10,229,929 bytes, and has SHA-256
`066b9251f47c7b916b298d453a9c11b95c63ea305247e3cbf8010e0340256d82`.
The archive entry count exactly matched the source list. It includes the green
Gate 1 specification and pre-implementation ledgers; transcribing this digest
and checking the ledger item afterward are documentation-only changes.

## Compatibility baseline: not a third route

The existing M26 Relay path remains the **unrouted compatibility baseline**.
It is not counted as a third M27 route and is never presented as one.

- A participant who sends no M27 opt-in follows the exact established drone,
  door, Warden, completion, one-fragment, and 100-XP behavior.
- The established movement to `[6, 0, 0]` remains sufficient to enter the
  baseline Warden phase. It does not silently select an M27 route.
- An unrouted session emits no route-specific protocol message or route replay
  event. Its existing generic wire sequence, combat arithmetic, rewards,
  history, and replay meaning remain unchanged.
- Frozen V1 and ordinary old V2 participants therefore retain their canonical
  flows. Frozen V1 files and hashes remain byte-identical.
- Before changing the Lua activity, Block 3 must preserve the current source
  as a golden fixture and prove that the extended loader produces an identical
  baseline objective/event/reward projection when no route is selected.
- Explicit routing is available only when every admitted participant has
  opted into the M27 capability through the later additive V2 flow. A mixed
  capable/unopted or V1 session remains eligible for the baseline but not for a
  route choice.

This compatibility rule prevents “absence of a new request” from becoming a
new behavioral choice and prevents old clients from receiving content they
cannot honestly present.

## Finite route and event taxonomy

M27 has exactly two candidate route identifiers and exactly two candidate
events for each route. Block 2 may tune or reject their bounded values; it may
not add a route or event.

| Route | Alternate objective shape | Reward candidate | Event candidates |
| --- | --- | --- | --- |
| `breach` | proceed from drones to the relay door and Warden | 2 fragments, 100 XP | `overcharged_armor`: Warden health +10%-20%; `arc_surge`: each Warden counter +1 to +5 damage |
| `stabilize` | reach one stabilizer area before the relay door and Warden | 1 fragment, 150 XP | `shielded_channel`: each Warden counter -1 to -5 damage; `residual_feedback`: Warden health +5%-10% |

Exactly one of a selected route's two events is resolved for an explicit
routed session. No event is resolved before a route is durably accepted. No
second event, reroll, nested event, event chain, rare variant, or client event
choice exists. The compatibility baseline resolves no M27 event.

The route reward is fixed by route and is independent of which event resolves.
An event cannot grant currency, XP, inventory, a module, a roll, or a later
modifier. Both rewards remain per participating character and retain the
existing once-per-session grant keys.

Gate 2 must select exact values within these envelopes:

- route fragments: 1 or 2;
- route XP: 100, 125, 150, or 175;
- Warden health multiplier: 10,000-12,000 basis points, applied once with
  checked round-half-up integer arithmetic;
- Warden counter damage: 10-20 after the event modifier;
- maximum total optimal-path hostile damage: 50, including the unchanged
  10-damage drone hit and at most two Warden counters; and
- no event modifies player health, weapon/module arithmetic, drone health or
  damage, cooldown, range, movement, inventory, or seed behavior.

The provisional route/event names and axes are frozen for the Gate 2 search.
Changing an axis, adding an objective kind, or increasing the finite catalog
requires stopping and reopening Gate 1.

## Risk/reward and non-dominance contract

“Risk” means only measured authoritative encounter pressure: Warden health,
accepted attacks to defeat, Warden counter damage, total hostile damage, and
completion duration. “Reward” means only exact per-character fragments and XP.
It does not mean tension, excitement, desirability, or human-perceived value.

For each weapon, canonical M26 build, participant count, route, and event, the
pure matrix must compute the complete encounter. A route dominates the other
only if, across both equally weighted event candidates, it is no worse in both
reward quantities and all pressure/duration quantities, and is strictly better
in at least one. The accepted pair must be non-dominated for every weapon,
build, and participant count. At minimum:

- one route must have strictly more fragments;
- the other must have strictly more XP or strictly less pressure;
- neither route may exceed the M25/M26 stat, health, cooldown, frame, or
  non-lethal optimal-pressure bounds; and
- no claimed duration advantage is accepted unless authoritative monotonic
  runtime evidence measures it. Objective count or map distance alone is not
  evidence that a route is faster or slower.

Equal event probability is a calculation rule, not a prediction of real-world
frequency. Gate 2 enumerates both outcomes exactly rather than relying on a
Monte Carlo sample.

## Seed ownership and deterministic resolution

- One non-negative 63-bit seed exists per explicitly routed session. The
  server creates it once; no client field, username, timestamp string, route
  request, movement, loadout, or retry may influence it.
- Tests and the pure laboratory may inject a seed through a non-network domain
  boundary. Production gateway input may not.
- Given the accepted route, authoring revision, and seed, a pure stable
  resolver returns exactly one of that route's two event identifiers and exact
  bounded effects. Request timing and hash-map iteration cannot affect it.
- Block 2 freezes the exact integer mixing algorithm, route salts, boundary
  vectors, and mapping vectors before persistence or protocol work.
- The seed, algorithm revision, route, resolved event, exact effects, and
  reward plan must be embedded in durable operation/replay evidence. Replay
  reconstruction never rerolls and never consults today's Lua file.
- A retry returns the original stored resolution. A conflict, rejection,
  reconnect, or process restart cannot consume another seed or select another
  event.

The algorithm need not provide gambling-grade unpredictability because M27 has
no purchase or random reward. It must provide stable, unbiased two-candidate
selection over the exhaustive Gate 2 seed vectors and remain server-owned.

## Choice operation and lifecycle

Route selection is one session-wide authoritative operation:

- choice opens only after `clear_drone_group` completes and before the
  compatibility door is reached or any route is already locked;
- the first admitted participant is the session leader, determined by durable
  join append order rather than wall-clock timestamps;
- only that leader may submit the choice, and only after every admitted
  participant has explicitly opted into M27 capability;
- the operation identifier is 1-32 ASCII alphanumeric/hyphen characters and
  the requested route is exactly `breach` or `stabilize`;
- the first accepted operation atomically locks the session route, seed,
  event, effects, reward plan, authoring revision, and objective path;
- retrying the same accepted identifier and route returns the original result;
  reusing it with a different route is a conflict, and any new identifier after
  lock is rejected;
- malformed, unknown-route, non-leader, wrong-phase, capability-incomplete,
  expired, or persistence-failed requests mutate nothing and do not reserve an
  identifier; and
- selection is immutable. There is no vote, veto, majority, timeout default,
  reroll, route switch, abandon-to-baseline, or per-participant route.

The compatibility door locks the unrouted baseline when reached. A later route
intent is then a wrong-phase rejection. This makes the fallback explicit and
prevents an old client from waiting indefinitely for an operation it never
knew existed.

## Multiplayer and disconnect rules

- A selected route, objective path, event, encounter effects, deadline, and
  reward plan are shared by the complete session.
- Every capable participant receives the same accepted server result and later
  operation summary. Existing generic actor/objective/door/completion/reward
  messages remain shared under their current rules.
- Route-specific messages may be sent only to participants that explicitly
  opted in. An unopted old V2 or frozen V1 participant receives no unsolicited
  new variant and makes the session route-ineligible.
- Each participant retains its immutable admitted weapon/module profile.
  Shared enemy health continues to scale only by the existing participant
  rule; an event effect is applied once, not once per participant.
- A disconnect after selection does not change the route or reroll the event.
  M27 adds no resume/rejoin protocol. If the current gateway cannot finish the
  session, replay reports an incomplete selected operation and no completion
  reward is fabricated.
- Reset clears in-memory route/capability/leader/deadline state for the next
  session but never deletes durable evidence.

## Duration, failure, and summary bounds

Only an explicitly routed operation has a new hard monotonic duration budget.
The compatibility baseline keeps its established timing behavior.

- The authored budget is one integer from 30 through 120 seconds and starts
  when route selection commits. Gate 2 selects the exact candidate value.
- The gateway uses a monotonic clock for enforcement. Wall-clock timestamps
  remain display evidence only and never decide expiry or ordering.
- Completion at or before the deadline follows the accepted success path.
  The first authoritative observation after the deadline fails every active
  route objective through existing Failed objective state and closes the route
  operation as failed.
- A failed route emits no `ActivityComplete`, item grant, XP grant, or
  activity-history completion. It cannot later complete in the same session.
- Success and failure each produce one bounded server-authored operation
  summary containing schema/authoring/algorithm revisions, session/operation,
  leader, route, seed, event/effects, objective transitions, participant
  identities, monotonic duration, outcome, and exact reward plan or no-reward
  reason.
- A crash or disconnect that prevents a terminal transaction remains
  reconstructibly incomplete. Startup does not invent a failure timestamp,
  resume an encounter, or grant a reward.

The later protocol may expose a bounded route summary to opted-in clients. It
must not reinterpret generic `ActivityComplete` as failure or inject new
messages into an unopted baseline flow.

## Restricted Lua authoring contract

Block 3 extends authoring only after the pure catalog is frozen. Lua remains a
declarative table boundary, not a general gameplay scripting runtime.

- source is valid UTF-8 and at most 64 KiB;
- evaluation enables only table and string standard libraries, uses at most
  1 MiB of Lua memory, and stops after at most 100,000 VM instructions;
- the returned graph contains only nil, booleans, bounded integers, bounded
  strings, and acyclic tables; functions, userdata, threads, metatables,
  bytecode, dynamic file/module loading, environment access, and callbacks are
  rejected;
- every table uses an explicit allowlist and rejects unknown keys, mixed
  sequence/map shape, sparse sequences, duplicate semantic keys, or values of
  the wrong type;
- identifiers match `[a-z0-9_]{1,32}` and labels/messages are UTF-8 with an
  explicit 128-byte maximum;
- one activity has at most eight objectives, twelve triggers, two routes, two
  events per route, four objectives on a selected routed path, and one
  terminal activity result;
- objective IDs and trigger edges are unique and known; initial-active,
  reachability, activation, completion, and terminal paths are validated;
  self-edges, cycles, multiple completions, unreachable objectives, ambiguous
  triggers, and activation after terminal state are rejected;
- route rewards, duration, event identities, and effect axes must match the
  Gate 2 frozen catalog and numeric bounds; arbitrary stat names or executable
  formulas are rejected; and
- the original M26 table remains valid and resolves to the exact compatibility
  baseline without an implicit route.

A local authoring validator/CLI must emit stable JSON plus human-readable
diagnostics, use nonzero exit status on rejection, and cover every bound with
fixtures. The gateway loads an already validated immutable activity revision;
it never hot-reloads a changed script into an active session.

## Persistence, migration, and transaction contract

Block 4 must review an additive idempotent `0007` migration before applying it
to the working database. The expected minimum is one bounded accepted route
operation per session with its canonical request/result and terminal state.
Exact table, column, and constraint names remain a Block 4 decision.

Required behavior:

- migration runs in the existing advisory transaction, succeeds twice, and
  never deletes, renames, rewrites, or reinterprets an existing row;
- old sessions and characters require no backfill and retain baseline meaning;
- a pre-migration custom-format dump, schema/table inventory, counts, and
  logical hashes are stored outside the repository; disposable restore and
  apply-twice proof precede any working-database migration;
- accepted selection locks the session operation identity and atomically
  persists the canonical route, seed, resolved event/effects, objective path,
  deadline budget, reward plan, participants, and its reviewed replay event;
- same-operation concurrency/retry returns one stored resolution with no
  second row, seed, event, objective activation, or replay append;
- terminal success atomically persists the operation summary, current
  activity completion/history, per-participant route rewards, and reviewed
  replay events using the existing grant idempotency keys;
- terminal failure atomically persists the no-reward operation summary and
  reviewed failure replay, without activity history or grant rows;
- validation/business rejection writes nothing and leaves its identifier
  unreserved; and
- injected failure at every write boundary leaves no partial operation,
  replay, history, inventory, progression, or in-memory confirmation.

Rollback means transaction rollback plus restoration proof on a disposable
database. No destructive down migration or deletion from the working database
is required. The additive table must be inert if pre-M27 code uses the
migrated schema.

## Replay and independent reconstruction contract

Block 5 separately reviews exact vocabulary and payloads before changing the
replay enum. The candidate minimum is one accepted-selection event, one
successful-summary event, and one failed-summary event; their names are not
accepted by Gate 1.

Every new payload is deterministic structured JSON with denied unknown fields,
explicit schema/authoring/algorithm revisions, fixed field order at emission,
bounded identifiers/lists/strings, and a reviewed maximum of at most 8 KiB.
Replay must contain enough immutable evidence to reconstruct:

- leader and capable participant set;
- accepted operation identity and canonical route;
- seed, exact resolver revision, selected event, and effects;
- complete selected objective graph and every transition;
- admitted participant/weapon/module combat inputs needed by M26 replay;
- enemy health/counter effects and authoritative encounter results;
- monotonic duration and success, failure, or incomplete outcome; and
- exact per-participant reward plan and committed grants.

Reconstruction follows append IDs, never timestamp order, and never reads the
current Lua, current route catalog, mutable loadout, current inventory, or a
new random value. It rejects a missing/duplicate selection, event inconsistent
with seed, route/effect mismatch, impossible transition, post-terminal event,
duration overflow, reward mismatch, partial participant grant, mixed
authoring revision, or malformed/oversized payload.

Old sessions with no route event reconstruct under the explicit legacy
baseline. The 17 `future_event` test fixtures must continue to fail as unknown
vocabulary. The Inspector remains GET-only and may decode bounded route detail
and summary; it gains no mutation, reroll, raw SQL, or secret surface.

## Additive Protocol V2 review contract

Block 6 decides exact wire structs before editing Protocol V2. The candidate
surface is a route-state/capability request and response, leader route-choice
intent and result, and terminal route-operation summary.

- Every new request is opt-in. A client that sends none receives the exact
  ordinary V2 sequence and compatibility baseline.
- Capability is session-local and established only by the new state request;
  `client_build`, username, timing, or protocol-version equality is not enough.
- Accepted route result and terminal summary are delivered only to opted-in
  participants. Rejections are targeted to the requester.
- All strings, identifiers, lists, effects, transitions, and payloads have
  explicit maxima below the existing 64 KiB frame bound.
- Route intent contains only operation ID and route ID. It contains no seed,
  event, effect, reward, leader, objective transition, or trusted time.
- The server response is the only authority for selection, event, objective
  path, effects, reward plan, duration, and outcome.
- Malformed/unknown/oversized inputs fail before domain or persistence
  mutation. A response serialization/write failure cannot create an optimistic
  client success.

Frozen V1 messages/files/hashes remain byte-identical and receive no route
message. If additive V2 cannot preserve old-V2 sequence compatibility and the
frame bound, Block 6 stops and proposes a separately reviewed protocol-version
milestone. It does not silently create Protocol V3.

## Godot truth boundary

- The client exposes routing only after the authoritative route-state response
  confirms capability, phase, leader, both exact routes, bounded risk/reward
  facts, and participant eligibility.
- Only the server-declared leader sees an enabled choice. Other participants
  see who decides and cannot synthesize a vote or local selection.
- A sent intent may show neutral pending state. It may not select a route,
  reveal an event, change objectives/stats/rewards, start a timer, or play
  success before the accepted response.
- The accepted display uses server-supplied route, event/effects, objective
  path, deadline budget, and reward plan. The client contains no seed resolver,
  encounter formula, event catalog, or reward arithmetic.
- Rejection, retry, conflict, lost connection, failure, incomplete operation,
  and success remain visibly distinct. Reconnect does not claim resumption.
- Keyboard navigation, visible focus, mute, Reduced Flash, semantic non-color
  cues, bounded audiovisual pools, and M17-M26 presentation remain intact.
- The UI says neither route is best, recommended, safer for a person, more fun,
  or more valuable. It presents exact engineering tradeoffs only.

## Generated matrix and runtime evidence contract

The pure routed encounter matrix has exactly:

`2 routes × 2 events × 2 participant counts × 2 weapons × 15 builds = 240`

rows. A separate 60-row baseline regression covers both participant counts,
both weapons, and all 15 builds without a route. Stable JSON and a reviewable
table must cover:

- all 240 routed rows and 60 compatibility rows;
- exact rewards, health multiplier/rounding, attacks-to-defeat, counter count
  and damage, total hostile damage, cooldown-derived lower duration bound,
  objective-path bound, and non-dominance;
- seed zero, maximum 63-bit seed, every resolver boundary, both events for each
  route, repeatability, route-salt separation, and deterministic rerun hashes;
- selection apply/retry/conflict, wrong route/leader/phase/capability,
  malformed IDs, operation limit, deadline boundary, success, failure,
  incomplete, disconnect, reset, and unchanged-state-on-error cases;
- source/memory/instruction/string/list/graph limits and every invalid Lua
  authoring fixture; and
- frame-size estimates for maximum later candidate messages.

Post-persistence evidence adds migration-twice, disposable restore, exact
legacy-row preservation, concurrent selection/retry, every-write failure
injection, completion/failure atomicity, and independent replay/Inspector
reconstruction.

The final local runtime matrix samples both routes, all four events through
injected server test seeds, both participant counts, both weapons, empty plus
at least three Pareto-distinct M26 builds, fresh/reused characters, retry,
conflict, non-leader, capability mismatch, deadline failure, disconnect,
baseline old V2, frozen V1, exact rewards, reset, and bounded RSS/descriptors/
threads/database connections. Runtime sampling does not replace the exhaustive
pure matrix or constitute human validation.

## Block plan and gates

### Block 1 — Specification and audited baseline

- Freeze this proposition, protected compatibility baseline, finite taxonomy,
  risk/reward definition, authority/lifecycle rules, evidence contract, and
  stop conditions.
- Review current Lua, objectives, gateway, protocol, persistence, replay,
  Inspector, Godot/fake-client assumptions, and live local schema without
  implementation edits.

Gate 1 requires a falsifiable finite design, exactly two routes/two events per
route/one resolved event, an explicit non-route compatibility path, no hidden
Protocol/schema/replay change, and no unresolved choice that can multiply the
state space before the pure laboratory. It authorizes only Block 2.

Gate 1 is **approved**. The source and live-schema review confirms the audited
baseline. The sole proposition is falsifiable; route/event/reward/effect,
seed, operation, graph, duration, transaction, reconstruction, matrix, and
client-truth bounds are finite. Exact numeric tuning and the resolver are
confined to Gate 2 envelopes. Protocol, schema, replay vocabulary, Lua,
gateway, client, frozen V1, package, and version received no M27 implementation
change. Block 2 is authorized only after the required external M26 closure
checkpoint is recorded below.

### Block 2 — Pure route/event domain and deterministic laboratory

- Preserve a checksum-verifiable external M26 closure checkpoint before the
  first implementation source edit.
- Implement dependency-light route catalog, seed resolver, encounter effects,
  operation lifecycle/idempotency, objective-plan values, and matrix CLI with
  no network, database, Lua, clock, gateway, or client dependency.
- Select exact rewards, effects, duration budget, algorithm/revision, salts,
  and boundary vectors from the complete generated output.
- Prove finite state, unchanged-on-error, non-dominance, pressure/stat bounds,
  deterministic hashes, and exact compatibility rows.

Gate 2 freezes or rejects the two-route/four-event catalog. It changes no Lua,
database, replay vocabulary, protocol, gateway, or client.

Gate 2 is **approved**. `docs/operations/m27-block2-route-lab-audit.md`
records the exact `m27-v1` catalog, `m27-permute63-v1` resolver, 90-second
budget, deterministic 240-row routed/60-row baseline reports, non-dominance,
30-50 incoming-damage envelope, complete operation lifecycle, deterministic
hashes, 23 focused tests, 121-test workspace gate, and byte-exact protected
boundaries. Block 3 is authorized only for restricted declarative Lua schema,
resource/graph validation, authoring CLI, and golden baseline proof.

### Block 3 — Restricted Lua schema and authoring validator

- Extend only the declarative schema and resource/instruction limits.
- Validate exact catalog, objective graph, route paths, rewards, duration, and
  events; preserve the original table as a golden compatibility fixture.
- Add the stable local authoring CLI and rejection fixtures.

Gate 3 requires all resource/shape/graph bounds, deterministic output, no
executable gameplay callback, and exact baseline projection.

Gate 3 is **approved**. `docs/operations/m27-block3-lua-authoring-audit.md`
records the bounded static Lua graph, exact catalog validation, stable CLI,
14 focused and 134 workspace tests, byte-identical deterministic JSON, the
exact 1,071-byte M26 golden and runtime projection, and 119 byte-identical
protected files. Block 4 may review exact additive DDL and prepare external
backup/disposable restore evidence. It may not apply a working-database
migration until those proofs pass.

### Block 4 — Schema, migration, and transactional persistence

- Review exact additive DDL and pre-migration backup/restore evidence.
- Implement durable accepted selection, terminal summary, row locking,
  idempotency/concurrency, reward/history coupling, and failure injection.
- Migrate the working database only after disposable apply-twice and restore
  proof.

Gate 4 requires byte/logical preservation of every legacy table, one resolution
per accepted session, no partial terminal transaction, and exact rollback at
every write boundary.

Gate 4 is **approved for the persistence boundary**.
`docs/operations/m27-block4-persistence-audit.md` records the external backup,
disposable restore, apply-twice proof, byte-identical twelve-table legacy
exports, exact additive schema constraints, concurrency/idempotency, terminal
success/failure reward coupling, nine injected write failures, 140-test
workspace gate, and empty working M27 tables. Replay vocabulary and atomic
replay coupling remain intentionally reserved for Block 5.

Block 5's exact pre-implementation vocabulary/payload review is **approved**
under `docs/operations/m27-block5-replay-contract.md`. Implementation stayed
inside that boundary and was required to prove atomic replay coupling,
independent reconstruction, corruption rejection, explicit legacy fallback,
and GET-only Inspector output before the Gate 5 decision below.

### Block 5 — Replay vocabulary, reconstruction, and Inspector

- Review exact event names and bounded payload structs before changing the
  replay enum.
- Couple selection/terminal events transactionally, reconstruct without
  current mutable content, and extend only read-only Inspector evidence.

Gate 5 requires exact success/failure/incomplete/legacy reconstruction,
corruption rejection, existing unknown-event rejection, and no mutation
surface.

Gate 5 is **approved**. `docs/operations/m27-block5-replay-audit.md` records
the exact three-event vocabulary, bounded nested-unknown-rejecting payloads,
admission-ordered immutable M26 evidence, independent reconstruction, explicit
legacy fallback, 15 replay-mode failure-injection boundaries, byte-exact
idempotent retries, GET-only Inspector proof, 149-test canonical gate, and 102
byte-identical protected files. Block 6 may review and freeze its exact
additive wire and gateway lifecycle contract; it may not implement that
contract until the pre-implementation review is recorded.

### Block 6 — Additive Protocol V2 and gateway integration

- Review exact bounded wire structs before editing the protocol.
- Add opt-in capability, leader choice, immutable routed state, authoritative
  objectives/effects/deadline/terminal behavior, and targeted results.
- Update fake/current clients and compatibility fixtures without changing an
  unopted sequence.

Gate 6 requires protocol round trips, maximum frames, malformed rejection,
same-result retry, two-active shared state, old-V2 baseline behavior, exact V1
hashes/gameplay/reconstruction, and no client-controlled seed/effect/reward.

Block 6's exact pre-implementation review is **approved** under
`docs/operations/m27-block6-protocol-contract.md`. It freezes exactly two
client and three server variants, schema/bounds, explicit capability, shared
leader/selection/terminal truth, server-only 63-bit seed, monotonic deadline,
the `[3, 0, 3]` stabilizer trigger, atomic Block 5 replay coupling, and
ordinary-V2/frozen-V1 non-interference. Implementation may now touch only the
reviewed Protocol/compatibility/gateway/fake-client surfaces and supporting
tests; Godot remains behind Block 7.

Gate 6 is **approved**. `docs/operations/m27-block6-protocol-audit.md` records
the exact opt-in Protocol V2 surface, server-only seed and immutable selection,
atomic terminal coupling, both route paths and four event effects, exact
deadline boundary, two-active shared truth, persistence-failure withholding,
ordinary-V2/frozen-V1 non-interference, two live source-gateway route runs,
162-test canonical gate, and loopback/protected-boundary audit. Block 7 may now
review and implement only the honest Godot route presentation described below.

### Block 7 — Honest Godot route presentation

- Add server-supplied tradeoff/leader/choice/pending/event/objective/summary
  presentation plus rejection, disconnect, keyboard, and accessibility
  fixtures.
- Preserve fixed audiovisual pools and existing semantic flows.

Gate 7 requires semantic harness evidence and creator-reviewable captures. It
does not manufacture a preference claim.

Gate 7 is **approved with bounded Block 8 residuals**.
`docs/operations/m27-block7-godot-audit.md` records the explicit opt-in route
surface, validated server-only projection, leader/pending/replay/terminal and
keyboard/accessibility semantics, MessagePack map16/nil boundary support,
both live Godot routes, independently reconciled persistence/replay/Inspector
truth, three checksum-verifiable captures, the 162-test canonical gate,
ordinary-V2/frozen-V1 preservation, and loopback/resource hygiene. Block 8
may now freeze and execute only the bounded runtime/repetition/resource
matrix below.

### Block 8 — Runtime, repetition, and resource matrix

- Execute the bounded route/event/participant/weapon/build/failure/
  compatibility matrix.
- Reconcile exact objectives, combat, duration, rewards, persistence, replay,
  Inspector, reset, and resources.

Gate 8 requires no integrity, dominance-bound, projection, timeout, migration,
compatibility, privacy, or resource blocker.

Block 8's exact pre-implementation review is **approved** under
`docs/operations/m27-block8-runtime-contract.md`. It freezes a separately
named, non-default-feature server seed-injection binary, eight exact routed
success sessions, bounded compatibility/failure/interruption cases, exact
fresh-prefix durable totals, independent SQL/CLI/Inspector reconciliation,
repeatability hashes, and resource ceilings. The normal/release gateway cannot
read the seed queue, and the real 90-second deadline is not shortened.

Gate 8 is **approved with bounded residuals assigned to Block 9**.
`docs/operations/m27-block8-runtime-audit.md` records the final fresh-prefix
23-session matrix, eight routed successes and twelve routed client
projections, failure/incomplete withholding, ordinary-V2/current-V1/frozen-V1
compatibility, exact persistence/replay/Inspector reconciliation, deterministic
laboratory hashes, bounded resources, and the uninterrupted 162-test canonical
gate. Block 9 may now perform only the complete M27 closure review.

### Block 9 — M27 closure

- Review the complete M27 diff and all accepted/rejected evidence.
- Run the uninterrupted canonical quality gate and protected-invariant audit.
- Classify residuals, decide Gate M27, and record exactly one bounded M28
  starting proposition without implementing M28.

Only a green Gate M27 permits M28 specification work.

Gate M27 is **approved with bounded residual risk (green)**.
`docs/operations/m27-closure-audit.md` records the complete checkpoint diff,
all accepted and rejected evidence, the final uninterrupted 162-test quality
gate, protected invariants, honest residuals, and exactly one bounded M28
starting proposition. M27 is complete; M28 may begin only at its Gate 1
specification and baseline audit.

## Stop conditions

- A third route, more than two candidate events per route, more than one
  resolved event, reroll, chain, arbitrary effect, random reward, or unbounded
  content/objective growth.
- Client-provided/derived seed, client event/reward resolution, timing-dependent
  selection, unstable resolver, retry reroll, or different participants seeing
  different routed truth.
- Route dominance across the complete matrix, lethal optimal pressure, stat or
  arithmetic overflow, M25/M26 catalog mutation, or duration outside the
  reviewed bound.
- Old-V2/frozen-V1 baseline change, implicit route selection, unsolicited new
  message to an unopted participant, or capability inferred from identity/build.
- Unbounded Lua execution/memory/source/graph, executable content callback,
  unknown field acceptance, ambiguous/cyclic/unreachable objective path, or
  hot reload into an active session.
- Duplicate operation, partial seed/event/summary/reward, reward on failure,
  failed operation recorded as activity completion, migration data loss,
  non-idempotent DDL, or reconstruction that consults current mutable state.
- Optimistic/dishonest client confirmation, Inspector mutation, secret or
  unnecessary personal-data collection, public listener, or another person.
- Protocol, schema, replay, Lua, gateway, client, V1, version, or package change
  outside its reviewed block.
- Any engineering result represented as outside-player preference, enjoyment,
  comprehension, accessibility validation, or external playtest evidence.

## Definition of done

M27 is complete only when exactly two opt-in routes and at most one resolved
event are frozen; every route remains non-dominated and inside combat/duration
bounds; the compatibility baseline is exact; Lua authoring is finite;
selection and terminal state are authoritative, idempotent, transactional, and
recoverable; replay reconstructs independently; the client presents server
truth; bounded runtime/resource/compatibility matrices pass; residuals are
honest; and Gate M27 is explicitly approved.
