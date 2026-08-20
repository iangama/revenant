# M1 — Handshake

Status: complete and validated locally.

## Scope

- Length-prefixed MessagePack protocol V1
- `ClientHello` and `ServerHello`
- TCP game listener in the gateway
- Real handshake from `revenant-bot`
- Real handshake from the Godot client
- Protocol unit tests and end-to-end smoke coverage

Identity, authentication, characters, and world state remain outside M1.
