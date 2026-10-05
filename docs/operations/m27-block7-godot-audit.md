# M27 Block 7 — honest Godot route-presentation audit

Date: 2026-09-05  
Decision: **Gate 7 green with bounded Block 8 residuals**  
Scope: the accepted contract in
`docs/operations/m27-block7-godot-contract.md`.

## Accepted implementation

Godot now exposes one neutral `ROUTES [R]` entry and a bounded 1280×720
`RouteConsole`. Opening it explicitly requests server route state. Candidate
routes do not appear before an accepted `RouteState`, and the client never
selects a default, resolves an event, computes an effect or reward, starts a
deadline, or infers a terminal outcome.

The authoritative projection owns distinct empty, pending, rejected,
accepted, accepted-replay, disconnected, succeeded, and failed-timeout
states. It validates schema and collection bounds, exact route vocabulary,
leader/capability/participant relationships, option and selection agreement,
fixed duration, ordered terminal transitions, and success/failure reward
shapes before replacing visible truth. A malformed message becomes an
explicit invalid-server rejection rather than a partial optimistic update.

The surface gives Breach and Stabilize equal visual weight and copies exact
server objective chains, budgets, reward plans, possible events, accepted
seed/event/effect, and terminal evidence. Choice is enabled only for the
server-declared leader when every admitted participant is capable and the
phase is open. Pending intent sends only operation ID and route ID. The active
Stabilize objective uses the server-authored `[3, 0, 3]` guidance; the door
path remains `[6, 0, 0]`.

Keyboard focus covers Refresh, both route buttons, and Close. `R` toggles the
surface, Escape closes and restores focus, and gameplay input is suppressed
while it is open. Reduced Flash preserves all textual truth. The block adds
no audio, particle, animation, resolver, catalog entry, preference language,
or dynamic audiovisual pool.

## Boundary support and semantic proof

The already-bounded Godot MessagePack implementation now encodes/decodes nil,
array16, and map16. This is required by optional terminal fields and the
reviewed route summary with more than 15 keys; it changes no Rust wire type or
protocol generation. The boundary harness proves these additional markers.

The isolated Godot 4.7.1 semantic run passed every retained M17–M26 marker and
the new `M27 honest route console validated` marker. Its route fixtures prove:

- no option before authoritative state and no mutation while pending;
- exactly two equally weighted server options and no banned preference text;
- leader enablement, non-leader readability, focus traversal, Escape/focus
  restoration, input suppression, Reduced Flash, and 1280×720 containment;
- exact accepted seed/event/effect/path/reward copying, accepted replay, and
  conflict/rejection behavior; and
- distinct success, timeout, disconnect, invalid-server, and reconnect-reset
  states.

## Real-wire Godot evidence

Fresh isolated current-source gateways and Godot imports exercised both exact
routes through the normal presentation/session loop:

- Breach: session `session-1788609519514376601`, event `arc_surge`, terminal
  elapsed 2,171 ms, two fragments and 100 XP;
- Stabilize: session `session-1788609536186148569`, event
  `shielded_channel`, terminal elapsed 2,878 ms, one fragment and 150 XP.

Each flow explicitly opened route state, selected through the Godot surface,
waited for the accepted server result, traversed the authoritative objective
path, defeated the Warden, and reconciled the generic completion/reward with
the route terminal summary.

The reproducible final capture run is session
`session-1788610041436284561`. Godot reported Stabilize,
`shielded_channel`, 3,309 ms, one fragment, 150 XP, and seven transitions.
The independently reconstructed CLI timeline contains one `route_selected`,
one `route_operation_succeeded`, one activity completion, one loot grant, and
one progression grant with the same operation, seed, event, path, encounter,
reward, and grant. The persisted `route_operations` row agrees. A current
source-built GET-only Inspector independently reported `completed=true`, one
participant, Stabilize, `shielded_channel`, `succeeded`, 3,309 ms, seven
transitions, and one rewarded participant. The isolated listeners were then
stopped.

