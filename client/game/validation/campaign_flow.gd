extends "res://validation/arc_warden_flow.gd"

var username := ""


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	username = "m36_campaign_%d" % Time.get_unix_time_from_system()
	game.get("_entry_shell").call("select_mode", "campaign")
	await RenderingServer.frame_post_draw
	await _capture("campaign-entry.png")
	game.get("_entry_shell").get("_mode").show_popup()
	await RenderingServer.frame_post_draw
	await _capture("campaign-selector.png")
	game.get("_entry_shell").get("_mode").get_popup().hide()
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and _checkpoint() == 0):
		_finish("campaign drone did not spawn")
		return
	_check_campaign_controls()
	_check(await game.call("_drive_validation_attacks", false, 6500), "campaign drone defeated")
	_check(await _until(func() -> bool: return _checkpoint() == 1), "drone checkpoint confirmed")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 1 and state.get("actors").has(state.get("player_actor_id"))), "chapter one resumes after drone")
	_check(game.get("_current_enemy_id") == 0, "resume does not respawn cleared drone")
	_check(await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 5000), "core door reached")
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "campaign Warden spawns")
	_check(await game.call("_drive_validation_attacks", false, 8500), "campaign Warden defeated")
	_check(await _until(func() -> bool: return state.get("activity_complete") and _cleared() == 1), "first chapter durably completed")
	_check(_balance() == 1, "first clear grants one fragment")
	await _capture("campaign-return-complete.png")
	await _rejoin()
	if not await _until(func() -> bool: return _checkpoint() == 0 and game.get("_campaign_chapter") == "meridian_readings" and state.get("objectives").has("meridian_arrival")):
		_finish("Meridian chapter not admitted")
		return
	_check(game.get("_current_enemy_id") == 0, "Meridian starts without unrelated combat")
	_check_campaign_controls()
	var objective_label: Label = game.get("_objective_label")
	_check("LEITURAS DE MERIDIAN" in objective_label.tr(objective_label.text), "campaign objective uses translated chapter title")
	_check(await game.call("_drive_to_route_position", Vector3i(-16, 0, 0), 6000), "Meridian arrival")
	_check(await _until(func() -> bool: return _checkpoint() == 1), "arrival saved")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 1 and _objective("meridian_arrival") == "Completed"), "arrival restored")
	await _walk([Vector3i(-24, 0, 0), Vector3i(-24, 0, -7), Vector3i(-28, 0, -7), Vector3i(-30, 0, -7), Vector3i(-30, 0, 0)])
	_check(_objective("meridian_log") == "Pending", "recovery waits for completed survey")
	await _walk([Vector3i(-30, 0, 7), Vector3i(-28, 0, 7)])
	_check(await _until(func() -> bool: return _checkpoint() == 2), "survey saved")
	await _capture("campaign-meridian-survey.png")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 2 and _objective("meridian_gallery") == "Completed"), "survey restored")
	await _walk([Vector3i(-30, 0, 7), Vector3i(-30, 0, 0), Vector3i(-30, 0, 7), Vector3i(-24, 0, 7), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0)])
	_check(await _until(func() -> bool: return state.get("activity_complete") and _cleared() == 2), "second chapter completed")
	_check(_balance() == 2 and state.get("progression").get("experience") == 200, "exact two chapter rewards")
	game.call("_refresh_meridian")
	_check(game.get("_status_label").text == game.tr("CHAPTER COMPLETE"), "Meridian refresh preserves chapter completion")
	await _capture("campaign-meridian-complete.png")
	await _rejoin("practice_return_signal")
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and _checkpoint() == 0), "explicit practice starts")
	_check(await game.call("_drive_validation_attacks", false, 6500), "practice drone defeated")
	_check(await _until(func() -> bool: return _checkpoint() == 1), "practice checkpoint")
	_check(await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 5000), "practice core reached")
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "practice Warden spawns")
	_check(await game.call("_drive_validation_attacks", false, 8500), "practice Warden defeated")
	_check(await _until(func() -> bool: return state.get("activity_complete")), "practice completes")
	_check(_balance() == 2 and state.get("progression").get("experience") == 200, "practice gives no repeated reward")
	await _capture("campaign-practice-complete.png")
	_finish()


func _check_campaign_controls() -> void:
	game.call("_refresh_selected_enemy")
	_check(not game.get("_cooperation_button").visible and not game.get("_route_button").visible, "standalone buttons stay hidden in campaign")
	game.call("_open_route_console")
	game.call("_open_cooperation_console")
	_check(not game.get("_route_console").visible and not game.get("_cooperation_console").visible, "standalone console shortcuts stay closed in campaign")


func _rejoin(mode := "campaign") -> void:
	game.call("_return_to_campaign_menu")
	game.get("_entry_shell").call("select_mode", mode)
	await create_timer(0.35).timeout
	game.call("_begin_connection", username)
	await _until(func() -> bool: return not game.get("_entry_shell").visible)


func _walk(points: Array) -> void:
	for point in points:
		if not await game.call("_drive_to_route_position", point, 14000):
			_finish("campaign walk failed at %s" % point)
			return


func _checkpoint() -> int:
	var active: Variant = state.get("campaign_state").get("active")
	return int(active.get("checkpoint", -1)) if active is Dictionary else -1


func _cleared() -> int:
	return int(state.get("campaign_state").get("cleared_chapters", 0))


func _balance() -> int:
	return int(state.get("inventory").get("relay_core_fragment", 0))


func _objective(id: String) -> String:
	return str(state.get("objectives").get(id, {}).get("state", ""))


func _finish(error := "") -> void:
	if not error.is_empty():
		_check(false, error)
	if failures.is_empty():
		print("M36 campaign passed: two chapters, drone/arrival/survey reconnect, first clears and unrewarded practice")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _defeat_pair(elite: bool) -> void:
	var mender := 0
	var guard := 0
	for id in game.call("_enemy_ids"):
		if state.get("actors")[id].archetype == "relay-mender":
			mender = id
		else:
			guard = id
	if mender == 0 or guard == 0:
		_finish("pair identities missing")
		return
	if game.get("_current_enemy_id") != mender:
		game.get("_target_button").emit_signal("pressed")
	_check(game.get("_current_enemy_id") == mender, "target control selects repair source")
	var deadline := Time.get_ticks_msec() + 8000
	while state.get("actors").has(mender) and Time.get_ticks_msec() < deadline:
		await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.3).timeout
	_check(not state.get("actors").has(mender), "repair source defeated")
	deadline = Time.get_ticks_msec() + 12000
	while state.get("actors").has(guard) and Time.get_ticks_msec() < deadline:
		if elite and game.get("_actors")[guard].call("presentation_state").braced:
			var facing: Array = game.get("_actors")[guard].call("presentation_state").facing
			await game.call("_drive_to_route_position", Vector3i(8 - facing[1] * 2, 0, -8 + facing[0] * 2), 2500)
		await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.3).timeout
	_check(not state.get("actors").has(guard), "remaining guard defeated")
