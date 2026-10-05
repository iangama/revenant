extends SceneTree

# Focused live-server flow. Run with --path client/game --script
# res://validation/archive_flow.gd; uses the usual loopback gateway.
var game: Node
var archive: Control
var failures: Array[String] = []


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	await process_frame
	archive = game.get("_relay_archive")
	game.call("_begin_connection", "m31_archive_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone did not spawn")
		return
	await game.call("_tap_validation_key", KEY_J)
	_check(not archive.visible, "archive must stay closed during combat")
	if not await game.call("_drive_validation_attacks", false, 7000):
		_finish("drone encounter did not complete")
		return
	if not await _until(func() -> bool: return archive.call("can_open")):
		_finish("archive unavailable between encounters")
		return
	var state: RefCounted = game.get("_authoritative_state")
	var inventory: Dictionary = state.get("inventory").duplicate(true)
	var progression: Dictionary = state.get("progression").duplicate(true)
	for record in 2:
		var position := [-2, 0, -1] if record == 0 else [2, 0, 4]
		if not await _move_to(position):
			_finish("terminal position not confirmed")
			return
		game.get("_player_intents").call("request_attack", true)
		game.get("_player_intents").call("set_ui_movement", Vector2.RIGHT)
		await game.call("_tap_validation_key", KEY_E)
		_check(archive.visible and archive.call("presentation_state").read_count == record + 1, "E discovers the nearby record")
		_check(not game.get("_player_intents").call("presentation_state").attack_requested, "opening clears queued attack")
		_check(game.get("_player_intents").call("presentation_state").ui_movement == Vector2.ZERO, "opening clears queued movement")
		if record == 0:
			_check(not archive.call("read_record", 1), "distant record remains undiscovered")
			_check(not archive.call("read_record", 2), "core stays unavailable before completion")
			# GUI Space closes the focused button without leaking a gameplay attack.
			await game.call("_tap_validation_key", KEY_SPACE)
			_check(not archive.visible, "focused close button accepts Space")
			_check(not game.get("_player_intents").call("take_attack", Time.get_ticks_msec()).requested, "closing Space does not become an attack")
			await game.call("_tap_validation_key", KEY_J)
			_check(archive.visible and archive.call("presentation_state").read_count == 1, "J rereads without duplicating discoveries")
			await game.call("_tap_validation_key", KEY_ESCAPE)
		else:
			await _capture("REVENANT_CAPTURE_ARCHIVE_DISCOVERY")
	_check(state.get("inventory") == inventory and state.get("progression") == progression, "reading has no inventory or progression effect")
	# An already in-flight server move may trigger the boss while the reader is open.
	game.call("_send_message", {"type": "MoveIntent", "position": [6, 0, 0]})
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden did not spawn")
		return
	await process_frame
	_check(not archive.visible, "new combat interrupts reading")
	await game.call("_tap_validation_key", KEY_E)
	_check(not archive.visible, "archive cannot reopen during Warden combat")
	if not await game.call("_drive_validation_attacks", false, 8000):
		_finish("Warden encounter did not complete")
		return
	if not await _until(func() -> bool: return state.get("activity_complete")):
		_finish("completion not confirmed")
		return
	await game.call("_tap_validation_key", KEY_J)
	_check(archive.visible and archive.call("presentation_state").read_count == 3, "confirmed completion recovers core memory remotely")
	await _capture("REVENANT_CAPTURE_ARCHIVE_COMPLETE")
	await game.call("_tap_validation_key", KEY_ESCAPE)
	# The HUD button can reopen the archive; its click cannot reach the game.
	var button: Button = game.get("_archive_button")
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.position = button.get_global_rect().get_center()
	click.pressed = true
	Input.parse_input_event(click)
	await process_frame
	click.pressed = false
	Input.parse_input_event(click)
	await process_frame
	_check(archive.visible, "HUD archive button opens with mouse")
	_check(not game.get("_player_intents").call("presentation_state").attack_requested, "HUD click leaves no pending attack")
	archive.call("reset_for_connection")
	_check(not archive.visible and archive.call("presentation_state").read_count == 0, "new run resets discoveries")
	_finish()


func _move_to(position: Array) -> bool:
	game.call("_send_message", {"type": "MoveIntent", "position": position})
	return await _until(func() -> bool:
		var state: RefCounted = game.get("_authoritative_state")
		return state.get("actors")[state.get("player_actor_id")].get("position") == position)


func _until(condition: Callable, timeout_ms := 6000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while not condition.call() and Time.get_ticks_msec() < deadline:
		await process_frame
	await process_frame
	return condition.call()


func _capture(variable: String) -> void:
	var path := OS.get_environment(variable)
	if not path.is_empty():
		await create_timer(0.3).timeout
		_check(await game.call("_save_review_capture", path) == OK, "archive capture saved")


func _check(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)
		push_error(message)


func _finish(error := "") -> void:
	if not error.is_empty():
		_check(false, error)
	if failures.is_empty():
		print("M31 archive flow passed: discovery, reread, input, combat interruption, completion, reset")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
