# Revenant

Revenant is an original game-engineering and software-preservation project. It will evolve into a small online game, an authoritative client-server runtime, and eventually a controlled compatibility laboratory for old builds of Revenant itself.

The repository contains the completed **M0–M38 scope**, retaining version **0.2.0**. The local solo game has a six-chapter campaign with optional records and endings, five weapons, ten fixed modules, eight challenge contracts, bounded modifiers, and six masteries with three non-power archive titles. Server authority, confirmed rewards, replay history and frozen V1 compatibility remain intact. English/Portuguese, remappable controls, scalable menus, contrast, captions and reduced motion/flash are available. Current scope and evidence are in [the M33–M38 ledger](docs/roadmap-m33-m38-experience-expansion.md).

The [M38 local archive guide](docs/operations/m38-local-archive.md) covers the accepted private Windows/Linux checkpoint `revenant-m38-local-0.2.0-56a3877cf431`, preserved outside the repository. Linux and Windows install/restart, restored campaign/mastery history, manifests and identical ZIP repacking passed. The accepted M32 archive `revenant-m32-local-0.2.0-0abebdfc8b58` remains preserved separately; its [historical guide](docs/operations/m32-local-archive.md) describes that earlier package. This source update does not create a new binary release or change the product version. Private backups, installation secrets and local continuation notes are kept outside Git.

## Repository map

- `runtime/gateway`: Rust runtime with health and TCP handshake listeners
- `runtime/identity`: local account policy and deterministic character creation
- `runtime/inventory`: server-owned item catalog and reward validation
- `runtime/protocol`: versioned MessagePack messages and framing
- `runtime/progression`: experience reward validation and deterministic levels
- `runtime/world`: authoritative world entry and player actor allocation
- `runtime/actors`: actor registry and explicit lifecycle
- `runtime/combat`: authoritative range, cooldown, damage, health, and death
- `runtime/compatibility`: version negotiation and wire-to-canonical adapters
- `runtime/ai`: server-side finite-state enemy decisions
- `runtime/objectives`: generic objective state and trigger transitions
- `runtime/activities`: generic restricted Lua activity loader and orchestrator
- `runtime/persistence`: PostgreSQL schema and typed persistence boundary
- `runtime/replay`: persisted-event vocabulary and deterministic state reconstruction
- `tools/revenant`: textual replay command-line tool
- `tools/fake-client`: `revenant-bot` handshake client
- `web/control-panel`: read-only session and event Inspector
- `archive/clients/v1`: frozen, author-controlled Protocol V1 client and evidence
- `tools/reconstruction-server`: isolated Protocol V1 backend reconstructed from frozen evidence
- `client/game`: Godot 4 game client
- `client/game/protocol`: Godot wire codec and bounded framed TCP transport
- `client/game/session`: Protocol V2 connection and initial-session orchestration
- `client/game/projection`: presentation-neutral authoritative client state
- `client/game/input`: local movement and attack intent collection
- `infra`: local PostgreSQL and gateway containers
- `docs`: architecture, protocol, and milestone records
- `tests`: canonical smoke flow
- `release`: ignored local release artifacts created by `make release`

## Prerequisites

- Rust stable with `rustfmt` and `clippy`
- `curl`
- Docker Compose for local infrastructure
- Python 3.10+ for backup tooling and its safety tests
- Godot 4.7.1 for opening the game client

## Validate

Use focused tests and a representative flow for the changed behavior, following
[the proportional validation rules](AGENTS.md). Text-only changes need text/diff
review. The full suite remains available for cross-cutting changes or packaging:

```bash
make check
```

The command checks Rust formatting/lint/tests/build, the Inspector TypeScript/build, a two-bot shared activity, the complete Godot flow, persistence, replay reconstruction, Inspector API responses, V1 compatibility, and the isolated M15 reconstruction experiment. PostgreSQL must be available through `DATABASE_URL`.

The creator-only M25 runtime matrix is available as
`tests/m25-runtime-matrix.sh` after a workspace build. It runs both weapons in
solo and two-active-client sessions at simulated 0/75/150 ms RTT, repeats ten
activities per weapon, and reconciles combat, resources, PostgreSQL replay,
and Inspector state into a checksum-verifiable local evidence directory.

