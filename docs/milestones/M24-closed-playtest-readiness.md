# M24 — Local Package Readiness and Solo Product Validation

Status: complete as solo technical readiness; Blocks 1-8 are closed, the exact unpublished package is frozen, and M25 remains separately gated.

## Owner scope revision

The original M24 plan targeted five to eight supervised first-contact sessions.
After Gate C, the owner explicitly limited the active project to work one person
can perform alone. No participant, sentinel, recruitment, interview, external
playtest, public alpha, or launch is now required or planned.

The existing package, consent material, local observation contract, and
first-contact hypotheses remain preserved as engineering history. They will not
be used to imply human validation. Active completion now consists of automated
endurance/recovery, bots and multiple local clients, creator dogfooding, and a
technical synthesis that marks human comprehension and broader enjoyment as
unanswered.

## Objective

Turn the integrated M23 vertical slice into a reproducible local package and a
bounded solo evidence loop before adding another activity, changing
authentication or protocol, or declaring a new release. M24 may establish
correctness, resilience, creator-operability, and creator preference; it cannot
establish what a new person understands or wants.

The milestone must distinguish authoritative session history from non-authoritative UX observation. Existing replay events remain the reconstruction vocabulary. Playtest evidence may derive summaries from those events, while optional client observations remain local diagnostics and never become domain facts.

## Deferred human hypotheses

- A new player can enter a session without developer explanation.
- The player understands movement, aiming, attack, equipment, the relay-door transition, and completion from the implemented presentation.
- Confirmed damage, objectives, loot, progression, and equipment feel legible without exposing server internals.
- `relay_awakening` sustains one useful 10–15 minute first-contact session including setup, play, retry, and feedback.
- The strongest reason not to continue is identifiable as content depth, interaction/clarity, technical reliability, or lack of interest rather than guessed in advance.

These remain hypotheses, not success claims, and are outside the active solo
roadmap.

## Active solo hypotheses

- Repeated fresh and reused-account sessions complete without partial or
  duplicate authoritative rewards.
- Two local clients and bots preserve multiplayer atomicity and clean session
  reset over repeated sessions.
- Bounded process and database interruptions fail closed and recover on a fresh
  Retry without volume recreation.
- The exact Windows package remains checksum-verifiable and usable from a clean
  extraction by the creator.
- The creator can identify one evidence-backed next development direction while
  labeling personal preference separately from deterministic facts.

## Active operator and environment

- The owner is the sole operator and player; there is no external audience.
- Desktop sessions run on the owner's local Windows/WSL environment, using bots
  or multiple local clients where multiplayer coverage is required.
- Windows desktop is the local export target; Linux remains a development and validation environment.
- Keyboard and mouse are the required acceptance path. Existing on-screen controls remain supported. Gamepad and complete remapping remain candidate follow-up work rather than hidden M24 scope.
- No public internet gateway, external deployment, account provider, password,
  credential collection, or other-person workflow.

## Solo evaluation questions

1. Does repeated execution preserve authoritative integrity, recovery, bounded
   resources, and exact reconciliation?
2. Which presentation or combat behaviors does the creator judge unclear or
   unsatisfying, separately from measurable system failures?
3. Which one next direction offers the strongest combination of creator
   interest, technical leverage, and bounded implementation risk?

Automated and authoritative facts are recorded before creator interpretation.
No self-observation is labeled as first-contact evidence.

## Evidence boundaries

### Authoritative derived evidence

Server-side summaries may be computed from the existing persisted replay stream:

- join-to-activity-start duration;
- activity-start-to-completion duration;
- completion or incomplete session;
- enemy and boss lifecycle;
- accepted equipment changes;
- authoritative loot and progression outcomes;
- participant count and activity identifier.

These are derived views. They do not add reconstruction events or change existing replay payloads.

### Local observational evidence

When an explicit playtest mode is enabled, the Godot client may write a bounded local report containing:

- anonymous participant code supplied for the test;
- build version and commit identifier;
- viewport size and display mode;
- timestamps for entry ready, connect request/outcome, first movement attempt, first attack attempt, settings opened, completion observed, disconnect/failure, and quit;
- enumerated connection or validation outcome;
- guidance, mute, reduced-flash, and display preferences;
- aggregate cooldown acknowledgement count, never raw key or pointer history.

The report is non-authoritative, is not sent over Protocol V2, and cannot update replay, persistence, progression, inventory, equipment, activity, or Inspector truth.

### Prohibited evidence

