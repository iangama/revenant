# M22 — Player Experience and Audio Slice

Status: Block 1 prepared and validated locally; runtime implementation has not started.

## Objective

Turn the M21 visual slice into a coherent first-run experience. The client must open on an explicit Revenant entry shell, let the player review connection and local presentation settings, explain the controls in context, and add a restrained original audio identity to the existing `relay_awakening` flow.

M22 improves how the existing game is entered, understood, heard, and adjusted. It does not add a gameplay system, activity, map, class, item, progression rule, protocol message, persistence field, version, tag, or release.

## Experience principles

- The player understands where they are, how to connect, and what the game will do before network activity begins.
- Connection, authentication, loading, failure, ready, playing, completion, and retry are explicit client states.
- Guidance appears when relevant and yields once the player demonstrates the action.
- Keyboard, mouse, and on-screen controls remain equivalent acceptance paths.
- Settings are local presentation preferences and never become server state.
- Sound reinforces confirmed state, spatial threat, interaction, and system condition without becoming required for play.
- Every audio cue has a visual or textual counterpart; silence remains a supported configuration.

## Scope

### Entry shell

- A title and identity frame consistent with the M21 palette and shape language.
- Editable local username with the existing safe account policy reflected before connection.
- Host and port presentation, using the existing environment defaults without exposing protocol internals as normal player-facing controls.
- Connect, retry, and return actions with visible focus and disabled/busy states.
- Clear status and failure messages for connection and session availability.

The shell may choose values for the existing connection and authentication flow. It must not bypass server validation, manufacture a successful state, store credentials, or create a new account protocol.

### Local settings and accessibility

- Master, ambience, effects, and interface volume controls with mute support.
- Fullscreen/window choice and a small set of renderer-safe presentation toggles where they can be applied without restarting domain state.
- Guidance density and reduced-flash options.
- Keyboard navigation, visible focus, readable labels, and color-independent state.
- Local persistence in Godot user data only; malformed or missing settings fall back safely.

### Contextual onboarding

- Introduce movement, aim/attack, relay-door traversal, Warden combat, loadout selection, and completion in the existing activity.
- Advance instructional presentation from observed authoritative progress or explicit local input, never predicted gameplay success.
- Allow guidance to be revisited or reduced without changing the activity.
- Preserve the existing deterministic manual-control path and M17-M21 validation markers.

### Audio identity

- A sparse relay-hub ambience bed expressing degraded machinery, unstable energy, and incomplete transmissions without intelligible dialogue.
- Distinct Operator, relay drone, Warden, weapon, impact, objective, door, damage, completion, and interface cue families.
- Spatial world cues for actors and environment; non-spatial cues for interface and critical player feedback.
- Confirmed combat and objective sounds follow the same authoritative messages as their visual counterparts.
- Local input may produce a restrained interface acknowledgement, but never an unconfirmed hit, reward, enemy action, or completion cue.

## Original audio strategy

Shipping audio must be authored specifically for Revenant or imported only after source and compatible license are recorded. Editable source sessions or synthesis recipes should accompany exports when practical. No proprietary game audio, trademarked sound, private library, generated imitation of a known work, or unrecorded third-party sample may enter the repository.

Prefer compact lossless sources for short cues and appropriately compressed streaming assets for ambience. Normalize families consistently, avoid destructive loudness, keep loop boundaries clean, and record authorship, tools, source inputs, processing, export settings, license, and hashes under `docs/audio/m22`.

## Block plan and acceptance

1. **Experience map and audio direction:** approve client states, navigation, onboarding sequence, sound vocabulary, provenance rules, and measured M21 baseline.
2. **Entry shell:** require an explicit connect action and expose username, connection status, retry, keyboard focus, and safe failure handling without protocol changes.
3. **Settings and accessibility:** add local validated settings, volume buses, mute, reduced flash, guidance density, display choice, and full keyboard navigation.
4. **Contextual onboarding:** reorganize existing guidance into dismissible or revisitable steps driven by local input and authoritative activity progress.
5. **Audio foundation and ambience:** establish buses, pooling/lifetime rules, sparse relay-hub ambience, door/system cues, and silent-mode validation.
6. **Character, enemy, and combat audio:** add bounded semantic cues for Operator, both weapons, relay drone, Warden, confirmed damage, defeat, and cooldown/interface feedback.
7. **Integration, mix, and consistency:** tune priority, spatial attenuation, simultaneous voices, accessibility equivalence, transitions, and the combined visual/audio performance budget.
8. **Captures and review package:** provide reproducible entry/onboarding/runtime captures, a short audiovisual presentation, provenance, hashes, test evidence, and measured budgets without creating a release.

