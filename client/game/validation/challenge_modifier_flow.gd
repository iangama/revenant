extends "res://validation/challenge_gauntlet_flow.gd"


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
	var before: Dictionary = game.get("_entry_shell").get("_challenge_snapshot").duplicate(true)
	_check(before.policy_revision == "m37-challenges-v8" and before.variants.size() == 8, "v8 closed variant catalog")
	game.get("_entry_shell").call("select_mode", "challenge_meridian_circuit@west_approach_pace")
	await _capture("modifiers-menu-pt150.png")
	await _join_challenge("challenge_meridian_circuit@west_approach_pace")
	_check(state.get("actors")[state.get("player_actor_id")].position == [-31, 0, 0], "western entry")
	await _walk([Vector3i(-30,0,0), Vector3i(-30,0,-7), Vector3i(-28,0,-7), Vector3i(-30,0,-7), Vector3i(-30,0,7), Vector3i(-28,0,7)])
	await _capture("modifiers-west-return-pt150.png")
	await _walk([Vector3i(-30,0,7), Vector3i(-30,0,0), Vector3i(-31,0,0)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "modified route completes")
	_check(game.get("_challenge_state").snapshot.last_result.run.elapsed_ms != null, "route time saved")
	await _capture("modifiers-route-complete-pt150.png")
	await _back_to_board()
	await _join_challenge("challenge_relay_gauntlet@bastion_first_single_reserve")
	_check(await _until(func() -> bool: return _saved().get("stage") == 2 and game.call("_enemy_ids").size() == 2), "Bastion starts first")
	await _capture("modifiers-bastion-first-pt150.png")
	await _pair(true)
	await _next_modified(2, 1)
	await _support()
	await _use_reserve(1)
	await _capture("modifiers-reserve-pt150.png")
	await _next_modified(1, 3)
	await _prism_stage(settings)
	_check(await _until(func() -> bool: return state.get("activity_complete")), "reversed gauntlet completes")
	await _back_to_board()
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	await _join_challenge("challenge_relay_gauntlet@single_reserve_pace")
	_check(await _until(func() -> bool: return _saved().get("stage") == 1 and game.call("_enemy_ids").size() == 2), "timed reserve gauntlet starts fresh")
	_check(not _saved().modifier.reserve_used, "new run has a fresh reserve")
	await _support()
	await _use_reserve(1)
	await _capture("modifiers-reserve-time-en150.png")
	await _next_modified(1, 2)
	await _pair(true)
	await _next_modified(2, 3)
	await _prism_stage(settings)
	_check(await _until(func() -> bool: return state.get("activity_complete")), "timed gauntlet completes")
	_check(game.get("_challenge_state").snapshot.last_result.run.elapsed_ms != null, "gauntlet time saved")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	await _capture("modifiers-gauntlet-time-result-en150.png")
	await _back_to_board()
	var after: Dictionary = game.get("_entry_shell").get("_challenge_snapshot")
	for record in before.records:
		if not record.contract.has("preset_id"): _check(record in after.records, "baseline record unchanged")
	for preset in ["west_approach_pace", "bastion_first_single_reserve", "single_reserve_pace"]:
		_check(after.records.any(func(r: Dictionary) -> bool: return r.contract.get("preset_id") == preset), "modified record reconnect: " + preset)
	var campaign_after := await _read_campaign()
	_check(campaign_after == campaign_before, "campaign unchanged")
	if failures.is_empty(): print("M37 modifiers passed: four families, three allowed pairs, west return, reversed stages, optional single recovery, saved timing/records and unchanged campaign, PT/EN150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _next_modified(stage: int, next: int) -> void:
	var health := _health()
	await _walk([Vector3i(2,0,6) if stage == 1 else Vector3i(5,0,-6)])
	_check(await _until(func() -> bool: return _saved().get("stage") == next and _saved().get("phase") == "combat" and not game.call("_enemy_ids").is_empty()), "modified stage transfer")
	_check(_health() == health, "single-reserve transfer does not heal")


func _use_reserve(stage: int) -> void:
	var health := _health()
	_check(health < 100, "reserve trial carries damage")
	await _walk([Vector3i(3,0,6) if stage == 1 else Vector3i(6,0,-6)])
	_check(await _until(func() -> bool: return _saved().get("modifier", {}).get("reserve_used", false)), "reserve hold confirmed")
	_check(_health() == mini(100, health + 24), "reserve recovery capped")


func _support() -> void:
	await _walk([Vector3i(5,0,6), Vector3i(5,0,8)])
	_check(await _until(func() -> bool: return _health() < 100), "modified run carries damage")
	await _pair(false)
	_check(await _until(func() -> bool: return _saved().get("phase") == "transfer" and _saved().get("stage") == 1), "support cleared at its selected stage")
