extends "res://validation/arc_warden_flow.gd"

# Reuses the short live-flow helpers; run against a normal solo local gateway.
func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var username := "m31_signal_%d" % Time.get_unix_time_from_system()
	for attempt in 2:
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return _enemy_archetype() == "relay-drone"):
			_finish("fresh drone encounter unavailable")
			return
		_check(state.get("actors").size() == 2, "retry clears old actors")
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("drone did not clear")
			return
		_check(await _until(func() -> bool: return game.call("_signal_available")), "west excursion is available")
		# Follow the south side of the barrier, then enter the beacon.
		await game.call("_drive_to_route_position", Vector3i(0, 0, 4), 4000)
		await game.call("_drive_to_route_position", Vector3i(-4, 0, 4), 4000)
		if not await _until(func() -> bool: return _enemy_archetype() == "signal-sentinel"):
			_finish("sentinel did not spawn")
			return
		var player_id: int = state.get("player_actor_id")
		var sentinel_id: int = game.get("_current_enemy_id")
		var protected_health: int = state.get("actor_health")[player_id]
		if attempt == 0:
			await create_timer(2.0).timeout
			_check(state.get("actor_health")[player_id] == protected_health, "barrier blocks timed ranged fire")
			game.call("_send_message", {"type": "AttackIntent", "target_actor_id": sentinel_id})
			game.call("_send_message", {"type": "MoveIntent", "position": [-4, 0, 0]})
			await create_timer(0.25).timeout
			_check(state.get("actor_health")[sentinel_id] == 200, "barrier blocks player shots")
			_check(state.get("actors")[player_id].position == [-4, 0, 4], "barrier blocks crossing")
			await _capture("sentinel-cover.png")
		await game.call("_drive_to_route_position", Vector3i(-7, 0, 4), 4000)
		await game.call("_drive_to_route_position", Vector3i(-7, 0, 0), 4000)
		_check(await _until(func() -> bool: return state.get("actor_health")[player_id] < protected_health), "exposed player takes timed fire without issuing attacks")
		if attempt == 0:
			var deadline := Time.get_ticks_msec() + 18000
			while not game.get("_entry_shell").visible and Time.get_ticks_msec() < deadline:
				await process_frame
			if not game.get("_entry_shell").visible:
				_finish("defeat did not offer a retry")
				return
			_check(not state.get("activity_complete"), "defeat never completes the mission")
			continue
		await _capture("sentinel-flank.png")
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("sentinel did not fall after flanking")
			return
		_check(await _until(func() -> bool: return state.get("objectives").get("recover_lost_signal", {}).get("progress") == 1), "sentinel defeat unlocks the terminal")
		_check(not state.get("activity_complete"), "sentinel defeat is not mission completion")
		await game.call("_drive_to_route_position", Vector3i(-7, 0, -3), 4000)
		await game.call("_drive_to_route_position", Vector3i(-4, 0, -3), 4000)
		_check(await _until(func() -> bool: return state.get("objectives").get("recover_lost_signal", {}).get("state") == "Completed"), "approaching the terminal recovers the signal")
		await _capture("signal-recovered.png")
		await game.call("_drive_to_route_position", Vector3i(6, 0, -3), 5000)
		await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 4000)
		if not await _until(func() -> bool: return _enemy_archetype() == "warden"):
			_finish("main mission did not resume")
			return
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("Warden did not fall")
			return
		_check(await _until(func() -> bool: return state.get("activity_complete")), "mission completes after the Warden")
		_check(state.get("inventory").get("relay_core_fragment", 0) == 1, "only the normal mission reward was granted")
		_check(state.get("progression").get("experience", 0) == 100, "failed attempt and detour granted no extra XP")
	if failures.is_empty():
		print("M31 signal flow passed: cover, periodic fire, defeat/retry, flanking, signal recovery, Warden, standard rewards")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _enemy_archetype() -> String:
	return state.get("actors").get(game.get("_current_enemy_id"), {}).get("archetype", "")
