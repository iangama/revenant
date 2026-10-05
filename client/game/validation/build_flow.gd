extends "res://validation/arsenal_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var index := int(OS.get_environment("REVENANT_BUILD_INDEX"))
	var preset: Dictionary = preload("res://presentation/modules/build_presets.gd").PRESETS[index]
	var expected: Array = [[84, 3, 540, 100], [50, 13, 380, 80], [32, 3, 200, 130]][index]
	if OS.get_environment("REVENANT_ARSENAL_ACCESSIBLE") == "1":
		var settings: Dictionary = game.get("_settings").duplicate(true)
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
		game.call("_apply_settings", settings, false)
	# Account is seeded with eight fragments only in the disposable test database.
	var username := OS.get_environment("REVENANT_BUILD_USERNAME")
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("build fixture did not join")
		return
	var initial_fragments: int = state.get("inventory").get("relay_core_fragment", 0)
	_check(initial_fragments == 8, "owned fixture has the documented fragment seed")
	game.call("_open_module_workshop")
	if not await _until(func() -> bool: return state.get("module_state").get("catalog", []).size() == 10):
		_finish("ten-module catalog missing")
		return
	var workshop: Control = game.get("_module_workshop")
	var original: Dictionary = state.get("weapon_profiles").duplicate(true)
	workshop.call("preview_preset", index)
	_check(await _until(func() -> bool: return not state.get("module_preview").is_empty()), "preset receives authoritative preview")
	var preview: Dictionary = state.get("module_preview")
	for weapon in preview.get("weapons", []):
		if weapon.item_id == preset.weapon:
			_check([weapon.effective_damage, weapon.effective_range, weapon.effective_cooldown_ms, preview.max_health] == expected, "preset profile matches authored tradeoff")
	_check(state.get("weapon_profiles") == original, "preview leaves active combat unchanged")
	workshop.get("_preset_choice").grab_focus()
	await _capture("build-preview.png")
	workshop.get("_close_button").grab_focus()
	await create_timer(0.2).timeout
	_check(workshop.get("_close_button").get_global_rect().end.y <= 720, "keyboard scroll reaches actions")
	await _capture("build-actions.png")
	game.call("_close_module_workshop")
	await _complete_mission(false)
	_check(state.get("inventory").get("relay_core_fragment", 0) == initial_fragments + 1, "baseline mission grants one fragment")
	game.call("_open_module_workshop")
	await _until(func() -> bool: return state.get("module_pending").is_empty())
	for _module_id in preset.modules:
		_check(not workshop.get("_combine_button").disabled, "selected preset exposes next missing recipe")
		workshop.get("_combine_button").pressed.emit()
		_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "combine completes")
		_check(state.get("module_result").get("accepted", false), "fixed module crafted")
	for module_id in preset.modules:
		_check(module_id in state.get("module_state").owned_modules, "preset owns both requested recipes")
	game.call("_request_module_loadout", state.get("module_state").loadout_revision, preset.modules)
	_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "loadout completes")
	_check(state.get("module_result").get("accepted", false), "preset modules saved")
	_check(state.get("weapon_profiles") == original, "saved modules wait for the next session")
	game.call("_close_module_workshop")
	game.get("_session").call("reset_connection")
	game.set("_connection_started", false)
	await create_timer(0.3).timeout
	game.call("_begin_connection", username)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0 and not state.get("activity_complete")), "build reconnects")
	await _equip(preset.weapon)
	var active: Dictionary = state.get("weapon_profiles")[preset.weapon]
	_check([active.damage, active.range, active.cooldown_ms] == expected.slice(0, 3), "saved build becomes active")
	await _capture("build-active.png")
	await _complete_mission(index == 1)
	_check(state.get("activity_complete"), "suggested build clears mission")
	if failures.is_empty():
		print("M35 build flow passed: %s preview, craft, save, reconnect and combat" % preset.name)
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _complete_mission(rail: bool) -> void:
	var target: int = game.get("_current_enemy_id")
	var position: Array = state.get("actors")[target].position
	await game.call("_drive_to_route_position", Vector3i(position[0] - (4 if rail else 2), 0, position[2]), 4000)
	_check(await game.call("_drive_validation_attacks", false, 8000), "build clears drone")
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 5000)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "build enters core")
	if rail:
		await game.call("_drive_to_route_position", Vector3i(4, 0, 0), 4000)
	_check(await game.call("_drive_validation_attacks", false, 8000), "build clears Warden")
	_check(await _until(func() -> bool: return state.get("activity_complete")), "build mission completes")
