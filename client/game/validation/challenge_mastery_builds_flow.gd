extends "res://validation/challenge_gauntlet_flow.gd"

var _expected_fragments := 8
var _expected_xp := 0


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m37_mastery_20261003"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	await _join_challenge("challenges")
	for build in 3:
		var goal: String = ["breacher", "marksman", "guard"][build]
		if _board().mastery.records.any(func(r: Dictionary) -> bool: return r.goal == goal): continue
		await _prepare_build(build)
		var contract: String = ["close_quarters", "distant_signal", "last_reserve"][build]
		await _join_challenge("challenge_" + contract)
		var equipment: Dictionary = game.get("_challenge_state").snapshot.active.equipment
		var preset: Dictionary = preload("res://presentation/modules/build_presets.gd").PRESETS[build]
		_check(equipment.weapon_id == preset.weapon and equipment.modules == preset.modules, "actual admitted build: " + preset.name)
		await _capture("mastery-build-%d-pt150.png" % build)
		match build:
			0:
				await _walk([Vector3i(5,0,6), Vector3i(5,0,8)])
				await _pair(false)
			1:
				await _walk([Vector3i(-7,0,4), Vector3i(-7,0,1)])
				await _shoot_target(game.get("_current_enemy_id"))
			2:
				_check(_health() == 130, "Guard carries its extra health")
				await _walk([Vector3i(-8,0,4), Vector3i(-8,0,0)])
				_check(await _until(func() -> bool: return _mask() == 1), "Guard west relay")
				await _walk([Vector3i(-8,0,4), Vector3i(0,0,4), Vector3i(0,0,0)])
				_check(await _until(func() -> bool: return _mask() == 3), "Guard east relay")
				await _walk([Vector3i(0,0,4), Vector3i(-4,0,4)])
				_check(not state.get("survival_state").reserve_used, "Guard preserved reserve")
		_check(await _until(func() -> bool: return state.get("activity_complete")), "build contract completes")
		_assert_no_grants()
		await _back_to_board()
		_check(_board().mastery.last_attempt.assessments == [{"goal": goal, "reason": "achieved"}], "saved build mastery: " + goal)
		await _archive_capture("mastery-build-%d-result-pt150.png" % build)
	_check(_board().mastery.badges.any(func(b: Dictionary) -> bool: return b.badge == "arsenal_adept"), "three builds unlock one Arsenal title")
	settings.language = "en"
	game.call("_apply_settings", settings, false)
	if not _board().mastery.records.any(func(r: Dictionary) -> bool: return r.goal == "prism_execution"):
		await _remaining_goals()
	_check(_board().mastery.records.size() == 6 and _board().mastery.badges.size() == 3, "six masteries and three one-time titles")
	var earned: Dictionary = _board().mastery.duplicate(true)
	await _archive_capture("mastery-six-three-en150.png")
	await _join_challenge("challenge_last_reserve")
	await _walk([Vector3i(-8,0,4), Vector3i(-8,0,0)])
	var defeat_deadline := Time.get_ticks_msec() + 25000
	while _health() > 0 and Time.get_ticks_msec() < defeat_deadline: await process_frame
	_check(_health() == 0, "defeat trial")
	_check(await _until(func() -> bool: return game.get("_challenge_state").snapshot.get("active") == null), "defeat terminal committed before returning")
	await _back_to_board()
	_check(_board().mastery.last_attempt.assessments[0].reason == "complete_contract", "defeat gives actionable feedback")
	_check(_board().mastery.records == earned.records and _board().mastery.badges == earned.badges, "defeat preserves one-time achievements")
	await _archive_capture("mastery-defeat-en150.png")
	if failures.is_empty(): print("M37 mastery builds passed: crafted/equipped three actual builds, untimed route, target priority, clean two-phase Prism, six goals/three titles, defeat feedback, first proofs/reconnect and no challenge rewards.")
	game.call("_quit_client", 0 if failures.is_empty() else 1)


func _remaining_goals() -> void:
	await _join_challenge("challenge_meridian_circuit@west_approach")
	await _walk([Vector3i(-30,0,0), Vector3i(-30,0,-7), Vector3i(-28,0,-7), Vector3i(-30,0,-7), Vector3i(-30,0,7), Vector3i(-28,0,7), Vector3i(-30,0,7), Vector3i(-30,0,0), Vector3i(-31,0,0)])
	_check(await _until(func() -> bool: return state.get("activity_complete")), "fresh archive route")
	await _back_to_board()
	await _join_challenge("challenge_bastion_link")
	await _walk([Vector3i(10,0,-6)])
	await _pair(true)
	_check(await _until(func() -> bool: return state.get("activity_complete")), "priority contract")
	await _back_to_board()
	_check(_board().mastery.last_attempt.assessments[0].reason == "achieved", "Mender before Bulwark damage")
	await _join_challenge("challenge_prism_discipline")
	await _clean_prism()
	_check(await _until(func() -> bool: return state.get("activity_complete")), "clean Prism completes")
	_assert_no_grants()
	await _back_to_board()
	_check(_board().mastery.records.size() == 6 and _board().mastery.badges.size() == 3, "six masteries and three one-time titles")
	_check(_board().mastery.last_attempt.prism_damage == 0, "zero damage proved across both phases")


