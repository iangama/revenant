extends Node

const DEFAULT_HOST := "127.0.0.1"
const DEFAULT_PORT := 7000
const OPERATOR_SCENE := preload("res://presentation/operator/operator.tscn")
const RELAY_HUB_ROOM_SCENE := preload("res://presentation/environment/relay_hub_room.tscn")
const RELAY_DRONE_SCENE := preload("res://presentation/enemies/relay_drone/relay_drone.tscn")
const WARDEN_SCENE := preload("res://presentation/enemies/warden/warden.tscn")
const ARC_WARDEN := preload("res://presentation/enemies/arc_warden/arc_warden.gd")
const SIGNAL_SENTINEL := preload("res://presentation/enemies/signal_sentinel/signal_sentinel.gd")
const SIGNAL_SITE := preload("res://presentation/environment/signal_site.gd")
const COMBAT_VFX_SCENE := preload("res://presentation/combat/combat_vfx.tscn")
const OPERATOR_HUD_SCENE := preload("res://presentation/hud/operator_hud.tscn")
const PRESENTATION_POLISH_SCENE := preload("res://presentation/polish/presentation_polish.tscn")
const LOOT_PICKUP_PRESENTATION := preload("res://presentation/rewards/loot_pickup.gd")
const SECTOR_TRANSITION := preload("res://presentation/environment/sector_transition.gd")
const ENTRY_SHELL_SCENE := preload("res://presentation/entry/entry_shell.tscn")
const SETTINGS_PANEL_SCENE := preload("res://presentation/settings/settings_panel.tscn")
const MODULE_WORKSHOP := preload("res://presentation/modules/module_workshop.gd")
const ROUTE_CONSOLE := preload("res://presentation/routes/route_console.gd")
const COOPERATION_CONSOLE := preload("res://presentation/cooperation/cooperation_console.gd")
const RELAY_ARCHIVE := preload("res://presentation/archive/relay_archive.gd")
const SETTINGS_STORE := preload("res://presentation/settings/settings_store.gd")
const ONBOARDING_CONTROLLER := preload("res://presentation/onboarding/onboarding_controller.gd")
const AUDIO_DIRECTOR := preload("res://presentation/audio/audio_director.gd")
const MESSAGEPACK_CODEC := preload("res://protocol/messagepack_codec.gd")
const SESSION_CONTROLLER := preload("res://session/session_controller.gd")
const AUTHORITATIVE_STATE := preload("res://projection/authoritative_state.gd")
const PLAYER_INTENT_CONTROLLER := preload("res://input/player_intent_controller.gd")
const HUD_PROJECTION := preload("res://presentation/hud_projection.gd")
const AUDIO_HARNESS := preload("res://validation/audio_harness.gd")
const BOUNDARY_HARNESS := preload("res://validation/boundary_harness.gd")
const COMBAT_HARNESS := preload("res://validation/combat_harness.gd")
const EXPERIENCE_HARNESS := preload("res://validation/experience_harness.gd")
const DISPLAY_FIRST_CONTACT_HARNESS := preload("res://validation/display_first_contact_harness.gd")
const PRESENTATION_HARNESS := preload("res://validation/presentation_harness.gd")
const PLAYTEST_OBSERVATION := preload("res://playtest/local_observation_report.gd")
const PLAYTEST_OBSERVATION_HARNESS := preload("res://validation/playtest_observation_harness.gd")
const MODULE_WORKSHOP_HARNESS := preload("res://validation/module_workshop_harness.gd")
const ROUTE_CONSOLE_HARNESS := preload("res://validation/route_console_harness.gd")
const COOPERATION_CONSOLE_HARNESS := preload("res://validation/cooperation_console_harness.gd")

var _session: Node
var _authoritative_state := AUTHORITATIVE_STATE.new()
var _player_intents := PLAYER_INTENT_CONTROLLER.new()
var _hud_projection := HUD_PROJECTION.new()
var _actors := {}
var _current_enemy_id := 0
var _camera: Camera3D
var _environment: Node3D
var _signal_site: Node3D
var _coolant_site: Node3D
var _meridian_sector: Node3D
var _lancer_site: Node3D
var _bulwark_site: Node3D
var _breach_site: Node3D
var _prism_core_site: Node3D
var _counter_site: Node3D
var _support_site: Node3D
var _elite_sites := {}
var _prism_site: Node3D
var _gauntlet_site: Node3D
const ELITE_NAMES := {"bastion_link": "BASTION LINK", "crossed_guard": "CROSSED GUARD"}
var _meridian_map: Control
var _meridian_zone := ""
var _combat_vfx: Node3D
var _loot_pickup: Node3D
var _sector_transition: CanvasLayer
var _hud_frame: Control
var _presentation_polish: CanvasLayer
var _door: Node3D
var _status_label: Label
var _health_label: Label
var _enemy_health_label: Label
var _player_health_bar: ProgressBar
var _enemy_health_bar: ProgressBar
var _position_label: Label
var _objective_label: Label
var _controls_label: Label
var _crosshair: Label
var _guidance_label: Label
var _input_log_label: Label
var _inventory_label: Label
var _progression_label: Label
var _equipment_label: Label
var _input_events: Array[String] = []
var _movement_buttons: Array[Button] = []
var _attack_button: Button
var _target_button: Button
var _weapon_buttons: Array[Button] = []
var _module_button: Button
var _module_workshop: Control
var _route_button: Button
var _route_console: Control
var _cooperation_button: Button
var _cooperation_console: Control
var _story_panel: Control
var _story_markers: Node3D
var _story_operation := ""
var _story_from_menu := false
var _relay_archive: Control
var _archive_button: Button
var _archive_hint: Label
var _hud_canvas: CanvasLayer
var _entry_shell: Control
var _connection_started := false
var _connection_generation := 0
var _settings_store: RefCounted
var _settings := {}
var _settings_panel: Control
var _guidance_mode := "Full"
var _onboarding: RefCounted
var _audio_director: Node3D
var _playtest_observation: RefCounted
var _quitting := false
var _cooling_cue_active := false
var _module_operation_counter := 0
var _route_operation_counter := 0
var _cooperation_operation_counter := 0
var _showcase_mode := false
var _last_defeated_enemy_position := Vector3(6.0, 0.0, 0.0)
var _camera_transition: Tween
var _camera_phase := "arrival"
var _action_bar_focused := false
var _last_input_controller := false
var _sound_captions: PanelContainer
var _telemetry_panel: Panel
var _guide_panel: Panel
var _inventory_panel: Panel
var _input_monitor_panel: Panel
var _input_monitor_title: Label
var _guide_title: Label
var _action_grid: GridContainer
var _campaign_button: Button
var _campaign_session := false
var _challenge_contract := ""
var _challenge_state := preload("res://projection/challenge_state.gd").new()
var _challenge_abandon_operation := ""
var _campaign_chapter := ""
var _campaign_practice := false
var _connection_username := ""
var _surface_scaler := preload("res://presentation/settings/surface_scaler.gd").new()
var _contrast_labels: Array[Label] = []


func _ready() -> void:
	_showcase_mode = OS.get_environment("REVENANT_SHOWCASE_MODE") == "1"
	_session = SESSION_CONTROLLER.new()
	_session.name = "SessionController"
	_session.connect("connection_state_requested", _set_connection_state)
	_session.connect("campaign_snapshot_loaded", _update_campaign_menu)
	_session.connect("challenge_snapshot_loaded", func(user: String, value: Dictionary) -> void: _entry_shell.call("set_challenge_progress", user, value))
	add_child(_session)
	_build_playable_scene()
	_build_entry_shell()
	_build_settings()
	_playtest_observation = PLAYTEST_OBSERVATION.new()
	_playtest_observation.call("configure_from_environment", _settings, _observation_viewport_size())
	print(_playtest_observation.call("diagnostic"))
	if _playtest_observation.call("is_active"):
		get_tree().auto_accept_quit = false
	_onboarding = ONBOARDING_CONTROLLER.new()
	_onboarding.call("reset", _guidance_mode)
	if OS.get_environment("REVENANT_ARCHIVE_PHASE") in ["install", "hold", "update", "final"]:
		call_deferred("_validate_archive_install")
		return
	if OS.get_environment("REVENANT_VALIDATE_SLICE") == "1":
		call_deferred("_validate_playable_slice")
		return
	if _should_auto_connect():
		call_deferred("_begin_connection", _default_username())


func _validate_archive_install() -> void:
	var validation := preload("res://validation/archive_install_flow.gd").new()
	if OS.get_environment("REVENANT_ARCHIVE_PHASE") == "final":
		validation = preload("res://validation/final_archive_flow.gd").new()
	await validation.run(self)


func _process(delta: float) -> void:
	if _equipment_label != null and _sector_transition != null:
		_equipment_label.visible = _challenge_contract != "coolant_recovery" and not _sector_transition.call("presentation_state").active
	if _settings.get("high_contrast", false):
		for label in _contrast_labels:
			if is_instance_valid(label):
				label.add_theme_color_override("font_color", Color.WHITE)
	if _sound_captions != null:
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if player != null:
			_sound_captions.set("listener_position", player.global_position)
	_refresh_archive_context()
	_refresh_signal_site()
	_update_campaign_travel_camera()
	_refresh_coolant_site()
	_refresh_meridian()
	if _lancer_site != null:
		var fighting: bool = _authoritative_state.objectives.get("glass_lancer", {}).get("state") == "Active"
		_lancer_site.call("present", _hud_canvas.visible and _session.get("encounter_capable") and _signal_available(), _hud_canvas.visible and fighting)
	if _bulwark_site != null:
		var fighting: bool = _authoritative_state.objectives.get("steel_bulwark", {}).get("state") == "Active"
		_bulwark_site.call("present", _hud_canvas.visible and _session.get("bulwark_capable") and _signal_available(), _hud_canvas.visible and fighting)
	if _support_site != null:
		var fighting: bool = _authoritative_state.objectives.get("relay_mender", {}).get("state") == "Active"
		_support_site.call("present", _hud_canvas.visible and _session.get("support_capable") and _signal_available(), _hud_canvas.visible and fighting, not _challenge_contract.is_empty())
		if _campaign_session and _campaign_chapter == "counter_signal" and fighting:
			_support_site.get("_label").text = "COUNTER-SIGNAL • REPAIR LINK"
	if _prism_site != null:
		_prism_site.call("present", not _campaign_session and _hud_canvas.visible and _session.get("prism_capable") and _signal_available(), not _campaign_session and _challenge_contract.is_empty() and _hud_canvas.visible and _prism_fighting())
	for id in _elite_sites:
		var fighting: bool = _authoritative_state.objectives.get(id, {}).get("state") == "Active"
		_elite_sites[id].call("present", _hud_canvas.visible and _session.get("elite_capable") and _signal_available(), _hud_canvas.visible and fighting, not _challenge_contract.is_empty())
		if _campaign_session and _campaign_chapter == "counter_signal" and fighting:
			_elite_sites[id].get("_label").text = "COUNTER-SIGNAL • EMITTER DEFENSE"
	if _showcase_mode:
		_update_showcase_camera(delta)


func _input(event: InputEvent) -> void:
	if _story_from_menu and _story_panel != null and _story_panel.visible:
		if event.is_action_pressed("ui_cancel") or event.is_action_pressed("archive"):
			_close_story_journal()
			get_viewport().set_input_as_handled()
		return
	if event is InputEventJoypadButton or event is InputEventJoypadMotion:
		_last_input_controller = true
	elif event is InputEventKey or event is InputEventMouseButton:
		_last_input_controller = false
	if _settings_panel != null and _settings_panel.visible:
		return
	if _entry_shell != null and _entry_shell.visible:
		return
	if not event.is_pressed() or event.is_echo() or _hud_canvas == null or not _hud_canvas.visible:
		return
	for surface in [
		[_story_panel, "archive", _close_story_journal],
		[_relay_archive, "archive", _close_relay_archive],
		[_module_workshop, "modules", _close_module_workshop],
		[_route_console, "routes", _close_route_console],
		[_cooperation_console, "cooperation", _close_cooperation_console],
	]:
		if surface[0] != null and surface[0].visible:
			if event.is_action_pressed("ui_cancel") or event.is_action_pressed(surface[1]):
				surface[2].call()
				_player_intents.call("clear_pending_input")
				if not _action_bar_focused:
					get_viewport().gui_release_focus()
				get_viewport().set_input_as_handled()
			return
	if event.is_action_pressed("settings") or (event is InputEventKey and event.keycode == KEY_ESCAPE) or (_action_bar_focused and event.is_action_pressed("ui_cancel")):
		if _action_bar_focused:
			_resume_gameplay()
		else:
			_open_settings(null)
		get_viewport().set_input_as_handled()
		return
	if event.is_action_pressed("menu_focus") or (not _action_bar_focused and event is InputEventKey and event.keycode == KEY_TAB):
		if _action_bar_focused:
			_resume_gameplay()
		else:
			_action_bar_focused = true
			_player_intents.call("clear_pending_input")
			if _module_button.visible and not _module_button.disabled:
				_module_button.grab_focus()
			else:
				_move_action_bar_focus(1)
		get_viewport().set_input_as_handled()
		return
	if _action_bar_focused:
		if event.is_action_pressed("ui_focus_next") or event.is_action_pressed("ui_right") or event.is_action_pressed("ui_down"):
			_move_action_bar_focus(1)
			get_viewport().set_input_as_handled()
		elif event.is_action_pressed("ui_focus_prev") or event.is_action_pressed("ui_left") or event.is_action_pressed("ui_up"):
			_move_action_bar_focus(-1)
			get_viewport().set_input_as_handled()
		return
	for action in ["interact", "archive", "modules", "routes", "cooperation", "help"]:
		if event.is_action_pressed(action):
			_player_intents.call("clear_pending_input")
			match action:
				"interact", "archive": _open_relay_archive(action == "interact")
				"modules": _open_module_workshop()
				"routes": _open_route_console()
				"cooperation": _open_cooperation_console()
				"help":
					_onboarding.call("toggle")
					_refresh_onboarding()
			get_viewport().set_input_as_handled()
			return
	for index in 3:
		if event.is_action_pressed("weapon_%d" % (index + 1)):
			_request_weapon(["pulse_rifle", "arc_sidearm", "coil_lance"][index])
			get_viewport().set_input_as_handled()
			return
	if event.is_action_pressed("weapon_previous") or event.is_action_pressed("weapon_next"):
		_cycle_weapon(-1 if event.is_action_pressed("weapon_previous") else 1)
		get_viewport().set_input_as_handled()
		return
	if event.is_action_pressed("target_next"):
		_cycle_enemy()
		get_viewport().set_input_as_handled()
		return
	if event.is_action_pressed("attack"):
		if _challenge_contract == "last_reserve": return
		# Keyboard/controller attack uses the explicit active enemy. A mouse
		# click retains cursor aiming and is ignored when it falls on a button.
		if event is InputEventMouseButton and get_viewport().gui_get_hovered_control() is BaseButton:
			return
		_player_intents.call("request_attack", not event is InputEventMouseButton)
		_append_input_log("Attack input detected")
		if not event is InputEventMouseButton:
			get_viewport().set_input_as_handled()


func _resume_gameplay() -> void:
	_action_bar_focused = false
	_player_intents.call("clear_pending_input")
	get_viewport().gui_release_focus()


func _move_action_bar_focus(direction: int) -> void:
	var buttons: Array[Control] = []
	for button in _weapon_buttons + [_module_button, _route_button, _cooperation_button, _archive_button, _target_button, _attack_button, _campaign_button]:
		if button.visible and not button.disabled:
			buttons.append(button)
	if not buttons.is_empty():
		var current := buttons.find(get_viewport().gui_get_focus_owner())
		buttons[posmod(current + direction, buttons.size())].grab_focus()


func _cycle_weapon(direction: int) -> void:
	var available: Array[String] = []
	for item_id in ["pulse_rifle", "arc_sidearm", "coil_lance", "scatter_caster", "rail_driver"]:
		if _authoritative_state.weapon_profiles.has(item_id):
			available.append(item_id)
	if not available.is_empty():
		var current := available.find(_authoritative_state.equipped_weapon_item_id)
		_request_weapon(available[posmod(current + direction, available.size())])


func _begin_connection(username: String) -> void:
	if _connection_started:
		return
	_connection_started = true
	_story_from_menu = false
	_story_panel.visible = false
	_connection_username = username
	_session.set("entry_mode", _entry_shell.call("entry_mode"))
	_session.set("challenge_preset", _entry_shell.call("entry_preset"))
	_connection_generation += 1
	_observe_first("connect_requested")
	_session.call("reset_connection")
	var host := OS.get_environment("REVENANT_GAME_HOST")
	if host.is_empty():
		host = DEFAULT_HOST
	var port_text := OS.get_environment("REVENANT_GAME_PORT")
	var port := DEFAULT_PORT if port_text.is_empty() else int(port_text)
	_entry_shell.call("configure_endpoint", host, port)
	_set_connection_state("Connecting", "Opening a local relay connection to %s:%d." % [host, port])
	call_deferred("_run_handshake", username, host, port)


func _run_handshake(username: String, host: String, port: int) -> void:
	var joined: Dictionary = await _session.call("join_initial_session", username, host, port)
	if joined.get("challenge_board", false):
		_connection_started = false
		_hud_canvas.visible = false
		_entry_shell.call("show_entry", username)
		return
	if joined.get("campaign_complete", false):
		_show_completed_campaign(joined.get("campaign_snapshot", {}))
		return
	if not joined.get("ok", false):
		var connection_outcome := str(joined.get("connection_outcome", "transport_failure"))
		_observe_connection_failure(joined.get("error", ""), connection_outcome)
		_fail(joined.get("error", "initial session join failed"), connection_outcome)
		return
	_observe_connection_outcome("connected")
	for actor in _actors.values():
		actor.queue_free()
	_actors.clear()
	_current_enemy_id = 0
	_enemy_health_label.visible = true
	_enemy_health_bar.visible = true
	_attack_button.visible = not _showcase_mode
	_meridian_zone = ""
	_authoritative_state.call("join_world", joined.get("world", {}))
	_challenge_contract = joined.get("challenge", "")
	_challenge_state = preload("res://projection/challenge_state.gd").new()
	_challenge_abandon_operation = ""
	_campaign_session = joined.get("campaign", false)
	_campaign_chapter = joined.get("campaign_chapter", "")
	_campaign_practice = false
	_campaign_button.visible = _campaign_session or not _challenge_contract.is_empty()
	_campaign_button.text = "CHALLENGE BOARD" if not _challenge_contract.is_empty() else "CHAPTER MENU"
	_module_button.visible = _challenge_contract.is_empty()
	_route_button.visible = not _campaign_session and _challenge_contract.is_empty()
	_cooperation_button.visible = not _campaign_session and _challenge_contract.is_empty()
	_apply_training_layout()
	_environment.call("set_prism_encounter", false)
	if _camera_phase in ["lancer", "bulwark", "prism", "campaign_prism", "coolant_recovery", "challenge_bastion", "challenge_prism", "challenge_signal"] or _camera_phase.begins_with("meridian_") or _camera_phase.begins_with("supply_") or _camera_phase.begins_with("counter_") or _camera_phase == "campaign_breach":
		if _camera_transition != null and _camera_transition.is_valid():
			_camera_transition.kill()
		_camera.look_at_from_position(Vector3(7.8, 9.3, 11), Vector3(2.2, 0.7, 1), Vector3.UP)
	_module_workshop.call("reset_for_connection")
	_route_console.call("reset_for_connection")
	_cooperation_console.call("reset_for_connection")
	_relay_archive.call("reset_for_connection")
	_story_panel.call("reset_for_connection")
	_story_operation = ""
	_apply_inventory_snapshot(joined.get("inventory", {}))
	_apply_progression(joined.get("progression", {}))
	_apply_equipment_snapshot(joined.get("equipment", {}))
	for button in _weapon_buttons:
		button.visible = _challenge_contract.is_empty() and not _showcase_mode and _authoritative_state.weapon_profiles.has(button.get_meta("weapon_item_id"))
	_apply_interface_layout()
	_set_connection_state("Waiting", "Relay joined. Waiting for the server to begin the activity.")
	_entry_shell.call("dismiss")
	_resume_gameplay()
	_hud_canvas.visible = true
	_environment.call("set_presentation_phase", "arrival", false)
	_camera_phase = "arrival"
	_sector_transition.call("present", "SECTOR 01  •  RELAY HUB", "ARRIVAL DECK", Color("35d0d0"))
	_status_label.text = "CONNECTED  •  RELAY-HUB"
	_controls_label.text = "WAITING FOR THE RELAY ACTIVITY..."
	_set_guidance("GETTING READY", "The server is preparing your encounter. This should take only a moment.")
	var activity := await _receive_bootstrap_message("ActivityStart", 5000)
	if activity.is_empty():
		return
	if not _challenge_contract.is_empty():
		_onboarding.call("reset", "off")
		_set_connection_state("Playing", "Authoritative challenge received.")
		_entry_shell.call("dismiss")
		await _run_manual_activity()
		return
	var initial_objective := await _receive_bootstrap_message("ObjectiveUpdate", 5000)
	if initial_objective.is_empty() or initial_objective.get("state") != "Active":
		_fail("expected active initial objective")
		return
	print("activity %s started with objective %s" % [activity.get("activity_id"), initial_objective.get("objective_id")])
	_onboarding.call("reset", _guidance_mode)
	_refresh_onboarding()
	_set_connection_state("Playing", "Authoritative activity state received.")
	_entry_shell.call("dismiss")
	_update_objective_hud(initial_objective)
	if _campaign_session:
		_refresh_campaign_guidance()
		await _run_manual_activity()
		return

	var enemy_id := 0
	var initial_actor_count := 0
	while enemy_id == 0 and initial_actor_count < 3:
		var actor_spawn := await _receive_bootstrap_message("ActorSpawn", 5000)
		if actor_spawn.is_empty():
			return
		initial_actor_count += 1
		_render_actor(actor_spawn)
		if actor_spawn.get("actor_kind") == "enemy":
			enemy_id = actor_spawn.get("actor_id")
	if enemy_id == 0:
		_fail("server did not spawn the first enemy actor")
		return
	var enemy_moved := false
	while true:
		var ai_message := await _receive_message(Time.get_ticks_msec() + 5000)
		if ai_message.get("type") == "CooperationState":
			_apply_cooperation_state(ai_message)
			continue
		if ai_message.get("type") == "ActorUpdate" and ai_message.get("actor_id") == enemy_id:
			enemy_moved = true
			_update_actor(ai_message)
		elif ai_message.get("type") == "DamageApplied" and ai_message.get("source_actor_id") == enemy_id:
			if not enemy_moved:
				_fail("enemy attacked without chasing")
				return
			print("enemy actor %d dealt %d damage; player has %d HP" % [enemy_id, ai_message.get("damage"), ai_message.get("remaining_health")])
			_handle_damage_feedback(ai_message)
			break
		else:
			_fail("unexpected message while observing enemy AI")
			return

	_current_enemy_id = enemy_id
	_refresh_selected_enemy()
	_update_enemy_proximity()
	if not _should_exit_after_flow():
		_status_label.text = "COMBAT ACTIVE  •  AIM AND FIRE"
		_set_guidance("STEP 1  •  CLEAR THE DRONE", "Move with WASD. Aim the orange crosshair at the red drone, then attack until its HP reaches zero.")
		if OS.get_environment("REVENANT_VALIDATE_KEYBOARD_FLOW") == "1":
			call_deferred("_drive_keyboard_activity")
		elif OS.get_environment("REVENANT_VALIDATE_MANUAL_FLOW") == "1":
			call_deferred("_drive_manual_activity")
		elif OS.get_environment("REVENANT_VALIDATE_MODULE_FLOW") == "1":
			call_deferred("_drive_module_activity")
		elif not OS.get_environment("REVENANT_VALIDATE_ROUTE_FLOW").is_empty():
			call_deferred("_drive_route_activity", OS.get_environment("REVENANT_VALIDATE_ROUTE_FLOW"))
		elif OS.get_environment("REVENANT_VALIDATE_COOPERATION_FLOW") == "1":
			call_deferred("_drive_cooperation_activity")
		await _run_manual_activity()
		return

	while true:
		if not _send_message({"type": "AttackIntent", "target_actor_id": enemy_id}):
			_fail("AttackIntent send failed")
			return
		var damage := await _receive_attack_confirmation(enemy_id)
		if damage.is_empty():
			return
		print("dealt %d damage to actor %d; %d HP remains" % [damage.get("damage"), damage.get("target_actor_id"), damage.get("remaining_health")])
		if damage.get("killed", false):
			break
		await get_tree().create_timer(0.35).timeout
	var destroy := await _receive_message(Time.get_ticks_msec() + 5000)
	if destroy.get("type") != "ActorDestroy" or destroy.get("actor_id") != enemy_id:
		_fail("expected ActorDestroy for defeated enemy")
		return
	_destroy_actor(enemy_id)
	var kill_objective := await _receive_message(Time.get_ticks_msec() + 5000)
	var reach_objective := await _receive_message(Time.get_ticks_msec() + 5000)
	_authoritative_state.call("apply_objective", kill_objective)
	_authoritative_state.call("apply_objective", reach_objective)
	if kill_objective.get("objective_type") != "KillActors" or kill_objective.get("state") != "Completed":
		_fail("KillActors objective did not complete")
		return
	if reach_objective.get("objective_type") != "ReachArea" or reach_objective.get("state") != "Active":
		_fail("ReachArea objective did not open")
		return
	print("objective %s completed; next objective %s is active" % [kill_objective.get("objective_id"), reach_objective.get("objective_id")])

	if not _send_message({"type": "MoveIntent", "position": [6, 0, 0]}):
		_fail("MoveIntent send failed")
		return
	var player_update := await _receive_message(Time.get_ticks_msec() + 5000)
	_update_actor(player_update)
	var reached := await _receive_message(Time.get_ticks_msec() + 5000)
	var boss_objective := await _receive_message(Time.get_ticks_msec() + 5000)
	var door := await _receive_message(Time.get_ticks_msec() + 5000)
	_authoritative_state.call("apply_objective", reached)
	_authoritative_state.call("apply_objective", boss_objective)
	if reached.get("state") != "Completed" or boss_objective.get("state") != "Active" or not door.get("open", false):
		_fail("boss stage did not open")
		return
	var boss := await _receive_message(Time.get_ticks_msec() + 5000)
	if boss.get("type") != "ActorSpawn" or boss.get("archetype") != "warden":
		_fail("expected warden ActorSpawn")
		return
	_render_actor(boss)
	var boss_id: int = boss.get("actor_id")
	print("door %s opened; boss warden spawned" % door.get("door_id"))
	while true:
		_send_message({"type": "AttackIntent", "target_actor_id": boss_id})
		var boss_damage := await _receive_attack_confirmation(boss_id)
		if boss_damage.is_empty():
			return
		print("dealt %d damage to boss; %d HP remains" % [boss_damage.get("damage"), boss_damage.get("remaining_health")])
		if boss_damage.get("killed", false):
			break
		await get_tree().create_timer(0.35).timeout
	var boss_destroy := await _receive_message(Time.get_ticks_msec() + 5000)
	_destroy_actor(boss_destroy.get("actor_id"))
	var boss_complete := await _receive_message(Time.get_ticks_msec() + 5000)
	var loot := await _receive_message(Time.get_ticks_msec() + 5000)
	var progression_grant := await _receive_message(Time.get_ticks_msec() + 5000)
	var activity_complete := await _receive_message(Time.get_ticks_msec() + 5000)
	if boss_complete.get("state") != "Completed" or loot.get("type") != "LootGranted" or progression_grant.get("type") != "ProgressionGranted" or activity_complete.get("type") != "ActivityComplete":
		_fail("activity did not complete after boss death")
		return
	_authoritative_state.call("apply_objective", boss_complete)
	_authoritative_state.call("apply_activity_complete", activity_complete)
	_observe_first("completion_observed")
	_observe_terminal_outcome("completed")
	_audio_director.call("play_completion")
	_apply_loot_grant(loot)
	_apply_progression(progression_grant)
	print("activity %s completed" % activity_complete.get("activity_id"))
	if _should_exit_after_flow():
		_quit_client(0)