- IP or hardware identifiers;
- passwords, external account identifiers, email, voice, video, or screen recording by default;
- raw usernames in the local report;
- arbitrary text logs, chat, keylogging, pointer trails, or full input history;
- automatic upload or background network transmission;
- indefinite retention;
- metrics that cannot be tied to one of the three research questions.

## Block plan

1. **Research brief and data contract:** finalize audience, session format, research questions, success signals, evidence fields, opt-in language, retention, deletion, and exit criteria.
2. **Authoritative session summary:** derive a typed playtest-oriented summary from existing replay events and expose it through the read-only Inspector without changing replay vocabulary.
3. **Local opt-in observation report:** add a bounded client-side collector and deterministic export fixture, disabled unless explicit playtest mode is active.
4. **Failure and recovery audit:** exercise unavailable gateway, rejected handshake, busy session, mid-activity disconnect, client termination, gateway restart, database unavailability, and idempotent retry; record current outcomes before proposing reconnect semantics.
5. **Display and first-contact matrix:** validate the current package at representative desktop resolutions, focus paths, mute/reduced-flash modes, clean user data, malformed settings, and a second run; fix only playtest-blocking defects.
6. **Closed-playtest package:** produce a versioned but unpublished package, one-page participant instructions, operator runbook, consent language, feedback form, reset procedure, and checksum evidence.
7. **Solo endurance and dogfooding:** run repeated fresh, reused-account,
   multiplayer-bot, recovery, and exact-package paths; measure resources and
   separate creator notes from authoritative evidence.
8. **Solo technical synthesis:** rank deterministic and creator findings,
   explicitly preserve unanswered human questions, and select one bounded next
   solo direction.

Every block requires separate review. A completed planning or instrumentation block authorizes neither participant recruitment nor later blocks, commit, push, PR, merge, version change, tag, release, or public exposure.

## Success signals

- At least 20 sequential solo completions and 10 two-client local completions
  reconcile with exact per-session and accumulated rewards.
- Sampled interrupted sessions contain no partial completion reward, and every
  successful Retry is a distinct fresh session.
- Gateway memory, file descriptors, database connections, and local report size
  remain bounded across the endurance window.
- Every observed blocker is traceable to evidence and classified as creator
  clarity/preference, interaction, gameplay/content, reliability, or
  environment.
- Local reports contain only allow-listed fields and are disabled outside explicit playtest mode.
- Server-derived summaries reconcile with replay reconstruction for the same session.
- The synthesis selects one next milestone direction rather than producing an unbounded backlog.

## Stop conditions

- Stop solo endurance on the first reproducible data-loss, reward-duplication,
  unrecoverable-session, privacy, or unbounded-resource defect.
- Stop instrumentation work if it requires a Protocol V2 change; review that need separately.
- Do not begin public-distribution planning in the active solo roadmap.
- Stop content expansion during M24; content requests are evidence for the next decision, not permission to implement them.

## Compatibility and acceptance

- The server remains authoritative for all gameplay and persistence.
- Protocol V2 wire bytes, ordering, limits, and failure outcomes remain unchanged unless a separately approved protocol milestone supersedes this rule.
- Frozen V1 artifacts and hashes remain unchanged.
- Existing replay vocabulary remains sufficient for deterministic reconstruction.
- Inspector additions remain read-only.
- The client collector performs no network upload and creates no report when playtest mode is disabled.
- All M17–M23 markers, automatic/manual flows, persistence/replay, Inspector, packaging, compatibility, and reconstruction remain green.
- `VERSION` remains `0.2.0`; no tag, release, Steam deployment, or public launch is part of M24.

## Risks and mitigations

- **Creator bias:** use fixed scripts and record authoritative facts before
  interpretation; label self-play conclusions as creator preference.
- **Missing human evidence:** keep first-contact comprehension, accessibility on
  other bodies/hardware, and wider enjoyment explicitly unanswered.
- **Replay contamination:** derive summaries without adding UX events to the authoritative event vocabulary.
- **Privacy creep:** use an allow-list, local opt-in export, short retention, and explicit deletion.
- **Instrumentation authority leak:** client observations never confirm or mutate gameplay outcomes.
- **Telemetry platform expansion:** prefer existing persistence and Inspector boundaries; no external telemetry backend is required.
- **Premature polishing:** repair only defects that block or invalidate the solo
  evidence during M24.
- **Public-network scope creep:** keep the gateway on trusted local infrastructure throughout M24.

## Research basis

