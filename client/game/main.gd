extends Node

const DEFAULT_HOST := "127.0.0.1"
const DEFAULT_PORT := 7000
const OPERATOR_SCENE := preload("res://presentation/operator/operator.tscn")
const RELAY_HUB_ROOM_SCENE := preload("res://presentation/environment/relay_hub_room.tscn")
const RELAY_DRONE_SCENE := preload("res://presentation/enemies/relay_drone/relay_drone.tscn")
const WARDEN_SCENE := preload("res://presentation/enemies/warden/warden.tscn")
const COMBAT_VFX_SCENE := preload("res://presentation/combat/combat_vfx.tscn")
const OPERATOR_HUD_SCENE := preload("res://presentation/hud/operator_hud.tscn")
const PRESENTATION_POLISH_SCENE := preload("res://presentation/polish/presentation_polish.tscn")
const ENTRY_SHELL_SCENE := preload("res://presentation/entry/entry_shell.tscn")
const SETTINGS_PANEL_SCENE := preload("res://presentation/settings/settings_panel.tscn")
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
const EXPERIENCE_HARNESS := preload("res://validation/experience_harness.gd")
const PRESENTATION_HARNESS := preload("res://validation/presentation_harness.gd")
const PLAYTEST_OBSERVATION := preload("res://playtest/local_observation_report.gd")
const PLAYTEST_OBSERVATION_HARNESS := preload("res://validation/playtest_observation_harness.gd")

var _session: Node
var _authoritative_state := AUTHORITATIVE_STATE.new()
var _player_intents := PLAYER_INTENT_CONTROLLER.new()
var _hud_projection := HUD_PROJECTION.new()
var _actors := {}
var _current_enemy_id := 0
var _camera: Camera3D
var _environment: Node3D
var _combat_vfx: Node3D
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
var _weapon_buttons: Array[Button] = []
var _hud_canvas: CanvasLayer
var _entry_shell: Control
var _connection_started := false
var _settings_store: RefCounted
var _settings := {}
var _settings_panel: Control
var _guidance_mode := "Full"
var _onboarding: RefCounted
var _audio_director: Node3D
var _playtest_observation: RefCounted
var _quitting := false


func _ready() -> void:
	_session = SESSION_CONTROLLER.new()
	_session.name = "SessionController"
	_session.connect("connection_state_requested", _set_connection_state)
	add_child(_session)
	_build_playable_scene()
	_build_entry_shell()
	_build_settings()
	_playtest_observation = PLAYTEST_OBSERVATION.new()
	_playtest_observation.call("configure_from_environment", _settings, Vector2i(get_viewport().get_visible_rect().size))
	print(_playtest_observation.call("diagnostic"))
	if _playtest_observation.call("is_active"):
		get_tree().auto_accept_quit = false
	_onboarding = ONBOARDING_CONTROLLER.new()
	_onboarding.call("reset", _guidance_mode)
	if OS.get_environment("REVENANT_VALIDATE_SLICE") == "1":
		call_deferred("_validate_playable_slice")
		return
	if _should_auto_connect():
		call_deferred("_begin_connection", _default_username())


func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_H:
		_onboarding.call("toggle")
		_refresh_onboarding()
		return
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_ESCAPE:
		if _settings_panel != null and _settings_panel.visible:
			return
		if _entry_shell != null and not _entry_shell.visible:
			_open_settings(null)
			get_viewport().set_input_as_handled()
			return
	if event is InputEventKey and event.pressed and not event.echo:
		if event.physical_keycode in [KEY_W, KEY_A, KEY_S, KEY_D, KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT]:
			_append_input_log("KEY %s detected" % event.as_text_physical_keycode())
		if event.is_action_pressed("attack"):
			_player_intents.call("request_attack", false)
			_append_input_log("SPACE attack detected")
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		_player_intents.call("request_attack", false)
		_append_input_log("LEFT CLICK detected")


