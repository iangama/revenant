# Relay Hub final-video presentation brief

Date: 2026-09-07  
Status: implemented and audited  
Scope: owner-authorized final presentation pass over the existing authoritative
Relay Hub activity.

## Sequencing and truth boundary

The owner explicitly paused the read-only M30 Gate 1 audit for the last work of
the day and requested gameplay/world improvements for one final video. This is
a bounded presentation-only interlude, not a green M30 decision, an M31 content
gate, or a roadmap reorder. After delivery, the continuation point returns to
M30 Gate 1.

The pass may improve only client-authored rendering and capture pacing around
facts that the current server already confirms. It does not add an activity,
map, enemy archetype, AI decision, damage value, item, reward rule, inventory
mutation, protocol variant, replay event, database schema, version, or public
service.

## Accepted presentation increment

1. Make the existing approximately 24x24 room read as a short arc through
   Arrival Deck, Breach Corridor, and Relay Core without calling those subspaces
   separate maps.
2. Raise practical visibility, preserve the Compatibility renderer, and make
   lighting respond to the accepted arrival, door, core, and completion phases.
3. Enrich the existing Relay Drone and Warden through bounded animation and
   silhouette/color hierarchy only; their authoritative families and behavior
   remain unchanged.
4. Keep the Warden visible after the open-door transition with a closer camera
   that does not change aiming authority. Recording-only camera following and
   HUD reduction remain behind `REVENANT_SHOWCASE_MODE=1`.
5. Present exactly one named fragment trajectory only after accepted
   `LootGranted(relay_core_fragment)`. The client cannot grant, select, predict,
   or persist the item.
6. Record the normal source-built gateway flow at 1280x720 and 30 FPS, review
   representative frames, verify the media and authoritative summary, and place
   the accepted movie in the owner's Downloads directory.

## Resource and accessibility limits

- At most 170 representative meshes, 32 material identities, five lights, zero
  shadow lights, zero particles, and 14 audio nodes.
- Relay Drone remains at most 12 meshes and three material identities; Warden
  remains at most 18 meshes and four material identities.
- The room remains at most 103 meshes, six material identities, and six
  explicitly named enrichment groups.
- The reward remains one active effect with three transient meshes and a
  non-authoritative world label; it lasts at least two seconds in the
  presentation path.
- Sector banners retain text/non-color identity, mouse pass-through, and Reduced
  Flash handling. Camera transitions remain presentation-only.

Implementation and evidence are recorded in
`docs/art/m28/relay-hub-final-video-audit.md`.