func _receive_bootstrap_message(expected_type: String, timeout_ms: int) -> Dictionary:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		var message := await _receive_message(deadline)
		if message.get("type") == expected_type:
			return message
		if message.get("type") == "CooperationState":
			_apply_cooperation_state(message)
			continue
		_fail("expected %s during session bootstrap; received %s" % [expected_type, message.get("type", "empty")])
		return {}
	_fail("timed out waiting for %s during session bootstrap" % expected_type)
	return {}


func _receive_attack_confirmation(target_id: int) -> Dictionary:
	while true:
		var message := await _receive_message(Time.get_ticks_msec() + 5000)
		if message.get("type") != "DamageApplied":
			_fail("expected DamageApplied")
			return {}
		var source_id: int = message.get("source_actor_id", 0)
		var message_target_id: int = message.get("target_actor_id", 0)
		_handle_damage_feedback(message)
		if source_id == _authoritative_state.player_actor_id and message_target_id == target_id:
			return message
		if source_id == target_id and message_target_id == _authoritative_state.player_actor_id:
			if message.get("killed", false):
				_fail("enemy pressure defeated the active player")
				return {}
			continue
		_fail("DamageApplied did not describe the active combat exchange")
		return {}
	return {}


func _send_message(value: Dictionary) -> bool:
	if not _session.call("send_message", value):
		_append_input_log("Protocol encode FAILED: %s" % value.get("type", "unknown"))
		return false
	return true


func _receive_message(deadline: int) -> Dictionary:
	return await _session.call("receive_message", deadline)


func _render_actor(actor: Dictionary) -> void:
	_authoritative_state.call("apply_actor_spawn", actor)
	var instance: Node3D
	if actor.get("actor_kind") == "player":
		instance = OPERATOR_SCENE.instantiate()
	elif actor.get("archetype") == "warden":
		instance = ARC_WARDEN.new() if _arc_surge_active() else WARDEN_SCENE.instantiate()
	elif actor.get("archetype") == "signal-sentinel":
		instance = SIGNAL_SENTINEL.new()
	elif actor.get("archetype") == "glass-lancer":
		instance = preload("res://presentation/enemies/glass_lancer/glass_lancer.gd").new()
	elif actor.get("archetype") == "relay-mender":
		instance = preload("res://presentation/enemies/relay_mender/relay_mender.gd").new()
	elif actor.get("archetype") == "prism-warden":
		instance = preload("res://presentation/enemies/prism_warden/prism_warden.gd").new()
	elif actor.get("archetype") == "steel-bulwark":
		instance = preload("res://presentation/enemies/steel_bulwark/steel_bulwark.gd").new()
	else:
		instance = RELAY_DRONE_SCENE.instantiate()
	instance.name = "Actor_%s" % actor.get("actor_id")
	var position: Array = actor.get("position", [0, 0, 0])
	instance.position = Vector3(position[0], position[1], position[2])
	instance.call("set_accessibility", _settings.get("reduced_motion", false), _settings.get("reduced_flash", false))
	add_child(instance)
	if actor.get("actor_kind") == "player":
		instance.call("set_weapon", _authoritative_state.equipped_weapon_item_id)
	_actors[actor.get("actor_id")] = instance
	_update_enemy_proximity()
	if actor.get("actor_kind") == "enemy":
		_audio_director.call("play_enemy_presence", actor.get("archetype", "relay-drone"), instance.global_position)
	if actor.get("actor_kind") == "enemy" and _enemy_health_label != null:
		_enemy_health_label.text = "ENEMY  %s  •  %03d / %03d HP" % [str(actor.get("archetype")).to_upper(), actor.get("health"), actor.get("max_health")]
		_enemy_health_bar.max_value = actor.get("max_health", 100)
		_enemy_health_bar.value = actor.get("health", 100)
	elif actor.get("actor_id") == _authoritative_state.player_actor_id:
		var maximum: int = actor.get("max_health", 100)
		_player_health_bar.max_value = maximum
		_player_health_bar.value = actor.get("health", maximum)
		_health_label.text = "HP  %03d / %03d" % [actor.get("health", maximum), maximum]
	print("rendered %s actor %d (%s)" % [actor.get("actor_kind"), actor.get("actor_id"), actor.get("archetype")])


func _destroy_actor(actor_id: int) -> void:
	var withdrawn: bool = (_authoritative_state.objectives.get("relay_mender", {}).get("state") == "Failed" or _elite_objective().get("state") == "Failed" or _authoritative_state.objectives.get("prism_warden", {}).get("state") == "Failed") and _authoritative_state.actor_health.get(_authoritative_state.player_actor_id, 0) > 0
	var actor: Node = _actors.get(actor_id)
	if actor != null:
		if actor is Node3D and actor_id != _authoritative_state.player_actor_id and not withdrawn:
			_last_defeated_enemy_position = (actor as Node3D).global_position
			_audio_director.call("play_confirmed_defeat", (actor as Node3D).global_position)
			_combat_vfx.call("play_confirmed_defeat", (actor as Node3D).global_position)
		_actors.erase(actor_id)
		if actor.has_method("retire"):
			actor.call("retire")
		else:
			actor.queue_free()
	_authoritative_state.call("apply_actor_destroy", {"actor_id": actor_id})
	if actor_id == _current_enemy_id and _enemy_health_label != null:
		_enemy_health_label.text = "ENEMY  •  DEFEATED"
		_enemy_health_bar.value = 0
		_status_label.text = "DEFEAT CONFIRMED  •  TARGET OFFLINE"
	if actor_id == _current_enemy_id:
		var remaining := _enemy_ids()
		_current_enemy_id = remaining[0] if not remaining.is_empty() else 0
	_refresh_selected_enemy()
	if withdrawn:
		_enemy_health_label.text = "MENDER PAIR • WITHDRAWN"
		var elite: Dictionary = _elite_objective()
		if not elite.is_empty():
			_enemy_health_label.text = tr("%s • WITHDRAWN") % tr(ELITE_NAMES[elite.objective_id])
		if _authoritative_state.objectives.has("prism_warden"):
			_enemy_health_label.text = "PRISM WARDEN • WITHDRAWN"
		_status_label.text = "CORE EAST • CONTINUE WHEN READY"
	print("enemy actor %d destroyed" % actor_id)


func _update_actor(update: Dictionary) -> void:
	_authoritative_state.call("apply_actor_update", update)
	var actor: Node3D = _actors.get(update.get("actor_id"))
	if actor == null:
		return
	var position: Array = update.get("position", [0, 0, 0])
	var previous_position := actor.position
	actor.position = Vector3(position[0], position[1], position[2])
	if actor.has_method("play_authoritative_move"):
		actor.call("play_authoritative_move", previous_position - actor.position)
	if update.has("charge") and actor.has_method("present_charge"):
		actor.call("present_charge", update.charge)
		_status_label.text = "CHARGE • STEP ASIDE" if update.charge.get("winding_up", false) else "RECOVERING • ATTACK"
		if update.charge.get("winding_up", false):
			_audio_director.call("play_lancer_charge")
	if update.has("defense") and actor.has_method("present_defense"):
		actor.call("present_defense", update.defense)
		_status_label.text = "SHIELD UP • FLANK" if update.defense.get("braced", false) else "SHIELD DOWN • ATTACK"
		if update.defense.get("braced", false):
			_audio_director.call("play_bulwark_slam")
	if update.get("actor_id") == _authoritative_state.player_actor_id and previous_position.distance_to(actor.position) > 0.01:
		_audio_director.call("play_confirmed_move", actor.global_position)
	_update_enemy_proximity()
	if update.get("actor_id") == _authoritative_state.player_actor_id and _position_label != null:
		_onboarding.call("confirm", "movement")
		_refresh_onboarding()
		var door_distance := maxi(0, 6 - int(position[0]))
		_position_label.text = "POSITION  [%d, %d]  •  DOOR %d STEPS" % [position[0], position[2], door_distance]
		if _challenge_contract == "coolant_recovery":
			_position_label.text = tr("POSITION [%d, %d] • NO TIME LIMIT") % [position[0], position[2]]
	print("actor %d moved to %s" % [update.get("actor_id"), position])


func _run_manual_activity() -> void:
	_controls_label.text = "LOCAL AUTHORITATIVE PLAYBACK  •  MOVEMENT, COMBAT AND REWARDS SERVER CONFIRMED" if _showcase_mode else "WASD MOVE  •  SPACE ATTACK  •  1/2/3 EQUIP  •  M MODULES  •  R ROUTES  •  C CO-OP"
	if not _challenge_contract.is_empty():
		_controls_label.text = tr("WASD MOVE • SPACE ATTACK • TAB CONTROLS • EQUIPMENT LOCKED")
	var generation := _connection_generation
	while generation == _connection_generation and _session.call("connection_status") == StreamPeerTCP.STATUS_CONNECTED:
		if not _authoritative_state.activity_complete:
			_handle_manual_input()
		var message := _try_receive_message()
		if not message.is_empty():
			_handle_manual_message(message)
		await get_tree().process_frame
	if generation != _connection_generation:
		return
	if not _authoritative_state.activity_complete:
		_observe_first("disconnect_observed")
		_observe_terminal_outcome("disconnected")
		if not _authoritative_state.route_pending.is_empty() or not _authoritative_state.route_state.is_empty():
			_authoritative_state.call("fail_route_request", "relay connection closed; reconnect from the beginning", true)
			_refresh_route_console()
		if not _authoritative_state.cooperation_pending.is_empty() or (not _authoritative_state.cooperation_state.is_empty() and _authoritative_state.cooperation_summary.is_empty()):
			_authoritative_state.call("fail_cooperation_request", "relay connection closed; reconnect from the beginning", true)
			_refresh_cooperation_console()


func _handle_manual_input() -> void:
	if _any_gameplay_surface_open():
		return
	var now := Time.get_ticks_msec()
	if _showcase_mode and _current_enemy_id != 0:
		var showcase_enemy: Node3D = _actors.get(_current_enemy_id)
		if showcase_enemy != null and not _camera.is_position_behind(showcase_enemy.global_position):
			_crosshair.position = _camera.unproject_position(showcase_enemy.global_position + Vector3(0.0, 0.85, 0.0)) - (_crosshair.size * 0.5)
			showcase_enemy.call("set_targeted", true)
	else:
		_crosshair.position = get_viewport().get_mouse_position() - (_crosshair.size * 0.5)
		_update_target_highlight()
	var movement: Vector2 = _player_intents.call("movement", now)
	if movement.length() > 0.2:
		_observe_first("first_movement_attempt")
		_onboarding.call("note_local", "movement")
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if player != null:
			var step := Vector3(signf(movement.x) if absf(movement.x) > 0.2 else 0.0, 0, signf(movement.y) if absf(movement.y) > 0.2 else 0.0)
			var target := player.position + step
			target.x = clampi(roundi(target.x), -32 if _meridian_active() else -12, 12)
			target.z = clampi(roundi(target.z), -12, 12)
			var position := [roundi(target.x), 0, roundi(target.z)]
			var sent := _send_message({"type": "MoveIntent", "position": position})
			_append_input_log("MoveIntent %s %s" % [position, "sent" if sent else "FAILED"])
			_player_intents.call("consume_movement", now)
	var attack: Dictionary = _player_intents.call("take_attack", now)
	if attack.get("requested", false):
		_cooling_cue_active = false
		_observe_first("first_attack_attempt")
		_onboarding.call("note_local", "attack")
		var target_id := _current_enemy_id if attack.get("use_active_target", false) else _aimed_enemy()
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if target_id != 0:
			if _rail_target_too_close(target_id):
				_status_label.text = "TOO CLOSE • KEEP 4 UNITS OR SWITCH"
				_audio_director.call("play_target_unavailable")
				return
			if player != null:
				_combat_vfx.call("play_local_attempt", player.global_position)
			_audio_director.call("play_attack_attempt")
			var sent := _send_message({"type": "AttackIntent", "target_actor_id": target_id})
			_status_label.text = "ATTACK SENT  •  AWAITING SERVER  •  TARGET %d" % target_id if sent else "ATTACK FAILED  •  CONNECTION ERROR"
			_append_input_log("AttackIntent target %d %s" % [target_id, "sent" if sent else "FAILED"])
			_player_intents.call("consume_attack", now)
			if sent:
				_observe_cooldown_acknowledgement()
				if player != null:
					_combat_vfx.call("play_local_cooldown", player.global_position, _current_attack_cooldown_ms())
		else:
			if player != null:
				_combat_vfx.call("play_target_unavailable", player.global_position)
			_audio_director.call("play_target_unavailable")
			_status_label.text = "TARGET UNAVAILABLE  •  ATTACK NOT SENT"
			_append_input_log("Attack unavailable: aim at active enemy")
	elif attack.get("cooling", false):
		_status_label.text = "WEAPON COOLING  •  %d MS" % attack.get("remaining_ms", 0)
		if not _cooling_cue_active:
			_audio_director.call("play_cooldown_acknowledgement")
			_cooling_cue_active = true
	else:
		_cooling_cue_active = false


func _handle_manual_message(message: Dictionary) -> void:
	match message.get("type"):
		"ChallengeSnapshot":
			if not _challenge_state.apply(message, str(_entry_shell.get("_challenge_snapshot").get("character_id", ""))):
				_fail("server returned an invalid challenge snapshot")
				return
			_entry_shell.call("set_challenge_progress", _connection_username, message)
			_authoritative_state.activity_complete = message.get("active") == null
			_refresh_challenge_guidance()
		"ChallengeActionResult":
			if message.get("operation_id") == _challenge_abandon_operation:
				_challenge_abandon_operation = ""
				if message.get("status") in ["accepted", "replayed"]:
					_show_challenge_board()
				else:
					_set_guidance(tr("CHALLENGES"), tr("Abandonment was not confirmed. Reconnect to check the saved attempt."))
		"CampaignSnapshot":
			if not _authoritative_state.call("apply_campaign_snapshot", message):
				_fail("server returned an invalid campaign snapshot")
				return
			if message.get("active") is Dictionary:
				_campaign_practice = message.active.get("practice", false)
			_update_campaign_menu(_connection_username, message)
			_refresh_campaign_guidance()
			_refresh_story_context()
		"CampaignStoryResult":
			if message.get("operation_id") == _story_operation:
				_story_panel.call("show_result", message.get("status", "unconfirmed"))
				_story_operation = ""
		"ActorUpdate":
			_update_actor(message)
		"DamageApplied":
			_handle_damage_feedback(message)
		"ChallengeSurvivalState":
			_handle_survival_state(message)
		"ChallengeGauntletState":
			_handle_gauntlet_state(message)
		"PrismState":
			_handle_prism_state(message)
		"RepairApplied":
			_handle_repair_feedback(message)
		"ActorDestroy":
			var destroyed_id: int = message.get("actor_id")
			_destroy_actor(destroyed_id)
			if destroyed_id == _current_enemy_id:
				_current_enemy_id = 0
		"ObjectiveUpdate":
			_update_objective_hud(message)
		"DoorState":
			_set_door_open(message.get("open", false))
		"ActorSpawn":
			_render_actor(message)
			if message.get("actor_kind") == "enemy":
				if _current_enemy_id == 0:
					_current_enemy_id = message.get("actor_id")
				_refresh_selected_enemy()
				if message.get("archetype") == "warden":
					_onboarding.call("confirm", "warden_spawn")
					_refresh_onboarding()
				_update_enemy_proximity()
				_status_label.text = "BOSS ONLINE  •  WARDEN"
				_set_guidance("STEP 3  •  DEFEAT THE WARDEN", "Aim at the purple Warden and attack until its HP reaches zero.")
				if _arc_surge_active():
					_status_label.text = "BOSS ONLINE  •  ARC WARDEN"
					_refresh_onboarding()
				if message.get("archetype") == "signal-sentinel":
					_status_label.text = "SIGNAL SENTINEL  •  LONG RANGE"
					_refresh_onboarding()
				if message.get("archetype") == "glass-lancer":
					_status_label.text = "GLASS LANCER • WATCH THE FLOOR"
					_refresh_onboarding()
				if message.get("archetype") == "steel-bulwark":
					_status_label.text = "SHIELD UP • FLANK"
					_refresh_onboarding()
				if _campaign_session:
					_refresh_campaign_guidance()
		"ActivityComplete":
			_observe_first("completion_observed")
			_observe_terminal_outcome("completed")
			_authoritative_state.call("apply_activity_complete", message)
			_onboarding.call("confirm", "completion")
			_refresh_onboarding()
			_presentation_polish.call("play_authoritative_completion")
			_audio_director.call("play_completion")
			_environment.call("set_presentation_phase", "secured", true)
			if _showcase_mode:
				call_deferred("_present_showcase_completion")
			else:
				_camera_phase = "secured"
				_sector_transition.call("present", "SECTOR SECURED", "RELAY HUB ONLINE", Color("35d0d0"))
			_status_label.text = "ACTIVITY COMPLETE  •  RELAY AWAKENED"
			_objective_label.text = "OBJECTIVE  •  COMPLETE"
			_controls_label.text = "RELAY_AWAKENING COMPLETED"
			_set_guidance("MISSION COMPLETE", "Reward received. Open Modules to prepare your next run.")
			_refresh_module_workshop()
			_request_acquisition_state()
			print("manual activity %s completed" % message.get("activity_id"))
			if _campaign_session:
				_refresh_campaign_guidance()
		"LootGranted":
			_apply_loot_grant(message)
		"ProgressionGranted":
			_apply_progression(message)
		"EquipmentChanged":
			_apply_equipment_change(message)
			_onboarding.call("confirm", "equipment")
			_refresh_onboarding()
		"AcquisitionSnapshot":
			if not _authoritative_state.call("apply_acquisition_snapshot", message):
				_fail("server returned an invalid commission snapshot")
			_refresh_module_workshop()
		"AcquisitionClaimed":
			if not _authoritative_state.call("apply_acquisition_claim", message):
				_fail("server returned an invalid commission result")
			_update_inventory_hud()
			_refresh_module_workshop()
		"ModuleSnapshot":
			_apply_module_snapshot(message)
		"ModulePreview":
			_authoritative_state.call("apply_module_preview", message)
			_refresh_module_workshop()
		"ModuleCombined":
			_apply_module_mutation(message, "combine")
		"ModuleLoadoutChanged":
			_apply_module_mutation(message, "loadout")
		"RouteState":
			_apply_route_state(message)
		"RouteChoiceResult":
			_apply_route_choice_result(message)
		"RouteOperationSummary":
			_apply_route_summary(message)
		"CooperationState":
			_apply_cooperation_state(message)
		"CooperationStartResult":
			_apply_cooperation_start_result(message)
		"CooperationPingResult":
			_apply_cooperation_ping_result(message)
		"CooperationLifeState":
			_apply_cooperation_life_state(message)
		"CooperationReviveResult":
			_apply_cooperation_revive_result(message)
		"CooperationOperationSummary":
			_apply_cooperation_summary(message)
		_:
			_fail("unexpected manual gameplay message: %s" % message.get("type"))


func _try_receive_message() -> Dictionary:
	return _session.call("try_receive_message")


func _enemy_ids() -> Array[int]:
	var ids: Array[int] = []
	for id in _actors:
		if _authoritative_state.actors.get(id, {}).get("actor_kind") == "enemy" and _authoritative_state.actor_health.get(id, 0) > 0:
			ids.append(id)
	ids.sort()
	return ids


func _cycle_enemy() -> void:
	var ids := _enemy_ids()
	if ids.is_empty():
		return
	_current_enemy_id = ids[(ids.find(_current_enemy_id) + 1) % ids.size()]
	_refresh_selected_enemy()
	_player_intents.call("clear_pending_input")


func _refresh_selected_enemy() -> void:
	if _target_button != null:
		var multiple := _enemy_ids().size() > 1 and not _showcase_mode
		_target_button.visible = multiple
		_cooperation_button.visible = not multiple and not _showcase_mode and not _campaign_session and _challenge_contract.is_empty()
	if _current_enemy_id == 0 or _enemy_health_label == null:
		return
	var actor: Dictionary = _authoritative_state.actors.get(_current_enemy_id, {})
	var health: int = _authoritative_state.actor_health.get(_current_enemy_id, 0)
	var maximum: int = _authoritative_state.actor_max_health.get(_current_enemy_id, 100)
	_enemy_health_label.text = tr("TARGET • %s • %d/%d HP") % [tr("PRISM WARDEN") if actor.get("archetype") == "prism-warden" else str(actor.get("archetype", "")).to_upper(), health, maximum]
	_enemy_health_bar.max_value = maximum
	_enemy_health_bar.value = health
	_update_target_highlight()


func _aimed_enemy() -> int:
	var target := _hovered_enemy()
	if target != 0:
		_current_enemy_id = target
		_refresh_selected_enemy()
	else:
		_status_label.text = "AIM AT THE HIGHLIGHTED TARGET"
	return target


func _hovered_enemy() -> int:
	var target := 0
	var distance := 110.0
	for id in _enemy_ids():
		var enemy: Node3D = _actors[id]
		if _camera.is_position_behind(enemy.global_position):
			continue
		var screen := _camera.unproject_position(enemy.global_position + Vector3(0, 0.8, 0))
		var candidate := get_viewport().get_mouse_position().distance_to(screen)
		if candidate < distance:
			distance = candidate
			target = id
	return target


func _is_enemy_aimed(enemy: Node3D) -> bool:
	return _actors.get(_hovered_enemy()) == enemy


func _update_target_highlight() -> void:
	for id in _enemy_ids():
		var enemy: Node3D = _actors[id]
		if enemy.has_method("set_targeted"):
			enemy.call("set_targeted", id == _current_enemy_id)


func _update_enemy_proximity() -> void:
	var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
	if player == null:
		return
	for id in _enemy_ids():
		var enemy: Node3D = _actors[id]
		if enemy.has_method("set_danger_close"):
			enemy.call("set_danger_close", enemy.position.distance_to(player.position) <= 2.05)
		if enemy.has_method("track_target"):
			enemy.call("track_target", player.global_position)


func _handle_survival_state(message: Dictionary) -> void:
	var run: Variant = _challenge_state.snapshot.get("active")
	if _challenge_contract != "last_reserve" or not run is Dictionary or not _authoritative_state.apply_survival_state(message, int(run.run_id)):
		_fail("server returned an invalid survival state")
		return
	_player_health_bar.value = message.health
	_health_label.text = "HP  %03d / %03d" % [message.health, message.max_health]
	if message.recovered_health > 0:
		_audio_director.call("play_reserve_recovery")
	_refresh_challenge_guidance()


func _handle_gauntlet_state(message: Dictionary) -> void:
	var saved: Dictionary = _challenge_state.snapshot
	var run: Variant = saved.get("active")
	if run == null and saved.get("last_result") is Dictionary: run = saved.last_result.run
	var previous: Dictionary = _authoritative_state.gauntlet_state
	if _challenge_contract != "relay_gauntlet" or not run is Dictionary or not _authoritative_state.apply_gauntlet_state(message, int(run.run_id), preload("res://projection/challenge_state.gd").preset_id(run.contract)):
		_fail("server returned an invalid gauntlet state")
		return
	_player_health_bar.value = message.health
	_health_label.text = "HP  %03d / %03d" % [message.health, message.max_health]
	if message.recovered_health > 0 and message.sequence != previous.get("sequence"):
		_audio_director.call("play_reserve_recovery" if message.get("modifier") is Dictionary and message.modifier.get("reserve_used", false) else "play_gauntlet_recovery")
	_refresh_challenge_guidance()


