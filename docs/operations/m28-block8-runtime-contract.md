# M28 Block 8 — runtime, recovery, and resource matrix contract

Date: 2026-09-07  
Review state: **accepted before Block 8 harness edits**  
Scope: bounded validation instrumentation, normal local activity preparation,
one reproducible runtime matrix, external evidence, and Gate 8 audit only.

## Frozen product boundary

Block 8 does not change the accepted cooperation/domain behavior, target or
timing constants, reward, actor life rules, M25 weapon arithmetic, M26 module
catalog/arithmetic, M27 route/event catalog, Lua revision, schema/migrations,
replay vocabulary, Protocol V2 shapes, gateway lifecycle, Inspector/Godot
projection, frozen V1 files, package, or `VERSION=0.2.0`.

The exhaustive 900-row pure report and 50-vector lifecycle report remain the
complete mathematical authority. This block samples the real runtime and may
not retune from observed wall time. Validation clients can choose an existing
weapon, module loadout, route, or cooperation intent already authorized by the
protocol; they cannot send a role, coordinate outside the fixed action, timer,
health, damage, contribution, reward, terminal, seed, or peer decision.

All module ownership, fragments, equipment, and progression used in the
runtime sample are earned through completed normal activities and accepted
post-completion module operations. SQL only inspects. No fixture row, account,
inventory, progression, module, operation, or replay event may be inserted or
updated by the harness.

## Exact cooperation build sample

The accepted profile sample contains exactly four additional successful
two-client cooperation sessions. Admission order fixes the first profile as
anchor and the second as runner:

| Case | Anchor | Runner | Fresh/reused |
| --- | --- | --- | --- |
| `build-success-01` | pulse rifle + Empty | arc sidearm + Force+Ward | fresh / reused |
| `build-success-02` | arc sidearm + Force | pulse rifle + Ward | reused / reused |
| `build-success-03` | pulse rifle + Ward | arc sidearm + Force | reused / reused |
| `build-success-04` | arc sidearm + Force+Ward | pulse rifle + Empty | reused / fresh |

This covers both weapons and Empty, Force, Ward, and Force+Ward in each role.
Expected admitted profiles are the existing M26 values:

| Build/weapon | Damage | Range | Cooldown | Maximum health |
| --- | ---: | ---: | ---: | ---: |
| Empty/rifle | 40 | 6 | 250 ms | 100 |
| Empty/sidearm | 25 | 8 | 150 ms | 100 |
| Force/rifle | 48 | 6 | 300 ms | 100 |
| Force/sidearm | 30 | 8 | 180 ms | 100 |
| Ward/rifle | 40 | 6 | 275 ms | 120 |
| Ward/sidearm | 25 | 8 | 165 ms | 120 |
| Force+Ward/rifle | 48 | 6 | 325 ms | 120 |
| Force+Ward/sidearm | 30 | 8 | 195 ms | 120 |

Six reused characters are prepared normally. A Force+Ward character completes
five baseline sessions and combines/equips both modules only after the fifth
completion. A Force or Ward character follows that same five-session path,
then completes one more baseline matrix session and changes to the required
whole loadout for its next admission, including a same-ID replay check. The two
Empty characters enter their first activity directly in the cooperation
sample. Thus preparation is exactly 34 successful baseline sessions.

For each build success the bot must retain the Gate 6 negative probes and
prove one shared immutable start, same-input replay, conflict rejection, one
ping, one scripted runner downing, one cancelled channel followed by one
completed revive, rejection of a second revive, route exclusion, Warden
completion, five contributions, active terminal participants, and two ordered
two-fragment/125-XP grants. SQL must match the exact weapon, canonical module
loadout, damage, range, cooldown, maximum health, life, and admission order.

Across the eight build-sample characters the exact durable ledger after the
four successes is:

- eight accounts, 38 sessions, and 42 per-character activity histories;
- four succeeded cooperation rows, eight cooperation participants, eight item
  grants of quantity two, and eight progression grants of 125 XP;
- 42 inventory-grant rows, 50 total fragments awarded, 24 fragments consumed,
  and 26 current fragments;
- 4,400 total XP;
- six materialized module states, twelve owned-module rows, eight equipped-module rows,
  ten summed loadout revisions, and 22 accepted module-operation rows; and
- zero M27 route row/event and at most one of every cooperation replay kind per
  cooperation session.

## Failure, recovery, and terminal sample

The unchanged bounded role-bot matrix runs under a fresh prefix and contains
exactly 16 two-client sessions:

- one complete success with retry/conflict/cancellation/second-action probes;
- real monotonic `failed_ping_timeout`, `failed_revive_timeout`, and
  `failed_operation_timeout`; and
