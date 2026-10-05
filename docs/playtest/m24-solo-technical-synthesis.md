# M24 solo technical synthesis

Date: 2026-08-30  
Scope: creator-only local engineering evidence.  
Decision: close M24 as **solo technical readiness** and propose M25 combat
pleasure and fairness as the next direction.

## Claim boundary

M24 establishes a reproducible local slice, authoritative persistence and
recovery behavior, bounded endurance on the current machine, and an exact
checksum-verifiable Windows package. It is not a completed human research
study. No other person played the package, and no evidence here establishes
first-contact comprehension, population preference, social cooperation, or
accessibility for other bodies and hardware.

No subjective creator note was supplied during the endurance campaign. The
next-direction decision is therefore an engineering inference, not a creator
preference or a claim about what players enjoy.

## Consolidated evidence

### Deterministic facts

- Seventeen gateway fixtures and five PostgreSQL integration tests cover
  persistence-before-projection ordering, multiplayer completion atomicity,
  idempotent reward grants, equipment/replay atomicity, fatal persistence
  abort, ordinary command rejection, and next-operation connection renewal.
- Selective failure inside `progression_granted` rolled back completion,
  rewards, history, grants, and reward replay for the whole activity. A fresh
  Retry completed exactly once without restarting the gateway.
- A real gateway termination preserved the first terminal client outcome as
  `disconnected`. Retry used a distinct session and did not duplicate reward.
- The D-S campaign completed 20 sequential solo sessions and 10 two-client
  sessions: 30 distinct completions, 40 joins, 40 loot replay events, 40
  progression replay events, and 40 of each idempotency grant. Every exact
  per-session and per-account assertion passed.
- Refusal, incompatible handshake, occupied admission, client termination,
  gateway restart, and PostgreSQL interruption followed their structured,
  fail-closed paths. Sampled incomplete sessions contained no partial reward.
- The exact Windows package completed the keyboard-only authoritative flow and
  reconciled one reward through replay and Inspector.
- The frozen ZIP still has SHA-256
  `8cdf81e534b9c2e49472e6cc3cc2c1f4c315114abc22d875767934ed6d62d91e`.
  All 14 internal files and all 173 source inputs verify. The added D-S drivers
  live outside that package identity.
- Protocol V2, replay vocabulary, frozen V1 files, `VERSION=0.2.0`, baseline
  HEAD, and loopback-only bindings remain unchanged. No temporary failure
  trigger or function remains.
- The final uninterrupted `make check` passed version, formatting, Clippy with
  warnings denied, workspace and PostgreSQL tests, build, Inspector, secret
  audit, all Godot paths and M17-M24 markers, replay/summary reconciliation,
  V1 compatibility, and V1 reconstruction.

### Measured local results

- The formal endurance campaign took 95 seconds on the current WSL/Windows
  host.
- Across 20 solo sessions, gateway RSS changed from 6,144 to 6,840 KiB; file
  descriptors, threads, and observed PostgreSQL connections remained 9, 3,
  and 3.
- Across 10 two-client sessions, gateway RSS changed from 5,888 to 7,036 KiB;
  descriptors, threads, and database connections again remained unchanged.
- Immediate refusal classified after 1 ms of Godot runtime; occupied admission
  classified in 145 ms.
- A populated synthetic local report was 714 bytes against the 16,384-byte
  ceiling. The abrupt Windows report was 724 bytes and truthfully remained
  `running`.
- Four clean Windows settings/display cases and one keyboard-only network flow
  passed from the exact extracted package on the current host.

These measurements bound the exercised window only. They do not prove an
unlimited-duration soak, another Windows installation, or another hardware
profile.

### Creator preference

None recorded. Automation and deterministic dogfooding supplied no statement
from the owner about enjoyment, feel, clarity, or desired content.

### Engineering inference

The server-authoritative vertical slice is now stable enough that the highest
bounded leverage comes from measuring and tuning its primary repeated
interaction: combat. Reliability work no longer exposes a reproducible
integrity blocker, while the current evidence says little about hit feel,
weapon contrast, encounter pacing, incoming-pressure fairness, or whether
combat remains interesting across repeated creator runs.

This supports proposing **M25 — combat pleasure and fairness**. It does not
establish that combat is objectively unfun or that another person would prefer
this direction.

### Explicitly unanswered human questions

- Whether an unfamiliar person understands entry, movement, equipment,
  objectives, the relay door, the Warden, rewards, failure, and Retry.
- Whether another person enjoys the combat, progression, pacing, presentation,
  or overall premise.
- Accessibility and perceptual behavior for other bodies, assistive
  technologies, displays, audio hardware, input devices, and operating-system
  policies.
- Human cooperative communication, role formation, social friction,
  abandonment, and anti-griefing behavior.
- General Windows portability beyond the current host and response to an
  unsigned executable under other trust policies.

## Ranked engineering findings

| Rank | Finding | Reproducibility | Severity | Confidence | Continued-development impact |
| --- | --- | --- | --- | --- | --- |
| 1 | No integrity, duplication, projection, or coordinator blocker in the bounded matrix | High | None observed | High for tested paths | Clears the current slice for a focused next milestone |
| 2 | Combat quality is not measured beyond correctness and deterministic completion | High | Directional gap | High | Highest leverage for the next bounded solo laboratory |
| 3 | Outage or process loss discards in-progress activity; Retry starts over | High | Medium during a failure | High | Acceptable locally; keep visible in every later package/runbook |
| 4 | Protocol V2 does not project session ID into the local report | High | Low | High | Correlation remains manual; no protocol change is justified by M24 alone |
| 5 | Abrupt termination leaves a truthful incomplete local report requiring cleanup | High | Low | High | Preserve selective deletion and reconciliation procedure |
| 6 | Resource evidence is bounded rather than a long soak | High for the 95-second window | Low residual | High | Repeat longer only if later content or load materially changes the runtime |
| 7 | Windows evidence is from one host and an unsigned executable | High on that host | Environment-dependent | High | Remains an archival/local constraint, not a public-distribution claim |

## Direction decision

Select exactly one next solo direction: **combat**.

- **Progression is deferred** because reward correctness and accumulation are
  already stable; deeper progression would multiply balance variables before
  the moment-to-moment combat baseline is measured.
- **Operations are deferred** because additional routes, objectives, and
  events would amplify the current combat loop without first establishing its
  pacing and fairness budgets.
- **Cooperation simulation is deferred** because bots prove atomicity and
  projection, not social quality. Revive, ping, and role mechanics should build
  on a sound solo combat baseline.
- **Core revision is deferred** because M24 found no technical evidence that
  justifies discarding the authoritative architecture or vertical slice.

The proposed M25 remains separately gated. Its first deliverable should be a
local combat laboratory and explicit hypotheses, not immediate content
expansion or broad tuning by intuition.

## Closure

Gate D-S is approved with bounded residual risk. Phase E-S is complete. M24 is
closed as **solo technical readiness** with the exact Gate C package retained
as an unpublished engineering artifact.

This closure creates no commit, push, merge, version change, tag, release,
public exposure, recruitment, or authorization to begin M25.
