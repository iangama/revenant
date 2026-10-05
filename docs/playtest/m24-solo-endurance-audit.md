# M24 solo endurance and recovery audit

Date: 2026-08-30  
Scope: creator-only local validation; no participant or external system.  
Frozen package: `m24-b6-45718926-871f1beafb43`.

## Claim boundary

This audit can establish local correctness, endurance, recovery, package
integrity, and creator-operability. It cannot establish first-contact
comprehension, another person's accessibility, cooperative social behavior, or
broader enjoyment. No subjective creator note was supplied during automation,
so this record contains no preference claim.

The new endurance, recovery, protocol, report, settings, and Windows drivers
live under `tests/`. That path is outside the frozen package source manifest.
The 173 package inputs and archive SHA-256 remained unchanged throughout.

## Primary endurance campaign

The formal campaign token was `220119-55461`; raw local evidence is outside the
repository at
`/mnt/c/Users/Ian/revenant-local-evidence/m24-ds-220119-55461`.

| Evidence | Result |
| --- | --- |
| Solo sessions | 20 completed, 20 distinct sessions |
| Solo accounts | 10 fresh accounts plus one account reused 10 times |
| Two-client sessions | 10 completed, 20 distinct local bot accounts |
| Total joins | 40 |
| Total completions | 30 |
| Total loot/progression replay events | 40 / 40 |
| Total inventory/progression grants | 40 / 40 |
| Formal campaign wall time | 95 seconds |

Every solo session contained one join, one completion, one loot replay, one
progression replay, one equipment change, two enemy deaths, and one boss spawn.
Every two-client session contained two joins, one completion, two loot replay
events, two progression replay events, one equipment change, two enemy deaths,
and one boss spawn.

The reused solo account ended with exactly 10 activity-history rows, 10 relay
core fragments, 1,000 XP, 10 inventory grants, 10 progression grants, and 10
reward replay pairs. Fresh solo and multiplayer accounts each ended with
exactly one history row, one fragment, 100 XP, and one of each grant. No
per-account or per-session integrity query returned an invalid row.

### Resource boundary

| Process phase | RSS before/after | Descriptors | Threads | PostgreSQL connections |
| --- | --- | --- | --- | --- |
| 20 solo sessions | 6,144 / 6,840 KiB | 9 / 9 | 3 / 3 | 3 / 3 |
| 10 two-client sessions | 5,888 / 7,036 KiB | 9 / 9 | 3 / 3 | 3 / 3 |

RSS growth was 696 KiB for the solo gateway and 1,148 KiB for the multiplayer
gateway. Descriptors, threads, and observed PostgreSQL connections were
unchanged. This bounded campaign found no accumulating resource leak; it is not
an unlimited-duration soak claim.

## Failure and recovery campaign

The accepted recovery token was `r220712-58251`; evidence is at
`/mnt/c/Users/Ian/revenant-local-evidence/m24-ds-recovery-r220712-58251`.

- A real refused connection classified as `transport_failure`; transport
  classification took 1 ms after Godot startup.
- A real Protocol V99 hello returned `unsupported protocol version` and
  classified as `rejected`.
- A second local client entering an active one-player session classified as
  `session_unavailable` in 145 ms and created no replay event.
- Killing the exact holder process left incomplete session
  `session-1788127647289572221` with three prefix events and no completion or
  reward. Retry created `session-1788127649609177495` and granted exactly one
  fragment and 100 XP.
- The same account completed sessions `session-1788127653026776647` and
  `session-1788127656791687547` across a real local gateway restart, ending at
  exactly two fragments, 200 XP, and two history rows.
- PostgreSQL interruption left session `session-1788127659661893079`
  incomplete with four prefix events and no reward. PostgreSQL restarted with
  the same container and volume, and the local gateway PID remained `58660`.
  The next operation emitted `session_persistence_reconnected`; fresh session
  `session-1788127662130967693` completed with exactly one fragment, 100 XP,
  one history row, one inventory grant, one progression grant, and one reward
  replay pair.
- No temporary `m24_fail_%` trigger or function remained.

An initial recovery-driver attempt `r220613-57288` was rejected as harness
evidence: it killed the `timeout` wrapper rather than the child bot, so the
intended client-termination reset did not occur. The gateway and PostgreSQL
remained healthy and no reward was created. Direct child-PID ownership fixed
the driver before the accepted campaign.

## Local report boundary

The deterministic report probe produced a fully populated 714-byte
`LocalObservationReportV1` against its 16,384-byte ceiling, then removed the
exact synthetic file. The Windows abrupt-close case produced a 724-byte report
with `terminal_outcome=running`, proving that a kill cannot manufacture a
clean terminal state. The packaged deletion tool removed only that exact
synthetic report.

## Frozen Windows package

The accepted Windows evidence is
`/mnt/c/Users/Ian/revenant-local-evidence/m24-ds-windows-w2218-2`.

- Internal package hashes verified before and after execution.
- Clean Full/audible, Compact/muted, Off/Reduced Flash, and Full/muted/Reduced
  Flash cases each emitted all M17–M24 markers and the expected saved state.
- The exact packaged executable completed keyboard-only equipment, attack,
  movement, relay-door, Warden, reward, and completion against the loopback
  Compose gateway.
- Account `local:m24ds-w2218-2-win` completed session
  `session-1788110379717767612` with one fragment, 100 XP, one history row, and
  one of each idempotency grant. Inspector reported `completed=true`, one
  participant, two enemy defeats, boss spawned, one equipment change, and one
  reward pair.
- The Compose gateway container remained
  `b1cadaacdaa91cd3f90b7a04f19eadabe4f5fd783f7185c6334a2cf8c5451a90`.

The preceding Windows attempt `w2210-1` passed all four local display cases,
then reached the Compose gateway's connection that had been broken by the
immediately preceding PostgreSQL injection. In accordance with fail-closed
semantics, that first new operation failed, was not replayed automatically, and
created no replay or reward for `local:m24ds-w2210-1-win`. Without restarting
the gateway, the accepted attempt opened the next connection, emitted
`session_persistence_reconnected`, and completed exactly once. This is recovery
evidence rather than a Windows package defect.

## Gate status

No loss, duplicate reward, privacy escape, inconsistent projection, unbounded
resource growth, or unrecoverable coordinator state was found.

An earlier quality-gate invocation was interrupted by its execution shell
during smoke and was rejected as incomplete. The subsequent uninterrupted
`make check` passed version consistency, formatting, Clippy with warnings
denied, all workspace and PostgreSQL tests, build, Inspector, secret audit,
multiplayer, automatic, reusable-session, manual, and keyboard-only Godot
flows, every M17-M24 marker, authoritative replay and Inspector reconciliation,
frozen V1 compatibility, and V1 reconstruction.

The post-gate audit reconfirmed the 38,728,181-byte archive and its exact
SHA-256, all 14 internal hashes, all 173 source hashes, native Windows
PowerShell verification from the clean extraction, M21/M22/M24 capture
manifests, the two exact V1 hashes, unchanged Protocol V2/replay/V1 paths,
`VERSION=0.2.0`, equal local/upstream HEAD, no tag on HEAD, zero temporary
failure objects, loopback-only healthy services, no old validator, and
`git diff --check`.

Gate D-S is **approved with bounded residual risk**. The accepted residuals
are loss of an in-progress activity, manual report/replay correlation,
truthfully incomplete abrupt-exit reports, bounded rather than unlimited soak
coverage, an unsigned executable, and Windows validation on the current host.
Human comprehension, broader enjoyment, accessibility elsewhere, and social
cooperation remain explicitly untested rather than accepted as technical
proof.