- both anchor and runner disconnecting in each of the six nonterminal phases,
  producing twelve `abandoned_disconnect` terminals.

Each failure/abandonment must have exactly one start and terminal replay event,
the correct optional subject, zero route row, zero activity history, zero item
grant, zero XP grant, no generic completion, and a clean reset before the next
admission. The success must have two equal reward/history rows.

The fifth terminal family, `failed_participant_defeated`, is intentionally not
made reachable by weakening production health or increasing production enemy
pressure: a correctly revived participant has 50 health while the frozen
optimal Warden path can deal at most 30. Block 8 samples this branch through
the current composed gateway integration test plus its real PostgreSQL/replay
tests. Those tests must prove fatal Warden damage, `active → defeated`, typed
subject/no-reward summary, transaction ordering, reconstruction, and zero
reward. No matrix-only health/damage override is authorized.

An incomplete/crash state remains covered by the accepted persistence/replay
fixtures. Runtime disconnects are terminal abandonment, not fabricated crash,
resume, replacement, or reconnect behavior.

## Compatibility and exclusivity sample

The same fresh evidence run additionally contains:

- one two-client ordinary, unopted current-V2 baseline completion with one
  fragment/100 XP per participant and no cooperation or route message/row;
- one solo complete current-V2 M27 Breach route selected through normal server
  entropy, with its existing two-fragment/100-XP reward and no cooperation
  message/row;
- one solo current Protocol-V1 baseline completion with one fragment/100 XP
  and no module/route/cooperation evidence; and
- the byte-frozen V1 client joining through the current compatibility adapter,
  with unchanged source hashes and no new-protocol evidence.

Cooperation success already attempts an M27 choice and must receive a bounded
rejection without creating route state. The complete M27 route must remain
cooperation-legacy. Ordinary V2 and both V1 probes must receive no unsolicited
cooperation variant.

## Replay, Inspector, and SQL evidence

Every cooperation and compatibility session is resolved from its fresh
account prefix and reconciled against direct SQL, current CLI reconstruction,
and current-source GET-only Inspector summary/events. Required checks include:

- exact session/participant/operation cardinality and immutable role/profile;
- ordered contribution, life, ping, revive, terminal, and generic reward
  replay cardinality with no post-terminal mutation;
- exact timer ordering and success/failure reward arithmetic;
- route/cooperation mutual exclusion;
- no partial participant reward/history;
- reconstruction agreement without consulting mutable actor state; and
- GET 200 plus HEAD/POST/PUT/PATCH/DELETE/OPTIONS 405 on Inspector summary.

The evidence directory is unique, external to the repository, prefix-isolated,
bounded, checksum-verifiable, and contains only synthetic local identities.
It contains gateway/client logs, structured session ledgers, Inspector JSON,
CLI replay, SQL assertions, pure reports, resource measurements, summary, and
`SHA256SUMS`. An existing target is never overwritten or promoted.

## Repetition and resource bounds

The cooperation matrix JSON and domain lifecycle JSON are generated twice,
before and after runtime execution. Each pair must be byte-identical, retain
900/50 rows, and match the accepted Gate 2 hashes.

Each isolated gateway is sampled before and after its complete group. It must
remain within:

- RSS growth at most 64 MiB;
- at most four retained file descriptors;
- at most four retained threads; and
- at most two retained PostgreSQL connections.

Logs must contain the exact expected reset count and no unexpected command,
connection, persistence, session-abort, panic, or seed-instrumentation error.
All isolated processes and listeners stop afterward. Permanent PostgreSQL,
Gateway, and Inspector stay healthy and loopback-only. The public schema ends
with zero non-internal trigger and zero function.

## Quality gate and stop conditions

The uninterrupted canonical `make check`, all capture manifests, Godot M17–M28
semantic markers, production Compose image builds, `git diff --check`, frozen
V1 hashes, M27 checkpoint, version, loopback listeners, and clean failure-
injection boundary remain mandatory.

Block 8 stops rather than widens scope if it requires SQL fixture mutation,
client/server outcome or health injection, weaker pressure/timing, a new
weapon/module/route/event, schema/protocol/replay/Godot behavior change,
public listener, another person, unbounded evidence, or any preference,
comprehension, accessibility, teamwork, anti-griefing, or enjoyment claim.

Harness expectation errors may be corrected only when authoritative evidence
shows the product behavior stayed within this contract. Rejected evidence
remains separate and is never promoted. Any integrity, authority, projection,
transaction, compatibility, privacy, reset, or resource failure keeps Gate 8
red.
