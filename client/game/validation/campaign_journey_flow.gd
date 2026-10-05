extends "res://validation/campaign_prism_flow.gd"

const CHAPTERS := ["return_signal", "meridian_readings", "broken_supply_line", "counter_signal", "the_breach", "prism_core"]


func _run() -> void:
	username = "m36_journey_%d" % Time.get_unix_time_from_system()
	print("Fresh journey fixture: " + username)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var entry: Control = game.get("_entry_shell")
	entry.call("select_mode", "campaign")
	for index in 6:
		game.call("_begin_connection", username)
		if not await _until(func() -> bool: return not entry.visible and game.get("_campaign_chapter") == CHAPTERS[index] and _checkpoint() == 0):
			_finish("fresh journey entry failed at chapter %d" % (index + 1))
			return
		_check_campaign_controls()
		if index == 0:
			game.get("_attack_button").grab_focus()
			game.call("_move_action_bar_focus", 1)
			_check(root.gui_get_focus_owner() == game.get("_campaign_button"), "chapter menu reachable from action-bar keyboard navigation")
			game.call("_resume_gameplay")
		match index:
			0: await _return_signal()
			1: await _meridian_readings()
			2: await _supply_line()
			3: await _counter_signal()
			4: await _grounded_breach()
			5: await _prism_core()
		if not await _until(func() -> bool: return state.get("activity_complete") and _cleared() == index + 1):
			_finish("fresh journey chapter %d did not complete" % (index + 1))
			return
		_check(_balance() == index + 1 and state.get("progression").get("experience") == 100 * (index + 1), "exact first-clear totals after chapter %d" % (index + 1))
		var story: Dictionary = state.get("campaign_state").story
		_check(story.empty_seat == "unseen" and story.held_connection == "unseen" and story.supply == null and story.core == null, "main path requires no optional records or choices")
		await _capture("journey-chapter-%d-complete.png" % (index + 1))
		game.get("_campaign_button").emit_signal("pressed")
		_check(entry.visible and entry.call("entry_mode") == "campaign", "chapter menu defaults to campaign continuation")
		for practice in 6:
			_check(entry.get("_mode").is_item_disabled(practice + 2) == (practice > index), "practice availability follows actual clears")
		_check(entry.get("_connect_button").text == game.tr("READ CAMPAIGN JOURNAL" if index == 5 else "START NEXT CHAPTER"), "menu names next action")
		if index in [1, 3, 5]: await _capture("journey-act-%d-menu.png" % (index / 2 + 1))
		await create_timer(0.35).timeout
	game.call("_begin_connection", username)
	_check(await _until(func() -> bool: return game.get("_story_from_menu") and game.get("_story_panel").visible), "fresh journey ending is rereadable after returning to menu")
	_check(state.get("campaign_state").story.epilogue == "signal_silent", "skipping optional choices gives complete default ending")
	await _capture("journey-default-ending.png")
	if failures.is_empty(): print("M36-D fresh journey passed: six chapters, no optional records, no farming, exact six grants, continuation menu, keyboard focus and ending reread")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _return_signal() -> void:
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "fresh relay drone starts")
	_check(await game.call("_drive_validation_attacks", false, 6500), "fresh relay drone defeated")
	await _walk([Vector3i(6, 0, 0)])
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "first Warden starts")
	_check(await game.call("_drive_validation_attacks", false, 7500), "first Warden defeated")


func _meridian_readings() -> void:
	await _walk([Vector3i(-16, 0, 0), Vector3i(-24, 0, 0), Vector3i(-24, 0, -7), Vector3i(-28, 0, -7), Vector3i(-30, 0, -7), Vector3i(-30, 0, 7), Vector3i(-28, 0, 7)])
	_check(_checkpoint() == 2, "fresh survey complete")
	await _walk([Vector3i(-30, 0, 7), Vector3i(-30, 0, 0), Vector3i(-30, 0, 7), Vector3i(-24, 0, 7), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0)])


func _supply_line() -> void:
	await _walk([Vector3i(0, 0, 4), Vector3i(-4, 0, 4), Vector3i(-7, 0, 4), Vector3i(-7, 0, 0)])
	_check(await game.call("_drive_validation_attacks", false, 7000), "fresh sentinel defeated with covered approach")
	await _walk([Vector3i(-7, 0, -3), Vector3i(-4, 0, -3), Vector3i(-1, 0, -3), Vector3i(-1, 0, -5)])


func _counter_signal() -> void:
	await _walk([Vector3i(0, 0, 6), Vector3i(2, 0, 6), Vector3i(5, 0, 7)])
	await _defeat_pair(false)
	await _walk([Vector3i(7, 0, 6), Vector3i(4, 0, 6), Vector3i(4, 0, -6), Vector3i(5, 0, -6), Vector3i(7, 0, -6)])
	await _defeat_pair(true)
	await _walk([Vector3i(9, 0, -6)])


func _grounded_breach() -> void:
	await _walk([Vector3i(6, 0, 0)])
	_check(await _until(func() -> bool: return _checkpoint() == 1 and game.get("_current_enemy_id") != 0), "grounded Warden starts")
	await _walk([Vector3i(3, 0, 0), Vector3i(3, 0, 3)])
	_check(await _until(func() -> bool: return _checkpoint() == 2), "fresh grounded route drains the shield")
	_check(await game.call("_drive_validation_attacks", false, 7500), "grounded Warden defeated")
	await _walk([Vector3i(10, 0, 3), Vector3i(10, 0, 0)])


func _prism_core() -> void:
	await _walk([Vector3i(5, 0, 0), Vector3i(5, 0, 2)])
	_check(await _until(func() -> bool: return _checkpoint() == 1 and _mode() != ""), "fresh Prism starts")
	await _fight_to_shift()
	await _finish_prism()
	_check(await _until(func() -> bool: return _checkpoint() == 2), "fresh Prism defeated")
	await _walk([Vector3i(8, 0, 0), Vector3i(0, 0, 0)])
