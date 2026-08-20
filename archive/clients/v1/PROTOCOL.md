# Frozen Protocol V1 contract

Transport is TCP. Every frame is a four-byte unsigned big-endian payload length followed by a named MessagePack map, with a maximum payload of 65,536 bytes.

The frozen connection sequence is:

```text
ClientHello(protocol_version=1, client_name, client_build)
ServerHello(protocol_version=1, accepted, server_name, message)
AuthRequest(username)
AuthResponse(authenticated, account_id, message)
CharacterListRequest
CharacterListResponse(characters)
WorldJoinRequest(character_id)
WorldJoinResponse(accepted, world_id, player_actor_id, spawn_position, message)
```

The source snapshot contains the exact serialized field names and types. This evidence was produced entirely from Revenant's own V1 client and server implementation.
