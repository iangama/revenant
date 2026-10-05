extends "res://validation/arc_warden_flow.gd"

var frame_ms: Array[float] = []


func _run() -> void:
	var capture_directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not capture_directory.is_empty():
		DirAccess.make_dir_recursive_absolute(capture_directory)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	if OS.get_environment("REVENANT_M33_ACCESSIBLE") == "1":
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "reduced_motion": true, "reduced_flash": true, "high_contrast": true, "muted": true}, true)
		game.call("_apply_settings", settings, false)
	var username := OS.get_environment("REVENANT_M33_USERNAME")
	if username.is_empty():
		username = "m33_meridian_%d" % Time.get_unix_time_from_system()
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	var initial_fragments: int = state.get("inventory").get("relay_core_fragment", 0)
	var initial_xp: int = state.get("progression").get("experience", 0)
	var archive: Control = game.get("_relay_archive")
	_check(archive.call("presentation_state").record_count == 3, "a fresh run does not reveal the Meridian memory")
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	_check(await _until(func() -> bool: return game.call("_meridian_available")), "current client has access to the annex")
	if not await _walk([Vector3i(-12, 0, 0), Vector3i(-16, 0, 0)]):
		return
	_check(game.call("_meridian_active"), "entrance starts authoritative exploration")
	_check(not game.call("_signal_available"), "exploration locks the standard route")
	await _capture("meridian-arrival.png")
	# The route uses actual input and waits for authoritative positions.
	if not await _walk([Vector3i(-24, 0, 0), Vector3i(-24, 0, -7), Vector3i(-28, 0, -7)]):
		return
	_check(await _done("meridian_lens"), "north lens is recorded")
	await _capture("meridian-lens.png")
	if OS.get_environment("REVENANT_M33_CRASH") == "1":
		print("M33 CRASH READY • confirmed lens • account ", username)
		while true:
			await process_frame
	if not await _walk([Vector3i(-30, 0, -7), Vector3i(-30, 0, 0)]):
		return
	_check(await _done("meridian_log"), "west log is recovered")
	await _capture("meridian-keeper.png")
	if not await _walk([Vector3i(-30, 0, 7), Vector3i(-28, 0, 7)]):
		return
	_check(await _done("meridian_gallery"), "survey finishes in the sky gallery")
	if not await _walk([Vector3i(-31, 0, 7), Vector3i(-31, 0, 8)]):
		return
	_check(await _done("meridian_memory"), "optional inscription is recorded")
	await _capture("meridian-gallery.png")
	settings = game.get("_settings").duplicate(true)
	var guidance_mode: String = settings.get("guidance_mode", "Full")
	settings.guidance_mode = "Off"
	game.call("_apply_settings", settings, false)
	await game.call("_tap_validation_key", KEY_J)
	_check(archive.visible and archive.call("presentation_state").selected == 3, "confirmed memory is readable with guidance off")
	_check(archive.call("presentation_state").body.contains("Vale"), "archive retains the recovered inscription")
	await _capture("meridian-memory-reader.png")
	await game.call("_tap_validation_key", KEY_ESCAPE)
	settings.guidance_mode = guidance_mode
	game.call("_apply_settings", settings, false)
	_check(not state.get("activity_complete"), "exploration does not complete the main mission")
	_check(state.get("inventory").get("relay_core_fragment", 0) == initial_fragments, "exploration issues no fragments")
	_check(state.get("progression").get("experience", 0) == initial_xp, "exploration issues no XP")
	var budget: Dictionary = game.get("_presentation_polish").call("scene_budget", game)
	print("M33 annex scene budget: ", budget)
	_check(budget.visible_meshes <= 170 and budget.meshes <= 220, "bounded annex geometry")
	_check(budget.materials <= 32 and budget.audio_nodes == 14 and budget.lights == 5 and budget.particles == 0, "annex retains bounded rendering and audio resources")
	var previous := Time.get_ticks_usec()
	for index in 90:
		await process_frame
		var now := Time.get_ticks_usec()
		frame_ms.append((now - previous) / 1000.0)
		previous = now
	frame_ms.sort()
	print("M33 local frame sample: median_ms=", frame_ms[45], " p95_ms=", frame_ms[85], " static_bytes=", Performance.get_monitor(Performance.MEMORY_STATIC))
	if not await _walk([Vector3i(-24, 0, 8), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0)]):
		return
	_check(await _done("meridian_return"), "returning the log finishes the second activity")
	await game.call("_tap_validation_key", KEY_J)
	_check(archive.visible and archive.call("presentation_state").selected == 3 and archive.call("presentation_state").read_count == 1, "memory can be reread away from the gallery without duplicate discovery")
	await game.call("_tap_validation_key", KEY_ESCAPE)
	await _capture("meridian-return.png")
	if not await _walk([Vector3i(-12, 0, 0), Vector3i(6, 0, 0)]):
		return
	_check(not game.get("_meridian_map").visible, "map closes on return to the hub")
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("normal Warden unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("Warden did not fall")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "normal mission completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == initial_fragments + 1, "exactly one normal fragment")
	_check(state.get("progression").get("experience", 0) == initial_xp + 100, "normal XP once")
	if failures.is_empty():
		print("M33 Meridian flow passed: both activities, optional memory, return loop, Warden and normal rewards • account ", username)
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _done(id: String) -> bool:
	return await _until(func() -> bool: return state.get("objectives").get(id, {}).get("state") == "Completed")


func _walk(points: Array) -> bool:
	for point: Vector3i in points:
		if not await game.call("_drive_to_route_position", point, 6500):
			_finish("could not reach %s" % point)
			return false
	return true
