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

M14 initially kept V1 and V2 message layouts identical. M18 extends only the V2 server vocabulary with inventory messages. V1 remains frozen and mapped by `revenant-compatibility`; version-aware projection at the gateway prevents V1 clients from receiving unknown inventory frames.

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

M17 does not add wire messages. `MoveIntent.position` may target integer coordinates within the relay-hub bounds `x,z = -12..12` and requires `y = 0`; the server updates and broadcasts the player actor. Reaching `[6,0,0]` during the active door stage triggers the boss encounter. A client still cannot submit HP, damage, objective, door, spawn, or completion state.

M18 adds V2-only `InventorySnapshot` and `LootGranted` server messages. `InventorySnapshot.items` contains stable `item_id` and `quantity` fields and follows an accepted `WorldJoinResponse`. `LootGranted` contains `activity_id`, `item_id`, granted `quantity`, and authoritative `resulting_quantity`; it precedes `ActivityComplete`. Clients cannot submit inventory mutations.

M19 adds V2-only `ProgressionSnapshot` and `ProgressionGranted`. The snapshot follows `InventorySnapshot` and contains authoritative `level`, total `experience`, and `experience_to_next_level`. A completion grant contains the activity, awarded and total experience, previous and resulting levels, and distance to the next level. It follows `LootGranted` and precedes `ActivityComplete`. Clients cannot submit experience or level changes.

M20 adds V2-only `EquipmentSnapshot`, `EquipIntent`, and `EquipmentChanged`. The snapshot follows progression and exposes the selected weapon plus server-owned profiles. `EquipIntent` contains only an owned item identifier. `EquipmentChanged` reports acceptance, message, actor, selected item, and authoritative damage/range/cooldown. Frozen V1 rejects the new client message and receives no equipment server messages.

## Compatibility matrix

| Client | Wire protocol | Runtime support |
| --- | --- | --- |
| Frozen Client 0.1.0 | V1 | `FrozenV1` adapter or isolated reconstructed V1 harness |
| Current bot and Godot client | V2 | `CurrentV2` adapter |

The optional `content_revision` field in `ClientHello` and `ServerHello`
negotiates content without changing wire protocol versions. The gateway echoes
recognized V2 revisions: `m31-v1` enables the extended equipment catalog;
`m33-v1` includes that same catalog and enables solo Meridian exploration;
`m34-v1` retains both and enables the optional solo Glass Lancer encounter;
`m34-v2` adds the optional Steel Bulwark; `m34-v3` adds the Relay Mender pair
and `RepairApplied`; `m34-v4` adds Bastion Link and Crossed Guard. The gateway echoes each recognized
revision, so clients retaining `m34-v1` still access the Lancer alone.
Absent or unknown revisions retain the original catalog and hub bounds.
V1 never enables these capabilities. Meridian uses existing `ObjectiveUpdate`
and `ActorUpdate` messages; the server gates entrance, adjacent walkable steps,
field discoveries, and the standard mission reward boundary. Field discoveries
persist as revisioned replay events, while reconnecting starts fresh run state.

M34 adds optional `ActorUpdate.charge` with a fixed `target`, `winding_up` and
`warning_ms`. It is omitted for ordinary actors, preserving old update bytes.
The server persists a warning before presenting it, waits at least 1.8 seconds
after confirmation, then resolves the locked cardinal line against the current
player position. Recovery lasts at least 1.4 seconds. The client holds the cue
until the next confirmed state; it never resolves damage or expires the warning
locally. `lancer-v1:` field evidence binds actor, timing, position, damage and
health transitions for replay. Withdrawal or defeat grants no extra reward.

The Bulwark uses optional `ActorUpdate.defense` with a cardinal `facing`,
`braced` state and `warning_ms`. The initial west-facing stance lasts at least
1.6 seconds before a two-unit, 90-degree frontal slam; recovery lasts at least
1.8 seconds. Facing changes only on a new brace. A front shot while braced
produces a confirmed zero-damage result and consumes its normal cooldown.
The client presents that as a shield block. Ordinary updates omit both optional
cues, preserving legacy bytes. `bulwark-v1:` evidence persists each accepted
attack; its fatal attack occupies one `EnemyDied` row, binding the health change
and death atomically. Other stance/strike/withdrawal evidence uses FieldActivity.


The optional Mender + Lancer encounter at [2, 6] uses one typed
`support-v1:` EnemySpawned record for both actors. Replay and Inspector count
both actors, retain each confirmed health transition, and validate repair
eligibility, amounts and timing. Every attack persists before confirmation;
fatal hits and their EnemyDied evidence share one row. RepairApplied carries
`source_actor_id`, `target_actor_id`, positive `amount` and `remaining_health`.
It represents a confirmed repair, never damage or an anticipated heal. The
client projects a brief link and lets mouse, a remappable next-target action,
or its visible button choose among living enemies. Both enemies must fall to
clear the optional objective; withdrawal removes both and keeps the core route.
The encounter grants no separate reward. Earlier revisions receive none of the
new support messages and preserve their existing encounter access.


The two elite compositions reuse defense, charge and repair messages under
`m34-v4`: Bastion Link (Bulwark + Mender) at [5, -6] and Crossed Guard
(Bulwark + Lancer) at [5, -10]. One typed `elite-v1:` EnemySpawned creates
both actors. Attacks, repairs, warnings, resolutions and withdrawal persist
before projection; fatal hits and deaths remain atomic. Crossed Guard
alternates damaging warnings with a minimum 600 ms gap, preserving the full
archetype warning/recovery after a slow write. Evidence includes the previous
transition's actual confirmation time so replay reproduces delayed cadence,
repair priority and death cancellation. The optional objective requires both
enemies, grants no separate reward, and leaves standard core completion
available after victory or withdrawal. Older negotiated revisions skip these
entrances and preserve their previous access.


Prism Warden: `m34-v5` recognizes the
optional core choice at [5, 2] while retaining m34-v4 capabilities. It introduces
`PrismState` with actor ID, phase (Lanes/Pulses), mode, an optional tagged pattern
and interval_ms. AcrossX locks z; AcrossZ locks x; Center and Perimeter partition
the authored arena. A client must retain the cue until the next server state;
the interval is presentation information, never permission to resolve damage.
The Godot client requests m34-v5 and projects the boss state without predicting
damage or ending a warning locally.

The boss occupies one typed `prism-v1:` BossSpawned row. Each blocked/accepted
shot, warning, resolution and retreat is persisted before confirmation. The
half-health phase change shares its attack evidence. Death is atomic with the
fatal hit; if standard completion fails afterward, the gateway retains the
pending fatal projection and retries completion without another death event.
Retreat restores the previous activity objectives and permits the normal
Warden. It grants nothing. Replay checks complete/partial phases, confirmation
times, phase protection, damage, defeat and fallback completion after retreat.

M35 acquisition negotiates content capability `m35-v3` while retaining the frozen
`m35-v2` weapon/module catalog. `AcquisitionStateRequest` returns three authored
commissions with server-verified requirements and locked/ready/claimed status.
`AcquisitionClaimIntent` carries only `arc_id`; accepted `AcquisitionClaimed`
returns the durable commission state and current module/fragment snapshot. Each
arc grants two fragments once per character after mission completion. A retry
credits zero and preserves the current balance after spending. Acquisition
claims have their own typed replay event and never increment normal loot grants.
Frozen V1 rejects these intents; earlier V2 content clients receive no acquisition
messages. The gateway only accepts requests after capability negotiation.
