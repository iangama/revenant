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
	_check(campaign_before.get("cleared_chapters") == 6, "old completed campaign loaded")
	await _join_challenge("challenges")
	var board: Dictionary = game.get("_entry_shell").get("_challenge_snapshot")
	_check(board.get("contracts", []).any(func(c: Dictionary) -> bool: return c.contract_id == "bastion_link"), "current board advertises Bastion")
	game.get("_entry_shell").call("select_mode", "challenge_bastion_link")
	await _capture("bastion-entry-pt150.png")
	await _join_challenge("challenge_bastion_link")
	if not await _until(func() -> bool: return game.call("_enemy_ids").size() == 2):
		_finish("elite pair missing")
		return
	_check(game.get("_camera_phase") == "challenge_bastion", "elite arena camera selected")
	_check(_mask() == 0, "fresh elite attempt")
	_check(not game.get("_module_button").visible, "loadout remains fixed during challenge")
	var snapshot: Dictionary = game.get("_challenge_state").snapshot.duplicate(true)
	var invalid := snapshot.duplicate(true)
	invalid.policy_revision = "m37-challenges-v2"
	invalid.contracts = invalid.contracts.slice(0, 3)
	_check(not game.get("_challenge_state").apply(invalid), "old policy cannot impersonate elite run")
	invalid = snapshot.duplicate(true)
	invalid.active.contract.gameplay_revision = "m37-baseline-v1"
	_check(not game.get("_challenge_state").apply(invalid), "elite gameplay identity is checked")
	await _capture("bastion-active-pt150.png")
	if OS.get_environment("REVENANT_BASTION_HUD_ONLY") == "1":
		settings.language = "en"
		game.call("_apply_settings", settings, false)
		game.call("_refresh_challenge_guidance")
		await _capture("bastion-active-en150.png")
		await _back_to_board()
		await _capture("bastion-records-en150.png")
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	await _walk([Vector3i(6, 0, -6)])
	await _defeat_pair(true)
	_check(await _until(func() -> bool: return state.get("activity_complete")), "elite challenge completed")
	_check(_has_record(game.get("_challenge_state").snapshot, "bastion_link"), "elite completion saved")
	_check(_balance() == 6 and state.get("progression").experience == 600, "six fragments and 600 XP preserved")
	await _capture("bastion-complete-pt150.png")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	game.call("_refresh_challenge_guidance")
	await _capture("bastion-complete-en150.png")
	await _back_to_board()
	board = game.get("_entry_shell").get("_challenge_snapshot")
	for contract in ["coolant_recovery", "meridian_circuit", "bastion_link"]:
		_check(_has_record(board, contract), "record survives reconnect: " + contract)
	await _capture("bastion-records-en150.png")
	_check(await _read_campaign() == campaign_before, "campaign and story unchanged")
	if failures.is_empty(): print("M37 Bastion passed: four-contract board, elite target/flank controls, completion/reconnect, unchanged old campaign/items/XP, PT/EN150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _has_record(board: Dictionary, contract: String) -> bool:
	for record in board.get("records", []):
		if record.contract.contract_id == contract: return true
	return false