func _refresh_gauntlet_guidance() -> void:
	var saved: Dictionary = _authoritative_state.gauntlet_state
	var stage: int = saved.get("stage", 1)
	var phase: String = saved.get("phase", "combat")
	var modifier: Dictionary = saved.get("modifier", {}) if saved.get("modifier") is Dictionary else {}
	var title := tr("RELAY GAUNTLET • STAGE %d/3") % int(modifier.get("stage_index", stage))
	_objective_label.text = title
	var camera_phase := "gauntlet_%d" % stage
	if _camera_phase != camera_phase:
		if _camera_transition != null and _camera_transition.is_valid(): _camera_transition.kill()
		_camera_phase = camera_phase
		var center := Vector3(6, 3.4, 7.5) if stage == 1 else Vector3(11, 3, -7.5) if stage == 2 else Vector3(10, 4, 0)
		var location := center + Vector3(8, 12.6, 17.5) if stage == 1 else Vector3(11, 23, 8) if stage == 2 else Vector3(10, 20, 10)
		_camera.transform = Transform3D(Basis.IDENTITY, location).looking_at(center, Vector3.UP)
		_environment.call("set_prism_encounter", stage == 3)
	if _gauntlet_site == null:
		_gauntlet_site = preload("res://presentation/environment/gauntlet_site.gd").new()
		add_child(_gauntlet_site)
	_gauntlet_site.call("present", stage, phase == "transfer", saved.get("transfer_remaining_ms") != null, modifier)
	var target_present := phase == "combat" and not _enemy_ids().is_empty()
	_enemy_health_label.visible = target_present
	_enemy_health_bar.visible = target_present
	_attack_button.visible = target_present and not _authoritative_state.activity_complete and not _showcase_mode
	if _authoritative_state.activity_complete: return
	if phase == "transfer":
		var entrance: Array = [[2, 6], [5, -6]][stage - 1]
		_set_guidance(title, tr("Return to [%d, %d]. Hold 1.2 s for up to +24 HP and the next stage. Leaving resets the hold.") % entrance)
		if "single_reserve" in str(modifier.get("preset_id", "")):
			_set_guidance(title, tr("Exit [%d, %d]: hold 1.2 s. No transfer healing.") % entrance)
			if not modifier.get("reserve_used", false):
				_set_guidance(title, tr("Exit [%d, %d]. Reserve [%d, %d]: +24 HP once. Hold 1.2 s.") % [entrance[0], entrance[1], entrance[0] + 1, entrance[1]])
		_status_label.text = tr("TRANSFER • STAY STILL") if saved.get("transfer_remaining_ms") != null else tr("STAGE CLEAR • RETURN TO TRANSFER")
	elif stage == 3:
		var cue := _prism_guidance()
		_set_guidance(title, tr(cue[1]))
		_status_label.text = tr(cue[0])
		_objective_label.text = tr("GAUNTLET 3/3 • PRISM PHASE %d/2") % (2 if _authoritative_state.prism_state.get("phase") == "Pulses" else 1)
	else:
		_set_guidance(title, tr("Defeat the Mender. Dodge the Lancer's line and attack during recovery.") if stage == 1 else tr("Stop the Mender’s repairs. Flank the Bulwark’s shield and avoid its marked slam."))


func _refresh_survival_guidance() -> void:
	_controls_label.text = tr("WASD MOVE • TAB CONTROLS • HOLD STILL AT STATIONS")
	var saved: Dictionary = _authoritative_state.survival_state
	var phase: String = saved.get("phase", "west")
	var advice: String = {"west": "Hold at west relay [-8, 0] for 3.6 s. Leaving resets the hold.", "east": "Hold at east relay [0, 0] for 3.6 s. Use cover between relays.", "return": "Both relays charged. Return to shelter [-4, 4] alive.", "completed": "Attempt saved, without items or XP. Open the board to restart.", "defeated": "Attempt saved, without items or XP. Open the board to restart."}[phase]
	_set_guidance(tr("LAST RESERVE"), tr(advice))
	_equipment_label.text = tr("RESERVE SPENT • WEAPONS OFFLINE") if saved.get("reserve_used", false) else tr("RESERVE [-4, 5]\nHOLD 1.2 S • UP TO +36 HP ONCE\nWEAPONS OFFLINE")
	if saved.get("charge_remaining_ms") != null:
		_status_label.text = tr("CHARGING RELAY • STAY STILL")
	elif saved.get("recovery_remaining_ms") != null:
		_status_label.text = tr("USING RESERVE • STAY STILL")
	elif saved.get("reserve_used", false):
		_status_label.text = tr("LAST RESERVE • RESERVE SPENT")


func _handle_repair_feedback(message: Dictionary) -> void:
	_authoritative_state.call("apply_repair", message)
	var source: Node3D = _actors.get(message.get("source_actor_id"))
	var target: Node3D = _actors.get(message.get("target_actor_id"))
	if source != null and target != null and source.has_method("present_repair"):
		source.call("present_repair", target, message.get("amount", 0))
	_audio_director.call("play_mender_repair")
	_refresh_selected_enemy()


func _actor_family(actor: Node) -> String:
	if actor != null and actor.has_method("presentation_state"):
		return actor.call("presentation_state").get("family", "relay-drone")
	return "relay-drone"


func _handle_damage_feedback(message: Dictionary) -> void:
	if message.get("source_actor_id") == _authoritative_state.player_actor_id and message.get("damage", 0) > 0:
		_onboarding.call("confirm", "damage")
		_refresh_onboarding()
	var source_id: int = message.get("source_actor_id")
	var target_id: int = message.get("target_actor_id")
	var remaining: int = message.get("remaining_health")
	_authoritative_state.call("apply_damage", message)
	var source_actor: Node = _actors.get(source_id)
	var target_actor: Node3D = _actors.get(target_id)
	if source_actor is Node3D:
		if source_id == _authoritative_state.player_actor_id:
			_audio_director.call("play_confirmed_attack", _authoritative_state.equipped_weapon_item_id, (source_actor as Node3D).global_position)
		else:
			_audio_director.call("play_enemy_attack", _actor_family(source_actor), (source_actor as Node3D).global_position)
	if message.get("damage", 0) == 0 and _campaign_session and _campaign_chapter == "the_breach" and target_id == _current_enemy_id:
		if source_actor != null and source_actor.has_method("play_confirmed_attack"):
			source_actor.call("play_confirmed_attack")
		_audio_director.call("play_shield_block")
		_status_label.text = tr("SHIELD POWERED • DRAIN THE RELAY")
		return
	if message.get("damage", 0) == 0 and target_actor != null and target_actor.has_method("play_confirmed_block"):
		target_actor.call("play_confirmed_block")
		if source_actor != null and source_actor.has_method("play_confirmed_attack"):
			source_actor.call("play_confirmed_attack")
		_audio_director.call("play_shield_block")
		_status_label.text = "SHELL CLOSED • WAIT FOR OPENING" if _prism_fighting() else "BLOCKED • FLANK OR WAIT"
		return
	if target_actor != null:
		_audio_director.call("play_confirmed_impact", target_actor.global_position)
	if source_actor != null and source_actor.has_method("play_confirmed_attack"):
		source_actor.call("play_confirmed_attack")
	if source_actor is Node3D and target_actor != null:
		_combat_vfx.call("play_confirmed_exchange", source_actor, target_actor, target_id == _authoritative_state.player_actor_id)
	if target_id == _authoritative_state.player_actor_id:
		_audio_director.call("play_player_damage")
		_presentation_polish.call("play_confirmed_player_damage")
		_player_health_bar.value = remaining
		var player_maximum: int = _authoritative_state.actor_max_health.get(target_id, 100)
		_health_label.text = "HP  %03d / %03d" % [remaining, player_maximum]
		_status_label.text = "HOSTILE HIT CONFIRMED  •  -%d  •  %d HP" % [message.get("damage"), remaining]
		_health_label.modulate = Color("ff6b6b")
		var health_tween := create_tween()
		health_tween.tween_property(_health_label, "modulate", Color.WHITE, 0.3)
	else:
		_status_label.text = "SERVER HIT CONFIRMED  •  %d DAMAGE  •  %d HP" % [message.get("damage"), remaining]
		if _enemy_health_label != null and target_id == _current_enemy_id:
			var maximum: int = _authoritative_state.actor_max_health.get(target_id, remaining)
			_enemy_health_label.text = "ENEMY  •  %03d / %03d HP" % [remaining, maximum]
			_enemy_health_bar.max_value = maximum
			_enemy_health_bar.value = remaining
		_refresh_selected_enemy()
		_append_input_log("Server confirmed %d damage" % message.get("damage"))
	var actor: Node3D = target_actor
	if actor != null:
		if actor.has_method("play_confirmed_hit"):
			actor.call("play_confirmed_hit")
			if remaining <= 0 and actor.has_method("play_defeat"):
				actor.call("play_defeat")
		else:
			var original := actor.scale
			actor.scale = original * 1.25
			var hit_tween := create_tween()
			hit_tween.tween_property(actor, "scale", original, 0.12)


func _update_objective_hud(objective: Dictionary) -> void:
	_authoritative_state.call("apply_objective", objective)
	var objective_id := str(objective.get("objective_id", ""))
	var objective_state := str(objective.get("state", ""))
	if objective_id == "prism_warden":
		_present_prism_objective(objective)
		return
	if objective_id in ["glass_lancer", "steel_bulwark", "relay_mender", "bastion_link", "crossed_guard"]:
		_present_training_objective(objective)
		return
	if objective_id.begins_with("supply_") or objective_id.begins_with("counter_") or objective_id.begins_with("breach_") or (_campaign_session and objective_id.begins_with("prism_")):
		_refresh_campaign_guidance()
		return
	if objective_id.begins_with("meridian_"):
		_refresh_meridian()
		_refresh_onboarding()
		return
	if objective_id.begins_with("coolant_"):
		_present_coolant_objective()
		return
	if objective_id == "recover_lost_signal":
		_present_signal_objective(objective)
		return
	if objective_id == "reach_relay_door" and objective_state == "Active":
		_onboarding.call("confirm", "door_objective")
		_refresh_onboarding()
	_objective_label.text = _hud_projection.call("objective_text", objective)
	if objective_id == "reach_relay_stabilizer" and objective_state == "Active":
		_status_label.text = "ROUTE OBJECTIVE  •  REACH STABILIZER [3, 0, 3]"
		_set_guidance("ROUTE STEP  •  STABILIZE", "Use D and S to reach the server-authored stabilizer coordinate [3, 0, 3]. The relay door remains pending.")
	elif objective_id == "reach_relay_door" and objective_state == "Active":
		if _camera_phase != "breach":
			_camera_phase = "breach"
			_environment.call("set_presentation_phase", "breach", true)
			_sector_transition.call("present", "SECTOR 01-B  •  RELAY HUB", "BREACH CORRIDOR", Color("f5a524"))
		_status_label.text = "RELAY DOOR UNLOCKED  •  MOVE TO X=6"
		_set_guidance("STEP 2  •  OPEN THE CORE", "Hold D to move right until you reach the orange relay door at x=6.")
		if _signal_available():
			_set_guidance("CORE OR LOST SIGNAL", "Go right to the core, or follow the west beacon for an optional encounter.")
	elif objective_state == "Failed":
		_status_label.text = "ROUTE FAILED  •  SERVER DEADLINE EXCEEDED"
		_set_guidance("ROUTE FAILED", "The server closed this routed operation after its deadline. No route reward was granted.")


func _apply_inventory_snapshot(message: Dictionary) -> void:
	_authoritative_state.call("apply_inventory_snapshot", message)
	_update_inventory_hud()


func _apply_loot_grant(message: Dictionary) -> void:
	var item_id: String = message.get("item_id", "unknown")
	_authoritative_state.call("apply_loot_grant", message)
	_update_inventory_hud()
	var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
	if player != null:
		_loot_pickup.call("play_confirmed_pickup", item_id, int(message.get("quantity", 0)), _last_defeated_enemy_position, player.global_position)
		if _showcase_mode:
			_camera_phase = "reward"
	_status_label.text = "LOOT SECURED  •  %s +%d" % [item_id.replace("_", " ").to_upper(), message.get("quantity", 0)]
	print("loot granted: %s x%d" % [item_id, message.get("quantity", 0)])


func _update_inventory_hud() -> void:
	_inventory_label.text = _hud_projection.call("inventory_text", _authoritative_state.inventory)


func _apply_progression(message: Dictionary) -> void:
	_authoritative_state.call("apply_progression", message)
	_update_progression_hud()
	if message.get("type") == "ProgressionGranted":
		_status_label.text = "PROGRESSION SECURED  •  +%d XP" % message.get("experience_granted", 0)
		print("progression granted: +%d XP, level %d" % [message.get("experience_granted", 0), message.get("level", 1)])


func _update_progression_hud() -> void:
	_progression_label.text = _hud_projection.call("progression_text", _authoritative_state.progression)


func _apply_equipment_snapshot(message: Dictionary) -> void:
	_authoritative_state.call("apply_equipment_snapshot", message)
	_sync_attack_cooldown()
	_update_operator_weapon()
	_update_equipment_hud()


func _apply_equipment_change(message: Dictionary) -> void:
	if not _authoritative_state.call("apply_equipment_change", message):
		_status_label.text = "EQUIP REJECTED  •  %s" % message.get("message", "SERVER REJECTED ITEM")
		return
	_sync_attack_cooldown()
	_update_operator_weapon()
	_update_equipment_hud()
	_status_label.text = "WEAPON EQUIPPED  •  %s" % _authoritative_state.equipped_weapon_item_id.replace("_", " ").to_upper()
	print("authoritative weapon equipped: %s" % _authoritative_state.equipped_weapon_item_id)


func _update_equipment_hud() -> void:
	var profile: Dictionary = _authoritative_state.weapon_profiles.get(_authoritative_state.equipped_weapon_item_id, {})
	_equipment_label.text = _hud_projection.call("equipment_text", _authoritative_state.equipped_weapon_item_id, profile)


func _toggle_module_workshop() -> void:
	if _module_workshop.visible:
		_close_module_workshop()
	else:
		_open_module_workshop()


func _open_module_workshop() -> void:
	if not _challenge_contract.is_empty(): return
	if _route_console != null:
		_route_console.call("close_console")
	if _cooperation_console != null:
		_cooperation_console.call("close_console")
	_module_workshop.call("select_profile_weapon", _authoritative_state.equipped_weapon_item_id)
	_module_workshop.call("open_workshop")
	_request_acquisition_state()
	if _authoritative_state.call("begin_module_request", "state"):
		_refresh_module_workshop()
		if not _send_message({"type": "ModuleStateRequest"}):
			_authoritative_state.call("fail_module_request", "module state request could not be sent", true)
	_refresh_module_workshop()


func _close_module_workshop() -> void:
	_module_workshop.call("close_workshop")
	if _module_button != null:
		_module_button.grab_focus()


func _request_acquisition_state() -> void:
	if _session.get("acquisition_capable"):
		_send_message({"type": "AcquisitionStateRequest"})


func _request_acquisition_claim(arc_id: String) -> void:
	if not _session.get("acquisition_capable") or not _authoritative_state.call("begin_module_request", "claim", "", {"arc_id": arc_id}):
		return
	_refresh_module_workshop()
	if not _send_message({"type": "AcquisitionClaimIntent", "arc_id": arc_id}):
		_authoritative_state.call("fail_module_request", "commission claim could not be sent", true)
		_refresh_module_workshop()


func _request_module_preview(modules: Array) -> void:
	if not _authoritative_state.call("begin_module_request", "preview", "", {"modules": modules}):
		return
	_refresh_module_workshop()
	if not _send_message({"type": "ModulePreviewRequest", "modules": modules}):
		_authoritative_state.call("fail_module_request", "module preview request could not be sent", true)
		_refresh_module_workshop()


func _request_module_combination(module_id: String) -> void:
	var operation_id := _new_module_operation_id("c")
	if not _authoritative_state.call("begin_module_request", "combine", operation_id, {"module_id": module_id}):
		return
	_refresh_module_workshop()
	if not _send_message({"type": "ModuleCombineIntent", "operation_id": operation_id, "module_id": module_id}):
		_authoritative_state.call("fail_module_request", "module combination could not be sent", true)
		_refresh_module_workshop()


func _request_module_loadout(expected_revision: int, modules: Array) -> void:
	var operation_id := _new_module_operation_id("l")
	if not _authoritative_state.call("begin_module_request", "loadout", operation_id, {"expected_revision": expected_revision, "modules": modules}):
		return
	_refresh_module_workshop()
	if not _send_message({"type": "ModuleLoadoutIntent", "operation_id": operation_id, "expected_revision": expected_revision, "modules": modules}):
		_authoritative_state.call("fail_module_request", "module loadout could not be sent", true)
		_refresh_module_workshop()


func _new_module_operation_id(kind: String) -> String:
	_module_operation_counter += 1
	var entropy := Crypto.new().generate_random_bytes(8)
	if entropy.size() == 8:
		return "godot-%s-%s" % [kind, entropy.hex_encode()]
	return "godot-%s-%d-%d" % [kind, Time.get_ticks_usec(), _module_operation_counter]


func _apply_module_snapshot(message: Dictionary) -> void:
	if not _authoritative_state.call("apply_module_snapshot", message):
		_fail("server returned an invalid module snapshot")
		return
	_update_inventory_hud()
	_refresh_module_workshop()


func _apply_module_mutation(message: Dictionary, kind: String) -> void:
	if kind == "combine":
		_authoritative_state.call("apply_module_combination", message)
	else:
		_authoritative_state.call("apply_module_loadout", message)
	_update_inventory_hud()
	_refresh_module_workshop()


func _refresh_module_workshop() -> void:
	if _module_workshop == null:
		return
	_module_workshop.call(
		"present",
		_authoritative_state.module_state,
		_authoritative_state.module_preview,
		_authoritative_state.module_pending,
		_authoritative_state.module_result,
		_authoritative_state.activity_complete
	)
	_module_workshop.call("present_commissions", _authoritative_state.acquisition_state)


func _toggle_route_console() -> void:
	if _route_console.visible:
		_close_route_console()
	else:
		_open_route_console()


func _open_route_console() -> void:
	if _campaign_session or not _challenge_contract.is_empty():
		return
	if _module_workshop != null:
		_module_workshop.call("close_workshop")
	if _cooperation_console != null:
		_cooperation_console.call("close_console")
	_route_console.call("open_console")
	_request_route_state()


func _close_route_console() -> void:
	_route_console.call("close_console")
	if _route_button != null:
		_route_button.grab_focus()


func _request_route_state() -> void:
	if not _authoritative_state.call("begin_route_state_request"):
		_refresh_route_console()
		return
	_refresh_route_console()
	if not _send_message({"type": "RouteStateRequest"}):
		_authoritative_state.call("fail_route_request", "route state request could not be sent", true)
		_refresh_route_console()


func _request_route_choice(route_id: String) -> void:
	var operation_id := _new_route_operation_id()
	if not _authoritative_state.call("begin_route_choice", operation_id, route_id):
		return
	_refresh_route_console()
	if not _send_message({"type": "RouteChoiceIntent", "operation_id": operation_id, "route_id": route_id}):
		_authoritative_state.call("fail_route_request", "route choice could not be sent", true)
		_refresh_route_console()


func _new_route_operation_id() -> String:
	_route_operation_counter += 1
	var entropy := Crypto.new().generate_random_bytes(8)
	if entropy.size() == 8:
		return "godot-r-%s" % entropy.hex_encode()
	return "godot-r-%d-%d" % [Time.get_ticks_usec(), _route_operation_counter]


func _arc_surge_active() -> bool:
	var selection = _authoritative_state.route_state.get("selection")
	return selection is Dictionary and selection.get("event_id") == "arc_surge"


func _apply_route_state(message: Dictionary) -> void:
	var accepted: bool = _authoritative_state.call("apply_route_state", message)
	_refresh_route_console()
	if not accepted:
		_status_label.text = "ROUTE STATE REJECTED  •  %s" % _authoritative_state.route_result.get("message", "SERVER REJECTED REQUEST")


func _apply_route_choice_result(message: Dictionary) -> void:
	var accepted: bool = _authoritative_state.call("apply_route_choice_result", message)
	_refresh_route_console()
	if not accepted:
		_status_label.text = "ROUTE CHOICE REJECTED  •  %s" % _authoritative_state.route_result.get("message", "SERVER REJECTED CHOICE")
		return
	var selection: Dictionary = _authoritative_state.route_result.get("selection", {})
	_status_label.text = "ROUTE ACCEPTED  •  %s  •  EVENT %s" % [
		str(selection.get("route_id", "")).to_upper(),
		str(selection.get("event_id", "")).replace("_", " ").to_upper(),
	]
	if selection.get("route_id") == "stabilize":
		_set_guidance("ROUTE ACCEPTED  •  STABILIZE", "Follow the server objective to [3, 0, 3], then continue to the relay door. Event and effects are shown in Routes [R].")
	else:
		_set_guidance("ROUTE ACCEPTED  •  BREACH", "Continue to the relay door at [6, 0, 0]. Event and effects are shown in Routes [R].")


func _apply_route_summary(message: Dictionary) -> void:
	var accepted: bool = _authoritative_state.call("apply_route_summary", message)
	_refresh_route_console()
	if not accepted:
		_status_label.text = "ROUTE SUMMARY REJECTED  •  INVALID SERVER DATA"
		return
	if message.get("outcome") == "succeeded":
		_status_label.text = "ROUTE SUCCEEDED  •  %s  •  %d MS" % [
			str(message.get("route_id", "")).to_upper(),
			int(message.get("elapsed_ms", 0)),
		]
	else:
		_status_label.text = "ROUTE FAILED  •  DEADLINE EXCEEDED  •  NO ROUTE REWARD"
		_set_guidance("ROUTE FAILED", "The authoritative deadline elapsed. No route reward was granted; reconnect starts a new run.")


func _refresh_route_console() -> void:
	if _route_console == null:
		return
	_route_console.call(
		"present",
		_authoritative_state.route_state,
		_authoritative_state.route_pending,
		_authoritative_state.route_result,
		_authoritative_state.route_summary,
		_authoritative_state.player_actor_id
	)


func _toggle_cooperation_console() -> void:
	if _cooperation_console.visible:
		_close_cooperation_console()
	else:
		_open_cooperation_console()


func _open_cooperation_console() -> void:
	if _campaign_session or not _challenge_contract.is_empty():
		return
	if _module_workshop != null:
		_module_workshop.call("close_workshop")
	if _route_console != null:
		_route_console.call("close_console")
	_cooperation_console.call("open_console")
	_request_cooperation_state()


func _close_cooperation_console() -> void:
	_cooperation_console.call("close_console")
	if _cooperation_button != null:
		_cooperation_button.grab_focus()


func _request_cooperation_state() -> void:
	if not _authoritative_state.call("begin_cooperation_state_request"):
		_refresh_cooperation_console()
		return
	_refresh_cooperation_console()
	if not _send_message({"type": "CooperationStateRequest"}):
		_authoritative_state.call("fail_cooperation_request", "cooperation state request could not be sent", true)
		_refresh_cooperation_console()


func _request_cooperation_start() -> void:
	var operation_id := _new_cooperation_operation_id("start")
	if not _authoritative_state.call("begin_cooperation_start", operation_id):
		return
	_refresh_cooperation_console()
	if not _send_message({"type": "CooperationStartIntent", "operation_id": operation_id}):
		_authoritative_state.call("fail_cooperation_request", "cooperation start could not be sent", true)
		_refresh_cooperation_console()


func _request_cooperation_ping() -> void:
	var operation_id := _new_cooperation_operation_id("ping")
	if not _authoritative_state.call("begin_cooperation_ping", operation_id):
		return
	_refresh_cooperation_console()
	if not _send_message({"type": "CooperationPingIntent", "operation_id": operation_id}):
		_authoritative_state.call("fail_cooperation_request", "cooperation ping could not be sent", true)
		_refresh_cooperation_console()


func _request_cooperation_revive() -> void:
	var operation_id := _new_cooperation_operation_id("revive")
	if not _authoritative_state.call("begin_cooperation_revive", operation_id):
		return
	_refresh_cooperation_console()
	if not _send_message({"type": "CooperationReviveIntent", "operation_id": operation_id}):
		_authoritative_state.call("fail_cooperation_request", "cooperation revive could not be sent", true)
		_refresh_cooperation_console()


func _new_cooperation_operation_id(kind: String) -> String:
	_cooperation_operation_counter += 1
	var entropy := Crypto.new().generate_random_bytes(6)
	if entropy.size() == 6:
		return "godot-c-%s-%s" % [kind.left(1), entropy.hex_encode()]
	return "godot-c-%s-%d" % [kind.left(1), _cooperation_operation_counter]


func _apply_cooperation_state(message: Dictionary) -> void:
	var accepted: bool = _authoritative_state.call("apply_cooperation_state", message)
	_refresh_cooperation_console()
	if not accepted:
		_status_label.text = "CO-OP STATE REJECTED  •  %s" % _authoritative_state.cooperation_result.get("message", "SERVER REJECTED REQUEST")


func _apply_cooperation_start_result(message: Dictionary) -> void:
	var accepted: bool = _authoritative_state.call("apply_cooperation_start_result", message)
	_refresh_cooperation_console()
	_status_label.text = "CO-OP START %s  •  %s" % ["ACCEPTED" if accepted else "REJECTED", str(message.get("message", "SERVER RESPONSE"))]


