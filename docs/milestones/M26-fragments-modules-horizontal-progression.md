# M26 — Fragments, modules, and horizontal progression

Status: complete; Gates 1-8 and Gate M26 approved with bounded residual risk.

## Authorization and boundary

The owner authorized sequential execution of the already-sketched solo
milestones M25-M32. M25 is complete and Gate M25 is green. M26 is therefore
authorized, but every block below remains behind its preceding gate.

M26 is creator-only local engineering work. It may use deterministic bots,
generated inventories, creator-operated Godot flows, and loopback services. It
does not authorize another person, public networking, a commit, push, merge,
version change, tag, release, distribution, or deletion of prior evidence.

The server owns fragment balances, recipes, module ownership, compatibility,
loadout state, operation idempotency, effective combat arithmetic, admission
state, persistence, and replay. A client may request, preview, and present; it
may never grant, consume, equip, resolve, or confirm a module locally.

Protocol V2, the existing replay vocabulary, the current PostgreSQL schema,
frozen V1 artifacts, the M24 package, and `VERSION=0.2.0` remain unchanged
through Gate 1. Each future Protocol, replay, or schema expansion has its own
explicit block and gate below. No change may leak into an earlier block.

## Single starting hypothesis

> A bounded three-slot module loadout with four tradeoff families can produce
> at least three non-dominated combat builds while combination, equip, grant,
> persistence, migration, and replay arithmetic remain server-authoritative,
> idempotent, and exactly reconstructible.

This is an engineering proposition to falsify with bounded arithmetic and
runtime evidence. It is not a claim about human preference or long-term
retention.

## Audited baseline before M26

- `revenant-inventory` has three catalogued item identifiers: two unique
  weapons and stackable `relay_core_fragment`. Activity rewards must be known
  and positive, but inventory has no subtraction or maximum-quantity domain
  operation.
- `relay_awakening` grants one fragment and 100 XP per participating character.
  `(session_id, character_id, item_id)` and `(session_id, character_id)` grant
  keys make completion rewards idempotent in one PostgreSQL transaction.
- Progression is total XP plus a derived level. It currently has no combat
  effect and M26 does not change that contract.
- Equipment has one persisted weapon slot. The gateway verifies ownership and
  equipability, commits equipment plus replay atomically, and keeps an active
  attack cooldown across weapon switches.
- Protocol V2 has inventory, progression, and equipment snapshots plus weapon
  equip intent/result messages. It has no module request, preview, mutation,
  snapshot, or result message. Frozen V1 has none of these V2 systems.
- A session starts as soon as its expected participants join. Weapon changes
  are currently allowed during combat and refused after completion. This
  lifecycle is unsafe for mutable health/stat modules and cannot be copied.
- PostgreSQL has five idempotent SQL files concatenated and applied under an
  advisory transaction. There is no destructive migration and no migration
  ledger. Module state does not yet exist.
- The 2026-08-31 local database observation contained 239 accounts/characters,
  622 inventory rows, 215 lazily established weapon loadouts, and 4,972 replay
  events. These mutable counts are migration fixtures, not product metrics.
- Replay recognizes nine event kinds and reconstructs bounded session counts
  from append-ID order. Payloads are currently human-readable text; there is no
  structured module arithmetic to reconstruct.
- The Inspector is read-only and displays ordered replay events plus a bounded
  authoritative summary. It has no mutation endpoint.
- Godot projects flat inventory quantities, progression, selected weapon, and
  server-supplied weapon profiles. It has no module state or comparison model.

The working tree intentionally contains the reviewed M24-M25 work without a
commit. Before the first M26 implementation source change, Block 2 must create
a checksum-verifiable M25 closure checkpoint outside the repository and record
its path, byte size, file count, and SHA-256.

That checkpoint was accepted before any M26 implementation edit:

`/mnt/c/Users/Ian/revenant-local-checkpoints/revenant-m25-closure-45718926-20260831.tar.gz`

It contains 290 repository source/evidence files, excludes `.git`, ignored
build output, and dependency caches, is 9,841,452 bytes, and has SHA-256
`25e5eed51c11534569f0daf550af05990f927060c6c5b6766bc62fc22777ea83`.
The first archive command exited on a positional-option warning after creating
an unaccepted partial target; that exact generated target was removed and the
complete command was rerun successfully. Only the successful archive above is
checkpoint evidence.

## Taxonomy decision questions

