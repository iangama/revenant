# M27 Block 6 — Protocol V2 and gateway integration audit

Date: 2026-09-05  
Decision: **Gate 6 green**  
Scope: the reviewed contract in
`docs/operations/m27-block6-protocol-contract.md`.

## Accepted implementation

Block 6 keeps `PROTOCOL_VERSION=2` and the existing 65,536-byte frame ceiling.
It adds exactly the reviewed two client variants, `RouteStateRequest` and
`RouteChoiceIntent`, and three server variants, `RouteState`,
`RouteChoiceResult`, and `RouteOperationSummary`. Every new structure has the
reviewed schema version, typed finite enums, explicit collection/string
bounds, and nested unknown-field rejection. The maximum-shaped route fixtures
remain below the private 8 KiB route-message budget and the public frame
ceiling. Route intent contains only operation ID and route ID; it exposes no
client seed, event, effect, reward, duration result, participant, or terminal
authority.

The compatibility adapter canonicalizes the new requests only for current V2.
An ordinary V2 client that does not request capability follows the unchanged
baseline and receives no route variant. Frozen V1 cannot submit a route
request and receives no route response.

The gateway now owns the complete reviewed lifecycle:

- an explicit per-participant capability bit, complete admitted-set check, and
  first-admitted leader;
- deterministic input/phase/capability validation before one OS-generated
  non-negative 63-bit seed is requested;
- atomic persisted selection plus replay evidence before any accepted result;
- immutable same-result retry, conflict/no-reroll behavior, and recovery from
  an already-durable same-input selection without generating a replacement
  seed;
- Breach's direct door path and Stabilize's authoritative `[3, 0, 3]`
  stabilizer step before the existing door;
- event-driven Warden health and counter-pressure values applied to the
  authoritative encounter once;
- an `Instant`-based 90,000 ms budget whose exact boundary succeeds and whose
  first observation strictly after the boundary fails;
- ordered objective/encounter/grant evidence passed to the accepted Block 5
  atomic success or timeout transaction; and
- route state, actors, rewards, generic completion, and opted-in terminal
  projection only after durable commit.

Disconnect may preserve an incomplete durable operation without inventing a
terminal. Full session reset clears capability, route, seed/deadline,
transition, and encounter state.

## Focused proof

Protocol tests cover both client and all three server round trips,
maximum-shaped state/choice/summary messages, oversized pre-allocation frame
rejection, nested unknown fields, and invalid enums. Compatibility tests cover
current route canonicalization and frozen-V1 rejection.

The gateway suite contains 33 passing tests. Its Block 6 cases prove explicit
capability, shared leader truth, non-leader and incomplete-set rejection,
invalid operation/route rejection before seed generation, applied selection,
same-result retry, conflict, and one seed generation. They also prove:

- no unsolicited variant or route persistence for ordinary V2;
- both route paths and all four event effects through injected seeds;
- exact Warden scaling, counter pressure, transition paths, and route rewards;
- identical selection and terminal truth for two active opted-in participants,
  including one exact grant per participant;
- success at exactly 90,000 ms, timeout at 90,001 ms, no failed-operation
  reward, terminal reset, and no later timeout after success; and
- selection, success-terminal, and timeout-terminal persistence failures leave
  the prior in-memory phase intact and expose no optimistic accepted result,
  failed objective, completion, reward, or summary.

Focused protocol/compatibility/gateway tests and the fake-client all compile
under warnings-denied Clippy.

## Live route evidence

A source-built gateway ran on isolated listeners `127.0.0.1:18080` and
`127.0.0.1:17000`, with one expected participant. The updated fake client
explicitly opted in, verified both exact options, selected, verified the
same-input replay and different-route conflict, traversed the authoritative
path, completed combat, and checked terminal rewards for each route.

The Breach run produced:

```json
{"session_id":"session-1788608357166269406","route_id":"breach","seed":7336626431646452232,"event_id":"overcharged_armor","outcome":"succeeded","elapsed_ms":2535,"transitions":5,"fragments":2,"experience":100}
```

The Stabilize run produced:

```json
{"session_id":"session-1788608369025799324","route_id":"stabilize","seed":1794573358912907558,"event_id":"residual_feedback","outcome":"succeeded","elapsed_ms":2410,"transitions":7,"fragments":1,"experience":150}
```

For each session, the fake-client terminal summary, independently reconstructed
CLI timeline, `route_operations` row, replay-event rows, and GET-only Inspector
summary agreed. Each had exactly one `route_selected`, one
`route_operation_succeeded`, one generic activity completion, one loot grant,
and one progression grant. Inspector reported `completed=true`, one
participant, the exact route/event/outcome/elapsed value, the expected five or
seven transitions, and one rewarded participant. The isolated gateway was
then stopped; neither isolated listener remained open.

## Complete validation and protected boundaries

The uninterrupted canonical command was:

```bash
CARGO_HOME="$PWD/.tooling/cargo" \
RUSTUP_HOME="$PWD/.tooling/rustup" \
PATH="$PWD/.tooling/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" \
GODOT_BIN="$PWD/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64" \
make check
```

It passed version consistency at `0.2.0`, formatting, workspace/all-target/
all-feature Clippy with warnings denied, **162 tests**, complete workspace
build, TypeScript check, production Inspector build, secret audit,
multiplayer fake-client smoke, four Godot flows, frozen-V1 compatibility, and
independent V1 reconstruction.

Frozen client hashes remain exactly:

- `archive/clients/v1/src/main.rs`:
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`;
- `archive/clients/v1/Cargo.toml`:
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.

Comparison with the M26 closure checkpoint found no content difference under
`VERSION`, `archive/clients/v1`, or `client/game`; Block 6 made no Godot
presentation change. All nine `.gd.uid` files are tracked pre-existing
identities. `git diff --check` passed.

After the live proof, the mutable local engineering database contained 165
route-operation rows, 81 selection replay rows, 49 success terminal rows, and
16 failure terminal rows. It contained zero installed M27 failure-injection
triggers and zero M27 failure-injection functions. These are local fixture
counts, not product or player metrics.

The long-running Docker services remained healthy and loopback-only on
`127.0.0.1:5432`, `:4173`, `:7000`, and `:8080`. No protocol-version bump,
tag, commit, push, release, public listener, or public artifact was created.

## Gate decision

Gate 6 is green. The exact accepted opt-in wire and lifecycle contract is
implemented without expanding the route/event taxonomy or changing the
ordinary-V2/frozen-V1 baseline. Block 7 may now review and implement honest
Godot route presentation. Automated and creator-operated evidence may prove
semantic correctness and reviewability; it must not be described as external
preference, comprehension, or enjoyment evidence.
