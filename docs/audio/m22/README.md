# M22 experience and audio direction

Status: Blocks 1 through 8 accepted; foundation, combat, integration measurement, runtime provenance, and reproducible evidence are recorded.

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

## Block 5 original synthesis record

All three exports were created for Revenant on 2026-08-26 by deterministic synthesis implemented by OpenAI Codex under Ian Gama's project direction. The editable source is `docs/audio/m22/synthesis/generate_block5.py`, using only Python standard-library oscillators, envelopes, seeded pseudorandom noise, PCM packing, and WAV writing. There are no recordings, samples, models, prompts, third-party source inputs, or external audio libraries. The work neither imitates nor derives from proprietary game audio or any known protected work.

Run `python3 docs/audio/m22/synthesis/generate_block5.py` from the repository root to reproduce the files exactly. Exports are mono, signed 16-bit PCM WAV at 48 kHz with no destructive normalization. The client reads their PCM payload directly into `AudioStreamWAV`, avoiding editor-import cache dependencies in clean headless checkouts. Runtime looping is enabled only for the ambience; short cues are one-shot.

| File | Purpose / bus | Synthesis and processing | Duration | Bytes | SHA-256 |
| --- | --- | --- | ---: | ---: | --- |
| `relay_hub_ambience.wav` | Sparse local machinery bed / Ambience | Phase-continuous 55, 110, and 220 Hz layers with slow integer-cycle modulation; seamless four-second boundary | 4.00 s | 384,044 | `a555aad62ae6fbd1f615ef664b4819848b27cfabd4b93a52701aed6b88eab0f4` |
| `system_ready.wav` | Confirmed healthy relay state / Interface | 660 and 990 Hz relay chirp under a squared-sine envelope | 0.36 s | 34,604 | `fa18b52e9a6f84e2ba0c59986d1e6310e7f1136ce815afc29b491766ada5a615` |
| `relay_door_unlock.wav` | Authoritative spatial door-open cue / Effects | Decaying low mechanism body, seeded dry-noise latch, and restrained 240 Hz relay tail | 0.60 s | 57,644 | `65ee10226e704d8afeb0a9deb902ba0f5010a9d913105af6fd4c5630c0811276` |

The packaged source total is 476,292 bytes and the decoded mono PCM estimate is 476,160 bytes. The fixed runtime allocation is four nodes with a four-voice ceiling: one ambience, one spatial door, and two pooled system players. The Block 5 validation reaches three simultaneous voices because the 500 ms system retrigger interval exceeds the 360 ms chirp. Master mute stops every active player and suppresses rather than queues requests. Block 7 retains responsibility for representative graphical output-level and frame-time measurements.

## Block 6 original synthesis record

The cumulative deterministic recipe also renders the following original character, enemy, combat, and critical/interface families. Authorship, date, tools, inputs, license posture, PCM format, runtime loading, and non-imitation declaration are identical to the Block 5 record. Seeded noise uses separate fixed seeds for Operator, relay drone, and Warden identity layers.

| File | Purpose / bus | Duration | Bytes | SHA-256 |
| --- | --- | ---: | ---: | --- |
| `operator_servo.wav` | Confirmed Operator movement / Effects spatial | 0.16 s | 15,404 | `0a738b578a9269aad32a283361b9dbdf1aebf6a2aaf121f8e17b7f025e180a9b` |
| `pulse_rifle_confirmed.wav` | Confirmed pulse-rifle damage / Effects spatial | 0.28 s | 26,924 | `e54381ac42f14fa630168d5d5c6301f6765ae0c353d28864257c0148e20952c7` |
| `arc_sidearm_confirmed.wav` | Confirmed arc-sidearm damage / Effects spatial | 0.20 s | 19,244 | `5a7aca0367bb341ced104532dd2e13572579ed9a343b1991b76face130e3ca0e` |
| `confirmed_impact.wav` | Confirmed target impact / Effects spatial | 0.14 s | 13,484 | `6c9bdb9c890bb7e96cab862084e3b812b4705605cea1c079c77be6993b574a77` |
| `relay_drone_cue.wav` | Confirmed drone presence/action / Effects spatial | 0.30 s | 28,844 | `d139755112200a17d551b924555e5aa98f98bd8ebdb5209143e0b8d143240cae` |
| `warden_cue.wav` | Confirmed Warden presence/action / Effects spatial | 0.45 s | 43,244 | `311addb79641de966c60a9065f8117db121b83efc3fc0a993b80aa24efe9e6c3` |
| `enemy_defeat.wav` | Confirmed enemy destruction / Effects spatial | 0.35 s | 33,644 | `1e60caee72b422b6d1cf71c01c85d2b136761ec83ea56686fdb5b3abcc07b767` |
| `player_damage.wav` | Confirmed player damage / Effects non-spatial | 0.20 s | 19,244 | `7d46a9f798aad2bd2ea77f1a2d07c9a013c6fdb2d29a6ec36a13fe38a7066311` |
| `cooldown_tick.wav` | Accepted local cooldown acknowledgement / Interface | 0.08 s | 7,724 | `868342b61b3f8864cd4167e3c09d87955b11b55f244b60425b68dcfc3fefedf6` |
| `completion.wav` | Confirmed activity completion / Effects non-spatial | 0.60 s | 57,644 | `7a4d739d2f2928e2e7b48c7ae94db147130f1137325683cc2634f1f25d6d437d` |

Block 6 adds 265,400 packaged bytes and 264,960 decoded PCM bytes. The cumulative M22 audio totals are 741,692 packaged bytes and 741,120 decoded bytes across 13 sources. Runtime capacity is 14 fixed voices: four foundation, eight combat, and two additional critical players; the combined Interface/player-critical allocation remains four.

## Block 7 graphical measurement checkpoint

Run `scripts/measure-m22.sh` from the repository root to exercise the representative combat mix through Godot's Master bus and print the combined level, captured sample count, peak voices, and graphical frame time. The measurement is opt-in and leaves the canonical headless smoke unchanged. It rejects an empty capture, a mixed peak above -3 dBFS, or more than 14 simultaneous voices.

The 2026-08-27 run used Godot 4.7.1 GL Compatibility at 1280x720 on Mesa llvmpipe. Thirty rendered frames averaged 18.403 ms and reached 37.446 ms; this software-renderer number is an environment-specific review measurement, not a shipping hardware target. The representative mix captured 7,680 stereo frames, peaked at -17.44 dBFS, measured -26.93 dBFS RMS, and reached seven simultaneous voices. The result preserves substantial output headroom and stays below the fixed voice ceiling.

PulseAudio and ALSA output were unavailable in the measurement environment, so Godot used its dummy output driver while the Master-bus capture continued to receive the mixed samples. Cue routing, mute behavior, concurrency, spatial configuration, transitions, visual/textual equivalence, and objective mixed level were validated there; subjective listening was completed separately through the Windows host.

The earlier exit warnings were audio-thread teardown artifacts rather than retained scene nodes. Verbose diagnostics identified only stopped `AudioStreamWAV` and `AudioStreamPlaybackWAV` instances. Allowing the audio server 100 ms to drain after stream removal eliminated the warnings from the isolated slice and from both automatic and manual Godot smoke flows.

The physical-output listening gate was accepted on 2026-08-27 after all 13 WAV families were reproduced through the Windows host. No subjective rebalancing request remained, so Block 7 closed without changing the deterministic source exports or their hashes.
