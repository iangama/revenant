extends "res://validation/campaign_flow.gd"

# Continue an accepted two-chapter fixture to exercise the historical save.
func _run() -> void:
	username = OS.get_environment("REVENANT_CAMPAIGN_FIXTURE_USER")
	if username.is_empty():
		push_error("REVENANT_CAMPAIGN_FIXTURE_USER must name a two-chapter fixture character")
		quit(1)
		return
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var modes := ["practice_broken_supply_line"] if OS.get_environment("REVENANT_SUPPLY_PRACTICE_ONLY") == "1" else ["campaign", "practice_broken_supply_line"]
	for mode in modes:
		game.get("_entry_shell").call("select_mode", mode)
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return not game.get("_entry_shell").visible and _checkpoint() == 0 and game.get("_campaign_chapter") == "broken_supply_line"):
			_finish("supply chapter unavailable")
			return
		_check_campaign_controls()
		_check(game.get("_current_enemy_id") == 0, "supply route starts before combat")
		await _capture("supply-route-" + mode + ".png")
		await _walk([Vector3i(0, 0, 4), Vector3i(-4, 0, 4)])
		_check(await _until(func() -> bool: return _checkpoint() == 1 and game.get("_current_enemy_id") != 0), "supply approach checkpoint")
		if mode == "campaign" or modes.size() == 1:
			var player_id: int = state.get("player_actor_id")
			var enemy_id: int = game.get("_current_enemy_id")
			var health: int = state.get("actor_health")[player_id]
			game.call("_send_message", {"type": "AttackIntent", "target_actor_id": enemy_id})
			game.call("_send_message", {"type": "MoveIntent", "position": [-4, 0, 0]})
			await create_timer(2).timeout
			_check(state.get("actor_health")[enemy_id] == 200 and state.get("actor_health")[player_id] == health, "cover blocks both sides")
			_check(state.get("actors")[player_id].position == [-4, 0, 4], "invalid crossing stays rejected")
			await _rejoin()
			_check(await _until(func() -> bool: return _checkpoint() == 1 and game.get("_current_enemy_id") != 0), "guard encounter resumes")
		await _walk([Vector3i(-7, 0, 4), Vector3i(-7, 0, 0)])
		await _capture("supply-flank-" + mode + ".png")
		_check(await game.call("_drive_validation_attacks", false, 7000), "supply sentinel defeated")
		_check(await _until(func() -> bool: return _checkpoint() == 2), "guard checkpoint confirmed")
		if mode == "campaign" or modes.size() == 1:
			await _rejoin()
			_check(await _until(func() -> bool: return _checkpoint() == 2 and _objective("supply_guard") == "Completed"), "guard defeat restored without respawn")
			_check(game.get("_current_enemy_id") == 0, "cleared guard remains absent")
		await _walk([Vector3i(-7, 0, -3), Vector3i(-4, 0, -3)])
		_check(await _until(func() -> bool: return _checkpoint() == 3), "cell checkpoint confirmed")
		if mode == "campaign" or modes.size() == 1:
			await _rejoin()
			_check(await _until(func() -> bool: return _checkpoint() == 3 and _objective("supply_cell") == "Completed"), "carried cell restored")
			await create_timer(9).timeout
			_check(not state.get("activity_complete") and _objective("supply_delivery") == "Active", "delivery has no Coolant deadline")
		await _capture("supply-delivery-" + mode + ".png")
		await _walk([Vector3i(-1, 0, -3), Vector3i(-1, 0, -5)])
		_check(await _until(func() -> bool: return state.get("activity_complete") and _cleared() == 3), "supply chapter completed")
		_check(_balance() == 3 and state.get("progression").get("experience") == 300, "exact first-clear reward and no practice grant")
		await _capture("supply-complete-" + mode + ".png")
		game.call("_return_to_campaign_menu")
		await create_timer(0.35).timeout
	if failures.is_empty():
		print("M36 supply passed: historical save, cover, four boundaries, three reconnects, untimed delivery, first clear and unrewarded practice")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
