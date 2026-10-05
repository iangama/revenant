# Revenant contributor rules

Revenant is a solo hobby game. Prioritize playable improvements and keep the
engineering process proportional to the change. The owner's 2026-09-09
instruction to reduce excessive verification supersedes earlier blanket
full-gate, repetition, and per-block evidence requirements in project documents.

## Working and validation policy

- At resumption, read `RESUMO.md` first when present. It is the owner's local,
  Git-ignored plain-text handoff and the default entry point for "revenant".
  In a fresh checkout without it, use the scope below and the closing ledger
  sections in `docs/roadmap-m33-m38-experience-expansion.md`. Read only the
  identified active files; consult the relevant roadmap section if the summary
  is missing, inconsistent or insufficient. Do not reread the full roadmap,
  rediscover the repository or repeat historical checks. Preserve existing work.
- Keep `RESUMO.md` under approximately 100 lines and update it after material
  progress and before ending a turn: current status, authorization, exact next
  step, modified files, checks/results, services/processes and effort forecast.
  On 2026-09-12 the owner requested a plain-text chat summary at **8% remaining
  in the five-hour usage window**. Run `python3 scripts/check-codex-quota.py`
  at resumption, between substantial batches and before long checks. When it
  reports `summary_due`, save the handoff and send its useful contents as text
  in the chat. This is sampled local telemetry, not a guaranteed background
  alert: if unavailable/stale, say so and keep the handoff current. Never infer
  the account allowance from context tokens, wall time or planning blocks.
- At each project resumption, report the estimated remaining 5-hour blocks to
  the end of the planned project (M38), including a range and central planning
  figure. Also state the portion needed for the currently authorized scope.
  Update the roadmap forecast from actual remaining work. These are effort
  equivalents, not a measured count of the owner's account usage-limit windows.
  This reporting preference was explicitly requested on 2026-09-10.
- Pick the smallest checks that establish the changed behavior. Reuse recent
  passing results for unchanged code; run a baseline only to resolve uncertainty
  or reproduce an issue, not as a compulsory step before every edit.
- Documentation/planning: review the edited text, links, and diff. No builds,
  game smoke, database drills, or new automated tests for text-only changes.
- Small presentation/content changes: check the affected screen or playable
  flow once, plus the relevant content validator when applicable. Do not add
  tests for cosmetic edits or repeat unrelated compatibility/recovery matrices.
- Logic changes and bug fixes: run affected tests and a short relevant flow;
  add a regression test when it protects meaningful behavior.
- Changes affecting saves, rewards, migrations, protocol, authentication, or
  recovery require focused integrity/failure coverage. Use `make check` once
  for cross-cutting changes or packaging when a recent pass does not cover the
  final implementation. It is not a mandatory command for every task or block.
- After checks pass, continue work or finish. Repeat only for changed behavior,
  an actual failure, or a concrete unresolved concern. No fixed test-count or
  repetition quota, new exhaustive matrix, or audit document for each small task.
- Keep one short ledger update for material scope/completion decisions. Existing
  tests and accepted evidence remain available; ordinary work needs only a
  concise record of what changed and how it was checked.

## Scope and boundaries

M0-M34 are complete. M30 closed on 2026-09-09 using the accepted Gates 1-5 and
the existing final quality result; the owner removed the additional 360-row
Gate 6 repetition. Those extra runs were not executed and must not be claimed.
M31 critical content mass is complete and applied locally. M32 accessibility,
operations and archival local build closed on 2026-09-11; its accepted private
Windows/Linux archive and focused checks are recorded in the roadmap.
Continue through authorized work; batch completion is not a stopping point.
Work in small playable batches with a short scope recorded in the roadmap;
a separate specification/audit stack is not required for each batch.
M33 Meridian closed on 2026-09-12 with both optional exploration activities,
accessible memory rereading, focused interruption/replay checks and the healthy
local gateway. The owner's 2026-09-12 "Pode continuar o trabalho" authorizes
M34 under its bounded scope in the roadmap. M34 closed on 2026-09-13 with
three archetypes, two elites and the optional two-phase Prism Warden, applied
locally with focused replay/rendered checks and make check. The owner's 2026-09-19 "Pode continuar o processo" authorizes M35: two weapons,
four fixed modules, at least three useful builds, loadout UX and authored reward
arcs under its roadmap scope. The owner's subsequent selection "1" (balanced
campaign/challenges) and "Blz, continue o projeto" on 2026-09-19 authorize
continued work through the expanded M36/M37 plan and M38 integration, in order
after M35. Keep the bounded scope in docs/roadmap-m33-m38-experience-expansion.md;
do not ask again merely on reaching the next milestone.
M35 closed locally on 2026-09-20: weapons/builds and the three one-time
commissions are applied, with focused integrity/replay/live checks. M36-A closed locally on 2026-09-22 with presentation fixes and normal rollout.
M36-B closed locally on 2026-09-23 with all six baseline chapters, the Prism
ending and preserved first-clear/reconnect behavior. M36-C closed locally on 2026-09-24 with persistent optional arcs, both recorded
approaches and epilogues, verified replay/save/reconnect flows and local rollout.
M36-D and M36 closed locally on 2026-09-25: continuation/practice/journal menus,
EN/PT presentation and one fresh-character six-chapter journey passed with all
optional records skipped and exactly six first-clear grants. M37 closed locally on 2026-10-03: eight contracts, bounded modifiers, six
masteries/three non-power titles, retained save integrity and local rollout passed.
M38 closed locally on 2026-10-03: retained-content journey/accessibility review,
focused presentation fixes, final check, disposable recovery and the private
Windows/Linux archive passed. M0–M38 have no remaining authorized milestone work;
resume from RESUMO for maintenance or a new owner-defined objective.
Execution authority does not include commit, push, merge, version change, tag,
release, public hosting, or involving another person.

On 2026-10-05 the owner's "Quero colocar no meu github" authorizes committing
the completed source/docs and pushing them to the existing public repository
`iangama/revenant`, retaining its history and version 0.2.0. This supersedes the
commit/push restriction for that source synchronization. Private archives,
database dumps, credentials and local continuation notes stay outside Git.
It does not authorize a new milestone, version, tag, binary release or hosting.

Keep server authority, persistence/reward integrity, honest client projection,
Protocol V2 and frozen V1 compatibility. Respect accepted gameplay contracts;
change only what the active scope calls for. Version 0.2.0 remains the baseline.
Use original or provenance-cleared assets/code. Keep replay separate from
optional local UX observations and make no outside-player preference claims.
Normal services remain loopback-only; identity/TLS labs stay default-disabled.
Protect working data and private secrets. Recovery drills use owned disposable
targets and exact cleanup; never restore over or remove the working volume.

The durable scope and continuation record is `docs/roadmap-2026-08-30.md`.
Preserve historical evidence as history; its superseded process requirements
must not silently become active obligations again.
