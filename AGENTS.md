# Revenant contributor rules

The authoritative project specification and milestone plan is the `AGENTS.md` supplied for this workspace session. Work strictly milestone by milestone; never start the next milestone without a new instruction.

Before changing code: inspect the repository, identify the current milestone, run existing checks, and form a short plan. Before reporting completion: run formatter, linter, tests, build, the relevant smoke test, and inspect `git status`.

The server is authoritative. Keep protocol parsing outside domain systems, prefer composition and explicit observable state, avoid premature services and abstractions, and do not use third-party proprietary code, assets, private protocols, or data.

M0-M20 are complete and version 0.2.0 is published as the recoverable M17-M20 baseline while preserving every previous acceptance and compatibility flow. Stop before M21.

For M20, validate ownership and equipability server-side, keep cooldowns authoritative across weapon switches, retain all previous smoke coverage, and do not modify frozen V1 artifacts. Do not begin armor, upgrades, random modifiers, trading, level effects, or M21 work. Do not create a commit without explicit owner authorization.
