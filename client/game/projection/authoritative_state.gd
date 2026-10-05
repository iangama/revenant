extends RefCounted

var player_actor_id := 0
var actors := {}
var actor_health := {}
var actor_max_health := {}
var inventory := {}
var progression := {"level": 1, "experience": 0, "experience_to_next_level": 500}
var equipped_weapon_item_id := "pulse_rifle"
var weapon_profiles := {}
var objectives := {}
var activity_complete := false
var prism_state := {}
var survival_state := {}
var gauntlet_state := {}
var acquisition_state := {}
var campaign_state := {}
var module_state := {}
var module_preview := {}
var module_pending := {}
var module_result := {}
var route_state := {}
var route_pending := {}
var route_result := {}
var route_summary := {}
var cooperation_state := {}
var cooperation_pending := {}
var cooperation_result := {}
var cooperation_life_states := {}
var cooperation_summary := {}


func join_world(message: Dictionary) -> void:
	actors.clear()
	actor_health.clear()
	actor_max_health.clear()
	objectives.clear()
	player_actor_id = message.get("player_actor_id", 0)
	activity_complete = false
	prism_state.clear()
	survival_state.clear()
	gauntlet_state.clear()
	acquisition_state.clear()
	campaign_state.clear()
	module_state.clear()
	module_preview.clear()
	module_pending.clear()
	module_result.clear()
	route_state.clear()
	route_pending.clear()
	route_result.clear()
	route_summary.clear()
	cooperation_state.clear()
	cooperation_pending.clear()
	cooperation_result.clear()
	cooperation_life_states.clear()
	cooperation_summary.clear()


func apply_campaign_snapshot(message: Dictionary) -> bool:
	if message.get("revision") not in ["m36-campaign-v1", "m36-campaign-v2", "m36-campaign-v3", "m36-campaign-v4", "m36-campaign-v5", "m36-campaign-v6"] or int(message.get("state_revision", -1)) < 0:
		return false
	var cleared := int(message.get("cleared_chapters", -1))
	var available: int = {"m36-campaign-v1": 2, "m36-campaign-v2": 3, "m36-campaign-v3": 4, "m36-campaign-v4": 5, "m36-campaign-v5": 6, "m36-campaign-v6": 6}[message.get("revision")]
	if message.get("available_chapters") != available or message.get("total_chapters") != 6 or cleared < 0 or cleared > available:
		return false
	var active: Variant = message.get("active")
	if active != null:
		if not active is Dictionary:
			return false
		var index := ["return_signal", "meridian_readings", "broken_supply_line", "counter_signal", "the_breach", "prism_core"].find(active.get("chapter_id"))
		if index < 0 or index >= available or int(active.get("checkpoint", -1)) < 0 or int(active.get("checkpoint", -1)) >= [2, 3, 4, 6, 4, 4][index]:
			return false
		if (active.get("practice", false) and index >= cleared) or (not active.get("practice", false) and index != cleared):
			return false
	if message.get("revision") == "m36-campaign-v6":
		var story: Variant = message.get("story")
		if not story is Dictionary or story.get("revision") != "m36-story-v1": return false
		if story.get("empty_seat") not in ["unseen", "found", "returned"] or story.get("held_connection") not in ["unseen", "found", "returned"]: return false
		if story.get("supply") not in [null, "covered", "service"] or story.get("core") not in [null, "grounded", "direct"]: return false
		var ending := "open_passage" if story.get("core") == "direct" else "route_lit" if story.get("supply") == "service" else "signal_silent"
		if story.get("epilogue") != ending: return false
		if active is Dictionary and (not active.get("run_id") is String or active.run_id.is_empty()): return false
	if int(message.get("state_revision")) < int(campaign_state.get("state_revision", 0)): return false
	campaign_state = message.duplicate(true)
	return true


func apply_actor_spawn(message: Dictionary) -> void:
	var actor_id: int = message.get("actor_id", 0)
	actors[actor_id] = message.duplicate(true)
	actor_health[actor_id] = message.get("health", 100)
	actor_max_health[actor_id] = message.get("max_health", 100)


func apply_actor_update(message: Dictionary) -> void:
	var actor_id: int = message.get("actor_id", 0)
	if not actors.has(actor_id):
		return
	var actor: Dictionary = actors[actor_id]
	actor["position"] = message.get("position", actor.get("position", [0, 0, 0]))


func apply_actor_destroy(message: Dictionary) -> void:
	var actor_id: int = message.get("actor_id", 0)
	if prism_state.get("actor_id") == actor_id:
		prism_state.clear()
	actors.erase(actor_id)
	actor_health.erase(actor_id)
	actor_max_health.erase(actor_id)


func apply_damage(message: Dictionary) -> void:
	actor_health[message.get("target_actor_id", 0)] = message.get("remaining_health", 0)


func apply_prism_state(message: Dictionary) -> void:
	if actors.has(message.get("actor_id")):
		prism_state = message.duplicate(true)


func apply_repair(message: Dictionary) -> void:
	var target: int = message.get("target_actor_id", 0)
	if actors.has(target):
		actor_health[target] = message.get("remaining_health", actor_health[target])


func apply_survival_state(message: Dictionary, run_id: int) -> bool:
	if message.get("run_id") != run_id or run_id < 1 or message.get("player_actor_id") != player_actor_id: return false
	if not actors.has(player_actor_id): return false
	var phases := ["west", "east", "return", "completed", "defeated"]
	var phase: String = message.get("phase", "")
	if phase not in phases or message.get("sequence") != int(survival_state.get("sequence", 0)) + 1: return false
	var health := int(message.get("health", -1))
	var maximum := int(message.get("max_health", -1))
	var recovered := int(message.get("recovered_health", -1))
	if maximum != actor_max_health.get(player_actor_id) or health < 0 or health > maximum: return false
	if recovered < 0 or recovered > 36 or health != actor_health[player_actor_id] + recovered: return false
	if (phase == "defeated") != (health == 0) or not message.get("reserve_used") is bool: return false
	var used: bool = survival_state.get("reserve_used", false)
	if (used and recovered > 0) or message.reserve_used != (used or recovered > 0): return false
	if survival_state.is_empty():
		if phase != "west" or recovered != 0 or health != maximum: return false
	else:
		var previous: int = phases.find(survival_state.phase)
		var current := phases.find(phase)
		if previous >= 3 or current < previous or (phase != "defeated" and current > previous + 1): return false
	var charge: Variant = message.get("charge_remaining_ms")
	var recovery: Variant = message.get("recovery_remaining_ms")
	var position: Array = actors[player_actor_id].position
	if charge != null:
		if typeof(charge) not in [TYPE_INT, TYPE_FLOAT] or charge != int(charge) or charge < 1 or charge > 3600 or phase not in ["west", "east"]: return false
		if position != ([-8, 0, 0] if phase == "west" else [0, 0, 0]): return false
	if recovery != null:
		if typeof(recovery) not in [TYPE_INT, TYPE_FLOAT] or recovery != int(recovery) or recovery < 1 or recovery > 1200 or message.reserve_used or position != [-4, 0, 5]: return false
	if charge != null and recovery != null: return false
	if phase in ["completed", "defeated"] and (charge != null or recovery != null): return false
	survival_state = message.duplicate(true)
	actor_health[player_actor_id] = health
	return true


