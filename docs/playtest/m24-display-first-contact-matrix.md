# M24 Block 5 — display and first-contact matrix

Status: evidence complete; Gate B approved with bounded residual risk.

## Boundary

This block validates the existing M24 package for a supervised first-contact
desktop pilot. It may correct defects that make the pilot unreadable,
inoperable, misleading, unsafe, or impossible to reconcile. It does not add
content, mechanics, gamepad support, remapping, a new UI system, protocol or
replay events, telemetry, public infrastructure, packaging, recruitment, or a
release.

The authored logical canvas remains 1280×720. Representative higher desktop
resolutions must retain comparable readable size and reachable controls rather
than leaving the interface in one unscaled corner. Non-16:9 displays are not a
new design target in this block; aspect ratio is preserved with bounded bars.

## Bounded pairwise matrix

The cases below cover every required value and important pair without expanding
into a Cartesian product.

| Case | Resolution | Mode | Guidance | Audio | Flash | Local data / lifecycle | Primary question |
| --- | --- | --- | --- | --- | --- | --- | --- |
| B5-01 | 1280×720 | Windowed | Full | Audible | Standard | Clean | Can a new participant enter settings and read the canonical layout? |
| B5-02 | 1366×768 | Windowed, switch to fullscreen and back | Compact | Muted | Standard | Valid saved settings | Are every control, focus return, Escape, mode switch, and saved preference stable? |
| B5-03 | 1920×1080 | Windowed | Full fallback | Audible | Standard fallback | Malformed settings | Does corruption recover to deterministic defaults without blocking entry? |
| B5-04 | 2560×1440 | Windowed | Full | Muted | Reduced | Clean combined path | Are HUD, targets, text, and controls still legible at 2× output? |
| B5-05 | Native host | Fullscreen, switch to windowed and back | Off | Audible | Reduced | Valid saved settings | Does fullscreen apply and restore without hiding authoritative status? |
| B5-06 | 1366×768 | Windowed | Compact | Muted | Standard | Second launch | Do valid settings survive another process start unchanged? |
| B5-07 | 1280×720 | Windowed | Off | Muted | Standard | Immediate refusal and Retry | Is the failure actionable, keyboard reachable, and explicitly from the start? |
| B5-08 | 1920×1080 | Windowed | Compact | Audible | Reduced | Abrupt close, then clean start | Does focus/process loss avoid stuck movement, reward invention, or irrecoverable local state? |

The graphical automation covers B5-01 through B5-06. B5-07 reuses the real
Block 4 refusal/retry evidence and adds the Block 5 keyboard/focus assertions.
B5-08 requires one controlled real process termination and a subsequent fresh
launch; its authoritative and local files are reconciled separately.

## Results

| Case | Result | Evidence |
| --- | --- | --- |
| B5-01 | Pass | Clean 1280×720 entry, settings, onboarding, and runtime; Full guidance and audible/default presentation. |
| B5-02 | Pass | Saved 1366×768 Compact/muted preferences, complete focus traversal, Escape return, and Windowed→Fullscreen→Windowed restoration. |
| B5-03 | Pass | A deliberately malformed `ConfigFile` produced the expected parser diagnostic and deterministic Full/audible/standard defaults without blocking the validation chain. |
| B5-04 | Pass after correction | 2560×1440 combined mute/Reduced Flash remained readable, bounded, and equivalent in scale to the authored canvas. |
| B5-05 | Pass after correction | Native 1920×1080 fullscreen with Guidance Off retained critical status and returned through Windowed to the same mode, window size, logical viewport, and 1920×1080 render target. |
| B5-06 | Pass | The B5-02 XDG data was launched by a second process and retained Windowed, Compact, muted, and standard-flash values unchanged. |
| B5-07 | Pass | A real refusal on unused port 17999 completed in 8 ms as `transport_failure`/`failed`, focused `RETRY CONNECTION`, retained no `session_id`, and created no account or replay event. |
| B5-08 | Pass with expected abrupt-exit limitation | SIGKILL during activity left only a durable join/start/spawn prefix and no reward; a second process using the same data/account started a fresh session and completed exactly once. |

## Assertions

Every automated case verifies:

- `canvas_items` scaling from the 1280×720 authored base with preserved aspect;
- entry and settings panels remain inside the logical canvas;
- username, Connect, Settings, Quit, every settings control, Apply, and Cancel
  are reachable through forward focus navigation;
- Escape closes settings and returns focus to the invoking control;
- focus loss clears latched on-screen movement and focus recovery is visible;
- malformed local settings recover to the allow-listed defaults;
- Full, Compact, and Off guidance retain critical status, objective, controls,
  reward, and completion presentation;
- mute and reduced flash work independently and together;
- Windowed and Fullscreen apply, switch, and restore;
- the opt-in local report follows the actual applied window size and mode;
- entry, settings, onboarding, and runtime captures are non-empty;
- every capture in one case retains the same physical render dimensions;
- inventory, progression, and bounded input history do not overlap or escape
  their logical panels;
- the M17–M24 deterministic validation chain remains green.

## Baseline finding and correction

The initial 2560×1440 graphical capture rendered all two-dimensional UI at
unscaled 1280×720 coordinates in the upper-left half of the window. Text,
buttons, HUD, crosshair, and target feedback were materially smaller than the
accepted 720p presentation and invalidated the 1440p first-contact case.

