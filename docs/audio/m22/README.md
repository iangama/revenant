# M22 experience and audio direction

Status: Block 1 direction and baseline prepared for review; no runtime audio asset is included.

## Client experience map

The current M21 client constructs the playable room and immediately starts the TCP handshake. It authenticates the fixed `revenant-godot` username, selects the first returned character, joins `relay-hub`, receives authoritative snapshots, waits for `ActivityStart`, and enters the manual activity loop. Any handshake or activity failure collapses into one terminal HUD state that asks the player to close the window.

M22 replaces that implicit progression with the following explicit client-owned presentation states:

| State | Entry evidence | Available action | Exit evidence | Authority boundary |
| --- | --- | --- | --- | --- |
| `Entry` | Client scene is ready | Edit local username, review endpoint, open settings, connect | Player activates Connect | No socket or gameplay state exists |
| `Connecting` | Connect action accepted locally | Cancel or wait | TCP connects or reports failure/timeout | A busy indicator is local; connection success comes from the socket |
| `Negotiating` | TCP connected | Wait | Accepted `ServerHello` or rejection | Protocol selection remains server-confirmed |
| `Authenticating` | Accepted server hello | Wait | Authenticated `AuthResponse` or rejection | Username originates locally; account identity is server-confirmed |
| `Joining` | Authentication accepted | Wait | Character list, accepted world join, and required snapshots | Character and actor identity remain server-owned |
| `Waiting` | World join and snapshots complete | Review controls/settings | `ActivityStart` and active objective | The client never invents activity readiness |
| `Playing` | Activity and actors are confirmed | Move, aim, attack, choose loadout, review guidance/settings | Completion, disconnect, or failure | Intents remain requests; outcomes remain authoritative |
| `Completed` | `ActivityComplete` received | Review result, settings, or return to entry when the session permits | Explicit return/retry action | Reward presentation follows authoritative grants |
| `Failed` | Socket, protocol, authentication, join, or session failure | Read reason, retry, edit entry values, return | Explicit player action | Failure text distinguishes known evidence without claiming recovery |

No M22 screen owns account creation, character creation, activity selection, inventory, progression, equipment, combat, rewards, or persistence.

## Navigation contract

The primary keyboard path is deterministic:

1. Launch focuses the username field on `Entry`.
2. `Tab` traverses username, Connect, Settings, and Quit in visible order; `Shift+Tab` reverses it.
3. `Enter` activates the focused control. Connect is the only route into network negotiation.
4. `Escape` closes Settings to its invoker, returns a non-busy failure view to Entry, or opens the in-session settings overlay.
5. Connecting and server-confirmed transition states disable duplicate Connect actions without trapping keyboard focus.
6. Settings opened during `Waiting`, `Playing`, or `Completed` pause only local presentation where safe; they never pause or suppress the authoritative session.
7. Returning from Settings restores the invoking control's focus.

Pointer and on-screen actions expose the same transitions. Environment-provided host and port remain supported defaults for tests and local deployment. Normal presentation shows the endpoint for diagnosis but does not turn protocol configuration into a gameplay choice.

## Contextual onboarding sequence

Guidance has three local density modes: `Full`, `Compact`, and `Off`. Critical connection, failure, objective, and completion state remains visible in every mode.

| Step | Full guidance trigger | Completion evidence | Compact behavior |
| --- | --- | --- | --- |
| Entry | First launch or explicit Help | Connect action activated | One-line Connect hint |
| Movement | First confirmed activity start | Local movement intent plus authoritative player `ActorUpdate` | Control legend only |
| Aim and attack | Relay drone confirmed active | Local attack intent followed by confirmed combat progress | Aim/attack legend only |
| Loadout | First authoritative equipment snapshot | Accepted `EquipmentChanged`, or dismissal | Weapon-button focus hint |
| Relay door | `ReachArea` becomes active | Authoritative door/player progress | Objective line only |
| Warden | Warden `ActorSpawn` | Confirmed damage or defeat progress | Attack legend only |
| Completion | `ActivityComplete` | Explicit dismissal/return | Completion summary only |

Local input can acknowledge that the player attempted an action, but it cannot complete an instructional step that claims movement, damage, equipment, door, enemy, reward, or activity success without the corresponding authoritative evidence. Help can replay explanations without resetting the activity.

## Sound vocabulary

Revenant audio is sparse, mechanical, and degraded. It should suggest infrastructure that still executes routines after its operators disappeared. It avoids orchestral heroism, horror stingers, militaristic weapon realism, dense cyberpunk music, intelligible voices, and continuous alarm fatigue.