func apply_gauntlet_state(message: Dictionary, run_id: int, preset: String = "baseline") -> bool:
	if message.get("run_id") != run_id or run_id < 1 or message.get("player_actor_id") != player_actor_id: return false
	if not actors.has(player_actor_id): return false
	var order := [2, 1, 3] if preset.begins_with("bastion_first") else [1, 2, 3]
	var single := "single_reserve" in preset
	var modifier: Dictionary = message.get("modifier", {}) if message.get("modifier") is Dictionary else {}
	if (preset == "baseline") != modifier.is_empty(): return false
	if not modifier.is_empty():
		if modifier.get("preset_id") != preset or modifier.get("stage_index") != order.find(int(message.get("stage", 0))) + 1: return false
		if not modifier.get("reserve_used") is bool or int(modifier.get("elapsed_ms", -1)) < 0: return false
		if not single and (modifier.reserve_used or modifier.get("reserve_remaining_ms") != null): return false
	var stage := int(message.get("stage", 0))
	var phase: String = message.get("phase", "")
	var sequence := int(message.get("sequence", 0))
	if stage < 1 or stage > 3 or phase not in ["combat", "transfer", "completed", "defeated", "abandoned", "interrupted"]: return false
	if sequence < 1 or sequence < int(gauntlet_state.get("sequence", 0)): return false
	if message == gauntlet_state: return true
	var health := int(message.get("health", -1))
	var maximum := int(message.get("max_health", -1))
	var recovered := int(message.get("recovered_health", -1))
	if maximum != actor_max_health.get(player_actor_id) or health < 0 or health > maximum: return false
	if recovered < 0 or recovered > 24 or health != actor_health[player_actor_id] + recovered: return false
	if (phase == "defeated") != (health == 0): return false
	if (phase == "transfer" and stage == 3) or (phase == "completed" and stage != 3): return false
	var entrances := [[2, 0, 6], [5, 0, -6], [5, 0, 2]]
	var position: Array = actors[player_actor_id].position
	if gauntlet_state.is_empty():
		if stage != order[0] or phase != "combat" or recovered != 0 or health != maximum: return false
	else:
		var previous: int = order.find(int(gauntlet_state.stage))
		var current: int = order.find(stage)
		if current < previous or current > previous + 1: return false
		if sequence == int(gauntlet_state.sequence) and phase not in ["abandoned", "interrupted"]: return false
		if gauntlet_state.phase in ["completed", "defeated", "abandoned", "interrupted"]: return false
		var old_modifier: Dictionary = gauntlet_state.get("modifier", {}) if gauntlet_state.get("modifier") is Dictionary else {}
		if not modifier.is_empty():
			if modifier.elapsed_ms < old_modifier.get("elapsed_ms", 0): return false
			if old_modifier.get("reserve_used", false) and not modifier.reserve_used: return false
			if modifier.reserve_used != old_modifier.get("reserve_used", false) and recovered == 0: return false
		if current > previous:
			if gauntlet_state.phase != "transfer" or phase != "combat" or position != entrances[stage - 1]: return false
			if recovered != (0 if single else mini(24, maximum - int(actor_health[player_actor_id]))): return false
		else:
			if recovered != 0:
				if not single or phase != "transfer" or gauntlet_state.phase != "transfer" or old_modifier.get("reserve_used", false) or not modifier.reserve_used: return false
				var pad: Array = [3, 0, 6] if stage == 1 else [6, 0, -6]
				if position != pad or recovered != mini(24, maximum - int(actor_health[player_actor_id])): return false
			if gauntlet_state.phase == "transfer" and phase not in ["transfer", "abandoned", "interrupted"]: return false
	var reserve_hold: Variant = modifier.get("reserve_remaining_ms")
	if reserve_hold != null:
		if typeof(reserve_hold) not in [TYPE_INT, TYPE_FLOAT] or reserve_hold != int(reserve_hold) or reserve_hold < 1 or reserve_hold > 1200: return false
		if not single or phase != "transfer" or modifier.reserve_used or position != ([3, 0, 6] if stage == 1 else [6, 0, -6]): return false
	var hold: Variant = message.get("transfer_remaining_ms")
	if hold != null:
		if typeof(hold) not in [TYPE_INT, TYPE_FLOAT] or hold != int(hold) or hold < 1 or hold > 1200: return false
		if phase != "transfer" or position != entrances[stage - 1]: return false
	gauntlet_state = message.duplicate(true)
	actor_health[player_actor_id] = health
	return true


func apply_objective(message: Dictionary) -> void:
	objectives[message.get("objective_id", "unknown")] = message.duplicate(true)


func apply_inventory_snapshot(message: Dictionary) -> void:
	inventory.clear()
	for item in message.get("items", []):
		inventory[item.get("item_id", "unknown")] = item.get("quantity", 0)


func apply_loot_grant(message: Dictionary) -> void:
	inventory[message.get("item_id", "unknown")] = message.get("resulting_quantity", 0)


func apply_progression(message: Dictionary) -> void:
	progression["level"] = message.get("level", 1)
	progression["experience"] = message.get("experience", 0)
	progression["experience_to_next_level"] = message.get("experience_to_next_level", 500)


func apply_equipment_snapshot(message: Dictionary) -> void:
	weapon_profiles.clear()
	for weapon in message.get("weapons", []):
		weapon_profiles[weapon.get("item_id", "unknown")] = weapon.duplicate(true)
	equipped_weapon_item_id = message.get("equipped_weapon_item_id", "pulse_rifle")


func apply_equipment_change(message: Dictionary) -> bool:
	if not message.get("accepted", false):
		return false
	equipped_weapon_item_id = message.get("equipped_weapon_item_id", equipped_weapon_item_id)
	return true


func apply_activity_complete(_message: Dictionary) -> void:
	activity_complete = true


func begin_module_request(kind: String, operation_id := "", request := {}) -> bool:
	if not module_pending.is_empty() or kind not in ["state", "preview", "combine", "loadout", "claim"]:
		return false
	if kind in ["combine", "loadout"] and not _valid_operation_id(operation_id):
		return false
	module_pending = {
		"kind": kind,
		"operation_id": operation_id,
		"request": request.duplicate(true),
	}
	if kind == "preview":
		module_preview.clear()
	return true


func apply_module_snapshot(message: Dictionary) -> bool:
	if not _valid_module_state(message):
		return false
	_replace_module_state(message)
	module_pending.clear()
	module_result = {"kind": "state", "accepted": true, "message": "module state received", "replayed": false}
	return true


func apply_acquisition_snapshot(message: Dictionary) -> bool:
	if not _valid_acquisition_state(message):
		return false
	acquisition_state = message.duplicate(true)
	return true


func apply_acquisition_claim(message: Dictionary) -> bool:
	if not _valid_acquisition_state(message.get("state", {})) or not _valid_module_state(message.get("modules", {})):
		return false
	var accepted: bool = message.get("accepted", false)
	var replayed: bool = message.get("replayed", false)
	if int(message.get("granted_fragments", -1)) != (2 if accepted and not replayed else 0):
		return false
	acquisition_state = message.state.duplicate(true)
	_replace_module_state(message.modules)
	module_pending.clear()
	module_result = {"kind": "claim", "accepted": accepted, "replayed": replayed, "message": str(message.get("message", ""))}
	return true