The project now uses Godot `canvas_items` stretch with `keep` aspect. This
scales the existing 2D presentation while retaining high-resolution 3D
rendering and the authored canvas. A second finding showed that returning from
fullscreen discarded the prior windowed size; the settings store now preserves
and restores that size. Local observation keeps the same allow-list but updates
its existing environment values after an applied display change.

The first keyboard-path review also found that equipment was reachable only
through pointer/on-screen controls. Number keys 1 and 2 now request Rifle and
Sidearm while the HUD is active, and the onboarding copy exposes those keys.
A real graphical driver injects keyboard events through Godot's input path: 2
equips the Sidearm, Space defeats the drone and Warden, and held D reaches the
relay door. PT-B513 completed session `session-1788107050345817032`; replay
contains one equipment change, both defeats, completion, loot, and progression,
while PostgreSQL contains exactly one fragment, 100 XP, one history row, and
one inventory and progression grant.

The keyboard completion capture exposed overlapping fourth inventory and
progression lines and one excess input-history line. The inventory panel and
label bounds were separated, the input history was bounded to five entries,
and deterministic geometry assertions now reject either regression. A final
1920×1080 completion capture shows the fragment and progression separately.

The initial fullscreen retest also exposed a validation race: the WSLg
compositor reported Windowed before its resize settled, so a same-frame
Fullscreen restoration could be ignored. The experience harness now waits for
mode confirmation plus two settled frames and then requires the original mode,
window size, and logical viewport. The display script independently rejects a
case if any of its four capture dimensions differ.

## Real failure, termination, and retry evidence

For B5-07, PT-B507 used Windowed 1280×720, Guidance Off, and mute. The report
recorded `connect_requested=6`, `connect_outcome=8`,
`connection_outcome=transport_failure`, and `terminal_outcome=failed`. The
entry capture is readable, Retry is the focus owner, the button remains enabled,
and PostgreSQL contains neither `local:m24-block5-refusal` nor a replay event.

For B5-08, the first intentionally terminated attempt for
`local:m24-block5-abrupt-2` was session `session-1788106545686377880`. Replay
contains only `player_joined`, `activity_started`, and `enemy_spawned`; fragment,
XP, history, and both grants are zero. Because SIGKILL cannot run a close
callback, PT-B510 remains explicitly unfinished with terminal value `running`.
This is an operator-visible incomplete report, not a manufactured disconnect or
completion.

The next process used the same XDG data and account as PT-B511 but entered fresh
session `session-1788106577199668650`. It completed with one fragment, 100 XP,
one history row, one inventory grant, one progression grant, and the matching
reward replay pair. Its local report recorded completion at 4986 ms and normal
quit at 5172 ms. No resume or reconnection occurred.

## Captures and review

The final matrix directory contained 24 hashed images. The retained review set
and its independent checksum manifest are in `docs/art/m24/captures`. Visual
inspection covered clean entry, saved settings, the combined 2560×1440 runtime,
native fullscreen, actionable refusal, and the real keyboard completion. No
remaining clipping, overlap, viewport escape, missing essential target, hidden
authoritative status, or irrecoverable local state was found.

## Evidence commands

From the repository root:

```bash
GODOT_BIN=.tooling/godot/Godot_v4.7.1-stable_linux.x86_64 \
  bash scripts/validate-m24-display.sh /tmp/revenant-m24-display-review
```

The command uses isolated XDG directories, creates no participant report unless
separately activated, writes per-case logs and captures, requires consistent
dimensions within each case, and verifies one `SHA256SUMS` manifest.

## Residual risks and boundary

- These graphical runs use the Linux development binary under WSLg. The
  unpublished Windows package and a clean-machine Windows dry run belong to
  Block 6 and are not authorized by this gate.
- Non-16:9 layouts, gamepad, remapping, and perceptual audio checks across
  participant hardware remain outside this bounded matrix.
- Deterministic labels and keyboard completion prove reachability and honest
  projection, not first-time human comprehension; that hypothesis remains for
  the supervised pilot.
- SIGKILL cannot finalize a local report. The retained `running` terminal is
  intentionally reconciled as incomplete by the operator rather than rewritten.
- No reconnect/resume was added. Retry always starts a fresh activity.

No Protocol V2, replay vocabulary, database schema, server authority, frozen V1
artifact, version, public exposure, participant, tag, or release changed in
Block 5.

## Gate B decision

Gate B is **approved with bounded residual risk**. The first `make check`
invocation stopped after the version check because this WSL shell had no Rust
toolchain in `PATH`; it executed no formatter or tests and left no process. An
official stable Rust 1.98.0 toolchain with rustfmt and Clippy was provisioned
only under `/tmp`, and the next uninterrupted gate passed version, formatting,
Clippy with warnings denied, all workspace and PostgreSQL tests, build,
Inspector, secret audit, multiplayer, automatic, manual, keyboard, M17–M24,
replay/summary, frozen V1 compatibility, and V1 reconstruction.

Post-gate review reconfirmed `VERSION=0.2.0`, exact V1 and capture hashes,
unchanged Protocol V2/replay/V1 paths, zero temporary database triggers or
injection functions, loopback-only healthy services, no old validator, equal
`main`/`origin/main` HEAD, and the intentional uncommitted tree. The residual
risks listed above are accepted only for the next separately reviewed M24
preparation block. This decision does not authorize Block 6, recruitment,
public exposure, commit, push, merge, tag, version change, or release.
