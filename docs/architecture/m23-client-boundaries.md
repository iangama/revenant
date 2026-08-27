# M23 client boundary audit

Baseline: integrated M22 commit `faeca38`, 2026-08-27.

## Current responsibility map

| Area | Current ownership in `main.gd` | Coupling | Proposed destination |
| --- | --- | --- | --- |
| Composition and lifecycle | Scene/controller instantiation, entry point, shutdown | All client layers | Remains in coordinator |
| Wire codec | Supported MessagePack encode/decode and failure flag | Pure bytes only | `protocol/messagepack_codec.gd` |
| Transport and framing | TCP peer, receive buffer, frame length, polling, deadlines | Codec and async scene frames | `protocol/framed_transport.gd` |
| Handshake/session sequence | Protocol hello through completion and retry/failure | Transport, projections, UI state | `session/session_controller.gd` |
| Authoritative projection | Actors, objectives, inventory, progression, equipment | Domain messages, scene instances, HUD/audio | Focused projection controllers |
| Local controls | Keyboard, pointer, UI intents, aim checks, cooldown acknowledgements | Scene actors, transport, HUD/audio | `input/player_input_controller.gd` |
| Presentation composition | Environment, camera, HUD, Entry, Settings | Native nodes and presentation scenes | Composition helpers, still wired by root |
| Validation | Deterministic drivers, all M17–M22 assertions, captures, measurement | Every client layer | Domain harnesses under `validation/` |

## Dependency direction

The intended direction is:

```text
main coordinator
  -> session controller -> framed transport -> MessagePack codec
  -> authoritative projections -> presentation scenes
  -> input controller -> session intents
  -> validation harnesses -> public controller/presentation state
```

Protocol and transport code must not access HUD nodes, audio players, settings, actors, or onboarding. Presentation code must not encode messages or poll sockets. Validation may observe public state and drive explicit test seams, but shipping controllers must not depend on validation code.

## Extraction order rationale

1. The codec is pure and already covered by end-to-end byte traffic plus a signed-fixint assertion.
2. Framing is next because it can consume the codec while returning decoded dictionaries without knowing session semantics.
3. Session orchestration can then depend on a stable transport boundary rather than owning socket details.
4. Projections and inputs can be separated after message delivery and intent submission have explicit interfaces.
5. Validation moves last so it tests the extracted production boundaries instead of freezing the old internal layout.

## Invariants to preserve

- Maximum accepted frame payload remains 64 KiB.
- The existing asymmetric subset remains explicit: outbound movement supports negative fixints, while inbound decoding does not add negative-fixint handling in this behavior-preserving block.
- Socket deadlines and failure-to-empty-result behavior remain unchanged until a separately reviewed typed-result migration.
- The client never manufactures successful handshake, activity, combat, objective, reward, inventory, progression, or equipment state.
- Local input stays an intent; only existing restrained presentation acknowledgements may precede confirmation.
- M22 settings and audio teardown remain local and bounded.
- Frozen V1 hashes remain `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72` and `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.

## Block 2 result

The codec extraction creates the first enforced dependency seam. It reduces `main.gd` by 98 net lines after the new boundary assertion while adding no node, socket, timer, signal, singleton, or runtime allocation beyond one `RefCounted` codec instance owned for the scene lifetime.

## Block 3 result

The framed transport creates the second dependency seam and is the sole owner of the TCP peer and receive buffer. `main.gd` retains connection-state presentation, handshake ordering, and one-line wrappers for send, awaited receive, and nonblocking receive; it no longer knows how frames are assembled, buffered, decoded, or read from the socket.

The node boundary preserves the existing asynchronous model rather than introducing signals or a background loop. This keeps deadlines tied to the same monotonic clock and scene frames while preventing transport code from reaching into session or presentation state.