func _apply_cooperation_ping_result(message: Dictionary) -> void:
	var accepted: bool = _authoritative_state.call("apply_cooperation_ping_result", message)
	_refresh_cooperation_console()
	_status_label.text = "CO-OP PING %s  •  %s" % ["ACCEPTED" if accepted else "REJECTED", str(message.get("message", "SERVER RESPONSE"))]


func _apply_cooperation_life_state(message: Dictionary) -> void:
	if not _authoritative_state.call("apply_cooperation_life_state", message):
		_refresh_cooperation_console()
		_status_label.text = "CO-OP LIFE REJECTED  •  INVALID SERVER DATA"
		return
	_refresh_cooperation_console()
	var actor_id := int(message.get("actor_id", 0))
	var actor: Node = _actors.get(actor_id)
	if actor != null and actor.has_method("set_authoritative_life"):
		actor.call("set_authoritative_life", str(message.get("life_after", "active")))
	if actor_id == _authoritative_state.player_actor_id:
		_player_health_bar.value = int(message.get("health_after", 0))
		_health_label.text = "HP  %03d / %03d  •  %s" % [int(message.get("health_after", 0)), int(message.get("max_health", 0)), str(message.get("life_after", "")).to_upper()]
	_status_label.text = "CO-OP LIFE  •  ACTOR %d  •  %s → %s" % [actor_id, str(message.get("life_before", "")).to_upper(), str(message.get("life_after", "")).to_upper()]


func _apply_cooperation_revive_result(message: Dictionary) -> void:
	var accepted: bool = _authoritative_state.call("apply_cooperation_revive_result", message)
	_refresh_cooperation_console()
	_status_label.text = "REVIVE %s  •  %s" % [str(message.get("status", "REJECTED")).to_upper() if accepted else "REJECTED", str(message.get("message", "SERVER RESPONSE"))]


func _apply_cooperation_summary(message: Dictionary) -> void:
	if not _authoritative_state.call("apply_cooperation_summary", message):
		_refresh_cooperation_console()
		_status_label.text = "CO-OP SUMMARY REJECTED  •  INVALID SERVER DATA"
		return
	_refresh_cooperation_console()
	if message.get("outcome") == "succeeded":
		_status_label.text = "CO-OP SUCCEEDED  •  %d MS  •  EQUAL SERVER GRANTS" % int(message.get("terminal_elapsed_ms", 0))
	else:
		_status_label.text = "CO-OP %s  •  NO REWARD" % str(message.get("outcome", "failed")).replace("_", " ").to_upper()


func _refresh_cooperation_console() -> void:
	if _cooperation_console == null:
		return
	_cooperation_console.call(
		"present",
		_authoritative_state.cooperation_state,
		_authoritative_state.cooperation_pending,
		_authoritative_state.cooperation_result,
		_authoritative_state.cooperation_life_states,
		_authoritative_state.cooperation_summary,
		_authoritative_state.player_actor_id
	)


func _any_gameplay_surface_open() -> bool:
	return (
		_action_bar_focused
		or (_entry_shell != null and _entry_shell.visible)
		or (_settings_panel != null and _settings_panel.visible)
		or
		(_module_workshop != null and _module_workshop.visible)
		or (_route_console != null and _route_console.visible)
		or (_cooperation_console != null and _cooperation_console.visible)
		or (_relay_archive != null and _relay_archive.visible)
		or (_story_panel != null and _story_panel.visible)
	)


func _other_surface_open(current: Control) -> bool:
	return (
		(_module_workshop != null and _module_workshop != current and _module_workshop.visible)
		or (_route_console != null and _route_console != current and _route_console.visible)
		or (_cooperation_console != null and _cooperation_console != current and _cooperation_console.visible)
		or (_relay_archive != null and _relay_archive != current and _relay_archive.visible)
		or (_story_panel != null and _story_panel != current and _story_panel.visible)
	)


func _coolant_available() -> bool:
	return _signal_available() and _authoritative_state.weapon_profiles.has("coil_lance") and not _authoritative_state.objectives.has("coolant_intake")


func _meridian_active() -> bool:
	return _authoritative_state.objectives.has("meridian_arrival")


func _meridian_available() -> bool:
	return _session != null and _session.get("exploration_capable") and _signal_available()


func _refresh_meridian() -> void:
	if _meridian_sector == null or _hud_canvas == null:
		return
	var active := _meridian_active()
	var available := _meridian_available()
	var actor: Dictionary = _authoritative_state.actors.get(_authoritative_state.player_actor_id, {})
	var p: Array = actor.get("position", [0, 0, 0])
	var player := Vector3(p[0], p[1], p[2])
	var playing: bool = _hud_canvas.visible and not _showcase_mode
	var zone: String = _meridian_sector.call("present", available and playing, active and playing, player, _authoritative_state.objectives, _settings)
	_environment.call("set_meridian_access", (active or available) and playing)
	_environment.call("set_meridian_inside", not zone.is_empty())
	_meridian_map.visible = not zone.is_empty() and not _any_gameplay_surface_open()
	if not _showcase_mode:
		var inventory_visible: bool = zone.is_empty() and not _training_fighting() and _challenge_contract != "coolant_recovery"
		_inventory_panel.visible = inventory_visible
		_inventory_label.visible = inventory_visible
		_progression_label.visible = inventory_visible
	if _meridian_map.visible:
		_meridian_map.call("present", _meridian_sector.get("layout"), player, _authoritative_state.objectives, _settings, Rect2(_inventory_panel.position, _inventory_panel.size))
		if _challenge_contract.is_empty() and not (_campaign_session and _authoritative_state.activity_complete):
			_status_label.text = "MERIDIAN • SAFE EXPLORATION"
	if zone.is_empty() and playing and (available or active) and player.x < -6:
		zone = "approach"
	if zone == _meridian_zone:
		return
	var previous := _meridian_zone
	_meridian_zone = zone
	_audio_director.call("set_exploration_zone", zone)
	if zone.is_empty() and previous.is_empty():
		return
	if zone.is_empty() and _camera_phase == "core":
		return
	if zone.is_empty() and not previous.is_empty() and not _campaign_session:
		_status_label.text = "RELAY DOOR UNLOCKED  •  MOVE TO X=6"
	if _camera_transition != null and _camera_transition.is_valid():
		_camera_transition.kill()
	var center: Vector3 = {"approach": Vector3(-9, 0.6, 0), "arrival": Vector3(-17, 0.6, 0), "lens": Vector3(-27, 0.6, -6), "gallery": Vector3(-27, 0.6, 6), "bridge": Vector3(-27, 0.6, 0)}.get(zone, Vector3(2.2, 0.7, 1))
	var target := Transform3D(Basis.IDENTITY, center + Vector3(6, 10.5, 10)).looking_at(center, Vector3.UP)
	_camera_phase = "meridian_" + zone if not zone.is_empty() else "breach"
	if _settings.get("reduced_motion", false) or _settings.get("reduced_flash", false):
		_camera.transform = target
	else:
		_camera_transition = create_tween()
		_camera_transition.tween_property(_camera, "transform", target, 0.45)
	if not zone.is_empty():
		var title: String = _meridian_sector.TITLES.get(zone, "WEST PASSAGE")
		_sector_transition.call("present", "SECTOR 03 • MERIDIAN", title, Color("edcf88"))
	_refresh_onboarding()


func _coolant_phase() -> String:
	var objectives: Dictionary = _authoritative_state.objectives
	if objectives.get("coolant_delivery", {}).get("state") == "Completed":
		return "completed"
	for id in ["coolant_transfer", "coolant_delivery"]:
		if objectives.get(id, {}).get("state") == "Failed":
			return "expired"
	if objectives.get("coolant_delivery", {}).get("state") == "Active":
		return "delivery"
	if objectives.get("coolant_transfer", {}).get("state") == "Active":
		return "transfer"
	return ""


func _refresh_coolant_site() -> void:
	if _coolant_site == null:
		return
	var recovery := _challenge_contract == "coolant_recovery"
	_coolant_site.call("present", recovery or _coolant_available(), _coolant_phase(), recovery)
	_coolant_site.visible = _coolant_site.visible and _hud_canvas.visible and not _showcase_mode


func _present_coolant_objective() -> void:
	var phase := _coolant_phase()
	var progress: int = {"transfer": 1, "delivery": 2, "completed": 3}.get(phase, 0)
	_objective_label.text = "COOLANT RUN  •  %d / 3" % progress
	_status_label.text = "COOLANT  •  %s" % phase.to_upper()
	if _camera_phase != "coolant":
		_camera_phase = "coolant"
		if _camera_transition != null and _camera_transition.is_valid():
			_camera_transition.kill()
		var target := Transform3D(Basis.IDENTITY, Vector3(7, 10, 6)).looking_at(Vector3(1.5, 0.8, -3), Vector3.UP)
		if _settings.get("reduced_motion", false) or _settings.get("reduced_flash", false):
			_camera.transform = target
		else:
			_camera_transition = create_tween()
			_camera_transition.tween_property(_camera, "transform", target, 0.5)
		_sector_transition.call("present", "NORTH RELAY • OPTIONAL EXCURSION", "COOLANT RUN", Color("35d0d0"))
	_refresh_onboarding()


func _signal_available() -> bool:
	if _campaign_session or not _challenge_contract.is_empty():
		return false
	if _authoritative_state.objectives.has("prism_warden") or _authoritative_state.objectives.has("glass_lancer") or _authoritative_state.objectives.has("steel_bulwark") or _authoritative_state.objectives.has("relay_mender") or not _elite_objective().is_empty():
		return false
	if _authoritative_state.activity_complete or _meridian_active() or _authoritative_state.objectives.has("recover_lost_signal") or _authoritative_state.objectives.has("coolant_intake"):
		return false
	var players := 0
	for actor in _authoritative_state.actors.values():
		if actor.get("actor_kind") == "player":
			players += 1
	return players == 1 and _current_enemy_id == 0 and _authoritative_state.objectives.get("reach_relay_door", {}).get("state") == "Active" and _authoritative_state.route_state.get("phase", "choice_open") == "choice_open"


func _refresh_signal_site() -> void:
	if _gauntlet_site != null:
		_gauntlet_site.visible = _challenge_contract == "relay_gauntlet" and _hud_canvas.visible and _authoritative_state.gauntlet_state.get("phase") == "transfer"
	_breach_site = _campaign_site(_breach_site, "the_breach", "res://presentation/environment/breach_site.gd")
	_counter_site = _campaign_site(_counter_site, "counter_signal", "res://presentation/environment/counter_site.gd")
	_prism_core_site = _campaign_site(_prism_core_site, "prism_core", "res://presentation/environment/prism_core_site.gd")
	if _prism_core_site != null:
		_prism_core_site.call("present", _campaign_session and _campaign_chapter == "prism_core" and _hud_canvas.visible, _campaign_checkpoint(), _authoritative_state.activity_complete)
	if _breach_site != null:
		var guard: Node3D = _actors.get(_current_enemy_id)
		_breach_site.call("present", _campaign_session and _campaign_chapter == "the_breach" and _hud_canvas.visible, _campaign_checkpoint(), _authoritative_state.activity_complete, guard.position if guard != null else Vector3(8, 0, 0), _authoritative_state.campaign_state.get("story", {}).get("core") == "direct")
	if _counter_site != null:
		_counter_site.call("present", _campaign_session and _campaign_chapter == "counter_signal" and _hud_canvas.visible, _campaign_checkpoint(), _authoritative_state.activity_complete)
	if _signal_site == null:
		return
	if _challenge_contract == "last_reserve":
		_signal_site.call("present_survival", _authoritative_state.survival_state)
	elif _challenge_contract == "distant_signal":
		_signal_site.call("present_challenge")
	elif _campaign_session and _campaign_chapter == "broken_supply_line":
		_signal_site.call("present_supply", _campaign_checkpoint(), _authoritative_state.activity_complete, _authoritative_state.campaign_state.get("story", {}).get("supply") == "service")
	else:
		_signal_site.call("present", _signal_available(), _authoritative_state.objectives.get("recover_lost_signal", {}))
	_signal_site.visible = _signal_site.visible and _hud_canvas.visible and not _showcase_mode


func _campaign_site(site: Node3D, chapter: String, path: String) -> Node3D:
	if _campaign_session and _campaign_chapter == chapter:
		if site == null:
			site = load(path).new()
			add_child(site)
		return site
	if site != null:
		site.queue_free()
	return null


func _present_training_objective(objective: Dictionary) -> void:
	if _challenge_contract == "relay_gauntlet":
		_refresh_challenge_guidance()
		return
	if not _challenge_contract.is_empty():
		var phase := "challenge_bastion" if _challenge_contract == "bastion_link" else "lancer"
		if _camera_phase != phase:
			if _camera_transition != null and _camera_transition.is_valid(): _camera_transition.kill()
			_camera_phase = phase
			var center := Vector3(11, 3, -7.5) if phase == "challenge_bastion" else Vector3(6, 3.4, 7.5)
			var position := Vector3(11, 23, 8) if phase == "challenge_bastion" else center + Vector3(8, 12.6, 17.5)
			_camera.transform = Transform3D(Basis.IDENTITY, position).looking_at(center, Vector3.UP)
		_refresh_challenge_guidance()
		_apply_training_layout()
		return
	if _campaign_session and _campaign_chapter == "counter_signal":
		_refresh_campaign_guidance()
		_apply_training_layout()
		return
	var active: bool = objective.get("state") == "Active"
	var elite: bool = ELITE_NAMES.has(objective.get("objective_id"))
	var bulwark: bool = objective.get("objective_id") == "steel_bulwark" or elite
	var support: bool = objective.get("objective_id") == "relay_mender"
	_objective_label.text = "GLASS LANCER • OPTIONAL ENCOUNTER" if active else "GLASS LANCER • CLEARED" if objective.get("state") == "Completed" else "GLASS LANCER • WITHDRAWN"
	if bulwark:
		_objective_label.text = "STEEL BULWARK • OPTIONAL ENCOUNTER" if active else "STEEL BULWARK • CLEARED" if objective.get("state") == "Completed" else "STEEL BULWARK • WITHDRAWN"
	if support:
		_objective_label.text = tr("MENDER PAIR • %d/2 DEFEATED") % int(objective.get("progress", 0)) if active else "MENDER PAIR • CLEARED" if objective.get("state") == "Completed" else "MENDER PAIR • WITHDRAWN"
	if elite:
		var title := tr(ELITE_NAMES[objective.objective_id])
		_objective_label.text = tr("%s • %d/2 DEFEATED") % [title, int(objective.get("progress", 0))] if active else tr("%s • CLEARED") % title if objective.get("state") == "Completed" else tr("%s • WITHDRAWN") % title
	if objective.get("state") == "Failed" and _authoritative_state.actor_health.get(_authoritative_state.player_actor_id, 0) <= 0:
		_session.call("reset_connection")
		_connection_started = false
		_hud_canvas.visible = false
		var reason := "The Bulwark stopped you. Flank the shield or attack when it lowers. Your existing items are safe." if bulwark else "The Lancer caught you. Step sideways when the line appears, then attack during recovery. Your existing items are safe."
		if support:
			reason = "The pair stopped you. Defeat the Mender to stop repairs and step off the charge line. Your existing items are safe."
		if elite:
			reason = "The elite pair stopped you. Choose your target, avoid the marked danger and use the recovery windows. Your existing items are safe."
		_entry_shell.call("show_failure", reason)
		return
	_camera_phase = ("bulwark" if bulwark else "lancer") if active else "breach"
	if _camera_transition != null and _camera_transition.is_valid():
		_camera_transition.kill()
	var center := Vector3(6, 0.6, 7.5) if active else Vector3(2.2, 0.7, 1)
	var location := Vector3(12, 12, 20) if active else Vector3(7.8, 9.3, 11)
	if active and bulwark:
		center = Vector3(7.5, 3, -7.5)
		location = Vector3(13.5, 12, 5)
	if active and support:
		center = Vector3(6, 3.4, 7.5)
		location = Vector3(14, 16, 25)
	if active and elite:
		center = Vector3(7.5, 3.4, -7.5)
		location = Vector3(14, 16, 10)
	var target := Transform3D(Basis.IDENTITY, location).looking_at(center, Vector3.UP)
	if (active and bulwark) or _settings.get("reduced_motion", false) or _settings.get("reduced_flash", false):
		_camera.transform = target
	else:
		_camera_transition = create_tween()
		_camera_transition.tween_property(_camera, "transform", target, 0.5)
	_status_label.text = "GLASS LANCER • WATCH THE FLOOR" if active else "CORE EAST • CONTINUE WHEN READY"
	if active and bulwark:
		_status_label.text = "SHIELD UP • FLANK"
	if active and support:
		_status_label.text = "MENDER PAIR • CHOOSE A TARGET"
	_refresh_onboarding()
	_apply_training_layout()


func _prism_fighting() -> bool:
	return _authoritative_state.objectives.get("prism_warden", {}).get("state") == "Active"


func _prism_guidance() -> Array[String]:
	var cue: Dictionary = _authoritative_state.prism_state
	var pattern: Dictionary = cue.get("pattern") if cue.get("pattern") is Dictionary else {}
	match cue.get("mode"):
		"Recovery": return ["SHELL OPEN • ATTACK", "Attack now. The shell closes before the next warning."]
		"Shifting": return ["PHASE II • CORE PULSES", "First leave the center, then return. Attack after both pulses."]
		"PulseGap": return ["PREPARE TO RETURN", "Center pulse resolved. Return to the center for the next warning."]
		"Warning":
			if pattern.get("shape") == "Center": return ["LEAVE THE CENTER", "Stand outside the marked center until the pulse resolves."]
			if pattern.get("shape") == "Perimeter": return ["RETURN TO THE CENTER", "Stand in the clear center. Attack after the outer pulse."]
	if _challenge_contract == "relay_gauntlet":
		return ["MOVE OFF THE STRIPE", "Step off the stripe. Attack when the shell opens. The board restarts all three stages."]
	if _challenge_contract == "prism_discipline":
		return ["MOVE OFF THE STRIPE", "Step off the stripe. Attack when the shell opens. The board restarts the whole fight."]
	return ["MOVE OFF THE STRIPE", "Step off the stripe. Attack when the shell opens. Chapter menu restarts this fight." if _campaign_session else "Step off the stripe. Attack when the shell opens. Leave the square to retreat."]


func _handle_prism_state(message: Dictionary) -> void:
	_authoritative_state.call("apply_prism_state", message)
	var actor: Node = _actors.get(message.get("actor_id"))
	if actor != null and actor.has_method("present_prism"):
		actor.call("present_prism", message)
	var pattern: Dictionary = message.get("pattern") if message.get("pattern") is Dictionary else {}
	if message.get("mode") == "Warning":
		_audio_director.call("play_prism_cue", {"Center": "center", "Perimeter": "perimeter"}.get(pattern.get("shape"), "lane"))
	elif message.get("mode") in ["Shifting", "Recovery"]:
		_audio_director.call("play_prism_cue", "shift" if message.get("mode") == "Shifting" else "recovery")
	if not _prism_fighting(): return
	var phase: int = 2 if message.get("phase") == "Pulses" else 1
	_objective_label.text = tr("PRISM WARDEN • PHASE %d/2") % phase
	_status_label.text = _prism_guidance()[0]
	_refresh_onboarding()


func _present_prism_objective(objective: Dictionary) -> void:
	if _challenge_contract == "relay_gauntlet":
		_refresh_challenge_guidance()
		return
	if _challenge_contract == "prism_discipline":
		_environment.call("set_prism_encounter", true)
		if _camera_phase != "challenge_prism":
			if _camera_transition != null and _camera_transition.is_valid(): _camera_transition.kill()
			_camera_phase = "challenge_prism"
			var large_ui: bool = _settings.get("ui_scale", 1.0) > 1.0
			_camera.transform = Transform3D(Basis.IDENTITY, Vector3(10, 20, 10) if large_ui else Vector3(8, 24, 10)).looking_at(Vector3(10, 4, 0) if large_ui else Vector3(8, 3.6, 0), Vector3.UP)
		_refresh_challenge_guidance()
		_apply_training_layout()
		return
	var active: bool = objective.get("state") == "Active"
	_environment.call("set_prism_encounter", active or (_campaign_session and _campaign_chapter == "prism_core"))
	if objective.get("state") == "Failed" and _authoritative_state.actor_health.get(_authoritative_state.player_actor_id, 0) <= 0:
		_session.call("reset_connection")
		_connection_started = false
		_hud_canvas.visible = false
		_entry_shell.call("show_failure", "The Prism Warden stopped you. Follow the floor warnings and attack when its shell opens. Your existing items are safe.")
		return
	_objective_label.text = tr("PRISM WARDEN • PHASE %d/2") % 1 if active else "PRISM WARDEN • CLEARED" if objective.get("state") == "Completed" else "PRISM WARDEN • WITHDRAWN"
	_camera_phase = "prism" if active else "core" if objective.get("state") == "Completed" else "breach"
	if _camera_transition != null and _camera_transition.is_valid(): _camera_transition.kill()
	if active:
		var large_ui: bool = _settings.get("ui_scale", 1.0) > 1.0
		_camera.transform = Transform3D(Basis.IDENTITY, Vector3(8, 30, 10) if large_ui else Vector3(8, 24, 10)).looking_at(Vector3(8, 9, 0) if large_ui else Vector3(8, 3.6, 0), Vector3.UP)
	elif objective.get("state") == "Failed":
		_camera.transform = Transform3D(Basis.IDENTITY, Vector3(7.8, 9.3, 11)).looking_at(Vector3(2.2, 0.7, 1), Vector3.UP)
	_refresh_onboarding()
	_apply_training_layout()


func _training_fighting() -> bool:
	if _challenge_contract == "relay_gauntlet":
		return true
	if _challenge_contract in ["distant_signal", "last_reserve"] and not _authoritative_state.activity_complete:
		return true
	if _campaign_session and _campaign_chapter == "prism_core" and not _authoritative_state.activity_complete:
		return true
	if _campaign_session and _campaign_chapter == "the_breach" and not _authoritative_state.activity_complete:
		return _campaign_checkpoint() in [1, 2]
	return _authoritative_state.objectives.get("glass_lancer", {}).get("state") == "Active" or _authoritative_state.objectives.get("steel_bulwark", {}).get("state") == "Active" or _authoritative_state.objectives.get("relay_mender", {}).get("state") == "Active" or _elite_objective().get("state") == "Active" or _prism_fighting()


func _elite_objective() -> Dictionary:
	for id in ELITE_NAMES:
		if _authoritative_state.objectives.has(id):
			return _authoritative_state.objectives[id]
	return {}


func _apply_training_layout() -> void:
	if _sound_captions == null or _showcase_mode:
		return
	var active := _training_fighting()
	var factor: float = _settings.get("ui_scale", 1.0)
	var equipment_rect := Rect2(24, _telemetry_panel.position.y + _telemetry_panel.size.y + 12, 340, 150) if active and factor > 1.0 else Rect2(244, _action_grid.position.y - 68, 1012, 62)
	_layout_text(_equipment_label, equipment_rect, 16 * factor)
	var width := _guide_panel.size.x if active else _telemetry_panel.size.x
	_sound_captions.position = Vector2(_guide_panel.position.x, _guide_panel.position.y + _guide_panel.size.y + 12) if active else Vector2(24, _telemetry_panel.position.y + _telemetry_panel.size.y + 12)
	_sound_captions.custom_minimum_size.x = width
	_sound_captions.size.x = width
	_sound_captions.call_deferred("reset_size")


func _present_signal_objective(objective: Dictionary) -> void:
	_objective_label.text = "LOST SIGNAL  •  %d / 2" % int(objective.get("progress", 0))
	if objective.get("state") == "Failed":
		_session.call("reset_connection")
		_connection_started = false
		_hud_canvas.visible = false
		_entry_shell.call("show_failure", "The sentinel stopped the recovery. Use the barrier and approach around its ends. Your existing items are safe.")
		return
	if objective.get("state") == "Completed":
		_status_label.text = "LOST SIGNAL  •  RECOVERED"
		_refresh_onboarding()
		return
	_refresh_onboarding()
	if _camera_phase != "signal":
		_camera_phase = "signal"
		if _camera_transition != null and _camera_transition.is_valid():
			_camera_transition.kill()
		var target := Transform3D(Basis.IDENTITY, Vector3(2, 9, 9)).looking_at(Vector3(-4, 0.8, -1), Vector3.UP)
		if _settings.get("reduced_motion", false) or _settings.get("reduced_flash", false):
			_camera.transform = target
		else:
			_camera_transition = create_tween()
			_camera_transition.tween_property(_camera, "transform", target, 0.6)
		_sector_transition.call("present", "WEST RELAY  •  OPTIONAL EXCURSION", "THE LOST SIGNAL", Color("f5a524"))