| Family | Character | Semantic use | Trigger boundary | Priority |
| --- | --- | --- | --- | --- |
| Relay ambience | Low electrical drift, distant mechanisms, unstable but restrained loop | Place and system condition | Local scene readiness; no gameplay claim | Low |
| Intact systems | Clean cyan-like pulse, stable relay chirp | Connection-ready and healthy infrastructure | Confirmed connection/system state | Medium |
| Objective systems | Dry amber relay, mechanical latch | Active route, door, objective transition | Authoritative objective/door message | High |
| Corruption | Granular unstable energy, asymmetric modulation | Enemy presence and released corruption | Confirmed spawn/proximity or combat event | Medium/high |
| Operator | Compact servo and protected core response | Confirmed movement, damage, defeat | Authoritative actor/damage state | Medium/high |
| Pulse rifle | Focused electrical discharge with longer body | Distinguish rifle profile | Confirmed player `DamageApplied`; local cooldown UI stays non-impactful | High |
| Arc sidearm | Short hooked snap with compact tail | Distinguish sidearm profile | Confirmed player `DamageApplied`; local cooldown UI stays non-impactful | High |
| Relay drone | Light radial mechanism and brittle corruption | Family identity, proximity, attack, defeat | Confirmed actor/proximity/damage/destroy evidence | Medium/high |
| Warden | Heavy containment resonance and restrained plates | Boss identity, threat, attack, defeat | Confirmed actor/damage/destroy evidence | High |
| Damage | Brief red-like transient with protected headroom | Confirmed player or enemy damage | `DamageApplied` only | Critical |
| Completion | Stable cyan resolution without fanfare | Confirmed activity completion | `ActivityComplete` only | Critical |
| Interface | Short dry ticks with distinct accept/back/error contours | Focused UI acknowledgement | Local UI action; never hit/reward/completion | Low/medium |

Color names describe shared semantics, not a requirement to translate pitch directly from color. No cue may be the sole carrier of state.

## Bus and voice direction

The proposed bus hierarchy is `Master` with `Ambience`, `Effects`, and `Interface` children. Player-critical confirmed damage and completion cues use `Effects` but receive priority over expendable combat transients. Muting any child or `Master` must leave the complete experience playable.

The Block 1 concurrency allocation is accepted as a ceiling, not a target:

- 16 audible voices overall;
- 4 ambience/system voices;
- 8 combat voices;
- 4 interface or player-critical voices;
- oldest/lowest-priority expendable cues yield first;
- completion, confirmed player damage, and explicit UI errors are protected from expendable ambience/combat saturation;
- repeated identical cues use minimum retrigger intervals or coalescing;
- all transient ownership is bounded and observable in validation.

## M21 baseline measured for M22

Measured from `main` commit `289489de38506cd21eff430ecc095ed8e369df06` with Godot 4.7.1 GL Compatibility deterministic slice validation on 2026-08-26:

| Metric | Baseline |
| --- | ---: |
| Scene meshes at representative combat peak | 109 |
| Material instances | 25 |
| Lights | 5 non-shadowing |
| Permanent particles | 0 |
| Audio nodes | 0 |
| Shipping audio files under `client/game` | 0 |
| Packaged audio bytes | 0 |
| Estimated decoded audio memory | 0 bytes |
| Simultaneously audible voices | 0 |
| Explicit audio buses | 0; only Godot's implicit `Master` baseline |
| Audio-specific frame-time effect | None present; no audio nodes or assets execute |
| Combat transient mesh ceiling | 24 |

The validation printed every M17-M21 marker and confirmed the 109/25/5/0/0 scene budget. Block 7 must compare its combined presentation against this baseline and record measurements from a graphical runtime; Block 1 does not claim a GPU frame-time number from headless validation.

Frozen V1 evidence remains unchanged:

- `archive/clients/v1/src/main.rs`: `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`
- `archive/clients/v1/Cargo.toml`: `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`

## Provenance requirements

Every shipping source or rendered audio file must have a record in this directory before inclusion. Use one entry per cue family with:

- file names and runtime purpose;
- author and creation date;
- source type: original recording, original synthesis, or compatible third-party source;
- recording device or synthesis tool and relevant parameters;
- all source inputs and their licenses;
- editing, processing, normalization, loop, and export steps;
- editable source/session location when practical;
- exported format, sample rate, channel layout, duration, and SHA-256;
- runtime import settings and bus;
- explicit confirmation that the source does not imitate or derive from proprietary game audio or a known protected work.

Generated audio is not automatically acceptable. Its provider terms, model/tool, prompt or procedure, inputs, output provenance, and intended runtime use must be reviewable. If those facts cannot be recorded, the output remains excluded.

## Block 1 acceptance

Block 1 is accepted when the state map, navigation contract, onboarding sequence, sound vocabulary, voice allocation, provenance requirements, M21 baseline, frozen hashes, scope exclusions, and Block 2 handoff are explicitly reviewed. Approval authorizes only the entry-shell implementation in Block 2.
