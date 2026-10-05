extends "res://validation/build_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var username := "m35_commission_%d" % Time.get_unix_time_from_system()
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("commission fixture did not join")
		return
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return not state.get("acquisition_state").is_empty() and state.get("module_pending").is_empty()), "commissions load while active")
	var workshop: Control = game.get("_module_workshop")
	workshop.get("_commission_toggle").pressed.emit()
	_check(not state.get("acquisition_state").can_claim, "active mission cannot claim")
	await _capture("commissions-active.png")
	game.call("_close_module_workshop")
	_check(await game.call("_drive_validation_attacks", false, 6500), "drone defeated")
	for position in [Vector3i(-12, 0, 0), Vector3i(-16, 0, 0), Vector3i(-24, 0, 0), Vector3i(-24, 0, -7), Vector3i(-28, 0, -7), Vector3i(-30, 0, -7), Vector3i(-30, 0, 0), Vector3i(-30, 0, 7), Vector3i(-28, 0, 7), Vector3i(-24, 0, 7), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0), Vector3i(-12, 0, 0), Vector3i(6, 0, 0)]:
		if not await game.call("_drive_to_route_position", position, 12000):
			_finish("commission walk failed at %s" % position)
			return
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "core reached")
	_check(await game.call("_drive_validation_attacks", false, 8000), "mission won")
	_check(await _until(func() -> bool: return state.get("activity_complete") and _status("meridian") == "ready"), "completed proof unlocks Meridian")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1, "commission does not auto-grant")
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "workshop state loaded")
	var button: Button = workshop.get("_commission_rows").meridian.button
	button.grab_focus()
	await create_timer(0.2).timeout
	_check(button.get_global_rect().end.y <= 720, "claim reached by keyboard at PT150")
	await _capture("commissions-ready.png")
	var profiles: Dictionary = state.get("weapon_profiles").duplicate(true)
	button.pressed.emit()
	_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "claim completes")
	_check(state.get("module_result").get("accepted", false) and _status("meridian") == "claimed", "server confirms one-time claim")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 3, "claim credits exactly two fragments")
	_check(state.get("weapon_profiles") == profiles, "claim leaves combat unchanged")
	_check(button.disabled, "claimed commission is disabled")
	await _capture("commissions-claimed.png")
	game.call("_request_module_combination", "module_breach_shunt")
	_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "commission fragments can buy a new module")
	var spent: int = state.get("inventory").get("relay_core_fragment", 0)
	game.call("_request_acquisition_claim", "meridian")
	_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "explicit claim retry completes")
	_check(state.get("module_result").get("replayed", false) and state.get("inventory").get("relay_core_fragment", 0) == spent, "retry preserves spending")
	game.call("_close_module_workshop")
	game.get("_session").call("reset_connection")
	game.set("_connection_started", false)
	await create_timer(0.4).timeout
	game.call("_begin_connection", username)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and not state.get("activity_complete")), "reconnected")
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return _status("meridian") == "claimed"), "claim and proof survive reconnect")
	_check(state.get("inventory").get("relay_core_fragment", 0) == spent, "reconnect preserves spent balance")
	game.call("_close_module_workshop")
	await _complete_mission(false)
	game.call("_request_acquisition_claim", "meridian")
	_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "retry after another mission completes")
	_check(state.get("module_result").get("replayed", false) and state.get("inventory").get("relay_core_fragment", 0) == spent + 1, "new mission grants only normal loot")
	if failures.is_empty():
		print("M35 acquisition flow passed: Meridian, PT150, claim, craft, retry, reconnect and normal loot")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _status(arc_id: String) -> String:
	for arc in state.get("acquisition_state").get("arcs", []):
		if arc.arc_id == arc_id:
			return arc.status
	return ""
