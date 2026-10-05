extends "res://validation/challenge_gauntlet_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m36_campaign_1790156347"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var campaign_before := await _read_campaign()
	await _join_challenge("challenges")
	var entry = game.get("_entry_shell")
	var before: Dictionary = entry.get("_challenge_snapshot").duplicate(true)
	_check(before.get("mastery") is Dictionary, "mastery-v1 archive received")
	if not before.get("mastery") is Dictionary:
		_finish("mastery archive missing")
		return
	_check(before.mastery.revision == "m37-mastery-v1", "frozen mastery rules")
	_validate_archive(before)
	entry.call("_open_mastery")
	var archive = entry.get("_mastery_archive")
	await create_timer(0.2).timeout
	archive.get_node("AccessibleSurfaceScroll").scroll_vertical = 0
	await _capture("mastery-archive-pt150.png")
	archive.get("_choice").select(3)
	archive.call("_refresh_goal")
	archive.get("_close").grab_focus()
	await _capture("mastery-route-detail-pt150.png")
	archive.contract_selected.emit("challenge_meridian_circuit@west_approach")
	_check(entry.call("entry_mode") == "challenge_meridian_circuit" and entry.call("entry_preset") == "west_approach", "archive chooses untimed west route")
	await _join_challenge("challenge_meridian_circuit@west_approach")
	await _walk([Vector3i(-30,0,0), Vector3i(-30,0,-7), Vector3i(-28,0,-7), Vector3i(-30,0,-7), Vector3i(-30,0,7), Vector3i(-28,0,7), Vector3i(-30,0,7), Vector3i(-30,0,0), Vector3i(-31,0,0)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "untimed planning route completed")
	_check(_balance() == 6 and state.get("progression").experience == 600, "mastery gives no items or XP")
	await _back_to_board()
	var after: Dictionary = entry.get("_challenge_snapshot")
	var attempt: Dictionary = after.mastery.last_attempt
	_check(attempt.assessments == [{"goal": "route_planner", "reason": "achieved"}], "route mastery derived from terminal")
	_check(attempt.route_moves == 38, "38 confirmed moves, no deadline")
	for record in before.mastery.records:
		_check(record in after.mastery.records, "first mastery proof retained")
	for badge in before.mastery.badges:
		_check(badge in after.mastery.badges, "one-time title retained")
	_validate_archive(after)
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	entry.call("_open_mastery")
	await create_timer(0.2).timeout
	archive.get_node("AccessibleSurfaceScroll").scroll_vertical = 0
	await _capture("mastery-result-en150.png")
	archive.get("_close").grab_focus()
	await _capture("mastery-route-detail-en150.png")
	entry.call("_close_mastery")
	_check(await _read_campaign() == campaign_before, "campaign preserved")
	if failures.is_empty(): print("M37 mastery archive passed: verified history, forged DTO rejection, goal selection, untimed 38-move route, first proofs/titles/reconnect, unchanged campaign and 6frag/600XP, PTEN150. Three build and Prism mastery rendered trials remain pending.")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _validate_archive(board: Dictionary) -> void:
	var projection := preload("res://projection/challenge_state.gd").new()
	_check(projection.apply(board), "archive projection accepted")
	for variant in 4:
		var bad := board.duplicate(true)
		match variant:
			0: bad.mastery.revision = "unknown"
			1: bad.mastery.records.append({"goal": "free_power", "run_id": 1})
			2: bad.mastery.badges.append({"badge": "arsenal_adept", "run_id": bad.state_revision + 1})
			3: bad.mastery.last_attempt.run_id = bad.state_revision + 1
		_check(not projection.apply(bad), "forged mastery rejected: %d" % variant)
