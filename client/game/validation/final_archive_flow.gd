extends "res://validation/archive_install_flow.gd"


func run(client: Node) -> void:
	game = client
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	if OS.get_environment("REVENANT_FINAL_ARCHIVE_VERIFY_SETTINGS") == "1":
		_check(settings.language == "pt_BR" and settings.ui_scale == 1.5 and settings.bindings.attack.key == KEY_K, "archived settings survive restart")
	else:
		settings.language = "pt_BR"
		settings.ui_scale = 1.5
		settings.muted = true
		preload("res://input/input_bindings.gd").rebind(settings.bindings, "attack", "key", KEY_K)
		game.call("_apply_settings", settings, true)
	var entry = game.get("_entry_shell")
	entry.call("select_mode", "challenges")
	game.call("_begin_connection", "m37_mastery_20261003")
	var deadline := Time.get_ticks_msec() + 15000
	while game.get("_connection_started") and Time.get_ticks_msec() < deadline:
		await game.get_tree().process_frame
	var board: Dictionary = entry.get("_challenge_snapshot")
	if not board.get("mastery") is Dictionary:
		_finish("archived mastery board unavailable")
		return
	_check(board.contracts.size() == 8, "archived server supplies all contracts")
	_check(board.mastery.records.size() == 6 and board.mastery.badges.size() == 3, "restored six-goal three-title history")
	entry.call("_open_mastery")
	await game.get_tree().create_timer(0.2).timeout
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty():
		_check(await game.call("_save_review_capture", directory.path_join("archive-restored-masteries.png")) == OK, "archived capture saved")
	entry.call("_close_mastery")
	var session: Node = game.get("_session")
	session.set("entry_mode", "campaign")
	var joined: Dictionary = await session.call("join_initial_session", "m36_campaign_1790156347", "127.0.0.1", int(OS.get_environment("REVENANT_GAME_PORT")))
	_check(joined.get("campaign_complete", false) and joined.get("campaign_snapshot", {}).get("cleared_chapters") == 6, "restored completed campaign")
	if failures.is_empty(): print("M38 archived checkpoint passed: eight contracts, restored six masteries/three titles, completed campaign and isolated persisted settings.")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