func _begin_connection(username: String) -> void:
	if _connection_started:
		return
	_connection_started = true
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
	if not joined.get("ok", false):
		_observe_connection_failure(joined.get("error", ""))
		_fail(joined.get("error", "initial session join failed"))
		return
	_observe_connection_outcome("connected")
	_authoritative_state.call("join_world", joined.get("world", {}))
	_apply_inventory_snapshot(joined.get("inventory", {}))
	_apply_progression(joined.get("progression", {}))
	_apply_equipment_snapshot(joined.get("equipment", {}))
	_set_connection_state("Waiting", "Relay joined. Waiting for the server to begin the activity.")
	_entry_shell.call("dismiss")
	_hud_canvas.visible = true
	_status_label.text = "CONNECTED  •  RELAY-HUB"
	_controls_label.text = "WAITING FOR THE RELAY ACTIVITY..."
	_set_guidance("GETTING READY", "The server is preparing your encounter. This should take only a moment.")
	var activity := await _receive_message(Time.get_ticks_msec() + 5000)
	if activity.get("type") != "ActivityStart":
		_fail("expected ActivityStart")
		return
	var initial_objective := await _receive_message(Time.get_ticks_msec() + 5000)
	if initial_objective.get("type") != "ObjectiveUpdate" or initial_objective.get("state") != "Active":
		_fail("expected active initial objective")
		return
	print("activity %s started with objective %s" % [activity.get("activity_id"), initial_objective.get("objective_id")])
	_onboarding.call("reset", _guidance_mode)
	_refresh_onboarding()
	_set_connection_state("Playing", "Authoritative activity state received.")
	_entry_shell.call("dismiss")
	_update_objective_hud(initial_objective)

	var enemy_id := 0
	for _actor_index in range(2):
		var actor_spawn := await _receive_message(Time.get_ticks_msec() + 5000)
		if actor_spawn.get("type") != "ActorSpawn":
			_fail("expected ActorSpawn")
			return
		_render_actor(actor_spawn)
		if actor_spawn.get("actor_kind") == "enemy":
			enemy_id = actor_spawn.get("actor_id")
	if enemy_id == 0:
		_fail("server did not spawn the first enemy actor")
		return
	var enemy_moved := false
	while true:
		var ai_message := await _receive_message(Time.get_ticks_msec() + 5000)
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
	_update_enemy_proximity()
	if not _should_exit_after_flow():
		_status_label.text = "COMBAT ACTIVE  •  AIM AND FIRE"
		_set_guidance("STEP 1  •  CLEAR THE DRONE", "Move with WASD. Aim the orange crosshair at the red drone, then click or press Space three times.")
		if OS.get_environment("REVENANT_VALIDATE_MANUAL_FLOW") == "1":
			call_deferred("_drive_manual_activity")
		await _run_manual_activity()
		return

	while true:
		if not _send_message({"type": "AttackIntent", "target_actor_id": enemy_id}):
			_fail("AttackIntent send failed")
			return
		var damage := await _receive_message(Time.get_ticks_msec() + 5000)
		if damage.get("type") != "DamageApplied":
			_fail("expected DamageApplied")
			return
		_authoritative_state.call("apply_damage", damage)
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
		var boss_damage := await _receive_message(Time.get_ticks_msec() + 5000)
		if boss_damage.get("type") != "DamageApplied":
			_fail("expected boss DamageApplied")
			return
		_authoritative_state.call("apply_damage", boss_damage)
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
		instance = WARDEN_SCENE.instantiate()
	else:
		instance = RELAY_DRONE_SCENE.instantiate()
	instance.name = "Actor_%s" % actor.get("actor_id")
	var position: Array = actor.get("position", [0, 0, 0])
	instance.position = Vector3(position[0], position[1], position[2])
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
	print("rendered %s actor %d (%s)" % [actor.get("actor_kind"), actor.get("actor_id"), actor.get("archetype")])


