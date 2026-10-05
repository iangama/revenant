extends RefCounted

const AUTHORITATIVE_STATE := preload("res://projection/authoritative_state.gd")
const LOGICAL_BOUNDS := Rect2(Vector2.ZERO, Vector2(1280, 720))


func validate(fixtures: Dictionary) -> String:
	var new_operation_id: Callable = fixtures.new_operation_id
	var operation_ids := [new_operation_id.call("c"), new_operation_id.call("c"), new_operation_id.call("l")]
	if operation_ids[0] == operation_ids[1]:
		return "M26 operation identifiers repeat within one client process"
	for operation_id in operation_ids:
		if not _valid_operation_id(operation_id):
			return "M26 operation identifier is outside the bounded wire grammar"
	var projection := AUTHORITATIVE_STATE.new()
	projection.call("join_world", {"player_actor_id": 26})
	projection.call("apply_actor_spawn", {"actor_id": 26, "health": 100, "max_health": 100})
	projection.call("apply_inventory_snapshot", {"items": [{"item_id": "relay_core_fragment", "quantity": 5}]})
	projection.call("apply_equipment_snapshot", {"equipped_weapon_item_id": "pulse_rifle", "weapons": [{"item_id": "pulse_rifle", "damage": 40, "range": 6, "cooldown_ms": 250}]})
	var empty := _snapshot(5, [], [], 0, 100, 40, 250)
	if not projection.call("apply_module_snapshot", empty):
		return "M26 empty authoritative module snapshot was rejected"
	var initial_state: Dictionary = projection.module_state.duplicate(true)
	var initial_inventory: Dictionary = projection.inventory.duplicate(true)
	if not projection.call("begin_module_request", "preview", "", {"modules": ["module_force_matrix"]}):
		return "M26 preview did not enter neutral pending state"
	if projection.module_state != initial_state or projection.inventory != initial_inventory or not projection.module_preview.is_empty():
		return "M26 preview pending state changed authoritative projection"
	var odd_preview := _preview(["module_force_matrix", "module_ward_capacitor"], 119, 47, 333)
	if not projection.call("apply_module_preview", odd_preview):
		return "M26 accepted server preview did not project"
	if int(projection.module_preview.get("weapons", [])[0].get("effective_damage")) != 47 or projection.weapon_profiles.get("pulse_rifle", {}).get("damage") != 40:
		return "M26 preview was locally recomputed or rewrote admitted equipment"

	if not projection.call("begin_module_request", "combine", "godot-c-1", {"module_id": "module_force_matrix"}):
		return "M26 valid combination did not enter pending state"
	if projection.call("apply_module_combination", _combined(false, false, "insufficient fragments", "godot-c-1", "module_force_matrix", empty)):
		return "M26 rejected combination was presented as accepted"
	if projection.module_state != initial_state or projection.inventory != initial_inventory:
		return "M26 rejected combination changed authoritative module state"
	var force_state := _snapshot(3, ["module_force_matrix"], [], 0, 100, 40, 250)
	projection.call("begin_module_request", "combine", "godot-c-2", {"module_id": "module_force_matrix"})
	if not projection.call("apply_module_combination", _combined(true, false, "module combined", "godot-c-2", "module_force_matrix", force_state)):
		return "M26 accepted combination did not project"
	if projection.inventory.get("relay_core_fragment") != 3 or projection.actor_max_health.get(26) != 100 or projection.weapon_profiles.get("pulse_rifle", {}).get("damage") != 40:
		return "M26 accepted combination changed completed-session combat state"
	var final_state := _snapshot(1, ["module_force_matrix", "module_ward_capacitor"], ["module_force_matrix", "module_ward_capacitor"], 1, 120, 48, 325)
	projection.call("begin_module_request", "loadout", "godot-l-1", {"modules": final_state.equipped_modules})
	if not projection.call("apply_module_loadout", _loadout(true, false, "module loadout changed", "godot-l-1", final_state)):
		return "M26 accepted loadout did not project"
	projection.call("begin_module_request", "loadout", "godot-l-1", {"modules": final_state.equipped_modules})
	if not projection.call("apply_module_loadout", _loadout(true, true, "module loadout change replayed", "godot-l-1", final_state)) or not projection.module_result.get("replayed", false):
		return "M26 replayed loadout is not distinct from first acceptance"
	if projection.actor_max_health.get(26) != 100 or projection.weapon_profiles.get("pulse_rifle", {}).get("damage") != 40:
		return "M26 next-session loadout rewrote the current actor or weapon"
	if projection.call("begin_module_request", "combine", "invalid_id", {}):
		return "M26 malformed local operation identifier entered pending state"
	projection.call("begin_module_request", "state")
	projection.call("fail_module_request", "request timed out; reconnect and retry", true)
	if not projection.module_result.get("connection_lost", false):
		return "M26 transport failure is not distinct from server rejection"
	projection.call("join_world", {"player_actor_id": 27})
	if not projection.module_state.is_empty() or not projection.module_result.is_empty():
		return "M26 reconnect retained stale module projection"

	var workshop: Control = fixtures.workshop
	workshop.call("open_workshop")
	workshop.call("present", empty, odd_preview, {}, {}, false)
	workshop.call("select_modules_for_validation", ["module_force_matrix", "module_ward_capacitor"])
	var state: Dictionary = workshop.call("presentation_state")
	if not LOGICAL_BOUNDS.encloses(state.get("panel_rect", Rect2())):
		return "M26 workshop escapes the logical 1280x720 canvas"
	if not state.get("combine_disabled", false) or not state.get("loadout_disabled", false) or state.get("preview_disabled", true):
		return "M26 active-session workshop exposes mutation or blocks preview"
	if "DMG 40" not in state.get("current_text", "") or "DMG 47" not in state.get("preview_text", "") or "NOT ACTIVE" not in state.get("preview_text", ""):
		return "M26 current/preview comparison does not preserve exact server values"
	if "LOCKED" not in state.get("lifecycle_text", ""):
		return "M26 active lifecycle is not explicit"

	workshop.call("present", empty, {}, {"kind": "preview"}, {}, false)
	state = workshop.call("presentation_state")
	if "REQUEST PENDING" not in state.get("result_text", "") or "ACCEPTED" in state.get("result_text", ""):
		return "M26 neutral pending state looks accepted"
	for rejection in [
		"insufficient fragments",
		"module is already owned",
		"module loadout revision is stale",
		"module loadout is unchanged",
		"operation identifier conflict",
		"duplicate module",
		"fourth module rejected",
		"malformed identifier",
	]:
		workshop.call("present", empty, {}, {}, {"kind": "loadout", "accepted": false, "message": rejection}, true)
		state = workshop.call("presentation_state")
		if "REJECTED" not in state.get("result_text", "") or rejection not in state.get("result_text", ""):
			return "M26 server rejection lost its distinct message: %s" % rejection

	var owned_unloaded := _snapshot(1, ["module_force_matrix", "module_ward_capacitor"], [], 0, 100, 40, 250)
	workshop.call("present", owned_unloaded, {}, {}, {}, true)
	workshop.call("select_modules_for_validation", ["module_force_matrix", "module_ward_capacitor"])
	state = workshop.call("presentation_state")
	if state.get("loadout_disabled", true) or "APPLIES NEXT SESSION" not in state.get("lifecycle_text", ""):
		return "M26 complete-state owned loadout is unavailable or lacks disclosure"
	workshop.call("present", final_state, {}, {}, {"kind": "loadout", "accepted": true, "message": "module loadout changed", "replayed": false}, true)
	state = workshop.call("presentation_state")
	if "ACCEPTED" not in state.get("result_text", "") or "CURRENT ACTOR UNCHANGED" not in state.get("result_text", ""):
		return "M26 accepted loadout lacks next-session/current-actor disclosure"
	var required_focus := ["ForceMatrix", "TempoRegulator", "ReachLattice", "WardCapacitor", "Preview", "CombineSelected", "ApplyLoadout", "Close"]
	var focus_cycle := _focus_cycle(workshop.call("focus_start"), required_focus.size() + 2)
	if state.get("focus_names", []) != required_focus or not _contains_all(focus_cycle, required_focus):
		return "M26 workshop controls are not completely keyboard-focusable"
	workshop.call("set_reduced_flash", true)
	if not workshop.call("presentation_state").get("reduced_flash", false):
		return "M26 workshop does not retain Reduced Flash state"
	workshop.call("reset_for_connection")
	state = workshop.call("presentation_state")
	if state.get("state_loaded", true) or not state.get("selected_modules", []).is_empty() or state.get("visible", true):
		return "M26 workshop reconnect reset retained stale state, selection, or visibility"
	workshop.call("close_workshop")
	return ""


