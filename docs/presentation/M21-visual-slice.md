# M21 visual identity slice — presentation

This package presents the implemented visual slice, not a new public release. `VERSION` remains `0.2.0`; no `v0.3.0` tag or release belongs to M21 review.

## Selected shots

1. [`01-relay-hub-overview.png`](../art/m21/captures/01-relay-hub-overview.png) — complete room, Operator, both enemy families, relay-core route, terminal, lighting, and HUD hierarchy.
2. [`02-enemy-telegraphs.png`](../art/m21/captures/02-enemy-telegraphs.png) — separate target and danger-close indicators, demonstrating honest telegraphs without invented AI state.
3. [`03-combat-feedback.png`](../art/m21/captures/03-combat-feedback.png) — bounded cooldown, confirmed shot trails, impacts, corruption fragments, hit/defeat poses, and peripheral polish.

The images are direct 1280×720 captures from the Godot 4.7.1 GL Compatibility runtime. They contain no external artwork, logo, character, or proprietary third-party asset.

## Short presentation script

Target duration: 35–45 seconds.

> Revenant is an authoritative online game and software-preservation project. M21 gives its existing playable slice a distinct identity: abandoned industrial systems in graphite and blue-petrol, with cyan integrity, amber objectives, and magenta corruption. The Operator, relay drone, Warden, modular relay chamber, combat feedback, and HUD are original runtime components. Visual effects follow confirmed server events; they never manufacture damage or AI state. The representative combat peak stays within 109 meshes, 25 materials, five non-shadowing lights, and no permanent particles. Protocol V1 remains frozen and compatible. This is a reviewed visual slice on version 0.2.0, not a new release.

## Capture reproduction

From the repository root, with a graphical display available:

```bash
GODOT_BIN=/path/to/Godot_v4.7.1-stable_linux.x86_64 \
  bash scripts/capture-m21.sh
```

The command regenerates all three images and `SHA256SUMS`. Reproduction is successful only if Godot prints every M17-M21 validation marker and the script verifies all files and hashes.

## Review evidence

- canonical resolution: 1280×720;
- renderer: Godot GL Compatibility;
- representative peak: 109 meshes, 25 material instances, five non-shadowing lights;
- permanent particles: zero;
- audio nodes: zero, intentionally deferred;
- combat VFX cap: 24 simultaneous transient meshes;
- frozen V1 hashes are checked by the canonical smoke;
- `make check` remains the functional acceptance command;
- release packaging must contain the full `client/game/presentation` tree.

## PR checklist for later authorization

- attach or link the three selected captures;
- summarize Blocks 1–8 and the Dockerfile build-context repair separately;
- include `make check`, package checksum, scene-budget, Docker health, and V1 hash evidence;
- identify generated concept images as documentation-only and runtime assets as native original Godot content;
- state explicitly that server authority, protocols, persistence, and mechanics are unchanged;
- do not claim audio completion or publish `v0.3.0`.
