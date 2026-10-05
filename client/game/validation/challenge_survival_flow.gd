extends "res://validation/challenge_signal_flow.gd"


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
	_check(game.get("_entry_shell").get("_challenge_snapshot").contracts.size() == 7, "seven-contract board")
	game.get("_entry_shell").call("select_mode", "challenge_last_reserve")
	await _capture("survival-entry-pt150.png")
	await _join_challenge("challenge_last_reserve")
	if not await _until(func() -> bool: return not state.get("survival_state").is_empty()):
		_finish("survival state missing")
		return
	var initial: Dictionary = state.get("survival_state").duplicate(true)
	_check_survival_projection(initial)
	_check(not game.get("_attack_button").visible and not game.get("_enemy_health_label").visible, "weapons offline")
	_check(game.get("_signal_site").get("_survival_stations").visible, "relay and reserve stations visible")
	await game.call("_tap_validation_key", KEY_SPACE)
	await create_timer(2).timeout
	_check(_health() == 100, "cover protects while waiting")
	_check(state.get("actor_health")[game.get("_current_enemy_id")] == 200, "attack cannot damage sentinel")
	await _capture("survival-cover-pt150.png")
	await _walk([Vector3i(-8, 0, 4), Vector3i(-8, 0, 0)])
	await create_timer(0.9).timeout
	await _walk([Vector3i(-8, 0, 1)])
	_check(await _until(func() -> bool: return state.get("survival_state").charge_remaining_ms == null and _mask() == 0), "leaving relay resets hold")
	await _walk([Vector3i(-8, 0, 0)])
	await create_timer(1).timeout
	_check(_mask() == 0, "new hold is required")
	await _capture("survival-charge-pt150.png")
	_check(await _until(func() -> bool: return _mask() == 1), "west relay charged")
	await _reserve()
	await _capture("survival-reserve-pt150.png")
	await _back_to_board()
	await _join_challenge("challenge_last_reserve")
	_check(await _until(func() -> bool: return int(state.get("survival_state").get("run_id", 0)) > initial.run_id), "retry has new identity")
	_check(_health() == 100 and _mask() == 0 and not state.get("survival_state").reserve_used, "retry restores health, reserve and relays")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	await _capture("survival-cover-en150.png")
	await _walk([Vector3i(-8, 0, 4), Vector3i(-8, 0, 0)])
	_check(await _until(func() -> bool: return _mask() == 1), "fresh west relay charged")
	await _reserve()
	await _walk([Vector3i(0, 0, 5), Vector3i(0, 0, 0)])
	await _capture("survival-east-en150.png")
	_check(await _until(func() -> bool: return _mask() == 3), "east relay charged")
	await _walk([Vector3i(0, 0, 4), Vector3i(-4, 0, 4)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "returned alive")
	_check(game.get("_challenge_state").snapshot.last_result.outcome == "completed", "saved successful terminal")
	_check(_balance() == 6 and state.get("progression").experience == 600, "no items or XP")
	await _capture("survival-complete-en150.png")
	await _back_to_board()
	var records: Array = game.get("_entry_shell").get("_challenge_snapshot").records
	for contract in ["coolant_recovery", "meridian_circuit", "bastion_link", "distant_signal", "last_reserve"]:
		_check(records.any(func(r: Dictionary) -> bool: return r.contract.contract_id == contract), "saved record: " + contract)
	_check(await _read_campaign() == campaign_before, "old campaign unchanged")
	if failures.is_empty(): print("M37 Last Reserve passed: cover, interrupted holds, one-use healing, full retry, completion/reconnect, old campaign and 6frag/600XP, PT/EN150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _health() -> int:
	return state.get("actor_health")[state.get("player_actor_id")]


func _reserve() -> void:
	await _walk([Vector3i(-8, 0, 5), Vector3i(-4, 0, 5)])
	var before := _health()
	_check(before < 100, "exposure inflicted damage")
	_check(await _until(func() -> bool: return state.get("survival_state").reserve_used), "reserve use confirmed")
	_check(_health() == mini(100, before + 36), "bounded healing projected")
	await create_timer(1.3).timeout
	_check(_health() == mini(100, before + 36), "reserve cannot repeat")


func _check_survival_projection(initial: Dictionary) -> void:
	var projection := preload("res://projection/authoritative_state.gd").new()
	for key in ["player_actor_id", "actors", "actor_health", "actor_max_health"]:
		var value: Variant = state.get(key)
		projection.set(key, value.duplicate(true) if value is Dictionary else value)
	_check(projection.apply_survival_state(initial, initial.run_id), "initial survival projection")
	for field in ["run_id", "player_actor_id", "sequence", "health", "reserve_used"]:
		var forged := initial.duplicate(true)
		forged.sequence += 1
		forged[field] = true if field == "reserve_used" else forged[field] + 1
		_check(not projection.apply_survival_state(forged, initial.run_id), "forged survival field rejected: " + field)