The taxonomy is driven by explicit questions rather than unrecorded design
assumptions:

| Question | Gate 1 decision or required evidence |
| --- | --- |
| What resource is combined? | Only the existing `relay_core_fragment`; no second currency. Exact recipe costs are selected by Gate 2 arithmetic within the bounds below. |
| Is a module a random instance or a catalog unlock? | A fixed, character-owned catalog unlock with quantity capped at one. No roll, seed, rarity, tier, level, affix, durability, or instance identity. |
| Why four families with only three slots? | Selecting at most three of four forces one explicit sacrifice. The four provisional axes are Force, Tempo, Reach, and Ward; exact names and magnitudes require Gate 2 evidence. |
| Are slots effect-specific? | No. Three presentation positions serialize in canonical catalog-family order; permutations never create different builds. |
| Can a family or module be repeated? | No. M26 has one catalog entry per family and at most one entry from each family in a loadout. |
| When may combination/loadout mutate? | Only after authoritative activity completion. A changed loadout applies to the next admitted session, never the completed or active one. |
| What state receives effects? | Both owned weapon profiles and the Operator's next-session maximum health, using one character-global module loadout. |
| May a client preview locally? | No authoritative result may be computed locally. A later bounded server preview may return candidate effective values without mutation. |
| Is dismantling, refund, trading, discard, or transfer needed? | No. They are outside M26; rejection paths must not consume fragments. |
| How does legacy state migrate? | Every existing character starts at revision zero with an empty module loadout and unchanged inventory/combat behavior. No synthetic module or fragment is granted. |
| What owns the state? | The selected character, consistent with inventory, progression, equipment, and activity rewards. |

Gate 2 may tune the four fixed entries or reject the taxonomy. It may not add a
fifth family, random generation, tiers, or more slots without stopping and
reopening this specification gate.

## Bounded catalog, slots, and recipes

- M26 contains exactly four module catalog entries, one per family.
- A character owns each module either zero or one time. Recombining an already
  owned module is a deterministic rejection with no fragment change.
- The loadout contains zero to three unique modules. Server serialization is
  canonical by family order, so the complete state space is
  `C(4,0)+C(4,1)+C(4,2)+C(4,3)=15` builds, never slot permutations.
- Every recipe consumes only `relay_core_fragment`, costs 1-8 fragments, and
  grants exactly one fixed module. Gate 2 freezes the four exact costs.
- A combination operation identifier is 1-32 ASCII alphanumeric/hyphen
  characters. It is scoped to character and operation kind. Once a mutation is
  accepted, retrying the same identifier and canonical input returns the
  original accepted outcome; reusing that reserved identifier with different
  input is rejected. A rejected request writes no ledger row and may be
  re-evaluated after its precondition changes.
- A loadout mutation submits the whole desired canonical set, not independent
  slot writes. Its accepted revision increases exactly once. Emptying a
  loadout is valid and consumes nothing.
- Unknown, unowned, duplicate-family, over-capacity, malformed, active-session,
  stale-revision, and conflicting-idempotency requests change nothing.

## Arithmetic contract and growth bounds

The four provisional family axes each require a measurable upside and cost:

| Family axis | Required upside | Required cost |
| --- | --- | --- |
| Force | weapon damage increases | weapon cooldown increases |
| Tempo | weapon cooldown decreases | weapon damage decreases |
| Reach | weapon range increases | weapon damage decreases |
| Ward | Operator maximum health increases | weapon cooldown increases |

Gate 2 chooses exact signed modifiers inside these per-entry envelopes:

- damage: 500-2,000 basis points in the required direction;
- cooldown: 500-2,000 basis points in the required direction;
- range: +1 or +2 authoritative distance units;
- maximum health: +10 to +30.

Effects are additive signed modifiers against immutable M25 base profiles,
then resolved once with checked integer arithmetic. Damage and cooldown use
the sum of basis points and round half upward after multiplication; range and
health use checked integer addition. Catalog order and request order cannot
change a result. Aggregate modifiers must stay within:

- damage: -3,000 to +3,000 basis points;
- cooldown: -3,000 to +3,000 basis points;
- range: 0 to +2 units;
- maximum health: 0 to +30.

Every accepted effective state must remain within damage 18-52, range 4-10,
cooldown 120-350 ms, and maximum health 100-130. Arithmetic rejects overflow,
underflow, a zero stat, an unknown catalog revision, or an out-of-envelope
catalog entry. It never saturates silently.

