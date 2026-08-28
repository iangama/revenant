# Revenant contributor rules

The authoritative project specification and milestone plan is the `AGENTS.md` supplied for this workspace session. Work strictly milestone by milestone; never start the next milestone without a new instruction.

Before changing code: inspect the repository, identify the current milestone, run existing checks, and form a short plan. Before reporting completion: run formatter, linter, tests, build, the relevant smoke test, and inspect `git status`.

The server is authoritative. Keep protocol parsing outside domain systems, prefer composition and explicit observable state, avoid premature services and abstractions, and do not use third-party proprietary code, assets, private protocols, or data.

M0-M23 are complete and integrated. Version 0.2.0 remains the published recoverable baseline while preserving every previous acceptance and compatibility flow. M24 is the current planning milestone: Closed Playtest Readiness and Product Validation.

For M24, build a bounded evidence loop for five to eight supervised first-contact sessions before selecting content, interaction, resilience, or core revision as the next direction. Keep authoritative replay separate from local opt-in UX observation, collect no unnecessary personal data, and do not expose the gateway publicly. Preserve server authority, Protocol V2, frozen V1 artifacts, all canonical flows, and version 0.2.0. Do not create `v0.3.0`, tag, release, recruit participants, or start a later block without explicit owner authorization. Do not create a commit without explicit owner authorization.