func _destroy_actor(actor_id: int) -> void:
	var actor: Node = _actors.get(actor_id)
	if actor != null:
		if actor is Node3D and actor_id != _authoritative_state.player_actor_id:
			_audio_director.call("play_confirmed_defeat", (actor as Node3D).global_position)
		_actors.erase(actor_id)
		if actor.has_method("retire"):
			actor.call("retire")
		else:
			actor.queue_free()
	_authoritative_state.call("apply_actor_destroy", {"actor_id": actor_id})
	if actor_id == _current_enemy_id and _enemy_health_label != null:
		_enemy_health_label.text = "ENEMY  •  DEFEATED"
		_enemy_health_bar.value = 0
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
	if update.get("actor_id") == _authoritative_state.player_actor_id and previous_position.distance_to(actor.position) > 0.01:
		_audio_director.call("play_confirmed_move", actor.global_position)
	_update_enemy_proximity()
	if update.get("actor_id") == _authoritative_state.player_actor_id and _position_label != null:
		_onboarding.call("confirm", "movement")
		_refresh_onboarding()
		var door_distance := maxi(0, 6 - int(position[0]))
		_position_label.text = "POSITION  [%d, %d]  •  DOOR %d STEPS" % [position[0], position[2], door_distance]
	print("actor %d moved to %s" % [update.get("actor_id"), position])


func _run_manual_activity() -> void:
	_controls_label.text = "WASD / ARROWS  MOVE    •    MOUSE / SPACE  ATTACK"
	while not _authoritative_state.activity_complete and _session.call("connection_status") == StreamPeerTCP.STATUS_CONNECTED:
		_handle_manual_input()
		var message := _try_receive_message()
		if not message.is_empty():
			_handle_manual_message(message)
		await get_tree().process_frame
	if not _authoritative_state.activity_complete:
		_observe_first("disconnect_observed")
		_observe_terminal_outcome("disconnected")


func _handle_manual_input() -> void:
	var now := Time.get_ticks_msec()
	_crosshair.position = get_viewport().get_mouse_position() - (_crosshair.size * 0.5)
	_update_target_highlight()
	var movement: Vector2 = _player_intents.call("movement", now)
	if movement.length() > 0.2:
		_observe_first("first_movement_attempt")
		_onboarding.call("note_local", "movement")
		var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
		if player != null:
			var step := Vector3(roundi(movement.x), 0, roundi(movement.y))
			var target := player.position + step
			target.x = clampi(roundi(target.x), -12, 12)
			target.z = clampi(roundi(target.z), -12, 12)
			var position := [roundi(target.x), 0, roundi(target.z)]
			var sent := _send_message({"type": "MoveIntent", "position": position})
			_append_input_log("MoveIntent %s %s" % [position, "sent" if sent else "FAILED"])
			_player_intents.call("consume_movement", now)
	var attack: Dictionary = _player_intents.call("take_attack", now)
	if attack.get("requested", false):
		_observe_first("first_attack_attempt")
		_onboarding.call("note_local", "attack")
		var target_id := _current_enemy_id if attack.get("use_active_target", false) else _aimed_enemy()
		if target_id != 0:
			var sent := _send_message({"type": "AttackIntent", "target_actor_id": target_id})
			_status_label.text = "ATTACK SENT  •  TARGET %d" % target_id if sent else "ATTACK FAILED  •  CONNECTION ERROR"
			_append_input_log("AttackIntent target %d %s" % [target_id, "sent" if sent else "FAILED"])
			_player_intents.call("consume_attack", now)
			if sent:
				_observe_cooldown_acknowledgement()
				var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
				if player != null:
					_combat_vfx.call("play_local_cooldown", player.global_position, 260)
				_audio_director.call("play_cooldown_acknowledgement")
		else:
			_append_input_log("Attack blocked: aim at active enemy")
	elif attack.get("cooling", false):
		_status_label.text = "WEAPON COOLING  •  %d MS" % attack.get("remaining_ms", 0)


func _handle_manual_message(message: Dictionary) -> void:
	match message.get("type"):
		"ActorUpdate":
			_update_actor(message)
		"DamageApplied":
			_handle_damage_feedback(message)
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
				_current_enemy_id = message.get("actor_id")
				if message.get("archetype") == "warden":
					_onboarding.call("confirm", "warden_spawn")
					_refresh_onboarding()
				_update_enemy_proximity()
				_status_label.text = "BOSS ONLINE  •  WARDEN"
				_set_guidance("STEP 3  •  DEFEAT THE WARDEN", "Aim at the purple Warden and attack until its HP reaches zero.")
		"ActivityComplete":
			_observe_first("completion_observed")
			_observe_terminal_outcome("completed")
			_authoritative_state.call("apply_activity_complete", message)
			_onboarding.call("confirm", "completion")
			_refresh_onboarding()
			_presentation_polish.call("play_authoritative_completion")
			_audio_director.call("play_completion")
			_status_label.text = "ACTIVITY COMPLETE  •  RELAY AWAKENED"
			_objective_label.text = "OBJECTIVE  •  COMPLETE"
			_controls_label.text = "RELAY_AWAKENING COMPLETED"
			_set_guidance("MISSION COMPLETE", "The relay is awake. Your authoritative reward is safely stored.")
			print("manual activity %s completed" % message.get("activity_id"))
		"LootGranted":
			_apply_loot_grant(message)
		"ProgressionGranted":
			_apply_progression(message)
		"EquipmentChanged":
			_apply_equipment_change(message)
			_onboarding.call("confirm", "equipment")
			_refresh_onboarding()
		_:
			_fail("unexpected manual gameplay message: %s" % message.get("type"))