## Creator-reviewable captures

| Capture | Server-owned state | Size | SHA-256 |
| --- | --- | ---: | --- |
| `01-route-choice.png` | Choice-open options, leader and complete capability | 100,709 bytes | `0faa9c75f8e6fe3989040443e570095e3025518c681677caf4793aff3132116b` |
| `02-route-accepted.png` | Accepted Stabilize seed, event, effect, path and reward | 118,324 bytes | `18fe9cd0a5af9ec5767364d4c3880f5bce3b82ec8219907fd02d515467d8f98a` |
| `03-route-summary.png` | Explicit succeeded phase, elapsed/budget, transitions and grant | 104,106 bytes | `317b30afd716fcd8905173ee1224fae63efaad7f508cc72182fdeb19b080531e` |

All are 1280×720 RGBA PNGs and pass
`docs/art/m27/captures/SHA256SUMS`. Visual inspection found the title,
authority label, equal cards, controls, accepted evidence, and terminal
summary legible without clipping or overlap. These captures prove
renderability and reviewed semantics only, not preference, outside-player
comprehension, enjoyment, or accessibility across other people or hardware.

## Complete quality and protected boundaries

The final uninterrupted `make check` passed version consistency at `0.2.0`,
formatting, workspace/all-target/all-feature Clippy with warnings denied,
**162 tests**, complete workspace build, TypeScript check, production
Inspector build, secret audit, multiplayer/current-client smoke, all Godot
flows and semantic markers, frozen-V1 compatibility, and independent V1
reconstruction. The ordinary unopted Godot and current-V2 flows remain on the
unchanged baseline and receive no unsolicited route message.

Frozen client hashes remain exactly:

- `archive/clients/v1/src/main.rs`:
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`;
- `archive/clients/v1/Cargo.toml`:
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.

`git diff --check` passed. Direct editor validation generated disposable
Godot identities, which were removed; no untracked `.gd.uid` remains. The
capture script uses an isolated copied project and is executable. The mutable
local engineering database contains 169 route rows, 85 selection events, 53
success terminals, 16 timeout terminals, and zero installed M27
failure-injection triggers or functions. These are fixture counts, not player
or product metrics.

The permanent Compose PostgreSQL, Inspector, and gateway services remain
healthy and bound only to `127.0.0.1:5432`, `:4173`, `:7000`, and `:8080`.
The long-lived gateway image predates M27 and correctly was not used to
interpret new route evidence; accepted Inspector evidence came from the
current isolated binary. No protocol/schema/replay/catalog/Lua change, tag,
commit, push, release, public listener, or public artifact was created in
Block 7.

## Rejected attempts and residuals

The first post-edit `make check` reached smoke while a manual one-participant
gateway still owned the smoke ports. The two-client smoke connected to that
wrong process and rejected its unexpected admission sequence. The process was
stopped, both ports were verified free, and the complete canonical gate then
passed. This was an isolated listener collision, not accepted product
evidence.

One graphical attempt used an identity longer than the reviewed authentication
bound. Authentication rejected it before join or route state, as required;
the shorter fresh run replaced it. Rendering used Mesa llvmpipe and dummy
audio because the WSL shell has no playback device. Hardware presentation,
both participant counts/weapons/build families, repeated fresh/reused runs,
wire failure cases, compatibility repetition, and resource envelopes remain
the explicit Block 8 matrix.

## Gate decision

Gate 7 is green. Godot exposes an explicit opt-in route surface that copies
validated server truth, withholds optimistic choice and terminal claims,
remains keyboard-operable and bounded, preserves existing semantic and
compatibility flows, and has accepted semantic, live-wire, replay,
persistence, Inspector, visual, smoke, and quality evidence. Block 8 may now
freeze and execute the bounded runtime/repetition/resource matrix; it may not
expand the accepted M27 state space or manufacture outside-player claims.
