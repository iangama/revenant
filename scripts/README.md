# Scripts

`generate-meridian-audio.py` reproduces the original M33 exploration bed in
`client/game/audio/m33/meridian.wav`: 16 seconds of mono 16-bit PCM at 22,050 Hz.
It was authored for Revenant on 2026-09-11 with Python standard-library
oscillators, envelopes and seeded noise, with no recordings, third-party
samples or external models. The existing Ambience bus loops it during annex
exploration, retaining mute and optional captions. Meridian's original geometry
and wayfinding are authored in `client/game/presentation/environment/meridian_sector.gd`;
both rendering and authoritative traversal consume `client/game/world/meridian.json`.
The two Lua field activities are `activities/meridian_survey.lua` and
`activities/meridian_return.lua`; their enclosing expedition suppresses separate
activity rewards and persists only ordered objective discoveries.

`python3 -B scripts/m30-backup.py create ABSOLUTE_NEW_LINUX_DIRECTORY` seals a
private database backup only after a disposable restore and two migration
passes match all data and schema exports. `verify DIRECTORY` repeats that
proof without overwriting the backup. See
[`docs/operations/postgresql-backup.md`](../docs/operations/postgresql-backup.md)
for setup, retention, failures, and the accepted Gate 5 application-recovery
proof. These commands have no active restore or teardown target argument.

`tests/m30-recovery-matrix.py --report /absolute/new/evidence/report.json`
drives all thirty Gate 5 recovery cases in two fresh disposable Compose
projects. It requires the built normal images and Rust bots, reserves only
loopback ports 15451, 17451/17452, 18451/18452, and 41451/41452, and removes
only resources bearing its generated ownership labels. The working stack
must stay quiet with PostgreSQL maintenance closed throughout the run.
See the [recovery contract](../docs/security/m30-gate5-backup-recovery-contract.md).

Restricted Lua activity definitions live under this boundary. They define objectives, triggers, completion loot, and experience rewards. Item identifiers and positive quantities are validated by the inventory domain; experience is bounded by the progression domain. Scripts cannot create arbitrary items or author resulting levels.

`check-version.sh` enforces `VERSION` as the canonical product version across Cargo, the Inspector package and lockfile, and the current Godot client build. `release.sh` refuses to package a mismatched tree.

`capture-m21.sh` runs the deterministic Godot visual validation with a graphical renderer and writes the three M21 review shots plus `SHA256SUMS`. Set `GODOT_BIN` when the repository-local Godot executable is unavailable. The default output is `docs/art/m21/captures`; pass another directory as the first argument for a disposable review run.

`capture-m27.sh` starts a source-built loopback gateway and drives the honest
Godot Stabilize route surface through authoritative choice, accepted event,
and terminal summary states. It writes three 1280×720 review captures plus
`SHA256SUMS` to `docs/art/m27/captures` by default. These images demonstrate
renderability and semantic state only; they are not preference evidence.

`measure-m22.sh` performs the opt-in graphical Master-bus and frame-time measurement for the M22 presentation. `capture-m22.sh` produces Entry, Settings, onboarding, and runtime PNGs plus a short MJPEG/PCM AVI and verified `SHA256SUMS`. Its default output is `docs/art/m22/captures`; pass another directory for a disposable run.

`capture-m25.sh` runs the deterministic combat-presentation fixture and writes
the paired local-attempt and authoritative-hit PNGs plus `SHA256SUMS`. Its
default output is `docs/art/m25/captures`; pass another directory for a
disposable review run. The pair is review evidence, never combat authority.

`m30-generate-secrets.sh` creates a non-overwriting, owner-only database secret
generation outside the repository. `m30-rotate-database-secrets.sh` installs a
named candidate only after verifying the sealed pre-change backup, proving new
and old connection behavior, recreating the migration/provisioning/Gateway
containers, and confirming a stable working-data digest. It retains the prior
complete generation as `previous`. `m30-rollback-database-roles.sh` atomically
swaps that generation back, rechecks rejected/recovered connections, recreates
dependants, and preserves the same data digest. These commands never publish
PostgreSQL on the host; `infra/docker-compose.maintenance.yml` remains the only
bounded maintenance override. Open it only for the named task with both
Compose files. Close it with the normal file and `--force-recreate` so
PostgreSQL, migration, provisioner, Gateway, and Inspector return in dependency
order with fresh database connections:

```bash
docker compose -f infra/docker-compose.yml \
  -f infra/docker-compose.maintenance.yml up -d postgres
# perform the bounded host-side task
docker compose -f infra/docker-compose.yml up -d --force-recreate
```

## M30 abuse and Inspector validation

Gate 4 uses `tests/m30-abuse-observability-matrix.py` against a fresh
`m30g4`-prefixed Compose project. Build the normal images first, then combine
`infra/docker-compose.yml` with `infra/docker-compose.m30-abuse.yml` using
`--no-build`. The override uses tmpfs database data and only loopback ports
17440, 18440, and 41473. Never apply it to the normal `infra` project.

With the isolated stack healthy and the local Rust toolchain on PATH, run:

```sh
python3 -B tests/m30-abuse-observability-matrix.py \
  --project m30g4example --report /absolute/new/evidence/report.json
```

The report path must not exist. The probe fingerprints every fixture table and
sequence before/after each rejection and the working database before/after
the complete run. Only the waiting-player A21 setup appends fixture state.
Exact timeout/rate/queue cases share production code; socket cases use the
normal timeouts and runtime image. After collecting the report, remove only
the named disposable project's containers, network, and socket volume with
that same pair of Compose files. A second acceptance run requires a fresh
project and database. Preserve failed evidence separately.

`tests/m30-inspector-browser.mjs` optionally uses the creator host's existing
Playwright/Chromium via `M30_PLAYWRIGHT_MODULE` and `M30_CHROMIUM_BIN`. It proves
same-origin browser GET/no-store and blocked foreign-origin reading. This is
an origin-disclosure check; local V1/V2 identity remains username-only.
