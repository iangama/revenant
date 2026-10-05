# Relay Hub gameplay-depth presentation brief

Date: 2026-09-07  
Status: implemented and audited  
Scope: richer presentation of existing authoritative enemies, the existing
Relay Core threshold, and the existing `LootGranted` reward only.

Implementation and evidence are recorded in
`docs/art/m28/relay-hub-gameplay-depth-audit.md`.

## Owner request and sequencing decision

The requested enemy variants, new areas, and world pickups belong to the
bounded M31 content tranche. M28 Gate 7 is still active, and the recorded
critical path requires green M28, M29, and M30 gates before M31. This pass must
therefore improve the playable slice now without silently creating unreviewed
server content, compatibility drift, or client-authored rewards.

## Exact increment

1. **Enemy readability:** enrich the existing Relay Drone and Warden
   silhouettes inside their established per-family mesh/material budgets.
   Health, behavior, spawn order, damage, timing, and identity remain unchanged.
2. **Sector transition:** present `ARRIVAL DECK` after the accepted Relay Hub
   join and `RELAY CORE` only after the authoritative open `DoorState`. A
   bounded camera transition may focus the existing core encounter. These are
   named subspaces of one room, not new worlds or activities.
3. **Confirmed collection:** animate one Relay Core Fragment from the last
   confirmed defeated enemy to the Operator only after `LootGranted` is
   accepted. The effect never creates, predicts, selects, or changes inventory.

## Budgets and stop conditions

- Relay Drone: at most 12 meshes and three material identities.
- Warden: at most 18 meshes and four material identities.
- Representative validation peak: at most 150 meshes, 32 materials, five
  lights, zero shadow lights, zero particles, and 14 audio nodes.
- Confirmed pickup: at most three transient meshes, one active pickup, reused
  environment materials, no new audio family, and no permanent node.
- Sector banner: one bounded 560×96 overlay, text plus non-color sector code,
  compatible with Reduced Flash and no gameplay-input capture.
- Default manual speed remains unchanged; showcase pacing remains recording
  only.

The pass stops before adding an archetype to the wire catalog, a second
activity/world, a pickup intent, proximity-owned loot, random drops, inventory
mutation, collision, AI behavior, protocol variant, persistence/replay event,
version change, or M31 completion claim.
