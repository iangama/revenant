# M24 — Closed Playtest Readiness and Product Validation

Status: Block 1 implemented locally as a reviewed planning and data-contract checkpoint; later blocks require separate review.

## Objective

Turn the integrated M23 vertical slice into a controlled closed-playtest package that can answer whether a new player understands, completes, and wants more from Revenant. M24 creates a bounded evidence loop before adding another activity, exposing the gateway publicly, changing authentication or protocol, or declaring a new release.

The milestone must distinguish authoritative session history from non-authoritative UX observation. Existing replay events remain the reconstruction vocabulary. Playtest evidence may derive summaries from those events, while optional client observations remain local diagnostics and never become domain facts.

## Product hypotheses to test

- A new player can enter a session without developer explanation.
- The player understands movement, aiming, attack, equipment, the relay-door transition, and completion from the implemented presentation.
- Confirmed damage, objectives, loot, progression, and equipment feel legible without exposing server internals.
- `relay_awakening` sustains one useful 10–15 minute first-contact session including setup, play, retry, and feedback.
- The strongest reason not to continue is identifiable as content depth, interaction/clarity, technical reliability, or lack of interest rather than guessed in advance.

These are hypotheses, not success claims. Block 1 may refine wording and thresholds, but it may not silently expand M24 into content production.

## Initial audience and environment

- Five to eight participants who have not worked on the project and have not seen the activity solution.
- Supervised or directly supported desktop sessions on trusted local machines or a trusted LAN.
- Windows desktop is the initial distribution hypothesis; Linux remains a development and validation environment.
- Keyboard and mouse are the required acceptance path. Existing on-screen controls remain supported. Gamepad and complete remapping remain candidate follow-up work rather than hidden M24 scope.
- No public internet gateway, Steam deployment, account provider, password, credential collection, or participant NDA workflow.

## Research questions

1. Where does a first-time player hesitate, fail, or require explanation before completing the activity?
2. Which presentation and system confirmations are understood, and which are mistaken for local prediction or ignored?
3. After completion, does the player want another activity, more expressive combat, better controls, or no continuation?

The pilot should collect behavior before opinion: observed actions and timestamps first, a short interview second.

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
7. **Pilot execution:** run five to eight observed sessions without coaching, retain only consented bounded evidence, and separate observation from interpretation.
8. **Synthesis and decision:** rank findings by frequency, severity, confidence, and product impact; select exactly one primary direction for the next milestone and document rejected alternatives.

Every block requires separate review. A completed planning or instrumentation block authorizes neither participant recruitment nor later blocks, commit, push, PR, merge, version change, tag, release, or public exposure.

## Success signals

- At least six usable sessions, unless an earlier repeated blocker justifies stopping the pilot.
- At least 75% of participants connect and complete without procedural coaching; this is an initial decision threshold, not a quality guarantee.
- Every observed blocker is traceable to evidence and classified as comprehension, interaction, gameplay/content, reliability, or environment.
- Local reports contain only allow-listed fields and are disabled outside explicit playtest mode.
- Server-derived summaries reconcile with replay reconstruction for the same session.
- The synthesis selects one next milestone direction rather than producing an unbounded backlog.

## Stop conditions

- Stop the pilot if two participants encounter the same data-loss, reward-duplication, unrecoverable session, or privacy defect.
- Stop instrumentation work if it requires a Protocol V2 change; review that need separately.
- Stop public-distribution planning if secure identity, transport exposure, abuse limits, operations, and rollback have not been designed.
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

- **Observer bias:** use a fixed script, do not coach, and record actions before interpretations.
- **Small sample overconfidence:** treat the pilot as blocker discovery and direction selection, not population statistics.
- **Replay contamination:** derive summaries without adding UX events to the authoritative event vocabulary.
- **Privacy creep:** use an allow-list, local opt-in export, short retention, and explicit deletion.
- **Instrumentation authority leak:** client observations never confirm or mutate gameplay outcomes.
- **Telemetry platform expansion:** prefer existing persistence and Inspector boundaries; no external telemetry backend is required.
- **Premature polishing:** repair only defects that block or invalidate the pilot.
- **Public-network scope creep:** keep the gateway on trusted local infrastructure throughout M24.

## Research basis

- Godot supports runtime action mapping, controller abstraction, resolution scaling, and pseudolocalization; these are candidates for evidence-led follow-up rather than automatic M24 scope.
- WCAG 2.2 keyboard, focus, animation, and target-size criteria are useful review heuristics without claiming web conformance for the game client.
- OWASP recommends threat modeling before external exposure; M24 deliberately avoids turning local identity and TCP transport into a public service.
- OpenTelemetry Rust traces, metrics, and logs remain beta as of the planning date, so the existing replay/Inspector boundary is preferred for this bounded pilot.
- Steam Playtest is a future controlled-distribution option, but its store and operational setup is unnecessary for the first supervised pilot.

References:

- https://docs.godotengine.org/en/stable/classes/class_inputmap.html
- https://docs.godotengine.org/en/latest/tutorials/rendering/multiple_resolutions.html
- https://docs.godotengine.org/en/stable/tutorials/i18n/pseudolocalization.html
- https://www.w3.org/TR/WCAG22/
- https://owasp.org/www-project-threat-modeling/
- https://opentelemetry.io/docs/languages/rust/
- https://partner.steamgames.com/doc/features/playtest

## Definition of done

M24 is complete when the bounded evidence system is validated, the controlled pilot has usable consented sessions, findings are synthesized, one next direction is selected, all compatibility and release invariants pass, and the result is reviewed without silently starting the selected follow-up milestone.

## Block 1 checkpoint

The research brief, participant boundary, fixed research questions, initial decision thresholds, consent and retention baseline, stop conditions, moderator protocol, finding classification, and next-direction rule are recorded in this milestone and `docs/playtest/m24-research-protocol.md`.

`docs/playtest/m24-data-contract.md` defines the two evidence products before implementation. `AuthoritativeSessionSummary` is a read-only derivation of existing replay events; `LocalObservationReportV1` is a 16 KiB-bounded, local-only, explicitly activated JSON report containing allow-listed environment, first-occurrence, outcome, aggregate, and consent fields. It stores no username, host, IP, raw input history, wall-clock interaction timeline, arbitrary error text, or automatic upload target.

The contract fixes null and contradiction behavior, participant-code format, atomic local storage, 30-day maximum retention, early deletion, reconciliation, and the minimum fixtures later blocks must implement. It changes no code, schema, replay vocabulary, protocol, server authority, published version, tag, release, or participant state. Block 2 may implement only the authoritative derived summary after separate review.
