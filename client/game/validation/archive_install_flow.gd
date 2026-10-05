extends RefCounted

var game: Node
var state: RefCounted
var failures: Array[String] = []


func run(client: Node) -> void:
	game = client
	state = game.get("_authoritative_state")
	var phase := OS.get_environment("REVENANT_ARCHIVE_PHASE")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	if phase == "install":
		settings.language = "pt_BR"
		settings.ui_scale = 1.25
		preload("res://input/input_bindings.gd").rebind(settings.bindings, "attack", "key", KEY_K)
		game.call("_apply_settings", settings, true)
	else:
		_check(settings.language == "pt_BR" and settings.bindings.attack.key == KEY_K, "installed language and binding survive replacement")
		_check(settings.ui_scale == (1.5 if phase == "hold" else 1.25), "settings load primary or previous readable backup as expected")
	game.call("_begin_connection", OS.get_environment("REVENANT_GAME_USERNAME"))
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("archived gateway unavailable")
		return
	var expected := 0 if phase == "install" else 1
	_check(state.get("inventory").get("relay_core_fragment", 0) == expected, "confirmed fragments survive client replacement and crash")
	_check(state.get("progression").get("experience", 0) == expected * 100, "confirmed experience survives client replacement and crash")
	if phase == "hold":
		var ready := FileAccess.open(OS.get_environment("REVENANT_ARCHIVE_READY"), FileAccess.WRITE)
		ready.store_string("ready\n" if failures.is_empty() else "failed\n")
		ready.close()
		# The external installation check kills only this disposable client.
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("archived drone did not clear")
		return
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 4000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("archived Warden unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("archived Warden did not clear")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "installed client completes a fresh activity")
	_check(state.get("inventory").get("relay_core_fragment", 0) == expected + 1, "exactly one new fragment is confirmed")
	_check(state.get("progression").get("experience", 0) == (expected + 1) * 100, "exactly 100 new XP is confirmed")
	if phase == "install":
		settings.ui_scale = 1.5
		game.call("_apply_settings", settings, true)
	if failures.is_empty():
		print("M32 archived installation passed: %s, persisted settings, fragments and XP" % phase)
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _until(condition: Callable) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while not condition.call() and Time.get_ticks_msec() < deadline:
		await game.get_tree().process_frame
	await game.get_tree().process_frame
	return condition.call()


func _check(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)
		push_error(message)


func _finish(message: String) -> void:
	_check(false, message)
	game.call("_quit_client", 1)
