extends "res://validation/challenge_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m36_campaign_1790156347"
	await _join_challenge("challenge_meridian_circuit")
	var has_recovery := false
	for record in game.get("_challenge_state").snapshot.get("records", []):
		if record.contract.contract_id == "coolant_recovery": has_recovery = true
	_check(has_recovery, "fixture already has expanded recovery record")
	await _walk([Vector3i(-24, 0, 0), Vector3i(-24, 0, -7), Vector3i(-28, 0, -7), Vector3i(-30, 0, -7), Vector3i(-30, 0, 0), Vector3i(-30, 0, 7), Vector3i(-28, 0, 7), Vector3i(-24, 0, 7), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "Meridian completes after recovery")
	_check(game.get("_challenge_state").snapshot.records.size() == 2, "both contract records retained")
	_check(_balance() == 6 and state.get("progression").experience == 600, "inventory and XP unchanged")
	game.call("_return_to_campaign_menu")
	await create_timer(0.3).timeout
	_check(await _until(func() -> bool: return not game.get("_connection_started")), "final board refreshed")
	_check(game.get("_entry_shell").get("_challenge_snapshot").records.size() == 2, "both records survive reconnect")
	if failures.is_empty(): print("M37 regression passed: Meridian after recovery retains both records, six fragments and 600 XP")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
