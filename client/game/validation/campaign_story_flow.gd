extends "res://validation/campaign_counter_flow.gd"


func _run() -> void:
	username = OS.get_environment("REVENANT_CAMPAIGN_FIXTURE_USER")
	var phase := OS.get_environment("REVENANT_STORY_PHASE")
	if username.is_empty() or phase.is_empty():
		push_error("Story flow requires an owned completed-campaign fixture and a phase")
		quit(1)
		return
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	var chapter: String = {"meridian": "meridian_readings", "supply": "broken_supply_line", "counter": "counter_signal", "breach": "the_breach"}[phase]
	game.get("_entry_shell").call("select_mode", "practice_" + chapter)
	game.call("_begin_connection", username)
	if not await _until(func() -> bool: return not game.get("_entry_shell").visible and game.get("_campaign_chapter") == chapter):
		_finish("story chapter not admitted")
		return
	var fragments := _balance()
	var xp: int = state.get("progression").get("experience")
	match phase:
		"meridian": await _meridian_story()
		"supply": await _supply_story()
		"counter": await _core_choice()
		"breach": await _direct_breach()
	_check(_balance() == fragments and state.get("progression").get("experience") == xp, "story, choices and practice never grant rewards")
	if failures.is_empty(): print("M36 story phase passed: " + phase)
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _interact(action: String) -> void:
	game.call("_open_relay_archive", true)
	var panel: Control = game.get("_story_panel")
	if not panel.visible or action not in panel.call("available_actions"):
		_finish("interaction missing: " + action)
		return
	var revision: int = state.get("campaign_state").get("state_revision")
	panel.get("_buttons")[action].emit_signal("pressed")
	_check(await _until(func() -> bool: return int(state.get("campaign_state").get("state_revision")) == revision + 1 and game.get("_story_operation").is_empty()), "saved " + action)
	await _capture("story-" + action + ".png")
	game.call("_close_story_journal")


