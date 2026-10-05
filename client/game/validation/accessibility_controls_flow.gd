extends "res://validation/arc_warden_flow.gd"

const BINDINGS := preload("res://input/input_bindings.gd")
const STORE := preload("res://presentation/settings/settings_store.gd")


func _run() -> void:
	_check_settings_recovery()
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	_check(BINDINGS.rebind(settings.bindings, "attack", "key", KEY_K), "keyboard attack can be rebound")
	_check(BINDINGS.rebind(settings.bindings, "attack", "button", JOY_BUTTON_B), "controller conflict swaps bindings")
	_check(settings.bindings.interact.button == JOY_BUTTON_A, "displaced controller action remains reachable")
	_check(BINDINGS.rebind(settings.bindings, "move_right", "key", KEY_L), "movement can be rebound")
	game.call("_apply_settings", settings, false)
	game.call("_begin_connection", "m32_controls_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	var enemy_id: int = game.get("_current_enemy_id")
	var health_before: int = state.get("actor_health")[enemy_id]
	await game.call("_tap_validation_key", KEY_SPACE)
	await create_timer(0.3).timeout
	_check(state.get("actor_health")[enemy_id] == health_before, "old keyboard attack no longer fires")
	await _joy_tap(JOY_BUTTON_START)
	_check(game.get("_settings_panel").visible, "controller opens settings")
	await game.call("_tap_validation_key", KEY_K)
	await create_timer(0.15).timeout
	_check(state.get("actor_health")[enemy_id] == health_before, "settings consume gameplay keys")
	await _capture("controls-settings.png")
	await _joy_tap(JOY_BUTTON_B)
	_check(not game.get("_settings_panel").visible, "controller closes settings")
	await create_timer(0.3).timeout
	_check(state.get("actor_health")[enemy_id] == health_before, "close button rebound to attack cannot leak a shot")
	await game.call("_tap_validation_key", KEY_K)
	_check(await _until(func() -> bool: return state.get("actor_health").get(enemy_id, 0) < health_before), "remapped keyboard attacks without cursor aiming")
	if not await _controller_attacks():
		_finish("controller attacks did not defeat drone")
		return
	await _joy_tap(JOY_BUTTON_BACK)
	_check(game.get("_action_bar_focused"), "controller focuses action bar")
	var focus_before: Control = root.gui_get_focus_owner()
	await game.call("_tap_validation_key", KEY_TAB)
	_check(root.gui_get_focus_owner() != focus_before and game.get("_action_bar_focused"), "Tab navigates action bar without resuming movement")
	await _joy_tap(JOY_BUTTON_B)
	_check(not game.get("_action_bar_focused"), "controller returns to gameplay")
	await _joy_tap(JOY_BUTTON_RIGHT_SHOULDER)
	_check(await _until(func() -> bool: return state.get("equipped_weapon_item_id") == "arc_sidearm"), "shoulder cycles advertised weapons")
	var motion := InputEventJoypadMotion.new()
	motion.axis = JOY_AXIS_LEFT_X
	motion.axis_value = 0.75
	Input.parse_input_event(motion)
	var reached := await _until(func() -> bool: return state.get("actors")[state.get("player_actor_id")].position[0] >= 6)
	motion.axis_value = 0.0
	Input.parse_input_event(motion)
	_check(reached, "analog movement reaches core")
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden unavailable")
		return
	if not await _controller_attacks():
		_finish("controller attacks did not defeat Warden")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "controller mission completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1, "controller completion receives one standard reward")
	await _capture("controls-completed.png")
	if failures.is_empty():
		print("M32 controls passed: keyboard/controller rebinding, conflict swap, menu input isolation, focus, analog movement, combat, rewards, settings recovery")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _controller_attacks() -> bool:
	var deadline := Time.get_ticks_msec() + 7000
	while game.get("_current_enemy_id") != 0 and Time.get_ticks_msec() < deadline:
		await _joy_tap(JOY_BUTTON_B)
		await create_timer(0.4).timeout
	return game.get("_current_enemy_id") == 0


func _joy_tap(button: int) -> void:
	var event := InputEventJoypadButton.new()
	event.button_index = button
	event.pressed = true
	Input.parse_input_event(event)
	await process_frame
	event.pressed = false
	Input.parse_input_event(event)
	await process_frame


func _check_settings_recovery() -> void:
	var settings_store := STORE.new()
	var path := "user://m32-controls-validation.cfg"
	for suffix in ["", ".bak", ".tmp"]:
		if FileAccess.file_exists(path + suffix):
			DirAccess.remove_absolute(path + suffix)
	var old := ConfigFile.new()
	old.set_value("presentation", "master_volume", 0.35)
	old.set_value("presentation", "reduced_flash", true)
	_check(old.save(path) == OK, "old settings fixture saved")
	var restored := settings_store.load_settings(path)
	_check(restored.master_volume == 0.35 and restored.reduced_flash and restored.bindings == BINDINGS.defaults(), "old settings preserve values and gain safe new defaults")
	_check(not BINDINGS.rebind(restored.bindings, "attack", "key", KEY_ESCAPE), "Escape remains a recovery key")
	BINDINGS.rebind(restored.bindings, "attack", "key", KEY_K)
	_check(settings_store.save_settings(restored, path) == OK, "new preferences saved atomically")
	_check(settings_store.load_settings(path).bindings.attack.key == KEY_K, "bindings survive reload")
	var invalid: Dictionary = restored.duplicate(true)
	invalid.bindings.move_right.key = KEY_K
	invalid.master_volume = NAN
	var sanitized := settings_store.sanitize(invalid)
	_check(sanitized.bindings.attack.key == KEY_SPACE and sanitized.master_volume == 0.8, "duplicate bindings and non-finite volume recover safely")
	var broken := FileAccess.open(path, FileAccess.WRITE)
	broken.store_string("[broken")
	broken.close()
	var recovered := settings_store.load_settings(path)
	_check(recovered.master_volume == 0.35 and recovered.bindings.attack.key == KEY_SPACE, "corrupt primary loads previous usable settings")
	_check(settings_store.save_settings(restored, path) == OK, "settings can be saved after recovery")
	_check(settings_store.load_settings(path).bindings.attack.key == KEY_K, "recovered settings remain writable")
	for suffix in ["", ".bak", ".tmp"]:
		if FileAccess.file_exists(path + suffix):
			DirAccess.remove_absolute(path + suffix)