func _try_receive_message() -> Dictionary:
	return _session.call("try_receive_message")


func _aimed_enemy() -> int:
	if _current_enemy_id == 0:
		return 0
	var enemy: Node3D = _actors.get(_current_enemy_id)
	if enemy == null or _camera.is_position_behind(enemy.global_position):
		return 0
	if _is_enemy_aimed(enemy) or Input.is_key_pressed(KEY_SPACE):
		return _current_enemy_id
	_status_label.text = "AIM AT THE HIGHLIGHTED TARGET"
	return 0


func _is_enemy_aimed(enemy: Node3D) -> bool:
	var cursor := get_viewport().get_mouse_position()
	var enemy_screen := _camera.unproject_position(enemy.global_position)
	return cursor.distance_to(enemy_screen) <= 110.0


func _update_target_highlight() -> void:
	if _current_enemy_id == 0:
		return
	var enemy: Node3D = _actors.get(_current_enemy_id)
	if enemy != null and enemy.has_method("set_targeted"):
		enemy.call("set_targeted", not _camera.is_position_behind(enemy.global_position) and _is_enemy_aimed(enemy))


func _update_enemy_proximity() -> void:
	if _current_enemy_id == 0 or _authoritative_state.player_actor_id == 0:
		return
	var enemy: Node3D = _actors.get(_current_enemy_id)
	var player: Node3D = _actors.get(_authoritative_state.player_actor_id)
	if enemy != null and player != null and enemy.has_method("set_danger_close"):
		enemy.call("set_danger_close", enemy.position.distance_to(player.position) <= 2.05)


func _actor_family(actor: Node) -> String:
	if actor != null and actor.has_method("presentation_state"):
		return actor.call("presentation_state").get("family", "relay-drone")
	return "relay-drone"


func _handle_damage_feedback(message: Dictionary) -> void:
	if message.get("source_actor_id") == _authoritative_state.player_actor_id:
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
		_health_label.text = "HP  %03d / 100  •  STABLE" % remaining
		_health_label.modulate = Color("ff6b6b")
		var health_tween := create_tween()
		health_tween.tween_property(_health_label, "modulate", Color.WHITE, 0.3)
	else:
		_status_label.text = "HIT %d  •  %d HP REMAINS" % [message.get("damage"), remaining]
		if _enemy_health_label != null:
			var maximum: int = _authoritative_state.actor_max_health.get(target_id, remaining)
			_enemy_health_label.text = "ENEMY  •  %03d / %03d HP" % [remaining, maximum]
			_enemy_health_bar.max_value = maximum
			_enemy_health_bar.value = remaining
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
	if objective.get("objective_type") == "ReachArea" and objective.get("state") == "Active":
		_onboarding.call("confirm", "door_objective")
		_refresh_onboarding()
	_objective_label.text = _hud_projection.call("objective_text", objective)
	if objective.get("objective_type") == "ReachArea" and objective.get("state") == "Active":
		_status_label.text = "RELAY DOOR UNLOCKED  •  MOVE TO X=6"
		_set_guidance("STEP 2  •  OPEN THE CORE", "Hold D to move right until you reach the orange relay door at x=6.")


func _apply_inventory_snapshot(message: Dictionary) -> void:
	_authoritative_state.call("apply_inventory_snapshot", message)
	_update_inventory_hud()


