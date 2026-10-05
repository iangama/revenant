# Relay Hub presentation enrichment brief

Date: 2026-09-07  
Status: implemented and audited under `relay-hub-enrichment-audit.md`  
Scope: the existing `relay-hub` room, its gameplay camera, and an opt-in clean
recording layout. No world, protocol, authority, objective, reward, collision,
or route expansion is included.

## Presentation goal

Make the one playable region easy to read and worth showing without pretending
that it is a larger world. The room must communicate, in one frame:

1. where the Operator arrived;
2. where the two M28 cooperation roles act;
3. where progress continues through the Relay Core; and
4. where the facility has failed or become corrupted.

This is a composition and environmental-storytelling pass, not decorative
clutter. The existing floor, boundary, Relay Terminal, Relay Core door, and
damaged section remain intact and retain their current meaning.

## Research-informed constraints

- Use constant ambient illumination plus authored practical lights for this
  enclosed scene. Godot documents constant ambient color as a performant indoor
  option and recommends scene exposure, rather than a post-tonemap brightness
  lift, when more brightness is needed.
- Remain inside the Compatibility renderer's feature set. Do not add volumetric
  fog, projector textures, SDFGI, or renderer-specific presentation claims.
- Reuse the six existing material resources. Godot's GPU guidance favors reused
  materials and fewer state changes; this room does not need new texture or
  shader families to gain hierarchy.
- Keep all five existing lights shadowless. Shadows add useful depth but carry a
  rendering cost; the current deterministic software-rendered capture path gets
  more value from silhouette, overlap, height, and emissive accents.
- Treat arrangement as narrative evidence: repeated objects should reinforce
  the facility's function, while the damaged objects form a distinct, bounded
  visual story instead of being spread evenly through the room.

Primary references:

- <https://docs.godotengine.org/en/stable/classes/class_environment.html>
- <https://docs.godotengine.org/en/stable/tutorials/3d/environment_and_post_processing.html>
- <https://docs.godotengine.org/en/stable/tutorials/3d/lights_and_shadows.html>
- <https://docs.godotengine.org/en/stable/tutorials/performance/gpu_optimization.html>
- <https://docs.godotengine.org/en/stable/tutorials/rendering/renderers.html>
- <https://media.gdcvault.com/gdc10/slides/Smith_Harvey_WhatHappenedHereWeb_Notes.pdf>

## Bounded composition

The source adds exactly four semantic groups:

| Group | Function | Semantic color |
| --- | --- | --- |
| Arrival platform | grounds the spawn and establishes a foreground origin | cyan / blue-petrol |
| Cooperation lane | connects `[0,0,0]`, anchor `[3,0,3]`, console `[4,0,3]`, and the core | cyan to amber |
| Relay infrastructure | frames the core and repeats functional service cabinets | graphite / blue-petrol / cyan |
| Corruption trace | concentrates damage and threat around the existing broken section | damaged metal / magenta |

The cooperation anchor and console are presentation-only landmarks at the exact
server-owned target coordinates. They create no collision and never decide
arrival, ping, downing, revive, door, encounter, or completion state.

The camera moves modestly closer and targets the central objective lane. It
must still retain the room boundary, damaged region, Relay Core, Operator, and
encounter silhouettes at 1280x720.

For a creator-operated recording only, `REVENANT_SHOWCASE_MODE=1` may hide the
input diagnostic panel. It changes no gameplay, layout, input, server
projection, semantic test fixture, or default playtest presentation.

## Measurable budget and review gate

- environment: at most 88 `MeshInstance3D` nodes;
- whole representative combat scene: at most 150 meshes;
- at most six material identities;
- exactly five lights, zero shadow lights, zero permanent particles;
- all four semantic groups present;
- exact anchor `[3,0,3]` and console `[4,0,3]` landmarks;
- ambient energy at least 0.70, ambient luminance at least 0.55, graphite
  luminance at least 0.15, and minimum practical-light range at least 5.5;
- semantic validation green under the Compatibility renderer;
- one real 1280x720 comparison capture reviewed for readability; and
- one real gameplay recording placed in the owner's Downloads directory.

The pass stops if it would require a new material family, external art asset,
new light, particle system, gameplay collision, authoritative mutation, or
claim that this one room constitutes a broader finished world.
