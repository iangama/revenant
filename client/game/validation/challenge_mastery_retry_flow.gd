extends "res://validation/challenge_mastery_builds_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m37_mastery_20261003"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true}, true)
	game.call("_apply_settings", settings, false)
	await _join_challenge("challenges")
	var earned: Dictionary = _board().mastery.duplicate(true)
	_check(earned.records.size() == 6 and earned.badges.size() == 3, "accepted archive fixture")
	await _join_challenge("challenge_meridian_circuit@west_approach")
	var first: Dictionary = game.get("_challenge_state").snapshot.active.duplicate(true)
	await _walk([Vector3i(-30,0,0), Vector3i(-30,0,-7), Vector3i(-28,0,-7)])
	# The western approach crosses the log before reaching the lens.
	_check(await _until(func() -> bool: return _mask() == 5), "log and lens recorded before interruption")
	await _disconnect_to_board()
	_check(_board().active == null and _board().last_result.outcome == "interrupted", "disconnect records interruption")
	_check(_board().mastery.last_attempt.run_id == first.run_id, "archive identifies interrupted attempt")
	_check(_board().mastery.last_attempt.assessments[0].reason == "complete_contract", "interrupted attempt gives non-award feedback")
	_check(_board().mastery.records == earned.records and _board().mastery.badges == earned.badges, "retry preserves first proofs and titles")
	await _archive_capture("mastery-interrupted-pt150.png")
	await _join_challenge("challenge_meridian_circuit@west_approach")
	_check(_mask() == 0, "retry resets partial objectives")
	_check(game.get("_challenge_state").snapshot.active.run_id > first.run_id, "retry admits a new attempt")
	await _back_to_board()
	_check(await _until(func() -> bool: return _board().get("mastery") is Dictionary and _board().get("active") == null), "abandoned board archive refreshed")
	_check(_board().mastery.records == earned.records and _board().mastery.badges == earned.badges, "abandon never changes achievements")
	if failures.is_empty(): print("M37 mastery retry passed: disconnect, partial progress reset, interrupted feedback, fresh identity, abandon and retained six goals/three titles.")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _disconnect_to_board() -> void:
	# Drop transport without an abandon command, then use the normal authenticated board read.
	game.call("_show_challenge_board")
	await create_timer(0.3).timeout
	_check(await _until(func() -> bool: return not game.get("_connection_started") and game.get("_entry_shell").visible), "reconnected board loaded")
