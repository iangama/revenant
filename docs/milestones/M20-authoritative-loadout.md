# M20 — Authoritative Equipment and Loadout

Status: complete and validated locally.

M20 adds one persisted weapon slot. Every Operator owns the established `pulse_rifle` and an original `arc_sidearm`; the rifle remains the default. `revenant-inventory` owns equipability and immutable weapon profiles, while PostgreSQL stores only the selected item identifier.

Protocol V2 clients receive an equipment snapshot and may submit `EquipIntent`. The shared-session coordinator verifies ownership and item type, persists the selection, and broadcasts the authoritative result. Combat uses the equipped profile for damage, range, and cooldown. A cooldown deadline belongs to the attacker rather than the weapon, so switching cannot reset it.

The frozen V1 client receives no M20 messages and continues with the default loadout. M20 intentionally adds no armor, extra slots, random modifiers, upgrades, crafting, trading, level effects, activity, class, or map.
