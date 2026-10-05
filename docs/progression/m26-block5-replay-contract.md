# M26 Block 5 replay vocabulary contract

Date: 2026-08-31  
Review state: **accepted before replay implementation**.  
Scope: exact event names, structured payloads, reconstruction, and read-only
Inspector projection only.

## Frozen vocabulary

Block 5 adds exactly these three `ReplayEventKind` values:

| Event kind | Cardinality and purpose |
| --- | --- |
| `module_state_snapshot` | Exactly one immediately after each M26 `player_joined`; records the participant's complete admitted module evidence. |
| `module_combined` | One only for a newly applied accepted combination; an idempotent retry adds no event. |
| `module_loadout_changed` | One only for a newly applied accepted whole-loadout change; an idempotent retry adds no event. |

No existing event name changes. No generic/fallback module event is accepted.
Unknown replay kinds still fail closed.

## Exact payload structs

All three payloads are deterministic compact JSON objects with
`schema_version: 1`, snake-case keys, canonical module/catalog order, and an
application-enforced maximum of 8,192 UTF-8 bytes. Unknown schema versions,
unknown fields, missing fields, non-object JSON, and oversized payloads are
invalid evidence.

`module_state_snapshot` is:

```text
ModuleStateSnapshotPayloadV1 {
  schema_version,
  character_id,
  protocol_generation: v1 | v2,
  state: ReplayModuleStateEvidence
}
```

`module_combined` is:

```text
ModuleCombinedPayloadV1 {
  schema_version,
  character_id,
  operation_id,
  module_id,
  recipe_fragments,
  before: ReplayModuleStateEvidence,
  after: ReplayModuleStateEvidence
}
```

`module_loadout_changed` is:

```text
ModuleLoadoutChangedPayloadV1 {
  schema_version,
  character_id,
  operation_id,
  before: ReplayModuleStateEvidence,
  after: ReplayModuleStateEvidence
}
```

Every embedded state is:

```text
ReplayModuleStateEvidence {
  catalog_revision,
  catalog: [exact four ModuleDefinition values],
  fragments,
  owned_modules,
  loadout_revision,
  persisted_loadout,
  applied_loadout,
  module_effects_active,
  modifiers,
  weapons: [
    { item_id, base: BaseCombatProfile, effective: EffectiveCombatProfile }
  ]
}
```

The two weapon entries are canonical `pulse_rifle`, then `arc_sidearm`.
Profiles include damage, range, cooldown, and maximum health. Repeating the
fixed catalog and base/effective values is intentional: replay reconstruction
must not query current inventory, loadout tables, or a mutable catalog.

For current V2 evidence, effects are active and `applied_loadout` equals the
persisted loadout. For frozen V1 evidence, effects are inactive,
`applied_loadout` is empty, and effective profiles equal base profiles even if
the same character owns or has persisted V2 modules. The V1 wire contract is
not changed or rewritten.

## Persistence and ordering contract

- `player_joined` and its module snapshot are inserted adjacently in one
  transaction before admission. A failure inserting either exposes neither.
- A newly applied combination inserts inventory, accepted-operation, and
  `module_combined` rows in one transaction. Retry returns the stored result
  and inserts no second replay event.
- A newly applied loadout replaces slots, advances revision, records the
  accepted operation, and inserts `module_loadout_changed` in one transaction.
  Retry inserts no second event.
- Each mutation event is after `activity_completed`, retains the same session,
  account, actor, activity, and character identity, and records exact complete
  before/after state.
- Rejected requests write no operation or replay row.
- Reconstruction consumes strictly increasing replay append IDs, not wall
  clocks.

## Independent reconstruction checks

The replay domain must decode and validate structured evidence independently
of PostgreSQL and the gateway. It rejects:

- mixed session IDs or non-increasing append IDs;
- a module mutation without a snapshot, a duplicate participant snapshot, or
  an M26 join missing its adjacent snapshot once M26 evidence is present;
- event actor/character disagreement or a mutation before completion;
- unknown payload schema/catalog/module/family/profile, non-canonical or
  duplicate ownership/loadout, unowned applied entries, or invalid V1/V2
  activation state;
- a catalog modifier outside the finite envelope or any embedded catalog that
  differs from the exact `m26-v1` catalog;
- embedded aggregate/effective values that disagree with checked, half-up
  arithmetic from the recorded bases, catalog, and applied loadout;
- impossible combination ownership, recipe, fragment, revision, loadout, or
  before/after transition; and
- impossible whole-loadout ownership, fragment, revision, or before/after
  transition.

A session with no module events retains the explicit legacy rule: empty
revision-zero module replay state and M25 base behavior. This rule preserves
all old sessions and does not fabricate a historical snapshot.

## Inspector boundary

The existing read-only session, summary, and event `GET` routes remain the only
Inspector surface. Summary may add module snapshot/participant/combination/
loadout counts. Event responses may add a validated decoded module payload and
the frontend may pretty-print its exact before/after fields. No mutation route,
query parameter, raw SQL, credential, packet injection, or client-owned result
is introduced.

Protocol V2 messages, gameplay gateway admission/mutation routing, and Godot
remain unchanged in Block 5. They are separately locked behind Gates 6 and 7.
