# M28 Block 7 — honest Godot cooperation-presentation audit

Date: 2026-09-07  
Decision: **Gate 7 green with bounded Block 8 residuals**  
Scope: the accepted contract in
`docs/operations/m28-block7-godot-contract.md`.

## Accepted implementation

Godot now exposes one neutral `CO-OP [C]` entry and a bounded 1280×720
`CooperationConsole`. Opening it explicitly requests capability state. Before
an accepted response, the client exposes no participant, role, target, timer,
life, contribution, or reward fact. Start, ping, and revive submit only a
bounded generated operation ID and remain neutral until matching server
results arrive.

The authoritative projection validates schema and text/identity bounds,
participant/capability sets, admission order, immutable anchor/runner roles,
fixed targets, exact 60,000/5,000/15,000/2,000 ms timing, squared revive range
four, 50 restored health, one-revive ceiling, ordered contributions, exact
ping/revive/life transitions, and the fixed two-fragment/125-XP-per-participant
reward. A contradictory accepted or rejected message clears partial
cooperation truth and displays `INVALID SERVER DATA`.

The console shows equal role cards, exact responsibilities, health/life,
server-observed timing, live/expired ping, revive channel/result, five ordered
contributions, and typed terminal/no-reward evidence. It does not run a local
countdown or infer phase, completion, healing, or reward. `C`, Escape, and
Tab/Shift+Tab are integrated; gameplay input and the existing module/route
surfaces are mutually blocked while it is open. Every color distinction is
repeated in text, and Reduced Flash removes no information.

The local Operator can display the server-owned active, downed, revived, and
defeated states. Existing Arrival Deck/Relay Core staging, Drone/Warden
presentation, fragment pickup, audio family, renderer, and budgets are reused;
Block 7 adds no new content authority, chat, marker system, particle family,
or claim about human cooperation quality.

## Semantic and live-wire proof

The isolated Godot 4.7.1 semantic run passed every retained M17–M27 marker and
the new `M28 honest cooperation console validated` marker. Its fixtures cover
empty/pending/eligible authority, leader/nonleader controls, accepted/replayed/
rejected start and ping, live/expired ping, downing, every revive status,
revival, encounter, success, all five failure families, disconnect, reset,
malformed timing, oversized rejection text, focus traversal, input blocking,
Reduced Flash, and 1280×720 containment. Pending client intents never mutate
authoritative state.

The accepted real-wire run used a fresh current-source gateway, the real Godot
client as anchor, and the standalone deterministic runner. Session
`session-1788786742450041749` completed the normal sequence:

1. both current-V2 clients separately opted in;
2. the Drone was defeated and the first-admitted participant started the
   operation;
3. the anchor reached `[3,0,3]` and sent the fixed console ping;
4. the runner reached `[4,0,3]`, was downed by relay feedback, and was revived
   through an observed server channel;
5. the anchor traversed the existing door to `[6,0,0]` and the Warden was
   defeated; and
6. both clients received the same `succeeded` summary and their own generic
   two-fragment/125-XP grant.

Godot reported all five contributions, one revive, equal rewards, and terminal
elapsed time 6,901 ms. The runner independently reported role `runner`, observed
ping/down, two grants, and `succeeded`. Both processes exited zero and the
isolated listener was stopped.

## Persistence, replay, and Inspector reconciliation

Direct PostgreSQL reconciliation found exactly one succeeded cooperation
operation, ordered immutable `anchor`/`runner` participants, all five stored
contributions, one revive, active terminal health of 90/50, zero M27 route
operation, two activity-history rows, two item grants of two fragments, and
two progression grants of 125 XP. Each cooperation start/ping/down/revive/
success transition and generic activity completion occurs once; only the
expected per-participant loot and progression events occur twice.

The current-source Inspector reports 19 events, two participants, completed
activity, two enemy defeats, no route, nonlegacy cooperation state `succeeded`,
five contributions, one revive, two rewarded participants, and 6,901 ms. Its
summary endpoint is GET-only: GET returns 200; HEAD, POST, PUT, PATCH, DELETE,
and OPTIONS each return 405.