func _apply_loot_grant(message: Dictionary) -> void:
	var item_id: String = message.get("item_id", "unknown")
	_authoritative_state.call("apply_loot_grant", message)
	_update_inventory_hud()
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
	_update_operator_weapon()
	_update_equipment_hud()


func _apply_equipment_change(message: Dictionary) -> void:
	if not _authoritative_state.call("apply_equipment_change", message):
		_status_label.text = "EQUIP REJECTED  •  %s" % message.get("message", "SERVER REJECTED ITEM")
		return
	_update_operator_weapon()
	_update_equipment_hud()
	_status_label.text = "WEAPON EQUIPPED  •  %s" % _authoritative_state.equipped_weapon_item_id.replace("_", " ").to_upper()
	print("authoritative weapon equipped: %s" % _authoritative_state.equipped_weapon_item_id)


func _update_equipment_hud() -> void:
	var profile: Dictionary = _authoritative_state.weapon_profiles.get(_authoritative_state.equipped_weapon_item_id, {})
	_equipment_label.text = _hud_projection.call("equipment_text", _authoritative_state.equipped_weapon_item_id, profile)


func _update_operator_weapon() -> void:
	var player: Node = _actors.get(_authoritative_state.player_actor_id)
	if player != null and player.has_method("set_weapon"):
		player.call("set_weapon", _authoritative_state.equipped_weapon_item_id)


func _set_door_open(open: bool) -> void:
	_environment.call("set_core_door_open", open, true)
	_status_label.text = "RELAY CORE OPEN  •  WARDEN INBOUND" if open else "RELAY CORE SEALED"
	_audio_director.call("apply_door_state", open, _door.global_position)
	if open:
		_set_guidance("CORE OPEN", "The Warden is spawning. Get ready to aim and attack.")


func _build_playable_scene() -> void:
	_environment = RELAY_HUB_ROOM_SCENE.instantiate()
	add_child(_environment)
	_door = _environment.call("get_core_door")
	_combat_vfx = COMBAT_VFX_SCENE.instantiate()
	add_child(_combat_vfx)
	_presentation_polish = PRESENTATION_POLISH_SCENE.instantiate()
	add_child(_presentation_polish)
	_audio_director = AUDIO_DIRECTOR.new()
	_audio_director.name = "AudioDirector"
	add_child(_audio_director)

	_camera = Camera3D.new()
	_camera.name = "GameplayCamera"
	_camera.position = Vector3(8, 13, 15)
	add_child(_camera)
	_camera.look_at_from_position(_camera.position, Vector3(4, 0, 0), Vector3.UP)

	var canvas := CanvasLayer.new()
	canvas.name = "HUD"
	add_child(canvas)
	_hud_canvas = canvas
	_hud_frame = OPERATOR_HUD_SCENE.instantiate()
	canvas.add_child(_hud_frame)
	_hud_frame.call("add_panel", Rect2(24, 24, 620, 210), "OPERATOR TELEMETRY", Color("35d0d0"))
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

	_hud_frame.call("add_panel", Rect2(894, 24, 362, 170), "MISSION GUIDE", Color("f5a524"))
	_hud_label(canvas, Vector2(918, 42), 16, Color("f5a524"), "MISSION GUIDE")
	_guidance_label = _hud_label(canvas, Vector2(918, 72), 18, Color.WHITE, "CONNECTING\n\nWait for the relay-hub connection.")
	_guidance_label.size = Vector2(314, 104)
	_guidance_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	_hud_frame.call("add_panel", Rect2(894, 214, 362, 188), "INPUT MONITOR", Color("35d0d0"))
	_hud_label(canvas, Vector2(918, 232), 16, Color("35d0ba"), "INPUT MONITOR")
	_input_log_label = _hud_label(canvas, Vector2(918, 262), 14, Color("a9b8cc"), "Click inside the game window.\nWaiting for keyboard or mouse input...")
	_input_log_label.size = Vector2(314, 126)
	_input_log_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	_hud_frame.call("add_panel", Rect2(894, 422, 362, 176), "INVENTORY", Color("a9b8cc"))
	_inventory_label = _hud_label(canvas, Vector2(918, 440), 16, Color("a9b8cc"), "INVENTORY\nWAITING FOR SERVER")
	_inventory_label.size = Vector2(314, 82)
	_progression_label = _hud_label(canvas, Vector2(918, 516), 15, Color("35d0ba"), "PROGRESSION  •  WAITING FOR SERVER")
	_progression_label.size = Vector2(314, 58)
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
	_create_weapon_button(canvas, "SIDEARM", "arc_sidearm", Vector2(400, 642))
	_hud_canvas.visible = false


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


