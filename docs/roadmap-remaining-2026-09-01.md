# Remaining roadmap and focused-effort forecast — updated 2026-09-08

This is a historical effort forecast. For current status and process, use
`docs/roadmap-2026-08-30.md` and `AGENTS.md`: M30 is complete, M31 is next, and
mandatory full-gate repetitions/per-batch paperwork were removed on 2026-09-09.
The older verification estimates below are not new work requirements.

## Planning basis

This forecast starts from the actual local state on 2026-09-08:

- M0-M28 are complete and green. M28 closed with exactly two immutable local
  cooperation roles, one ping/down/revive sequence, additive persistence and
  replay, additive Protocol V2, honest Godot projection, bounded role/runtime
  matrices, and the uninterrupted 217-test quality gate.
- M29 is complete by evidence-based removal after its pure value laboratory
  failed the frozen keep threshold. M30 Gates 1-3 are green: the finite
  local-only threat/recovery contract, default-disabled pure identity/session
  laboratory, and transport/secrets hardening are retained; Gate 4 is the
  active boundary.
  M31-M32 remain sequential and require their own finite Gate 1
  specifications, so their estimates have progressively lower confidence.
- At the owner's explicit request, the read-only M30 Gate 1 audit was paused for
  one bounded Relay Hub final-video presentation interlude. The interlude is
  complete under `docs/art/m28/relay-hub-final-video-audit.md`; it does not mark
  M30 green, begin authoritative M31 content, or change the estimates below.
- M33-M38 are now numbered future experience milestones under
  `docs/roadmap-m33-m38-experience-expansion.md`. They are deliberately excluded
  from the active M30-M32 forecast and remain locked behind green M32 and a new
  explicit owner instruction.

Hours below are focused implementation, review, validation, evidence, and
documentation hours in the current local environment. They are not CPU hours
or calendar promises. The low bound assumes the present architecture holds and
no gate finds a redesign. The high bound allows one bounded remediation cycle
at each integration-heavy gate, but not a Protocol V3, public service, external
playtest, major asset outsourcing, or platform expansion.

## Executive forecast

| Delivery boundary | Remaining hours | Planning target | Cumulative range |
| --- | ---: | ---: | ---: |
| M28 local cooperation simulation (complete) | 0 | 0 | 0 |
| M29 Echo decision/removal (complete) | 0 | 0 | 0 |
| Finish M30 local security/operations laboratory | 12-21 | 18 | 12-21 |
| Finish one bounded M31 content-mass tranche | 32-72 | 50 | 44-93 |
| Finish M32 accessibility/archival local build | 22-38 | 30 | 66-131 |

The working planning target is approximately **98 focused hours**. Reserve
about 15% for cross-milestone integration/regression recovery, producing a
practical planning budget of **about 113 hours**. The honest total range is
**66-131 focused hours** before contingency.

Illustrative calendar conversion:

| Sustainable pace | Planning-target duration | With 15% reserve |
| --- | ---: | ---: |
| 10 focused hours/week | about 10 weeks | about 11 weeks |
| 20 focused hours/week | about 5 weeks | about 6 weeks |
| 30 focused hours/week | about 4 weeks | about 4 weeks |

These durations are sequential because every milestone is gated. They must not
be compressed by implementing a later milestone before the preceding green
closure.

## M27 — Routes, risks, events, and replayable operations

Completed green on 2026-09-06. The final decision and residuals are recorded
in `docs/operations/m27-closure-audit.md`; M27 contributes zero remaining hours
to this forecast.

## M28 — Local cooperation simulation

Planning objective: prove a strictly local, server-authoritative cooperation
slice with two real local clients and deterministic role bots. No human
cooperative-behavior claim is allowed.

Current status: complete. Gates 1-8 and Gate M28 are green. The exact two-role, one-objective/ping/down/
revive, timing, reward, compatibility, evidence, and stop bounds are frozen in
`docs/milestones/M28-local-cooperation-simulation.md`. The required external
M27 closure checkpoint is verified. The pure domain, 900-row matrix, and
50-vector lifecycle report are green under
`docs/operations/m28-block2-domain-audit.md`; additive persistence,
backup/restore, apply-twice, idempotency/concurrency, rollback, and working
database evidence are green under
`docs/operations/m28-block3-persistence-audit.md`. The exact six-event replay,
atomic coupling, independent reconstruction, corruption matrix, and GET-only
Inspector evidence are green under
`docs/operations/m28-block4-replay-audit.md`. The additive current-V2 Protocol
and gateway composition is green under
`docs/operations/m28-block5-protocol-audit.md`. The bounded role-bot matrix is
green under `docs/operations/m28-block6-bot-audit.md`. Honest Godot cooperation
presentation and live creator evidence are green under
`docs/operations/m28-block7-godot-audit.md`. The finite runtime, recovery, and
resource matrix is green under `docs/operations/m28-block8-runtime-audit.md`;
the final decision and residuals are in
`docs/operations/m28-closure-audit.md`.

