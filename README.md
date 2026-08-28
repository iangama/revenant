# Revenant

Revenant is an original game-engineering and software-preservation project. It will evolve into a small online game, an authoritative client-server runtime, and eventually a controlled compatibility laboratory for old builds of Revenant itself.

The repository has completed **M22 — Player Experience and Audio Slice** on top of the published 0.2.0 baseline and has begun **M23 — Client Architecture and Validation Refactor**. The playable `relay_awakening` slice retains its server-authoritative inventory, progression, loadout, combat, and activity flow while M23 incrementally separates client protocol, transport, projection, input, presentation, and validation responsibilities without changing behavior or the published version.

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
- `client/game`: Godot 4 project skeleton
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
- Godot 4 for opening the client skeleton

## Validate

```bash
make check
```

The command checks Rust formatting/lint/tests/build, the Inspector TypeScript/build, a two-bot shared activity, the complete Godot flow, persistence, replay reconstruction, Inspector API responses, V1 compatibility, and the isolated M15 reconstruction experiment. PostgreSQL must be available through `DATABASE_URL`.

Create the local 0.2.0 distribution with `make release`; see `docs/release-0.2.0.md`. PostgreSQL backup and safe restore-drill commands are documented in `docs/operations/postgresql-backup.md`.

Future stable releases are validated and published by `.github/workflows/release.yml` when an annotated `vMAJOR.MINOR.PATCH` tag is pushed. Its manual dry-run mode executes the same validation and produces an artifact without publishing a release.

`VERSION` is the canonical product version. `scripts/check-version.sh` verifies the required Cargo, npm lockfile, and Godot representations, and both local and CI release packaging derive artifact paths from that canonical value.

## Run locally

```bash
docker compose -f infra/docker-compose.yml up -d postgres
cargo run -p revenant-gateway
curl http://127.0.0.1:8080/health
cargo run -p revenant-bot
```

To run the local stack:

```bash
cp .env.example .env
docker compose --env-file .env -f infra/docker-compose.yml up --build
```

Do not commit `.env`; the checked-in example contains local-only defaults.

Open `http://127.0.0.1:4173` for the Inspector. The container serves the static application and proxies its read-only `/api/inspector` requests to the gateway.

Open `client/game/project.godot` with Godot 4.7.1 to play the vertical slice. The client opens on an explicit local identity and endpoint screen; Settings controls Master, Ambience, Effects, Interface, mute, display mode, guidance density, and reduced flash. Use WASD, arrow keys, or the on-screen directional pad to move; aim the cursor at the active enemy and click, press Space, or use the on-screen Attack button. Press H to revisit contextual guidance and Escape to open in-session settings. After defeating the relay drone, move to `x=6` to open the relay core and fight the Warden. Every audio cue remains optional and has a visual or textual counterpart. The gateway prepares a fresh run automatically after all players leave the current session.

The inventory panel is read-only and reflects server messages. Each successful `relay_awakening` completion grants every participating character one persisted `relay_core_fragment`.

The progression display is also read-only. Each completion grants 100 XP; characters begin at level 1 and gain one level per 500 total XP.

The loadout offers a `pulse_rifle` and `arc_sidearm`. Selection is an intent: ownership, equipability, damage, range, cooldown, and the persisted result remain server-authoritative.

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

The gateway applies idempotent migrations on startup. `DATABASE_URL` defaults to the local Compose PostgreSQL instance.