func _valid_acquisition_state(state: Dictionary) -> bool:
	if state.get("revision") != "m35-acquisition-v1" or not state.get("can_claim") is bool:
		return false
	var requirements := {"meridian": ["meridian_recovered"], "routes": ["breach_completed", "stabilize_completed"], "prism": ["prism_completed"]}
	var seen: Array[String] = []
	for arc in state.get("arcs", []):
		var id: String = arc.get("arc_id", "")
		if not requirements.has(id) or id in seen or arc.get("status") not in ["locked", "ready", "claimed"] or arc.get("reward_fragments") != 2:
			return false
		seen.append(id)
		var milestones: Array[String] = []
		var complete := true
		for requirement in arc.get("requirements", []):
			if requirement.get("milestone") not in requirements[id] or requirement.get("milestone") in milestones or not requirement.get("completed") is bool:
				return false
			milestones.append(requirement.milestone)
			complete = complete and requirement.completed
		if milestones.size() != requirements[id].size() or (arc.status != "locked") != complete:
			return false
	return seen.size() == 3


func apply_module_preview(message: Dictionary) -> bool:
	module_pending.clear()
	var accepted: bool = message.get("accepted", false)
	if accepted:
		var requested: Array = message.get("requested_modules", [])
		var weapons: Array = message.get("weapons", [])
		if requested.size() > 3 or not _valid_module_weapons(weapons) or int(message.get("max_health", 0)) <= 0:
			return false
		module_preview = message.duplicate(true)
	else:
		module_preview.clear()
	module_result = {
		"kind": "preview",
		"accepted": accepted,
		"message": str(message.get("message", "server returned no preview message")),
		"replayed": false,
	}
	return accepted


func apply_module_combination(message: Dictionary) -> bool:
	return _apply_module_mutation("combine", message)


func apply_module_loadout(message: Dictionary) -> bool:
	return _apply_module_mutation("loadout", message)


func fail_module_request(message: String, connection_lost := false) -> void:
	var kind := str(module_pending.get("kind", "transport"))
	module_result = {
		"kind": kind,
		"accepted": false,
		"message": message,
		"replayed": false,
		"connection_lost": connection_lost,
	}
	module_pending.clear()


func begin_route_state_request() -> bool:
	if not route_pending.is_empty():
		return false
	route_pending = {"kind": "state"}
	route_result.clear()
	return true


func begin_route_choice(operation_id: String, route_id: String) -> bool:
	if (
		not route_pending.is_empty()
		or not _valid_operation_id(operation_id)
		or route_id not in ["breach", "stabilize"]
		or not _route_choice_available()
	):
		return false
	route_pending = {
		"kind": "choice",
		"operation_id": operation_id,
		"route_id": route_id,
	}
	route_result.clear()
	return true


func apply_route_state(message: Dictionary) -> bool:
	if not message.get("accepted", false):
		route_pending.clear()
		route_state.clear()
		route_result = {
			"kind": "state",
			"accepted": false,
			"message": str(message.get("message", "server rejected route capability")),
			"connection_lost": false,
		}
		return false
	if not _valid_route_state(message):
		return _reject_invalid_route_message("invalid authoritative route state")
	var existing_selection = route_state.get("selection", null)
	var incoming_selection = message.get("selection", null)
	if existing_selection is Dictionary and incoming_selection is Dictionary and existing_selection != incoming_selection:
		return _reject_invalid_route_message("authoritative route selection changed")
	route_state = message.duplicate(true)
	if route_pending.get("kind") == "state":
		route_pending.clear()
		route_result = {
			"kind": "state",
			"accepted": true,
			"message": str(message.get("message", "route state received")),
			"connection_lost": false,
		}
	return true


func apply_route_choice_result(message: Dictionary) -> bool:
	var pending := route_pending.duplicate(true)
	route_pending.clear()
	if int(message.get("schema_version", 0)) != 1 or not _bounded_text(str(message.get("message", "")), 128):
		return _reject_invalid_route_message("invalid authoritative route choice result")
	if not message.get("accepted", false):
		route_result = {
			"kind": "choice",
			"accepted": false,
			"replayed": false,
			"message": str(message.get("message", "server rejected route choice")),
			"operation_id": str(message.get("operation_id", "")),
			"connection_lost": false,
		}
		return false
	var operation_id := str(message.get("operation_id", ""))
	var selection = message.get("selection", null)
	if (
		pending.get("kind") != "choice"
		or not _valid_operation_id(operation_id)
		or operation_id != pending.get("operation_id")
		or message.get("session_id") != route_state.get("session_id")
		or not selection is Dictionary
		or not _valid_selection(selection, route_state)
		or selection.get("route_id") != pending.get("route_id")
	):
		return _reject_invalid_route_message("authoritative route choice disagrees with the pending intent")
	route_result = {
		"kind": "choice",
		"accepted": true,
		"replayed": bool(message.get("replayed", false)),
		"message": str(message.get("message", "route choice accepted")),
		"operation_id": operation_id,
		"selection": selection.duplicate(true),
		"connection_lost": false,
	}
	return true


func apply_route_summary(message: Dictionary) -> bool:
	if not _valid_route_summary(message):
		return _reject_invalid_route_message("invalid authoritative route operation summary")
	route_pending.clear()
	route_summary = message.duplicate(true)
	route_result = {
		"kind": "terminal",
		"accepted": true,
		"outcome": str(message.get("outcome", "")),
		"message": "route operation summary received",
		"connection_lost": false,
	}
	return true


func fail_route_request(message: String, connection_lost := false) -> void:
	var kind := str(route_pending.get("kind", "transport"))
	route_pending.clear()
	route_result = {
		"kind": kind,
		"accepted": false,
		"message": message,
		"replayed": false,
		"connection_lost": connection_lost,
	}


func begin_cooperation_state_request() -> bool:
	if not cooperation_pending.is_empty():
		return false
	cooperation_pending = {"kind": "state"}
	cooperation_result.clear()
	return true


func begin_cooperation_start(operation_id: String) -> bool:
	if (
		not cooperation_pending.is_empty()
		or not _valid_operation_id(operation_id)
		or cooperation_state.get("phase") != "eligible"
		or not cooperation_state.get("all_capable", false)
		or cooperation_state.get("operation", null) != null
		or cooperation_state.get("participant_actor_ids", []).is_empty()
		or int(cooperation_state.get("participant_actor_ids", [0])[0]) != player_actor_id
	):
		return false
	cooperation_pending = {"kind": "start", "operation_id": operation_id}
	cooperation_result.clear()
	return true


func begin_cooperation_ping(operation_id: String) -> bool:
	var operation: Dictionary = cooperation_state.get("operation", {})
	if (
		not cooperation_pending.is_empty()
		or not _valid_operation_id(operation_id)
		or operation.get("phase") != "awaiting_ping"
		or _cooperation_local_role() != "anchor"
		or operation.get("ping", null) != null
		or (cooperation_result.get("kind") == "ping" and cooperation_result.get("accepted", false))
	):
		return false
	cooperation_pending = {"kind": "ping", "operation_id": operation_id}
	cooperation_result.clear()
	return true


