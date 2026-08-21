# Revenant contributor rules

The authoritative project specification and milestone plan is the `AGENTS.md` supplied for this workspace session. Work strictly milestone by milestone; never start the next milestone without a new instruction.

Before changing code: inspect the repository, identify the current milestone, run existing checks, and form a short plan. Before reporting completion: run formatter, linter, tests, build, the relevant smoke test, and inspect `git status`.

The server is authoritative. Keep protocol parsing outside domain systems, prefer composition and explicit observable state, avoid premature services and abstractions, and do not use third-party proprietary code, assets, private protocols, or data.

M0-M16 are complete and version 0.1.0 is published. M17 adds the manual Godot playable vertical slice while preserving the automated acceptance client. Stop before M18.

For M17, keep movement and combat server-authoritative, retain all previous smoke coverage, do not modify frozen V1 artifacts, and do not begin loot/inventory gameplay from M18. Do not create a commit without explicit owner authorization.
