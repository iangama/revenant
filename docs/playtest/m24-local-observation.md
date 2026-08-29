# M24 local observation operator guide

`LocalObservationReportV1` is disabled by default. Enable it only after the participant has received the M24 explanation and explicitly consented to observation.

Set these launch variables:

```text
REVENANT_PLAYTEST_MODE=1
REVENANT_PLAYTEST_PARTICIPANT=PT-A1B2
REVENANT_PLAYTEST_BUILD_ID=m24-pilot-01
REVENANT_PLAYTEST_OBSERVATION_CONSENT=1
REVENANT_PLAYTEST_RETENTION_CONSENT=1
```

The participant code must match `PT-[A-Z0-9]{4}`. The build ID is operator supplied, contains at most 64 ASCII letters, digits, `.`, `_`, or `-`, and must not contain a participant name or Git-derived identifier from the participant machine.

The client prints an explicit active/disabled diagnostic at startup. A retained report is written to `user://playtest/m24-<report_id>.json`. When retention is declined, the report remains a `.tmp` file for immediate supervised review and is deleted on controlled quit. The operator must also delete any remaining temporary file after a crash or forced process termination.

Reports are never uploaded automatically. Reconcile an optional `session_id` with the read-only authoritative Inspector summary; an absent ID or contradiction is a finding, not permission to edit either source. Delete raw reports no later than 30 days after Block 8 synthesis, or earlier when requested using the participant code and report ID.