Each block remains separately reviewed. Completing one block authorizes neither the next block nor commit, push, PR, merge, tag, release, or version change.

## Initial budgets

These are acceptance ceilings to validate during Block 1 against the existing 1280×720 GL Compatibility slice:

- at most 16 simultaneously audible voices;
- at most 4 concurrent ambience/system voices;
- at most 8 concurrent combat voices;
- at most 4 concurrent interface/player-critical voices;
- no unbounded audio-node creation or playback queue;
- expired transient players return to a bounded pool or are freed promptly;
- no audio cue may extend an authoritative transition or delay input handling;
- retain the M21 representative peak of 109 meshes, 25 materials, five non-shadowing lights, zero permanent particles, and at most 24 transient combat meshes unless a separately reviewed measurement changes it.

The Block 1 baseline must record packaged audio size, decoded memory estimate, peak simultaneous voices, bus layout, representative output level, and any measurable frame-time effect. These ceilings are not loudness targets.

## Compatibility and tests

- Preserve every M0-M21 automated and manual acceptance flow.
- Keep the server authoritative and Protocol V1 frozen.
- Recalculate the frozen V1 hashes during each block review.
- Add deterministic entry-state, settings-fallback, focus-navigation, onboarding, audio-routing, cue-authority, voice-budget, and silent-mode assertions.
- Keep headless CI independent from a physical audio device.
- Run formatter, lint, tests, builds, Inspector validation, Godot slice/manual flow, canonical smoke, packaging, `git diff --check`, and final `git status` before completion.
- Manually review keyboard-only operation, readable focus, reduced flash, guidance modes, mix clarity, spatial legibility, looping, silence, and reconnect/failure behavior.

## Scope exclusions

- No new server endpoint, protocol message, authentication method, or credential storage.
- No new activity, map, enemy, weapon, item, armor, crafting, trading, progression, or combat rule.
- No voice acting, intelligible dialogue, cinematic, trailer, localization system, or complete remapping system.
- No replacement of the authoritative HUD state with local presentation state.
- No modification of frozen V1 artifacts.
- No `v0.3.0`, tag, release, or public launch.

## Risks and decisions

- **Front-end authority leak:** entry and onboarding consume existing outcomes; they do not invent session or activity state.
- **Test fragility:** keep explicit state boundaries and injectable or disabled audio output for headless validation.
- **Audio overload:** use semantic priority, concurrency caps, short cues, and sparse ambience.
- **Accessibility regression:** maintain visual/textual equivalents and verify keyboard-only and silent play.
- **Asset provenance:** reject any source whose authorship, inputs, processing, or license cannot be recorded.
- **Settings corruption:** validate every local value and recover to deterministic defaults.
- **Scope expansion:** defer account services, character selection changes, control rebinding, localization, and new content.

## Definition of done

M22 is done when a first-time player can launch the client, understand and initiate connection, recover from common failures, configure local presentation, learn and complete the unchanged `relay_awakening` activity, and receive a coherent but optional original audio presentation. All M0-M21 flows must still pass, the server and protocols remain unchanged, frozen V1 hashes remain identical, budgets and provenance are recorded, and the result has reproducible review evidence.

## Delivery strategy

Planning uses `docs/m22-player-experience-audio-plan`. Runtime work should move to a dedicated `feat/m22-player-experience-audio-slice` branch only after this plan is approved and integrated. Prefer block-aligned commits that keep entry/settings, onboarding, audio assets, and operational fixes reviewable. `VERSION` remains `0.2.0`, and every commit, push, PR, merge, tag, or release remains separately authorized.

## Block 1 checkpoint

Block 1 is represented by [`docs/audio/m22/README.md`](../audio/m22/README.md). It maps the current implicit connection flow into explicit Entry, Connecting, Negotiating, Authenticating, Joining, Waiting, Playing, Completed, and Failed presentation states; defines keyboard/pointer navigation and authoritative onboarding evidence; establishes the sparse mechanical sound vocabulary, proposed bus hierarchy, priority and voice rules; and records provenance requirements.

The baseline was measured from `main` commit `289489de38506cd21eff430ecc095ed8e369df06` with Godot 4.7.1 deterministic slice validation. It remains 109 meshes, 25 materials, five non-shadowing lights, zero permanent particles, zero audio nodes, zero shipping audio files, zero packaged audio bytes, zero decoded audio memory, and zero audible voices. Every M17-M21 validation marker passed, and both frozen V1 hashes remain unchanged.

Block 1 adds no runtime dependency and changes no client, server, protocol, persistence, mechanic, version, tag, or release. Completion of this checkpoint may authorize only Block 2 after separate review.
