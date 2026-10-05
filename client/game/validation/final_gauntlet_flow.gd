extends "res://validation/challenge_gauntlet_flow.gd"

var _observations := preload("res://validation/final_observations.gd").new()
var _review_stage := ""


func _initialize() -> void:
	process_frame.connect(func() -> void: _observations.sample(game, _review_stage))
	super._initialize()


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m36_campaign_1790156347"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "en", "ui_scale": 1.5, "muted": false, "captions": true, "high_contrast": true, "reduced_motion": true, "reduced_flash": true}, true)
	settings.bindings = preload("res://input/input_bindings.gd").defaults()
	game.call("_apply_settings", settings, false)
	await _join_gauntlet()
	_observations.start()
	_review_stage = "support"
	await game.call("_tap_validation_key", KEY_TAB)
	_check(root.gui_get_focus_owner() == game.get("_target_button"), "Tab focuses first available challenge control")
	game.call("_resume_gameplay")
	if OS.get_environment("REVENANT_REVIEW_ENTRY_ONLY") == "1":
		await _back_to_board()
		if failures.is_empty(): print("M38 final entry passed: multi-roundtrip admission and first visible challenge focus.")
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	await _support()
	await game.call("_tap_validation_key", KEY_TAB)
	_check(root.gui_get_focus_owner() == game.get("_campaign_button"), "transfer focus reaches board when combat controls are hidden")
	await _capture("final-transfer-en150.png")
	game.call("_resume_gameplay")
	await _transfer(1, Vector3i(2,0,6))
	_review_stage = "bastion"
	await _walk([Vector3i(10,0,-6), Vector3i(10,0,-8)])
	await _pair(true)
	await _transfer(2, Vector3i(5,0,-6))
	_review_stage = "prism"
	await _prism_stage(settings)
	_check(await _until(func() -> bool: return state.get("activity_complete")), "final gauntlet complete")
	_check(_balance() == 6 and state.get("progression").experience == 600, "campaign fixture rewards remain unchanged")
	await _back_to_board()
	var entry = game.get("_entry_shell")
	_check(await _until(func() -> bool: return entry.get("_challenge_snapshot").get("mastery") is Dictionary), "mastery board refreshed")
	_review_stage = "archive_cycles"
	var initial_nodes := 0
	var initial_memory := 0
	for cycle in 12:
		entry.call("_open_mastery")
		await create_timer(0.15).timeout
		var archive: Control = entry.get("_mastery_archive")
		archive.get("_close").grab_focus()
		await process_frame
		await process_frame
		_check(root.get_visible_rect().encloses(archive.get("_close").get_global_rect()), "archive scroll follows close focus")
		if cycle == 0: await _capture("final-archive-close-pt150.png")
		await _joy_cancel()
		_check(not archive.visible and root.gui_get_focus_owner() == entry.get("_mastery_button"), "controller closes archive and restores board focus")
		await create_timer(0.15).timeout
		if cycle == 1:
			initial_nodes = int(Performance.get_monitor(Performance.OBJECT_NODE_COUNT))
			initial_memory = int(Performance.get_monitor(Performance.MEMORY_STATIC))
	var final_nodes := int(Performance.get_monitor(Performance.OBJECT_NODE_COUNT))
	var growth := int(Performance.get_monitor(Performance.MEMORY_STATIC)) - initial_memory
	_check(final_nodes <= initial_nodes + 2, "repeated archive navigation does not retain controls")
	_check(growth < 8 * 1024 * 1024, "warm archive memory remains bounded")
	var report: Dictionary = _observations.finish(OS.get_environment("REVENANT_CAPTURE_ARC_DIR"), "gauntlet-observations")
	for row in report.stages.values():
		_check(row.voices_max <= 14 and row.audio_bytes_max < 2 * 1024 * 1024, "largest encounter keeps audio resources bounded")
		_check(row.audio_peak <= 0.708, "sampled gauntlet mix has at least 3 dB headroom")
	print("M38 archive warm-loop observations: nodes %d -> %d; memory delta %d bytes" % [initial_nodes, final_nodes, growth])
	if failures.is_empty(): print("M38 gauntlet review passed: three stages/two Prism phases, transfer focus, controller archive close, 12 warm archive cycles, unchanged rewards and bounded audio/resources.")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _join_gauntlet() -> void:
	# Authentication, verified board and world admission each have their own
	# transport deadline; the whole sequence can validly take over five seconds.
	game.get("_entry_shell").call("select_mode", "challenge_relay_gauntlet")
	game.call("_begin_connection", username)
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		if not game.get("_entry_shell").visible and not _saved().is_empty() and game.call("_enemy_ids").size() == 2:
			_check(game.get("_challenge_state").snapshot.active.contract.contract_id == "relay_gauntlet", "gauntlet identity admitted")
			return
		await process_frame
	_check(false, "gauntlet admission exceeded its complete handshake budget")


func _joy_cancel() -> void:
	var event := InputEventJoypadButton.new()
	event.button_index = JOY_BUTTON_B
	event.pressed = true
	Input.parse_input_event(event)
	await process_frame
	event.pressed = false
	Input.parse_input_event(event)
	await process_frame
