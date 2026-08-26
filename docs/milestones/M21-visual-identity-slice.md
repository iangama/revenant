# M21 — Visual Identity Vertical Slice

Status: complete, integrated into `main`, and validated locally and in CI.

Integration: PR [#10](https://github.com/iangama/revenant/pull/10) merged as `b62b6dd363537d937a9d52d7728513ebb4b0ff18` on 2026-08-26. The pull-request and post-merge CI runs passed, including formatting, lint, tests, builds, Godot smoke coverage, `0.2.0` packaging, clean extraction, and artifact upload. No tag or release was created.

## Objective

Turn one representative `relay-hub` room into the visual reference for Revenant without replacing its M0-M20 mechanics. The slice must contain one player, one relay drone, the relay-core door, one terminal, the current HUD, lighting, and basic combat feedback.

M21 validates identity before asset volume. It does not create a new activity, map, class, progression system, protocol, or release.

## Fiction and tone

Revenant depicts abandoned technology, dead networks, and systems that continue operating without their makers. The tone is dark, solitary, and mysterious science fiction. It must avoid generic horror, gore, fantasy ornament, military realism, and crowded neon cyberpunk.

Industrial construction is angular, modular, and designed for maintenance. Intact energy and corruption contrast it with curved, branching, organic forms.

## Visual language

### Palette

| Role | Color | Use |
| --- | --- | --- |
| Graphite | `#10151D` | Primary floor, structural mass, deep UI panels |
| Blue-petrol | `#12313A` | Secondary surfaces, environmental separation |
| Intact cyan | `#35D0D0` | Player core, healthy systems, confirmed neutral state |
| Objective amber | `#F5A524` | Doors, terminals, interaction, current objective |
| Threat magenta | `#D93678` | Enemy core, corruption, hostile telegraphs |
| Damage red | `#E8505B` | Damage confirmation and critical state only |
| Cool neutral | `#A9B8CC` | Secondary text and inactive technical detail |

Emission must remain local and purposeful. Large surfaces stay dark and matte so cyan, amber, and magenta retain semantic value.

### Shape and material rules

- Architecture uses chamfered rectangles, reinforced seams, service channels, and replaceable panels.
- Corruption uses asymmetry, branching strands, hooked silhouettes, and glassy energy nodes.
- Wear communicates abandonment through scratches, oxidation, missing panels, and repaired seams; it must not become visual noise.
- Floor seams reinforce navigation and combat space instead of forming arbitrary decoration.
- Door and terminal silhouettes must remain recognizable without color.

### Character

The Operator needs an immediate full-body silhouette, a cyan chest or back core visible from the gameplay camera, and weapons distinguishable by profile. Animation priorities are idle, locomotion, attack, hit response, and defeat. Presentation never authors position, health, equipment, damage, or completion.

### Enemy family

The relay drone combines a compact machine chassis with corruption growing through it. Its hooked radial silhouette distinguishes it from the upright Operator. Idle, chase, wind-up, attack, hit, and death must be readable through pose and timing before relying on color or HUD.

### Environment and lighting

The selected room is a compact industrial relay chamber. The terminal and sealed relay-core door establish an amber route through the space. Cyan service lights show infrastructure that still functions. Magenta light belongs to the enemy and local corruption only.

Use one dominant environmental direction, restrained local practical lights, limited transparency, and sparse atmosphere. The elevated three-quarter camera must keep player, enemy, route, door, and terminal readable simultaneously.

### HUD

The HUD remains a projection of authoritative state. It should feel like a surviving operator interface: graphite translucent surfaces, restrained seams, compact technical hierarchy, and semantic color shared with the world.

Required information from M17-M20 remains present: connection, player and enemy health, objective, position and door distance, guidance, input feedback, inventory, progression, weapon, and controls. Color is never the only state indicator. Damage feedback must be brief and must not obscure the target.

## Block plan and acceptance

1. **Direction + guide:** this document and one approved concept frame define palette, shapes, lighting, composition, UI hierarchy, exclusions, provenance, and performance guardrails.
2. **Character + animation:** Operator silhouette and both weapon profiles are distinct; existing input and server-confirmed state remain unchanged.
3. **Modular environment + lighting:** one room is assembled from reusable floor, wall, door, and terminal modules with a readable route.
4. **Enemy + telegraphs:** the relay drone has a unique family language and readable server-aligned combat states.
5. **Combat + VFX:** shot, trail, impact, damage, cooldown, and corruption feedback form one consistent vocabulary.
6. **HUD + menus:** all existing information is reorganized without dropping M17-M20 acceptance state.
7. **Audiovisual polish + consistency:** provisional presentation is removed from the selected room and the scene meets the performance budget.
8. **Captures + public presentation:** reproducible captures and a short presentation script are prepared; no release is created by M21.

Each block requires explicit approval before the next begins.

## Scope exclusions

- No new gameplay system, activity, class, map, weapon, armor, crafting, trading, or level effect.
- No server-authority, persistence, protocol, compatibility, or frozen V1 change.
- No replacement of functional mechanics solely for aesthetics.
- No complete audio pass, trailer, tag, or `v0.3.0` release.
- No proprietary third-party code, character, mark, or asset.

## Expected implementation surfaces

- `client/game/main.gd`, currently responsible for scene construction and HUD presentation;
- `client/game/main.tscn` and `client/game/project.godot` where scene resources or rendering configuration become necessary;
- new original Godot scenes, materials, meshes, textures, icons, shaders, and effects under `client/game`;
- `tests/smoke.sh` and the existing Godot slice/manual validation paths;
- this milestone record and `docs/art/m21` provenance records.

The exact runtime file layout is a Block 2 decision. Block 1 adds no runtime asset dependency.

## Original asset strategy

Runtime assets will be authored for Revenant or imported only after a compatible license and source are recorded. Editable sources should accompany exported assets when practical. Generated concept material is inspiration and composition guidance; it is not automatically a production texture, mesh, UI element, or shipping asset.

The Block 1 concept frame, its generation method, prompt, hash, and usage limits are recorded in `docs/art/m21/README.md`.

## Performance guardrails

Before Block 2, capture a baseline on the current 1280×720 GL Compatibility client. Later blocks must track frame time, draw calls, visible geometry, material count, texture memory, shadow-casting lights, transparent effects, and particle counts.

Prefer shared materials, instanced modules, baked or non-shadowing practical lights where appropriate, bounded particles, and effects that expire promptly. No visual effect may delay, predict as confirmed, or replace an authoritative state transition.

## Compatibility and tests

- Preserve every M0-M20 automated flow and keep the server authoritative.
- Keep Protocol V1 projections unchanged and recalculate the frozen hashes during each block review.
- Retain handshake, inventory, progression, loadout, reusable-session, manual-control, and complete activity validation.
- Add deterministic scene-composition assertions and reproducible captures as visual components land.
- Run formatter, lint, tests, build, Inspector checks, Godot slice/manual flow, canonical smoke, `git diff --check`, and final `git status` before completion.
- Manually inspect 1280×720 presentation, HUD scale, contrast, color-independent state, target visibility, combat feedback, and frame pacing.

## Risks and decisions

- **Style drift:** approve the single room before producing asset families.
- **Readability loss:** reserve emission colors by meaning and test without relying on color alone.
- **Performance regression:** set a measured baseline before runtime visual implementation.
- **Client authority leak:** drive persistent presentation from server messages; local anticipation stays transient.
- **Test fragility:** preserve stable node/state contracts and add visual checks incrementally.
- **Asset provenance:** document authorship or compatible license before inclusion.
- **Scope expansion:** do not combine visual identity with major mechanics.

## Definition of done

M21 is done when the approved room communicates Revenant's identity, every required gameplay state remains legible, performance stays within an agreed measured budget, all M0-M20 acceptance flows pass, frozen V1 hashes remain unchanged, provenance is complete, and the result has reproducible review captures.

## Delivery strategy

Development uses `feat/m21-visual-identity-slice`. Commits should align with the paired blocks and keep operational/build fixes separate from visual work. A pull request must include captures, performance comparison, provenance, test evidence, and the frozen hashes. Commit, push, PR, merge, tag, and release remain separately authorized actions.

## Block 1 checkpoint

Block 1 is represented by this guide and [`relay-hub-concept-frame.png`](../art/m21/relay-hub-concept-frame.png). Approval of this checkpoint authorizes only the next paired block; it does not authorize bulk asset production.

## Block 2 checkpoint

Block 2 replaces the provisional player capsule with an original modular Operator assembled from native Godot primitive meshes. The runtime presentation lives under `client/game/presentation/operator` and keeps the actor root at the server-confirmed position.

The Operator provides an elevated-camera silhouette, cyan front/back core, graphite and blue-petrol armor, an amber maintenance mark, and distinct `pulse_rifle` and `arc_sidearm` scenes. `EquipmentSnapshot` and accepted `EquipmentChanged` messages alone select the persistent weapon presentation.

Presentation states are driven by existing authoritative messages:

- `ActorUpdate` moves the logical root immediately and eases only the internal visual offset;
- `DamageApplied` animates confirmed attacker recoil and target hit response;
- zero confirmed health triggers defeat;
- equipment messages select the visible weapon;
- idle core pulse and local intent feedback do not manufacture gameplay state.

Because the modular mesh hierarchy is created at runtime, the short procedural transitions use bounded Godot tweens rather than fragile authored tracks targeting generated nodes. Every transition returns to an explicit neutral transform, and no tween changes the authoritative actor root.

The deterministic slice validation instantiates the Operator, checks its part composition and weapon exclusivity, exercises movement, attack, hit, and defeat states, and can save a graphical review capture through `REVENANT_CAPTURE_SLICE`. The canonical smoke requires the M21 validation marker. Release packaging includes the new `presentation` tree.

The documentation-only [`operator-concept-sheet.png`](../art/m21/operator-concept-sheet.png) defines the identity reference and is not loaded by the game. Completion of this checkpoint authorized only the separately reviewed Block 3.

## Block 3 checkpoint

Block 3 replaces the provisional plane, single light, and box door with an original modular `relay-hub` room under `client/game/presentation/environment`.

The room uses sixteen reusable floor panels, sixteen boundary wall panels, four structural columns, a composed relay-core door, a passive relay terminal, and one damaged/corrupted section. Visual walls align with the server-owned `[-12, 12]` movement bounds, while the door remains at `x=6.5`. The terminal is deliberately non-interactive and introduces no client intent or gameplay rule.

Six shared semantic materials cover graphite structure, blue-petrol support, intact cyan, objective amber, damaged metal, and threat magenta. One non-shadowing directional light and four non-shadowing practical lights provide readable GL Compatibility illumination without permanent particles, volumetric fog, or per-frame environment logic.

`DoorState` remains the sole source of the relay-core door state. The environment exposes a narrow `set_core_door_open` presentation boundary; opening retracts the two panels and disables the amber sealed-state strips without changing movement, encounter, or objective logic.

Deterministic validation enforces the 12-unit visual bounds, exact door position, passive terminal, damaged landmark, maximum of 60 meshes, maximum of six materials, exactly five lights, zero shadow lights, and reversible door state. The M21 environment marker is required by the canonical smoke, graphical captures use the existing `REVENANT_CAPTURE_SLICE` path, and release packaging already includes the complete presentation tree.

All runtime environment assets are authored as native Godot scenes, primitive meshes, resources, and scripts. No generated or third-party bitmap is loaded by the room. Completion of this checkpoint authorized only the separately reviewed Block 4.

## Block 4 checkpoint

Block 4 replaces the provisional enemy boxes with two original families under `client/game/presentation/enemies`.

The relay drone has a low radial chassis, four angled mechanical arms, and an exposed magenta corrupted core. The Warden uses a taller containment body, four grounded pylons, four amber restraint plates, and a protected magenta core. Their geometry, proportions, and material hierarchy distinguish them without depending on a recolor.

Both scenes inherit a narrow presentation boundary for spawn, confirmed movement, confirmed attack, confirmed hit, target selection, confirmed defeat, and proximity state. The relay drone is capped at 12 meshes and three materials; the Warden is capped at 18 meshes and four materials. Neither owns a light, permanent particle system, network call, gameplay position, health value, cooldown, or AI decision.

The current protocol does not project internal AI states. Honest telegraphing therefore uses only available evidence:

- `ActorSpawn` drives entry presentation;
- `ActorUpdate` drives orientation and chase interpolation;
- confirmed actor positions drive a proximity disk that indicates danger, not a promised attack;
- the existing aiming calculation controls a separate target disk;
- `DamageApplied` drives attack and hit confirmation;
- `ActorDestroy` removes the actor from targeting immediately, then permits a 260 ms visual collapse before freeing the node.

No cast bar, predictive damage, invented wind-up, server delay, AI change, or protocol expansion is introduced. Deterministic validation enforces family identity, relative structural complexity, independent target/danger states, material and mesh budgets, and confirmed retirement. The canonical smoke requires the M21 enemy marker, graphical captures include both families, and release packaging includes the complete presentation tree.

All runtime enemy assets are authored as native Godot primitives and scripts. Block 5 remains separately gated.

## Block 5 checkpoint

Block 5 adds a shared, original combat feedback controller under `client/game/presentation/combat`. It gives confirmed combat a single semantic vocabulary: cyan marks an Operator shot, magenta marks a hostile shot or released corruption, red marks the confirmed damage impact, and amber marks the local weapon cooldown.

`DamageApplied` is the sole trigger for muzzle confirmation, shot trail, impact, corruption fragments, and the existing actor hit response. Consequently, the client never displays a hit, damage, or hostile attack before the server confirms it. A successfully sent local `AttackIntent` may display a short amber cooldown ring, but that cue makes no claim that the attack was accepted or connected.

All effects use native Godot primitive meshes and four shared materials. They expire within 220 ms except for the cooldown cue, whose bounded lifetime follows the existing 260 ms client input throttle. The controller permits at most 24 simultaneous transient meshes and owns no permanent particles, lights, physics, damage, health, target selection, or network behavior.

Deterministic validation exercises friendly and hostile confirmed exchanges, the local cooldown cue, semantic composition, effect bounds, and the absence of permanent particles. The canonical smoke requires the combat VFX marker and release packaging already includes the complete presentation tree. Block 6 remains separately gated.

## Block 6 checkpoint

Block 6 replaces the provisional flat rectangles and default controls with a reusable Operator HUD frame under `client/game/presentation/hud`. Four compact panels organize Operator telemetry, mission guidance, input monitoring, and inventory/progression without obscuring the central combat space.

Every M17-M20 information contract remains present: connection status, player and enemy health, objective, position and door distance, guidance, input feedback, inventory, progression, authoritative weapon profile, movement controls, attack, and loadout selection. Player and enemy health now combine exact text with proportional rails, so neither color nor shape alone carries the state.

Graphite panels and blue-petrol controls keep the world visible. Cyan identifies intact Operator systems and movement, amber identifies objective/loadout interaction, magenta identifies attack controls and hostile state, red remains confirmed damage, and cool neutral remains supporting information. Focus, hover, pressed, labels, and button silhouettes provide redundant interaction feedback.

The HUD component is passive: it owns no socket, intent, health, inventory, progression, equipment, objective, or cooldown rule. Existing main-client handlers continue projecting authoritative values into the same labels and controls. Deterministic validation enforces four panels, four semantic accent roles, pointer passthrough, both health rails, every prior label/control contract, and the canonical HUD marker. Block 7 remains separately gated.

## Block 7 checkpoint

Block 7 completes visual consistency for the selected room. The audit found no remaining provisional player capsule, enemy box, base plane, single-light setup, door box, or flat HUD panel in the runtime scene. Character, enemy, environment, combat, and interface presentation now share the same graphite, blue-petrol, cyan, amber, magenta, red, and neutral semantics.

A small passive polish layer under `client/game/presentation/polish` adds four ten-pixel screen-edge rails. Confirmed player damage briefly pulses them red; authoritative activity completion briefly pulses them cyan. Maximum screen coverage is six percent, the center remains unobscured, input passes through, and no cue predicts health, damage, completion, or reward.

The deterministic whole-scene budget is at most 120 meshes, 32 material instances, five non-shadowing lights, zero shadow lights, zero permanent particle nodes, and the existing maximum of 24 simultaneous combat-effect meshes. The four edge overlays are bounded tweens. These constraints target the canonical 1280×720 GL Compatibility presentation and favor stable frame pacing over expensive transparency, shadows, or post-processing.

Audio remains intentionally absent: the validation requires zero audio nodes, no third-party sound is introduced, and the later audio direction remains degraded machinery, unstable energy, and incomplete transmissions. This is recorded debt rather than silent scope expansion. Deterministic validation and the canonical smoke require the presentation-polish marker. Block 8 remains separately gated.

## Block 8 checkpoint

Block 8 provides three deterministic 1280×720 runtime shots: relay-hub overview, enemy telegraphs, and confirmed combat feedback. `scripts/capture-m21.sh` generates them with Godot 4.7.1, verifies that every expected file is non-empty, writes `SHA256SUMS`, and checks the manifest immediately.

The selected images, reproduction command, 35–45 second narration, performance evidence, disclosure language, and future PR checklist live in `docs/presentation/M21-visual-slice.md`. Captures come directly from the implemented GL Compatibility scene; concept images remain clearly separated documentation references.

Completion of this block closed M21 implementation scope. Final integration retained the test evidence, frozen V1 hashes, asset provenance, and the Dockerfile repair as a separately explained operational change. `VERSION` remains `0.2.0`; no tag or release was created.
