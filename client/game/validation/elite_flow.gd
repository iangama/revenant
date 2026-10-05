extends "res://validation/arc_warden_flow.gd"

var guard_id := 0
var partner_id := 0
var composition := "bastion_link"


func _run() -> void:
	composition = OS.get_environment("REVENANT_ELITE_KIND")
	if composition.is_empty(): composition = "bastion_link"
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	if not directory.is_empty(): DirAccess.make_dir_recursive_absolute(directory)
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	if OS.get_environment("REVENANT_ELITE_ACCESSIBLE") == "1":
		var settings: Dictionary = game.get("_settings").duplicate(true)
		settings.merge({"language": "pt_BR", "ui_scale": 1.5, "reduced_motion": true, "reduced_flash": true, "high_contrast": true, "muted": true}, true)
		game.call("_apply_settings", settings, false)
	game.call("_begin_connection", "m34_%s_%d" % [composition, Time.get_unix_time_from_system()])
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("drone unavailable")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("drone did not clear")
		return
	_check(game.get("_session").get("elite_capable"), "elite content negotiated")
	var z := -6 if composition == "bastion_link" else -10
	for point in [Vector3i(4, 0, 0), Vector3i(4, 0, z), Vector3i(5, 0, z)]:
		if not await game.call("_drive_to_route_position", point, 4000):
			_finish("elite approach blocked")
			return
	if not await _until(func() -> bool: return state.get("objectives").get(composition, {}).get("state") == "Active"):
		_finish("elite pair did not start")
		return
	for id in game.call("_enemy_ids"):
		if state.get("actors")[id].archetype == "steel-bulwark": guard_id = id
		else: partner_id = id
	_check(guard_id != 0 and partner_id != 0, "both enemy identities projected")
	var guard: Node = game.get("_actors")[guard_id]
	var partner: Node = game.get("_actors")[partner_id]
	var player_id: int = state.get("player_actor_id")
	var health: int = state.get("actor_health")[player_id]
	await game.call("_drive_to_route_position", Vector3i(6, 0, -8), 1800)
	await game.call("_tap_validation_key", KEY_SPACE)
	_check(await _until(func() -> bool: return game.get("_status_label").text == "BLOCKED • FLANK OR WAIT"), "front shot is confirmed as blocked")
	_check(state.get("actor_health")[guard_id] == 240, "shield holds initial health")
	await _capture("elite-shield.png")
	await game.call("_drive_to_route_position", Vector3i(6, 0, -6), 1400)
	await game.call("_drive_to_route_position", Vector3i(7, 0, -6), 1000)
	await game.call("_tap_validation_key", KEY_SPACE)
	_check(await _until(func() -> bool: return state.get("actor_health")[guard_id] < 240), "flank hits the Bulwark")
	if composition == "bastion_link":
		_check(await _until(func() -> bool: return partner.call("presentation_state").repair_visible), "Mender visibly repairs the Bulwark")
		_check(state.get("actor_health")[guard_id] == 208, "one repair restores exactly eight health")
		await _capture("elite-repair.png")
	else:
		_check(await _until(func() -> bool: return partner.call("presentation_state").charge_warning), "Lancer begins a locked charge after the slam")
		_check(not guard.call("presentation_state").braced, "shield remains down during charge warning")
		await _capture("elite-charge.png")
		var deadline := Time.get_ticks_msec() + 3000
		while partner.call("presentation_state").charge_warning and Time.get_ticks_msec() < deadline:
			_check(not guard.call("presentation_state").braced, "damage warnings never overlap")
			await process_frame
		_check(not partner.call("presentation_state").charge_warning, "charge resolution confirmed")
		_check(state.get("actor_health")[player_id] == health, "stepping away avoids both opening attacks")
		await _capture("elite-breather.png")
	var budget: Dictionary = game.get("_presentation_polish").call("scene_budget", game)
	print("M34 Elite scene budget: ", budget)
	_check(budget.visible_meshes <= 170 and budget.materials <= 32, "elite pair stays within rendering budget")
	if OS.get_environment("REVENANT_ELITE_CAPTURE_ONLY") == "1":
		game.call("_quit_client", 0 if failures.is_empty() else 1)
		return
	if OS.get_environment("REVENANT_ELITE_RETREAT") == "1":
		await game.call("_drive_to_route_position", Vector3i(4, 0, -6), 2500)
		_check(await _until(func() -> bool: return state.get("objectives")[composition].state == "Failed"), "leaving the pad confirms withdrawal")
		_check(game.call("_enemy_ids").is_empty(), "withdrawal clears both enemies")
		_check(not game.get("_entry_shell").visible, "retreat preserves mission control")
		await _capture("elite-withdrawn.png")
	else:
		await game.call("_tap_validation_key", KEY_V)
		_check(game.get("_current_enemy_id") == partner_id, "accessible target action selects the partner")
		var kill_deadline := Time.get_ticks_msec() + 9000
		while state.get("actors").has(partner_id) and Time.get_ticks_msec() < kill_deadline:
			await game.call("_tap_validation_key", KEY_SPACE)
			await create_timer(0.32).timeout
		_check(not state.get("actors").has(partner_id), "partner defeat confirmed")
		_check(game.get("_current_enemy_id") == guard_id, "surviving guard becomes the selected target")
		if composition == "bastion_link":
			var confirmed_health: int = state.get("actor_health")[guard_id]
			await create_timer(1.7).timeout
			_check(state.get("actor_health")[guard_id] == confirmed_health, "Mender death stops repair pulses")
		await _capture("elite-survivor.png")
		kill_deadline = Time.get_ticks_msec() + 12000
		while state.get("actors").has(guard_id) and Time.get_ticks_msec() < kill_deadline:
			if guard.call("presentation_state").braced:
				var facing: Array = guard.call("presentation_state").facing
				await game.call("_drive_to_route_position", Vector3i(8 - facing[1] * 2, 0, -8 + facing[0] * 2), 2200)
			await game.call("_tap_validation_key", KEY_SPACE)
			await create_timer(0.32).timeout
		_check(await _until(func() -> bool: return state.get("objectives")[composition].state == "Completed"), "both elites defeated")
	_check(not state.get("activity_complete"), "elite completion is separate from the mission")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 0 and state.get("progression").get("experience", 0) == 0, "elite pair grants no extra reward")
	await game.call("_drive_to_route_position", Vector3i(6, 0, -6), 3000)
	await game.call("_drive_to_route_position", Vector3i(6, 0, 0), 4000)
	if not await _until(func() -> bool: return game.get("_current_enemy_id") != 0):
		_finish("Warden did not spawn")
		return
	if not await game.call("_drive_validation_attacks", false, 6500):
		_finish("Warden did not fall")
		return
	_check(await _until(func() -> bool: return state.get("activity_complete")), "normal core mission completes")
	_check(state.get("inventory").get("relay_core_fragment", 0) == 1 and state.get("progression").get("experience", 0) == 100, "exactly one standard mission reward")
	if failures.is_empty(): print("M34 Elite flow passed: ", composition, " shield/flank, repair or alternating warnings, targets, victory or retreat, Warden and standard reward")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
