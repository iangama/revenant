extends SceneTree

# One live Arc Surge encounter; the existing seeded development gateway uses seed 1.
var game: Node
var state: RefCounted
var failures: Array[String] = []


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	game.call("_begin_connection", "m31_arc_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone did not spawn")
		return
	var initial_fragments: int = state.get("inventory").get("relay_core_fragment", 0)
	var initial_xp: int = state.get("progression").get("experience", 0)
	game.call("_open_route_console")
	if not await game.call("_wait_for_route_state", "drone", 4000):
		_finish("route capability unavailable")
		return
	game.call("_close_route_console")
	if not await game.call("_drive_validation_attacks", false, 6000):
		_finish("drone not defeated")
		return
	if not await game.call("_wait_for_route_state", "choice_open", 3000):
		_finish("route choice not open")
		return
	game.call("_open_route_console")
	await game.call("_wait_for_route_state", "choice_open", 3000)
	await _capture("route-choice.png")
	game.get("_route_console").call("select_for_validation", "breach")
	if not await game.call("_wait_for_route_choice", 3000):
		_finish("route was not accepted")
		return
	game.call("_close_route_console")
	if not await _until(func() -> bool: return game.call("_arc_surge_active")):
		_finish("this flow requires a server-confirmed Arc Surge event")
		return
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 5000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden did not spawn")
		return
	var enemy_id: int = game.get("_current_enemy_id")
	var enemy: Node3D = game.get("_actors")[enemy_id]
	_check(enemy.call("presentation_state").get("variant") == "arc_surge", "confirmed event selects the Arc Warden silhouette")
	await create_timer(0.85).timeout
	await _capture("arc-warden.png")
	for expected_position in [[8, 0, -3], [10, 0, 3]]:
		var health_before: int = state.get("actor_health")[enemy_id]
		var click := InputEventMouseButton.new()
		click.button_index = MOUSE_BUTTON_LEFT
		click.position = game.get("_camera").unproject_position(enemy.global_position)
		Input.warp_mouse(click.position)
		await process_frame
		await process_frame
		click.pressed = true
		Input.parse_input_event(click)
		await process_frame
		click.pressed = false
		Input.parse_input_event(click)
		if not await _until(func() -> bool: return state.get("actor_health")[enemy_id] < health_before):
			await _capture("click-failure.png")
			_finish("aimed mouse shot was not confirmed")
			return
		_check(await _until(func() -> bool: return state.get("actors")[enemy_id].position == expected_position), "confirmed hit replicates the alternating core position")
		await create_timer(0.4).timeout
	await _capture("arc-repositioned.png")
	if not await game.call("_drive_validation_attacks", false, 6000):
		_finish("Arc Warden not defeated")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete") and not state.get("route_summary").is_empty()), "route completion confirmed")
	_check(state.get("inventory").get("relay_core_fragment", 0) == initial_fragments + 2, "Breach grants the usual two fragments")
	_check(state.get("progression").get("experience", 0) == initial_xp + 100, "Breach grants the usual XP")
	await game.call("_tap_validation_key", KEY_J)
	_check(game.get("_relay_archive").call("presentation_state").selected == 2, "core archive is readable after the routed encounter")
	_finish()


func _until(condition: Callable) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while not condition.call() and Time.get_ticks_msec() < deadline:
		await process_frame
	await process_frame
	return condition.call()


func _capture(filename: String) -> void:
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty():
		_check(await game.call("_save_review_capture", directory.path_join(filename)) == OK, "capture saved")


func _check(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)
		push_error(message)


func _finish(error := "") -> void:
	if not error.is_empty():
		_check(false, error)
	if failures.is_empty():
		print("M31 Arc Warden flow passed: route choice, silhouette, mouse aiming, replicated repositioning, rewards, archive")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