Create the local 0.2.0 distribution with `make release`; see `docs/release-0.2.0.md`. The owner-only backup command creates a custom archive, verifies its checksums, restores every table and sequence in a disposable container, and applies the existing migrations twice. See [PostgreSQL backup and restore](docs/operations/postgresql-backup.md). The [Gate 5 recovery audit](docs/security/m30-gate5-recovery-audit.md) records 60/60 passing recovery cases and restored application health in about two minutes in the small disposable fixture.

Future stable releases are validated and published by `.github/workflows/release.yml` when an annotated `vMAJOR.MINOR.PATCH` tag is pushed. Its manual dry-run mode executes the same validation and produces an artifact without publishing a release.

`VERSION` is the canonical product version. `scripts/check-version.sh` verifies the required Cargo, npm lockfile, and Godot representations, and both local and CI release packaging derive artifact paths from that canonical value.

## Run locally

Generate the owner-only local database files once, then run the complete local
stack:

```bash
scripts/m30-generate-secrets.sh current
export REVENANT_SECRETS_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/revenant/m30-secrets/current"
docker compose -f infra/docker-compose.yml up --build
curl http://127.0.0.1:8080/health
cargo run -p revenant-bot
```

If an explicit path is needed, copy `.env.example` to `.env`, set only
`REVENANT_SECRETS_DIR` to an absolute owner-controlled Linux path, and pass
`--env-file .env`. Do not commit `.env`.

Open `http://127.0.0.1:4173` for the Inspector. The container serves the static application and proxies its read-only `/api/inspector` requests to the gateway.

Open `client/game/project.godot` with Godot 4.7.1 to play the vertical slice. The client opens on an explicit local identity and endpoint screen. Settings provides audio levels/mute, display mode, guidance density, 100/125/150% interface size, high contrast, sound captions, separate motion/flash reduction, and English, Brazilian Portuguese, or expanded-text preview. The Controls tab remaps gameplay keys and controller buttons; assigning an occupied input swaps its bindings. Tab/arrows/D-pad navigate menus, Enter/South selects, and Escape/East closes them. The left stick moves and the shoulder buttons cycle weapons by default.

With default bindings, use WASD, arrow keys, or the on-screen directional pad to move; aim the cursor at the active enemy and click, press Space, or use the on-screen Attack button. Keyboard/controller attacks use the active enemy without requiring cursor aiming. Press H to revisit contextual guidance and Escape to open in-session settings. After defeating the relay drone, move to `x=6` to open the relay core and fight the Warden. Every audio cue remains optional and has a visual or textual counterpart. The gateway prepares a fresh run automatically after all players leave the current session. Shortcuts below use defaults; the interface displays remapped keys.

The Relay Field Archive adds three optional stories to the hub. After clearing the drone, find the amber terminals near the arrival platform and maintenance bench, then press **E** to read. Press **J** or click the Archive button to revisit discoveries; **Escape/J** closes the reader. The final core memory becomes available after defeating the Warden. Reading closes if combat resumes, and discoveries last for the current run.

Press **R** after clearing the drone to choose an optional route. Breach can resolve to **Arc Surge**, whose Arc Warden has cyan fins and repositions after each nonlethal hit. Follow its movement trail and reacquire your target. The event is selected by the server; the ordinary Warden encounter remains available by continuing straight to the door.

In a solo run, the **Lost Signal** excursion is another option after the drone, before choosing a route. Head south and west to the amber beacon at `[-4, 4]`. The signal sentinel fires periodically at long range; its barrier blocks movement and shots in both directions. Go around either end, defeat it, then approach the terminal at `[-4, -3]`. The recovered transmission belongs to the current run. Continue east to the normal Warden for the standard mission reward. Taking this excursion selects the standard mission path instead of Breach/Stabilize.

The **Coolant Run** offers a second solo excursion before route selection. Enter the north intake at `[-1, -5]`, then reach transfer `[4, -5]` and delivery `[4, -1]`. Stay at each station for 1.2 seconds and finish within eight seconds. Leaving a station restarts its charge. If time expires, return to intake to retry or continue to the core. This excursion selects the standard path; defeating the normal Warden grants the usual reward.

The **Meridian annex** is an untimed solo excursion after clearing the drone,
before selecting another route or excursion. Follow the west passage through
`[-12, 0]` to Arrival Lock. For **Meridian Survey**, visit Arrival, the Lens
Cistern to the north, then the Sky Gallery to the south. For **Keeper's Return**,
recover the diary on the western walkway and bring it back to Arrival; the map
marks that return point with **5**. The loop always leads east to the hub and
Warden, and both activities can be skipped. A hidden inscription in the gallery
unlocks a fourth record in **Archive [J]**, readable throughout the current run
even with guidance off. Exploration grants no separate currency or XP; normal
Warden completion grants one fragment and 100 XP. Disconnecting starts a fresh
run while preserving the completed or interrupted session's replay evidence.
The M32 archive remains the sealed earlier checkpoint. M38 includes Meridian
and the later campaign/challenge content.

