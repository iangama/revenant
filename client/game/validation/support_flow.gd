extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	var bindings := preload("res://input/input_bindings.gd")
	var legacy: Dictionary = bindings.defaults()
	legacy.erase("target_next")
	legacy.attack.key = KEY_V
	legacy.attack.button = JOY_BUTTON_RIGHT_STICK
	var migrated: Dictionary = bindings.sanitize(legacy)
	_check(migrated.attack.key == KEY_V and migrated.attack.button == JOY_BUTTON_RIGHT_STICK, "new target action preserves old custom controls")
	_check(migrated.target_next.key != KEY_V and migrated.target_next.button == -1, "target action avoids existing custom bindings")
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty():
		DirAccess.make_dir_recursive_absolute(directory)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	if OS.get_environment("REVENANT_SUPPORT_ACCESSIBLE") == "1":
		var settings: Dictionary = game.get("_settings").duplicate(true)
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "reduced_motion": true, "reduced_flash": true, "high_contrast": true, "muted": true}, true)
		game.call("_apply_settings", settings, false)
	game.call("_begin_connection", "m34_support_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	_check(game.get("_session").get("support_capable"), "support capability negotiated")
	for point in [Vector3i(0, 0, 6), Vector3i(2, 0, 6)]:
		if not await game.call("_drive_to_route_position", point, 4000):
			_finish("pair approach blocked")
			return
	if not await _until(func() -> bool: return game.call("_enemy_ids").size() == 2):
		_finish("pair did not spawn")
		return
	var lancer := 0
	var mender := 0
	for id in game.call("_enemy_ids"):
		if state.get("actors")[id].archetype == "glass-lancer": lancer = id
		else: mender = id
	_check(game.get("_current_enemy_id") == lancer, "second spawn preserves selected target")
	await game.call("_drive_to_route_position", Vector3i(5, 0, 7), 2500)
	await game.call("_tap_validation_key", KEY_SPACE)
	_check(await _until(func() -> bool: return state.get("actor_health")[lancer] < 200), "selected Lancer receives keyboard attack")
	_check(await _until(func() -> bool: return game.get("_actors")[mender].call("presentation_state").repair_visible), "confirmed repair link is visible")
	_check(state.get("actor_health")[lancer] == 168, "repair increases confirmed Lancer health by eight")
	await _capture("support-repair.png")
	if OS.get_environment("REVENANT_SUPPORT_CAPTURE_ONLY") == "1":
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	var budget: Dictionary = game.get("_presentation_polish").call("scene_budget", game)
	print("M34 Support scene budget: ", budget)
	_check(budget.visible_meshes <= 170 and budget.materials <= 32, "pair respects rendering budget")
	await game.call("_tap_validation_key", KEY_V)
	_check(game.get("_current_enemy_id") == mender, "keyboard cycles to Mender")
	_check(game.get("_actors")[mender].call("presentation_state").targeted, "selected Mender has a target ring")
	var controller := InputEventJoypadButton.new()
	controller.button_index = JOY_BUTTON_RIGHT_STICK
	controller.pressed = true
	Input.parse_input_event(controller)
	await process_frame
	controller.pressed = false
	Input.parse_input_event(controller)
	_check(game.get("_current_enemy_id") == lancer, "controller cycles back to Lancer")
	var enemy: Node3D = game.get("_actors")[mender]
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.position = game.get("_camera").unproject_position(enemy.global_position + Vector3(0, 0.8, 0))
	Input.warp_mouse(click.position)
	await process_frame
	click.pressed = true
	Input.parse_input_event(click)
	await process_frame
	click.pressed = false
	Input.parse_input_event(click)
	_check(await _until(func() -> bool: return state.get("actor_health")[mender] < 120), "mouse chooses and attacks the Mender")
	_check(game.get("_current_enemy_id") == mender, "mouse selection becomes the accessible active target")
	game.get("_target_button").emit_signal("pressed")
	_check(game.get("_current_enemy_id") == lancer, "visible button cycles target without firing")
	game.get("_target_button").emit_signal("pressed")
	await _capture("support-selected.png")
	if OS.get_environment("REVENANT_SUPPORT_RETREAT") == "1":
		await game.call("_drive_to_route_position", Vector3i(1, 0, 7), 2500)
		_check(await _until(func() -> bool: return state.get("objectives").get("relay_mender", {}).get("state") == "Failed"), "leaving the pad withdraws from the pair")
		_check(game.call("_enemy_ids").is_empty(), "withdrawal removes both targets")
		await _capture("support-withdrawn.png")
	else:
		while state.get("actors").has(mender):
			await game.call("_tap_validation_key", KEY_SPACE)
			await create_timer(0.32).timeout
		_check(game.get("_current_enemy_id") == lancer, "Mender death automatically selects surviving Lancer")
		_check(not game.get("_target_button").visible, "target cycle button retires with the second target")
		var health: int = state.get("actor_health")[lancer]
		await create_timer(1.7).timeout
		_check(state.get("actor_health")[lancer] == health, "Mender death stops all repair pulses")
		await _capture("support-link-broken.png")
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("remaining Lancer did not fall")
			return
		_check(await _until(func() -> bool: return state.get("objectives").get("relay_mender", {}).get("state") == "Completed"), "pair completion confirmed")
	_check(not state.get("activity_complete"), "pair is an optional encounter")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 0 and state.get("progression").get("experience", 0) == 0, "pair grants no separate reward")
	await game.call("_drive_to_route_position", Vector3i(5, 0, 0), 5000)
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 2000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden did not spawn")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("Warden did not fall")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "standard mission completes after pair")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1 and state.get("progression").get("experience", 0) == 100, "one standard mission reward")
	if failures.is_empty():
		print("M34 Support flow passed: repair, mouse/keyboard/controller/button targets, death or withdrawal, Warden, standard reward")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
