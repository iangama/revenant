# M16 — Release Baseline 0.1

Status: implementation and validation prepared; commit and tag await explicit owner authorization.

M16 turns the completed M0–M15 laboratory into a recoverable `0.1.0` baseline. It adds automated secret/path auditing, locked release builds, deterministic packaging, checksums, PostgreSQL backup and isolated restore-drill instructions, CI release artifact generation, and clean extraction verification.

Local outputs live under ignored `release/`. Database dumps use the ignored `.pgdump` suffix. Neither local tooling nor credentials are distributable artifacts.

This milestone changes no gameplay, protocol, domain, persistence schema, or compatibility behavior. Creating the first commit and annotated `v0.1.0` tag remains a separate, explicitly authorized final action.
