extends "res://validation/challenge_recovery_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m36_campaign_1790156347"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true}, true)
	game.call("_apply_settings", settings, false)
	var campaign_before := await _read_campaign()
	await _join_challenge("challenges")
	_check(game.get("_entry_shell").get("_challenge_snapshot").contracts.size() == 6, "six-contract board")
	game.get("_entry_shell").call("select_mode", "challenge_distant_signal")
	await _capture("signal-entry-pt150.png")
	await _join_challenge("challenge_distant_signal")
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("signal enemy missing")
		return
	var original: Dictionary = game.get("_challenge_state").snapshot.duplicate(true)
	var enemy: int = game.get("_current_enemy_id")
	_check(game.get("_camera_phase") == "challenge_signal", "signal camera active")
	_check(not game.get("_inventory_panel").visible, "combat leaves the lane clear")
	_check(game.get("_signal_site").visible, "cover is visible")
	_check(not game.get("_signal_site").get("_terminal").visible, "no recovery terminal")
	await game.call("_tap_validation_key", KEY_SPACE)
	await create_timer(2).timeout
	_check(state.get("actor_health")[enemy] == 200, "cover blocks player shot")
	_check(state.get("actor_health")[state.get("player_actor_id")] == 100, "cover blocks sentinel shot")
	await _capture("signal-cover-pt150.png")
	if OS.get_environment("REVENANT_SIGNAL_HUD_ONLY") == "1":
		settings.language = "en"
		game.call("_apply_settings", settings, false)
		await _capture("signal-cover-en150.png")
		await _back_to_board()
		if failures.is_empty(): print("M37 signal presentation passed: compact combat and full lane, PT/EN150")
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	await _walk([Vector3i(-7, 0, 4), Vector3i(-7, 0, 1)])
	await game.call("_tap_validation_key", KEY_SPACE)
	_check(await _until(func() -> bool: return state.get("actor_health")[enemy] < 200), "flank opens rifle fire")
	await _capture("signal-flank-pt150.png")
	await _back_to_board()
	await _join_challenge("challenge_distant_signal")
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "retry enemy spawned")
	enemy = game.get("_current_enemy_id")
	_check(game.get("_challenge_state").snapshot.active.run_id > original.active.run_id, "fresh retry identity")
	_check(state.get("actor_health")[enemy] == 200, "retry restores enemy")
	_check(state.get("actors")[state.get("player_actor_id")].position == [-4, 0, 4], "retry restores spawn")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	await _capture("signal-cover-en150.png")
	await _walk([Vector3i(-7, 0, 4), Vector3i(-7, 0, 1)])
	for i in 5:
		var health_before: int = state.get("actor_health").get(enemy, 0)
		await game.call("_tap_validation_key", KEY_SPACE)
		if not await _until(func() -> bool: return state.get("actor_health").get(enemy, 0) < health_before):
			print("Unconfirmed signal shot ", i + 1, ": ", game.get("_input_events"))
			_finish("signal shot %d was not confirmed" % (i + 1))
			return
		if i == 0: await _capture("signal-flank-en150.png")
		# Space taps are intents, not hit acknowledgements. Wait after the
		# confirmed hit so transport/storage jitter cannot compress the cadence.
		await create_timer(float(game.call("_current_attack_cooldown_ms") + 80) / 1000.0).timeout
	_check(await _until(func() -> bool: return state.get("activity_complete")), "signal complete")
	_check(game.get("_challenge_state").snapshot.last_result.outcome == "completed", "server confirmed terminal")
	_check(_balance() == 6 and state.get("progression").experience == 600, "no items or XP")
	await _capture("signal-complete-en150.png")
	await _back_to_board()
	var records: Array = game.get("_entry_shell").get("_challenge_snapshot").records
	for contract in ["coolant_recovery", "meridian_circuit", "bastion_link", "distant_signal"]:
		_check(records.any(func(r: Dictionary) -> bool: return r.contract.contract_id == contract), "saved record: " + contract)
	_check(await _read_campaign() == campaign_before, "old campaign unchanged")
	if failures.is_empty(): print("M37 Distant Signal passed: cover/flank, full retry, completion/reconnect, old campaign and 6frag/600XP unchanged, PT/EN150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _capture(filename: String) -> void:
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty():
		await process_frame
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(directory.path_join(filename)) == OK, "capture saved")