- Godot supports runtime action mapping, controller abstraction, resolution scaling, and pseudolocalization; these are candidates for evidence-led follow-up rather than automatic M24 scope.
- WCAG 2.2 keyboard, focus, animation, and target-size criteria are useful review heuristics without claiming web conformance for the game client.
- OWASP recommends threat modeling before external exposure; M24 deliberately avoids turning local identity and TCP transport into a public service.
- OpenTelemetry Rust traces, metrics, and logs remain beta as of the planning date, so the existing replay/Inspector boundary is preferred for this bounded solo validation.

References:

- https://docs.godotengine.org/en/stable/classes/class_inputmap.html
- https://docs.godotengine.org/en/latest/tutorials/rendering/multiple_resolutions.html
- https://docs.godotengine.org/en/stable/tutorials/i18n/pseudolocalization.html
- https://www.w3.org/TR/WCAG22/
- https://owasp.org/www-project-threat-modeling/
- https://opentelemetry.io/docs/languages/rust/

## Definition of done

M24 is complete when the bounded evidence system and exact package are
validated, the solo endurance/recovery matrix passes, creator notes are kept
distinct from authoritative facts, unanswered human questions remain explicit,
one next solo direction is selected, all compatibility and release invariants
pass, and the result is reviewed without silently starting the selected
follow-up milestone.

## Block 1 checkpoint

The research brief, participant boundary, fixed research questions, initial decision thresholds, consent and retention baseline, stop conditions, moderator protocol, finding classification, and next-direction rule are recorded in this milestone and `docs/playtest/m24-research-protocol.md`.

`docs/playtest/m24-data-contract.md` defines the two evidence products before implementation. `AuthoritativeSessionSummary` is a read-only derivation of existing replay events; `LocalObservationReportV1` is a 16 KiB-bounded, local-only, explicitly activated JSON report containing allow-listed environment, first-occurrence, outcome, aggregate, and consent fields. It stores no username, host, IP, raw input history, wall-clock interaction timeline, arbitrary error text, or automatic upload target.

The contract fixes null and contradiction behavior, participant-code format, atomic local storage, 30-day maximum retention, early deletion, reconciliation, and the minimum fixtures later blocks must implement. It changes no code, schema, replay vocabulary, protocol, server authority, published version, tag, release, or participant state. Block 2 may implement only the authoritative derived summary after separate review.

## Block 2 checkpoint

`revenant-persistence` now owns the typed `AuthoritativeSessionSummary` projection for one session. It derives counts and lifecycle facts solely from the existing persisted event kinds, uses replay timestamps only for elapsed projections, returns `null` for a missing start or completion, and rejects a missing join or negative elapsed evidence rather than manufacturing a result. The query is read-only and bounded by the canonical session identifier accepted by the gateway.

The gateway exposes `GET /api/inspector/sessions/{session_id}/summary` without adding protocol messages or mutation paths. The Inspector consumes that endpoint to show join-to-start, activity duration, participant, enemy, boss, accepted equipment, loot, progression, and event totals next to the unchanged event stream. Persistence fixtures cover a completed solo session, an incomplete session with no start, a started session with no completion, two participants, and a missing session; the canonical smoke reconciles a completed two-participant session with the summary endpoint.

Block 2 changes no database schema, replay vocabulary, replay payload, gameplay authority, frozen V1 artifact, version, tag, release, participant recruitment, or local observation state. The `LocalObservationReportV1` remains deferred to separately reviewed Block 3.

## Block 3 checkpoint

The Godot client now owns a local-only `LocalObservationReportV1` collector under `client/game/playtest`. It is inactive unless playtest mode, a valid anonymous participant code, a bounded operator build ID, and observation consent are all explicit. Retention consent is independent: confirmed reports use atomic temporary-file replacement into `user://playtest/m24-<report_id>.json`; declined reports remain temporary and are deleted during controlled closeout.

`main.gd` supplies only allow-listed semantic observations: connection request/outcome, first movement and attack attempts, settings opening, authoritative completion as observed locally, disconnect, quit, preferences, and a saturating cooldown-acknowledgement aggregate. It supplies no username, endpoint, raw input, arbitrary error, payload, wall-clock interaction timestamp, or authority mutation. Protocol V2 does not currently project the authoritative replay session ID to the client, so the optional `session_id` remains absent rather than expanding the wire contract.

