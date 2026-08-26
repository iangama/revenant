# M21 visual development provenance

## Relay-hub concept frame

- File: `relay-hub-concept-frame.png`
- Purpose: visual-development reference for composition, palette, lighting, silhouettes, and modular environment language
- Runtime status: documentation only; not loaded or shipped by the Godot client
- Dimensions: 1672×941 RGB PNG
- SHA-256: `293195eb67b4697bd041becef757d89aa02d444b2fdd6f13ce46dfd13aef7b52`
- Created: 2026-08-26
- Method: OpenAI built-in image generation tool, from an original Revenant-specific prompt supplied during M21 Block 1
- Inputs: text prompt only; no third-party reference images, brands, characters, or proprietary assets

The image is a direction reference rather than a source asset. Future production meshes, textures, icons, animations, and effects must be authored or licensed and documented independently.

## Final prompt

```text
Use case: stylized-concept
Asset type: original visual-development concept frame for the Revenant game, M21 relay-hub vertical slice
Primary request: depict one coherent playable sci-fi room that validates the visual identity of abandoned technology, dead networks, and autonomous systems still operating alone
Scene/backdrop: a compact relay-hub chamber with modular graphite floor panels, angular blue-petrol industrial walls, one sealed relay-core door, one damaged terminal, cable trenches, structural damage, and restrained depth beyond the room edges
Subject: one immediately recognizable player operator with a luminous cyan core and distinct compact weapon, facing one relay-drone enemy with its own corrupted organic-machine silhouette; both small enough to read as gameplay pieces
Style/medium: polished original game concept art, stylized 3D environment render, practical indie-game asset language, readable forms suitable for later reconstruction in Godot, not photorealistic
Composition/framing: 16:9 landscape, elevated three-quarter top-down gameplay camera, entire representative room visible, clear navigation path from player through terminal toward sealed door, HUD-safe negative space near screen edges
Lighting/mood: dark solitary mysterious science fiction, low-key industrial lighting, cyan intact systems, amber interaction and objective light around terminal and door, red-magenta corruption and threat on enemy, volumetric atmosphere used sparingly, never generic horror
Color palette: graphite and deep blue-petrol foundation; cyan for healthy systems; amber for interaction; red-magenta for threat and corruption
Materials/textures: worn painted metal, matte graphite composite, scratched floor panels, oxidized edges, glassy energy cores, subtle organic corruption strands
Constraints: exactly one room, one player, one enemy, one sealed door, one terminal; strong silhouettes; gameplay clarity over cinematic clutter; original design only; coherent modular shapes; visible basic combat feedback as a short cyan projectile trail and restrained magenta impact; no external intellectual property
Avoid: no logos, no readable text, no watermark, no brand references, no zombies, no skulls, no gore, no fantasy, no military realism, no neon cyberpunk city, no excessive fog, no lens flare, no crowded props, no extra characters, no UI panels baked into the image
```

## Operator concept sheet

- File: `operator-concept-sheet.png`
- Purpose: Block 2 identity, proportion, core, armor, and weapon-silhouette reference
- Runtime status: documentation only; not loaded or shipped by the Godot client
- Dimensions: 1536×1024 RGB PNG
- SHA-256: `aa7bcfeceebc5e8a28f57c9d5a4a1b95f49456a1b3934eae17d73c3613a94a3c`
- Created: 2026-08-26
- Method: OpenAI built-in image generation tool, from an original Revenant-specific prompt supplied during M21 Block 2
- Inputs: text prompt only; no third-party reference images, brands, characters, or proprietary assets

The runtime Operator is independently reconstructed with native Godot primitives and shared procedural materials. This image is not a texture, mesh source, or shipping dependency.

### Final prompt

```text
Use case: stylized-concept
Asset type: original character design sheet for Revenant M21 Block 2, documentation reference only
Primary request: design the Operator player character from the approved Revenant visual direction as a compact modular industrial survivor maintained by abandoned autonomous systems
Scene/backdrop: clean dark graphite studio sheet with subtle floor shadow, no environment and no decorative UI
Subject: exactly one character design shown in three consistent full-body views—front, side, and back—plus two small separate weapon silhouette studies for a pulse rifle and an arc sidearm; the same identity and proportions in every view
Style/medium: polished stylized 3D game character concept, practical low-to-mid-poly construction suitable for reconstruction with Godot primitive meshes, strong readable silhouette from an elevated gameplay camera, original design
Character design: upright compact armored Operator, small faceless helmet, angular shoulder plates, segmented forearms and boots, narrow protected waist, luminous cyan circular core visible from both front and back through a mechanical spine housing, asymmetrical weapon mount, no exposed skin, no cape
Weapons: pulse rifle is a longer two-handed angular industrial frame with a cyan energy chamber; arc sidearm is a short one-handed hooked profile with a compact cyan cell; silhouettes must remain distinct without relying on color
Color palette: matte graphite and deep blue-petrol armor, cool steel joints, restrained cyan emission only at the core and weapon cells, tiny amber maintenance accents
Materials/textures: worn painted metal, matte composite, scratched edges, dark rubberized joints, glassy energy core
Composition/framing: landscape character turnaround sheet, orthographic-like views at equal scale, generous separation, full bodies and both weapons entirely visible
Lighting/mood: neutral controlled studio lighting that clearly exposes form, with restrained cyan glow and no dramatic darkness
Constraints: exactly one character identity across three views; exactly two weapon studies; practical modular geometry; strong silhouette; original Revenant design; no readable text required
Avoid: no logos, no watermark, no brand references, no external intellectual property, no extra characters, no face, no skin, no skulls, no gore, no fantasy armor, no military camouflage, no bulky space marine proportions, no neon cyberpunk fashion, no photoreal human
```

## Runtime review captures

These files are direct output from the original Godot runtime, not generated concept art and not third-party assets. They are produced by `scripts/capture-m21.sh` at the canonical 1280×720 viewport with the GL Compatibility renderer.

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `captures/01-relay-hub-overview.png` | Room, route, silhouettes, lighting, and complete HUD | `f69591371c36a926c54a787a927964853552707e1ed84a981fb8d2f1ca7b8a7a` |
| `captures/02-enemy-telegraphs.png` | Independent target and danger-close presentation | `f255101ba2ab8cdc3e015528903289e197566abb71090d11d40f746c9653b35c` |
| `captures/03-combat-feedback.png` | Confirmed combat feedback and bounded visual peak | `04b0b470c5a480f79e74282683ec888c8e96bf2cea62ad34d5a8c80e524db978` |

`captures/SHA256SUMS` is generated and checked during capture. Runtime geometry, materials, scripts, lighting, UI, and effects are authored in this repository from native Godot components. The screenshots contain no imported bitmap, third-party mark, external character, or proprietary game asset.
