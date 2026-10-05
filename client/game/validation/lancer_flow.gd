extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty():
		DirAccess.make_dir_recursive_absolute(directory)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	if OS.get_environment("REVENANT_M34_ACCESSIBLE") == "1":
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "reduced_motion": true, "reduced_flash": true, "high_contrast": true, "muted": true}, true)
		game.call("_apply_settings", settings, false)
	var username := "m34_lancer_%d" % Time.get_unix_time_from_system()
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	_check(game.get("_session").get("encounter_capable"), "M34 capability confirmed")
	await game.call("_drive_to_route_position", Vector3i(4, 0, 4), 4000)
	await game.call("_drive_to_route_position", Vector3i(4, 0, 8), 4000)
	if not await _until(func() -> bool: return state.get("objectives").get("glass_lancer", {}).get("state") == "Active"):
		_finish("Lancer did not start")
		return
	var enemy_id: int = game.get("_current_enemy_id")
	var enemy: Node = game.get("_actors")[enemy_id]
	var player_id: int = state.get("player_actor_id")
	var health_before: int = state.get("actor_health")[player_id]
	_check(await _until(func() -> bool: return enemy.call("presentation_state").charge_warning), "server starts the visible warning")
	await _capture("lancer-warning.png")
	if OS.get_environment("REVENANT_M34_DEFEAT") == "1":
		var deadline := Time.get_ticks_msec() + 20000
		while not game.get("_entry_shell").visible and Time.get_ticks_msec() < deadline:
			await process_frame
		if not game.get("_entry_shell").visible:
			_finish("lethal charge did not offer a retry")
			return
		_check(not state.get("activity_complete"), "lethal charge cannot complete the mission")
		await _capture("lancer-defeat.png")
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return state.get("actors").get(game.get("_current_enemy_id"), {}).get("archetype") == "relay-drone"):
			_finish("fresh retry did not start")
			return
		_check(not state.get("objectives").has("glass_lancer"), "retry clears the old encounter")
		_check(game.get("_sound_captions").position.x == 24, "retry restores the normal HUD")
		_check(game.get("_camera").position.is_equal_approx(Vector3(7.8, 9.3, 11)), "retry restores the arrival camera")
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("retry drone did not clear")
			return
	elif OS.get_environment("REVENANT_M34_RETREAT") == "1":
		await game.call("_drive_to_route_position", Vector3i(4, 0, 4), 4000)
		_check(await _until(func() -> bool: return state.get("objectives").get("glass_lancer", {}).get("state") == "Failed"), "leaving the pad abandons the optional encounter")
		_check(not game.get("_entry_shell").visible and game.get("_current_enemy_id") == 0, "live withdrawal returns control to the mission")
		await _capture("lancer-withdrawn.png")
	else:
		await game.call("_drive_to_route_position", Vector3i(4, 0, 9), 1500)
		_check(await _until(func() -> bool: return not enemy.call("presentation_state").charge_warning), "warning clears only after server resolution")
		_check(state.get("actors")[enemy_id].position == [4, 0, 8], "charge follows the original locked target")
		_check(state.get("actor_health")[player_id] == health_before, "a sideways step avoids charge damage")
		await _capture("lancer-dodged.png")
		_check(await _until(func() -> bool: return enemy.call("presentation_state").charge_warning), "recovery ends with another warning")
		_check(await _until(func() -> bool: return state.get("actor_health")[player_id] < health_before), "standing on the line takes server damage")
		_check(state.get("actor_health")[player_id] == health_before - 18, "one resolved charge deals exactly 18 damage")
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("Lancer did not fall")
			return
		_check(await _until(func() -> bool: return state.get("objectives").get("glass_lancer", {}).get("state") == "Completed"), "Lancer defeat confirmed")
		await game.call("_drive_to_route_position", Vector3i(4, 0, 4), 4000)
	_check(not state.get("activity_complete"), "optional encounter is not mission completion")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 0 and state.get("progression").get("experience", 0) == 0, "optional encounter adds no reward")
	await game.call("_drive_to_route_position", Vector3i(4, 0, 0), 4000)
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 4000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden did not spawn")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("Warden did not fall")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "normal mission resumes and completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1 and state.get("progression").get("experience", 0) == 100, "one standard mission reward")
	if failures.is_empty():
		print("M34 Lancer flow passed: telegraph, dodge/hit or withdrawal, Warden, standard reward")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
