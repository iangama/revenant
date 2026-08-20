# Revenant 0.1.0 release baseline

Release status: prepared locally; commit and tag require explicit owner authorization.

This baseline contains milestones M0 through M15: bootstrap, handshake, identity, world join, actors, authoritative combat and AI, objectives and triggers, Lua activity runtime, PostgreSQL persistence, replay, multiplayer, Inspector, frozen V1 compatibility, and the controlled reconstruction experiment.

## Build

Run `make check`, then create the Linux x86-64 distribution with:

```bash
make release
```

The release script uses locked Rust dependencies, a clean npm install, release-mode binaries, deterministic archive metadata, per-file checksums, and a checksum for the final archive. It refuses to overwrite an existing release directory.

## Distribution contents

- `bin/revenant-gateway`: current authoritative runtime;
- `bin/revenant`: replay CLI;
- `bin/revenant-client-v1`: unchanged frozen V1 client;
- `bin/revenant-reconstructed-v1`: isolated M15 reconstruction harness;
- `inspector/`: production static Inspector;
- `game/`: Godot project entry files;
- `scripts/activities/`: Lua activity content;
- `docs/`: release and PostgreSQL recovery guidance;
- `SHA256SUMS`: checksums for every packaged file.

## Clean-install verification

Extract the archive into a new temporary directory, validate `SHA256SUMS`, run `bin/revenant-gateway --healthcheck`, serve `inspector/` with any static HTTP server, and open `game/project.godot` with Godot 4.7.1. The full network/database acceptance test remains `make check` from source.

## Version control checkpoint

After owner approval, the intended immutable checkpoint is one initial commit followed by annotated tag `v0.1.0`. The tag must only be created after the final checks pass and must not include `.env`, `.tooling`, build outputs, database dumps, or local release artifacts.
