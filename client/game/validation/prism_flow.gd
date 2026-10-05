extends "res://validation/arc_warden_flow.gd"

var boss_id := 0


func _mode() -> String:
	return state.get("prism_state").get("mode", "")


func _run() -> void:
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty(): DirAccess.make_dir_recursive_absolute(directory)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	if OS.get_environment("REVENANT_PRISM_ACCESSIBLE") == "1":
		var settings: Dictionary = game.get("_settings").duplicate(true)
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "reduced_motion": true, "reduced_flash": true, "high_contrast": true, "muted": true}, true)
		game.call("_apply_settings", settings, false)
	var username := "m34_prism_%d" % Time.get_unix_time_from_system()
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	_check(game.get("_session").get("prism_capable"), "Prism revision negotiated")
	for point in [Vector3i(4,0,0), Vector3i(4,0,2), Vector3i(5,0,2)]:
		if not await game.call("_drive_to_route_position", point, 2000):
			_finish("Prism approach failed")
			return
	if not await _until(func() -> bool: return _mode() == "Opening"):
		_finish("Prism did not spawn")
		return
	boss_id = game.get("_current_enemy_id")
	_check(state.get("actors")[boss_id].archetype == "prism-warden", "authored core boss selected")
	await game.call("_tap_validation_key", KEY_SPACE)
	_check(await _until(func() -> bool: return game.get("_status_label").text == "SHELL CLOSED • WAIT FOR OPENING"), "closed shell honestly blocks the shot")
	_check(state.get("actor_health")[boss_id] == 320, "opening phase cannot be skipped")
	_check(await _until(func() -> bool: return _mode() == "Warning"), "first lane warning confirmed")
	await _capture("prism-lane.png")
	if OS.get_environment("REVENANT_PRISM_DEFEAT") == "1":
		var deadline := Time.get_ticks_msec() + 22000
		while not game.get("_entry_shell").visible and Time.get_ticks_msec() < deadline: await process_frame
		_check(game.get("_entry_shell").visible, "lethal pattern offers retry")
		_check(not state.get("activity_complete"), "defeat grants no completion")
		await _capture("prism-defeat.png")
		game.call("_begin_connection", username)
		_check(await _until(func() -> bool: return state.get("actors").get(game.get("_current_enemy_id"), {}).get("archetype") == "relay-drone"), "retry starts a fresh drone")
		_check(state.get("prism_state").is_empty() and not state.get("objectives").has("prism_warden"), "retry clears boss state")
		_check(game.get("_sound_captions").position.x == 24, "retry restores normal caption position")
		_check(game.get("_camera").position.is_equal_approx(Vector3(7.8,9.3,11)), "retry restores arrival camera")
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("retry drone failed")
			return
		await _finish_via_warden()
		return
	await game.call("_drive_to_route_position", Vector3i(5,0,1), 1500)
	_check(await _until(func() -> bool: return _mode() == "Recovery"), "lane resolves into an attack window")
	_check(state.get("actor_health")[state.get("player_actor_id")] == 90, "sidestep avoids locked lane")
	for index in 4:
		await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.29).timeout
	_check(await _until(func() -> bool: return _mode() == "Shifting"), "half health starts protected second phase")
	_check(state.get("actor_health")[boss_id] == 160, "phase floor is exact")
	await _capture("prism-shift.png")
	await game.call("_tap_validation_key", KEY_SPACE)
	_check(state.get("actor_health")[boss_id] == 160, "phase pause protects boss")
	_check(await _until(func() -> bool: return _mode() == "Warning"), "center pulse warning confirmed")
	await game.call("_drive_to_route_position", Vector3i(6,0,0), 1800)
	_check(state.get("prism_state").pattern.shape == "Center", "first pulse marks the center")
	await _capture("prism-center.png")
	_check(await _until(func() -> bool: return _mode() == "PulseGap"), "center resolves before outer pulse")
	await game.call("_drive_to_route_position", Vector3i(7,0,0), 1500)
	_check(await _until(func() -> bool: return _mode() == "Warning"), "outer pulse warning confirmed")
	_check(state.get("prism_state").pattern.shape == "Perimeter", "outer pulse leaves a clear center")
	await _capture("prism-perimeter.png")
	var budget: Dictionary = game.get("_presentation_polish").call("scene_budget", game)
	print("M34 Prism scene budget: ", budget)
	_check(budget.visible_meshes <= 170 and budget.materials <= 32, "Prism stays within rendering budget")
	if OS.get_environment("REVENANT_PRISM_CAPTURE_ONLY") == "1":
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	if OS.get_environment("REVENANT_PRISM_RETREAT") == "1":
		await game.call("_drive_to_route_position", Vector3i(4,0,0), 2000)
		_check(await _until(func() -> bool: return state.get("objectives").get("prism_warden", {}).get("state") == "Failed"), "withdrawal confirmed")
		_check(game.call("_enemy_ids").is_empty(), "withdrawal removes boss")
		_check(state.get("prism_state").is_empty(), "withdrawal removes the warning")
		_check(not game.get("_entry_shell").visible, "withdrawal preserves mission control")
		await _capture("prism-withdrawn.png")
		await _finish_via_warden()
		return
	_check(await _until(func() -> bool: return _mode() == "Recovery"), "both pulses open the shell")
	_check(state.get("actor_health")[state.get("player_actor_id")] == 90, "moving out then in avoids both pulses")
	await _capture("prism-open.png")
	var deadline := Time.get_ticks_msec() + 20000
	while not state.get("activity_complete") and Time.get_ticks_msec() < deadline:
		if _mode() == "Recovery": await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.29).timeout
	_check(state.get("activity_complete"), "Prism victory completes the core mission")
	_check(state.get("inventory").get("relay_core_fragment",0) == 1 and state.get("progression").get("experience",0) == 100, "one standard reward without a second boss")
	await _capture("prism-complete.png")
	if failures.is_empty(): print("M34 Prism flow passed: shield, lane dodge, phase floor, center/outer movement, victory and one standard reward")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _finish_via_warden() -> void:
	await game.call("_drive_to_route_position", Vector3i(6,0,0), 3500)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "normal Warden available")
	_check(await game.call("_drive_validation_attacks", false, 6500), "normal Warden defeated")
	_check(await _until(func() -> bool: return state.get("activity_complete")), "ordinary completion preserved")
	_check(state.get("inventory").get("relay_core_fragment",0) == 1 and state.get("progression").get("experience",0) == 100, "fallback mission gives one standard reward")
	if failures.is_empty(): print("M34 Prism fallback passed: withdrawal or defeat/retry, normal Warden, standard reward")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
