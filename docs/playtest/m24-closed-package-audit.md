# M24 Block 6 closed-package audit

Date: 2026-08-30  
Scope: unpublished Windows x86-64 package for same-host supervised use.  
Candidate: `m24-b6-45718926-871f1beafb43`.

## Contract and boundary

The package retains `VERSION=0.2.0`, Protocol V2, server authority, frozen V1,
existing replay vocabulary, and loopback-only infrastructure. It contains no
server, database, source checkout, credential, upload behavior, installer,
updater, tag, release, or public-network configuration. The participant client
is intentionally unsigned; Windows trust-policy handling remains an operator
decision and antivirus or SmartScreen must not be weakened globally.

The official Godot 4.7.1 Windows release template was accepted only after its
SHA-256 matched
`86409db6200b6f8fd3230989c2d2002851f3dd18acf11d7bdbafddf5a0dd0f72`.
The export uses a separate PCK. The package ID combines the frozen short HEAD
with the first 12 hexadecimal digits of the complete source-manifest digest so
uncommitted reviewed inputs cannot be mistaken for baseline HEAD alone.

## Defects found and corrected during dry-run

1. The first staging directory could not be renamed atomically on the Windows
   filesystem from WSL. Promotion now copies to a previously absent exact
   target and verifies the complete internal manifest again.
2. M22 audio read raw WAV bytes through `FileAccess`; Godot correctly exported
   imported audio resources instead of the source files, so the Windows PCK
   produced silent streams and the harness observed `decoded_bytes=0`. Audio
   now loads `AudioStreamWAV` resources, all 13 deterministic `.wav.import`
   sidecars select uncompressed PCM, and a clean import plus Windows export
   passes the original 741,120-byte M22 contract.
3. `Remove-RevenantReport.ps1` used `$matches`, colliding case-insensitively
   with PowerShell's regex `$Matches` hashtable. The bounded collection is now
   `$matchedFiles`; both `-WhatIf` and confirmed deletion pass on an isolated
   exact report target.
4. Final review separated moderator intervention and evidence disposition from
   the consent/observation sheets, making each operator decision auditable
   without adding personal data.

Each correction changed the source manifest and therefore invalidated the
prior package ID. Rejected IDs are listed in
`docs/playtest/m24-package-manifest.md`.

## Static and clean-extraction evidence

- Linux and Windows PowerShell independently matched the final archive digest
  `8cdf81e534b9c2e49472e6cc3cc2c1f4c315114abc22d875767934ed6d62d91e`.
- The archive was expanded by Windows PowerShell into the previously absent
  directory `clean-room-m24-b6-871f1beafb43`.
- The verifier inside that extraction matched all 14 internal files without
  accessing repository files.
- `Revenant.exe` is a PE32+ GUI executable for x86-64; `Revenant.pck` is
  separate and 963,352 bytes.
- The source manifest contains 173 inputs and still verifies against the
  reviewed tree.
- Archive-name scanning and direct inspection found no `.env`, credential,
  key, log, report, database dump, source checkout, `target`, `node_modules`,
  or release artifact.
- The launcher rejected retention `yes` when local observation consent was
  `no`, before starting the game.
- The deletion tool's `-WhatIf` preserved a synthetic report, showed the exact
  isolated AppData target, and the confirmed call removed only that file.

## Exported Windows client evidence

The final extracted `Revenant.exe` ran on Windows with isolated AppData and no
repository resources. Deterministic validation emitted every expected M17,
M18, M19, M20, M21, M22, M23, and M24 marker with no script, parse, or runtime
error. This includes M22 PCM audio and M24 display/first-contact plus local
observation boundaries.

The same exact package then completed `relay_awakening` through the loopback
gateway:

| Evidence | Value |
| --- | --- |
| Synthetic account | `local:m24-b6-win-871f1b` |
| Session | `session-1788110148370552260` |
| `activity_completed` | 1 |
| Relay core fragments | 1 |
| Experience | 100 |
| Activity history rows | 1 |
| Inventory grants | 1 |
| Progression grants | 1 |
| `loot_granted` replay | 1 |
| `progression_granted` replay | 1 |

