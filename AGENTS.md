# Revenant contributor rules

The authoritative project specification and milestone plan is the `AGENTS.md` supplied for this workspace session. Work strictly milestone by milestone; never start the next milestone without a new instruction.

Before changing code: inspect the repository, identify the current milestone, run existing checks, and form a short plan. Before reporting completion: run formatter, linter, tests, build, the relevant smoke test, and inspect `git status`.

The server is authoritative. Keep protocol parsing outside domain systems, prefer composition and explicit observable state, avoid premature services and abstractions, and do not use third-party proprietary code, assets, private protocols, or data.

M0-M21 are complete and version 0.2.0 remains the published recoverable M17-M20 baseline while preserving every previous acceptance and compatibility flow. M21 is integrated on `main` as a visual-identity slice and does not change the published version. Stop before M22.

For M21, preserve server authority, protocols, persistence, mechanics, the frozen V1 artifacts, the measured visual budget, and the original-asset provenance. Audio remains deliberately deferred. Do not begin M22, create `v0.3.0`, tag, or release without a new instruction. Do not create a commit without explicit owner authorization.
