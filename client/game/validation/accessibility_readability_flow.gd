extends "res://validation/arc_warden_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	game.call("_begin_connection", "m32_readability_%d" % Time.get_unix_time_from_system())
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.reduced_motion = true
	settings.reduced_flash = true
	settings.muted = true
	settings.high_contrast = true
	settings.captions = true
	game.call("_apply_settings", settings, false)
	var player: Node3D = game.get("_actors")[state.get("player_actor_id")]
	_check(player.call("presentation_state").reduced_motion, "motion setting reaches existing actors")
	game.get("_audio_director").call("play_player_damage")
	_check("Operator hit" in game.get("_sound_captions").call("presentation_state").text, "captions remain available when audio is muted")
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 4000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden unavailable")
		return
	_check(game.get("_actors")[game.get("_current_enemy_id")].call("presentation_state").reduced_motion, "motion setting reaches newly spawned actors")
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("Warden did not clear")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "reduced presentation preserves completion")
	for factor in [1.0, 1.25, 1.5]:
		settings.ui_scale = factor
		game.call("_apply_settings", settings, false)
		await process_frame
		await process_frame
		_check_hud_bounds()
		await _capture("hud-%d.png" % roundi(factor * 100))
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return not state.get("module_state").is_empty()), "workshop receives current catalog")
	await _capture("workshop-large.png")
	var workshop: Control = game.get("_module_workshop")
	var close_button: Button = workshop.get("_close_button")
	close_button.grab_focus()
	await process_frame
	await process_frame
	_check(root.get_visible_rect().encloses(close_button.get_global_rect()), "scaled workshop scroll follows keyboard focus to Close")
	await _capture("workshop-large-close.png")
	game.call("_close_module_workshop")
	game.call("_open_settings", null)
	var panel: Control = game.get("_settings_panel")
	var tabs: TabContainer = panel.get("_tabs")
	for index in tabs.get_tab_count():
		tabs.current_tab = index
		await process_frame
		await process_frame
		await _capture("settings-large-%d.png" % index)
	panel.call("close_panel")
	settings.captions = false
	game.call("_apply_settings", settings, false)
	game.get("_audio_director").call("play_completion")
	_check(game.get("_sound_captions").call("presentation_state").line_count == 0, "captions can be disabled independently")
	if failures.is_empty():
		print("M32 readability passed: 100/125/150% HUD, focus scrolling, contrast, reduced motion/flash, muted captions, normal rewards")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _check_hud_bounds() -> void:
	var bounds := root.get_visible_rect()
	var grid: GridContainer = game.get("_action_grid")
	_check(bounds.encloses(grid.get_global_rect()), "scaled action bar stays inside viewport")
	var health: Label = game.get("_health_label")
	var bar: ProgressBar = game.get("_player_health_bar")
	_check(not health.get_global_rect().intersects(bar.get_global_rect()), "health text does not overlap its bar")
	var inventory: Label = game.get("_inventory_label")
	var progression: Label = game.get("_progression_label")
	_check(not inventory.get_global_rect().intersects(progression.get_global_rect()), "inventory and progression remain separate")
	for child in grid.get_children():
		if child.visible:
			_check(bounds.encloses(child.get_global_rect()), "scaled action remains on screen: " + child.name)
	var captions: Control = game.get("_sound_captions")
	if captions.visible:
		_check(bounds.encloses(captions.get_global_rect()) and captions.size.x >= 300 and captions.size.y <= 150, "caption box remains bounded and readable")