func begin_cooperation_revive(operation_id: String) -> bool:
	var operation: Dictionary = cooperation_state.get("operation", {})
	if (
		not cooperation_pending.is_empty()
		or not _valid_operation_id(operation_id)
		or operation.get("phase") != "runner_downed"
		or _cooperation_local_role() != "anchor"
		or operation.get("revive", null) != null
		or (cooperation_result.get("kind") == "revive" and cooperation_result.get("accepted", false))
	):
		return false
	cooperation_pending = {"kind": "revive", "operation_id": operation_id}
	cooperation_result.clear()
	return true


func apply_cooperation_state(message: Dictionary) -> bool:
	if not message.get("accepted", false):
		if not _valid_cooperation_state_rejection(message):
			return _reject_invalid_cooperation_message("invalid authoritative cooperation capability rejection")
		cooperation_pending.clear()
		cooperation_state.clear()
		cooperation_result = {
			"kind": "state",
			"accepted": false,
			"message": str(message.get("message", "server rejected cooperation capability")),
			"connection_lost": false,
		}
		return false
	if not _valid_cooperation_state(message):
		return _reject_invalid_cooperation_message("invalid authoritative cooperation state")
	var incoming_operation = message.get("operation", null)
	var existing_operation = cooperation_state.get("operation", null)
	if (
		existing_operation is Dictionary
		and incoming_operation is Dictionary
		and not _same_cooperation_immutable(existing_operation, incoming_operation)
	):
		return _reject_invalid_cooperation_message("immutable cooperation operation changed")
	if (
		not cooperation_state.is_empty()
		and cooperation_state.get("session_id") != message.get("session_id")
	):
		return _reject_invalid_cooperation_message("cooperation session identity changed")
	cooperation_state = message.duplicate(true)
	if cooperation_pending.get("kind") == "state":
		cooperation_pending.clear()
		cooperation_result = {
			"kind": "state",
			"accepted": true,
			"message": str(message.get("message", "cooperation state received")),
			"connection_lost": false,
		}
	return true


func apply_cooperation_start_result(message: Dictionary) -> bool:
	var pending := cooperation_pending.duplicate(true)
	cooperation_pending.clear()
	if not _valid_cooperation_result_envelope(message, "start"):
		return _reject_invalid_cooperation_message("invalid authoritative cooperation start result")
	if not message.get("accepted", false):
		cooperation_result = _cooperation_rejection("start", message)
		return false
	var operation = message.get("operation", null)
	var operation_id := str(message.get("operation_id", ""))
	if (
		not operation is Dictionary
		or not _valid_operation_id(operation_id)
		or operation.get("start_operation_id") != operation_id
		or not _valid_cooperation_operation(operation, cooperation_state.get("participant_actor_ids", []))
		or (pending.get("kind") == "start" and pending.get("operation_id") != operation_id)
	):
		return _reject_invalid_cooperation_message("authoritative cooperation start disagrees with accepted identities")
	cooperation_state["phase"] = operation.get("phase")
	cooperation_state["operation"] = operation.duplicate(true)
	cooperation_result = {
		"kind": "start",
		"accepted": true,
		"replayed": bool(message.get("replayed", false)),
		"message": str(message.get("message", "cooperation start accepted")),
		"operation_id": operation_id,
		"connection_lost": false,
	}
	return true


func apply_cooperation_ping_result(message: Dictionary) -> bool:
	var pending := cooperation_pending.duplicate(true)
	cooperation_pending.clear()
	if not _valid_cooperation_result_envelope(message, "ping"):
		return _reject_invalid_cooperation_message("invalid authoritative cooperation ping result")
	if not message.get("accepted", false):
		cooperation_result = _cooperation_rejection("ping", message)
		return false
	var ping = message.get("ping", null)
	var operation_id := str(message.get("operation_id", ""))
	if (
		not ping is Dictionary
		or not _valid_operation_id(operation_id)
		or ping.get("operation_id") != operation_id
		or not _valid_cooperation_ping(ping)
		or (pending.get("kind") == "ping" and pending.get("operation_id") != operation_id)
	):
		return _reject_invalid_cooperation_message("authoritative cooperation ping disagrees with the request")
	cooperation_result = {
		"kind": "ping",
		"accepted": true,
		"replayed": bool(message.get("replayed", false)),
		"message": str(message.get("message", "cooperation ping accepted")),
		"operation_id": operation_id,
		"ping": ping.duplicate(true),
		"connection_lost": false,
	}
	return true


func apply_cooperation_life_state(message: Dictionary) -> bool:
	if not _valid_cooperation_life_state(message):
		return _reject_invalid_cooperation_message("invalid authoritative cooperation life transition")
	var actor_id := int(message.get("actor_id", 0))
	cooperation_life_states[actor_id] = message.duplicate(true)
	actor_health[actor_id] = int(message.get("health_after", 0))
	return true


func apply_cooperation_revive_result(message: Dictionary) -> bool:
	var pending := cooperation_pending.duplicate(true)
	cooperation_pending.clear()
	if not _valid_cooperation_result_envelope(message, "revive"):
		return _reject_invalid_cooperation_message("invalid authoritative cooperation revive result")
	var accepted: bool = message.get("accepted", false)
	var status := str(message.get("status", ""))
	var operation_id := str(message.get("operation_id", ""))
	var revive = message.get("revive", null)
	if not accepted:
		if status != "rejected" or revive != null:
			return _reject_invalid_cooperation_message("rejected cooperation revive exposed channel state")
		cooperation_result = _cooperation_rejection("revive", message)
		return false
	if (
		status not in ["started", "replayed", "pending", "cancelled", "completed"]
		or not _valid_operation_id(operation_id)
		or not revive is Dictionary
		or revive.get("operation_id") != operation_id
		or revive.get("status") != status
		or not _valid_cooperation_revive(revive)
		or (pending.get("kind") == "revive" and pending.get("operation_id") != operation_id)
	):
		return _reject_invalid_cooperation_message("authoritative cooperation revive channel is contradictory")
	cooperation_result = {
		"kind": "revive",
		"accepted": true,
		"replayed": bool(message.get("replayed", false)),
		"message": str(message.get("message", "cooperation revive updated")),
		"operation_id": operation_id,
		"status": status,
		"revive": revive.duplicate(true),
		"connection_lost": false,
	}
	return true


func apply_cooperation_summary(message: Dictionary) -> bool:
	if not _valid_cooperation_summary(message):
		return _reject_invalid_cooperation_message("invalid authoritative cooperation operation summary")
	cooperation_pending.clear()
	cooperation_summary = message.duplicate(true)
	cooperation_result = {
		"kind": "terminal",
		"accepted": true,
		"outcome": str(message.get("outcome", "")),
		"message": "cooperation operation summary received",
		"connection_lost": false,
	}
	return true


func fail_cooperation_request(message: String, connection_lost := false) -> void:
	var kind := str(cooperation_pending.get("kind", "transport"))
	cooperation_pending.clear()
	cooperation_result = {
		"kind": kind,
		"accepted": false,
		"message": message,
		"replayed": false,
		"connection_lost": connection_lost,
	}


func _apply_module_mutation(kind: String, message: Dictionary) -> bool:
	module_pending.clear()
	var state: Dictionary = message.get("state", {})
	if not _valid_module_state(state):
		return false
	_replace_module_state(state)
	var accepted: bool = message.get("accepted", false)
	module_result = {
		"kind": kind,
		"accepted": accepted,
		"message": str(message.get("message", "server returned no module result message")),
		"replayed": bool(message.get("replayed", false)),
		"operation_id": str(message.get("operation_id", "")),
	}
	return accepted


