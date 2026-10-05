extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var username := "m31_modules_%d" % Time.get_unix_time_from_system()
	for attempt in 3:
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and not state.get("activity_complete")):
			_finish("fresh module run unavailable")
			return
		game.call("_request_weapon", "coil_lance")
		_check(await _until(func() -> bool: return state.get("equipped_weapon_item_id") == "coil_lance"), "lance equipped")
		var expected := [{"damage": 48, "range": 9, "cooldown_ms": 350}, {"damage": 58, "range": 7, "cooldown_ms": 350}, {"damage": 58, "range": 5, "cooldown_ms": 280}]
		var profile: Dictionary = state.get("weapon_profiles").get("coil_lance", {})
		for key in expected[attempt]:
			_check(profile.get(key) == expected[attempt][key], "next admission uses the saved %s" % key)
		if attempt == 2:
			_check(state.get("inventory").get("relay_core_fragment", 0) == 0, "two recipes consume exactly four earned fragments")
			await _capture("content-modules-admitted.png")
			break
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("drone did not clear with the admitted build")
			return
		game.call("_open_route_console")
		if not await game.call("_wait_for_route_state", "choice_open", 4000):
			_finish("route choice unavailable")
			return
		game.get("_route_console").call("select_for_validation", "breach")
		if not await game.call("_wait_for_route_choice", 3000):
			_finish("Breach selection failed")
			return
		game.call("_close_route_console")
		await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 5000)
		if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
			_finish("Breach Warden did not spawn")
			return
		if not await game.call("_drive_validation_attacks", false, 6500):
			_finish("Breach Warden did not fall")
			return
		_check(await _until(func() -> bool: return state.get("activity_complete")), "Breach mission completed")
		game.call("_open_module_workshop")
		if not await _until(func() -> bool: return state.get("module_state").get("catalog", []).size() == 6):
			_finish("six-module catalog unavailable")
			return
		var module_id: String = ["module_focus_lens", "module_cycle_bypass"][attempt]
		game.get("_module_workshop").call("select_modules_for_validation", [module_id])
		game.call("_request_module_combination", module_id)
		if not await _until(func() -> bool: return module_id in state.get("module_state").get("owned_modules", [])):
			_finish("module combination failed")
			return
		var modules: Array = ["module_focus_lens"] if attempt == 0 else ["module_focus_lens", "module_cycle_bypass"]
		game.get("_module_workshop").call("select_modules_for_validation", modules)
		game.call("_request_module_preview", modules)
		_check(await _until(func() -> bool: return state.get("module_preview").get("requested_modules", []) == modules), "server preview accepted the new build")
		game.call("_request_module_loadout", state.get("module_state").get("loadout_revision", 0), modules)
		_check(await _until(func() -> bool: return state.get("module_state").get("equipped_modules", []) == modules), "loadout persisted")
		_check(state.get("weapon_profiles").get("coil_lance", {}) == profile, "loadout change leaves this actor's admitted profile intact")
		await _capture("content-modules-%d.png" % attempt)
		game.call("_close_module_workshop")
		game.get("_session").call("reset_connection")
		game.set("_connection_started", false)
		await create_timer(0.3).timeout
	if failures.is_empty():
		print("M31 content modules passed: earned recipes, six-entry workshop, preview, fixed admission, persistence, three successive builds")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
