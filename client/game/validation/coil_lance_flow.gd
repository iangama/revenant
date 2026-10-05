extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var username := "m31_lance_%d" % Time.get_unix_time_from_system()
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone did not spawn")
		return
	_check(state.get("inventory").get("coil_lance", 0) == 1, "content equipment is provisioned once")
	_check(state.get("weapon_profiles").get("coil_lance", {}).get("cooldown_ms") == 350, "lance profile comes from the server")
	await game.call("_tap_validation_key", KEY_3)
	if not await _until(func() -> bool: return state.get("equipped_weapon_item_id") == "coil_lance"):
		_finish("keyboard could not equip the lance")
		return
	var player: Node3D = game.get("_actors")[state.get("player_actor_id")]
	_check(player.call("presentation_state").get("coil_lance_visible"), "confirmed equipment selects the lance model")
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("lance could not clear the drone")
		return
	await game.call("_drive_to_route_position", Vector3i(0, 0, 4), 4000)
	await game.call("_drive_to_route_position", Vector3i(-4, 0, 4), 4000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("signal sentinel did not spawn")
		return
	await game.call("_drive_to_route_position", Vector3i(-8, 0, 4), 4000)
	await _capture("coil-lance-flank.png")
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("lance could not defeat the sentinel at range nine")
		return
	await game.call("_drive_to_route_position", Vector3i(-8, 0, -3), 4000)
	await game.call("_drive_to_route_position", Vector3i(-4, 0, -3), 4000)
	await game.call("_drive_to_route_position", Vector3i(6, 0, -3), 5000)
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 4000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden did not spawn")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("lance could not clear the Warden")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "lance mission completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1, "normal reward is preserved")
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return state.get("module_state").get("weapons", []).size() == 3), "workshop accepts the negotiated three-weapon catalog")
	game.call("_request_module_preview", ["module_force_matrix"])
	_check(await _until(func() -> bool: return state.get("module_preview").get("weapons", []).size() == 3), "module preview covers the lance")
	await _capture("coil-lance-workshop.png")
	game.call("_close_module_workshop")
	game.get("_session").call("reset_connection")
	game.set("_connection_started", false)
	await create_timer(0.3).timeout
	game.call("_begin_connection", username)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and not state.get("activity_complete")), "fresh mission starts after reconnect")
	_check(state.get("equipped_weapon_item_id") == "coil_lance", "saved lance survives reconnect")
	_check(state.get("inventory").get("coil_lance", 0) == 1, "reconnect does not duplicate the lance")
	if failures.is_empty():
		print("M31 lance flow passed: provisioning, keyboard equip, model, long-range combat, reward, module preview, persisted reconnect")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
