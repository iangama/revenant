# M26 Block 6 Protocol V2 and gateway contract

Date: 2026-08-31  
Review state: **accepted before protocol source changes**.  
Scope: additive current-V2 messages, authoritative gateway routing, immutable
admission state, and frozen-V1 proof.

## Compatibility decision

The protocol number remains 2. M26 adds opt-in enum variants to current V2; it
does not send a new message during the established join sequence unless the
client explicitly requests module state. A current V2 client that stops after
`EquipmentSnapshot` continues to receive `ActivityStart` next. Existing
equipment profiles become the admitted server-resolved effective values, so an
old V2 client can play correctly without understanding modules.

Frozen V1 receives none of the new variants and rejects every module request at
the compatibility adapter. Its wire source/artifact remains byte-identical. A
V1 participant always admits with base pulse rifle, base weapon arithmetic,
and 100 maximum health even if the character has persisted V2 equipment or
modules; the database is neither rewritten nor consumed.

## Exact client messages

```text
ModuleStateRequest {}

ModulePreviewRequest {
  modules: [String]
}

ModuleCombineIntent {
  operation_id: String,
  module_id: String
}

ModuleLoadoutIntent {
  operation_id: String,
  expected_revision: u64,
  modules: [String]
}
```

`modules` contains zero to three entries and has a maximum encoded request
count of three. `operation_id` is 1-32 ASCII alphanumeric/hyphen. Module IDs
must be one of the four exact catalog identifiers and are at most 32 bytes.
Unknown, duplicate, fourth-entry, malformed, or out-of-frame input is rejected
before domain mutation. The existing 64 KiB frame cap remains the allocation
boundary.

Preview is non-mutating and may include owned or unowned fixed-catalog modules;
it cannot reserve an operation or prime later acceptance. Combine and loadout
mutation are accepted only while the authoritative session is `Complete`.

## Exact server messages

```text
ModuleSnapshot {
  catalog_revision,
  fragments,
  owned_modules,
  loadout_revision,
  equipped_modules,
  maximum_slots,
  catalog: [ModuleDefinition],
  weapons: [ModuleWeaponProfile],
  max_health
}

ModulePreview {
  accepted,
  message,
  requested_modules,
  weapons,
  max_health
}

ModuleCombined {
  accepted,
  replayed,
  message,
  operation_id,
  module_id,
  state: ModuleSnapshot
}

ModuleLoadoutChanged {
  accepted,
  replayed,
  message,
  operation_id,
  state: ModuleSnapshot
}
```

Wire `ModuleDefinition` contains module ID, family, recipe fragment cost, damage
and cooldown basis points, range delta, and maximum-health delta.
`ModuleWeaponProfile` contains item ID plus base and effective damage, range,
and cooldown. `max_health` is character-global. Catalog and weapon arrays have
exact counts four and two; ownership is at most four and loadout at most three.

Every response is server-computed. A rejected result returns the unchanged
authoritative current snapshot, never a client-proposed projection. `replayed`
is true only for a previously accepted same-operation/same-input result. A
preview rejection returns no guessed profiles: `weapons` is empty and
`max_health` is zero.

## Gateway state and ordering

- Before admission, persistence loads the character module state. The gateway
  resolves and stores immutable admitted profiles for both weapons and admitted
  maximum health. The actor is created with that health.
- The shared-session coordinator atomically persists `player_joined` plus the
  V1/V2 module snapshot before inserting the actor or participant in memory.
- Attacks and equipment responses use only the immutable admitted profile map;
  they never query a mutable loadout or accept client arithmetic.
- A post-completion combination/loadout result updates the participant's
  persisted module projection for response purposes, but never its admitted
  actor, health, cooldown deadline, or combat profiles. Effects begin only on a
  future admission.
- Mutation routes use the Gate 5 persistence methods that commit mutation,
  accepted operation, and replay together. Domain rejection is projected to
  the requesting V2 participant without disconnect/reset; persistence/replay
  failure remains fail-closed and aborts the session coordinator state.
- Module state and preview responses are targeted to the requester. Mutation
  results are also targeted; no participant may overwrite another character's
  projection.

## Evidence required for Gate 6

- round trips for all eight new messages plus maximum-shaped frames;
- malformed ID, unknown module, duplicate/fourth entry, stale revision,
  insufficient fragment, already-owned, unchanged, active-state, and conflict
  rejection with unchanged state;
- ordinary old V2 join sequence with no unsolicited module response;
- opt-in state and preview truth for empty and non-empty loadouts;
- next-session activation proving damage/range/cooldown/health while the
  completed actor remains unchanged;
- same-operation retry produces one consume/grant/revision/replay;
- V1 source/artifact hashes, reconstructed V1 handshake/gameplay, base rifle and
  100-health evidence for a module-owning character;
- multiplayer participants resolve distinct immutable builds; and
- replay/Inspector reconstruction agrees with gateway state.

No Godot workshop/presentation change is part of Gate 6. The fake/current
client and deterministic gateway fixtures may exercise the new optional flow;
the player-facing UI remains behind Gate 7.
