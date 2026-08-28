# M24 playtest evidence data contract

Status: Block 1 contract; authoritative summary implemented in Block 2 and local report implemented in Block 3.

## Boundary decision

M24 has two evidence products with different trust levels:

1. `AuthoritativeSessionSummary` is a read-only projection derived from persisted replay events. It may describe only server-confirmed facts.
2. `LocalObservationReportV1` is an optional file created by the Godot client during a supervised playtest. It describes local presentation and attempts, is never authoritative, and is never uploaded automatically.

Neither product adds a replay event, changes Protocol V2, or grants gameplay state. Correlation uses the existing random session identifier when one was successfully received; participant identity is never inferred from an account or username.

## Authoritative session summary

### Shape

| Field | Type | Derivation |
| --- | --- | --- |
| `session_id` | string | Existing replay session ID |
| `activity_id` | string or null | Confirmed replay activity ID |
| `first_joined_at` | timestamp | Earliest `player_joined` |
| `activity_started_at` | timestamp or null | Earliest `activity_started` |
| `activity_ended_at` | timestamp | `activity_completed` when present, otherwise latest event |
| `join_to_start_ms` | non-negative integer or null | Start minus first join |
| `activity_duration_ms` | non-negative integer or null | End minus activity start |
| `participant_count` | non-negative integer | Distinct persisted accounts in the session |
| `completed` | boolean | Presence of `activity_completed` |
| `enemy_spawn_count` | non-negative integer | Count of `enemy_spawned` |
| `enemy_defeat_count` | non-negative integer | Count of `enemy_died` |
| `boss_spawned` | boolean | Presence of `boss_spawned` |
| `equipment_change_count` | non-negative integer | Count of accepted `equipment_changed` events |
| `loot_grant_count` | non-negative integer | Count of `loot_granted` |
| `progression_grant_count` | non-negative integer | Count of `progression_granted` |
| `event_count` | non-negative integer | Number of replay records in the session |

### Invariants

- Event order remains replay record ID order; timestamps are used only for elapsed-time projections.
- A missing start or completion produces `null` duration rather than a manufactured zero.
- Negative elapsed values caused by malformed evidence are rejected, not clamped.
- Counts are derived from typed known event kinds. Unknown vocabulary remains a reconstruction error under the existing replay contract.
- The summary query is read-only and bounded to one canonical session or the existing 1–100 session listing limit.
- The Inspector may display this projection but cannot edit, annotate, delete, or reclassify replay data.

## Local observation report V1

### Activation

- Disabled by default.
- Enabled only when `REVENANT_PLAYTEST_MODE=1` is present at launch.
- Requires a valid participant code supplied through `REVENANT_PLAYTEST_PARTICIPANT`.
- Requires an operator-supplied package identifier through `REVENANT_PLAYTEST_BUILD_ID`.
- Requires explicit observation consent through `REVENANT_PLAYTEST_OBSERVATION_CONSENT=1`; retention is independently confirmed through `REVENANT_PLAYTEST_RETENTION_CONSENT=1`.
- A valid code matches `PT-[A-Z0-9]{4}` and is assigned independently of name, email, username, or schedule record.
- Missing or malformed activation values disable collection and expose an explicit local diagnostic; they must not block normal play.

### Storage

- Directory: `user://playtest/`.
- One UTF-8 JSON file per attempted session.
- Filename: `m24-<report_id>.json`, where `report_id` is a random 128-bit lowercase hexadecimal value.
- Maximum encoded size: 16 KiB.
- Write through a temporary sibling followed by atomic rename where supported.
- No automatic upload, HTTP request, game-protocol message, clipboard copy, or background synchronization.

### Shape

```json
{
  "schema_version": 1,
  "report_id": "128-bit-lowercase-hex",
  "participant_code": "PT-A1B2",
  "product_version": "0.2.0",
  "build_id": "operator-supplied-package-id",
  "session_id": "optional-authoritative-session-id",
  "environment": {
    "os_family": "windows|linux|macos|other",
    "viewport_width": 1280,
    "viewport_height": 720,
    "display_mode": "Windowed|Fullscreen"
  },
  "preferences": {
    "guidance_mode": "Full|Compact|Off",
    "muted": false,
    "reduced_flash": false
  },
  "observed_at_ms": {
    "entry_ready": 0,
    "connect_requested": null,
    "connect_outcome": null,
    "first_movement_attempt": null,
    "first_attack_attempt": null,
    "settings_opened": null,
    "completion_observed": null,
    "disconnect_observed": null,
    "quit_requested": null
  },
  "connection_outcome": "not_attempted|connected|rejected|timeout|transport_failure|session_unavailable",
  "terminal_outcome": "running|completed|failed|disconnected|quit",
  "cooldown_acknowledgement_count": 0,
  "consent": {
    "observation": true,
    "retention": true
  }
}
```

### Field rules

- `build_id` is an operator-supplied package identifier, not a value obtained by executing Git on the participant machine.
- `observed_at_ms` values use monotonic milliseconds relative to `entry_ready`; wall-clock timestamps are not stored.
- Event fields are first occurrence only. Repeated raw inputs are never recorded.
- `cooldown_acknowledgement_count` is an aggregate bounded to `0..65535`.
- `session_id` is absent until confirmed by the server and is used only to reconcile the two evidence products.
- Error outcomes use the allow-listed enum. Raw exception, host, port, username, payload, or log text is prohibited.
- Preference values record the active accessibility context needed to interpret observation; volume levels and unrelated settings are omitted.
- Closing before consent or retention confirmation deletes the temporary report.

## Consent, retention, and deletion

- The moderator explains both evidence products before enabling playtest mode.
- `observation=false` means the client collector remains inactive.
- `retention=false` permits immediate session review but requires deletion before closeout ends.
- Raw local reports and moderator sheets have a maximum retention of 30 days after Block 8 synthesis.
- A participant may request earlier deletion using only the participant code and report ID.
- The operator keeps any scheduling-to-code mapping outside the repository and deletes it after session reconciliation.
- Synthesized project findings contain counts, classifications, and non-identifying behavior only.

## Reconciliation

Reconciliation is explicit and read-only:

1. validate the local schema and allow-list;
2. when `session_id` exists, load the corresponding authoritative summary;
3. verify product/build context and that client completion observation does not contradict server completion;
4. retain contradictions as reliability findings rather than rewriting either source;
5. remove the participant-code mapping after the session is classified.

The local report may be absent, incomplete, or wrong. The replay-derived summary remains authoritative for gameplay outcomes.

## Block 1 acceptance fixtures

Blocks 2 and 3 must later implement tests for at least:

- completed solo session;
- incomplete session with no `activity_started`;
- started but disconnected session with no completion;
- multiplayer session with two participants;
- local report disabled by default;
- malformed participant code;
- completion before optional settings interaction;
- failure before a server session ID exists;
- count saturation and 16 KiB size ceiling;
- temporary-report deletion when retention is declined;
- contradiction between local completion observation and authoritative replay.

No implementation may weaken these fixtures without updating and separately reviewing this contract. Block 2 covers the authoritative completed, incomplete, started/disconnected, and multiplayer fixtures. Block 3 covers disabled and malformed activation, explicit consent, completion before settings, pre-session failure, first-occurrence semantics, count and encoded-size bounds, retention-declined deletion, and explicit contradiction detection.