func _focus_cycle(start: Control, limit: int) -> Array[String]:
	var names: Array[String] = []
	var current := start
	for _index in range(limit):
		if current == null:
			break
		var current_name := str(current.name)
		if current_name in names:
			break
		names.append(current_name)
		current = current.find_next_valid_focus()
	return names


func _contains_all(actual: Array[String], expected: Array) -> bool:
	for name in expected:
		if name not in actual:
			return false
	return true


func _valid_operation_id(value: String) -> bool:
	if value.is_empty() or value.length() > 32:
		return false
	var bytes := value.to_utf8_buffer()
	if bytes.size() != value.length():
		return false
	for byte in bytes:
		var alphanumeric := (byte >= 48 and byte <= 57) or (byte >= 65 and byte <= 90) or (byte >= 97 and byte <= 122)
		if not alphanumeric and byte != 45:
			return false
	return true


func _snapshot(fragments: int, owned: Array, equipped: Array, revision: int, max_health: int, rifle_damage: int, rifle_cooldown: int) -> Dictionary:
	return {
		"type": "ModuleSnapshot",
		"catalog_revision": "m26-v1",
		"fragments": fragments,
		"owned_modules": owned,
		"loadout_revision": revision,
		"equipped_modules": equipped,
		"maximum_slots": 3,
		"catalog": [
			_definition("module_force_matrix", "force", 2000, 2000, 0, 0),
			_definition("module_tempo_regulator", "tempo", -1000, -1500, 0, 0),
			_definition("module_reach_lattice", "reach", -1000, 0, 2, 0),
			_definition("module_ward_capacitor", "ward", 0, 1000, 0, 20),
		],
		"weapons": [
			_weapon("pulse_rifle", 40, 6, 250, rifle_damage, 6, rifle_cooldown),
			_weapon("arc_sidearm", 25, 8, 150, 30 if max_health == 120 else 25, 8, 195 if max_health == 120 else 150),
		],
		"max_health": max_health,
	}


