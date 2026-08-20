# Revenant Client 0.1.0 — Frozen Protocol V1

This directory is the authorized M14 source freeze of Revenant's own Protocol V1 diagnostic client. It contains an independent copy of the V1 MessagePack schemas and framing rather than depending on `revenant-protocol`, so later protocol changes cannot silently alter the archived client.

The frozen client performs handshake, local authentication, character listing, and world join. It intentionally stops after proving that the current runtime can restore its expected backend contract. All source and protocol evidence is original to Revenant.

Build with `cargo build -p revenant-frozen-client-v1`. Run the resulting `revenant-client-v1` against a local gateway with `REVENANT_GAME_ADDR` when necessary. Do not evolve this directory after M14 except to repair preservation metadata; new clients belong outside the archive.
