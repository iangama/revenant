# Revenant 0.2.0 release baseline

Release status: published as the stable `v0.2.0` baseline.

This release packages milestones M17 through M20 on top of the published 0.1.0 baseline. It makes `relay_awakening` manually playable, activates authoritative loot and inventory, adds deterministic persisted progression, and introduces one persisted weapon slot backed by server-owned weapon profiles.

## Highlights

- M17: a playable Godot vertical slice with authoritative movement, combat, objectives, encounter state, and reusable gateway sessions;
- M18: validated, persisted, idempotent activity loot with a server-owned catalog and inventory presentation;
- M19: validated experience rewards and deterministic levels persisted with activity completion;
- M20: ownership-checked weapon selection, persisted loadout, authoritative damage, range, and cooldown profiles, and cooldown continuity across weapon switches.

Protocol V2 remains current. The author-controlled Protocol V1 client stays frozen and continues to work through the compatibility adapter and the isolated reconstruction harness. No file under `archive/clients/v1` is evolved by this release.

## Build

Run the complete source validation, then create the Linux x86-64 distribution:

```bash
make check
make release
```

The release script requires Rust and Inspector versions to match, uses locked Rust dependencies and a clean npm install, produces normalized archive metadata, records per-file checksums, and writes a checksum for the final archive. It refuses to overwrite an existing release directory.

## Distribution contents

- `bin/revenant-gateway`: authoritative runtime with M17-M20 behavior;
- `bin/revenant`: replay CLI;
- `bin/revenant-client-v1`: unchanged frozen V1 client;
- `bin/revenant-reconstructed-v1`: isolated M15 reconstruction harness;
- `inspector/`: production static Inspector;
- `game/`: Godot project entry files for the playable slice;
- `scripts/activities/`: Lua activity content and reward declarations;
- `docs/`: release and PostgreSQL recovery guidance;
- `SHA256SUMS`: checksums for every packaged file.

## Upgrade and persistence

The gateway applies the inventory reward, progression reward, and equipment loadout migrations idempotently on startup. Existing accounts, characters, activity history, and replay events remain in PostgreSQL. Operators retain `pulse_rifle` as the default weapon and gain the server-defined `arc_sidearm`; the database stores only the selected item identifier.

Back up PostgreSQL before an operational upgrade. The checked-in backup and isolated restore-drill procedure does not replace or delete the Compose volume.

## Clean-install verification

Extract the archive into a new temporary directory, validate `SHA256SUMS`, run `bin/revenant-reconstructed-v1` and `bin/revenant-client-v1` together on an isolated local port, serve `inspector/` with a static HTTP server, and open `game/project.godot` with Godot 4.7.1. The complete current-runtime, PostgreSQL, Godot, compatibility, and reconstruction acceptance test remains `make check` from source.

## Scope limits

This release does not add armor, extra equipment slots, upgrades, random modifiers, crafting, trading, level-based combat effects, a new activity, class, weapon, or map. M21 is outside this release.

## Publication checkpoint

The immutable checkpoint is an annotated `v0.2.0` tag on the clean, fully validated `main` commit. The release does not include `.env`, `.tooling`, build outputs, database dumps, or local release artifacts.