For the same base weapon, build A dominates build B only when A has damage,
range, and maximum health greater than or equal to B, cooldown less than or
equal to B, and at least one strict improvement. The generated matrix must
contain at least three Pareto-non-dominated builds for each weapon, and no one
build may dominate every other build. All 15 builds must also retain positive
damage, bounded cooldown/range/health, non-lethal optimal M25 pressure, and the
approved encounter-time safety envelope. This Pareto test is a bounded design
check, not proof of human preference.

## Lifecycle and authority invariants

- The loadout is copied into immutable participant combat state at admission.
  The session does not query mutable database loadout state per attack.
- Combination, preview, and loadout requests are V2-only. Mutating requests are
  accepted only for a participant in `Complete`; preview may be available
  earlier but cannot mutate or prime a later acceptance.
- A post-completion accepted mutation affects only a future session. It does
  not heal, damage, alter cooldown, rewrite an actor, or revise the completed
  replay.
- On the next admission, server-resolved maximum health owns the player actor's
  initial/max health and server-resolved weapon profiles own damage, range, and
  cooldown. An active cooldown deadline remains attacker-owned.
- Multiplayer resolves each participant independently from that character's
  immutable admitted loadout. Shared enemy health and rewards retain their M25
  contracts.
- Module effects are active only for current V2 participants. A frozen V1
  participant always receives the established base rifle/100-health behavior;
  its persisted V2 module loadout is ignored, never consumed or rewritten.
- Disconnect, reset, failed persistence, malformed intent, or replay failure
  exposes no partial in-memory state or optimistic client confirmation.

## Persistence, migration, and rollback invariants

Block 4 must review one additive idempotent `0006` migration before applying it
to the working database. The expected minimal model is existing inventory rows
for module ownership, zero-to-three module-loadout rows, a monotonic loadout
revision, and bounded operation ledgers for combination/loadout idempotency.
Exact table and constraint names remain a Block 4 decision.

Required behavior:

- migration runs inside the existing advisory transaction, succeeds twice,
  and never deletes, renames, rewrites, or reinterprets an existing row;
- legacy characters resolve to empty revision-zero state without backfill;
- a pre-migration custom-format `pg_dump`, schema inventory, table counts, and
  hashes of existing logical rows are stored outside the repository;
- the migration is tested first against a restored disposable database, not
  first against the working database;
- combination locks the character's fragment/module rows, verifies catalog,
  ownership, cost, and operation identity, then atomically records the
  idempotency result, decrements fragments, grants the module, and appends its
  replay event;
- loadout mutation locks revision/ownership, canonicalizes before comparing an
  idempotency payload, replaces all slots, increments revision once, and
  appends replay in the same transaction;
- same-operation retries return the persisted result without a second consume,
  grant, revision, or replay event;
- validation/business rejection writes no operation or replay row and leaves
  the identifier unreserved; only an accepted atomic mutation reserves it;
- injected failure at every write boundary leaves inventory, loadout,
  operation ledger, revision, replay, and projection unchanged; and
- rollback means transaction rollback plus restoration proof on a disposable
  database. No destructive down migration or live-data deletion is required.

The additive tables must remain inert and harmless if pre-M26 code is run
against the migrated schema.

## Replay and reconstruction contract

Block 5 separately reviews the vocabulary expansion. The candidate minimum is
`module_state_snapshot`, `module_combined`, and `module_loadout_changed`.
Names and payload structs are not accepted until that gate.

New payloads must be deterministic structured JSON stored in the existing
bounded replay payload column. The join snapshot records enough state to
reconstruct without querying current inventory or today's catalog: catalog
revision, fixed modifiers, fragment/module ownership, loadout/revision, base
profiles, effective profiles, and maximum health. Mutation events record
operation identity plus exact before/after quantities, loadout, revision, and
effective arithmetic.

`player_joined` and one per-participant module join snapshot must persist
atomically before admission. A V1 snapshot explicitly records modules inactive
and base stats, so mixed V1/V2 sessions remain reconstructible without changing
the V1 wire contract. Combination/loadout persistence and their replay event
are also one transaction. Reconstruction follows replay append IDs, never
wall-clock order, and must reject mixed sessions, missing required M26
snapshot, invalid revision, impossible balance, unknown modifier, or arithmetic
mismatch. Old sessions without module events reconstruct under the explicit
legacy rule: empty revision-zero loadout and M25 base profiles.

