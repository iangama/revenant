# M27 Block 8 — runtime, repetition, and resource matrix contract

Date: 2026-09-05  
Review state: **accepted before Block 8 source edits**  
Scope: bounded test instrumentation, one reproducible local matrix, external
evidence, and Gate 8 audit only.

## Frozen product boundary

Block 8 does not change the accepted route/event/module catalogs, Lua
revision, schema or migration, replay vocabulary, Protocol V2 messages,
gateway lifecycle rules, Inspector projection, Godot presentation, frozen V1
files, package, or `VERSION=0.2.0`. The exhaustive 240 routed plus 60
compatibility rows from Gate 2 remain the complete mathematical authority;
this block samples the real runtime and does not retune from observed timing.

No network message may contain or influence a seed. The normal
`revenant-gateway` binary continues to obtain one seed only from the operating
system and must not read matrix seed configuration. Block 8 may add a
non-default Cargo feature and a separately named, required-feature local test
binary. That binary accepts a finite process-local seed queue, validates every
value as a non-negative 63-bit `RouteSeed`, consumes exactly one entry only
after a valid first selection reaches the established server seed boundary,
and fails closed when exhausted. It must print an explicit test-instrumentation
banner and reuse the same session, persistence, replay, protocol, and listener
logic as production. The feature is disabled by default and excluded from
release commands.

Fake-client additions are validation instrumentation only. They may observe
and assert server responses but may not provide a seed, resolve an event,
choose for a peer, invent a reward, or change production authority.

## Exact successful runtime sample

The accepted success matrix contains exactly eight routed sessions:

| Participants | Route/event seed | Weapon | Admitted build coverage |
| ---: | --- | --- | --- |
| 1 | Breach / seed 0 / `overcharged_armor` | rifle | empty, fresh |
| 1 | Breach / seed 1 / `arc_surge` | sidearm | Force+Ward, reused |
| 1 | Stabilize / seed 0 / `shielded_channel` | rifle | Force, reused |
| 1 | Stabilize / seed 2 / `residual_feedback` | sidearm | Ward, reused |
| 2 | Breach / seed 0 / `overcharged_armor` | rifle | empty + Force+Ward |
| 2 | Breach / seed 1 / `arc_surge` | sidearm | Force+Ward + Ward |
| 2 | Stabilize / seed 0 / `shielded_channel` | rifle | Force + Force+Ward |
| 2 | Stabilize / seed 2 / `residual_feedback` | sidearm | Ward + Force+Ward |

The exact build sequence may use accepted post-completion whole-loadout
changes so the requested profile becomes active only on the following
admission. Every such change is retried with the same operation identifier and
must return the identical `replayed=true` state. All module ownership and
fragments are earned through normal activities and combinations; SQL may
inspect but must not create fixture state.

For every routed success, each active client must record its exact admitted
damage/cooldown/maximum health, server option and accepted selection, route,
seed, event/effect, objective path, Warden spawn health, every accepted hit and
remaining-health value, counter count/damage, hostile damage, elapsed/budget,
transition count, per-character reward, and terminal grant set. Both clients
in a two-active session must agree on all shared truth while retaining their
distinct admitted profile facts. The first-admitted client alone submits the
accepted choice.

Every two-active success also samples a real non-leader choice attempt before
the leader selection. It must be rejected without consuming a seed, reserving
an identifier, changing phase, or exposing a selection. The leader then proves
first acceptance, same-input retry, different-route conflict, one immutable
selection, and normal completion.

## Compatibility, failure, and interruption sample

The same isolated matrix additionally runs:

- ten ordinary unopted V2 bootstrap activities on two reused characters,
  earning and combining exactly Force and Ward without route state;
- one fresh ordinary V2 completed baseline and one completed Protocol-V1
  gameplay probe, both with the exact one-fragment/100-XP baseline and zero
  route row/event/message;
- the byte-frozen V1 client against the current compatibility adapter, with
  its exact source hashes and no route evidence;
- one two-active capability mismatch in which only the leader opts in, its
  route attempt is rejected before seed use, both clients complete the
  ordinary baseline, and no route row/event is written;
- one accepted routed disconnect after selection, producing one durable
  selection and reconstructible incomplete operation but no terminal,
  completion, history, or reward; and
- one real monotonic deadline run that selects Breach, waits beyond the exact
  90,000 ms budget, triggers authoritative observation, receives
  `failed_timeout` with a failed active objective and
  `deadline_exceeded`, and receives no completion or reward.

Existing unit/domain/persistence coverage remains responsible for the exact
90,000 ms success boundary and all write-failure rollback boundaries. The
runtime case waits for the real boundary rather than adding a network or
production clock override.

## Exact evidence ledger

For a fresh matrix prefix, the expected durable totals are:

- nine prefixed accounts, 23 distinct prefixed sessions, 28 participant joins,
  21 completed sessions, and 26 per-character completion histories;
- ten route rows and `route_selected` events: eight succeeded, one
  `failed_timeout`, and one incomplete;
- twelve route-success participant grants, 26 inventory reward grants, 26
  progression grants, a final 24-fragment balance, and 2,900 total XP across
  the prefixed characters;
- two module states, four owned-module rows, four accepted combination
  operations/events, eleven accepted loadout operations/events, and the exact
  final loadouts established by the scenario order; and
- zero route row/event for the unopted V2, V1, frozen V1, capability-mismatch,
  or bootstrap sessions.

Every prefixed session is reconciled against direct SQL, current CLI
reconstruction, and current-source GET-only Inspector summary/events. Success
requires exact append counts and payload identity, one terminal at most, no
post-terminal event, no partial participant grant, reward only on success,
failure/incomplete without history, and reset before the next admission.

The evidence directory is unique, external to the repository, immutable for
the accepted run, and contains client/gateway logs, structured client rows,
Inspector responses, CLI timelines, SQL assertions, pure/domain reports,
resource measurements, summary, and `SHA256SUMS`. Reusing an existing target
is rejected.

## Resource and repeatability bounds

Separate one- and two-participant matrix gateways are sampled before and after
their complete session groups. Each must return within:

- RSS growth at most 64 MiB;
- at most four retained file descriptors;
- at most four retained threads; and
- at most two retained PostgreSQL connections.

Logs must contain the exact reset count, an explicit matrix-seed banner, no
seed exhaustion, and no unexpected command/connection/session-abort failure.
All isolated processes and listeners stop afterward. Permanent services stay
healthy and loopback-only.

The operation lab JSON and domain JSON are generated twice around the runtime
work and must be byte-identical with stable SHA-256 hashes, 240 routed rows, 60
baseline rows, four reachable events, and no dominance. The final canonical
quality gate, capture manifests, absence of untracked `.gd.uid`, exact frozen
V1 hashes, and zero installed failure-injection trigger/function remain
mandatory.

## Stop conditions

Block 8 stops rather than widens scope if it requires a default/release seed
override, a client-provided seed or outcome, SQL fixture mutation, a third
route/event, altered tuning, shorter production deadline, protocol/schema/
replay/Godot change, unbounded evidence, public listener, another person, or a
preference/comprehension/enjoyment claim.

Any integrity, dominance-bound, projection, deadline, migration,
compatibility, privacy, reset, or resource failure keeps Gate 8 red. Harness
expectation errors may be corrected only when persisted authoritative evidence
shows the product behavior remained inside this contract; rejected runs stay
recorded and are never promoted as accepted evidence.
