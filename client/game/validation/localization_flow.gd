extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.language = "pt_BR"
	settings.ui_scale = 1.5
	settings.reduced_motion = true
	settings.reduced_flash = true
	game.call("_apply_settings", settings, false)
	_check(TranslationServer.translate("SETTINGS") == "CONFIGURAÇÕES", "static interface text is translated")
	_check(TranslationServer.translate("HP  090 / 100") == "PV  090 / 100", "formatted numbers survive translation")
	_check(TranslationServer.translate("ARC SIDEARM  x1") == "PISTOLA DE ARCO  x1", "assembled inventory names are translated")
	_check(TranslationServer.translate("DMG -10%  CD +20%  RANGE -2  HP +0") == "DANO -10%  RECARGA +20%  ALCANCE -2  PV +0", "signed module tradeoffs preserve exact numeric values")
	await process_frame
	await process_frame
	await _capture("entry-portuguese.png")
	game.call("_begin_connection", "m32_pt_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	game.call("_open_route_console")
	await game.call("_wait_for_route_state", "choice_open", 4000)
	await _capture("routes-portuguese.png")
	game.call("_close_route_console")
	await game.call("_drive_to_route_position", Vector3i(-2, 0, -1), 4000)
	game.call("_open_relay_archive", true)
	_check(game.get("_relay_archive").visible, "archive opens in translated session")
	await process_frame
	await process_frame
	await _capture("archive-portuguese.png")
	game.call("_close_relay_archive")
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 5000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("Warden did not clear")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "Portuguese session completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1, "canonical item identifiers and reward remain unchanged")
	await _capture("hud-portuguese.png")
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return not state.get("module_state").is_empty()), "workshop receives canonical content state")
	await _capture("modules-portuguese.png")
	game.call("_close_module_workshop")
	game.call("_open_cooperation_console")
	_check(await _until(func() -> bool: return not state.get("cooperation_result").is_empty()), "solo cooperation response is displayed")
	await _capture("cooperation-portuguese.png")
	game.call("_close_cooperation_console")
	var inventory: Dictionary = state.get("inventory").duplicate(true)
	settings.language = "pseudo"
	game.call("_apply_settings", settings, false)
	_check(TranslationServer.pseudolocalization_enabled, "expanded text preview enabled")
	_check(state.get("inventory") == inventory, "changing language never rewrites authoritative data")
	game.call("_open_settings", null)
	var panel: Control = game.get("_settings_panel")
	var tabs: TabContainer = panel.get("_tabs")
	tabs.current_tab = 1
	await process_frame
	await process_frame
	await _capture("settings-expanded.png")
	panel.call("close_panel")
	await _capture("hud-expanded.png")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	_check(TranslationServer.translate("SETTINGS") == "SETTINGS" and not TranslationServer.pseudolocalization_enabled, "English can be restored during the session")
	if failures.is_empty():
		print("M32 localization passed: Portuguese text, exact placeholders, catalog IDs, archive, menus, live rewards, expansion preview and English restoration")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