func _refresh_archive_context() -> void:
	if not _challenge_contract.is_empty():
		_archive_button.visible = false
		_archive_hint.visible = false
		_story_markers.visible = false
		return
	if _story_from_menu:
		_refresh_story_context()
		return
	if _relay_archive == null:
		return
	if _campaign_session:
		_relay_archive.call("update_context", Vector3.ZERO, false, false, true)
		_refresh_story_context()
		return
	_story_markers.visible = false
	var player: Dictionary = _authoritative_state.actors.get(_authoritative_state.player_actor_id, {})
	var position: Array = player.get("position", [0, 0, 0])
	var complete: bool = _authoritative_state.activity_complete
	var cleared: bool = _authoritative_state.objectives.get("clear_drone_group", {}).get("state") == "Completed"
	var safe: bool = (cleared or complete) and not player.is_empty() and _authoritative_state.actor_health.get(_authoritative_state.player_actor_id, 0) > 0
	for actor_id in _authoritative_state.actors:
		if _authoritative_state.actors[actor_id].get("actor_kind") == "enemy" and _authoritative_state.actor_health.get(actor_id, 0) > 0:
			safe = false
	var suspended := not _hud_canvas.visible or _showcase_mode or _other_surface_open(_relay_archive)
	suspended = suspended or (_entry_shell != null and _entry_shell.visible) or (_settings_panel != null and _settings_panel.visible)
	suspended = suspended or (not complete and _session.call("connection_status") != StreamPeerTCP.STATUS_CONNECTED)
	_relay_archive.call("update_meridian_memory", _authoritative_state.objectives.get("meridian_memory", {}).get("state") == "Completed")
	_relay_archive.call("update_context", Vector3(position[0], position[1], position[2]), safe, complete, suspended)
	var prompt: String = _relay_archive.call("prompt_text")
	if _settings.has("bindings"):
		var bindings: Dictionary = _settings.bindings
		prompt = prompt.replace("[E]", "[%s]" % preload("res://input/input_bindings.gd").key_label(bindings, "interact"))
		prompt = prompt.replace("[J]", "[%s]" % preload("res://input/input_bindings.gd").key_label(bindings, "archive"))
	_archive_button.text = prompt
	_archive_button.disabled = not _relay_archive.call("can_open")
	_archive_hint.text = _relay_archive.call("hint_text")


func _open_relay_archive(nearby := false) -> void:
	_refresh_archive_context()
	if _campaign_session:
		if _story_panel.call("open_journal", nearby): _player_intents.call("clear_pending_input")
		return
	if _relay_archive.call("open_archive", nearby):
		_player_intents.call("clear_pending_input")


func _close_relay_archive() -> void:
	_relay_archive.call("close_archive")
	_player_intents.call("clear_pending_input")
	get_viewport().gui_release_focus()


func _rail_target_too_close(target_id: int) -> bool:
	if _authoritative_state.equipped_weapon_item_id != "rail_driver":
		return false
	var player: Dictionary = _authoritative_state.actors.get(_authoritative_state.player_actor_id, {})
	var target: Dictionary = _authoritative_state.actors.get(target_id, {})
	if player.is_empty() or target.is_empty():
		return false
	var a: Array = player.get("position", [0, 0, 0])
	var b: Array = target.get("position", [0, 0, 0])
	return Vector3(a[0], a[1], a[2]).distance_squared_to(Vector3(b[0], b[1], b[2])) < 16.0


func _current_attack_cooldown_ms() -> int:
	var profile: Dictionary = _authoritative_state.weapon_profiles.get(_authoritative_state.equipped_weapon_item_id, {})
	return int(profile.get("cooldown_ms", 260))


func _sync_attack_cooldown() -> void:
	var cooldown_ms := _current_attack_cooldown_ms()
	if not _player_intents.call("set_attack_cooldown_ms", cooldown_ms):
		push_error("authoritative weapon cooldown is outside the supported local input range: %d" % cooldown_ms)


func _update_operator_weapon() -> void:
	var player: Node = _actors.get(_authoritative_state.player_actor_id)
	if player != null and player.has_method("set_weapon"):
		player.call("set_weapon", _authoritative_state.equipped_weapon_item_id)


func _set_door_open(open: bool) -> void:
	_environment.call("set_core_door_open", open, true)
	_status_label.text = "RELAY CORE OPEN  •  WARDEN INBOUND" if open else "RELAY CORE SEALED"
	_audio_director.call("apply_door_state", open, _door.global_position)
	if open:
		_environment.call("set_presentation_phase", "core", true)
		_sector_transition.call("present", "SECTOR 02  •  RELAY HUB", "RELAY CORE", Color("f5a524"))
		_focus_camera_on_core()
		_set_guidance("CORE OPEN", "The Warden is spawning. Get ready to aim and attack.")


func _focus_camera_on_core() -> void:
	if _camera == null:
		return
	_camera_phase = "core"
	if _showcase_mode:
		return
	if _camera_transition != null and _camera_transition.is_valid():
		_camera_transition.kill()
	var target_position := Vector3(9.8, 8.4, 8.5)
	var target_transform := Transform3D(Basis.IDENTITY, target_position).looking_at(Vector3(7.4, 0.8, 0.0), Vector3.UP)
	var reduced_motion: bool = _settings.get("reduced_motion", false) or _settings.get("reduced_flash", false)
	if reduced_motion:
		_camera.transform = target_transform
		return
	_camera_transition = create_tween()
	_camera_transition.tween_property(_camera, "transform", target_transform, 0.72).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_IN_OUT)


func _build_playable_scene() -> void:
	_environment = RELAY_HUB_ROOM_SCENE.instantiate()
	add_child(_environment)
	_signal_site = SIGNAL_SITE.new()
	_signal_site.name = "LostSignalSite"
	add_child(_signal_site)
	_coolant_site = preload("res://presentation/environment/coolant_site.gd").new()
	_coolant_site.name = "CoolantSite"
	add_child(_coolant_site)
	_lancer_site = preload("res://presentation/environment/lancer_site.gd").new()
	_lancer_site.name = "LancerSite"
	add_child(_lancer_site)
	_bulwark_site = preload("res://presentation/environment/lancer_site.gd").new()
	_bulwark_site.name = "BulwarkSite"
	_bulwark_site.set("bulwark", true)
	add_child(_bulwark_site)
	_support_site = preload("res://presentation/environment/lancer_site.gd").new()
	_support_site.name = "SupportSite"
	_support_site.set("support", true)
	add_child(_support_site)
	for id in ELITE_NAMES:
		var site := preload("res://presentation/environment/lancer_site.gd").new()
		site.set("elite_id", id)
		_elite_sites[id] = site
		add_child(site)
	_prism_site = preload("res://presentation/environment/prism_site.gd").new()
	add_child(_prism_site)
	_meridian_sector = preload("res://presentation/environment/meridian_sector.gd").new()
	_meridian_sector.name = "MeridianSector"
	add_child(_meridian_sector)
	_door = _environment.call("get_core_door")
	_combat_vfx = COMBAT_VFX_SCENE.instantiate()
	add_child(_combat_vfx)
	_loot_pickup = LOOT_PICKUP_PRESENTATION.new()
	_loot_pickup.name = "ConfirmedLootPickup"
	add_child(_loot_pickup)
	_presentation_polish = PRESENTATION_POLISH_SCENE.instantiate()
	add_child(_presentation_polish)
	_sector_transition = SECTOR_TRANSITION.new()
	_sector_transition.name = "SectorTransition"
	add_child(_sector_transition)
	_audio_director = AUDIO_DIRECTOR.new()
	_audio_director.name = "AudioDirector"
	add_child(_audio_director)

	_camera = Camera3D.new()
	_camera.name = "GameplayCamera"
	_camera.fov = 54.0 if _showcase_mode else 58.0
	_camera.position = Vector3(7.8, 9.3, 11.0)
	add_child(_camera)
	_camera.look_at_from_position(_camera.position, Vector3(2.2, 0.7, 1.0), Vector3.UP)

	var canvas := CanvasLayer.new()
	canvas.name = "HUD"
	add_child(canvas)
	_hud_canvas = canvas
	_meridian_map = preload("res://presentation/environment/meridian_map.gd").new()
	_meridian_map.name = "MeridianMap"
	_meridian_map.visible = false
	canvas.add_child(_meridian_map)
	_sound_captions = preload("res://presentation/audio/sound_captions.gd").new()
	canvas.add_child(_sound_captions)
	_audio_director.connect("caption_requested", _sound_captions.request)
	_hud_frame = OPERATOR_HUD_SCENE.instantiate()
	canvas.add_child(_hud_frame)
	_telemetry_panel = _hud_frame.call("add_panel", Rect2(24, 24, 620, 210), "OPERATOR TELEMETRY", Color("35d0d0"))
	_status_label = _hud_label(canvas, Vector2(44, 38), 22, Color("35d0ba"), "CONNECTING TO REVENANT CORE")
	_health_label = _hud_label(canvas, Vector2(44, 72), 26, Color.WHITE, "HP  100 / 100")
	_player_health_bar = _hud_frame.call("make_bar", Rect2(410, 82, 210, 8), Color("35d0d0"))
	_objective_label = _hud_label(canvas, Vector2(44, 116), 18, Color("f5a524"), "OBJECTIVE  •  WAITING FOR ACTIVITY")
	_enemy_health_label = _hud_label(canvas, Vector2(44, 152), 18, Color("e8505b"), "ENEMY  •  WAITING FOR ENCOUNTER")
	_enemy_health_bar = _hud_frame.call("make_bar", Rect2(410, 160, 210, 8), Color("d93678"))
	_position_label = _hud_label(canvas, Vector2(44, 194), 16, Color("a9b8cc"), "POSITION  [0, 0]  •  DOOR 6 STEPS")
	_controls_label = _hud_label(canvas, Vector2(24, 674), 16, Color("a9b8cc"), "CONNECTING...")
	_crosshair = _hud_label(canvas, Vector2(632, 344), 28, Color("f5a524"), "+")
	_crosshair.mouse_filter = Control.MOUSE_FILTER_IGNORE

	_guide_panel = _hud_frame.call("add_panel", Rect2(894, 24, 362, 170), "MISSION GUIDE", Color("f5a524"))
	_guide_title = _hud_label(canvas, Vector2(918, 42), 16, Color("f5a524"), "MISSION GUIDE")
	_guidance_label = _hud_label(canvas, Vector2(918, 72), 18, Color.WHITE, "CONNECTING\n\nWait for the relay-hub connection.")
	_guidance_label.size = Vector2(314, 104)
	_guidance_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	_input_monitor_panel = _hud_frame.call("add_panel", Rect2(894, 214, 362, 188), "INPUT MONITOR", Color("35d0d0"))
	_input_monitor_title = _hud_label(canvas, Vector2(918, 232), 16, Color("35d0ba"), "INPUT MONITOR")
	_input_log_label = _hud_label(canvas, Vector2(918, 262), 14, Color("a9b8cc"), "Click inside the game window.\nWaiting for keyboard or mouse input...")
	_input_log_label.size = Vector2(314, 126)
	_input_log_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	_inventory_panel = _hud_frame.call("add_panel", Rect2(894, 422, 362, 184), "INVENTORY", Color("a9b8cc"))
	_inventory_label = _hud_label(canvas, Vector2(918, 440), 16, Color("a9b8cc"), "INVENTORY\nWAITING FOR SERVER")
	_inventory_label.size = Vector2(314, 102)
	_progression_label = _hud_label(canvas, Vector2(918, 548), 15, Color("35d0ba"), "PROGRESSION  •  WAITING FOR SERVER")
	_progression_label.size = Vector2(314, 58)
	_input_monitor_panel.visible = false
	_input_monitor_title.visible = false
	_input_log_label.visible = false
	_equipment_label = _hud_label(canvas, Vector2(280, 586), 15, Color("35d0ba"), "WEAPON  •  WAITING FOR SERVER")
	_equipment_label.size = Vector2(360, 54)

	_create_control_button(canvas, "W", Vector2(92, 558), Vector2(58, 48), Vector2(0, -1))
	_create_control_button(canvas, "A", Vector2(28, 612), Vector2(58, 48), Vector2(-1, 0))
	_create_control_button(canvas, "S", Vector2(92, 612), Vector2(58, 48), Vector2(0, 1))
	_create_control_button(canvas, "D", Vector2(156, 612), Vector2(58, 48), Vector2(1, 0))
	_attack_button = Button.new()
	_attack_button.text = "ATTACK"
	_attack_button.position = Vector2(1090, 610)
	_attack_button.size = Vector2(150, 54)
	_attack_button.add_theme_font_size_override("font_size", 18)
	_hud_frame.call("style_button", _attack_button, Color("d93678"), false)
	_attack_button.pressed.connect(_request_ui_attack)
	canvas.add_child(_attack_button)
	_create_weapon_button(canvas, "RIFLE", "pulse_rifle", Vector2(280, 642))
	_create_weapon_button(canvas, "SIDEARM", "arc_sidearm", Vector2(380, 642))
	_create_weapon_button(canvas, "LANCE", "coil_lance", Vector2(480, 642))
	_create_weapon_button(canvas, "SCATTER", "scatter_caster", Vector2.ZERO)
	_create_weapon_button(canvas, "RAIL", "rail_driver", Vector2.ZERO)
	_module_button = Button.new()
	_module_button.name = "Modules"
	_module_button.text = "MODULES [M]"
	_module_button.position = Vector2(580, 642)
	_module_button.size = Vector2(124, 40)
	_hud_frame.call("style_button", _module_button, Color("35d0d0"), true)
	_module_button.pressed.connect(_open_module_workshop)
	canvas.add_child(_module_button)
	_route_button = Button.new()
	_route_button.name = "Routes"
	_route_button.text = "ROUTES [R]"
	_route_button.position = Vector2(712, 642)
	_route_button.size = Vector2(124, 40)
	_hud_frame.call("style_button", _route_button, Color("f5a524"), true)
	_route_button.pressed.connect(_open_route_console)
	canvas.add_child(_route_button)
	_cooperation_button = Button.new()
	_cooperation_button.name = "Cooperation"
	_cooperation_button.text = "CO-OP [C]"
	_cooperation_button.position = Vector2(844, 642)
	_cooperation_button.size = Vector2(102, 40)
	_hud_frame.call("style_button", _cooperation_button, Color("d93678"), true)
	_cooperation_button.pressed.connect(_open_cooperation_console)
	canvas.add_child(_cooperation_button)
	_target_button = Button.new()
	_target_button.name = "NextTarget"
	_target_button.text = "NEXT ENEMY"
	_target_button.visible = false
	_hud_frame.call("style_button", _target_button, Color("f5a524"), true)
	_target_button.pressed.connect(_cycle_enemy)
	canvas.add_child(_target_button)
	_build_module_workshop()
	_build_route_console()
	_build_cooperation_console()
	_build_relay_archive()
	_campaign_button = Button.new()
	_campaign_button.name = "CampaignMenu"
	_campaign_button.text = "CHAPTER MENU"
	_campaign_button.visible = false
	_hud_frame.call("style_button", _campaign_button, Color("35d0d0"), true)
	_campaign_button.pressed.connect(_return_to_campaign_menu)
	canvas.add_child(_campaign_button)
	_action_grid = GridContainer.new()
	_action_grid.name = "ActionBar"
	_action_grid.add_theme_constant_override("h_separation", 8)
	_action_grid.add_theme_constant_override("v_separation", 8)
	canvas.add_child(_action_grid)
	for button in _weapon_buttons + [_module_button, _route_button, _cooperation_button, _archive_button, _target_button, _attack_button, _campaign_button]:
		button.reparent(_action_grid)
		button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.size_flags_vertical = Control.SIZE_EXPAND_FILL
	if _showcase_mode:
		_configure_showcase_hud()
	_hud_canvas.visible = false


func _configure_showcase_hud() -> void:
	for button in _movement_buttons:
		button.visible = false
	for button in _weapon_buttons:
		button.visible = false
	_attack_button.visible = false
	_module_button.visible = false
	_route_button.visible = false
	_cooperation_button.visible = false
	_archive_button.visible = false
	_archive_hint.visible = false
	_equipment_label.position = Vector2(44.0, 612.0)
	_equipment_label.size = Vector2(540.0, 32.0)
	_controls_label.text = "LOCAL AUTHORITATIVE PLAYBACK  •  ALL COMBAT, MOVEMENT AND REWARDS SERVER CONFIRMED"


func _update_showcase_camera(delta: float) -> void:
	if _camera == null:
		return
	var focus := Vector3(2.2, 0.8, 1.0)
	var offset := Vector3(5.7, 8.2, 8.7)
	var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
	var enemy: Node3D = _actors.get(_current_enemy_id)
	match _camera_phase:
		"breach":
			focus = Vector3(5.2, 0.75, 0.2) if player == null else player.global_position.lerp(Vector3(6.5, 0.8, 0.0), 0.55)
			offset = Vector3(4.9, 7.5, 7.4)
		"core":
			focus = Vector3(7.4, 0.9, 0.0)
			if player != null and enemy != null:
				focus = player.global_position.lerp(enemy.global_position, 0.58) + Vector3(0.0, 0.85, 0.0)
			offset = Vector3(2.2, 7.4, 8.6)
		"reward":
			focus = Vector3(7.0, 1.15, 0.0)
			offset = Vector3(1.8, 6.4, 7.2)
		"secured":
			focus = Vector3(8.2, 1.0, 0.0)
			offset = Vector3(2.6, 7.5, 8.4)
		_:
			if player != null and enemy != null:
				focus = player.global_position.lerp(enemy.global_position, 0.52) + Vector3(0.0, 0.7, 0.0)
	var desired_position := focus + offset
	var desired_transform := Transform3D(Basis.IDENTITY, desired_position).looking_at(focus, Vector3.UP)
	_camera.transform = _camera.transform.interpolate_with(desired_transform, clampf(delta * 2.4, 0.0, 1.0))


func _present_showcase_completion() -> void:
	await get_tree().create_timer(1.35).timeout
	_camera_phase = "secured"
	_sector_transition.call("present", "SECTOR SECURED", "RELAY HUB ONLINE", Color("35d0d0"))


func _build_module_workshop() -> void:
	var canvas := CanvasLayer.new()
	canvas.name = "ModuleWorkshop"
	canvas.layer = 15
	add_child(canvas)
	_module_workshop = MODULE_WORKSHOP.new()
	_module_workshop.name = "ModuleWorkshop"
	canvas.add_child(_module_workshop)
	_module_workshop.connect("preview_requested", _request_module_preview)
	_module_workshop.connect("combine_requested", _request_module_combination)
	_module_workshop.connect("loadout_requested", _request_module_loadout)
	_module_workshop.connect("claim_requested", _request_acquisition_claim)
	_module_workshop.connect("close_requested", _close_module_workshop)


func _build_route_console() -> void:
	var canvas := CanvasLayer.new()
	canvas.name = "RouteConsole"
	canvas.layer = 16
	add_child(canvas)
	_route_console = ROUTE_CONSOLE.new()
	_route_console.name = "RouteConsole"
	canvas.add_child(_route_console)
	_route_console.connect("state_requested", _request_route_state)
	_route_console.connect("choice_requested", _request_route_choice)
	_route_console.connect("close_requested", _close_route_console)


func _build_cooperation_console() -> void:
	var canvas := CanvasLayer.new()
	canvas.name = "CooperationConsole"
	canvas.layer = 17
	add_child(canvas)
	_cooperation_console = COOPERATION_CONSOLE.new()
	_cooperation_console.name = "CooperationConsole"
	canvas.add_child(_cooperation_console)
	_cooperation_console.connect("state_requested", _request_cooperation_state)
	_cooperation_console.connect("start_requested", _request_cooperation_start)
	_cooperation_console.connect("ping_requested", _request_cooperation_ping)
	_cooperation_console.connect("revive_requested", _request_cooperation_revive)
	_cooperation_console.connect("close_requested", _close_cooperation_console)


func _build_relay_archive() -> void:
	var canvas := CanvasLayer.new()
	canvas.name = "RelayArchive"
	canvas.layer = 18
	add_child(canvas)
	_relay_archive = RELAY_ARCHIVE.new()
	_relay_archive.name = "RelayArchive"
	canvas.add_child(_relay_archive)
	_relay_archive.connect("close_requested", _close_relay_archive)
	_story_panel = preload("res://presentation/campaign/story_panel.gd").new()
	_story_panel.name = "CampaignJournal"
	canvas.add_child(_story_panel)
	_story_panel.connect("close_requested", _close_story_journal)
	_story_panel.connect("action_requested", _request_story_action)
	_story_markers = preload("res://presentation/campaign/story_markers.gd").new()
	add_child(_story_markers)
	var terminals := Node3D.new()
	terminals.name = "ArchiveLandmarks"
	add_child(terminals)
	_relay_archive.call("build_terminals", terminals)
	_archive_button = Button.new()
	_archive_button.name = "Archive"
	_archive_button.position = Vector2(668, 24)
	_archive_button.size = Vector2(208, 44)
	_archive_button.focus_mode = Control.FOCUS_ALL
	_hud_frame.call("style_button", _archive_button, Color("f5a524"), true)
	_archive_button.pressed.connect(func() -> void: _open_relay_archive(_relay_archive.call("presentation_state").nearest >= 0))
	_hud_canvas.add_child(_archive_button)
	_archive_hint = _hud_label(_hud_canvas, Vector2(680, 80), 16, Color("f5a524"), "")
	_archive_hint.size = Vector2(184, 100)
	_archive_hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_archive_hint.mouse_filter = Control.MOUSE_FILTER_IGNORE


func _build_entry_shell() -> void:
	var canvas := CanvasLayer.new()
	canvas.name = "Entry"
	canvas.layer = 10
	add_child(canvas)
	_entry_shell = ENTRY_SHELL_SCENE.instantiate()
	canvas.add_child(_entry_shell)
	_entry_shell.call("configure_endpoint", _connection_host(), _connection_port())
	_entry_shell.connect("connect_requested", _begin_connection)
	_entry_shell.connect("settings_requested", _open_settings)
	_entry_shell.connect("quit_requested", _quit_client.bind(0))


func _return_to_campaign_menu() -> void:
	if not _challenge_contract.is_empty():
		if _challenge_state.snapshot.get("active") is Dictionary:
			if _challenge_abandon_operation.is_empty():
				_challenge_abandon_operation = "godot-abandon-%d-%d" % [int(Time.get_unix_time_from_system() * 1000000), Time.get_ticks_usec()]
				_send_message({"type": "ChallengeAbandonIntent", "operation_id": _challenge_abandon_operation, "expected_revision": _challenge_state.snapshot.state_revision, "run_id": _challenge_state.snapshot.active.run_id})
		else:
			_show_challenge_board()
		return
	_connection_generation += 1
	_session.call("reset_connection")
	_story_panel.visible = false
	_connection_started = false
	_player_intents.call("clear_pending_input")
	_hud_canvas.visible = false
	_entry_shell.call("select_mode", "campaign")
	_entry_shell.call("show_entry", _connection_username)


func _show_challenge_board() -> void:
	_connection_generation += 1
	_session.call("reset_connection")
	_connection_started = false
	_player_intents.call("clear_pending_input")
	_hud_canvas.visible = false
	_challenge_contract = ""
	_entry_shell.call("select_mode", "challenges")
	_entry_shell.call("show_entry", _connection_username)
	# A new authenticated read resolves the final save before presenting records.
	call_deferred("_begin_connection", _connection_username)


