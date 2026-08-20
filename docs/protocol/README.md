# Protocol V1 and V2

M1 introduces a TCP protocol on port 7000. Every message is a four-byte unsigned big-endian payload length followed by one MessagePack map. Frames larger than 64 KiB are rejected before allocation.

The `type` field identifies the canonical message. Protocol parsing remains in `revenant-protocol`; the gateway receives typed messages.

## ClientHello

Sent immediately after connecting:

```text
type: "ClientHello"
protocol_version: unsigned integer
client_name: string
client_build: string
```

## ServerHello

Returned once for every valid `ClientHello`:

```text
type: "ServerHello"
protocol_version: unsigned integer
server_name: string
accepted: boolean
message: string
```

The current client uses Protocol V2. The frozen Revenant Client 0.1.0 uses Protocol V1. The gateway negotiates both versions and echoes the selected wire version in `ServerHello`; any other version is rejected before authentication.

M14 intentionally keeps V1 and V2 message layouts identical. The version change establishes an explicit compatibility lifecycle: V1 is frozen and mapped by `revenant-compatibility`, while V2 remains the current contract. Domain systems receive canonical inputs and do not branch on protocol versions.

M15 independently reconstructs only the V1 connection flow consumed by the frozen client. The reconstruction harness owns separate wire types and does not import this current protocol crate or the M14 adapter. This duplicates a small evidence-backed contract intentionally: successful interoperation while `revenant-gateway` is stopped is the experiment's proof.

## M2 connection flow

After an accepted handshake, messages must follow this order:

```text
ClientHello -> ServerHello
AuthRequest -> AuthResponse
CharacterListRequest -> CharacterListResponse
WorldJoinRequest -> WorldJoinResponse
```

An unexpected message closes the connection. M2 local authentication accepts a `username` containing 1-32 ASCII letters, digits, underscores, dots, or hyphens. It intentionally has no password or external identity provider.

`AuthResponse` contains `authenticated`, `account_id`, and a human-readable `message`. `CharacterListResponse.characters` contains `character_id`, `display_name`, `class_name`, and `level` for each character.

`WorldJoinRequest` selects one returned `character_id`. The server verifies ownership, creates the player actor, and returns `world_id`, `player_actor_id`, and `spawn_position` in `WorldJoinResponse`.

M4 adds `ActorSpawn`, `ActorUpdate`, and `ActorDestroy`. Actor messages carry server-owned IDs; clients never allocate authoritative actor IDs. The initial world snapshot contains the player and one `relay-drone` enemy.

M5 adds `AttackIntent(target_actor_id)` and `DamageApplied`. Clients never submit damage or resulting HP. `DamageApplied` reports server-calculated source, target, damage, remaining health, and death state.

M7 adds `ActivityStart` and `ObjectiveUpdate`. Objective updates expose the generic ID, type, state, progress, and target; trigger evaluation remains entirely server-side.

M8 adds `MoveIntent`, `DoorState`, and `ActivityComplete`. `MoveIntent` is validated as an intention to reach the relay door; door state, boss spawn, objective completion, and activity completion remain server decisions.

M12 does not add wire message types or change Protocol V1 framing. After the configured player count joins, each client receives `ActivityStart`, the current objective, an `ActorSpawn` for every participating player, and the shared enemy. Authoritative `ActorUpdate`, `DamageApplied`, `ActorDestroy`, objective, door, boss, and completion messages are broadcast to every participant. Intents from either player are validated against the same server-owned actor and activity state.

## Compatibility matrix

| Client | Wire protocol | Runtime support |
| --- | --- | --- |
| Frozen Client 0.1.0 | V1 | `FrozenV1` adapter or isolated reconstructed V1 harness |
| Current bot and Godot client | V2 | `CurrentV2` adapter |
