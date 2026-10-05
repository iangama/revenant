extends "res://validation/campaign_flow.gd"


func _run() -> void:
	username = OS.get_environment("REVENANT_CAMPAIGN_FIXTURE_USER")
	if username.is_empty():
		push_error("REVENANT_CAMPAIGN_FIXTURE_USER must name a three-chapter fixture")
		quit(1)
		return
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var modes := ["practice_counter_signal"] if OS.get_environment("REVENANT_COUNTER_PRACTICE_ONLY") == "1" else ["campaign", "practice_counter_signal"]
	for mode in modes:
		game.get("_entry_shell").call("select_mode", mode)
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return not game.get("_entry_shell").visible and _checkpoint() == 0 and game.get("_campaign_chapter") == "counter_signal"):
			_finish("counter chapter unavailable")
			return
		_check_campaign_controls()
		var fragments := _balance()
		var xp: int = state.get("progression").get("experience")
		await _walk([Vector3i(0, 0, 6), Vector3i(2, 0, 6)])
		_check(await _until(func() -> bool: return _checkpoint() == 1 and game.call("_enemy_ids").size() == 2), "support checkpoint")
		if mode == modes[0]:
			await _resume_at(1, "counter_approach")
			_check(await _until(func() -> bool: return game.call("_enemy_ids").size() == 2), "interrupted support pair restarts")
			var player: int = state.get("player_actor_id")
			game.call("_send_message", {"type": "MoveIntent", "position": [1, 0, 6]})
			await create_timer(0.2).timeout
			_check(state.get("actors")[player].position == [2, 0, 6], "active pair keeps arena boundary")
		await _walk([Vector3i(5, 0, 7)])
		await _capture("counter-support-" + mode + ".png")
		await _defeat_pair(false)
		_check(await _until(func() -> bool: return _checkpoint() == 2), "support pair saved")
		_check(_balance() == fragments, "support pair has no independent reward")
		if mode == modes[0]:
			await _resume_at(2, "counter_support")
			_check(game.call("_enemy_ids").is_empty(), "cleared support stays absent")
		await _walk([Vector3i(7, 0, 6)])
		_check(await _until(func() -> bool: return _checkpoint() == 3), "transmission decoded")
		await _capture("counter-reversal-" + mode + ".png")
		if mode == modes[0]:
			await _resume_at(3, "counter_transmission")
		await _walk([Vector3i(4, 0, 6), Vector3i(4, 0, -6), Vector3i(5, 0, -6)])
		_check(await _until(func() -> bool: return _checkpoint() == 4 and game.call("_enemy_ids").size() == 2), "Bastion checkpoint")
		if mode == modes[0]:
			await _resume_at(4, "counter_bastion")
			_check(await _until(func() -> bool: return game.call("_enemy_ids").size() == 2), "interrupted elite pair restarts")
		await _walk([Vector3i(7, 0, -6)])
		await _capture("counter-elite-" + mode + ".png")
		await _defeat_pair(true)
		_check(await _until(func() -> bool: return _checkpoint() == 5), "elite pair saved")
		_check(_balance() == fragments, "elite pair has no independent reward")
		if mode == modes[0]:
			await _resume_at(5, "counter_guard")
			_check(game.call("_enemy_ids").is_empty(), "cleared elite stays absent")
		await _walk([Vector3i(9, 0, -6)])
		_check(await _until(func() -> bool: return state.get("activity_complete") and _cleared() == 4), "counter-signal isolated")
		var grant := 1 if mode == "campaign" else 0
		_check(_balance() == fragments + grant and state.get("progression").get("experience") == xp + 100 * grant, "exact first clear or unrewarded practice")
		await _capture("counter-complete-" + mode + ".png")
		game.call("_return_to_campaign_menu")
		await create_timer(0.35).timeout
	if failures.is_empty():
		print("M36 counter passed: two pairs, six boundaries, five reconnects, ordered transmission, exact first clear and unrewarded practice")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _resume_at(checkpoint: int, completed: String) -> void:
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == checkpoint and _objective(completed) == "Completed"), "counter boundary restored: %d" % checkpoint)

