# Revenant contributor rules

The authoritative project specification and milestone plan is the `AGENTS.md` supplied for this workspace session. Work strictly milestone by milestone; never start the next milestone without a new instruction.

Before changing code: inspect the repository, identify the current milestone, run existing checks, and form a short plan. Before reporting completion: run formatter, linter, tests, build, the relevant smoke test, and inspect `git status`.

The server is authoritative. Keep protocol parsing outside domain systems, prefer composition and explicit observable state, avoid premature services and abstractions, and do not use third-party proprietary code, assets, private protocols, or data.

M0-M15 are complete. M16 prepares the recoverable 0.1.0 release baseline with auditing, deterministic packaging, checksums, CI artifacts, and recovery documentation. Stop after M16.

For M16, do not modify frozen artifacts or runtime behavior. Local releases and database dumps must stay ignored. Do not create a commit or tag without explicit owner authorization.