| Block | Deliverable | Hours |
| --- | --- | ---: |
| Specification | Complete at Gate 1 | 0 |
| Pure domain | Complete at Gate 2 | 0 |
| Persistence/replay | Complete at Gates 3-4 | 0 |
| Protocol/gateway | Complete at Gate 5 | 0 |
| Bots/client | Complete at Gates 6-7 | 0 |
| Runtime matrix | Complete at Gate 8 | 0 |
| Closure | Complete at Gate M28 | 0 |
| **M28 remaining** |  | **0** |

Do not add matchmaking, public lobbies, voice/chat, or human anti-griefing
claims. Those would materially expand scope.

## M29 — Isolated Echo prototype

Planning objective: build a removable prototype and make an evidence-based
keep/remove decision. Removal is a valid successful outcome.

Current status: complete by evidence-based removal under
`docs/operations/m29-block2-echo-lab-audit.md`. The exact 2,790-row value
matrix and 50-case lifecycle matrix were reproduced twice. Lifecycle and
baseline invariants passed, but only 1,560 of 1,805 successful executions
removed exactly one direct attack and zero of 900 two-player pairs exposed the
required early/held non-dominated tradeoff. No integration gate opened; the
temporary pure implementation was deleted and exact M28 restoration plus the
uninterrupted 217-test gate passed.

| Block | Deliverable | Hours |
| --- | --- | ---: |
| Specification/removal gate | Complete at Gate 1 | 0 |
| Pure prototype | Complete; threshold failed at Gate 2 | 0 |
| Integration | Correctly not opened | 0 |
| Presentation/dogfood | Correctly not opened | 0 |
| Decision/cleanup | Complete removal and M28 restoration proven | 0 |
| **M29 remaining** |  | **0** |

The estimate assumes the removal criterion is honored. Expanding a weak
prototype to justify sunk effort is explicitly outside the plan.

## M30 — Local identity, lobby, security, and recovery laboratory

Planning objective: harden local operation and prove recovery/adversarial
behavior without exposing a public service.

Current status: Gates 1-3 are green under
`docs/security/m30-gate1-threat-model-audit.md` and
`docs/security/m30-gate2-identity-session-audit.md`, with transport/secrets
accepted under `docs/security/m30-gate3-transport-secrets-audit.md`. The exact
30-threat, 120-vector, local-only contract and no-Protocol-V3 decision are
frozen in `docs/milestones/M30-local-security-recovery-laboratory.md`. Only
Gate 4 observability/abuse is active.

| Block | Deliverable | Hours |
| --- | --- | ---: |
| Threat model/specification | Complete at Gate 1: exact assets/zones/threats, privacy/resource/recovery limits, 120-vector matrix, explicit no-Protocol-V3 boundary | 0 |
| Identity/session laboratory | Complete at Gate 2: credential/session lifecycle, local lobby/invite/presence prototype, authentication-rate boundaries and negative tests | 0 |
| Transport/secrets | Complete at Gate 3: loopback topology, separated file secrets/roles, rotation/rollback, isolated TLS and compatibility matrix | 0 |
| Observability/abuse | Structured bounded logs/metrics, correlation, redaction, abuse/fault injection and local alerts | 4-7 |
| Backup/recovery | Automated backup, restore, corruption/crash recovery, migration rollback and recovery-time evidence | 5-8 |
| Closure | Adversarial matrix, documentation/runbooks, complete gate and Gate M30 | 3-6 |
| **M30 remaining** |  | **12-21** |

Public hosting, production certificate operations, account-email recovery,
support staffing, and Internet adversaries are outside this estimate.

## M31 — Bounded critical-content-mass tranche

The strategic M31 sketch is open-ended and therefore has no honest finite
total by itself. For planning, use one explicit bounded tranche. Gate 1 must
confirm or reduce this assumption before implementation:

- two new activities using the validated M27 authoring path;
- two enemy variants and one boss variant;
- one weapon and two fixed modules without new rarity/tier systems;
- four bounded activity events;
- environment/presentation reuse with only original repository-owned assets;
  and