func _replace_module_state(state: Dictionary) -> void:
	module_state = state.duplicate(true)
	inventory["relay_core_fragment"] = int(module_state.get("fragments", 0))


func _valid_module_weapons(weapons: Array, revision := "") -> bool:
	var expected := ["pulse_rifle", "arc_sidearm"]
	if revision in ["m31-v1", "m35-v1", "m35-v2"] or (revision.is_empty() and weapon_profiles.has("coil_lance")):
		expected.append("coil_lance")
	if revision in ["m35-v1", "m35-v2"] or (revision.is_empty() and weapon_profiles.has("scatter_caster")):
		expected.append_array(["scatter_caster", "rail_driver"])
	if weapons.size() != expected.size():
		return false
	for weapon in weapons:
		if not weapon is Dictionary or weapon.get("item_id", "") not in expected:
			return false
		expected.erase(weapon.get("item_id"))
	return expected.is_empty()


func _valid_module_state(state: Dictionary) -> bool:
	return (
		str(state.get("catalog_revision", "")) in ["m26-v1", "m31-v1", "m35-v1", "m35-v2"]
		and state.get("catalog", []).size() == (10 if state.get("catalog_revision") == "m35-v2" else (6 if state.get("catalog_revision") in ["m31-v1", "m35-v1"] else 4))
		and _valid_module_weapons(state.get("weapons", []), str(state.get("catalog_revision", "")))
		and state.get("owned_modules", []).size() <= state.get("catalog", []).size()
		and state.get("equipped_modules", []).size() <= 3
		and int(state.get("maximum_slots", 0)) == 3
		and int(state.get("fragments", -1)) >= 0
		and int(state.get("max_health", 0)) > 0
	)


func _valid_operation_id(value: String) -> bool:
	if value.is_empty() or value.length() > 32:
		return false
	var bytes := value.to_utf8_buffer()
	if bytes.size() != value.length():
		return false
	for byte in bytes:
		var ascii_alphanumeric := (byte >= 48 and byte <= 57) or (byte >= 65 and byte <= 90) or (byte >= 97 and byte <= 122)
		if not ascii_alphanumeric and byte != 45:
			return false
	return true


func _route_choice_available() -> bool:
	return (
		not route_state.is_empty()
		and route_state.get("accepted", false)
		and route_state.get("phase") == "choice_open"
		and route_state.get("all_capable", false)
		and int(route_state.get("leader_actor_id", 0)) == player_actor_id
		and route_state.get("selection", null) == null
		and route_result.get("selection", null) == null
	)


func _valid_route_state(message: Dictionary) -> bool:
	var participants: Array = message.get("participant_actor_ids", [])
	var capable: Array = message.get("capable_actor_ids", [])
	var routes: Array = message.get("routes", [])
	if (
		int(message.get("schema_version", 0)) != 1
		or not message.get("accepted", false)
		or not _bounded_text(str(message.get("message", "")), 128)
		or not _valid_identity(str(message.get("session_id", "")))
		or not _valid_id(str(message.get("activity_id", "")))
		or message.get("phase") not in ["waiting", "drone", "choice_open", "baseline_locked", "routed", "succeeded", "failed"]
		or participants.size() < 1
		or participants.size() > 2
		or not _unique_positive_integers(participants)
		or int(message.get("leader_actor_id", 0)) != int(participants[0])
		or capable.size() > participants.size()
		or not _unique_positive_integers(capable, true)
	):
		return false
	for actor_id in capable:
		if actor_id not in participants:
			return false
	if bool(message.get("all_capable", false)) != (capable.size() == participants.size()):
		return false
	if routes.size() != 2 or not _valid_route_option(routes[0]) or not _valid_route_option(routes[1]):
		return false
	var route_ids := [str(routes[0].get("route_id", "")), str(routes[1].get("route_id", ""))]
	if "breach" not in route_ids or "stabilize" not in route_ids or route_ids[0] == route_ids[1]:
		return false
	var selection = message.get("selection", null)
	if selection == null:
		return true
	return (
		selection is Dictionary
		and message.get("phase") in ["routed", "succeeded", "failed"]
		and _valid_selection(selection, message)
	)


func _valid_route_option(option: Dictionary) -> bool:
	var path: Array = option.get("objective_path", [])
	var events: Array = option.get("events", [])
	var route_id := str(option.get("route_id", ""))
	if (
		route_id not in ["breach", "stabilize"]
		or path.size() < 3
		or path.size() > 4
		or path[0] != "clear_drone_group"
		or path[-1] != "defeat_warden"
		or path[0] != "clear_drone_group"
		or path[-1] != "defeat_warden"
		or not _unique_strings(path, ["clear_drone_group", "reach_relay_stabilizer", "reach_relay_door", "defeat_warden"])
		or int(option.get("duration_budget_ms", 0)) != 90_000
		or not _valid_reward(option.get("reward", {}))
		or events.size() != 2
	):
		return false
	var event_ids: Array[String] = []
	for event in events:
		if not event is Dictionary or not _valid_id(str(event.get("event_id", ""))) or not _valid_effect(event.get("effect", {})):
			return false
		event_ids.append(str(event.get("event_id")))
	return event_ids[0] != event_ids[1]


func _valid_selection(selection: Dictionary, state: Dictionary) -> bool:
	var participants: Array = selection.get("participant_actor_ids", [])
	var path: Array = selection.get("objective_path", [])
	if (
		int(selection.get("leader_actor_id", 0)) <= 0
		or participants.size() < 1
		or participants.size() > 2
		or not _unique_positive_integers(participants)
		or int(selection.get("leader_actor_id", 0)) != int(participants[0])
		or selection.get("route_id") not in ["breach", "stabilize"]
		or int(selection.get("seed", -1)) < 0
		or not _valid_id(str(selection.get("event_id", "")))
		or not _valid_effect(selection.get("effect", {}))
		or path.size() < 3
		or path.size() > 4
		or not _unique_strings(path, ["clear_drone_group", "reach_relay_stabilizer", "reach_relay_door", "defeat_warden"])
		or int(selection.get("duration_budget_ms", 0)) != 90_000
		or not _valid_reward(selection.get("reward", {}))
	):
		return false
	if state.is_empty():
		return true
	if (
		selection.get("leader_actor_id") != state.get("leader_actor_id")
		or participants != state.get("participant_actor_ids", [])
	):
		return false
	for option in state.get("routes", []):
		if option.get("route_id") != selection.get("route_id"):
			continue
		if (
			option.get("objective_path", []) != path
			or option.get("duration_budget_ms") != selection.get("duration_budget_ms")
			or option.get("reward", {}) != selection.get("reward", {})
		):
			return false
		for event in option.get("events", []):
			if event.get("event_id") == selection.get("event_id"):
				return event.get("effect", {}) == selection.get("effect", {})
	return false