func _meridian_story() -> void:
	if state.get("campaign_state").story.empty_seat == "returned" and state.get("campaign_state").story.supply == "service":
		game.call("_open_relay_archive")
		game.get("_story_panel").get("_records")[0].emit_signal("pressed")
		await _capture("story-empty-seat-reading-fixed.png")
		game.call("_close_story_journal")
		await _walk([Vector3i(-30, 0, 7), Vector3i(-30, 0, 0), Vector3i(-30, 0, 7), Vector3i(-24, 0, 7), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0)])
		_check(await _until(func() -> bool: return state.get("activity_complete")), "Meridian continuation completes")
		return
	await _walk([Vector3i(-16, 0, 0), Vector3i(-24, 0, 0), Vector3i(-24, 0, -7), Vector3i(-28, 0, -7), Vector3i(-30, 0, -7), Vector3i(-30, 0, 7), Vector3i(-28, 0, 7)])
	_check(await _until(func() -> bool: return _checkpoint() == 2), "story survey boundary")
	await _walk([Vector3i(-30, 0, 7), Vector3i(-31, 0, 7), Vector3i(-31, 0, 8)])
	await _interact("empty_seat_memory")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 2), "memory reconnect")
	_check(state.get("campaign_state").story.empty_seat == "found", "discovery survives reconnect")
	await _walk([Vector3i(-30, 0, 7), Vector3i(-24, 0, 7), Vector3i(-24, 0, 0), Vector3i(-18, 0, 0)])
	await _interact("empty_seat_returned")
	await _interact("supply_service")
	game.call("_open_relay_archive")
	var panel: Control = game.get("_story_panel")
	panel.get("_records")[0].emit_signal("pressed")
	_check("O ASSENTO VAZIO" in panel.get("_body").text, "returned record is rereadable in Portuguese")
	await _capture("story-empty-seat-reading.png")
	game.call("_close_story_journal")
	await _walk([Vector3i(-24, 0, 0), Vector3i(-24, 0, 7), Vector3i(-30, 0, 7), Vector3i(-30, 0, 0), Vector3i(-30, 0, 7), Vector3i(-24, 0, 7), Vector3i(-24, 0, 0), Vector3i(-16, 0, 0)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "Meridian practice complete")


func _supply_story() -> void:
	await _walk([Vector3i(0, 0, 4), Vector3i(-4, 0, 4)])
	_check(_checkpoint() == 0, "covered beacon cannot start a recorded service route")
	await _walk([Vector3i(-8, 0, 4), Vector3i(-8, 0, -3)])
	_check(await _until(func() -> bool: return _checkpoint() == 1 and game.get("_current_enemy_id") != 0), "service approach starts sentinel")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 1 and game.get("_current_enemy_id") != 0), "service guard resumes")
	_check(state.get("actors")[state.get("player_actor_id")].position == [-8, 0, -3], "resume keeps service approach")
	_check(await game.call("_drive_validation_attacks", false, 7000), "service sentinel defeated")
	_check(await _until(func() -> bool: return _checkpoint() == 2), "service guard clear")
	await _walk([Vector3i(-6, 0, -3)])
	await _interact("held_connection_note")
	await _walk([Vector3i(-7, 0, -3), Vector3i(-7, 0, 4), Vector3i(2, 0, 4)])
	await _interact("held_connection_returned")
	game.call("_open_relay_archive")
	var panel: Control = game.get("_story_panel")
	panel.get("_records")[1].emit_signal("pressed")
	_check("A CONEXÃO MANTIDA" in panel.get("_body").text, "second arc rereadable")
	await _capture("story-held-connection-reading.png")
	game.call("_close_story_journal")
	await _walk([Vector3i(-7, 0, 4), Vector3i(-7, 0, -3), Vector3i(-4, 0, -3), Vector3i(-1, 0, -3), Vector3i(-1, 0, -5)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "service delivery completes practice")


func _core_choice() -> void:
	await _walk([Vector3i(0, 0, 6), Vector3i(2, 0, 6), Vector3i(5, 0, 7)])
	await _defeat_pair(false)
	await _walk([Vector3i(7, 0, 6), Vector3i(4, 0, 6), Vector3i(4, 0, -6), Vector3i(5, 0, -6), Vector3i(7, 0, -6)])
	await _defeat_pair(true)
	_check(await _until(func() -> bool: return _checkpoint() == 5), "core decision boundary")
	await _walk([Vector3i(7, 0, -6)])
	await _interact("core_direct")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 5), "core choice reconnect")
	_check(state.get("campaign_state").story.core == "direct", "direct choice persists")
	await _walk([Vector3i(9, 0, -6)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "counter practice completes")


func _direct_breach() -> void:
	await _walk([Vector3i(6, 0, 0)])
	_check(await _until(func() -> bool: return _checkpoint() == 1 and game.get("_current_enemy_id") != 0), "direct Warden starts")
	_check(not game.get("_breach_site").get("_shield").visible, "direct shield absent")
	await _walk([Vector3i(3, 0, 0), Vector3i(3, 0, 3)])
	_check(_checkpoint() == 1, "direct stabilizer waits for the guard")
	await _capture("story-direct-warden.png")
	_check(await game.call("_drive_validation_attacks", false, 7500), "direct Warden defeated")
	_check(await _until(func() -> bool: return _checkpoint() == 2), "direct guard checkpoint")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 2), "direct guard clear resumes")
	_check(game.get("_current_enemy_id") == 0, "direct guard stays absent before stabilizer")
	game.call("_send_message", {"type": "MoveIntent", "position": [9, 0, 0]})
	await create_timer(0.2).timeout
	_check(state.get("actors")[state.get("player_actor_id")].position == [8, 0, 0], "direct core still requires stabilizer")
	await _walk([Vector3i(3, 0, 0), Vector3i(3, 0, 3)])
	_check(await _until(func() -> bool: return _checkpoint() == 3), "direct stabilizer saved")
	await _rejoin()
	_check(await _until(func() -> bool: return _checkpoint() == 3), "direct stabilizer resumes")
	_check(state.get("actors")[state.get("player_actor_id")].position == [3, 0, 3], "direct restore uses its latest boundary")
	await _walk([Vector3i(10, 0, 3), Vector3i(10, 0, 0)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "direct breach practice completes")
	game.call("_open_relay_archive")
	_check("a passagem está aberta" in game.get("_story_panel").get("_body").text, "saved choices select the open passage epilogue")
	await _capture("story-open-passage.png")