The deterministic M24 harness verifies disabled-by-default and malformed activation, explicit observation consent, bounded build identity, the exact top-level allow-list, a failure before session correlation exists, first-occurrence timing, completion without optional settings interaction, count saturation, the 16 KiB ceiling, retention-declined deletion, and contradiction detection without rewriting either evidence source. Block 3 changes no server, database, replay vocabulary, protocol, gameplay authority, frozen V1 artifact, published version, tag, release, participant recruitment, or network upload. Failure and recovery behavior remains deferred to separately reviewed Block 4.

## Block 4 final checkpoint

`docs/playtest/m24-failure-recovery-audit.md` now separates the behavior found
before correction, the implemented fail-closed and atomic changes, deterministic
fixtures, post-correction runtime evidence, eliminated risks, residual risks,
and operational reconciliation.

The session coordinator now persists required replay before related stage and
client projection, discards a PostgreSQL connection after any failed operation,
and opens a replacement only for the next independent operation. The failed
operation is never replayed automatically. A persistence failure after
admission closes participant sockets, discards the defective session, and
resets a fresh session. Normal cooldown, target, equipment, and movement
rejections remain non-fatal.

Equipment loadout and `equipment_changed` replay now commit together. One
completion transaction contains `activity_completed`, every participant's
inventory and progression reward, history, both idempotency grants,
`loot_granted`, and `progression_granted`. A failure for any participant or
reward replay write rolls back the entire group. Participant reward and
completion messages are emitted only after commit.

Seventeen deterministic gateway tests cover start, spawn, death, boss,
equipment, completion, multiplayer order and reward values, normal command
rejection, participant abort, late disconnect after failed broadcast, and
next-operation connection renewal. Five
PostgreSQL integration tests cover multiplayer rollback, idempotent repetition,
reward/replay atomicity, and equipment/replay atomicity. The Godot harness
separately fixes timeout, immediate refusal, incompatible handshake, occupied
admission, report allow-list, and first-terminal monotonicity.

A real selective `progression_granted` failure produced session
`session-1788089644734997724` with no completion, reward replay, fragment, XP,
history, or grants. Without restarting the gateway, the same account entered
fresh session `session-1788089759666834529`, the replacement connection opened,
and the activity completed with exactly one reward. The gateway container ID
remained unchanged.

An immediate gateway termination during the real manual Godot flow produced
incomplete session `session-1788090621746254249`. PT-H426 retained
`disconnect_observed=512` and `terminal_outcome=disconnected` even after a later
driver failure. After health returned, session
`session-1788090662243881295` restarted the activity and completed with exactly
one fragment, 100 XP, one history row, and one of each grant. The already
committed equipment choice remained durable; no activity resume or reward
duplication occurred.

Retry is always from the activity beginning. No reconnect or resume was added.
Residual risks are limited to loss of in-progress activity, manual correlation
because Protocol V2 does not project a session ID into the local report,
abrupt-exit local-report cleanup, and operator restart if coordinator reset
itself cannot load. These are acceptable only for the supervised local pilot
and must appear in the Block 6 runbook.

The uninterrupted repository gate passed after one local smoke-harness
robustness correction: its five-second fake-client socket deadline could expire
while the second `cargo run` process was still being scheduled. The bounded
fake-client I/O deadline now matches the gateway's 15-second handshake window;
an instrumented smoke and the subsequent full `make check` both completed.

The final gate confirmed version, formatting, Clippy with warnings denied,
workspace and PostgreSQL tests, build, Inspector, secret audit, automatic and
manual Godot flows, every M17–M24 marker, replay, Inspector summary, exact V1
hashes, V1 compatibility and reconstruction, capture manifests, unchanged
protocol/replay/V1 paths, loopback-only bindings, removed failure injections,
no old validators, `git diff --check`, and the intentional final Git state.

Gate A is approved with the bounded residual risks recorded in the audit. No
reproducible integrity, duplication, participant-projection, privacy, or
unrecoverable-session pilot blocker remains. This authorizes only the already
approved Block 5 review. It authorizes neither protocol or replay changes,
automatic reconnect/resume, participant recruitment, public gateway exposure,
Block 6, commit, push, merge, version change, tag, nor release.

## Block 5 checkpoint

`docs/playtest/m24-display-first-contact-matrix.md` records a bounded eight-case
matrix across 1280×720, 1366×768, 1920×1080, 2560×1440, Windowed, Fullscreen,
Full/Compact/Off guidance, mute, Reduced Flash, clean/saved/malformed data,
second process launch, refusal, abrupt termination, and clean Retry.

