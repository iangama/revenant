extends "res://validation/campaign_flow.gd"


func _mode() -> String:
	return state.get("prism_state").get("mode", "")


func _run() -> void:
	username = OS.get_environment("REVENANT_CAMPAIGN_FIXTURE_USER")
	if username.is_empty():
		push_error("REVENANT_CAMPAIGN_FIXTURE_USER must name a five-chapter fixture")
		quit(1)
		return
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty(): DirAccess.make_dir_recursive_absolute(directory)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var modes := ["practice_prism_core"] if OS.get_environment("REVENANT_PRISM_PRACTICE_ONLY") == "1" else ["campaign", "practice_prism_core"]
	for entry in modes:
		game.get("_entry_shell").call("select_mode", entry)
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return not game.get("_entry_shell").visible and _checkpoint() == 0 and game.get("_campaign_chapter") == "prism_core"):
			_finish("Prism core entry unavailable")
			return
		_check_campaign_controls()
		var fragments := _balance()
		var xp: int = state.get("progression").get("experience")
		await _capture("prism-entry-" + entry + ".png")
		await _walk([Vector3i(5, 0, 0), Vector3i(5, 0, 2)])
		_check(await _until(func() -> bool: return _checkpoint() == 1 and _mode() != ""), "chamber admitted and Prism started")
		game.call("_send_message", {"type": "MoveIntent", "position": [4, 0, 2]})
		await create_timer(0.15).timeout
		_check(state.get("actors")[state.get("player_actor_id")].position == [5, 0, 2], "campaign arena exit requires Chapter menu")
		if entry == modes[0]:
			await _fight_to_shift()
			await _capture("prism-phase-two.png")
			await _rejoin()
			_check(await _until(func() -> bool: return _checkpoint() == 1 and state.get("prism_state").get("phase") == "Lanes" and game.get("_current_enemy_id") != 0), "interrupted phase two restarts whole encounter")
			_check(state.get("actor_health")[game.get("_current_enemy_id")] == 320, "restarted Prism has full health")
			_check(_objective("prism_arrival") == "Completed", "chamber checkpoint restored without repeated visit")
		await _fight_to_shift()
		await _finish_prism()
		_check(await _until(func() -> bool: return _checkpoint() == 2 and game.get("_current_enemy_id") == 0), "Prism clear saved")
		_check(_balance() == fragments and state.get("progression").get("experience") == xp and not state.get("activity_complete"), "boss death grants nothing and leaves shutdown pending")
		if entry == modes[0]:
			await _rejoin()
			_check(await _until(func() -> bool: return _checkpoint() == 2 and _objective("prism_guard") == "Completed"), "cleared Prism boundary restored")
			_check(game.get("_current_enemy_id") == 0 and state.get("prism_state").is_empty(), "cleared boss and telegraphs stay absent")
		_check(game.get("_status_label").text == game.tr("PRISM WARDEN • CLEARED"), "cleared status does not announce another guardian")
		await _capture("prism-shutdown-" + entry + ".png")
		await _walk([Vector3i(8, 0, 0)])
		_check(await _until(func() -> bool: return _checkpoint() == 3), "source shutdown saved")
		_check(_balance() == fragments and not state.get("activity_complete"), "shutdown still requires final return")
		if entry == modes[0]:
			await _rejoin()
			_check(await _until(func() -> bool: return _checkpoint() == 3 and _objective("prism_shutdown") == "Completed"), "shutdown boundary restored")
		await _walk([Vector3i(0, 0, 0)])
		_check(await _until(func() -> bool: return state.get("activity_complete") and _cleared() == 6), "return completes all six chapters")
		var grant := 1 if entry == "campaign" else 0
		_check(_balance() == fragments + grant and state.get("progression").get("experience") == xp + 100 * grant, "exact final first clear and no practice grant")
		if entry == "campaign":
			_check(game.get("_status_label").text == game.tr("CAMPAIGN COMPLETE"), "campaign has an explicit ending")
		await _capture("prism-ending-" + entry + ".png")
		game.call("_return_to_campaign_menu")
		await create_timer(0.35).timeout
	if failures.is_empty():
		print("M36 Prism core passed: two phases, phase-two restart, cleared/shutdown reconnect, final return, six first clears and unrewarded practice")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _fight_to_shift() -> void:
	var boss: int = game.get("_current_enemy_id")
	_check(await _until(func() -> bool: return _mode() == "Warning"), "lane warning starts")
	_check(game.tr("Step off the stripe. Attack when the shell opens. Chapter menu restarts this fight.") in game.get("_guidance_label").text, "campaign guidance explains fight restart")
	await _walk([Vector3i(5, 0, 1)])
	_check(await _until(func() -> bool: return _mode() == "Recovery"), "sidestep opens first attack window")
	for index in 4:
		await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.29).timeout
	_check(await _until(func() -> bool: return _mode() == "Shifting"), "second phase reached")
	_check(state.get("actor_health")[boss] == 160, "phase floor preserved")
	_check(not game.get("_prism_site").visible, "campaign hides the standalone retreat instruction")
	_check(game.tr("First leave the center, then return. Attack after both pulses.") in game.get("_guidance_label").text, "campaign guidance follows phase two")


func _finish_prism() -> void:
	_check(await _until(func() -> bool: return _mode() == "Warning"), "center pulse starts")
	await _walk([Vector3i(6, 0, 0)])
	_check(state.get("prism_state").pattern.shape == "Center", "center warning confirmed")
	_check(await _until(func() -> bool: return _mode() == "PulseGap"), "center pulse resolved")
	await _walk([Vector3i(7, 0, 0)])
	_check(await _until(func() -> bool: return _mode() == "Warning"), "outer pulse starts")
	_check(state.get("prism_state").pattern.shape == "Perimeter", "outer warning confirmed")
	_check(await _until(func() -> bool: return _mode() == "Recovery"), "second attack window opens")
	var deadline := Time.get_ticks_msec() + 15000
	while _checkpoint() == 1 and Time.get_ticks_msec() < deadline:
		if _mode() == "Recovery": await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.29).timeout
