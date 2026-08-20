# M14 — Frozen Client

Status: complete and validated locally.

Revenant Client 0.1.0 is frozen under `archive/clients/v1` with independent MessagePack types, framing, build instructions, protocol snapshot, validation transcript, and SHA-256 metadata. Every artifact is author-controlled and belongs to Revenant; no third-party executable, asset, packet, protocol, or private data is present.

The active protocol is now V2. `revenant-compatibility` negotiates frozen V1 and current V2 connections, then maps versioned client messages into a canonical gateway vocabulary before domain logic. V1 and V2 share their field layout in M14 by design: the milestone proves the version boundary and continued operation of a genuinely independent client without inventing unnecessary gameplay incompatibility.

The canonical smoke completes current V2 multiplayer, Godot, persistence, replay, and Inspector checks, then runs the frozen V1 client against the V2 runtime and verifies handshake, authentication, character listing, and world join through the adapter.

M14 intentionally does not remove or disable a backend, infer unknown protocol behavior, reconstruct a server from evidence, or begin M15.