The baseline 2560×1440 run exposed an unscaled 1280×720 interface. Godot now
uses `canvas_items` with preserved aspect so the authored 2D canvas scales while
3D retains output resolution. Windowed size is remembered across Fullscreen.
Mode restoration waits for compositor convergence, and the matrix rejects any
entry/settings/onboarding/runtime dimension change within a case.

Every entry and settings control is reachable in forward keyboard focus order;
Escape returns focus to its invoker. Focus loss clears latched on-screen
movement. Guidance, mute, and Reduced Flash combinations retain critical
authoritative status. Inventory, progression, and input-history bounds are
asserted after a completion capture exposed and corrected a real overlap.

Equipment now has visible keyboard shortcuts 1/2. A real 1920×1080 graphical
flow used only keyboard events for Sidearm equipment, Space attacks, and held D
movement, then crossed the door, defeated the Warden, and received exactly one
fragment and 100 XP in session `session-1788107050345817032`. Replay, history,
and both idempotency grants reconcile exactly once.

Immediate refusal remains `transport_failure`/`failed`, exposes a focused Retry
from start, invents no session, and creates no database state. SIGKILL during a
separate activity left an incomplete replay prefix and no reward; the same local
data and account then started a fresh session and completed exactly once. An
abrupt report remains `running` because no close callback can execute and must
be reconciled as incomplete by the operator.

Six representative captures and `SHA256SUMS` are retained under
`docs/art/m24/captures`; the full matrix generated and verified 24 captures.
The Linux/WSLg matrix does not substitute for the Windows package dry run in
Block 6, and deterministic reachability does not claim first-time participant
comprehension. No reconnect/resume, protocol, replay vocabulary, schema,
authority, version, frozen V1, public exposure, recruitment, tag, or release
change is part of this checkpoint.

The final uninterrupted repository gate passed version, formatting, Clippy with
warnings denied, all workspace and PostgreSQL tests, build, Inspector, secret
audit, multiplayer, automatic, manual, keyboard, every M17–M24 marker,
replay/summary, frozen V1 compatibility, and V1 reconstruction. A preliminary
invocation had stopped after the version check solely because `cargo` was absent
from this WSL shell; a temporary official toolchain under `/tmp` enabled the
unchanged canonical command and left no validation process.

Post-gate review reconfirmed `VERSION=0.2.0`, exact V1 and M21/M22/M24 capture
hashes, unchanged Protocol V2/replay/V1 paths, zero temporary triggers or
injection functions, loopback-only healthy services, equal local/upstream HEAD,
and the intentional uncommitted tree.

Gate B is approved with bounded residual risk: Windows package validation was
deferred to Block 6, human comprehension remains a pilot hypothesis,
non-16:9/gamepad/remapping remain outside scope, and SIGKILL reports remain
explicitly incomplete. None blocks the supervised local package preparation,
but each must remain visible in later review. Block 6 was subsequently
authorized without authorizing Block 7, recruitment, commit, push, merge, tag,
release, or public exposure.

## Block 6 checkpoint

`docs/playtest/m24-package-manifest.md` identifies one exact unpublished
Windows x86-64 candidate:
`m24-b6-45718926-871f1beafb43`, archive SHA-256
`8cdf81e534b9c2e49472e6cc3cc2c1f4c315114abc22d875767934ed6d62d91e`.
The ID combines frozen HEAD with the SHA-256 of 173 runtime and operational
inputs so the reviewed uncommitted tree is not misrepresented by HEAD alone.
Official Godot 4.7.1 templates were checksum-verified. The export is an
unsigned Windows release executable with a separate PCK, not an installer or
release.

The ZIP contains participant instructions, operator runbook, bounded launcher,
checksum verifier, selective report deletion, and separate consent,
observation, intervention, feedback, and evidence-disposition forms. Fourteen
internal hashes and the complete source manifest verify without repository
files after PowerShell extraction into a new directory. Source, credentials,
logs, reports, database dumps, upload behavior, public endpoint configuration,
tag, and release artifacts are absent.

Package dry-run found and corrected two participant-facing blockers: exported
audio initially lost the raw WAV files expected by `FileAccess`, and report
deletion collided with PowerShell's automatic `$Matches` variable. Audio now
uses imported `AudioStreamWAV` resources with deterministic uncompressed PCM
sidecars; clean import and the final Windows PCK satisfy the existing M22 byte
contract. Report deletion now passes both `-WhatIf` and exact confirmed removal.
Every superseded package ID is explicitly rejected in the package manifest.