Review also detected that the long-running Compose gateway predated the M28
replay vocabulary: raw events returned 200 while its summary returned 500 for
an unknown replay kind. Rebuilding exposed a second packaging defect—the
gateway Dockerfile did not copy newly added workspace crates. The Dockerfile
now includes every missing workspace member required for manifest resolution.
Both Gateway and Inspector images build, their services were recreated without
replacing the PostgreSQL volume, and the same preserved session now returns the
correct 200 summary. This is accepted application evidence, not a fabricated
fixture.

## Creator-reviewable captures

| Capture | Server-owned state | Size | SHA-256 |
| --- | --- | ---: | --- |
| `01-eligible-roles.png` | Eligible two-participant capability and role plan | 73,530 bytes | `b3e8143d048867cd746ad64055e514c6ef65b286c5af661da279b1021b2a4542` |
| `02-live-ping.png` | Accepted live ping, source, accepted/expires time | 116,167 bytes | `705f7976ea4aff5a246ecb780fc8c8585aedadeceb8c762d7f2a71db51389f3b` |
| `03-runner-downed.png` | Runner life transition and role-specific state | 120,068 bytes | `9c420df823af325ea221ef17dcfb5301ba94f18f479099140e1600bd18ad6621` |
| `04-revive-channel.png` | Accepted revive channel, duration, range and source | 124,007 bytes | `e1e4caae9b8ac9ab489c8d673ffd0fda0627982a20772072d0a0c094c4ef2d14` |
| `05-revive-complete.png` | Authoritative downed-to-active transition | 120,880 bytes | `2c6145927111bc151a9051ae35ed337c65b69c6f196b3c94483d001ccc45355a` |
| `06-terminal-success.png` | Five completed contributions and equal rewards | 122,399 bytes | `a48bc0d7fb0cbf27309a785a271868d425de11dfe3a8989534cf9545ebced3dc` |

All are 1280×720 PNGs. Visual inspection confirmed legible role cards,
non-color states, exact timing/reward text, disabled duplicate actions, five
terminal contributions, and no clipping or overlap. The complete accepted
evidence directory is
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block7-godot-20260907h`;
its `SHA256SUMS` hash is
`1cbf0d832aafc3bd4b56c9a4520e0ad219abc381b63a8d3a461ea02b9ee4a10a`.

## Complete quality and protected boundaries

The final uninterrupted `make check` passed version consistency at `0.2.0`,
formatting, workspace/all-target/all-feature Clippy with warnings denied,
**217 Rust tests**, complete workspace build, TypeScript check, production
Inspector build, secret audit, multiplayer/current-client smoke, manual and
keyboard Godot flows, all semantic markers, frozen-V1 compatibility, and
independent V1 reconstruction. `git diff --check` passed.

Frozen client hashes remain exactly:

- `archive/clients/v1/src/main.rs`:
  `4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72`;
- `archive/clients/v1/Cargo.toml`:
  `c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322`.

The permanent PostgreSQL, Gateway, and Inspector services are healthy and
bound only to `127.0.0.1:5432`, `:7000`, `:8080`, and `:4173`. The public
schema retains zero non-internal trigger and zero function. No protocol,
schema, replay vocabulary, route/module catalog, Lua revision, version,
frozen-V1 source, tag, commit, push, release, public listener, or public
artifact was created in Block 7.

## Rejected attempts and residuals

Earlier live attempts remain preserved but rejected. They exposed and led to
corrections for bootstrap message interleaving, a startup race that assigned
the wrong anchor, missing door traversal, duplicate ping/revive availability,
accepted-ping display ordering, and terminal contribution display ordering.
The accepted `20260907h` run occurred after those corrections.

Rendering used Mesa llvmpipe and dummy audio because the WSL shell has no
playback device. Hardware/GPU/audio-device presentation, every build/weapon
pair, repeated fresh/reused progression, terminal failure families, both
disconnect identities, retry/conflict/cancellation, compatibility repetition,
and resource envelopes remain the explicit Block 8 matrix. Creator playback
and automation prove the reviewed semantics and renderability only—not
outside-player comprehension, accessibility experience, social quality,
preference, or enjoyment.

## Gate decision

Gate 7 is green. Godot now projects the complete M28 cooperation lifecycle
from validated server truth, withholds optimistic mutations, remains bounded
and keyboard-operable, reconciles real wire/persistence/replay/Inspector truth,
and preserves every protected compatibility boundary. Block 8 may now freeze
and execute its finite runtime/recovery/resource matrix; it may not expand the
accepted M28 state space or make human-behavior claims.