- complete replay, compatibility, performance, audio, localization-key, and
  provenance coverage for that tranche.

| Block | Deliverable | Hours |
| --- | --- | ---: |
| Specification/content budget | Exact catalog, dependency graph, acceptance metrics, provenance and removal/cut criteria | 3-5 |
| Authoritative content/domain | Activities, objectives, enemies/boss, weapon/modules/events and validators | 10-20 |
| Persistence/replay/protocol integration | Only additions required by the frozen content catalog, with compatibility evidence | 5-10 |
| Godot presentation/assets | Original environment/combat/UI/audio presentation and accessibility hooks | 8-18 |
| Generated/runtime matrices | Full content validation, seeded bots, resource/performance/replay evidence | 4-9 |
| Closure | Creator dogfood synthesis, cut decisions, complete gate and Gate M31 | 2-10 |
| **One M31 tranche** |  | **32-72** |

Additional content is not included. If Gate 1 chooses a larger catalog, reprice
M31 before work begins; do not hide open-ended production inside contingency.

## M32 — Accessibility, operations, and archival local build

Planning objective: finish creator-verifiable accessibility/operations work and
produce a checksum-verifiable local archival build. This is not public or
limited distribution authorization.

| Block | Deliverable | Hours |
| --- | --- | ---: |
| Specification/matrix | Exact settings, input devices, scale/contrast/non-color/caption/localization and operations acceptance matrix | 2-3 |
| Input/presentation accessibility | Remapping, gamepad/keyboard navigation, UI scale, contrast, motion/flash, captions and audio controls | 6-10 |
| Localization | String extraction, pseudolocalization, layout bounds and deterministic fixtures | 3-5 |
| Operations/recovery | Install/update/crash/save/backup/restore/privacy/incident/rollback validation | 4-7 |
| Archival packaging | Reproducible local package, provenance, manifests, checksum and restore verification | 3-5 |
| Final closure | Complete cross-milestone regression, residuals, archival audit and Gate M32 | 4-8 |
| **M32 total** |  | **22-38** |

Independent accessibility conformance, multiple bodies/assistive technologies,
external hardware coverage, store deployment, and public release remain outside
the evidence and estimate.

## M33-M38 — Numbered future experience milestones

The roadmap continues after the M32 archival checkpoint with six explicit
experience milestones:

| Milestone | Focus | Indicative focused hours |
| --- | --- | ---: |
| M33 | World expansion and exploration identity | 28-52 |
| M34 | Enemy ecology and boss encounters | 28-56 |
| M35 | Arsenal, build identity, and rewarding progression | 24-48 |
| M36 | Cohesive solo campaign and narrative payoff | 30-60 |
| M37 | Replayability, challenge, and mastery | 22-44 |
| M38 | Experiential polish and definitive local edition | 20-40 |
| **Total** | **Planned experience expansion** | **152-300** |

These estimates are not part of the current 66-131-hour M30-M32 delivery
forecast. Exact catalogs and acceptance gates must be frozen one milestone at a
time. Full scope and boundaries are in
`docs/roadmap-m33-m38-experience-expansion.md`.

## Critical path and replanning triggers

The critical path is:

`M27 replayable operation → M28 cooperation → M29 Echo decision → M30 hardening → M31 bounded content → M32 archival build`

The planned, separately authorized continuation after that checkpoint is:

`M33 world → M34 encounters → M35 builds → M36 campaign → M37 mastery → M38 definitive local edition`

Re-estimate before continuing if any gate requires:

- Protocol V3 or breaking old-V2/frozen-V1 behavior;
- destructive schema migration or data repair in the working database;
- public networking, external accounts/services, or another person;
- a third M27 route, larger M26/M27 catalogs, or an expanded M31 tranche;
- a new rendering/audio/asset pipeline rather than current-system reuse;
- platform support beyond the current local environment; or
- accessibility or preference claims that require independent human evidence.

## Immediate next actions

1. Create and verify the Gate 3 pre-change custom-format database backup and a
   disposable restore without mutating the working volume.
2. Freeze the exact implementation and rollback details for `T01`–`T30`,
   preserving loopback-only operation, PostgreSQL 16, Protocol V2/V1, and all
   normal client compatibility boundaries.
3. Execute only the bounded transport/secrets matrix: secret provisioning and
   rotation, least-privilege mounts, loopback binds, optional isolated TLS,
   version/failure behavior, compatibility, and complete rollback evidence.
