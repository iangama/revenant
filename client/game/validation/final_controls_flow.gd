extends "res://validation/challenge_mastery_builds_flow.gd"


func _run() -> void:
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	state = game.get("_authoritative_state")
	username = "m37_mastery_20261003"
	var settings: Dictionary = game.get("_settings").duplicate(true)
	var bindings = preload("res://input/input_bindings.gd")
	settings.bindings = bindings.defaults()
	_check(bindings.rebind(settings.bindings, "attack", "key", KEY_K), "attack key remapped")
	_check(bindings.rebind(settings.bindings, "attack", "button", JOY_BUTTON_B), "attack controller remapped")
	settings.merge({"language": "pt_BR", "ui_scale": 1.5, "high_contrast": true, "captions": true, "muted": true, "reduced_motion": true, "reduced_flash": true}, true)
	game.call("_apply_settings", settings, false)
	await _join_challenge("challenge_distant_signal")
	_check(game.get("_attack_button").text == "ATACAR [K]", "visible attack label reflects configured key and language")
	_check(state.get("equipped_weapon_item_id") == "rail_driver", "accepted Marksman fixture")
	var fragments := _balance()
	var xp: int = state.get("progression").experience
	await _walk([Vector3i(-7,0,4), Vector3i(-7,0,1)])
	var target: int = game.get("_current_enemy_id")
	var health: int = state.get("actor_health")[target]
	await game.call("_tap_validation_key", KEY_SPACE)
	await create_timer(0.2).timeout
	_check(state.get("actor_health")[target] == health, "old attack key no longer fires")
	game.call("_open_settings", null)
	await game.call("_tap_validation_key", KEY_K)
	await create_timer(0.2).timeout
	_check(state.get("actor_health")[target] == health, "settings isolate remapped combat input")
	game.get("_settings_panel").call("close_panel")
	game.call("_resume_gameplay")
	await game.call("_tap_validation_key", KEY_K)
	_check(await _until(func() -> bool: return state.get("actor_health").get(target, 0) < health), "remapped keyboard attack confirmed")
	await create_timer(1.0).timeout
	health = state.get("actor_health").get(target, 0)
	var event := InputEventJoypadButton.new()
	event.button_index = JOY_BUTTON_B
	event.pressed = true
	Input.parse_input_event(event)
	await process_frame
	event.pressed = false
	Input.parse_input_event(event)
	_check(await _until(func() -> bool: return state.get("actor_health").get(target, 0) < health), "remapped controller attack confirmed")
	for factor in [1.0, 1.25, 1.5]:
		settings.ui_scale = factor
		game.call("_apply_settings", settings, false)
		await process_frame
		await process_frame
		game.get("_attack_button").grab_focus()
		_check(root.get_visible_rect().encloses(game.get("_attack_button").get_global_rect()), "attack focus remains visible at scale %s" % factor)
		await _capture("final-controls-pt%d.png" % roundi(factor * 100))
	game.call("_resume_gameplay")
	var deadline := Time.get_ticks_msec() + 12000
	while state.get("actors").has(target) and Time.get_ticks_msec() < deadline:
		await game.call("_tap_validation_key", KEY_K)
		await create_timer(1.0).timeout
	_check(await _until(func() -> bool: return state.get("activity_complete")), "remapped challenge completed")
	_check(_balance() == fragments and state.get("progression").experience == xp, "challenge still grants no items or XP")
	await _back_to_board()
	_check(await _until(func() -> bool: return _board().get("mastery") is Dictionary), "archive reloaded")
	_check(_board().mastery.records.size() == 6 and _board().mastery.badges.size() == 3, "existing mastery archive preserved")
	if failures.is_empty(): print("M38 controls passed: visible remapped key, keyboard/controller attacks, menu isolation, PT100/125/150 high-contrast focus, muted/reduced presentation, completion and preserved archive.")
	game.call("_quit_client", 0 if failures.is_empty() else 1)
