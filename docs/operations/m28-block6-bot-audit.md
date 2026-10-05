# M28 Block 6 — deterministic role-bot audit

Date: 2026-09-07  
Decision: **Gate 6 green**

## Accepted boundary

Block 6 adds one private current-V2 engineering client,
`revenant-cooperation-bot`, and the bounded `tests/m28-bot-matrix.sh` runner.
It does not add a public client, matchmaking, human-behavior simulation,
Godot presentation, protocol vocabulary, server outcome authority, a release,
or a version change.

The paired mode opens exactly two separate current-V2 connections. Admission
order deterministically fixes the first as anchor and the second as runner;
the clients never send a role, elapsed time, health result, reward, terminal
outcome, or peer decision. Every advancement waits for authoritative protocol
truth and every final result is copied from `CooperationOperationSummary`.

A separate runner-only mode is available for the next local Godot block. It
only opts in, waits for an authoritative ping, moves to the fixed relay
console, validates its server-owned downed state, and consumes the terminal
summary. It cannot start, ping, revive, complete, or choose an outcome.

## Positive and negative coverage

The success scenario performs the complete required contribution split and
also probes invalid or conflicting input without granting it authority:

- invalid start ID and non-leader start rejection;
- accepted start, same-input retry, and conflicting-start rejection;
- runner movement before ping and anchor ping before arrival rejection;
- wrong-role ping rejection, accepted ping, same-ID later retry, and
  conflicting ping rejection;
- exact feedback downing, wrong-role revive rejection, pending same revive,
  out-of-range cancellation, replacement channel completion, and rejection of
  a second revive;
- route exclusion while cooperation is active; and
- Warden completion followed by the server summary and two ordered grants.

The negative set contains real ping, revive, and whole-operation timeouts plus
both immutable roles disconnecting in every nonterminal phase:
`awaiting_anchor`, `awaiting_ping`, `awaiting_runner`, `runner_downed`,
`revive_channel`, and `encounter_active`. This is 15 reward-free terminal
cases in addition to the one success case.

The first exploratory success exposed that a later same-ID ping retry rebuilt
its replay payload with the retry observation time. Gate 5 was reopened; the
retry now reconstructs the durable accepted time, its PostgreSQL regression
passes, and the full Gate 5 quality suite was rerun before this Gate 6
decision. The bot therefore found and helped close a server defect rather
than accommodating it.

## Reproducible runtime evidence

The accepted isolated gateway listened only on `127.0.0.1:18086` and
`127.0.0.1:17006`, expected exactly two players, and was stopped by the matrix
trap. `tests/m28-bot-matrix.sh` completed 16 sessions and observed 16 resets.
The accepted sessions are recorded in
`/mnt/c/Users/Ian/revenant-local-evidence/m28-block6-bots-0907g6c`.

The success session `session-1788778420072959164` completed at 6,343 ms with
anchor arrival, one ping, runner arrival/downing, one completed revive, Warden
completion, two activity-history rows, two fragment grants of quantity two,
and two 125-XP grants. Each timeout had its exact typed outcome and zero
grants. All 12 disconnect cases had `abandoned_disconnect`, the exact anchor
or runner subject role, and zero grants.

For every session the bot summary, GET-only Inspector summary/events, and SQL
agreed on one operation, `anchor,runner` admission order, zero route rows, one
start, one terminal event, and a maximum cardinality of one for every accepted
cooperation replay kind. Across the matrix there were 146 matching SQL
assertions, no gateway command/abort failure, zero retained public trigger,
and zero retained public function.

The evidence `SHA256SUMS` verifies cleanly and has SHA-256
`5acbb4351b28d3a8e86f2c694445ddd219f6b3cf1a9ccbc64a70b56d35d79a2b`.
Two earlier collector-only attempts remain visibly separate and are not
accepted evidence: one read the Inspector wrapper at the wrong JSON level and
one queried activity history by a nonexistent session column. Neither was a
runtime or persistence failure.

## Compatibility and quality

The final uninterrupted `make check` included the bot in workspace format,
warnings-denied Clippy, tests, and all-target builds. It passed 217 tests,
Inspector check/build, secret audit, multiplayer smoke, the pre-M28 Godot
surface, current and frozen V1 compatibility, and V1 reconstruction. Focused
compatibility tests still reject cooperation requests from frozen V1; the
ordinary V2 baseline remains silent; gateway fault tests continue to withhold
optimistic truth. `git diff --check` is green and `VERSION` remains `0.2.0`.

The complete Gate 5 revalidation log and evidence hashes are recorded in
`docs/operations/m28-block5-protocol-audit.md`. No bot process or isolated
listener remains.

## Gate decision

Gate 6 is green. The deterministic clients prove distinct required
contributions, invalid-input rejection, durable retry, exact terminal truth,
timeout/disconnect coverage, rewards, route exclusion, and reset without
client-owned outcome or a human-behavior claim.

Block 7 may now review and implement honest Godot cooperation presentation.
Automated playback and creator captures may prove correctness and
renderability, but not outside comprehension, preference, teamwork quality,
or enjoyment.
