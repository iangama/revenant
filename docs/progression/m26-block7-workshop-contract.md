# M26 Block 7 honest Godot workshop contract

Date: 2026-08-31  
Review state: **accepted before Godot workshop source changes**.  
Scope: current-V2 presentation and semantic validation only.

## Surface and interaction

The existing gameplay HUD gains one `MODULES [M]` control. Opening it sends an
opt-in `ModuleStateRequest`; ordinary join ordering remains unchanged. The
workshop is a bounded overlay inside the authored 1280x720 canvas with:

- exact fragment balance, revision, owned modules, equipped modules, and
  maximum three slots from `ModuleSnapshot`;
- the four server-supplied catalog rows and recipe costs;
- current server-supplied pulse-rifle/sidearm profiles and maximum health;
- a zero-to-three local candidate selection used only as request input;
- `PREVIEW`, `COMBINE SELECTED`, `APPLY LOADOUT`, and `CLOSE` controls; and
- explicit status/result text, including `SERVER PREVIEW — NOT ACTIVE` and
  `APPLIES NEXT SESSION AFTER ACCEPTANCE`.

The overlay opens and closes by mouse, keyboard focus, `M`, and `Escape`.
Every interactive control has visible focus. The existing `1`, `2`, movement,
attack, settings, guidance, mute, and Reduced Flash paths remain available and
semantically unchanged when the overlay is closed.

## Truth and mutation boundary

Godot never resolves recipe, damage, range, cooldown, or health arithmetic.
It formats only fields received in `ModuleSnapshot`, `ModulePreview`,
`ModuleCombined`, or `ModuleLoadoutChanged`. Candidate selection is not
presented as a build result.

Preview is available whenever a snapshot is loaded. Combination/loadout
controls remain disabled until the authoritative activity is complete. Known
ownership, fragment, slot, and pending state may further disable a control,
but server rejection remains the final truth.

Sending an intent records only a neutral pending descriptor. It does not
change inventory, ownership, loadout, revision, weapon profiles, actor health,
or success styling. A rejected response clears pending, displays its exact
server message, and replaces state only with the unchanged authoritative state
included in the response. An accepted response may replace module state and
fragment presentation, but it does not rewrite the completed actor or active
weapon profile. The interface always says the accepted loadout activates on a
future admission.

Operation identifiers are generated as bounded ASCII alphanumeric/hyphen
tokens. One send attempt owns one token; an explicit transport retry reuses the
pending token. Preview has no operation identity. No confirmation sound or
success effect plays before an accepted server result.

## Projection model

The existing authoritative Godot projection adds:

- `module_state`, replaced only from a server snapshot/result state;
- `module_preview`, populated only by an accepted preview and cleared by a
  rejected preview or new candidate request;
- `module_pending`, a neutral request descriptor; and
- `module_result`, the last accepted/rejected server result.

Accepted module state synchronizes the displayed fragment inventory quantity
because both values came from the same server result. It never overwrites the
admitted `weapon_profiles`, `actor_health`, or `actor_max_health` maps.

## Required semantic evidence

The deterministic harness must prove:

- exact empty and non-empty snapshot formatting;
- current versus preview labels and server values without local arithmetic;
- preview of unowned modules without mutation;
- pending state changing no authoritative field;
- accepted/replayed/rejected/timeout/reconnect wording remaining distinct;
- active-state controls disabled and complete-state controls enabled only from
  authoritative lifecycle state;
- insufficient, already-owned, stale, unchanged, conflict, duplicate, fourth,
  and malformed server messages remain rejection, not success;
- next-session disclosure remains visible after acceptance;
- keyboard focus reaches module choices and all four action controls;
- overlay bounds fit 1280x720 and critical text does not overlap;
- mute and Reduced Flash require no alternate truth path or dynamic resource;
  and
- the prior boundary, presentation, display, combat, manual, and keyboard
  harnesses remain green.

Creator-reviewable capture must show at least the current/preview comparison
and an accepted-next-session state. It is local engineering evidence, not a
claim that another person understood or preferred the workshop.

No catalog, domain, schema, replay, Protocol V2, gateway, frozen V1, version,
or public-network change is authorized in Block 7.
