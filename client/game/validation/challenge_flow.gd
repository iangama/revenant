extends "res://validation/campaign_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m37_godot_%d" % Time.get_unix_time_from_system()
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true}, true)
	game.call("_apply_settings", settings, false)
	await _join_challenge("challenges")
	_check(await _until(func() -> bool: return not game.get("_connection_started")), "board returns without world entry")
	_check(not game.get("_hud_canvas").visible, "board is read-only")
	await _capture("challenge-board-pt150.png")
	await _join_challenge("challenge_meridian_circuit")
	if not await _until(func() -> bool: return not game.get("_challenge_state").snapshot.is_empty()):
		_done("Meridian not admitted")
		return
	_check(not game.get("_module_button").visible and not game.get("_route_button").visible, "equipment and other activities hidden")
	var original: Dictionary = game.get("_challenge_state").snapshot.duplicate(true)
	var invalid := original.duplicate(true)
	invalid.character_id = "foreign"
	_check(not game.get("_challenge_state").apply(invalid), "foreign snapshot rejected")
	invalid = original.duplicate(true)
	invalid.active.objectives = 99
	_check(not game.get("_challenge_state").apply(invalid), "invalid progress rejected")
	await _walk([Vector3i(-24, 0, 0), Vector3i(-24, 0, -7), Vector3i(-28, 0, -7)])
	_check(await _until(func() -> bool: return game.get("_challenge_state").snapshot.active.objectives == 1), "lens confirmed")
	await _capture("challenge-meridian-active-pt150.png")
	if OS.get_environment("REVENANT_CHALLENGE_HUD_ONLY") == "1":
		_check(state.get("objectives").meridian_return.state == "Pending", "return waits for all readings")
		_check(not game.get("_attack_button").visible and not game.get("_enemy_health_label").visible, "exploration hides combat controls")
		await _walk([Vector3i(-30, 0, -7), Vector3i(-30, 0, 0), Vector3i(-30, 0, 7), Vector3i(-28, 0, 7)])
		_check(await _until(func() -> bool: return state.get("objectives").meridian_return.state == "Active"), "three readings enable the return marker")
		await _capture("challenge-meridian-return-pt150.png")
		game.call("_return_to_campaign_menu")
		await create_timer(0.25).timeout
		_check(await _until(func() -> bool: return not game.get("_connection_started")), "HUD review abandons attempt")
		if failures.is_empty(): print("M37 focused Meridian HUD passed: pending/active return, PT150 map and no combat controls")
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	game.call("_return_to_campaign_menu")
	await create_timer(0.25).timeout
	_check(await _until(func() -> bool: return not game.get("_connection_started") and game.get("_entry_shell").visible), "abandon opens refreshed board")
	await _join_challenge("challenge_meridian_circuit")
	_check(await _until(func() -> bool: return not game.get("_challenge_state").snapshot.is_empty() and game.get("_challenge_state").snapshot.active != null), "retry admitted")
	_check(game.get("_challenge_state").snapshot.active.objectives == 0, "retry starts fresh")
	await _walk([Vector3i(-24, 0, 0), Vector3i(-24, 0, 7), Vector3i(-28, 0, 7), Vector3i(-30, 0, 7), Vector3i(-30, 0, 0), Vector3i(-30, 0, -7), Vector3i(-28, 0, -7), Vector3i(-24, 0, -7), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "Meridian completes")
	await _capture("challenge-meridian-complete-pt150.png")
	game.call("_return_to_campaign_menu")
	await create_timer(0.25).timeout
	_check(await _until(func() -> bool: return not game.get("_connection_started")), "completed attempt returns to board")
	await _join_challenge("challenge_close_quarters")
	if not await _until(func() -> bool: return game.call("_enemy_ids").size() == 2):
		_done("support pair missing")
		return
	await _walk([Vector3i(5, 0, 6), Vector3i(5, 0, 8)])
	await _capture("challenge-combat-pt150.png")
	for archetype in ["relay-mender", "glass-lancer"]:
		var target := 0
		for id in state.get("actors"):
			if state.get("actors")[id].get("archetype") == archetype: target = id
		game.set("_current_enemy_id", target)
		_check(await game.call("_drive_validation_attacks", false, 8500), "target defeated: " + archetype)
	_check(await _until(func() -> bool: return state.get("activity_complete")), "combat completes")
	_check(game.get("_challenge_state").snapshot.records.size() == 2, "both records saved")
	_check(_balance() == 0 and state.get("progression").experience == 0, "no items or XP")
	await _capture("challenge-combat-complete-pt150.png")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	game.call("_refresh_challenge_guidance")
	await _capture("challenge-combat-complete-en150.png")
	game.call("_return_to_campaign_menu")
	await create_timer(0.25).timeout
	_check(await _until(func() -> bool: return not game.get("_connection_started")), "final records refreshed")
	_check(game.get("_entry_shell").get("_challenge_snapshot").records.size() == 2, "records survive reconnect")
	await _capture("challenge-records-en150.png")
	_done()


func _join_challenge(mode: String) -> void:
	var previous: Dictionary = game.get("_challenge_state").snapshot
	var previous_run := 0
	if previous.get("active") is Dictionary: previous_run = int(previous.active.run_id)
	elif previous.get("last_result") is Dictionary: previous_run = int(previous.last_result.run.run_id)
	game.get("_entry_shell").call("select_mode", mode)
	await create_timer(0.4).timeout
	game.call("_begin_connection", username)
	if mode == "challenges":
		_check(await _until(func() -> bool: return not game.get("_connection_started")), "board handshake finished")
	else:
		_check(await _until(func() -> bool:
			var saved: Dictionary = game.get("_challenge_state").snapshot
			return not game.get("_entry_shell").visible and saved.get("active") is Dictionary and int(saved.active.run_id) > previous_run and saved.active.contract.contract_id == mode.trim_prefix("challenge_").split("@")[0]), "new contract identity admitted")


func _done(error := "") -> void:
	if not error.is_empty(): _check(false, error)
	if failures.is_empty(): print("M37 challenge flow passed: board, both contracts, abandon/retry, records, PT150 and no grants")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