func _valid_route_summary(summary: Dictionary) -> bool:
	var selection: Dictionary = route_result.get("selection", route_state.get("selection", {}))
	var participants: Array = summary.get("participant_actor_ids", [])
	var transitions: Array = summary.get("transitions", [])
	if (
		selection.is_empty()
		or int(summary.get("schema_version", 0)) != 1
		or not _valid_identity(str(summary.get("session_id", "")))
		or summary.get("session_id") != route_state.get("session_id")
		or summary.get("authoring_revision") != "m27-v1"
		or summary.get("catalog_revision") != "m27-v1"
		or summary.get("resolver_revision") != "m27-permute63-v1"
		or not _valid_operation_id(str(summary.get("operation_id", "")))
		or summary.get("operation_id") != route_result.get("operation_id", summary.get("operation_id"))
		or summary.get("leader_actor_id") != selection.get("leader_actor_id")
		or participants != selection.get("participant_actor_ids", [])
		or summary.get("route_id") != selection.get("route_id")
		or summary.get("seed") != selection.get("seed")
		or summary.get("event_id") != selection.get("event_id")
		or summary.get("effect", {}) != selection.get("effect", {})
		or summary.get("objective_path", []) != selection.get("objective_path", [])
		or summary.get("duration_budget_ms") != selection.get("duration_budget_ms")
		or int(summary.get("elapsed_ms", -1)) < 0
		or transitions.is_empty()
		or transitions.size() > 8
	):
		return false
	for index in transitions.size():
		var transition: Dictionary = transitions[index]
		if (
			int(transition.get("ordinal", -1)) != index
			or transition.get("objective_id") not in selection.get("objective_path", [])
			or transition.get("from") not in ["pending", "active"]
			or transition.get("to") not in ["active", "completed", "failed"]
			or int(transition.get("progress", -1)) < 0
			or int(transition.get("target", 0)) != 1
		):
			return false
	var outcome := str(summary.get("outcome", ""))
	if outcome == "succeeded":
		return _valid_success_summary(summary, selection, participants)
	if outcome == "failed_timeout":
		var failed_transitions := transitions.filter(func(transition: Dictionary) -> bool: return transition.get("to") == "failed")
		return (
			int(summary.get("elapsed_ms", 0)) > int(summary.get("duration_budget_ms", 0))
			and summary.get("reward", null) == null
			and summary.get("grants", []).is_empty()
			and summary.get("no_reward_reason") == "deadline_exceeded"
			and failed_transitions.size() == 1
		)
	return false


func _valid_success_summary(summary: Dictionary, selection: Dictionary, participants: Array) -> bool:
	var reward: Dictionary = summary.get("reward", {})
	var grants: Array = summary.get("grants", [])
	if (
		int(summary.get("elapsed_ms", 0)) > int(summary.get("duration_budget_ms", 0))
		or reward != selection.get("reward", {})
		or summary.get("no_reward_reason", null) != null
		or grants.size() != participants.size()
		or summary.get("transitions", [])[-1].get("objective_id") != "defeat_warden"
		or summary.get("transitions", [])[-1].get("to") != "completed"
	):
		return false
	var granted_actors: Array = []
	for grant in grants:
		if (
			not grant is Dictionary
			or grant.get("actor_id") not in participants
			or grant.get("actor_id") in granted_actors
			or grant.get("item_id") != reward.get("item_id")
			or grant.get("item_quantity") != reward.get("item_quantity")
			or grant.get("experience") != reward.get("experience")
		):
			return false
		granted_actors.append(grant.get("actor_id"))
	return true


func _valid_reward(reward: Dictionary) -> bool:
	return (
		reward.get("item_id") == "relay_core_fragment"
		and int(reward.get("item_quantity", 0)) >= 1
		and int(reward.get("item_quantity", 0)) <= 2
		and int(reward.get("experience", 0)) >= 100
		and int(reward.get("experience", 0)) <= 175
	)


func _valid_effect(effect: Dictionary) -> bool:
	return (
		int(effect.get("warden_health_basis_points", 0)) >= 10_000
		and int(effect.get("warden_health_basis_points", 0)) <= 12_000
		and int(effect.get("warden_counter_damage", 0)) >= 10
		and int(effect.get("warden_counter_damage", 0)) <= 20
	)


func _valid_identity(value: String) -> bool:
	return _bounded_text(value, 64) and not value.is_empty()


func _valid_id(value: String) -> bool:
	if value.is_empty() or not _bounded_text(value, 32):
		return false
	for byte in value.to_utf8_buffer():
		var valid := (byte >= 97 and byte <= 122) or (byte >= 48 and byte <= 57) or byte in [45, 95]
		if not valid:
			return false
	return true


func _bounded_text(value: String, maximum_bytes: int) -> bool:
	return value.to_utf8_buffer().size() <= maximum_bytes


func _unique_positive_integers(values: Array, allow_empty := false) -> bool:
	if values.is_empty():
		return allow_empty
	var seen := {}
	for value in values:
		if not value is int or int(value) <= 0 or seen.has(value):
			return false
		seen[value] = true
	return true


func _unique_strings(values: Array, allowed: Array) -> bool:
	var seen := {}
	for value in values:
		if not value is String or value not in allowed or seen.has(value):
			return false
		seen[value] = true
	return true


func _reject_invalid_route_message(message: String) -> bool:
	route_pending.clear()
	route_result = {
		"kind": "protocol",
		"accepted": false,
		"message": message,
		"replayed": false,
		"connection_lost": false,
	}
	return false


func _cooperation_local_role() -> String:
	var operation: Dictionary = cooperation_state.get("operation", {})
	for participant in operation.get("participants", []):
		if int(participant.get("actor_id", 0)) == player_actor_id:
			return str(participant.get("role", ""))
	return ""


func _valid_cooperation_result_envelope(message: Dictionary, _kind: String) -> bool:
	return (
		int(message.get("schema_version", 0)) == 1
		and message.get("accepted", false) is bool
		and message.get("replayed", false) is bool
		and _bounded_text(str(message.get("message", "")), 128)
		and _valid_identity(str(message.get("session_id", "")))
		and message.get("session_id") == cooperation_state.get("session_id")
		and _bounded_text(str(message.get("operation_id", "")), 32)
	)


func _valid_cooperation_state(message: Dictionary) -> bool:
	var participants: Array = message.get("participant_actor_ids", [])
	var capable: Array = message.get("capable_actor_ids", [])
	var phase := str(message.get("phase", ""))
	if (
		int(message.get("schema_version", 0)) != 1
		or not message.get("accepted", false)
		or not _bounded_text(str(message.get("message", "")), 128)
		or not _valid_identity(str(message.get("session_id", "")))
		or message.get("activity_id") != "relay_awakening"
		or phase not in ["waiting", "drone", "eligible", "awaiting_anchor", "awaiting_ping", "awaiting_runner", "runner_downed", "revive_channel", "encounter_active", "succeeded", "failed", "unavailable"]
		or participants.size() < 1
		or participants.size() > 2
		or not _unique_positive_integers(participants)
		or capable.size() > participants.size()
		or not _unique_positive_integers(capable, true)
	):
		return false
	for actor_id in capable:
		if actor_id not in participants:
			return false
	if bool(message.get("all_capable", false)) != (capable.size() == participants.size()):
		return false
	var operation = message.get("operation", null)
	if operation == null:
		return phase in ["waiting", "drone", "eligible", "unavailable"]
	return (
		operation is Dictionary
		and participants.size() == 2
		and phase == operation.get("phase")
		and _valid_cooperation_operation(operation, participants)
	)