func _open_settings(focus_source: Control) -> void:
	_observe_first("settings_opened")
	_settings_panel.call("open", _settings, focus_source)


func _on_settings_applied(settings: Dictionary) -> void:
	_apply_settings(settings, true)


func _apply_settings(settings: Dictionary, persist: bool) -> void:
	_settings = _settings_store.call("apply", settings)
	if _playtest_observation != null:
		_playtest_observation.call("update_preferences", _settings)
	_guidance_mode = _settings.get("guidance_mode", "Full")
	if _onboarding != null:
		_onboarding.call("set_mode", _guidance_mode)
		_refresh_onboarding()
	if _presentation_polish != null:
		_presentation_polish.call("set_reduced_flash", _settings.get("reduced_flash", false))
	if _audio_director != null:
		_audio_director.call("configure_routes")
		_audio_director.call("set_silent", _settings.get("muted", false))
	if persist:
		var save_error: Error = _settings_store.call("save_settings", _settings)
		if save_error != OK:
			push_warning("Revenant settings could not be saved: %s" % error_string(save_error))


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
	_observe_first("first_attack_attempt")
	_player_intents.call("request_attack", true)
	_append_input_log("ON-SCREEN ATTACK detected")


func _create_weapon_button(parent: Node, text: String, item_id: String, position: Vector2) -> void:
	var button := Button.new()
	button.text = text
	button.position = position
	button.size = Vector2(108, 40)
	_hud_frame.call("style_button", button, Color("f5a524"), true)
	button.pressed.connect(_request_weapon.bind(item_id))
	parent.add_child(button)
	_weapon_buttons.append(button)


func _request_weapon(item_id: String) -> void:
	var sent := _send_message({"type": "EquipIntent", "item_id": item_id})
	_append_input_log("EquipIntent %s %s" % [item_id, "sent" if sent else "FAILED"])


func _drive_manual_activity() -> void:
	await get_tree().create_timer(0.2).timeout
	_request_weapon("arc_sidearm")
	var equip_deadline := Time.get_ticks_msec() + 2000
	while _authoritative_state.equipped_weapon_item_id != "arc_sidearm" and Time.get_ticks_msec() < equip_deadline:
		await get_tree().process_frame
	if _authoritative_state.equipped_weapon_item_id != "arc_sidearm":
		_driver_fail("sidearm did not equip through the on-screen loadout control")
		return
	for _attack_index in range(4):
		_request_ui_attack()
		await get_tree().create_timer(0.35).timeout
	var deadline := Time.get_ticks_msec() + 3000
	while _current_enemy_id != 0 and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if _current_enemy_id != 0:
		_driver_fail("drone did not die through the on-screen Attack control")
		return

	await get_tree().create_timer(0.2).timeout
	_set_ui_movement(Vector2(1, 0))
	deadline = Time.get_ticks_msec() + 4000
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

	deadline = Time.get_ticks_msec() + 3000
	while _current_enemy_id == 0 and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if _current_enemy_id == 0:
		_driver_fail("Warden did not spawn after manual movement")
		return
	for _attack_index in range(5):
		_request_ui_attack()
		await get_tree().create_timer(0.35).timeout
	deadline = Time.get_ticks_msec() + 3000
	while not _authoritative_state.activity_complete and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if not _authoritative_state.activity_complete:
		_driver_fail("relay_awakening did not complete through manual controls")
		return
	print("M17 manual controls completed relay_awakening without user input")
	_quit_client(0)


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
		_guidance_label.text = "%s\n\n%s" % [title, instructions]


func _refresh_onboarding() -> void:
	if _onboarding == null or _guidance_label == null:
		return
	var guidance: Dictionary = _onboarding.call("guidance")
	if not guidance.get("visible", false):
		_guidance_label.text = "GUIDANCE OFF" if _guidance_mode == "Off" else "GUIDANCE HIDDEN\n\nPress H to review this step."
	else:
		_guidance_label.text = guidance.get("title", "") if guidance.get("compact", false) else "%s\n\n%s" % [guidance.get("title", ""), guidance.get("instructions", "")]


