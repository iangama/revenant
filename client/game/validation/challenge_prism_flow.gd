extends "res://validation/challenge_recovery_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m36_campaign_1790156347"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var campaign_before := await _read_campaign()
	await _join_challenge("challenges")
	_check(game.get("_entry_shell").get("_challenge_snapshot").contracts.size() == 5, "five-contract board")
	game.get("_entry_shell").call("select_mode", "challenge_prism_discipline")
	await _capture("prism-entry-pt150.png")
	await _join_prism()
	if OS.get_environment("REVENANT_PRISM_CHALLENGE_HUD_ONLY") == "1":
		_check(await _until(func() -> bool: return _mode() == "Warning"), "HUD lane warning")
		await _capture("prism-lane-pt150.png")
		settings.language = "en"
		game.call("_apply_settings", settings, false)
		await _capture("prism-lane-en150.png")
		await _back_to_board()
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	game.call("_send_message", {"type": "MoveIntent", "position": [4, 0, 2]})
	await create_timer(0.15).timeout
	_check(state.get("actors")[state.get("player_actor_id")].position == [5, 0, 2], "arena exit uses the board")
	var original: Dictionary = game.get("_challenge_state").snapshot.duplicate(true)
	var invalid := original.duplicate(true)
	invalid.policy_revision = "m37-challenges-v3"
	invalid.contracts = invalid.contracts.slice(0, 4)
	_check(not game.get("_challenge_state").apply(invalid), "old policy cannot contain Prism")
	await _phase_one()
	await _capture("prism-shift-pt150.png")
	await _back_to_board()
	await _join_prism()
	_check(game.get("_challenge_state").snapshot.active.run_id > original.active.run_id, "retry has a new identity")
	_check(state.get("actor_health")[game.get("_current_enemy_id")] == 320, "retry restores the whole boss")
	_check(state.get("prism_state").phase == "Lanes", "retry restores phase one")
	await _phase_one()
	_check(await _until(func() -> bool: return _mode() == "Warning"), "center warning begins")
	await _walk([Vector3i(6, 0, 0)])
	_check(state.get("prism_state").pattern.shape == "Center", "center pulse confirmed")
	await _capture("prism-center-pt150.png")
	_check(await _until(func() -> bool: return _mode() == "PulseGap"), "center pulse resolves")
	await _walk([Vector3i(7, 0, 0)])
	_check(await _until(func() -> bool: return _mode() == "Warning"), "perimeter warning begins")
	_check(state.get("prism_state").pattern.shape == "Perimeter", "perimeter pulse confirmed")
	await _capture("prism-perimeter-pt150.png")
	_check(await _until(func() -> bool: return _mode() == "Recovery"), "both pulses open the shell")
	for index in 4:
		await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.29).timeout
	_check(await _until(func() -> bool: return state.get("activity_complete")), "challenge complete")
	_check(game.get("_challenge_state").snapshot.last_result.outcome == "completed", "server confirmed victory")
	_check(_balance() == 6 and state.get("progression").experience == 600, "no additional fragments or XP")
	await _capture("prism-complete-pt150.png")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	await _capture("prism-complete-en150.png")
	await _back_to_board()
	var records: Array = game.get("_entry_shell").get("_challenge_snapshot").records
	for contract in ["coolant_recovery", "meridian_circuit", "bastion_link", "prism_discipline"]:
		_check(records.any(func(r: Dictionary) -> bool: return r.contract.contract_id == contract), "record survives reconnect: " + contract)
	await _capture("prism-records-en150.png")
	_check(await _read_campaign() == campaign_before, "campaign and story unchanged")
	if failures.is_empty(): print("M37 Prism passed: five contracts, two phases, fresh retry after shift, saved victory/reconnect, unchanged campaign/items/XP, PT/EN150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _join_prism() -> void:
	await _join_challenge("challenge_prism_discipline")
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and not state.get("prism_state").is_empty()), "Prism admitted")
	_check(not game.get("_prism_site").visible, "standalone retreat instruction hidden")
	_check(_mask() == 0, "fresh challenge progress")


func _mode() -> String:
	return state.get("prism_state").get("mode", "")


func _phase_one() -> void:
	_check(await _until(func() -> bool: return _mode() == "Warning"), "lane warning starts")
	_check(game.tr("Step off the stripe. Attack when the shell opens. The board restarts the whole fight.") in game.get("_guidance_label").text, "guidance explains challenge restart")
	await _capture("prism-lane-pt150.png")
	await _walk([Vector3i(5, 0, 1)])
	_check(await _until(func() -> bool: return _mode() == "Recovery"), "sidestep opens shell")
	for index in 4:
		await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.29).timeout
	_check(await _until(func() -> bool: return _mode() == "Shifting"), "phase two begins")
	_check(state.get("actor_health")[game.get("_current_enemy_id")] == 160, "half-health floor preserved")