func _refresh_challenge_guidance() -> void:
	if _challenge_contract.is_empty() or _challenge_state.snapshot.is_empty(): return
	var saved: Dictionary = _challenge_state.snapshot
	var run: Dictionary = saved.active if saved.active is Dictionary else saved.last_result.run
	var mask: int = run.objectives
	var combat := _challenge_contract in ["close_quarters", "bastion_link", "prism_discipline", "distant_signal", "relay_gauntlet"]
	_enemy_health_label.visible = combat
	_enemy_health_bar.visible = combat
	_attack_button.visible = combat and saved.active is Dictionary and not _showcase_mode
	var title := tr({"close_quarters": "CLOSE QUARTERS", "meridian_circuit": "MERIDIAN CIRCUIT", "coolant_recovery": "COOLANT RECOVERY", "bastion_link": "BASTION LINK", "prism_discipline": "PRISM DISCIPLINE", "distant_signal": "DISTANT SIGNAL", "last_reserve": "LAST RESERVE", "relay_gauntlet": "RELAY GAUNTLET"}[_challenge_contract])
	# Keep the source key for Button's automatic translation on language changes.
	_campaign_button.text = "ABANDON / BOARD" if saved.active is Dictionary else "CHALLENGE BOARD"
	var progress := 0
	for bit in int(run.contract.objective_count):
		if mask & (1 << bit): progress += 1
	_objective_label.text = "%s • %d/%d" % [title, progress, run.contract.objective_count]
	if _challenge_contract == "meridian_circuit":
		_authoritative_state.objectives["meridian_arrival"] = {"state": "Completed"}
		for index in 4:
			_authoritative_state.objectives[["meridian_lens", "meridian_gallery", "meridian_log", "meridian_return"][index]] = {"state": "Completed" if mask & (1 << index) else "Pending" if index == 3 and mask != 7 else "Active"}
		if preload("res://projection/challenge_state.gd").preset_id(run.contract).begins_with("west_approach"):
			_authoritative_state.objectives["meridian_return"]["position"] = [-31, 0, 0]
	if _challenge_contract == "coolant_recovery":
		for index in 3:
			_authoritative_state.objectives[["coolant_intake", "coolant_transfer", "coolant_delivery"][index]] = {"state": "Completed" if mask & (1 << index) else "Active" if index == progress else "Pending"}
		_controls_label.text = tr("WASD MOVE • TAB CONTROLS • HOLD STILL AT STATIONS")
		var actor: Dictionary = _authoritative_state.actors.get(_authoritative_state.player_actor_id, {})
		var p: Array = actor.get("position", [-1, 0, -3])
		_position_label.text = tr("POSITION [%d, %d] • NO TIME LIMIT") % [p[0], p[2]]
		if _camera_phase != "coolant_recovery":
			if _camera_transition != null and _camera_transition.is_valid(): _camera_transition.kill()
			_camera_phase = "coolant_recovery"
			_camera.transform = Transform3D(Basis.IDENTITY, Vector3(6, 11, 11)).looking_at(Vector3(6, 1, -3), Vector3.UP)
	if _challenge_contract in ["distant_signal", "last_reserve"] and _camera_phase != "challenge_signal":
		if _camera_transition != null and _camera_transition.is_valid(): _camera_transition.kill()
		_camera_phase = "challenge_signal"
		var large_ui: bool = _settings.get("ui_scale", 1.0) > 1.0
		_camera.transform = Transform3D(Basis.IDENTITY, Vector3(-2, 32, 10) if large_ui else Vector3(-2, 24, 10)).looking_at(Vector3(-2, 9, 0) if large_ui else Vector3(-2, 3.6, 0), Vector3.UP)
	_apply_training_layout()
	if saved.active == null:
		var outcome: String = {"completed": "CONTRACT COMPLETE", "defeated": "CONTRACT DEFEATED", "abandoned": "CONTRACT ABANDONED", "interrupted": "CONTRACT INTERRUPTED"}.get(saved.last_result.outcome, "CHALLENGES")
		_status_label.text = tr(outcome)
		_set_guidance(title, tr("Attempt saved, without items or XP. Open the board to restart."))
	else:
		_status_label.text = title
		_set_guidance(title, tr("Defeat the Mender. Dodge the Lancer's line and attack during recovery.") if _challenge_contract == "close_quarters" else tr("Read the lens, gallery and log, then return to the entrance. Readings confirm on arrival."))
		if _challenge_contract == "prism_discipline":
			var cue := _prism_guidance()
			_status_label.text = tr(cue[0])
			_set_guidance(title, tr(cue[1]))
			_objective_label.text = tr("PRISM WARDEN • PHASE %d/2") % (2 if _authoritative_state.prism_state.get("phase") == "Pulses" else 1)
		if _challenge_contract == "last_reserve":
			_refresh_survival_guidance()
		if _challenge_contract == "distant_signal":
			_set_guidance(title, tr("Go around either end of cover to fire. Stay inside the marked lane. The board restarts the fight."))
		if _challenge_contract == "bastion_link":
			_set_guidance(title, tr("Stop the Mender’s repairs. Flank the Bulwark’s shield and avoid its marked slam."))
		if _challenge_contract == "coolant_recovery":
			var advice: String = ["Retrieve coolant at station 1 [-1, -5]. No time limit.", "Hold at station 2 [4, -5] for 1.2 s. Moving away resets the hold.", "Hold at station 3 [4, -1] for 1.2 s to deliver the coolant."][progress]
			_set_guidance(title, tr(advice))

	if _challenge_contract == "relay_gauntlet":
		_refresh_gauntlet_guidance()
	var view = preload("res://projection/challenge_state.gd")
	var preset: String = view.preset_id(run.contract)
	if _challenge_contract == "meridian_circuit" and preset.begins_with("west_approach") and saved.active is Dictionary:
		_set_guidance(title, tr("Read all three stations; return west [-31, 0]."))
	var goal: Variant = view.time_goal(run.contract)
	if goal != null:
		var elapsed: int = int(saved.get("elapsed_ms", 0)) if saved.get("elapsed_ms") != null else 0
		if _challenge_contract == "relay_gauntlet":
			var modifier: Dictionary = _authoritative_state.gauntlet_state.get("modifier", {}) if _authoritative_state.gauntlet_state.get("modifier") is Dictionary else {}
			elapsed = int(modifier.get("elapsed_ms", 0))
		if run.get("elapsed_ms") != null: elapsed = int(run.elapsed_ms)
		_objective_label.text += " • " + tr("TIME %.1f / %d s") % [elapsed / 1000.0, int(goal) / 1000]
		if saved.active == null and saved.last_result.outcome == "completed":
			_status_label.text = tr("TIME GOAL MET") if elapsed <= int(goal) else tr("COMPLETE • TIME GOAL MISSED")


func _refresh_campaign_guidance() -> void:
	if not _campaign_session:
		return
	_apply_training_layout()
	var title := tr({"return_signal": "RETURN SIGNAL", "meridian_readings": "MERIDIAN READINGS", "broken_supply_line": "BROKEN SUPPLY LINE", "counter_signal": "COUNTER-SIGNAL", "the_breach": "THE BREACH", "prism_core": "PRISM CORE"}.get(_campaign_chapter, "CHAPTER MENU"))
	if _authoritative_state.activity_complete:
		var practice := _campaign_practice
		_set_guidance(title, tr("Practice complete. No reward granted. Open Chapter menu to continue.") if practice else tr("Chapter complete. Progress saved. Open Chapter menu to continue."))
		_status_label.text = tr("CHAPTER COMPLETE")
		if _campaign_chapter == "prism_core":
			_set_guidance(title, tr("Practice complete. No reward granted. Read the campaign ending in the journal.") if practice else tr({"signal_silent": "The false signal is silent. You are back at the relay. Campaign complete.", "route_lit": "The service route stays lit. The relay carries one clear pulse. Campaign complete.", "open_passage": "The passage is open. A distant relay answers once. Campaign complete."}.get(_authoritative_state.campaign_state.get("story", {}).get("epilogue"), "The false signal is silent. You are back at the relay. Campaign complete.")))
			_status_label.text = tr("CAMPAIGN COMPLETE")
			_objective_label.text = tr("CAMPAIGN COMPLETE")
		return
	if _authoritative_state.actor_health.get(_authoritative_state.player_actor_id, 100) <= 0:
		_set_guidance(title, tr("Open Chapter menu to restart from your last checkpoint."))
		return
	var instructions := tr("Recover the return signal. Clear the relay drone, then reach the core and defeat the Warden.")
	if _campaign_chapter == "meridian_readings":
		instructions = tr("Trace the signal through Meridian. Reach the entrance, link the lens and gallery, then collect the log and return.")
	if _campaign_chapter == "broken_supply_line":
		var checkpoint := _campaign_checkpoint()
		instructions = tr([
			"The core has no power. Follow the west supply beacon to recover a cell for the north relay.",
			"The sentinel blocks the supply terminal. The barrier stops fire from both sides; flank its west end.",
			"The route is clear. Collect the power cell at the terminal north of the barrier.",
			"Carry the cell to the marked north relay intake. There is no time limit."
		][clampi(checkpoint, 0, 3)])
		if checkpoint == 0 and _authoritative_state.campaign_state.get("story", {}).get("supply") == "service":
			instructions = tr("Your recorded service route leads around the far west end of the barrier. Reach its marker to approach the sentinel from the terminal side.")
		_objective_label.text = tr("SUPPLY LINE • %d/4") % checkpoint
		var phase := "supply_north" if checkpoint >= 3 else "supply_west"
		if _camera_phase != phase:
			_camera_phase = phase
			if _camera_transition != null and _camera_transition.is_valid():
				_camera_transition.kill()
			var center := Vector3(-2, 0.5, -3) if checkpoint >= 3 else Vector3(-4, 0.5, 0)
			_camera.transform = Transform3D(Basis.IDENTITY, center + Vector3(6, 11, 10)).looking_at(center, Vector3.UP)
	if _campaign_chapter == "counter_signal":
		var checkpoint := _campaign_checkpoint()
		instructions = tr([
			"The powered relay repeats a false return signal. Reach the marked eastern repair link.",
			"Defeat the Mender and dodge the charge line. Chapter menu resumes this fight.",
			"The link is down. Reach its terminal to decode the repeated transmission.",
			"The signal is a trap from the core. Cut its source at the northern Bastion Link.",
			"Defeat the Mender, then flank the shield. Chapter menu resumes this fight.",
			"The defense is down. Reach the emitter to isolate the counter-signal and reveal the way into the core."
		][clampi(checkpoint, 0, 5)])
		_objective_label.text = tr("COUNTER-SIGNAL • %d/6") % checkpoint
		var phase := "counter_travel" if checkpoint in [0, 3] else "counter_north" if checkpoint >= 3 else "counter_east"
		if _camera_phase != phase:
			_camera_phase = phase
			if _camera_transition != null and _camera_transition.is_valid():
				_camera_transition.kill()
			var center := Vector3(7.5, 3.4, -7.5) if checkpoint >= 3 else Vector3(6, 3.4, 7.5)
			var location := Vector3(14, 16, 10) if checkpoint >= 3 else Vector3(14, 16, 25)
			_camera.transform = Transform3D(Basis.IDENTITY, location).looking_at(center, Vector3.UP)
	if _campaign_chapter == "the_breach":
		var checkpoint := _campaign_checkpoint()
		instructions = tr([
			"The false signal is isolated. Open the marked east door to reach the core relay.",
			"The shield absorbs shots; the Warden still retaliates. Reach the marked stabilizer to drain it.",
			"Relay drained. The Warden is vulnerable. Defeat it to unlock the core threshold.",
			"The core is open. Cross the marked threshold to reach the Prism chamber."
		][clampi(checkpoint, 0, 3)])
		if _authoritative_state.campaign_state.get("story", {}).get("core") == "direct":
			if checkpoint == 1: instructions = tr("Direct breach: the shield feed is bypassed. Defeat the Warden before draining the relay.")
			if checkpoint == 2: instructions = tr("The Warden is down. Drain the marked stabilizer to open the core threshold.")
		_objective_label.text = tr("THE BREACH • %d/4") % checkpoint
		if _camera_phase != "campaign_breach":
			_camera_phase = "campaign_breach"
			if _camera_transition != null and _camera_transition.is_valid():
				_camera_transition.kill()
			var center := Vector3(6, 3.4, 0)
			_camera.transform = Transform3D(Basis.IDENTITY, Vector3(14, 16, 17)).looking_at(center, Vector3.UP)
	if _campaign_chapter == "prism_core":
		var checkpoint := _campaign_checkpoint()
		_environment.call("set_prism_encounter", true)
		instructions = tr([
			"Enter the marked Prism chamber. Stop its guardian, then shut down the false signal.",
			"Follow the floor warnings. Chapter menu restarts this fight from the chamber entrance.",
			"The guardian is down. Reach the marked source to shut down the false signal.",
			"The source is silent. Return to the marked relay hub to finish the campaign."
		][clampi(checkpoint, 0, 3)])
		if checkpoint == 1 and _prism_fighting():
			var cue := _prism_guidance()
			title = tr(cue[0])
			instructions = tr(cue[1])
			_objective_label.text = tr("PRISM WARDEN • PHASE %d/2") % (2 if _authoritative_state.prism_state.get("phase") == "Pulses" else 1)
		else:
			_objective_label.text = tr("PRISM CORE • %d/4") % checkpoint
			_camera_phase = "campaign_prism"
		_status_label.text = title if checkpoint < 2 else tr("PRISM WARDEN • CLEARED") if checkpoint == 2 else tr("SOURCE SILENT • RETURN TO THE RELAY")
	_set_guidance(title, instructions)


func _update_campaign_travel_camera() -> void:
	var travel := (_campaign_chapter == "counter_signal" and _campaign_checkpoint() in [0, 3]) or (_campaign_chapter == "prism_core" and _campaign_checkpoint() != 1)
	if not _campaign_session or not travel or not _hud_canvas.visible or _authoritative_state.activity_complete:
		return
	if _campaign_chapter == "prism_core":
		_camera.transform = Transform3D(Basis.IDENTITY, Vector3(12, 16, 17)).looking_at(Vector3(4, 3.4, 0), Vector3.UP)
		return
	var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
	if player == null:
		return
	var center := player.position + Vector3(0, 7, 0)
	_camera.transform = Transform3D(Basis.IDENTITY, center + Vector3(8, 12.6, 17.5)).looking_at(center, Vector3.UP)


func _campaign_checkpoint() -> int:
	var active: Variant = _authoritative_state.campaign_state.get("active")
	return int(active.get("checkpoint", 0)) if active is Dictionary else 0


func _build_settings() -> void:
	_settings_store = SETTINGS_STORE.new()
	_settings = _settings_store.call("load_settings")
	_apply_settings(_settings, false)
	var canvas := CanvasLayer.new()
	canvas.name = "Settings"
	canvas.layer = 20
	add_child(canvas)
	_settings_panel = SETTINGS_PANEL_SCENE.instantiate()
	canvas.add_child(_settings_panel)
	_settings_panel.connect("settings_applied", _on_settings_applied)
	_settings_panel.connect("closed", _on_settings_closed)
	_apply_interface_layout()


func _open_settings(focus_source: Control) -> void:
	_player_intents.call("clear_pending_input")
	_observe_first("settings_opened")
	_settings_panel.call("open", _settings, focus_source)


func _on_settings_closed() -> void:
	_player_intents.call("clear_pending_input")
	if not _action_bar_focused and not _entry_shell.visible:
		get_viewport().gui_release_focus()


func _on_settings_applied(settings: Dictionary) -> void:
	_apply_settings(settings, true)


func _apply_settings(settings: Dictionary, persist: bool) -> void:
	_settings = _settings_store.call("apply", settings)
	_apply_interface_layout()
	if _playtest_observation != null:
		_playtest_observation.call("update_preferences", _settings)
		_playtest_observation.call("update_environment", _observation_viewport_size())
	_guidance_mode = _settings.get("guidance_mode", "Full")
	if _onboarding != null:
		_onboarding.call("set_mode", _guidance_mode)
		_refresh_onboarding()
	if _presentation_polish != null:
		_presentation_polish.call("set_reduced_flash", _settings.get("reduced_flash", false))
	if _sector_transition != null:
		_sector_transition.call("set_reduced_motion", _settings.get("reduced_motion", false) or _settings.get("reduced_flash", false))
	if _combat_vfx != null:
		_combat_vfx.call("set_reduced_flash", _settings.get("reduced_flash", false))
		_combat_vfx.call("set_reduced_motion", _settings.get("reduced_motion", false))
	for actor in _actors.values():
		actor.call("set_accessibility", _settings.get("reduced_motion", false), _settings.get("reduced_flash", false))
	for presentation in [_environment, _loot_pickup]:
		if presentation != null:
			presentation.call("set_reduced_motion", _settings.get("reduced_motion", false))
	if _settings.get("reduced_motion", false) and _camera_transition != null and _camera_transition.is_valid():
		_camera_transition.custom_step(1000.0)
	if _module_workshop != null:
		_module_workshop.call("set_reduced_flash", _settings.get("reduced_flash", false))
	if _route_console != null:
		_route_console.call("set_reduced_flash", _settings.get("reduced_flash", false))
	if _cooperation_console != null:
		_cooperation_console.call("set_reduced_flash", _settings.get("reduced_flash", false))
	if _audio_director != null:
		_sound_captions.call("configure", _settings.get("captions", true), float(_settings.get("ui_scale", 1.0)))
		_audio_director.call("configure_routes")
		_audio_director.call("set_silent", _settings.get("muted", false))
	if _entry_shell != null: _entry_shell.call("_refresh_campaign_menu")
	if _story_panel != null: _story_panel.call("_refresh")
	if _authoritative_state.actors.has(_current_enemy_id): _refresh_selected_enemy()
	if not _challenge_contract.is_empty(): _refresh_challenge_guidance()
	if persist:
		var save_error: Error = _settings_store.call("save_settings", _settings)
		if save_error != OK:
			push_warning("Revenant settings could not be saved: %s" % error_string(save_error))


func _apply_interface_layout() -> void:
	if _showcase_mode or _action_grid == null:
		return
	var factor: float = _settings.get("ui_scale", 1.0)
	var contrast: bool = _settings.get("high_contrast", false)
	var left_width := 620.0 + (factor - 1.0) * 280.0
	var right_width := 362.0 + (factor - 1.0) * 176.0
	var right_x := 1256.0 - right_width
	_telemetry_panel.position = Vector2(24, 24)
	_telemetry_panel.size = Vector2(left_width, 196 * factor + 16)
	_layout_text(_status_label, Rect2(40, 34, left_width - 32, 42 * factor), 18 * factor)
	_layout_text(_health_label, Rect2(40, 38 + 42 * factor, left_width - 32, 32 * factor), 24 * factor)
	_player_health_bar.position = Vector2(40, 38 + 82 * factor)
	_player_health_bar.size = Vector2(left_width - 32, 8)
	_layout_text(_objective_label, Rect2(40, 44 + 86 * factor, left_width - 32, 42 * factor), 17 * factor)
	_layout_text(_enemy_health_label, Rect2(40, 44 + 131 * factor, left_width - 32, 28 * factor), 17 * factor)
	_enemy_health_bar.position = Vector2(40, 46 + 162 * factor)
	_enemy_health_bar.size = Vector2(left_width - 32, 6)
	_layout_text(_position_label, Rect2(40, 54 + 174 * factor, left_width - 32, 25 * factor), 14 * factor)
	_position_label.visible = false
	_guide_panel.position = Vector2(right_x, 24)
	_guide_panel.size = Vector2(right_width, 176 + (factor - 1) * 80)
	_layout_text(_guide_title, Rect2(right_x + 16, 36, right_width - 32, 24 * factor), 15 * factor)
	_layout_text(_guidance_label, Rect2(right_x + 16, 44 + 24 * factor, right_width - 32, _guide_panel.size.y - 24 * factor - 32), 16 * factor)
	_inventory_panel.position = Vector2(right_x, _guide_panel.position.y + _guide_panel.size.y + 12)
	_inventory_panel.size = Vector2(right_width, 174 * factor + 24)
	_layout_text(_inventory_label, Rect2(_inventory_panel.position + Vector2(16, 12), Vector2(right_width - 32, 132 * factor)), 15 * factor)
	_layout_text(_progression_label, Rect2(_inventory_panel.position + Vector2(16, 148 * factor), Vector2(right_width - 32, 42 * factor)), 14 * factor)
	if _authoritative_state.inventory.has("scatter_caster"):
		_inventory_panel.size.y = 132 * factor + 24
		_layout_text(_inventory_label, Rect2(_inventory_panel.position + Vector2(16, 12), Vector2(right_width - 32, 68 * factor)), 15 * factor)
		_layout_text(_progression_label, Rect2(_inventory_panel.position + Vector2(16, 84 * factor), Vector2(right_width - 32, 42 * factor)), 14 * factor)
	_action_grid.columns = 6
	_action_grid.position = Vector2(244, 584 if factor == 1.0 else 548)
	_action_grid.size = Vector2(1012, 108 if factor == 1.0 else 144)
	_sector_transition.call("set_bottom", _action_grid.position.y)
	for button in _weapon_buttons + [_module_button, _route_button, _cooperation_button, _archive_button, _target_button, _attack_button, _campaign_button]:
		button.custom_minimum_size = Vector2(0, 48 if factor == 1.0 else 68)
		button.add_theme_font_size_override("font_size", roundi(14 * factor))
	_layout_text(_equipment_label, Rect2(244, _action_grid.position.y - 68, 1012, 62), 16 * factor)
	_controls_label.visible = false
	_archive_hint.visible = false
	_sound_captions.position = Vector2(24, _telemetry_panel.position.y + _telemetry_panel.size.y + 12)
	_sound_captions.size.x = left_width
	_sound_captions.custom_minimum_size.x = left_width
	_sound_captions.call_deferred("reset_size")
	for index in _movement_buttons.size():
		var action: String = ["move_forward", "move_left", "move_back", "move_right"][index]
		_movement_buttons[index].text = preload("res://input/input_bindings.gd").key_label(_settings.bindings, action)
	_attack_button.text = tr("ATTACK [%s]") % preload("res://input/input_bindings.gd").key_label(_settings.bindings, "attack")
	_target_button.text = tr("NEXT ENEMY [%s]") % preload("res://input/input_bindings.gd").key_label(_settings.bindings, "target_next")
	_module_button.text = "MODULES [%s]" % preload("res://input/input_bindings.gd").key_label(_settings.bindings, "modules")
	_route_button.text = "ROUTES [%s]" % preload("res://input/input_bindings.gd").key_label(_settings.bindings, "routes")
	_cooperation_button.text = "CO-OP [%s]" % preload("res://input/input_bindings.gd").key_label(_settings.bindings, "cooperation")
	var palette := {"font_color": Color.WHITE if contrast else Color("dce7f2"), "font_hover_color": Color.WHITE, "font_pressed_color": Color("071016")}
	_style_accessible_controls(_hud_canvas, palette, contrast)
	for surface in [_entry_shell, _settings_panel, _module_workshop, _route_console, _cooperation_console, _relay_archive, _story_panel]:
		if surface != null:
			_style_accessible_controls(surface, palette, contrast)
			if surface != _settings_panel:
				_surface_scaler.apply(surface, factor)
	if _settings_panel != null:
		var settings_theme := Theme.new()
		settings_theme.default_font_size = roundi(18 * factor)
		_settings_panel.theme = settings_theme
	if _sector_transition != null:
		_sector_transition.visible = factor == 1.0
	_apply_training_layout()
	_health_label.visible = _challenge_contract != "coolant_recovery"
	_player_health_bar.visible = _challenge_contract != "coolant_recovery"
	_crosshair.visible = _challenge_contract != "coolant_recovery"
	if _challenge_contract == "coolant_recovery":
		_telemetry_panel.size.y = 40 + 76 * factor
		_layout_text(_objective_label, Rect2(40, 40 + 34 * factor, left_width - 32, 30 * factor), 17 * factor)
		_layout_text(_position_label, Rect2(40, 44 + 62 * factor, left_width - 32, 26 * factor), 14 * factor)
		_position_label.visible = true
		_sound_captions.position = Vector2(right_x, _guide_panel.position.y + _guide_panel.size.y + 12)
		_sound_captions.custom_minimum_size.x = right_width
		_sound_captions.size = Vector2(right_width, 1)
		_sound_captions.call_deferred("reset_size")
	_telemetry_panel.get_node("SemanticRail").size.y = _telemetry_panel.size.y


func _layout_text(label: Label, rect: Rect2, font_size: float) -> void:
	label.position = rect.position
	label.size = rect.size
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.clip_text = label == _status_label
	label.add_theme_font_size_override("font_size", roundi(font_size))


func _style_accessible_controls(node: Node, palette: Dictionary, contrast: bool) -> void:
	if node is BaseButton:
		for color_name in palette:
			node.add_theme_color_override(color_name, palette[color_name])
		node.add_theme_color_override("font_disabled_color", Color("a9b8cc"))
		if node is CheckButton:
			node.add_theme_color_override("font_pressed_color", Color.WHITE)
			node.add_theme_color_override("font_hover_pressed_color", Color.WHITE)
		else:
			var pressed := StyleBoxFlat.new()
			pressed.bg_color = Color("35d0d0")
			pressed.border_color = Color.WHITE
			pressed.set_border_width_all(2)
			node.add_theme_stylebox_override("pressed", pressed)
		var focus := StyleBoxFlat.new()
		focus.bg_color = Color.TRANSPARENT
		focus.border_color = Color.WHITE
		focus.set_border_width_all(3)
		node.add_theme_stylebox_override("focus", focus)
	if node is Label:
		if not node.has_meta("normal_font_color"):
			node.set_meta("normal_font_color", node.get_theme_color("font_color"))
			_contrast_labels.append(node)
		node.add_theme_color_override("font_color", Color.WHITE if contrast else node.get_meta("normal_font_color"))
	if node is Panel and node.has_theme_stylebox_override("panel"):
		var style := node.get_theme_stylebox("panel") as StyleBoxFlat
		if not node.has_meta("normal_background"):
			node.set_meta("normal_background", style.bg_color)
		style.bg_color = Color("071016") if contrast else node.get_meta("normal_background")
	for child in node.get_children():
		_style_accessible_controls(child, palette, contrast)


func _hud_label(parent: Node, position: Vector2, size: int, color: Color, text: String) -> Label:
	var label := Label.new()
	label.position = position
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", color)
	label.text = text
	parent.add_child(label)
	return label


func _create_control_button(parent: Node, text: String, position: Vector2, size: Vector2, direction: Vector2) -> void:
	var button := Button.new()
	button.text = text
	button.position = position
	button.size = size
	button.add_theme_font_size_override("font_size", 20)
	_hud_frame.call("style_button", button, Color("35d0d0"), true)
	button.button_down.connect(_set_ui_movement.bind(direction))
	button.button_up.connect(_clear_ui_movement)
	parent.add_child(button)
	_movement_buttons.append(button)


func _set_ui_movement(direction: Vector2) -> void:
	_observe_first("first_movement_attempt")
	_player_intents.call("set_ui_movement", direction)
	_append_input_log("ON-SCREEN MOVE %s" % direction)


func _clear_ui_movement() -> void:
	_player_intents.call("clear_ui_movement")


func _request_ui_attack() -> void:
	if _challenge_contract == "last_reserve": return
	_action_bar_focused = false
	get_viewport().gui_release_focus()
	_observe_first("first_attack_attempt")
	_player_intents.call("request_attack", true)
	_append_input_log("ON-SCREEN ATTACK detected")


func _create_weapon_button(parent: Node, text: String, item_id: String, position: Vector2) -> void:
	var button := Button.new()
	button.text = text
	button.position = position
	button.size = Vector2(92, 40)
	button.set_meta("weapon_item_id", item_id)
	button.visible = item_id in ["pulse_rifle", "arc_sidearm"]
	if item_id == "scatter_caster":
		button.tooltip_text = "Full damage within 3 units; half damage beyond. Use weapon cycling or this button."
	elif item_id == "rail_driver":
		button.tooltip_text = "Cannot fire within 4 units. Retreat or switch weapons when rushed."
	_hud_frame.call("style_button", button, Color("f5a524"), true)
	button.pressed.connect(_request_weapon.bind(item_id))
	parent.add_child(button)
	_weapon_buttons.append(button)


func _request_weapon(item_id: String) -> void:
	if not _challenge_contract.is_empty(): return
	var sent := _send_message({"type": "EquipIntent", "item_id": item_id})
	_append_input_log("EquipIntent %s %s" % [item_id, "sent" if sent else "FAILED"])


