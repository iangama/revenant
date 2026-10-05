extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	if OS.get_environment("REVENANT_ARSENAL_ACCESSIBLE") == "1":
		var settings: Dictionary = game.get("_settings").duplicate(true)
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
		game.call("_apply_settings", settings, false)
	var username := "m35_arsenal_%d" % Time.get_unix_time_from_system()
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone did not spawn")
		return
	_check(state.get("weapon_profiles").size() == 5, "negotiated five-weapon catalog")
	for id in ["scatter_caster", "rail_driver"]:
		_check(state.get("inventory").get(id, 0) == 1, "fixed weapon provisioned once")
	if OS.get_environment("REVENANT_ARSENAL_LAYOUT_ONLY") == "1":
		await _equip("scatter_caster")
		await create_timer(2.0).timeout
		await _capture("final-hud.png")
		game.call("_open_module_workshop")
		_check(await _until(func() -> bool: return state.get("module_state").get("weapons", []).size() == 5), "workshop loaded")
		game.call("_request_module_preview", ["module_force_matrix"])
		_check(await _until(func() -> bool: return state.get("module_preview").get("weapons", []).size() == 5), "preview loaded")
		await _capture("final-workshop.png")
		var workshop: Control = game.get("_module_workshop")
		workshop.get("_profile_weapon").grab_focus()
		await game.call("_tap_validation_key", KEY_SPACE)
		await create_timer(0.1).timeout
		await game.call("_tap_validation_key", KEY_DOWN)
		await game.call("_tap_validation_key", KEY_ENTER)
		_check(workshop.call("presentation_state").comparison_weapon == "rail_driver", "keyboard selects comparison weapon")
		workshop.get("_close_button").grab_focus()
		await create_timer(0.2).timeout
		_check(workshop.get("_close_button").get_global_rect().end.y <= 720, "enlarged actions scroll into view with keyboard focus")
		await _capture("final-workshop-actions.png")
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	var target: int = game.get("_current_enemy_id")
	var position: Array = state.get("actors")[target].position
	# Enter the rail dead zone, then bypass the local guidance to exercise the server rejection.
	await game.call("_drive_to_route_position", Vector3i(position[0] - 2, 0, position[2]), 4000)
	await _equip("rail_driver")
	var before: int = state.get("actor_health")[target]
	game.call("_send_message", {"type": "AttackIntent", "target_actor_id": target})
	await create_timer(0.5).timeout
	_check(state.get("actor_health")[target] == before, "server rejects rail shot inside minimum range")
	game.call("_request_ui_attack")
	await create_timer(0.1).timeout
	_check("TOO CLOSE" in game.get("_status_label").text, "local guidance explains dead zone")
	await _capture("rail-too-close.png")
	# Weapon cycling is shared by remappable keyboard and shoulder-button input.
	game.call("_cycle_weapon", -1)
	_check(await _until(func() -> bool: return state.get("equipped_weapon_item_id") == "scatter_caster"), "cycling selects Scatter Caster")
	game.call("_send_message", {"type": "AttackIntent", "target_actor_id": target})
	_check(await _until(func() -> bool: return state.get("actor_health")[target] == before - 56), "scatter full damage inside three units")
	await _capture("scatter-close.png")
	await game.call("_drive_to_route_position", Vector3i(position[0] - 4, 0, position[2]), 4000)
	await create_timer(0.45).timeout
	game.call("_send_message", {"type": "AttackIntent", "target_actor_id": target})
	_check(await _until(func() -> bool: return state.get("actor_health")[target] == before - 84), "scatter half damage beyond three units")
	await _capture("scatter-falloff.png")
	await _equip("rail_driver")
	await create_timer(0.45).timeout
	game.call("_send_message", {"type": "AttackIntent", "target_actor_id": target})
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") == 0), "rail clears drone from valid range")
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 5000)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "core boss spawns")
	await game.call("_drive_to_route_position", Vector3i(4, 0, 0), 4000)
	await _capture("rail-core.png")
	_check(await game.call("_drive_validation_attacks", false, 8000), "rail defeats Warden at range")
	_check(await _until(func() -> bool: return state.get("activity_complete")), "mission completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1, "standard reward unchanged")
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return state.get("module_state").get("weapons", []).size() == 5), "workshop receives arsenal catalog")
	game.call("_request_module_preview", ["module_force_matrix"])
	_check(await _until(func() -> bool: return state.get("module_preview").get("weapons", []).size() == 5), "server preview covers new weapons")
	await _capture("arsenal-workshop.png")
	game.call("_close_module_workshop")
	game.get("_session").call("reset_connection")
	game.set("_connection_started", false)
	await create_timer(0.3).timeout
	game.call("_begin_connection", username)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and not state.get("activity_complete")), "fresh mission after reconnect")
	_check(state.get("equipped_weapon_item_id") == "rail_driver", "saved rail selection survives reconnect")
	_check(state.get("inventory").get("rail_driver", 0) == 1, "reconnect does not duplicate arsenal")
	if failures.is_empty():
		print("M35 arsenal flow passed: geometry, guidance, cycling, models, reward, preview and reconnect")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _equip(item_id: String) -> void:
	for button in game.get("_weapon_buttons"):
		if button.get_meta("weapon_item_id") == item_id:
			button.pressed.emit()
	_check(await _until(func() -> bool: return state.get("equipped_weapon_item_id") == item_id), "button equips weapon")
	var player: Node3D = game.get("_actors")[state.get("player_actor_id")]
	_check(player.call("presentation_state").get(item_id + "_visible", false), "confirmed equipment selects the original model")