The final extracted Windows executable emitted every M17–M24 validation marker
with isolated AppData and no repository dependency. It then completed a real
loopback activity as `local:m24-b6-win-871f1b` in session
`session-1788110148370552260`: one completion, one fragment, 100 XP, one history
row, one inventory grant, one progression grant, and one of each reward replay.
Inspector reconciled the same completed solo session. Observation and retention
were declined for this synthetic run and no local report was created.

The runbook makes checksum, loopback health, exact reset, no-coaching,
PostgreSQL recovery without volume deletion, replay/summary reconciliation,
Retry-from-start, stop conditions, consent, retention, and deletion explicit.
`docs/playtest/m24-closed-package-audit.md` records the corrections, evidence,
and residual risks. The executable remains unsigned; testing used a clean
extraction on the current Windows host rather than a second clean physical
machine; human comprehension, hardware perception, and manual local-report
correlation remain bounded pilot risks.

The uninterrupted final `make check` passed version, formatting, Clippy with
warnings denied, all workspace and PostgreSQL tests, build, Inspector, secret
audit, multiplayer, four Godot flows, every M17–M24 marker, replay/summary,
frozen V1 compatibility, and V1 reconstruction. The smoke imports a temporary
project copy without the worktree `.godot` cache, so the 13 PCM sidecars are
validated from clean source and no generated import state is left behind.

The post-gate audit matched the exact archive and all internal/source hashes,
the Windows clean-extraction verification, capture manifests, frozen V1,
unchanged protocol/replay/V1 paths, loopback-only healthy services, zero
temporary failure objects, equal local/upstream HEAD, and the intentional
uncommitted tree. Gate C is approved with the bounded residual risks listed in
the package audit. This checkpoint does not authorize participants, a sentinel
session, Block 7, commit, push, merge, tag, release, or public exposure.

## Block 7 solo checkpoint

`tests/m24-solo-endurance.sh`, `tests/m24-solo-recovery.sh`, and
`tests/m24-solo-windows.ps1` provide creator-only deterministic coverage
outside the frozen package identity. The accepted endurance campaign completed
20 solo and 10 two-client sessions: 30 distinct completions, 40 joins, and
exactly 40 loot/progression replay pairs and idempotency grants. Fresh accounts,
one account reused ten times, and both participants in every multiplayer
session reconciled exactly; no integrity query found partial or duplicate
reward.

Descriptors, threads, and observed PostgreSQL connections remained unchanged
through both campaign phases. RSS increased by 696 KiB during the solo window
and 1,148 KiB during the multiplayer window. This is bounded endurance evidence,
not an unlimited soak claim.

Real refusal, incompatible protocol, occupied admission, client termination,
gateway restart, and PostgreSQL interruption passed their structured paths.
Incomplete sessions retained only replay prefixes and zero reward; Retry always
used a fresh session. PostgreSQL recovery reused the same volume and gateway
process and renewed persistence on the next operation.

The exact frozen Windows package passed four guidance/audio/Reduced Flash
profiles, keyboard-only network completion, normal and abrupt lifecycle
boundaries, exact report deletion, and hashes before and after execution. The
accepted activity reconciled one fragment, 100 XP, one history row, one of each
grant, and one reward replay pair through Inspector.

The uninterrupted repository gate and post-gate invariant audit passed. Gate
D-S is approved with bounded residual risk. No subjective creator observation
was supplied, and none was inferred from automation.

## Block 8 solo synthesis and M24 closure

`docs/playtest/m24-solo-technical-synthesis.md` separates deterministic facts,
measured local results, absent creator preference, engineering inference, and
unanswered human questions. It ranks the remaining engineering findings and
selects exactly one proposed next direction: **M25 combat pleasure and
fairness**.

Combat is selected because the authoritative vertical slice is stable while
hit feel, weapon contrast, encounter pacing, pressure, and fairness remain the
highest-leverage unmeasured part of the repeated interaction. Progression is
deferred until the combat baseline is measured; operations would otherwise
amplify an untuned loop; cooperation bots cannot establish social quality; and
no M24 evidence justifies a core revision.

First-contact comprehension, wider enjoyment, accessibility on other bodies or
hardware, social cooperation, longer soak behavior, and general Windows
portability remain explicitly unanswered. M24 is therefore closed as **solo
technical readiness**, not as human research or public-release readiness.

This checkpoint starts no M25 work and authorizes no participant, commit, push,
merge, version change, tag, release, or public exposure.
