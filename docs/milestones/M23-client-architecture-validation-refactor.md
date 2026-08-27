# M23 — Client Architecture and Validation Refactor

Status: Blocks 1 and 2 implemented and validated locally; later blocks require separate review.

## Objective

Turn the Godot client's monolithic scene coordinator into explicit, testable boundaries without changing the playable slice, server authority, Protocol V2 wire behavior, frozen V1 evidence, presentation, content, published version, or release state.

M23 is a behavior-preserving refactor. It pays down concentration in `client/game/main.gd` through small extractions whose inputs, outputs, ownership, failure behavior, and validation remain reviewable after every block.

## Baseline

At integrated M22 commit `faeca38`, `client/game/main.gd` contains 1,763 lines, 61 functions, 52 state variables, and 12 preloaded collaborators. It owns scene construction, entry/settings/onboarding, TCP transport, framing, MessagePack, handshake sequencing, session projection, manual controls, HUD projection, audio dispatch, deterministic driving, graphical capture, performance measurement, and assertions spanning M17 through M22.

The M22 gate is green: full `make check`, graphical evidence, audio budgets, frozen V1 hashes, clean packaging, and post-merge CI all pass. This is the behavioral baseline for every M23 block.

## Boundary rules

- `main.gd` remains the composition root and high-level scene coordinator during incremental extraction.
- Extract pure or cohesive leaves before stateful orchestration.
- Dependencies point from the coordinator toward protocol, transport, presentation, and validation helpers; helpers do not reach back into `main.gd` by node path or global singleton.
- Server messages remain the only source of authoritative world, combat, inventory, progression, equipment, objective, reward, and completion state.
- No block changes wire bytes, message order, deadlines, error outcomes, controls, HUD semantics, audio triggers, or saved settings.
- No compatibility shim may duplicate the frozen V1 implementation or move domain authority into the client.
- Each extraction must reduce responsibility in `main.gd`, add no unbounded ownership, and pass the relevant Godot flow before the next block begins.

## Block plan

1. **Responsibility and dependency baseline:** record current ownership, coupling, invariant tests, proposed seams, and exit criteria.
2. **MessagePack codec:** extract the pure supported MessagePack subset and its failure state under `client/game/protocol` while preserving exact bytes and decode behavior.
3. **Framing and TCP transport:** isolate bounded frame assembly, receive buffering, deadlines, and socket polling behind explicit results; keep handshake sequencing in the coordinator initially.
4. **Connection/session orchestration:** move the Protocol V2 handshake and session transition sequence into a controller that emits presentation-neutral outcomes.
5. **Authoritative state projection:** separate actor, inventory, progression, equipment, objective, and completion projection from UI construction.
6. **Input and presentation coordination:** isolate local intent collection and HUD projection without allowing local attempts to manufacture confirmed outcomes.
7. **Validation harnesses:** move M17–M23 assertions into domain-focused harnesses with explicit fixtures, preserving canonical marker output and capture behavior.
8. **Integration and architecture evidence:** run the complete gate, document final ownership and dependency direction, compare baseline metrics, and record deferred work.

## Budgets and acceptance

- No block may increase `main.gd` line count above the preceding accepted checkpoint without an explicit rationale.
- The final coordinator should contain composition, lifecycle, and high-level routing rather than codec or fixture implementation; a numeric final line target is intentionally secondary to dependency quality.
- Protocol helpers remain bounded to `MAX_FRAME_SIZE` and the existing supported MessagePack subset.
- All M17–M22 markers, automatic/manual activity flows, persistence/replay, Inspector, V1 compatibility/reconstruction, packaging, and clean extraction remain green.
- `VERSION` remains `0.2.0`; no tag or release is part of M23.

## Block 1 checkpoint

The ownership audit is recorded in `docs/architecture/m23-client-boundaries.md`. The lowest-risk first seam is the pure MessagePack subset: it has no scene-tree, socket, presentation, time, or domain dependency and is called from only send, receive, nonblocking receive, and one deterministic assertion.

Transport is deliberately deferred. Although framing is cohesive, it owns socket polling, deadlines, receive buffering, and asynchronous frame waits; moving it together with the codec would make the first review unnecessarily broad.

## Block 2 checkpoint

`client/game/protocol/messagepack_codec.gd` owns map/array/string/integer encoding, supported value decoding, and codec-local failure state. `main.gd` now depends on one explicit codec instance and no longer implements wire primitives or owns `_encode_failed`.

The extraction preserves the existing error text, supported markers, signed fixint representation, byte order, map/array behavior, and empty-result failure contract. The isolated Godot slice passes every M17–M23 marker with no `ObjectDB` warnings. `main.gd` is reduced from 1,763 to 1,665 lines after adding the codec-specific validation. No server, protocol schema, message ordering, persistence, mechanic, presentation asset, frozen artifact, version, tag, or release changes.