func _valid_cooperation_state_rejection(message: Dictionary) -> bool:
	var participants: Array = message.get("participant_actor_ids", [])
	var capable: Array = message.get("capable_actor_ids", [])
	if (
		int(message.get("schema_version", 0)) != 1
		or message.get("accepted", true)
		or not _bounded_text(str(message.get("message", "")), 128)
		or not _valid_identity(str(message.get("session_id", "")))
		or message.get("activity_id") != "relay_awakening"
		or message.get("phase") != "unavailable"
		or participants.size() < 1
		or participants.size() > 2
		or not _unique_positive_integers(participants)
		or capable.size() > participants.size()
		or not _unique_positive_integers(capable, true)
		or message.get("all_capable", true)
		or message.get("operation", null) != null
	):
		return false
	for actor_id in capable:
		if actor_id not in participants:
			return false
	return true


func _valid_cooperation_operation(operation: Dictionary, actor_ids: Array) -> bool:
	var participants: Array = operation.get("participants", [])
	var targets: Array = operation.get("targets", [])
	var timing: Dictionary = operation.get("timing", {})
	var contributions: Dictionary = operation.get("contributions", {})
	var phase := str(operation.get("phase", ""))
	if (
		operation.get("catalog_revision") != "m28-v1"
		or not _valid_operation_id(str(operation.get("start_operation_id", "")))
		or phase not in ["awaiting_anchor", "awaiting_ping", "awaiting_runner", "runner_downed", "revive_channel", "encounter_active", "succeeded", "failed"]
		or participants.size() != 2
		or targets.size() != 2
		or actor_ids.size() != 2
		or int(operation.get("observed_elapsed_ms", -1)) < 0
		or int(operation.get("observed_elapsed_ms", -1)) > 60_001
		or participants[0].get("actor_id") != actor_ids[0]
		or participants[1].get("actor_id") != actor_ids[1]
		or not _valid_cooperation_participant(participants[0], "anchor")
		or not _valid_cooperation_participant(participants[1], "runner")
		or not _valid_cooperation_targets(targets)
		or timing != {
			"operation_duration_ms": 60_000,
			"ping_ttl_ms": 5_000,
			"revive_window_ms": 15_000,
			"revive_channel_ms": 2_000,
			"revive_health": 50,
			"maximum_distance_squared": 4,
		}
		or operation.get("reward", {}) != {
			"item_id": "relay_core_fragment",
			"item_quantity": 2,
			"experience": 125,
		}
		or not _valid_cooperation_contributions(contributions, phase)
	):
		return false
	var terminal = operation.get("terminal_outcome", null)
	if phase in ["succeeded", "failed"]:
		if terminal not in ["succeeded", "failed_ping_timeout", "failed_revive_timeout", "failed_operation_timeout", "failed_participant_defeated", "abandoned_disconnect"]:
			return false
	elif terminal != null:
		return false
	var ping = operation.get("ping", null)
	if phase in ["awaiting_runner", "runner_downed", "revive_channel", "encounter_active", "succeeded"]:
		if not ping is Dictionary or not _valid_cooperation_ping(ping):
			return false
	elif ping != null and (not ping is Dictionary or not _valid_cooperation_ping(ping)):
		return false
	var revive = operation.get("revive", null)
	if phase == "revive_channel" and (not revive is Dictionary or not _valid_cooperation_revive(revive)):
		return false
	if revive != null and (not revive is Dictionary or not _valid_cooperation_revive(revive)):
		return false
	return true


func _valid_cooperation_participant(participant: Dictionary, expected_role: String) -> bool:
	var health := int(participant.get("current_health", -1))
	var maximum := int(participant.get("max_health", 0))
	var life := str(participant.get("life", ""))
	return (
		int(participant.get("actor_id", 0)) > 0
		and participant.get("role") == expected_role
		and maximum > 0
		and health >= 0
		and health <= maximum
		and life in ["active", "downed", "defeated"]
		and ((life == "active" and health > 0) or (life in ["downed", "defeated"] and health == 0))
	)


func _valid_cooperation_targets(targets: Array) -> bool:
	var by_name := {}
	for target in targets:
		if not target is Dictionary:
			return false
		by_name[target.get("target", "")] = target.get("position", [])
	return (
		by_name.size() == 2
		and by_name.get("relay_anchor", []) == [3, 0, 3]
		and by_name.get("relay_console", []) == [4, 0, 3]
	)


func _valid_cooperation_contributions(contributions: Dictionary, phase: String) -> bool:
	var values := [
		bool(contributions.get("anchor_arrived", false)),
		bool(contributions.get("pinged", false)),
		bool(contributions.get("runner_arrived", false)),
		bool(contributions.get("revived", false)),
		bool(contributions.get("warden_completed", false)),
	]
	var seen_false := false
	for value in values:
		if seen_false and value:
			return false
		if not value:
			seen_false = true
	var revive_count := int(contributions.get("revive_count", -1))
	if revive_count not in [0, 1] or (values[3] and revive_count != 1) or (not values[3] and revive_count != 0):
		return false
	var required_prefix := {
		"awaiting_anchor": 0,
		"awaiting_ping": 1,
		"awaiting_runner": 2,
		"runner_downed": 3,
		"revive_channel": 3,
		"encounter_active": 4,
		"succeeded": 5,
	}
	if required_prefix.has(phase):
		return values.count(true) == int(required_prefix[phase])
	return phase == "failed"


func _valid_cooperation_ping(ping: Dictionary) -> bool:
	var accepted := int(ping.get("accepted_elapsed_ms", -1))
	var expires := int(ping.get("expires_elapsed_ms", -1))
	return (
		_valid_operation_id(str(ping.get("operation_id", "")))
		and int(ping.get("source_actor_id", 0)) > 0
		and ping.get("target") == "relay_console"
		and accepted >= 0
		and expires == accepted + 5_000
		and ping.get("active", false) is bool
	)


func _valid_cooperation_revive(revive: Dictionary) -> bool:
	var started := int(revive.get("started_elapsed_ms", -1))
	var observed := int(revive.get("observed_elapsed_ms", -1))
	var distance := int(revive.get("distance_squared", -1))
	var status := str(revive.get("status", ""))
	return (
		_valid_operation_id(str(revive.get("operation_id", "")))
		and int(revive.get("source_actor_id", 0)) > 0
		and int(revive.get("target_actor_id", 0)) > 0
		and revive.get("source_actor_id") != revive.get("target_actor_id")
		and started >= 0
		and observed >= started
		and int(revive.get("required_duration_ms", 0)) == 2_000
		and int(revive.get("maximum_distance_squared", -1)) == 4
		and distance >= 0
		and distance <= 1_152
		and status in ["started", "replayed", "pending", "cancelled", "completed"]
		and ((status == "cancelled" and distance > 4) or (status != "cancelled" and distance <= 4))
	)