The Inspector remains read-only. It may add bounded module counts and decoded
before/after detail, but no mutation endpoint, raw SQL, secret, or client-owned
truth.

## Protocol review contract

Block 6 separately decides any additive Protocol V2 surface. The candidate
shape is:

- client: module-state request, server-preview request, combine intent, and
  whole-loadout intent;
- server: module snapshot, authoritative preview, combination result, and
  loadout result.

The ordinary V2 join sequence remains valid for a client that never sends a
module-state request; the server must not inject an unsolicited message that
breaks the existing sequence. Effective weapon values in the existing
equipment snapshot remain server-owned. Every new string/list/count is
bounded, every rejection is explicit, and only accepted mutation results may
alter client projection.

Frozen V1 messages/files/hashes remain byte-identical, receive no module
message, and retain base combat arithmetic even when the same character owns a
V2 loadout. If additive V2 cannot preserve the compatibility harness and frame
bound, Block 6 stops and proposes a separately reviewed protocol-version
milestone; it does not silently create Protocol V3 or reinterpret V2.

## Comparison UI truth boundary

- Godot displays four fixed module choices, exact owned/fragment quantities,
  the three canonical positions, recipe cost, current revision, and the
  authoritative current effective profile.
- Candidate before/after damage, cooldown, range, and max health comes only
  from a server preview response and is labeled as a preview until accepted.
- Combine/loadout buttons are disabled outside the authoritative completed
  state and explain that changes apply next session.
- Sending an intent may show a neutral pending state. It may not subtract a
  fragment, add a module, fill a position, change stats, or play success until
  the accepted server result arrives.
- Rejection, stale revision, insufficient fragments, already-owned module,
  disconnect, and retry remain distinguishable from success.
- Keyboard navigation, visible focus, mute, Reduced Flash, bounded effects,
  existing HUD information, and M17-M25 flows remain intact.
- The UI does not describe a build as best, fun, preferred, or recommended.
  It presents exact tradeoffs and lets the creator choose.

## Generated matrix and runtime evidence contract

The pure matrix must emit stable machine-readable JSON and a reviewable table
covering:

- all 15 canonical loadouts for both weapons (30 resolved build rows);
- all four recipes at fragment counts 0, cost-1, cost, cost+1, owned, retry,
  and conflicting-operation cases;
- all valid subsets plus unknown, unowned, duplicate, fourth-entry,
  non-canonical-order, stale-revision, and malformed identifiers;
- checked arithmetic boundaries, request-order invariance, catalog-order
  invariance, Pareto frontier, encounter TTK, hostile pressure, and frame-size
  estimates; and
- deterministic rerun hashes.

Post-persistence evidence adds migration-twice, disposable restore, existing-row
reconciliation, every-write failure injection, concurrent/same-operation
retry, and exact replay reconstruction.

The final local runtime matrix selects the empty baseline plus at least three
Pareto-distinct builds per weapon. It exercises fresh and reused characters,
combine/preview/equip/reconnect, insufficient/retry/conflict, solo and two
active clients, both weapons, exact combat/reward state, Inspector/replay
agreement, reset, and bounded RSS/descriptors/threads/database connections.
Pure coverage proves the full 15-state space; runtime sampling does not pretend
to be exhaustive human evaluation.

## Block plan and gates

### Block 1 — Specification and audited baseline

- Freeze this hypothesis, baseline, taxonomy questions, finite bounds,
  lifecycle, evidence contract, stop conditions, and explicit change blocks.
- Review the current source, live local schema shape, and prior compatibility
  boundaries without implementation edits.

Gate 1 requires a falsifiable finite design, no hidden Protocol/schema/replay
change, and no unresolved choice that could multiply the state space beyond 15
before the pure laboratory. It authorizes only Block 2.

Gate 1 is **approved**. Source and live-schema review confirmed the baseline;
the one hypothesis is falsifiable; catalog, slot, recipe, arithmetic, lifecycle,
matrix, migration, replay, UI, and stop bounds are finite; and exact tuning
choices are confined to the Gate 2 envelopes. The structural review found no
M26 implementation and no diff under Protocol, replay, frozen V1, or version.
Block 2 is authorized after its required external M25 closure checkpoint.

### Block 2 — Pure arithmetic and non-dominance laboratory

- Preserve the external M25 source checkpoint before the first source edit.
  Completed with the path and digest recorded above.
