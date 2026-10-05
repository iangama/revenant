extends "res://validation/challenge_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m36_campaign_1790156347"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true}, true)
	game.call("_apply_settings", settings, false)
	var campaign_before := await _read_campaign()
	_check(campaign_before.get("cleared_chapters") == 6, "existing completed campaign loaded")
	await _join_challenge("challenges")
	var board: Dictionary = game.get("_entry_shell").get("_challenge_snapshot")
	_check(board.get("contracts", []).any(func(c: Dictionary) -> bool: return c.contract_id == "coolant_recovery"), "current board advertises recovery")
	game.get("_entry_shell").call("select_mode", "challenge_coolant_recovery")
	await _capture("recovery-entry-pt150.png")
	await _join_challenge("challenge_coolant_recovery")
	if _mask() != 0:
		_finish("recovery did not start from intake")
		return
	var initial: Dictionary = game.get("_challenge_state").snapshot.duplicate(true)
	_check_projection(initial)
	_check(game.get("_camera_phase") == "coolant_recovery", "recovery camera selected at spawn")
	_check(not game.get("_attack_button").visible and not game.get("_module_button").visible, "recovery hides combat and equipment controls")
	_check(_balance() == 6 and state.get("progression").experience == 600, "existing inventory and XP retained")
	await _capture("recovery-intake-pt150.png")
	for label in game.get("_coolant_site").get("_labels"):
		_check("8 SECOND" not in label.text and "8 SEGUNDO" not in label.text, "recovery station has no old deadline")
	if OS.get_environment("REVENANT_RECOVERY_HUD_ONLY") == "1":
		await _walk([Vector3i(-1, 0, -5)])
		_check(await _until(func() -> bool: return _mask() == 1), "retrieval confirmed for HUD review")
		await _capture("recovery-transfer-pt150.png")
		settings.language = "en"
		game.call("_apply_settings", settings, false)
		await _capture("recovery-transfer-en150.png")
		await _back_to_board()
		if failures.is_empty(): print("M37 recovery HUD passed: entry, intake and transfer PT/EN150")
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	await _walk([Vector3i(-1, 0, -5)])
	_check(await _until(func() -> bool: return _mask() == 1), "retrieval confirmed")
	await create_timer(8.3).timeout
	_check(_mask() == 1, "recovery has no eight-second expiration")
	await _walk([Vector3i(4, 0, -5)])
	await create_timer(0.2).timeout
	await _walk([Vector3i(3, 0, -5)])
	await create_timer(0.45).timeout
	_check(_mask() == 1, "leaving transfer cancels the hold")
	await _capture("recovery-transfer-pt150.png")
	await _walk([Vector3i(4, 0, -5)])
	await create_timer(0.55).timeout
	_check(_mask() == 1, "return requires a new full hold")
	_check(await _until(func() -> bool: return _mask() == 3), "server confirms uninterrupted transfer")
	await _back_to_board()
	await _join_challenge("challenge_coolant_recovery")
	_check(_mask() == 0, "abandon and retry clears the ordered checkpoints")
	_check(game.get("_challenge_state").snapshot.active.run_id > initial.active.run_id, "retry has a new attempt identity")
	_check(state.get("actors")[state.get("player_actor_id")].position == [-1, 0, -3], "retry returns to intake spawn")
	await _walk([Vector3i(-1, 0, -5), Vector3i(4, 0, -5)])
	_check(await _until(func() -> bool: return _mask() == 3), "fresh transfer confirmed")
	await _capture("recovery-delivery-pt150.png")
	await _walk([Vector3i(4, 0, -1)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "delivery completes the contract")
	_check(game.get("_challenge_state").snapshot.last_result.outcome == "completed", "server saved successful terminal")
	_check(_balance() == 6 and state.get("progression").experience == 600, "no additional items or XP")
	await _capture("recovery-complete-pt150.png")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	game.call("_refresh_challenge_guidance")
	await _capture("recovery-complete-en150.png")
	await _back_to_board()
	board = game.get("_entry_shell").get("_challenge_snapshot")
	var record_found := false
	for record in board.records:
		if record.contract.contract_id == "coolant_recovery": record_found = true
	_check(record_found, "reconnected board retains the recovery record")
	await _capture("recovery-records-en150.png")
	_check(await _read_campaign() == campaign_before, "existing campaign and story unchanged")
	_finish()


func _read_campaign() -> Dictionary:
	var session: Node = game.get("_session")
	session.set("entry_mode", "campaign")
	var joined: Dictionary = await session.call("join_initial_session", username, "127.0.0.1", int(OS.get_environment("REVENANT_GAME_PORT")))
	_check(joined.get("campaign_complete", false), "campaign query returns without opening a run")
	return joined.get("campaign_snapshot", {}).duplicate(true)


func _back_to_board() -> void:
	game.call("_return_to_campaign_menu")
	await create_timer(0.3).timeout
	# Abandon, authentication, character listing and save verification are separate round trips.
	var deadline := Time.get_ticks_msec() + 15000
	while game.get("_connection_started") and Time.get_ticks_msec() < deadline:
		await process_frame
	_check(not game.get("_connection_started") and game.get("_entry_shell").visible, "board refresh finished")


func _mask() -> int:
	var active: Variant = game.get("_challenge_state").snapshot.get("active")
	return int(active.objectives) if active is Dictionary else -1


func _check_projection(original: Dictionary) -> void:
	var projection := preload("res://projection/challenge_state.gd").new()
	var old := original.duplicate(true)
	old.policy_revision = "m37-challenges-v1"
	old.contracts = old.contracts.slice(0, 2)
	old.active = null
	old.last_result = null
	old.records = []
	_check(projection.apply(old), "frozen v1 board still projects")
	_check(projection.apply(original), "expanded board upgrades the projection")
	var invalid := original.duplicate(true)
	invalid.active.objectives = 2
	_check(not projection.apply(invalid), "out-of-order recovery rejected")
	invalid.active.objectives = 7
	_check(not projection.apply(invalid), "completed objectives require a terminal result")
	invalid = original.duplicate(true)
	invalid.active.contract.gameplay_revision = "m37-baseline-v1"
	_check(not projection.apply(invalid), "old gameplay cannot impersonate recovery")
	invalid = original.duplicate(true)
	invalid.policy_revision = "m37-challenges-v1"
	invalid.contracts = invalid.contracts.slice(0, 2)
	_check(not projection.apply(invalid), "v1 policy cannot contain a recovery run")


func _finish(error := "") -> void:
	if not error.is_empty(): _check(false, error)
	if failures.is_empty(): print("M37 recovery passed: expanded/old projection, untimed holds, reset, abandon/retry, reconnect, unchanged old campaign/items/XP, PT/EN150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