func _valid_cooperation_life_state(message: Dictionary) -> bool:
	var operation: Dictionary = cooperation_state.get("operation", {})
	var participant: Dictionary = {}
	for candidate in operation.get("participants", []):
		if candidate.get("actor_id") == message.get("actor_id"):
			participant = candidate
			break
	var cause := str(message.get("cause", ""))
	var source = message.get("source_actor_id", null)
	var before := str(message.get("life_before", ""))
	var after := str(message.get("life_after", ""))
	var health_before := int(message.get("health_before", -1))
	var health_after := int(message.get("health_after", -1))
	var maximum := int(message.get("max_health", 0))
	if (
		int(message.get("schema_version", 0)) != 1
		or message.get("session_id") != cooperation_state.get("session_id")
		or participant.is_empty()
		or message.get("role") != participant.get("role")
		or cause not in ["relay_feedback", "revive", "warden"]
		or before not in ["active", "downed"]
		or after not in ["active", "downed", "defeated"]
		or before == after
		or int(message.get("elapsed_ms", -1)) < 0
		or maximum <= 0
		or maximum != int(participant.get("max_health", 0))
		or health_before < 0
		or health_before > maximum
		or health_after < 0
		or health_after > maximum
	):
		return false
	if cause == "relay_feedback":
		return source == null and before == "active" and after == "downed" and health_before > 0 and health_after == 0
	if source == null or int(source) <= 0:
		return false
	if cause == "revive":
		return before == "downed" and after == "active" and health_before == 0 and health_after == 50
	return before == "active" and after == "defeated" and health_before > 0 and health_after == 0


func _same_cooperation_immutable(left: Dictionary, right: Dictionary) -> bool:
	for key in ["catalog_revision", "start_operation_id", "targets", "timing", "reward"]:
		if left.get(key) != right.get(key):
			return false
	var left_participants: Array = left.get("participants", [])
	var right_participants: Array = right.get("participants", [])
	if left_participants.size() != 2 or right_participants.size() != 2:
		return false
	for index in 2:
		for key in ["actor_id", "role", "max_health"]:
			if left_participants[index].get(key) != right_participants[index].get(key):
				return false
	return true


func _valid_cooperation_summary(summary: Dictionary) -> bool:
	var operation: Dictionary = cooperation_state.get("operation", {})
	var participants: Array = summary.get("participants", [])
	var outcome := str(summary.get("outcome", ""))
	if (
		operation.is_empty()
		or int(summary.get("schema_version", 0)) != 1
		or summary.get("session_id") != cooperation_state.get("session_id")
		or summary.get("catalog_revision") != "m28-v1"
		or summary.get("start_operation_id") != operation.get("start_operation_id")
		or summary.get("last_nonterminal_phase") not in ["awaiting_anchor", "awaiting_ping", "awaiting_runner", "runner_downed", "revive_channel", "encounter_active"]
		or participants.size() != 2
		or participants[0].get("actor_id") != operation.get("participants", [])[0].get("actor_id")
		or participants[0].get("role") != "anchor"
		or participants[1].get("actor_id") != operation.get("participants", [])[1].get("actor_id")
		or participants[1].get("role") != "runner"
		or not _valid_cooperation_participant(participants[0], "anchor")
		or not _valid_cooperation_participant(participants[1], "runner")
		or outcome not in ["succeeded", "failed_ping_timeout", "failed_revive_timeout", "failed_operation_timeout", "failed_participant_defeated", "abandoned_disconnect"]
		or int(summary.get("terminal_elapsed_ms", -1)) < 0
		or not _valid_cooperation_contributions(summary.get("contributions", {}), "succeeded" if outcome == "succeeded" else "failed")
	):
		return false
	if outcome == "succeeded":
		return _valid_cooperation_success_summary(summary, participants)
	var expected_reasons := {
		"failed_ping_timeout": "ping_timeout",
		"failed_revive_timeout": "revive_timeout",
		"failed_operation_timeout": "operation_timeout",
		"failed_participant_defeated": "participant_defeated",
		"abandoned_disconnect": "participant_disconnected",
	}
	var requires_subject := outcome in ["failed_participant_defeated", "abandoned_disconnect"]
	if not (
		summary.get("reward", null) == null
		and summary.get("grants", []).is_empty()
		and summary.get("no_reward_reason") == expected_reasons[outcome]
		and ((summary.get("subject_role") in ["anchor", "runner"]) == requires_subject)
	):
		return false
	if outcome == "failed_participant_defeated":
		var subject_role := str(summary.get("subject_role", ""))
		for participant in participants:
			if participant.get("role") == subject_role:
				return participant.get("life") == "defeated" and int(participant.get("current_health", -1)) == 0
		return false
	return true


func _valid_cooperation_success_summary(summary: Dictionary, participants: Array) -> bool:
	var reward := {"item_id": "relay_core_fragment", "item_quantity": 2, "experience": 125}
	var grants: Array = summary.get("grants", [])
	if (
		summary.get("subject_role", null) != null
		or int(summary.get("terminal_elapsed_ms", 60_001)) > 60_000
		or summary.get("reward", {}) != reward
		or summary.get("no_reward_reason", null) != null
		or grants.size() != 2
	):
		return false
	for index in 2:
		var grant: Dictionary = grants[index]
		if (
			grant.get("actor_id") != participants[index].get("actor_id")
			or grant.get("item_id") != reward.item_id
			or grant.get("item_quantity") != reward.item_quantity
			or grant.get("experience") != reward.experience
		):
			return false
	return true


func _cooperation_rejection(kind: String, message: Dictionary) -> Dictionary:
	return {
		"kind": kind,
		"accepted": false,
		"replayed": false,
		"message": str(message.get("message", "server rejected cooperation request")),
		"operation_id": str(message.get("operation_id", "")),
		"connection_lost": false,
	}


func _reject_invalid_cooperation_message(message: String) -> bool:
	cooperation_pending.clear()
	cooperation_state.clear()
	cooperation_life_states.clear()
	cooperation_summary.clear()
	cooperation_result = {
		"kind": "protocol",
		"accepted": false,
		"message": "INVALID SERVER DATA  •  %s" % message,
		"replayed": false,
		"connection_lost": false,
	}
	return false


func presentation_state() -> Dictionary:
	return {
		"player_actor_id": player_actor_id,
		"actor_count": actors.size(),
		"inventory": inventory.duplicate(true),
		"progression": progression.duplicate(true),
		"equipped_weapon_item_id": equipped_weapon_item_id,
		"weapon_profile_count": weapon_profiles.size(),
		"objective_count": objectives.size(),
		"activity_complete": activity_complete,
		"module_state_loaded": not module_state.is_empty(),
		"module_owned_count": module_state.get("owned_modules", []).size(),
		"module_loadout_count": module_state.get("equipped_modules", []).size(),
		"module_pending_kind": module_pending.get("kind", ""),
		"module_result_kind": module_result.get("kind", ""),
		"module_result_accepted": module_result.get("accepted", false),
		"route_state_loaded": not route_state.is_empty(),
		"route_phase": route_state.get("phase", ""),
		"route_pending_kind": route_pending.get("kind", ""),
		"route_result_kind": route_result.get("kind", ""),
		"route_result_accepted": route_result.get("accepted", false),
		"route_summary_outcome": route_summary.get("outcome", ""),
		"cooperation_state_loaded": not cooperation_state.is_empty(),
		"cooperation_phase": cooperation_state.get("phase", ""),
		"cooperation_pending_kind": cooperation_pending.get("kind", ""),
		"cooperation_result_kind": cooperation_result.get("kind", ""),
		"cooperation_result_accepted": cooperation_result.get("accepted", false),
		"cooperation_life_count": cooperation_life_states.size(),
		"cooperation_summary_outcome": cooperation_summary.get("outcome", ""),
	}