Inspector reported `completed=true`, one participant, two enemy defeats, boss
spawned, one loot grant, and one progression grant. Observation and retention
were explicitly declined for this synthetic run, and isolated AppData contained
no `m24-*.json` report.

A preceding superseded package with the same corrected game code also ran two
fresh attempts for `local:m24-b6-win-aa49d1`. Distinct sessions
`session-1788109939227981066` and `session-1788110143871254969` produced exactly
two fragments, 200 XP, two history rows, and one of each grant/reward replay per
session. This supports the runbook statement that Retry starts over rather than
resuming or duplicating one session; it is supplementary, not the identity of
the final artifact.

## Operational recovery, reconciliation, and privacy

The packaged runbook requires checksum and loopback health gates before every
attempt, exact local-state reset, no coaching, bounded intervention, PostgreSQL
recovery without deleting its volume, replay/Inspector reconciliation, and
stop conditions for loss, duplication, contradiction, privacy defects, or an
unrecoverable session. Block 4 already exercised the same recovery procedure:
a real PostgreSQL outage aborted the defective session, and the next operation
renewed persistence without restarting the gateway; selective completion
failure rolled back all completion/reward state; gateway termination preserved
the first disconnected outcome and a fresh Retry rewarded exactly once.

Consent, local report collection, and retention remain explicit. Participant
codes contain no name/email mapping. Reports stay local, use an allow-list, are
never uploaded, and may be selectively removed by participant code/report ID.
Separate forms preserve observation, intervention, participant statement, and
evidence disposition without combining them into authoritative facts.

## Residual risk

- The executable is unsigned and may be blocked by local Windows policy.
- Clean extraction and execution were proven on the current Windows host, not
  on a second independently provisioned physical machine.
- Human first-contact comprehension, perceptual audio on representative
  hardware, and participant stop/deletion experience remain pilot evidence;
  automation cannot claim them.
- Protocol V2 does not send authoritative session ID into the local report, so
  supervised attempt-order correlation remains manual.
- Abrupt process termination can leave an explicitly incomplete local report;
  the operator must reconcile and remove it.
- There is no reconnect or resume. In-progress activity is lost, and Retry
  begins a fresh session.
- The package is a portable ZIP, not an installer, updater, public release, or
  general-distribution build.

## Final repository gate and decision

The uninterrupted `make check` passed version consistency, formatting, Clippy
with warnings denied, all workspace and PostgreSQL tests, build, Inspector,
secret audit, multiplayer, automatic, reusable-session, manual, and
keyboard-only Godot flows, every M17–M24 marker, authoritative replay and
summary reconciliation, frozen V1 compatibility, and V1 reconstruction.

The smoke was made independent of the worktree `.godot` cache: it copies the
Godot project without generated imports, performs an editor import in a
temporary directory, runs every Godot flow from that copy, and removes it on
exit. The clean import validated all 13 PCM sidecars and the M22 741,120-byte
audio contract without changing package identity or leaving generated files.

The final audit reconfirmed the exact ZIP digest and size, 14 internal hashes,
173 source hashes, Windows PowerShell verification from the clean extraction,
M21/M22/M24 capture hashes, `VERSION=0.2.0`, exact frozen V1 hashes, unchanged
Protocol V2/replay/V1 paths, zero temporary failure triggers or functions,
loopback-only healthy services, equal local/upstream HEAD, no release tag,
`git diff --check`, and the intentional dirty tree.

Gate C is **approved with bounded residual risk**. The residuals above remain
operating constraints for creator-only use. The owner subsequently removed the
sentinel and all external-participant work from the active roadmap. This package
is retained as a frozen engineering artifact; it authorizes neither another
person's session, recruitment, commit, push, merge, tag, release, nor public
exposure.