- Implement dependency-light module catalog/arithmetic and a local matrix CLI.
- Select exact four modifiers and recipe costs from complete generated output.
- Prove invalid catalogs, overflow, rounding, order invariance, state-space
  bound, encounter safety, and Pareto claims.

Gate 2 freezes or rejects the candidate catalog. It changes no database,
protocol, replay vocabulary, gateway, or client.

Gate 2 is **approved**. The complete 30-row matrix, exact catalog/costs,
deterministic report hash, rejected pre-gate evidence, nine focused tests, and
66-test workspace gate are recorded in
`docs/progression/m26-block2-module-lab-audit.md`. All 15 builds per weapon are
Pareto non-dominated and remain inside timing/pressure bounds. Block 3 is
authorized; later change surfaces remain locked behind their own gates.

### Block 3 — Authoritative combination/loadout domain

- Implement pure ownership, recipe, whole-loadout, canonicalization, revision,
  lifecycle, operation-key, retry, and conflict behavior.
- Keep persistence behind a testable boundary and expose no network message.

Gate 3 requires exhaustive valid/invalid generated inventory matrices with no
partial domain transition and no state-space growth.

Gate 3 is **approved**. `docs/progression/m26-block3-domain-audit.md` records
the 34-case deterministic matrix, all 15 canonical loadouts, accepted-operation
retry/conflict, lifecycle/revision/ownership/capacity/overflow/limit rejection,
full state equality on every error, 17 focused tests, and the 74-test workspace
gate. Block 4 is authorized to review exact additive DDL and disposable
backup/restore evidence before any working-database migration.

### Block 4 — Schema, migration, and transactional persistence

- Review the exact additive DDL and pre-migration backup/restore evidence.
- Implement migration, typed persistence, row locks, operation ledgers, atomic
  consume/grant/loadout/revision, and failure-injection fixtures.
- Migrate the working local database only after disposable proof passes.

Gate 4 requires exact preservation of legacy rows, apply-twice success,
idempotent concurrency behavior, and rollback at every write boundary.

Gate 4 is **approved**. `docs/progression/m26-block4-persistence-audit.md`
records the pre-migration dump and manifests, byte-identical preservation of
all nine legacy tables, apply-twice disposable and working migrations, typed
durable state, six PostgreSQL transition/concurrency/limit/failure tests, and
the 80-test workspace gate. The block deliberately added no replay event;
mutation/replay atomicity remains an explicit Gate 5 requirement. Block 5 is
authorized to review exact bounded vocabulary and reconstruction before the
replay enum changes.

### Block 5 — Replay vocabulary, reconstruction, and Inspector

- Review exact new event names and bounded payload structs before changing the
  replay enum.
- Persist snapshot/mutations atomically, reconstruct arithmetic independently,
  and extend the read-only summary/detail fixtures.

Gate 5 requires exact old/new reconstruction, legacy fallback, corruption
rejection, and no Inspector mutation surface.

Gate 5 is **approved**. The pre-implementation vocabulary decision and final
audit are recorded in `docs/progression/m26-block5-replay-contract.md` and
`docs/progression/m26-block5-replay-audit.md`. Three exact bounded event kinds,
V1/V2/legacy reconstruction, every replay write boundary, concurrent retry,
read-only Inspector decoding, CLI parity, 90 workspace tests, and the
production frontend build are green. Block 6 is authorized to review exact
additive Protocol V2 structs before changing protocol or gameplay routing.

### Block 6 — Additive Protocol V2 and gateway integration

- Review exact bounded message structs before editing the protocol.
- Add opt-in request/preview/mutation flow, admitted immutable combat state,
  complete-only mutation, authoritative profiles/health, and V1 routing.
- Update fake/current clients and compatibility fixtures without unsolicited
  join-sequence expansion.

Gate 6 requires protocol round trips, malformed/bounded rejection, old V2 join
behavior, exact V1 hashes/reconstruction, and no active-session mutation.

Gate 6 is **approved**. The exact pre-code contract and implementation audit
are recorded in `docs/progression/m26-block6-protocol-contract.md` and
`docs/progression/m26-block6-protocol-audit.md`. All eight opt-in variants,
maximum frames, immutable admission, targeted preview/mutation, complete-only
lifecycle, next-session activation, distinct multiplayer builds, ordinary old
V2 sequencing, frozen V1 gameplay/reconstruction/hashes, replay/Inspector
parity, 98 workspace tests, strict Clippy, and production Inspector build are
green. Block 7 is authorized only for honest Godot presentation and semantic
fixtures; the taxonomy, protocol, schema, replay, and authority boundaries are
frozen.

