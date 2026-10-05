extends "res://validation/campaign_flow.gd"


func _run() -> void:
	username = OS.get_environment("REVENANT_CAMPAIGN_FIXTURE_USER")
	if username.is_empty():
		push_error("REVENANT_CAMPAIGN_FIXTURE_USER must name a four-chapter fixture")
		quit(1)
		return
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var modes := ["practice_the_breach"] if OS.get_environment("REVENANT_BREACH_PRACTICE_ONLY") == "1" else ["campaign", "practice_the_breach"]
	for mode in modes:
		game.get("_entry_shell").call("select_mode", mode)
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return not game.get("_entry_shell").visible and _checkpoint() == 0 and game.get("_campaign_chapter") == "the_breach"):
			_finish("breach chapter unavailable")
			return
		_check_campaign_controls()
		var fragments := _balance()
		var xp: int = state.get("progression").get("experience")
		await _walk([Vector3i(3, 0, 0), Vector3i(3, 0, 3)])
		_check(_checkpoint() == 0, "stabilizer cannot advance before the door")
		await _walk([Vector3i(6, 0, 3), Vector3i(6, 0, 0)])
		_check(await _until(func() -> bool: return _checkpoint() == 1 and game.get("_current_enemy_id") != 0), "powered guard starts at the door")
		if mode == modes[0]:
			await _resume_at(1, "breach_door", true)
		var enemy: int = game.get("_current_enemy_id")
		game.call("_send_message", {"type": "AttackIntent", "target_actor_id": enemy})
		_check(await _until(func() -> bool: return game.get("_status_label").text == game.tr("SHIELD POWERED • DRAIN THE RELAY")), "blocked shot clearly directs to the relay")
		_check(state.get("actor_health")[enemy] == 240, "powered shield absorbs the accepted shot")
		_check(game.get("_breach_site").get("_shield").visible, "powered shield projected")
		_check(game.tr("The shield absorbs shots; the Warden still retaliates. Reach the marked stabilizer to drain it.") in game.get("_guidance_label").text, "powered guidance survives Warden spawn and resume")
		_check(game.get("_sound_captions").position.x == game.get("_guide_panel").position.x, "combat captions leave the arena visible at 150 percent")
		await _capture("breach-powered-" + mode + ".png")
		await _walk([Vector3i(8, 0, 0)])
		game.call("_send_message", {"type": "MoveIntent", "position": [9, 0, 0]})
		await create_timer(0.2).timeout
		_check(state.get("actors")[state.get("player_actor_id")].position == [8, 0, 0], "core cannot be bypassed")
		await _walk([Vector3i(3, 0, 0), Vector3i(3, 0, 3)])
		_check(await _until(func() -> bool: return _checkpoint() == 2), "relay drained")
		_check(not game.get("_breach_site").get("_shield").visible, "shield disappears only at accepted drain")
		if mode == modes[0]:
			await _resume_at(2, "breach_stabilizer", true)
			_check(not game.get("_breach_site").get("_shield").visible, "drained shield remains absent on resume")
		_check(game.tr("Relay drained. The Warden is vulnerable. Defeat it to unlock the core threshold.") in game.get("_guidance_label").text, "drained guidance survives Warden spawn and resume")
		await _capture("breach-drained-" + mode + ".png")
		_check(await game.call("_drive_validation_attacks", false, 7500), "vulnerable Warden defeated")
		_check(await _until(func() -> bool: return _checkpoint() == 3), "guard clear saved")
		_check(_balance() == fragments and not state.get("activity_complete"), "guard death has no separate reward or terminal")
		if mode == modes[0]:
			await _resume_at(3, "breach_guard", false)
			_check(game.get("_current_enemy_id") == 0, "cleared guard stays absent")
		await _walk([Vector3i(10, 0, 0)])
		_check(await _until(func() -> bool: return state.get("activity_complete") and _cleared() == 5), "core crossing completes chapter five")
		var grant := 1 if mode == "campaign" else 0
		_check(_balance() == fragments + grant and state.get("progression").get("experience") == xp + 100 * grant, "exact first clear and no practice grant")
		await _capture("breach-complete-" + mode + ".png")
		game.call("_return_to_campaign_menu")
		await create_timer(0.35).timeout
	if failures.is_empty():
		print("M36 breach passed: ordered relay, shield, blocked core, three reconnects, fifth first-clear and unrewarded practice")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _resume_at(checkpoint: int, completed: String, enemy_expected: bool) -> void:
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == checkpoint and _objective(completed) == "Completed" and (not enemy_expected or game.get("_current_enemy_id") != 0)), "breach boundary restored: %d" % checkpoint)
