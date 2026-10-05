extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty():
		DirAccess.make_dir_recursive_absolute(directory)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	if OS.get_environment("REVENANT_BULWARK_ACCESSIBLE") == "1":
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "reduced_motion": true, "reduced_flash": true, "high_contrast": true, "muted": true}, true)
		game.call("_apply_settings", settings, false)
	game.call("_begin_connection", "m34_bulwark_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	_check(game.get("_session").get("bulwark_capable"), "Bulwark capability confirmed")
	for point in [Vector3i(4, 0, 0), Vector3i(4, 0, -8), Vector3i(6, 0, -8)]:
		if not await game.call("_drive_to_route_position", point, 4000):
			_finish("training approach blocked")
			return
	if not await _until(func() -> bool: return state.get("objectives").get("steel_bulwark", {}).get("state") == "Active"):
		_finish("Bulwark did not start")
		return
	var enemy_id: int = game.get("_current_enemy_id")
	var enemy: Node = game.get("_actors")[enemy_id]
	var player_id: int = state.get("player_actor_id")
	var health: int = state.get("actor_health")[player_id]
	_check(enemy.call("presentation_state").braced, "initial shield stance is visible")
	await game.call("_tap_validation_key", KEY_SPACE)
	_check(await _until(func() -> bool: return game.get("_status_label").text == "BLOCKED • FLANK OR WAIT"), "blocked shot has distinct confirmed feedback")
	_check(state.get("actor_health")[enemy_id] == 240, "front shot deals no damage")
	await _capture("bulwark-blocked.png")
	var budget: Dictionary = game.get("_presentation_polish").call("scene_budget", game)
	print("M34 Bulwark scene budget: ", budget)
	_check(budget.visible_meshes <= 170 and budget.materials <= 32, "new encounter respects the existing rendering budget")
	if OS.get_environment("REVENANT_BULWARK_CAPTURE_ONLY") == "1":
		print("M34 Bulwark capture reviewed; interrupted encounter without a mission reward")
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	if OS.get_environment("REVENANT_BULWARK_RETREAT") == "1":
		await game.call("_drive_to_route_position", Vector3i(4, 0, -8), 4000)
		_check(await _until(func() -> bool: return state.get("objectives").get("steel_bulwark", {}).get("state") == "Failed"), "leaving the pad withdraws")
		_check(not game.get("_entry_shell").visible, "withdrawal preserves live mission control")
		await _capture("bulwark-withdrawn.png")
	else:
		await game.call("_drive_to_route_position", Vector3i(6, 0, -6), 1500)
		await game.call("_drive_to_route_position", Vector3i(7, 0, -6), 1000)
		_check(enemy.call("presentation_state").braced, "flank reached while shield is still raised")
		await game.call("_tap_validation_key", KEY_SPACE)
		_check(await _until(func() -> bool: return state.get("actor_health")[enemy_id] == 200), "flanking shot bypasses the shield")
		_check(await _until(func() -> bool: return not enemy.call("presentation_state").braced), "server lowers the shield after the slam")
		_check(state.get("actor_health")[player_id] == health, "stepping outside the wedge avoids the slam")
		await _capture("bulwark-recovery.png")
		await game.call("_drive_to_route_position", Vector3i(6, 0, -6), 1000)
		await game.call("_drive_to_route_position", Vector3i(6, 0, -8), 1500)
		await game.call("_tap_validation_key", KEY_SPACE)
		_check(await _until(func() -> bool: return state.get("actor_health")[enemy_id] == 160), "front is vulnerable during recovery")
		_check(await _until(func() -> bool: return enemy.call("presentation_state").braced), "new brace retargets the player")
		_check(await _until(func() -> bool: return state.get("actor_health")[player_id] < health), "standing in the wedge takes timed damage")
		_check(state.get("actor_health")[player_id] == health - 14, "one slam deals exactly 14 damage")
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("Bulwark did not fall")
			return
		_check(await _until(func() -> bool: return state.get("objectives").get("steel_bulwark", {}).get("state") == "Completed"), "Bulwark defeat confirmed")
	_check(not state.get("activity_complete"), "training encounter is not mission completion")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 0 and state.get("progression").get("experience", 0) == 0, "training grants no separate reward")
	await game.call("_drive_to_route_position", Vector3i(6, 0, -8), 2000)
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 4000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden did not spawn")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("Warden did not fall")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "normal mission completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1 and state.get("progression").get("experience", 0) == 100, "one standard mission reward")
	if failures.is_empty():
		print("M34 Bulwark flow passed: shield/flank/recovery/slam or withdrawal, Warden, standard reward")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