After clearing the drone, the **Glass Lancer** is an optional solo encounter
at the south training marker `[4, 8]`. Its bright floor line locks a charge:
step sideways when warned, then attack during recovery. Leaving the marked
pad withdraws from the encounter and lets you continue north, then east to
the normal Warden. Winning or withdrawing grants no separate XP or loot.
The warning has text, a sound cue and an optional caption; it remains visible
with reduced motion and flash.

The **Steel Bulwark** offers a second solo training encounter at `[6, -8]`.
Approach via `[4, -8]` after clearing the drone. Its fixed front shield blocks
shots; flank it or attack during recovery when the shield lowers. The marked
front wedge warns of a close-range slam. Leaving the pad withdraws and lets
you return south to the core door `[6, 0]`. Training adds no separate reward.
M34's Relay Mender support enemy, elite combinations and two-phase Prism Warden
are complete and are also used by the campaign and challenge contracts.

The inventory panel is read-only and reflects server messages. Completing `relay_awakening` without an optional route grants every participating character one persisted `relay_core_fragment`; the route console shows rewards for optional routes.

The progression display is also read-only. Each completion grants 100 XP; characters begin at level 1 and gain one level per 500 total XP.

Press **1**, **2**, or **3** to select Pulse Rifle, Arc Sidearm, or Coil Lance. The lance has 48 base damage, range 9, and a 350 ms cooldown. Selection is an intent: ownership, equipability, damage, range, cooldown, and the persisted result remain server-authoritative.

Press **M** to open the module workshop. Focus Lens trades two range for 20% more damage; Cycle Bypass trades two range for a 20% shorter cooldown. Each costs two fragments. The workshop previews all five weapons and saves up to three modules for the next admission. Use **Q/F**, the weapon buttons, or the controller shoulder buttons to cycle through the full arsenal, including Rail Driver and Scatter Caster. Within Protocol V2, the current Godot client negotiates `m35-v3` for standalone play, `m36-v2` for campaign play, and `m37-v9` for challenges and the mastery archive. Older V2 clients retain their negotiated content and need a current client to use newer saved equipment; historical replays retain their recorded catalog.

The game protocol listens on TCP port 7000. Clients complete a versioned handshake, authenticate a local username, and request their character list; see `docs/protocol/README.md` for the contract.
The current protocol is V2. The gateway also accepts the frozen V1 client through `revenant-compatibility`; both versions map into the same canonical domain inputs.

For the controlled reconstruction experiment, run `revenant-reconstructed-v1` on a separate address after stopping the gateway. It is intentionally a one-client, one-session evidence harness: it depends only on generic serialization crates, owns its reconstructed V1 wire types, and exits after restoring the frozen client's handshake, local identity, character list, and world join. It is not a replacement production backend.

Activity content is loaded from `scripts/activities/relay_awakening.lua`; override the path with `REVENANT_ACTIVITY_SCRIPT`.

`REVENANT_EXPECTED_PLAYERS` controls how many authenticated players must join before the shared activity starts. It defaults to `1`; the canonical M12 smoke uses `2` and verifies that both clients receive the same replicated state.

To replay a persisted session:

```bash
cargo run -p revenant-cli -- replay <session-id>
# or locate the latest completed session for a local account
cargo run -p revenant-cli -- replay --latest local:revenant-bot
```

The installed binary form is `revenant replay <session-id>`. It prints the ordered event timeline and a compact reconstructed state summary.

The host tools require exactly one of `DATABASE_URL` or `DATABASE_URL_FILE`;
there is no committed database credential fallback. Normal Compose uses
owner-only files generated outside the repository by
`scripts/m30-generate-secrets.sh`, runs idempotent migrations with the
administrator secret, reconciles a least-privileged runtime role, and gives
the Gateway only its runtime URL file. PostgreSQL has no normal host port;
use the explicit `infra/docker-compose.maintenance.yml` override for bounded
local maintenance and close it afterward. The canonical test target may also
receive `DATABASE_ADMIN_URL` for test-only schema/failure-trigger fixtures;
normal smoke and Gateway processes continue to use the runtime
`DATABASE_URL`.
