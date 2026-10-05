# M26 Block 6 Protocol V2 and gateway audit

Date: 2026-08-31  
Gate: 6 — additive Protocol V2, immutable admission, and frozen V1.  
Decision: **approved**.

## Reviewed contract and implemented surface

The exact pre-implementation decision remains
`docs/progression/m26-block6-protocol-contract.md`. Protocol version remains 2.
The accepted implementation adds only the four opt-in client variants and four
targeted server variants frozen there:

- state request/snapshot;
- server preview request/result;
- idempotent combination intent/result; and
- whole-loadout intent/result.

Operation identifiers are bounded to 32 bytes and then validated as 1-32 ASCII
alphanumeric/hyphen characters. Module identifiers are bounded to 32 bytes and
must parse to one of the four fixed catalog entries. Preview/loadout lists are
bounded to three unique entries before domain or persistence mutation. The
existing 64 KiB frame boundary still rejects oversized input before allocation.

No new response is sent during ordinary V2 admission. An older V2 client still
observes `WorldJoinResponse`, inventory, progression, equipment, then
`ActivityStart`. The existing equipment snapshot now carries server-resolved
effective profiles, so an unaware V2 client uses authoritative arithmetic
without understanding modules.

## Gateway authority and lifecycle

Persistence loads module state before admission. The gateway resolves both
weapon profiles and maximum health once, copies them into the participant, and
uses only that immutable projection for actor creation, attacks, and equipment
responses. A successful post-completion mutation updates the response-facing
persisted state but not the admitted actor, health, weapon profiles, or an
active cooldown deadline. The changed loadout becomes active only at the next
admission.

All module responses are targeted to the requesting current-V2 participant.
Preview accepts unowned fixed-catalog modules but never mutates. Combination and
loadout reject before persistence unless the session is complete. An accepted
mutation, accepted operation identity, and replay event share the Gate 5
transaction. A business rejection returns the unchanged authoritative snapshot
without disconnecting; an infrastructure/replay failure remains fail-closed.

Activity completion now updates the connection's fragment projection from the
same accepted reward result. This prevents a post-reward state request from
returning the admission-time balance while preserving the immutable combat
projection.

## Deterministic verification

Protocol tests round-trip all eight new variants. The maximum request uses a
32-byte operation identifier, revision `u64::MAX`, and three entries; the
maximum snapshot uses four catalog/ownership entries, three equipped entries,
two weapon profiles, and maximum integer quantities while remaining inside the
frame bound.

Gateway fixtures prove:

- exact base and Force+Ward projections for both weapons;
- frozen-V1 projection always returning base profiles and 100 health;
- targeted, non-mutating previews, including unowned candidates;
- empty rejected preview arithmetic for unknown, duplicate, and fourth-entry
  requests;
- active combination/loadout rejection with no persistence call;
- accepted and replayed combination/loadout behavior;
- current-session actor/profile immutability after accepted mutation;
- next-admission activation at 120 health and rifle 48/6/325;
- each multiplayer participant attacking with its own admitted profile; and
- reward fragment projection matching the committed multiplayer reward.

Domain and PostgreSQL suites continue to cover malformed identifiers,
insufficient fragments, already-owned modules, stale revision, unchanged
loadout, operation conflict, concurrent retry, ledger bounds, and rollback at
every write boundary with complete before/after state equality.

The accepted focused and workspace checks were:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=... cargo test --workspace
cargo build --workspace --all-targets
npm run check
npm run build
git diff --check
```

They passed 98 Rust/PostgreSQL tests, strict workspace Clippy, every target,
Inspector TypeScript, and the production Vite build. Protocol has nine unit
tests, compatibility three, and gateway 25.

## Accepted real-process evidence

The checksum-verifiable local record is:

`/mnt/c/Users/Ian/revenant-local-evidence/m26-block6-gateway-20260831`

It is 4,754 bytes. Its verified `SHA256SUMS` file has SHA-256
`7d92807cc61867bacaf0c1378b1a7b523eb0cd08b479ee1e0163b43b58973487`.
All accepted traffic used loopback-only temporary gateways; they were stopped
after capture and the canonical services stayed healthy.

The accepted mutation session is
`session-1788225143162438227`. The current client completed the activity,
observed five fragments, previewed Force+Ward as rifle 48/6/325 and 120 health,
and then exercised malformed/unknown/duplicate/fourth-entry, active-state,
insufficient, already-owned, stale, unchanged, and conflict rejection. Every
rejection kept the exact state and connection. Same-input retries returned
`replayed=true`. Exactly two combinations and one loadout event exist; the
final state is one fragment, two owned/equipped modules, and revision one.

Inspector reports 14 ordered events, one snapshot, two combinations, one
loadout change, one participant, and completion. The replay CLI independently
reports the same counts and reconstructs the final Force+Ward effective
profiles. The next session, `session-1788225166946893079`, admitted 120 health,
dealt rifle damage `[48,48,48]` to the drone and five 48-damage hits to the
Warden, and used the 325 ms server cooldown. The completed actor from the
mutation session remained at its base admitted projection.

An unchanged old-V2 client completed
`session-1788225174659692823` with the established sequence and zero
unsolicited module messages. A two-client session,
`session-1788225365859845513`, admitted one base participant at 100 health and
one Force+Ward participant at 120; the latter attacked at 48/325, both observed
the same completion, and rewards remained participant-specific.

## Frozen V1 proof

There is no diff under `archive/clients/v1`. The two enforced source hashes
remain exactly:

```text
4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72  src/main.rs
c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322  Cargo.toml
```

The archived client joined both the current gateway compatibility adapter and
the independently reconstructed V1 backend, which completed without the
gateway. A diagnostic V1 gameplay probe then used the same character that
persisted Force+Ward, revision one, and sidearm equipment. Session
`session-1788225183798245801` admitted it at 100 health with pulse rifle,
produced exact 40-damage drone/Warden sequences at 250 ms, and completed.
Afterward sidearm equipment, both module slots, revision one, two combination
operations, and one loadout operation were unchanged. The normal completion
reward alone increased its fragment balance once; no module was consumed or
rewritten.

## Rejected evidence and correction

1. The first account-bootstrap attempt used an observer in a solo session. It
   correctly waited for a driver and timed out; it created no reward and was
   not accepted as gameplay evidence.
2. The first mutation attempt completed gameplay but the bot rejected its own
   evidence because `ModuleSnapshot` still held the pre-reward fragment count.
   It sent no combination or loadout intent. The gateway was corrected to copy
   the committed reward total into the response-facing participant state, a
   multiplayer regression assertion was added, binaries were explicitly
   rebuilt, and the entire accepted flow was rerun.

The synthetic fixture quantity was restored to four between rejected and
accepted runs so the accepted activity deterministically exposed five
fragments and the insufficient-recipe boundary after two combinations. This
was evidence preparation on one named synthetic account, not product data or a
claim about organic progression.

## Gate decision

Gate 6 is **approved**. Additive V2 is bounded and opt-in; gateway arithmetic,
lifecycle, mutation, replay, and participant isolation remain server-owned;
old V2 remains sequence-compatible; frozen V1 remains byte-identical and base;
and no active-session mutation or partial projection was observed. Block 7 may
begin. It is limited to the honest Godot workshop/presentation contract and
may not broaden module taxonomy, protocol, persistence, or replay.