func _prepare_build(index: int) -> void:
	var entry = game.get("_entry_shell")
	entry.call("select_mode", "standalone")
	game.call("_begin_connection", username)
	_check(await _until(func() -> bool: return not entry.visible and game.get("_current_enemy_id") != 0), "workshop operation joined")
	_expected_fragments = _balance()
	_expected_xp = state.get("progression").experience
	var preset: Dictionary = preload("res://presentation/modules/build_presets.gd").PRESETS[index]
	game.call("_request_weapon", preset.weapon)
	_check(await _until(func() -> bool: return state.get("equipped_weapon_item_id") == preset.weapon), "build weapon saved before terminal")
	var rail: bool = state.get("equipped_weapon_item_id") == "rail_driver"
	var target: int = game.get("_current_enemy_id")
	var p: Array = state.get("actors")[target].position
	await game.call("_drive_to_route_position", Vector3i(p[0] - (4 if rail else 2),0,p[2]), 4000)
	_check(await game.call("_drive_validation_attacks", false, 8000), "workshop operation drone")
	await game.call("_drive_to_route_position", Vector3i(6,0,0), 5000)
	_check(await _until(func() -> bool: return game.get("_current_enemy_id") != 0), "workshop operation core")
	if rail: await game.call("_drive_to_route_position", Vector3i(4,0,0), 4000)
	_check(await game.call("_drive_validation_attacks", false, 8000), "workshop operation Warden")
	_check(await _until(func() -> bool: return state.get("activity_complete")), "workshop operation complete")
	_expected_fragments += 1
	_expected_xp += 100
	game.call("_open_module_workshop")
	_check(await _until(func() -> bool: return state.get("module_state").get("catalog", []).size() == 10), "workshop available")
	for module in preset.modules:
		if module in state.get("module_state").owned_modules: continue
		game.call("_request_module_combination", module)
		_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "craft finished")
		_check(state.get("module_result").get("accepted", false), "owned module crafted")
		_expected_fragments -= 2 if module in ["module_focus_lens", "module_cycle_bypass"] else 1
	game.call("_request_module_loadout", state.get("module_state").loadout_revision, preset.modules)
	_check(await _until(func() -> bool: return state.get("module_pending").is_empty()), "build loadout saved")
	_check(state.get("module_result").get("accepted", false), "build loadout accepted")
	game.call("_close_module_workshop")
	_assert_no_grants()
	game.call("_return_to_campaign_menu")
	await create_timer(0.3).timeout


func _shoot_target(target: int) -> void:
	game.set("_current_enemy_id", target)
	game.call("_refresh_selected_enemy")
	_check(await game.call("_drive_validation_attacks", false, 12000), "mastery target defeated")


func _clean_prism() -> void:
	_check(await _until(func() -> bool: return not state.get("prism_state").is_empty() and game.get("_current_enemy_id") != 0), "Prism ready")
	var target: int = game.get("_current_enemy_id")
	var health := _health()
	var seen := {}
	var deadline := Time.get_ticks_msec() + 90000
	while state.get("actors").has(target) and _health() > 0 and Time.get_ticks_msec() < deadline:
		if _mode() == "Warning":
			var pattern: Dictionary = state.get("prism_state").pattern
			var p: Array = state.get("actors")[state.get("player_actor_id")].position
			var safe := Vector3i(p[0],0,p[2])
			match pattern.shape:
				"AcrossX": safe.z = int(pattern.z) + (1 if int(pattern.z) < 3 else -1)
				"AcrossZ": safe.x = int(pattern.x) + (1 if int(pattern.x) < 11 else -1)
				"Center": safe = Vector3i(6,0,0)
				"Perimeter": safe = Vector3i(8,0,0)
			await _walk([safe])
			seen[pattern.shape] = true
			await _until(func() -> bool: return _mode() != "Warning")
		elif _mode() == "Recovery":
			await _walk([Vector3i(6,0,1)])
			await game.call("_tap_validation_key", KEY_SPACE)
			await create_timer(float(game.call("_current_attack_cooldown_ms") + 80) / 1000.0).timeout
		else:
			await process_frame
	_check(seen.has("Center") and seen.has("Perimeter") and _health() == health, "both Prism phases clean")


func _board() -> Dictionary:
	return game.get("_entry_shell").get("_challenge_snapshot")


func _assert_no_grants() -> void:
	_check(_balance() == _expected_fragments and state.get("progression").experience == _expected_xp, "only standalone rewards and module recipe costs")


func _archive_capture(filename: String) -> void:
	var entry = game.get("_entry_shell")
	entry.call("_open_mastery")
	await create_timer(0.2).timeout
	entry.get("_mastery_archive").get_node("AccessibleSurfaceScroll").scroll_vertical = 0
	await _capture(filename)
	entry.call("_close_mastery")