func _drive_manual_activity() -> void:
	await get_tree().create_timer(0.2).timeout
	await _showcase_pause(1.30)
	_request_weapon("arc_sidearm")
	var equip_deadline := Time.get_ticks_msec() + 2000
	while _authoritative_state.equipped_weapon_item_id != "arc_sidearm" and Time.get_ticks_msec() < equip_deadline:
		await get_tree().process_frame
	if _authoritative_state.equipped_weapon_item_id != "arc_sidearm":
		_driver_fail("sidearm did not equip through the on-screen loadout control")
		return
	await _showcase_pause(0.80)
	var drone_timeout_ms := 15000 if _showcase_mode else 5000
	if not await _drive_validation_attacks(false, drone_timeout_ms):
		_driver_fail("drone did not die through the on-screen Attack control")
		return

	await get_tree().create_timer(0.2).timeout
	await _showcase_pause(1.25)
	_set_ui_movement(Vector2(1, 0))
	var deadline := Time.get_ticks_msec() + 4000
	while Time.get_ticks_msec() < deadline:
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if player != null and player.position.x >= 6:
			break
		await get_tree().process_frame
	_clear_ui_movement()
	var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
	if player == null or player.position.x < 6:
		_driver_fail("player did not reach the relay door through the on-screen direction control")
		return
	await _showcase_pause(1.00)

	deadline = Time.get_ticks_msec() + 3000
	while _current_enemy_id == 0 and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if _current_enemy_id == 0:
		_driver_fail("Warden did not spawn after manual movement")
		return
	await _showcase_pause(1.20)
	var warden_timeout_ms := 18000 if _showcase_mode else 6000
	if not await _drive_validation_attacks(false, warden_timeout_ms):
		_driver_fail("Warden did not die through the on-screen Attack control")
		return
	deadline = Time.get_ticks_msec() + 3000
	while not _authoritative_state.activity_complete and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if not _authoritative_state.activity_complete:
		_driver_fail("relay_awakening did not complete through manual controls")
		return
	await _showcase_pause(3.10)
	print("M17 manual controls completed relay_awakening without user input")
	_quit_client(0)


func _showcase_pause(seconds: float) -> void:
	if _showcase_mode:
		await get_tree().create_timer(seconds).timeout


func _drive_module_activity() -> void:
	await get_tree().create_timer(0.2).timeout
	var admitted_max_health: int = _authoritative_state.actor_max_health.get(_authoritative_state.player_actor_id, 0)
	var admitted_rifle: Dictionary = _authoritative_state.weapon_profiles.get("pulse_rifle", {}).duplicate(true)
	_request_weapon("pulse_rifle")
	var equip_deadline := Time.get_ticks_msec() + 2000
	while _authoritative_state.equipped_weapon_item_id != "pulse_rifle" and Time.get_ticks_msec() < equip_deadline:
		await get_tree().process_frame
	if _authoritative_state.equipped_weapon_item_id != "pulse_rifle":
		_driver_fail("M26 module flow could not equip the admitted rifle")
		return
	if not await _drive_validation_attacks(false, 6000):
		_driver_fail("M26 module flow did not defeat the drone")
		return
	await get_tree().create_timer(0.2).timeout
	_set_ui_movement(Vector2(1, 0))
	var deadline := Time.get_ticks_msec() + 4000
	while Time.get_ticks_msec() < deadline:
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if player != null and player.position.x >= 6:
			break
		await get_tree().process_frame
	_clear_ui_movement()
	deadline = Time.get_ticks_msec() + 3000
	while _current_enemy_id == 0 and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if _current_enemy_id == 0:
		_driver_fail("M26 module flow did not spawn the Warden")
		return
	var warden_defeated: bool = await _drive_validation_attacks(false, 7000)
	if not warden_defeated:
		_driver_fail("M26 module flow did not defeat the Warden")
		return
	deadline = Time.get_ticks_msec() + 3000
	while not _authoritative_state.activity_complete and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if not _authoritative_state.activity_complete:
		_driver_fail("M26 module flow did not reach authoritative completion")
		return

	_open_module_workshop()
	if not await _wait_for_module_result("state", 5000) or _authoritative_state.module_state.is_empty():
		_driver_fail("M26 module flow expected ModuleSnapshot")
		return
	var current_revision := int(_authoritative_state.module_state.get("loadout_revision", -1))
	var current_modules: Array = _authoritative_state.module_state.get("equipped_modules", [])
	var candidate_modules: Array[String] = []
	var expected_max_health := 0
	var expected_rifle_cooldown := 0
	if current_modules == ["module_force_matrix", "module_ward_capacitor"]:
		candidate_modules = ["module_force_matrix"]
		expected_max_health = 100
		expected_rifle_cooldown = 300
	elif current_modules == ["module_force_matrix"]:
		candidate_modules = ["module_force_matrix", "module_ward_capacitor"]
		expected_max_health = 120
		expected_rifle_cooldown = 325
	else:
		_driver_fail("M26 module fixture must begin with Force or Force+Ward")
		return
	_module_workshop.call("select_modules_for_validation", candidate_modules)
	_request_module_preview(candidate_modules)
	if not await _wait_for_module_result("preview", 5000) or not _authoritative_state.module_result.get("accepted", false):
		_driver_fail("M26 module flow expected accepted ModulePreview")
		return
	var preview_rifle: Dictionary = _authoritative_state.module_preview.get("weapons", [])[0]
	if (
		_authoritative_state.module_preview.get("max_health") != expected_max_health
		or preview_rifle.get("effective_damage") != 48
		or preview_rifle.get("effective_cooldown_ms") != expected_rifle_cooldown
	):
		_driver_fail("M26 Force preview arithmetic diverged from server evidence")
		return
	var preview_capture := OS.get_environment("REVENANT_CAPTURE_M26_PREVIEW")
	if not preview_capture.is_empty() and await _save_review_capture(preview_capture) != OK:
		_driver_fail("M26 preview capture could not be saved")
		return

	_request_module_loadout(current_revision, candidate_modules)
	if not await _wait_for_module_result("loadout", 5000) or not _authoritative_state.module_result.get("accepted", false):
		_driver_fail("M26 module flow expected accepted ModuleLoadoutChanged")
		return
	if _authoritative_state.module_state.get("equipped_modules", []) != candidate_modules or int(_authoritative_state.module_state.get("loadout_revision", -1)) != current_revision + 1:
		_driver_fail("M26 accepted loadout projection diverged")
		return
	if _authoritative_state.actor_max_health.get(_authoritative_state.player_actor_id) != admitted_max_health or _authoritative_state.weapon_profiles.get("pulse_rifle", {}) != admitted_rifle:
		_driver_fail("M26 accepted loadout rewrote the completed admitted actor")
		return
	var accepted_capture := OS.get_environment("REVENANT_CAPTURE_M26_ACCEPTED")
	if not accepted_capture.is_empty() and await _save_review_capture(accepted_capture) != OK:
		_driver_fail("M26 accepted-state capture could not be saved")
		return
	print("M26 Godot workshop flow validated: authoritative preview and accepted next-session loadout are honest")
	_quit_client(0)


func _drive_route_activity(route_id: String) -> void:
	if route_id not in ["breach", "stabilize"]:
		_driver_fail("M27 route validation requires breach or stabilize")
		return
	await get_tree().create_timer(0.2).timeout
	var initial_fragments := int(_authoritative_state.inventory.get("relay_core_fragment", 0))
	var initial_experience := int(_authoritative_state.progression.get("experience", 0))
	_open_route_console()
	if not await _wait_for_route_state("drone", 3000):
		_driver_fail("M27 Godot route flow did not receive authoritative drone-phase capability")
		return
	_close_route_console()

	if not await _drive_validation_attacks(false, 6000):
		_driver_fail("M27 Godot route flow did not defeat the relay drone")
		return
	if not await _wait_for_route_state("choice_open", 3000):
		_driver_fail("M27 Godot route flow did not receive the authoritative open choice")
		return
	_open_route_console()
	if not await _wait_for_route_state("choice_open", 3000):
		_driver_fail("M27 Godot route choice refresh did not settle")
		return
	var choice_capture := OS.get_environment("REVENANT_CAPTURE_M27_DIR")
	if not choice_capture.is_empty() and await _save_review_capture(choice_capture.path_join("01-route-choice.png")) != OK:
		_driver_fail("M27 route choice capture could not be saved")
		return
	if not _route_console.call("select_for_validation", route_id):
		_driver_fail("M27 server-declared leader could not activate the requested route button")
		return
	if not await _wait_for_route_choice(3000):
		_driver_fail("M27 Godot route flow did not receive accepted server selection")
		return
	var selection: Dictionary = _authoritative_state.route_result.get("selection", {})
	if selection.get("route_id") != route_id or int(selection.get("seed", -1)) < 0:
		_driver_fail("M27 Godot route selection diverged from server truth")
		return
	var accepted_capture := OS.get_environment("REVENANT_CAPTURE_M27_DIR")
	if not accepted_capture.is_empty() and await _save_review_capture(accepted_capture.path_join("02-route-accepted.png")) != OK:
		_driver_fail("M27 accepted route capture could not be saved")
		return
	_close_route_console()

	if route_id == "stabilize" and not await _drive_to_route_position(Vector3i(3, 0, 3), 5000):
		_driver_fail("M27 Stabilize flow did not reach authoritative coordinate [3, 0, 3]")
		return
	if route_id == "stabilize" and not await _drive_to_route_position(Vector3i(3, 0, 0), 5000):
		_driver_fail("M27 Stabilize flow did not return to the relay-door axis")
		return
	if not await _drive_to_route_position(Vector3i(6, 0, 0), 5000):
		_driver_fail("M27 route flow did not reach authoritative relay door [6, 0, 0]")
		return
	var spawn_deadline := Time.get_ticks_msec() + 3000
	while _current_enemy_id == 0 and Time.get_ticks_msec() < spawn_deadline:
		await get_tree().process_frame
	if _current_enemy_id == 0 or _actor_family(_actors.get(_current_enemy_id)) != "warden":
		_driver_fail("M27 route flow did not render the authoritative Warden")
		return
	if not await _drive_validation_attacks(false, 8000):
		_driver_fail("M27 route flow did not defeat the authoritative Warden")
		return
	var terminal_deadline := Time.get_ticks_msec() + 4000
	while (_authoritative_state.route_summary.is_empty() or not _authoritative_state.activity_complete) and Time.get_ticks_msec() < terminal_deadline:
		await get_tree().process_frame
	if _authoritative_state.route_summary.get("outcome") != "succeeded" or not _authoritative_state.activity_complete:
		_driver_fail("M27 Godot route flow did not receive matching completion and route success")
		return
	var reward: Dictionary = selection.get("reward", {})
	if (
		int(_authoritative_state.inventory.get("relay_core_fragment", 0)) != initial_fragments + int(reward.get("item_quantity", 0))
		or int(_authoritative_state.progression.get("experience", 0)) != initial_experience + int(reward.get("experience", 0))
	):
		_driver_fail("M27 generic reward projection disagrees with server route summary")
		return
	_route_console.call("open_console")
	_refresh_route_console()
	await get_tree().process_frame
	var summary_capture := OS.get_environment("REVENANT_CAPTURE_M27_DIR")
	if not summary_capture.is_empty() and await _save_review_capture(summary_capture.path_join("03-route-summary.png")) != OK:
		_driver_fail("M27 terminal route capture could not be saved")
		return
	print("M27 Godot route flow validated: %s / %s / %d ms / server-owned reward" % [
		str(selection.get("route_id", "")),
		str(selection.get("event_id", "")),
		int(_authoritative_state.route_summary.get("elapsed_ms", 0)),
	])
	_quit_client(0)


func _drive_cooperation_activity() -> void:
	await get_tree().create_timer(0.2).timeout
	var initial_fragments := int(_authoritative_state.inventory.get("relay_core_fragment", 0))
	var initial_experience := int(_authoritative_state.progression.get("experience", 0))
	_open_cooperation_console()
	if not await _wait_for_cooperation_phase("drone", 5000, true):
		_driver_fail("M28 Godot cooperation flow did not receive both explicit capabilities")
		return
	_close_cooperation_console()

	if not await _drive_validation_attacks(false, 7000):
		_driver_fail("M28 Godot cooperation flow did not defeat the relay drone")
		return
	if not await _wait_for_cooperation_phase("eligible", 4000, true):
		_driver_fail("M28 Godot cooperation flow did not reach authoritative eligibility")
		return
	_open_cooperation_console()
	if not await _wait_for_cooperation_phase("eligible", 3000, true):
		_driver_fail("M28 cooperation eligibility refresh did not settle")
		return
	if not await _capture_cooperation_state("01-eligible-roles.png"):
		return
	if not _cooperation_console.call("activate_for_validation", "start"):
		_driver_fail("M28 server-declared anchor could not activate Start")
		return
	if not await _wait_for_cooperation_phase("awaiting_anchor", 4000):
		_driver_fail("M28 Godot cooperation start was not authoritatively accepted")
		return
	_close_cooperation_console()

	if not await _drive_to_route_position(Vector3i(3, 0, 3), 6000):
		_driver_fail("M28 anchor did not reach authoritative target [3,0,3]")
		return
	if not await _wait_for_cooperation_phase("awaiting_ping", 4000):
		_driver_fail("M28 anchor arrival did not unlock the authoritative ping phase")
		return
	_open_cooperation_console()
	if not await _wait_for_cooperation_phase("awaiting_ping", 3000):
		_driver_fail("M28 ping-phase refresh did not settle")
		return
	if not _cooperation_console.call("activate_for_validation", "ping"):
		_driver_fail("M28 anchor could not activate the server-authorized ping")
		return
	if not await _wait_for_cooperation_result("ping", "", 4000):
		_driver_fail("M28 Godot cooperation ping was not authoritatively accepted")
		return
	if not await _capture_cooperation_state("02-live-ping.png"):
		return
	_close_cooperation_console()

	if not await _wait_for_cooperation_phase("runner_downed", 5000):
		_driver_fail("M28 standalone runner did not reach the console and enter server-owned downed state")
		return
	if not _authoritative_state.cooperation_life_states.has(_cooperation_runner_actor_id()):
		_driver_fail("M28 Godot did not observe the runner life transition")
		return
	_open_cooperation_console()
	if not await _wait_for_cooperation_phase("runner_downed", 3000):
		_driver_fail("M28 runner-downed refresh did not settle")
		return
	if not await _capture_cooperation_state("03-runner-downed.png"):
		return
	if not _cooperation_console.call("activate_for_validation", "revive"):
		_driver_fail("M28 anchor could not activate the server-authorized revive")
		return
	if not await _wait_for_cooperation_result("revive", "started", 4000):
		_driver_fail("M28 revive channel did not receive its authoritative start")
		return
	if not await _capture_cooperation_state("04-revive-channel.png"):
		return

	await get_tree().create_timer(2.15).timeout
	_request_cooperation_state()
	if not await _wait_for_cooperation_phase("encounter_active", 5000):
		_driver_fail("M28 revive did not complete into the authoritative encounter phase")
		return
	if not await _capture_cooperation_state("05-revive-complete.png"):
		return
	_close_cooperation_console()
	if not await _drive_to_route_position(Vector3i(6, 0, 0), 6000):
		_driver_fail("M28 revived anchor did not reach the authoritative relay door [6,0,0]")
		return

	var spawn_deadline := Time.get_ticks_msec() + 4000
	while _current_enemy_id == 0 and Time.get_ticks_msec() < spawn_deadline:
		await get_tree().process_frame
	if _current_enemy_id == 0 or _actor_family(_actors.get(_current_enemy_id)) != "warden":
		_driver_fail("M28 cooperation flow did not render the authoritative Warden")
		return
	if not await _drive_validation_attacks(false, 9000):
		_driver_fail("M28 cooperation flow did not defeat the authoritative Warden")
		return
	var terminal_deadline := Time.get_ticks_msec() + 5000
	while (_authoritative_state.cooperation_summary.is_empty() or not _authoritative_state.activity_complete) and Time.get_ticks_msec() < terminal_deadline:
		await get_tree().process_frame
	if _authoritative_state.cooperation_summary.get("outcome") != "succeeded" or not _authoritative_state.activity_complete:
		_driver_fail("M28 Godot cooperation flow did not receive matching generic and typed success")
		return
	if (
		int(_authoritative_state.inventory.get("relay_core_fragment", 0)) != initial_fragments + 2
		or int(_authoritative_state.progression.get("experience", 0)) != initial_experience + 125
		or _authoritative_state.cooperation_summary.get("grants", []).size() != 2
	):
		_driver_fail("M28 generic local reward projection disagrees with the typed equal-grant summary")
		return
	_cooperation_console.call("open_console")
	_refresh_cooperation_console()
	if not await _capture_cooperation_state("06-terminal-success.png"):
		return
	print("M28 Godot cooperation flow validated: anchor / runner / ping / downed / revive / Warden / %d ms / equal server rewards" % int(_authoritative_state.cooperation_summary.get("terminal_elapsed_ms", 0)))
	_quit_client(0)


func _wait_for_cooperation_phase(phase: String, timeout_ms: int, require_all_capable := false) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if (
			_authoritative_state.cooperation_pending.is_empty()
			and _authoritative_state.cooperation_state.get("phase") == phase
			and (not require_all_capable or _authoritative_state.cooperation_state.get("all_capable", false))
		):
			return true
		await get_tree().process_frame
	return false


