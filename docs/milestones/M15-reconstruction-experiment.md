# M15 — Reconstruction Experiment

Status: implemented; validation is part of the canonical smoke.

## Objective

Demonstrate that the unchanged, author-controlled Revenant Client 0.1.0 frozen in M14 can return to its available world-entry state when the backend that originally served it is unavailable.

## Evidence boundary

The reconstruction uses only:

- `archive/clients/v1/src/main.rs`, the frozen client implementation;
- `archive/clients/v1/PROTOCOL.md`, the preserved connection transcript and framing contract;
- `archive/clients/v1/MANIFEST.txt`, the provenance and SHA-256 record;
- `archive/clients/v1/VALIDATION.txt`, the M14 observation record.

All evidence belongs to Revenant. No third-party executable, asset, packet capture, private protocol, or proprietary data is used. The frozen source and Cargo manifest remain byte-for-byte unchanged from their recorded M14 hashes.

## Reconstruction

`revenant-reconstructed-v1` is a deliberately isolated, single-session Rust binary. It independently implements the named MessagePack V1 frames observed by the frozen client:

1. validate the frozen protocol, client name, and build;
2. return a V1 handshake;
3. reconstruct deterministic local identity state;
4. expose one reconstructed Operator character;
5. accept that character into `relay-hub` at the preserved origin.

It depends only on `serde` and `rmp-serde`. It does not link to `revenant-gateway`, `revenant-protocol`, `revenant-compatibility`, PostgreSQL, or any gameplay/domain crate. This is an experiment harness, not a new microservice or production backend.

## Canonical proof

The smoke test first completes the current V2 multiplayer, Godot, persistence, replay, Inspector, and M14 adapter flows. It verifies the frozen source hashes, terminates the local `revenant-gateway`, starts the reconstructed backend on the vacated address, and runs the unchanged frozen binary against it. Success requires both the client's world-join result and the reconstruction process's explicit confirmation that no gateway served the session.

## Scope limit

The frozen M14 client ends after world join, so M15 reconstructs exactly that observable contract. It does not infer undocumented gameplay messages, change the frozen artifact, implement multiplayer, introduce compatibility for third-party software, or claim a production recovery system.