### Block 7 — Honest Godot module workshop

- Add server-previewed comparison, owned/recipe/loadout presentation, pending,
  accepted, rejected, reconnect, keyboard, and accessibility fixtures.
- Preserve honest confirmation and fixed audiovisual/resource pools.

Gate 7 requires semantic harness evidence and creator-reviewable captures. It
does not manufacture a creator preference.

Gate 7 is **approved with bounded residuals assigned to Block 8**.
`docs/progression/m26-block7-workshop-audit.md` records the honest bounded UI,
signed/string-array MessagePack correction, actual keyboard focus and reconnect
fixtures, current-gateway graphical flow, checksum-verifiable captures, exact
database/replay/Inspector reconciliation, 98-test technical gate, and final
integrated smoke. No taxonomy, domain, schema, replay vocabulary, Rust Protocol
V2, gateway lifecycle, frozen V1, package, or version change entered the block.
Block 8 is authorized for only the bounded runtime/repetition/resource matrix.

### Block 8 — Runtime and repetition matrix

- Execute the bounded solo/two-active/build/reconnect/idempotency matrix.
- Reconcile combat, fragments, modules, revisions, rewards, replay, Inspector,
  compatibility, reset, and resources.

Gate 8 requires no integrity, dominance-bound, projection, migration,
compatibility, privacy, or resource blocker.

Gate 8 is **approved with bounded residuals assigned to Block 9**.
`docs/progression/m26-block8-runtime-audit.md` records 27 accepted sessions,
35 active-client completions, fresh and reused authoritative progression,
empty/Force/Ward/Force+Ward on both weapons, distinct-build two-active combat,
complete mutation/retry/conflict/reconnect behavior, exact database/replay/
Inspector reconciliation, an inactive-module V1 admission, and bounded reset,
RSS, descriptor, thread, and database-connection evidence. The accepted
125-file manifest is checksum-verifiable. Final integrated smoke and the
98-test technical gate passed without broadening the frozen catalog, domain,
schema, replay vocabulary, Protocol V2, gateway lifecycle, Godot presentation,
V1 artifacts, package, or version. Block 9 is authorized only for complete M26
closure review.

### Block 9 — M26 closure

- Review the complete M26 diff and every accepted/rejected evidence set.
- Run the uninterrupted canonical quality gate and protected-invariant audit.
- Classify residuals, decide Gate M26, and record exactly one M27 starting
  proposition without implementing M27.

Only a green Gate M26 permits M27 specification work.

Gate M26 is **approved with bounded residual risk (green)**. The complete diff,
accepted/rejected evidence, deterministic reports, migration backup/restore,
runtime resources, frozen artifacts, integrated smoke, 98-test technical gate,
security/local-only boundary, and residual classification are recorded in
`docs/progression/m26-closure-audit.md`. M26 is complete. Only the already
authorized M27 specification step may begin from the single proposition
recorded there; no M27 implementation is authorized before its own green
specification gate.

## Stop conditions

- More than four catalog entries, more than three equipped entries, more than
  15 canonical builds, random/tiered growth, or a build that dominates every
  alternative.
- Overflow, order-dependent arithmetic, out-of-bounds stat, lethal optimal
  baseline pressure, cooldown bypass, or mutation of an active/completed actor.
- Negative fragment balance, duplicate module unlock, second consume/grant on
  retry, operation-key alias, stale revision acceptance, or partial loadout.
- Migration data loss/rewrite, non-idempotent DDL, unproved restore, partial
  transaction, replay/persistence mismatch, or reconstruction that depends on
  current mutable state.
- Optimistic client inventory/loadout/stat confirmation, misleading preview,
  unbounded payload/effect growth, or Inspector mutation.
- Protocol, schema, replay, V1, version, or frozen-package change outside its
  reviewed block.
- Any engineering result represented as outside-player preference or external
  validation.

## Definition of done

M26 is complete only when the exact four-entry catalog and 15-state space are
frozen, at least three builds per weapon are non-dominated, combination and
whole-loadout operations are authoritative/idempotent/transactional, migration
preserves legacy data, replay reconstructs exact arithmetic, the UI presents
server truth, bounded runtime matrices pass, all compatibility gates remain
green, residuals are honest, and Gate M26 is explicitly approved.