func _wait_for_cooperation_result(kind: String, status: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		var result: Dictionary = _authoritative_state.cooperation_result
		if (
			_authoritative_state.cooperation_pending.is_empty()
			and result.get("kind") == kind
			and result.get("accepted", false)
			and (status.is_empty() or result.get("status") == status)
		):
			return true
		await get_tree().process_frame
	return false


func _cooperation_runner_actor_id() -> int:
	for participant in _authoritative_state.cooperation_state.get("operation", {}).get("participants", []):
		if participant.get("role") == "runner":
			return int(participant.get("actor_id", 0))
	return 0


func _capture_cooperation_state(filename: String) -> bool:
	var capture_dir := OS.get_environment("REVENANT_CAPTURE_M28_DIR")
	if capture_dir.is_empty():
		return true
	if await _save_review_capture(capture_dir.path_join(filename)) != OK:
		_driver_fail("M28 cooperation capture could not be saved: %s" % filename)
		return false
	return true


func _wait_for_route_state(phase: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if (
			_authoritative_state.route_pending.is_empty()
			and _authoritative_state.route_state.get("phase") == phase
		):
			return true
		await get_tree().process_frame
	return false


func _wait_for_route_choice(timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if (
			_authoritative_state.route_pending.is_empty()
			and _authoritative_state.route_result.get("kind") == "choice"
			and _authoritative_state.route_result.get("accepted", false)
		):
			return true
		await get_tree().process_frame
	return false


func _drive_to_route_position(target: Vector3i, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if player == null:
			return false
		var current := Vector3i(roundi(player.position.x), 0, roundi(player.position.z))
		if current == target:
			_clear_ui_movement()
			return true
		var direction := Vector2.ZERO
		if current.x != target.x:
			direction.x = signi(target.x - current.x)
		else:
			direction.y = signi(target.z - current.z)
		_set_ui_movement(direction)
		await get_tree().process_frame
	_clear_ui_movement()
	return false


func _wait_for_module_result(kind: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if (
			_authoritative_state.module_pending.is_empty()
			and _authoritative_state.module_result.get("kind") == kind
		):
			return true
		await get_tree().process_frame
	return false


func _drive_keyboard_activity() -> void:
	await get_tree().create_timer(0.2).timeout
	if "ACTIVE" not in _objective_label.text.to_upper():
		_driver_fail("initial authoritative objective is not visible before keyboard input")
		return
	var initial_fragment_count := int(_authoritative_state.inventory.get("relay_core_fragment", 0))
	var initial_experience := int(_authoritative_state.progression.get("experience", 0))

	await _tap_validation_key(KEY_2)
	var equip_deadline := Time.get_ticks_msec() + 2000
	while _authoritative_state.equipped_weapon_item_id != "arc_sidearm" and Time.get_ticks_msec() < equip_deadline:
		await get_tree().process_frame
	if _authoritative_state.equipped_weapon_item_id != "arc_sidearm":
		_driver_fail("sidearm did not equip through keyboard shortcut 2")
		return

	if not await _drive_validation_attacks(true, 5000):
		_driver_fail("drone did not die through keyboard Space attacks")
		return
	var deadline := Time.get_ticks_msec() + 3000
	while "REACH" not in _objective_label.text and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if "REACH" not in _objective_label.text:
		_driver_fail("relay-door objective is not visible after keyboard combat")
		return

	await _set_validation_key(KEY_D, true)
	deadline = Time.get_ticks_msec() + 4000
	while Time.get_ticks_msec() < deadline:
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if player != null and player.position.x >= 6:
			break
		await get_tree().process_frame
	await _set_validation_key(KEY_D, false)
	var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
	if player == null or player.position.x < 6:
		_driver_fail("player did not reach the relay door through keyboard D")
		return

	deadline = Time.get_ticks_msec() + 3000
	while _current_enemy_id == 0 and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if _current_enemy_id == 0:
		_driver_fail("Warden did not visibly spawn after the keyboard door transition")
		return
	var warden: Node = _actors.get(_current_enemy_id)
	if _actor_family(warden) != "warden":
		_driver_fail("keyboard flow active target is not the Warden")
		return
	if not await _drive_validation_attacks(true, 6000):
		_driver_fail("Warden did not die through keyboard Space attacks")
		return
	deadline = Time.get_ticks_msec() + 3000
	while not _authoritative_state.activity_complete and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if not _authoritative_state.activity_complete:
		_driver_fail("relay_awakening did not complete through keyboard controls")
		return
	if int(_authoritative_state.inventory.get("relay_core_fragment", 0)) != initial_fragment_count + 1:
		_driver_fail("keyboard completion did not project exactly one fragment reward")
		return
	if int(_authoritative_state.progression.get("experience", 0)) != initial_experience + 100:
		_driver_fail("keyboard completion did not project exactly 100 experience")
		return
	if "COMPLETE" not in _objective_label.text or "MISSION COMPLETE" not in _guidance_label.text:
		_driver_fail("keyboard completion is not visibly explained")
		return
	var capture_path := OS.get_environment("REVENANT_CAPTURE_M24_KEYBOARD")
	if not capture_path.is_empty() and await _save_review_capture(capture_path) != OK:
		_driver_fail("keyboard completion capture could not be saved")
		return
	print("M24 keyboard-only flow validated: equipment, attack, movement, door, Warden, reward and completion")
	_quit_client(0)


func _drive_validation_attacks(keyboard: bool, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while _current_enemy_id != 0 and Time.get_ticks_msec() < deadline:
		if keyboard:
			await _tap_validation_key(KEY_SPACE)
		else:
			_request_ui_attack()
		var interval_seconds := float(_current_attack_cooldown_ms() + 80) / 1000.0
		if _showcase_mode:
			interval_seconds = maxf(interval_seconds, 0.68)
		await get_tree().create_timer(interval_seconds).timeout
	return _current_enemy_id == 0


func _tap_validation_key(keycode: Key) -> void:
	await _set_validation_key(keycode, true)
	await _set_validation_key(keycode, false)


func _set_validation_key(keycode: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.physical_keycode = keycode
	event.pressed = pressed
	Input.parse_input_event(event)
	await get_tree().process_frame


func _driver_fail(message: String) -> void:
	_fail(message)
	_quit_client(1)


func _material(color: Color) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.metallic = 0.35
	material.roughness = 0.42
	return material


func _set_guidance(title: String, instructions: String) -> void:
	if _guidance_mode == "Compact":
		_guidance_label.text = title
	elif _guidance_mode == "Off":
		_guidance_label.text = title if title in ["WHAT HAPPENED?", "MISSION COMPLETE"] else "GUIDANCE OFF"
	else:
		_guidance_label.text = "%s\n%s" % [title, instructions]


func _refresh_onboarding() -> void:
	if not _challenge_contract.is_empty():
		_refresh_challenge_guidance()
		return
	if _onboarding == null or _guidance_label == null:
		return
	if _campaign_session:
		_refresh_campaign_guidance()
		return
	var guidance: Dictionary = _onboarding.call("guidance")
	var signal_objective: Dictionary = _authoritative_state.objectives.get("recover_lost_signal", {})
	if signal_objective.get("state") == "Active":
		guidance["title"] = "RECOVER THE SIGNAL" if signal_objective.get("progress") == 1 else "SENTINEL  •  USE COVER"
		guidance["instructions"] = "Approach the marked terminal behind the sentinel." if signal_objective.get("progress") == 1 else "The barrier blocks both sides. Go around its ends to fire."
	elif signal_objective.get("state") == "Completed" and _camera_phase == "signal":
		guidance["title"] = "A VOICE BEYOND THE RELAY"
		guidance["instructions"] = "“Vale is alive.” Signal received. Head east to the core."
	elif _signal_available() and guidance.get("step") == "Door":
		guidance["title"] = "CORE OR LOST SIGNAL"
		guidance["instructions"] = "Head east to the core, or follow the west beacon for an optional encounter."
	var coolant_phase := _coolant_phase()
	if _camera_phase == "coolant" and not coolant_phase.is_empty():
		var advice: Array = {
			"transfer": ["COOLANT • STATION 2", "Hold at [4, -5] for 1.2 s. Finish both stations within 8 s."],
			"delivery": ["COOLANT • STATION 3", "Hold at [4, -1] for 1.2 s to deliver the charge."],
			"expired": ["CHARGE EXPIRED", "Return to intake [-1, -5] to retry, or head east to the core."],
			"completed": ["COOLANT DELIVERED", "The north relay is stable. Head east to the core and Warden."],
		}[coolant_phase]
		guidance["title"] = advice[0]
		guidance["instructions"] = advice[1]
	elif _coolant_available() and guidance.get("step") == "Door":
		guidance["title"] = "CHOOSE YOUR NEXT STEP"
		guidance["instructions"] = "Core east; Lost Signal west; Coolant Run north at [-1, -5]."
	if _meridian_available() and guidance.get("step") == "Door":
		guidance["instructions"] = "Core east; Meridian due west at [-12, 0]; Signal southwest; Coolant north."
	if _session.get("encounter_capable") and _signal_available() and guidance.get("step") == "Door":
		guidance["instructions"] = "Core east; Meridian west; Signal southwest; Coolant north; Lancer south at [4, 8]."
	if _session.get("bulwark_capable") and _signal_available() and guidance.get("step") == "Door":
		guidance["instructions"] = "Core east; Meridian west. Train against Lancer south [4, 8] or Bulwark north [6, -8]."
	if _session.get("support_capable") and _signal_available() and guidance.get("step") == "Door":
		guidance["instructions"] = "Core east; Meridian west. Lancer [4, 8], Bulwark [6, -8], Mender pair [2, 6]."
	if _session.get("elite_capable") and _signal_available() and guidance.get("step") == "Door":
		guidance["instructions"] = "Core east; Meridian west. Elites: Link [5,-6], Guard [5,-10]."
	var lancer_state: String = _authoritative_state.objectives.get("glass_lancer", {}).get("state", "")
	if lancer_state == "Active":
		guidance["title"] = "LANCER • STEP OFF THE LINE"
		guidance["instructions"] = "Step off the line. Attack during recovery. Leave the pad to withdraw."
	elif not lancer_state.is_empty() and guidance.get("step") == "Door":
		guidance["title"] = "CONTINUE TO THE CORE"
		guidance["instructions"] = "Head north off the pad, then east to the core and Warden."
	var bulwark_state: String = _authoritative_state.objectives.get("steel_bulwark", {}).get("state", "")
	if bulwark_state == "Active":
		guidance["title"] = "BULWARK • FLANK THE SHIELD"
		guidance["instructions"] = "Avoid the front wedge. Attack the sides or wait for the shield to lower. Leave the pad to withdraw."
	elif not bulwark_state.is_empty() and guidance.get("step") == "Door":
		guidance["title"] = "CONTINUE TO THE CORE"
		guidance["instructions"] = "Head south off the pad to the core door [6, 0]."
	var support_state: String = _authoritative_state.objectives.get("relay_mender", {}).get("state", "")
	if support_state == "Active":
		guidance["title"] = "STOP MENDER REPAIRS"
		guidance["instructions"] = tr("Targets: click or [%s]. Dodge charge lines. Leave the pad to retreat.") % preload("res://input/input_bindings.gd").key_label(_settings.bindings, "target_next")
	elif not support_state.is_empty() and guidance.get("step") == "Door":
		guidance["title"] = "CONTINUE TO THE CORE"
		guidance["instructions"] = "Head north off the pad, then east to the core and Warden."
	var elite: Dictionary = _elite_objective()
	if elite.get("state") == "Active":
		guidance["title"] = "BREAK THE REPAIR LINK" if elite.get("objective_id") == "bastion_link" else "ALTERNATING ATTACKS"
		guidance["instructions"] = tr("Targets: click or [%s]. Flank the shield; dodge the marks. Leave the pad to retreat.") % preload("res://input/input_bindings.gd").key_label(_settings.bindings, "target_next")
	elif not elite.is_empty() and guidance.get("step") == "Door":
		guidance["title"] = "CONTINUE TO THE CORE"
		guidance["instructions"] = "Head south off the pad to the core door [6, 0]."
	if _meridian_active() and _current_enemy_id == 0 and not _authoritative_state.activity_complete:
		var objectives: Dictionary = _authoritative_state.objectives
		var survey_done: bool = objectives.get("meridian_gallery", {}).get("state") == "Completed"
		var log_done: bool = objectives.get("meridian_return", {}).get("state") == "Completed"
		_objective_label.text = "MERIDIAN • SURVEY %d/2 • LOG %d/1" % [int(objectives.get("meridian_lens", {}).get("state") == "Completed") + int(survey_done), int(log_done)]
		guidance["title"] = "MERIDIAN • EXPLORE AT YOUR PACE"
		guidance["instructions"] = "Survey the north lens, then the south gallery. Bring the west log back to Arrival. Return east anytime."
		if _meridian_zone in ["", "approach"]:
			guidance["instructions"] = "Meridian is west. Continue east to the Warden whenever you are ready."
		var actor: Dictionary = _authoritative_state.actors.get(_authoritative_state.player_actor_id, {})
		var p: Array = actor.get("position", [0, 0, 0])
		if Vector2(p[0] + 31, p[2] - 8).length() <= 1.5 and objectives.has("meridian_memory"):
			guidance["title"] = "A NAME IN THE GLASS"
			guidance["instructions"] = "“Vale, we kept your window facing home.” A name scratched beneath the last intact star."
		elif _meridian_zone == "bridge" and objectives.get("meridian_log", {}).get("state") == "Completed":
			guidance["title"] = "THE KEEPER'S LAST ROUND"
			guidance["instructions"] = "“The relay went dark. We left the sky open.” Carry this log east to the Arrival marker."
		elif survey_done and log_done:
			guidance["title"] = "MERIDIAN RECORDED"
			guidance["instructions"] = "The lens still points home. Follow the pale paths east to the hub and Warden."
	if guidance.get("step") == "Warden" and _arc_surge_active():
		guidance["title"] = "ARC WARDEN  •  TRACK THE JUMPS"
		guidance["instructions"] = "Each hit makes it jump. Follow the trail and aim again."
	if _session.get("prism_capable") and _signal_available() and guidance.get("step") == "Door":
		guidance["instructions"] = "Core [6,0]. Optional Prism Warden [5,2]. Elites north; Meridian west."
	if _prism_fighting():
		var prism_guide := _prism_guidance()
		guidance["title"] = tr(prism_guide[0])
		guidance["instructions"] = tr(prism_guide[1])
	if not guidance.get("visible", false):
		_guidance_label.text = "GUIDANCE OFF" if _guidance_mode == "Off" else "GUIDANCE HIDDEN\n\nPress H to review this step."
	else:
		_guidance_label.text = guidance.get("title", "") if guidance.get("compact", false) else "%s\n%s" % [guidance.get("title", ""), guidance.get("instructions", "")]


func _append_input_log(message: String) -> void:
	if _input_log_label == null:
		return
	_input_events.push_front("• %s" % message)
	if _input_events.size() > 5:
		_input_events.resize(5)
	_input_log_label.text = "WINDOW FOCUS: %s\n%s" % ["YES" if get_window().has_focus() else "NO", "\n".join(_input_events)]


func _validate_playable_slice() -> void:
	var validation_error := AUDIO_HARNESS.new().validate(_audio_director, _door)
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	var display_fixtures := {
		"root": self,
		"tree": get_tree(),
		"entry_shell": _entry_shell,
		"settings_store": _settings_store,
		"settings_panel": _settings_panel,
		"onboarding": _onboarding,
		"presentation_polish": _presentation_polish,
		"loot_pickup": _loot_pickup,
		"sector_transition": _sector_transition,
		"player_intents": _player_intents,
		"status_label": _status_label,
		"objective_label": _objective_label,
		"controls_label": _controls_label,
		"action_grid": _action_grid,
		"input_log_label": _input_log_label,
		"inventory_label": _inventory_label,
		"inventory_panel": _inventory_panel,
		"progression_label": _progression_label,
		"open_settings": Callable(self, "_open_settings"),
		"apply_settings": Callable(self, "_apply_settings"),
		"current_settings": Callable(self, "_validation_settings"),
		"guidance_mode": Callable(self, "_validation_guidance_mode"),
	}
	validation_error = await DISPLAY_FIRST_CONTACT_HARNESS.new().validate(display_fixtures)
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	var experience_fixtures := {
		"tree": get_tree(),
		"entry_shell": _entry_shell,
		"settings_store": _settings_store,
		"settings_panel": _settings_panel,
		"onboarding": _onboarding,
		"hud_canvas": _hud_canvas,
		"status_label": _status_label,
		"controls_label": _controls_label,
		"action_grid": _action_grid,
		"presentation_polish": _presentation_polish,
		"open_settings": Callable(self, "_open_settings"),
		"apply_settings": Callable(self, "_apply_settings"),
		"refresh_onboarding": Callable(self, "_refresh_onboarding"),
		"save_capture": Callable(self, "_save_review_capture"),
		"guidance_mode": Callable(self, "_validation_guidance_mode"),
		"current_settings": Callable(self, "_validation_settings"),
	}
	validation_error = await EXPERIENCE_HARNESS.new().validate(experience_fixtures)
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	var presentation_fixtures := {
		"camera": _camera,
		"door": _door,
		"status_label": _status_label,
		"health_label": _health_label,
		"enemy_health_label": _enemy_health_label,
		"position_label": _position_label,
		"objective_label": _objective_label,
		"crosshair": _crosshair,
		"guidance_label": _guidance_label,
		"input_log_label": _input_log_label,
		"inventory_label": _inventory_label,
		"inventory_panel": _inventory_panel,
		"progression_label": _progression_label,
		"equipment_label": _equipment_label,
		"hud_frame": _hud_frame,
		"presentation_polish": _presentation_polish,
		"loot_pickup": _loot_pickup,
		"sector_transition": _sector_transition,
		"player_health_bar": _player_health_bar,
		"enemy_health_bar": _enemy_health_bar,
		"movement_buttons": _movement_buttons,
		"attack_button": _attack_button,
		"weapon_buttons": _weapon_buttons,
		"module_button": _module_button,
		"route_button": _route_button,
		"cooperation_button": _cooperation_button,
	}
	var presentation_harness := PRESENTATION_HARNESS.new()
	validation_error = presentation_harness.validate_foundation(presentation_fixtures)
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	validation_error = BOUNDARY_HARNESS.new().validate(_session)
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	validation_error = MODULE_WORKSHOP_HARNESS.new().validate({
		"workshop": _module_workshop,
		"new_operation_id": Callable(self, "_new_module_operation_id"),
	})
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	validation_error = ROUTE_CONSOLE_HARNESS.new().validate({
		"console": _route_console,
	})
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	validation_error = COOPERATION_CONSOLE_HARNESS.new().validate({
		"console": _cooperation_console,
	})
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	var combat_harness := COMBAT_HARNESS.new()
	validation_error = combat_harness.validate()
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	validation_error = PLAYTEST_OBSERVATION_HARNESS.new().validate()
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	presentation_fixtures.merge({
		"root": self,
		"tree": get_tree(),
		"environment": _environment,
		"combat_vfx": _combat_vfx,
		"loot_pickup": _loot_pickup,
		"sector_transition": _sector_transition,
		"save_capture": Callable(self, "_save_review_capture"),
		"onboarding": _onboarding,
		"refresh_onboarding": Callable(self, "_refresh_onboarding"),
	})
	validation_error = await combat_harness.validate_presentation({
		"root": self,
		"tree": get_tree(),
		"combat_vfx": _combat_vfx,
		"audio_director": _audio_director,
		"status_label": _status_label,
		"health_label": _health_label,
		"enemy_health_label": _enemy_health_label,
		"controls_label": _controls_label,
		"action_grid": _action_grid,
		"save_capture": Callable(self, "_save_review_capture"),
	})
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	var scene_validation: Dictionary = await presentation_harness.validate_scene(presentation_fixtures)
	validation_error = scene_validation.get("error", "")
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	var operator: Node3D = scene_validation.operator
	var drone: Node3D = scene_validation.drone
	var warden: Node3D = scene_validation.warden
	if OS.get_environment("REVENANT_MEASURE_M22") == "1":
		var audio_harness := AUDIO_HARNESS.new()
		var measurement: Dictionary = await audio_harness.measure(get_tree(), _audio_director, operator, drone, warden)
		validation_error = audio_harness.validate_measurement(measurement)
		if not validation_error.is_empty():
			_validation_fail(validation_error)
			return
	operator.queue_free()
	await get_tree().process_frame
	print("M17 playable slice validated: camera, HUD, movement, aiming and attack inputs are ready")
	print("M18 inventory HUD validated: authoritative snapshot and loot presentation are ready")
	print("M19 progression HUD validated: authoritative experience and level presentation are ready")
	print("M20 loadout HUD validated: authoritative weapon selection and profiles are ready")
	print("M21 Operator validated: modular silhouette, distinct weapons and authoritative animation states are ready")
	print("M21 relay-hub environment validated: modular room, semantic lighting and authoritative door are ready")
	print("M21 enemies validated: distinct families and honest authoritative telegraphs are ready")
	print("M21 combat VFX validated: bounded shot, trail, impact, damage, cooldown and corruption feedback are ready")
	print("M21 Operator HUD validated: complete M17-M20 state, semantic hierarchy and redundant health feedback are ready")
	print("M21 presentation polish validated: consistent confirmed feedback and bounded scene budgets are ready")
	print("M21 presentation captures validated: reproducible overview, telegraph and combat shots are ready")
	print("M22 entry shell validated: explicit connection, safe identity, focus and retry states are ready")
	print("M22 settings validated: local persistence, buses, display, guidance and reduced flash are ready")
	print("M22 onboarding validated: local attempts, authoritative progress and revisitable guidance are ready")
	print("M22 audio foundation validated: original ambience, bounded cues, routing and silent mode are ready")
	print("M22 combat audio validated: Operator, weapons, enemies, confirmed damage, defeat and interface cues are ready")
	print("M22 integration evidence validated: mix measurement and reproducible review captures are ready")
	print("M23 MessagePack boundary validated: exact signed bytes and supported round-trip are preserved")
	print("M23 framed transport validated: socket, buffering, deadlines and 64 KiB ceiling are isolated")
	print("M23 session controller validated: negotiation, identity, join and initial snapshots are isolated")
	print("M23 authoritative state projection validated: actors, rewards, equipment, objectives and completion are server-owned")
	print("M23 input and HUD coordination validated: local attempts stay intents and server facts drive presentation")
	print("M24 local observation validated: opt-in allow-list, bounded atomic storage and consent deletion are ready")
	print("M24 display and first-contact validated: scaled desktop layout, keyboard focus, preferences and recovery are ready")
	print("M25 authoritative combat cadence validated: equipped cooldowns and profile-switch boundaries are exact")
	print("M25 honest combat presentation validated: attempt, cooling, unavailable, confirmed hit, hostile hit and defeat cues are distinct and bounded")
	print("M26 honest module workshop validated: server preview, pending, rejection, acceptance, replay and next-session disclosure are distinct")
	print("M27 honest route console validated: opt-in, leader, pending, selection, terminal and accessibility states are server-owned")
	print("M28 honest cooperation console validated: roles, ping, life, revive, contributions, terminal truth and accessibility are server-owned")
	_audio_director.call("shutdown")
	_audio_director.queue_free()
	_audio_director = null
	await get_tree().process_frame
	await get_tree().create_timer(0.1).timeout
	get_tree().quit(0)


func _validation_guidance_mode() -> String:
	return _guidance_mode


func _validation_settings() -> Dictionary:
	return _settings.duplicate(true)


func _observation_viewport_size() -> Vector2i:
	var window_size := DisplayServer.window_get_size()
	if window_size.x > 0 and window_size.y > 0:
		return window_size
	return Vector2i(get_viewport().get_visible_rect().size)


func _validation_fail(message: String) -> void:
	_fail(message)
	get_tree().quit(1)


func _save_review_capture(path: String) -> Error:
	await get_tree().process_frame
	var capture_image := get_viewport().get_texture().get_image()
	if capture_image == null:
		return ERR_CANT_CREATE
	return capture_image.save_png(path)


func _fail(message: String, failure_kind := "") -> void:
	_observe_terminal_outcome("failed")
	push_error("Revenant session failed: %s" % message)
	if not _authoritative_state.module_pending.is_empty():
		_authoritative_state.call("fail_module_request", message, true)
		_refresh_module_workshop()
	if (
		not _authoritative_state.route_pending.is_empty()
		or (not _authoritative_state.route_state.is_empty() and _authoritative_state.route_summary.is_empty())
	):
		_authoritative_state.call("fail_route_request", message, true)
		_refresh_route_console()
	if (
		not _authoritative_state.cooperation_pending.is_empty()
		or (not _authoritative_state.cooperation_state.is_empty() and _authoritative_state.cooperation_summary.is_empty())
	):
		_authoritative_state.call("fail_cooperation_request", message, true)
		_refresh_cooperation_console()
	if _module_workshop != null:
		_module_workshop.call("close_workshop")
	if _route_console != null:
		_route_console.call("close_console")
	if _cooperation_console != null:
		_cooperation_console.call("close_console")
	_connection_started = false
	if _entry_shell != null and not _should_exit_after_flow():
		_hud_canvas.visible = false
		_entry_shell.call("show_failure", message)
	if _status_label != null:
		_status_label.text = "CONNECTION INTERRUPTED  •  RETRY FROM START"
		_status_label.modulate = Color("ff6b6b")
	if _objective_label != null:
		_objective_label.text = "OBJECTIVE  •  ACTIVITY DID NOT COMPLETE"
	if _controls_label != null:
		_controls_label.text = "RETURN TO ENTRY AND RETRY FROM THE BEGINNING"
	if _guidance_label != null:
		var guidance := "The relay connection failed. Retry from the beginning after the operator confirms service health."
		if failure_kind == "session_unavailable":
			guidance = "This relay already has an active run. Retry from the beginning after the current players leave."
		elif failure_kind == "rejected":
			guidance = "This client was rejected as incompatible or invalid. Verify the playtest build before retrying."
		elif failure_kind == "timeout":
			guidance = "The relay did not answer before the deadline. Verify service health, then retry from the beginning."
		_set_guidance("WHAT HAPPENED?", guidance)
	var failure_capture := OS.get_environment("REVENANT_CAPTURE_M24_FAILURE")
	if not failure_capture.is_empty() and not _should_exit_after_flow():
		call_deferred("_capture_failure_validation", failure_capture)
	if _should_exit_after_flow():
		_quit_client(1)


func _capture_failure_validation(path: String) -> void:
	await get_tree().process_frame
	await get_tree().process_frame
	var entry_state: Dictionary = _entry_shell.call("presentation_state")
	if (
		entry_state.get("state") != "Failed"
		or entry_state.get("focus_owner") != "Connect"
		or entry_state.get("connect_text") != "RETRY CONNECTION"
		or not entry_state.get("connect_enabled", false)
	):
		push_error("M24 failure state is not keyboard-ready for Retry from start")
		_quit_client(1)
		return
	if await _save_review_capture(path) != OK:
		push_error("M24 failure-state capture could not be saved")
		_quit_client(1)
		return
	print("M24 failure entry validated: Retry from start is focused and keyboard reachable")
	_quit_client(0)


func _quit_client(exit_code: int) -> void:
	if _quitting:
		return
	_quitting = true
	_observe_first("quit_requested")
	if _playtest_observation != null:
		if _playtest_observation.call("report").get("terminal_outcome") == "running":
			_observe_terminal_outcome("quit")
		_playtest_observation.call("finalize")
	if _audio_director != null:
		_audio_director.call("shutdown")
		_audio_director.queue_free()
		_audio_director = null
	await get_tree().process_frame
	await get_tree().create_timer(0.1).timeout
	get_tree().quit(exit_code)


func _should_exit_after_flow() -> bool:
	return OS.get_environment("REVENANT_EXIT_AFTER_FLOW") == "1"


func _should_auto_connect() -> bool:
	return (
		_should_exit_after_flow()
		or OS.get_environment("REVENANT_VALIDATE_MANUAL_FLOW") == "1"
		or OS.get_environment("REVENANT_VALIDATE_KEYBOARD_FLOW") == "1"
		or OS.get_environment("REVENANT_VALIDATE_MODULE_FLOW") == "1"
		or not OS.get_environment("REVENANT_VALIDATE_ROUTE_FLOW").is_empty()
		or OS.get_environment("REVENANT_VALIDATE_COOPERATION_FLOW") == "1"
	)


func _default_username() -> String:
	var username := OS.get_environment("REVENANT_GAME_USERNAME")
	return "revenant-godot" if username.is_empty() else username


func _connection_host() -> String:
	var host := OS.get_environment("REVENANT_GAME_HOST")
	return DEFAULT_HOST if host.is_empty() else host


func _connection_port() -> int:
	var port_text := OS.get_environment("REVENANT_GAME_PORT")
	return DEFAULT_PORT if port_text.is_empty() else int(port_text)


func _set_connection_state(state: String, detail: String) -> void:
	if _entry_shell != null:
		_entry_shell.call("set_connection_state", state, detail)
	if state == "Waiting" and _audio_director != null:
		_audio_director.call("play_system_ready")


func _exit_tree() -> void:
	# Script-backed translations must be released before the script runtime.
	preload("res://presentation/settings/settings_store.gd").shutdown_translation()


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		_player_intents.call("clear_ui_movement")
		_append_input_log("WINDOW FOCUS LOST  •  MOVEMENT CLEARED")
	elif what == NOTIFICATION_APPLICATION_FOCUS_IN:
		_append_input_log("WINDOW FOCUS RESTORED")
	elif what == NOTIFICATION_WM_CLOSE_REQUEST and _playtest_observation != null and _playtest_observation.call("is_active"):
		_quit_client(0)


func _observe_first(key: String) -> void:
	if _playtest_observation != null and _playtest_observation.call("is_active"):
		_playtest_observation.call("record_first", key)


func _observe_connection_outcome(outcome: String) -> void:
	if _playtest_observation != null and _playtest_observation.call("is_active"):
		_playtest_observation.call("set_connection_outcome", outcome)


func _observe_connection_failure(message: String, classified_outcome := "") -> void:
	if not classified_outcome.is_empty():
		_observe_connection_outcome(classified_outcome)
		return
	var normalized := message.to_lower()
	var outcome := "transport_failure"
	if "timed out" in normalized:
		outcome = "timeout"
	elif "rejected" in normalized and "world join" in normalized:
		outcome = "session_unavailable"
	elif "rejected" in normalized:
		outcome = "rejected"
	_observe_connection_outcome(outcome)


func _observe_terminal_outcome(outcome: String) -> void:
	if _playtest_observation != null and _playtest_observation.call("is_active"):
		_playtest_observation.call("set_terminal_outcome", outcome)


func _observe_cooldown_acknowledgement() -> void:
	if _playtest_observation != null and _playtest_observation.call("is_active"):
		_playtest_observation.call("increment_cooldown_acknowledgement")


func _refresh_story_context() -> void:
	if _story_from_menu:
		_story_panel.call("update_context", _authoritative_state.campaign_state, [], true, true)
		_story_markers.visible = false
		return
	var player: Dictionary = _authoritative_state.actors.get(_authoritative_state.player_actor_id, {})
	var safe: bool = not player.is_empty() and _authoritative_state.actor_health.get(_authoritative_state.player_actor_id, 0) > 0
	for actor_id in _authoritative_state.actors:
		if _authoritative_state.actors[actor_id].get("actor_kind") == "enemy" and _authoritative_state.actor_health.get(actor_id, 0) > 0: safe = false
	safe = safe and _hud_canvas.visible and not _other_surface_open(_story_panel) and not _settings_panel.visible and not _entry_shell.visible
	var position: Array = player.get("position", [0, 0, 0])
	_story_panel.call("update_context", _authoritative_state.campaign_state, position, safe)
	_archive_button.text = tr("INTERACT [E]") if not _story_panel.call("available_actions").is_empty() else tr("JOURNAL [J]")
	if _settings.has("bindings"):
		for pair in [["[E]", "interact"], ["[J]", "archive"]]:
			_archive_button.text = _archive_button.text.replace(pair[0], "[%s]" % preload("res://input/input_bindings.gd").key_label(_settings.bindings, pair[1]))
	_archive_button.disabled = not safe
	_archive_hint.text = ""
	_story_markers.call("present", _authoritative_state.campaign_state, _hud_canvas.visible)


func _close_story_journal() -> void:
	var return_to_menu := _story_from_menu
	_story_from_menu = false
	_story_panel.visible = false
	_player_intents.call("clear_pending_input")
	get_viewport().gui_release_focus()
	if return_to_menu: _entry_shell.get("_connect_button").grab_focus()


func _request_story_action(action: String) -> void:
	if _story_from_menu: return
	if not _story_operation.is_empty() or action not in _story_panel.call("available_actions"): return
	var active: Variant = _authoritative_state.campaign_state.get("active")
	if not active is Dictionary: return
	_story_operation = "ui-%d-%d" % [Time.get_ticks_usec(), randi()]
	_story_panel.call("set_pending", true)
	if not _send_message({"type": "CampaignStoryIntent", "operation_id": _story_operation, "run_id": active.get("run_id"), "expected_revision": _authoritative_state.campaign_state.get("state_revision"), "action": action}):
		_story_panel.call("show_result", "unconfirmed")
		_story_operation = ""


func _update_campaign_menu(username: String, snapshot: Dictionary) -> void:
	var projection := AUTHORITATIVE_STATE.new()
	if projection.call("apply_campaign_snapshot", snapshot):
		_entry_shell.call("set_campaign_progress", username, snapshot)


func _show_completed_campaign(snapshot: Dictionary) -> void:
	var projection := AUTHORITATIVE_STATE.new()
	if not projection.call("apply_campaign_snapshot", snapshot) or snapshot.get("cleared_chapters") != 6 or snapshot.get("active") != null:
		_fail("server returned an invalid completed campaign")
		return
	_connection_started = false
	_story_operation = ""
	_story_from_menu = true
	_hud_canvas.visible = false
	_authoritative_state.campaign_state = snapshot.duplicate(true)
	_entry_shell.call("show_entry", _connection_username)
	_story_panel.call("reset_for_connection")
	_story_panel.call("update_context", snapshot, [], true, true)
	_story_panel.call("open_journal")
