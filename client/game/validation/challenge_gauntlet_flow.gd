extends "res://validation/challenge_signal_flow.gd"


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
	var prior_records: Array = game.get("_entry_shell").get("_challenge_snapshot").records.duplicate(true)
	_check(game.get("_entry_shell").get("_challenge_snapshot").contracts.size() == 8, "eight-contract board")
	game.get("_entry_shell").call("select_mode", "challenge_relay_gauntlet")
	await _capture("gauntlet-entry-pt150.png")
	await _join_gauntlet()
	var initial: Dictionary = state.get("gauntlet_state").duplicate(true)
	_check_gauntlet_projection(initial)
	_check(not game.get("_module_button").visible and not game.get("_route_button").visible, "equipment and routes remain locked")
	await _capture("gauntlet-support-pt150.png")
	await _support()
	await _capture("gauntlet-transfer-pt150.png")
	await _walk([Vector3i(2, 0, 6)])
	await create_timer(0.3).timeout
	await _walk([Vector3i(3, 0, 6)])
	_check(await _until(func() -> bool: return _saved().transfer_remaining_ms == null), "leaving transfer resets the hold")
	var damaged := _health()
	await _walk([Vector3i(2, 0, 6)])
	await create_timer(0.4).timeout
	_check(_saved().stage == 1 and _health() == damaged, "no predicted transfer or healing")
	_check(await _until(func() -> bool: return _saved().stage == 2), "transfer confirmed")
	_check(_health() == mini(100, damaged + 24), "capped transfer recovery")
	_check(await _until(func() -> bool: return game.call("_enemy_ids").size() == 2), "Bastion pair arrived")
	await _capture("gauntlet-bastion-pt150.png")
	await _back_to_board()
	await _join_gauntlet()
	_check(_saved().run_id > initial.run_id and _saved().stage == 1 and _health() == 100 and _mask() == 0, "retry resets the whole gauntlet")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	await _capture("gauntlet-support-en150.png")
	await _support()
	await _transfer(1, Vector3i(2, 0, 6))
	await _walk([Vector3i(10, 0, -6), Vector3i(10, 0, -8)])
	await _pair(true)
	_check(await _until(func() -> bool: return _saved().phase == "transfer" and _mask() == 3), "Bastion stage confirmed")
	await _capture("gauntlet-transfer-en150.png")
	await _transfer(2, Vector3i(5, 0, -6))
	await _prism_stage(settings)
	_check(await _until(func() -> bool: return state.get("activity_complete") and _saved().phase == "completed"), "all three stages complete")
	_check(game.get("_challenge_state").snapshot.last_result.outcome == "completed", "saved successful terminal")
	_check(_balance() == 6 and state.get("progression").experience == 600, "no items or XP")
	_check(_saved().recovered_health == 0, "no final healing")
	await _capture("gauntlet-complete-pt150.png")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	await _capture("gauntlet-complete-en150.png")
	await _back_to_board()
	var records: Array = game.get("_entry_shell").get("_challenge_snapshot").records
	_check(records.any(func(r: Dictionary) -> bool: return r.contract.contract_id == "relay_gauntlet" and r.objectives == 7), "gauntlet record survives reconnect")
	for record in prior_records:
		_check(record in records, "prior record retained: " + str(record.contract.contract_id))
	await _capture("gauntlet-records-en150.png")
	_check(await _read_campaign() == campaign_before, "campaign and story unchanged")
	if failures.is_empty(): print("M37 Relay Gauntlet passed: eight contracts, interrupted hold, capped recovery, full retry, three stages/two Prism phases, saved record/reconnect, unchanged campaign and 6frag/600XP, PT/EN150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _join_gauntlet() -> void:
	await _join_challenge("challenge_relay_gauntlet")
	_check(await _until(func() -> bool: return not _saved().is_empty() and game.call("_enemy_ids").size() == 2), "gauntlet admitted")


func _saved() -> Dictionary:
	return state.get("gauntlet_state")


func _health() -> int:
	return state.get("actor_health")[state.get("player_actor_id")]


func _mode() -> String:
	return state.get("prism_state").get("mode", "")


func _support() -> void:
	await _walk([Vector3i(5, 0, 6), Vector3i(5, 0, 8)])
	_check(await _until(func() -> bool: return _health() < 100), "damage carried into transfer")
	await _pair(false)
	_check(await _until(func() -> bool: return _saved().phase == "transfer" and _mask() == 1), "support stage confirmed")
	_check(game.get("_gauntlet_site").visible and not game.get("_attack_button").visible, "transfer marker replaces attack control")


func _pair(elite: bool) -> void:
	for archetype in ["relay-mender", "steel-bulwark" if elite else "glass-lancer"]:
		var target := 0
		for id in game.call("_enemy_ids"):
			if state.get("actors")[id].archetype == archetype: target = id
		_check(target != 0, "target present: " + archetype)
		game.set("_current_enemy_id", target)
		game.call("_refresh_selected_enemy")
		if elite and archetype == "steel-bulwark": await _walk([Vector3i(10, 0, -6)])
		var deadline := Time.get_ticks_msec() + 35000
		while state.get("actors").has(target) and Time.get_ticks_msec() < deadline:
			if elite and archetype == "steel-bulwark" and game.get("_actors")[target].call("presentation_state").braced:
				await process_frame
				continue
			await game.call("_tap_validation_key", KEY_SPACE)
			await create_timer(0.36).timeout
		if state.get("actors").has(target):
			print("Remaining target ", archetype, " health ", state.get("actor_health").get(target), " inputs ", game.get("_input_events"))
			_finish("target defeated: " + archetype)
			return


func _transfer(stage: int, entrance: Vector3i) -> void:
	var before := _health()
	await _walk([entrance])
	_check(await _until(func() -> bool: return _saved().stage == stage + 1), "next stage confirmed")
	_check(_health() == mini(100, before + 24), "stage recovery capped")


func _prism_stage(settings: Dictionary) -> void:
	if not await _until(func() -> bool: return not state.get("prism_state").is_empty() and game.get("_current_enemy_id") != 0):
		_finish("Prism actor did not arrive after transfer")
		return
	var target: int = game.get("_current_enemy_id")
	var captured := {}
	var deadline := Time.get_ticks_msec() + 90000
	while state.get("actors").has(target) and _health() > 0 and Time.get_ticks_msec() < deadline:
		var mode := _mode()
		if mode == "Warning":
			var pattern: Dictionary = state.get("prism_state").pattern
			var player: Array = state.get("actors")[state.get("player_actor_id")].position
			var safe := Vector3i(player[0], 0, player[2])
			match pattern.shape:
				"AcrossX": safe.z = int(pattern.z) + (1 if int(pattern.z) < 3 else -1)
				"AcrossZ": safe.x = int(pattern.x) + (1 if int(pattern.x) < 11 else -1)
				"Center": safe = Vector3i(6, 0, 0)
				"Perimeter": safe = Vector3i(8, 0, 0)
			await _walk([safe])
			if not captured.has(pattern.shape):
				captured[pattern.shape] = true
				if pattern.shape == "Center":
					settings.language = "pt_BR"
					game.call("_apply_settings", settings, false)
				await _capture("gauntlet-prism-%s.png" % pattern.shape)
			await _until(func() -> bool: return _mode() != "Warning")
		elif mode == "Recovery":
			var before: int = state.get("actor_health").get(target, 0)
			await game.call("_tap_validation_key", KEY_SPACE)
			# Storage/transport can span the end of an opening. Observe the
			# next warning instead of requiring four hits in one recovery window.
			await _until(func() -> bool: return state.get("actor_health").get(target, 0) < before or _mode() != "Recovery")
			await create_timer(float(game.call("_current_attack_cooldown_ms") + 60) / 1000.0).timeout
		else:
			await process_frame
	_check(captured.has("Center") and captured.has("Perimeter"), "both Prism pulse patterns played")
	_check(not state.get("actors").has(target), "Prism defeated across confirmed openings")


func _check_gauntlet_projection(initial: Dictionary) -> void:
	var projection := preload("res://projection/authoritative_state.gd").new()
	for key in ["player_actor_id", "actors", "actor_health", "actor_max_health"]:
		var value: Variant = state.get(key)
		projection.set(key, value.duplicate(true) if value is Dictionary else value)
	_check(projection.apply_gauntlet_state(initial, initial.run_id), "initial gauntlet projection")
	_check(projection.apply_gauntlet_state(initial, initial.run_id), "duplicate projection is harmless")
	for field in ["run_id", "player_actor_id", "stage", "health", "recovered_health"]:
		var forged := initial.duplicate(true)
		forged.sequence += 1
		forged[field] += 1
		_check(not projection.apply_gauntlet_state(forged, initial.run_id), "forged gauntlet field rejected: " + field)