func _append_input_log(message: String) -> void:
	if _input_log_label == null:
		return
	_input_events.push_front("• %s" % message)
	if _input_events.size() > 6:
		_input_events.resize(6)
	_input_log_label.text = "WINDOW FOCUS: %s\n%s" % ["YES" if get_window().has_focus() else "NO", "\n".join(_input_events)]


func _validate_playable_slice() -> void:
	var validation_error := AUDIO_HARNESS.new().validate(_audio_director, _door)
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
		"presentation_polish": _presentation_polish,
		"open_settings": Callable(self, "_open_settings"),
		"apply_settings": Callable(self, "_apply_settings"),
		"refresh_onboarding": Callable(self, "_refresh_onboarding"),
		"save_capture": Callable(self, "_save_review_capture"),
		"guidance_mode": Callable(self, "_validation_guidance_mode"),
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
		"progression_label": _progression_label,
		"equipment_label": _equipment_label,
		"hud_frame": _hud_frame,
		"presentation_polish": _presentation_polish,
		"player_health_bar": _player_health_bar,
		"enemy_health_bar": _enemy_health_bar,
		"movement_buttons": _movement_buttons,
		"attack_button": _attack_button,
		"weapon_buttons": _weapon_buttons,
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
	validation_error = PLAYTEST_OBSERVATION_HARNESS.new().validate()
	if not validation_error.is_empty():
		_validation_fail(validation_error)
		return
	presentation_fixtures.merge({
		"root": self,
		"tree": get_tree(),
		"environment": _environment,
		"combat_vfx": _combat_vfx,
		"save_capture": Callable(self, "_save_review_capture"),
		"onboarding": _onboarding,
		"refresh_onboarding": Callable(self, "_refresh_onboarding"),
	})
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
	_audio_director.call("shutdown")
	_audio_director.queue_free()
	_audio_director = null
	await get_tree().process_frame
	await get_tree().create_timer(0.1).timeout
	get_tree().quit(0)


func _validation_guidance_mode() -> String:
	return _guidance_mode


func _validation_fail(message: String) -> void:
	_fail(message)
	get_tree().quit(1)


func _save_review_capture(path: String) -> Error:
	await get_tree().process_frame
	var capture_image := get_viewport().get_texture().get_image()
	if capture_image == null:
		return ERR_CANT_CREATE
	return capture_image.save_png(path)


func _fail(message: String) -> void:
	_observe_terminal_outcome("failed")
	push_error("Revenant handshake failed: %s" % message)
	_connection_started = false
	if _entry_shell != null and not _should_exit_after_flow():
		_hud_canvas.visible = false
		_entry_shell.call("show_failure", message)
	if _status_label != null:
		_status_label.text = "SESSION UNAVAILABLE  •  RECONNECT REQUIRED"
		_status_label.modulate = Color("ff6b6b")
	if _objective_label != null:
		_objective_label.text = "OBJECTIVE  •  ACTIVITY DID NOT START"
	if _controls_label != null:
		_controls_label.text = "CLOSE THIS WINDOW AND TRY AGAIN AFTER THE CURRENT RUN ENDS"
	if _guidance_label != null:
		_set_guidance("WHAT HAPPENED?", "This gateway already has an active run. Try again after the current players leave.")
	if _should_exit_after_flow():
		_quit_client(1)


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
	return _should_exit_after_flow() or OS.get_environment("REVENANT_VALIDATE_MANUAL_FLOW") == "1"


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


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST and _playtest_observation != null and _playtest_observation.call("is_active"):
		_quit_client(0)


func _observe_first(key: String) -> void:
	if _playtest_observation != null and _playtest_observation.call("is_active"):
		_playtest_observation.call("record_first", key)


func _observe_connection_outcome(outcome: String) -> void:
	if _playtest_observation != null and _playtest_observation.call("is_active"):
		_playtest_observation.call("set_connection_outcome", outcome)


func _observe_connection_failure(message: String) -> void:
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
