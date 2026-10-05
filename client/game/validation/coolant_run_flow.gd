extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	game.call("_begin_connection", "m31_coolant_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	_check(await _until(func() -> bool: return game.call("_coolant_available")), "north excursion is available")
	_check_scene_budget()
	await game.call("_drive_to_route_position", Vector3i(0, 0, -5), 4000)
	await game.call("_drive_to_route_position", Vector3i(-1, 0, -5), 4000)
	_check(await _until(func() -> bool: return game.call("_coolant_phase") == "transfer"), "intake primes the cell")
	_check(not game.call("_signal_available"), "selecting coolant hides the incompatible signal excursion")
	await create_timer(8.2).timeout
	_check(await _until(func() -> bool: return game.call("_coolant_phase") == "expired"), "charge expires on the server")
	_check(not state.get("activity_complete"), "expiration does not complete the mission")
	await _capture("coolant-expired.png")
	await game.call("_drive_to_route_position", Vector3i(-1, 0, -4), 3000)
	await game.call("_drive_to_route_position", Vector3i(-1, 0, -5), 3000)
	_check(await _until(func() -> bool: return game.call("_coolant_phase") == "transfer"), "intake starts a fresh attempt")
	await game.call("_drive_to_route_position", Vector3i(4, 0, -5), 4000)
	await _capture("coolant-transfer.png")
	if not await _until(func() -> bool: return game.call("_coolant_phase") == "delivery"):
		_finish("transfer dwell did not complete")
		return
	await game.call("_drive_to_route_position", Vector3i(4, 0, -1), 4000)
	if not await _until(func() -> bool: return game.call("_coolant_phase") == "completed"):
		_finish("delivery dwell did not complete")
		return
	_check(not state.get("activity_complete"), "delivery still requires the Warden")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 0, "field activity grants no extra fragments")
	await _capture("coolant-delivered.png")
	await game.call("_drive_to_route_position", Vector3i(6, 0, -1), 3000)
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 3000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("normal Warden did not spawn")
		return
	_check_scene_budget()
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("normal Warden did not fall")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "main mission completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1, "one normal fragment granted")
	_check(state.get("progression").get("experience", 0) == 100, "normal XP granted once")
	if failures.is_empty():
		print("M31 coolant flow passed: intake, expiration, retry, continuous station dwell, delivery, Warden, normal rewards")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _check_scene_budget() -> void:
	var budget: Dictionary = game.get("_presentation_polish").call("scene_budget", game)
	print("M31 field scene budget: ", budget)
	_check(budget.visible_meshes <= 170 and budget.meshes <= 220, "field geometry stays within rendering and residency limits")
	_check(budget.materials <= 32 and budget.audio_nodes == 14 and budget.particles == 0, "field content preserves material, audio and particle budgets")