func _definition(module_id: String, family: String, damage: int, cooldown: int, range_delta: int, health: int) -> Dictionary:
	return {"module_id": module_id, "family": family, "recipe_fragments": 2, "damage_basis_points": damage, "cooldown_basis_points": cooldown, "range_delta": range_delta, "max_health_delta": health}


func _weapon(item_id: String, base_damage: int, base_range: int, base_cooldown: int, effective_damage: int, effective_range: int, effective_cooldown: int) -> Dictionary:
	return {"item_id": item_id, "base_damage": base_damage, "base_range": base_range, "base_cooldown_ms": base_cooldown, "effective_damage": effective_damage, "effective_range": effective_range, "effective_cooldown_ms": effective_cooldown}


func _preview(modules: Array, max_health: int, damage: int, cooldown: int) -> Dictionary:
	return {"type": "ModulePreview", "accepted": true, "message": "module preview resolved", "requested_modules": modules, "weapons": [_weapon("pulse_rifle", 40, 6, 250, damage, 6, cooldown), _weapon("arc_sidearm", 25, 8, 150, 29, 8, 199)], "max_health": max_health}


func _combined(accepted: bool, replayed: bool, message: String, operation_id: String, module_id: String, state: Dictionary) -> Dictionary:
	return {"type": "ModuleCombined", "accepted": accepted, "replayed": replayed, "message": message, "operation_id": operation_id, "module_id": module_id, "state": state}


func _loadout(accepted: bool, replayed: bool, message: String, operation_id: String, state: Dictionary) -> Dictionary:
	return {"type": "ModuleLoadoutChanged", "accepted": accepted, "replayed": replayed, "message": message, "operation_id": operation_id, "state": state}
