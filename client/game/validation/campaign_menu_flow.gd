extends "res://validation/campaign_flow.gd"


func _run() -> void:
	username = OS.get_environment("REVENANT_CAMPAIGN_FIXTURE_USER")
	if username.is_empty():
		push_error("Campaign menu flow requires an owned completed-campaign fixture")
		quit(1)
		return
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true}, true)
	game.call("_apply_settings", settings, false)
	var entry: Control = game.get("_entry_shell")
	entry.call("select_mode", "campaign")
	game.call("_begin_connection", username)
	_check(await _until(func() -> bool: return game.get("_story_from_menu") and game.get("_story_panel").visible), "completed save opens journal without another run")
	_check(not game.get("_hud_canvas").visible and not game.get("_connection_started"), "reading does not enter the world")
	_check(game.get("_story_panel").call("available_actions").is_empty(), "completed journal cannot submit decisions")
	_check(_cleared() == 6 and state.get("campaign_state").active == null, "completion snapshot is preserved")
	await _capture("menu-completed-journal-pt150.png")
	await game.call("_tap_validation_key", KEY_ESCAPE)
	_check(not game.get("_story_panel").visible and entry.visible, "Escape returns to the chapter menu")
	_check(get_root().gui_get_focus_owner() == entry.get("_connect_button"), "keyboard focus returns to continue")
	_check(entry.get("_connect_button").text == game.tr("READ CAMPAIGN JOURNAL"), "menu describes completed continuation")
	for index in range(2, 8): _check(not entry.get("_mode").is_item_disabled(index), "cleared chapter remains available for practice")
	await _capture("menu-completed-pt150.png")
	entry.get("_username").text = "different_identity"
	entry.get("_username").emit_signal("text_changed", "different_identity")
	_check(entry.get("_connect_button").text == game.tr("CONTINUE CAMPAIGN"), "cached completion does not follow another identity")
	entry.get("_username").text = username
	entry.get("_username").emit_signal("text_changed", username)
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	_check(entry.get("_connect_button").text == "READ CAMPAIGN JOURNAL", "menu refreshes when language changes")
	await _capture("menu-completed-en150.png")
	if failures.is_empty(): print("M36-D completed menu passed: saved journal, no world entry, Escape/focus, identity scope and EN/PT150")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
