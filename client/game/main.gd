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
const M21_CAPTURE_FILENAMES := [
	"01-relay-hub-overview.png",
	"02-enemy-telegraphs.png",
	"03-combat-feedback.png",
]

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


func _ready() -> void:
	_session = SESSION_CONTROLLER.new()
	_session.name = "SessionController"
	_session.connect("connection_state_requested", _set_connection_state)
	add_child(_session)
	_build_playable_scene()
	_build_entry_shell()
	_build_settings()
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
		_fail(joined.get("error", "initial session join failed"))
		return
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


func _handle_manual_input() -> void:
	var now := Time.get_ticks_msec()
	_crosshair.position = get_viewport().get_mouse_position() - (_crosshair.size * 0.5)
	_update_target_highlight()
	var movement: Vector2 = _player_intents.call("movement", now)
	if movement.length() > 0.2:
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
		_onboarding.call("note_local", "attack")
		var target_id := _current_enemy_id if attack.get("use_active_target", false) else _aimed_enemy()
		if target_id != 0:
			var sent := _send_message({"type": "AttackIntent", "target_actor_id": target_id})
			_status_label.text = "ATTACK SENT  •  TARGET %d" % target_id if sent else "ATTACK FAILED  •  CONNECTION ERROR"
			_append_input_log("AttackIntent target %d %s" % [target_id, "sent" if sent else "FAILED"])
			_player_intents.call("consume_attack", now)
			if sent:
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
	_settings_panel.call("open", _settings, focus_source)


func _on_settings_applied(settings: Dictionary) -> void:
	_apply_settings(settings, true)


func _apply_settings(settings: Dictionary, persist: bool) -> void:
	_settings = _settings_store.call("apply", settings)
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
	_player_intents.call("set_ui_movement", direction)
	_append_input_log("ON-SCREEN MOVE %s" % direction)


func _clear_ui_movement() -> void:
	_player_intents.call("clear_ui_movement")


func _request_ui_attack() -> void:
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
	var initial_audio_state: Dictionary = _audio_director.call("presentation_state")
	if (
		initial_audio_state.get("fixed_nodes") != 14
		or initial_audio_state.get("maximum_voices") != 14
		or initial_audio_state.get("foundation_voices") != 4
		or initial_audio_state.get("combat_voices") != 8
		or initial_audio_state.get("interface_critical_voices") != 4
		or initial_audio_state.get("decoded_bytes") != 741120
		or not initial_audio_state.get("ambience_looping", false)
		or initial_audio_state.get("routes") != {"ambience": "Ambience", "door": "Effects", "system": "Interface", "combat": "Effects"}
	):
		_fail("M22 audio foundation is not fixed, bounded or loop-ready: %s" % initial_audio_state)
		get_tree().quit(1)
		return
	_audio_director.call("set_silent", true)
	_audio_director.call("play_system_ready")
	_audio_director.call("apply_door_state", false, Vector3.ZERO)
	_audio_director.call("apply_door_state", true, Vector3.ZERO)
	_audio_director.call("play_cooldown_acknowledgement")
	_audio_director.call("play_confirmed_attack", "pulse_rifle", Vector3.ZERO)
	_audio_director.call("play_enemy_presence", "warden", Vector3.ZERO)
	_audio_director.call("play_player_damage")
	_audio_director.call("play_completion")
	var silent_audio_state: Dictionary = _audio_director.call("presentation_state")
	if (
		silent_audio_state.get("active_voices") != 0
		or silent_audio_state.get("suppressed", 0) < 7
		or silent_audio_state.get("requests", {}).get("door_unlock") != 1
	):
		_fail("M22 silent mode permits audible or queued cues")
		get_tree().quit(1)
		return
	_audio_director.call("set_silent", false)
	_audio_director.call("play_system_ready")
	_audio_director.call("apply_door_state", true, _door.global_position)
	_audio_director.call("play_cooldown_acknowledgement")
	var local_ack_state: Dictionary = _audio_director.call("presentation_state")
	if local_ack_state.get("played", {}).get("cooldown") != 1 or local_ack_state.get("played", {}).get("pulse_rifle") != 0:
		_fail("M22 local attack acknowledgement manufactures a confirmed combat cue")
		get_tree().quit(1)
		return
	_audio_director.call("play_confirmed_move", Vector3.ZERO)
	_audio_director.call("play_confirmed_attack", "pulse_rifle", Vector3.ZERO)
	_audio_director.call("play_confirmed_attack", "arc_sidearm", Vector3.ZERO)
	_audio_director.call("play_enemy_presence", "relay-drone", Vector3.ZERO)
	_audio_director.call("play_enemy_presence", "warden", Vector3.ZERO)
	_audio_director.call("play_confirmed_impact", Vector3.ZERO)
	_audio_director.call("play_confirmed_defeat", Vector3.ZERO)
	_audio_director.call("play_player_damage")
	_audio_director.call("play_completion")
	var audible_audio_state: Dictionary = _audio_director.call("presentation_state")
	if audible_audio_state.get("active_voices", 0) > 14:
		_fail("M22 audio presentation exceeds its voice budget")
		get_tree().quit(1)
		return
	var onboarding_validation := ONBOARDING_CONTROLLER.new()
	onboarding_validation.call("reset", "Full")
	onboarding_validation.call("note_local", "movement")
	onboarding_validation.call("note_local", "attack")
	var onboarding_state: Dictionary = onboarding_validation.call("presentation_state")
	if onboarding_state.get("step") != "Movement":
		_fail("M22 local onboarding attempts manufacture authoritative progress")
		get_tree().quit(1)
		return
	onboarding_validation.call("confirm", "movement")
	onboarding_validation.call("confirm", "damage")
	onboarding_validation.call("confirm", "equipment")
	onboarding_validation.call("confirm", "door_objective")
	onboarding_validation.call("confirm", "warden_spawn")
	onboarding_validation.call("toggle")
	onboarding_validation.call("toggle")
	onboarding_validation.call("confirm", "completion")
	onboarding_state = onboarding_validation.call("presentation_state")
	if onboarding_state.get("step") != "Completion" or onboarding_state.get("dismissed", true):
		_fail("M22 onboarding does not follow confirmed evidence or revisitable guidance")
		get_tree().quit(1)
		return
	var entry_state: Dictionary = _entry_shell.call("presentation_state")
	if (
		entry_state.get("state") != "Entry"
		or not entry_state.get("username_valid", false)
		or not entry_state.get("connect_enabled", false)
		or not entry_state.get("mouse_captured", false)
		or not _entry_shell.call("is_username_valid", "Echo.Runner-1")
		or _entry_shell.call("is_username_valid", "bad user")
		or _entry_shell.call("is_username_valid", "")
	):
		_fail("M22 entry shell does not expose a safe explicit connection state")
		get_tree().quit(1)
		return
	var entry_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_ENTRY")
	if not entry_capture_path.is_empty():
		await get_tree().process_frame
		await get_tree().process_frame
		var entry_capture_error := await _save_review_capture(entry_capture_path)
		if entry_capture_error != OK:
			_fail("M22 entry shell validation capture could not be saved")
			get_tree().quit(1)
			return
	_entry_shell.call("set_connection_state", "Connecting", "Validation")
	entry_state = _entry_shell.call("presentation_state")
	if entry_state.get("state") != "Connecting" or entry_state.get("connect_enabled", true):
		_fail("M22 entry shell does not prevent duplicate connection actions")
		get_tree().quit(1)
		return
	_entry_shell.call("show_failure", "Validation failure")
	entry_state = _entry_shell.call("presentation_state")
	if entry_state.get("state") != "Failed" or not entry_state.get("connect_enabled", false):
		_fail("M22 entry shell does not provide a keyboard-reachable retry state")
		get_tree().quit(1)
		return
	var sanitized_settings: Dictionary = _settings_store.call("sanitize", {
		"master_volume": 4.0,
		"ambience_volume": -2.0,
		"effects_volume": "invalid",
		"interface_volume": 0.35,
		"muted": "invalid",
		"display_mode": "Borderless",
		"reduced_flash": true,
		"guidance_mode": "Unknown",
	})
	if (
		sanitized_settings.get("master_volume") != 1.0
		or sanitized_settings.get("ambience_volume") != 0.0
		or sanitized_settings.get("effects_volume") != 0.85
		or sanitized_settings.get("interface_volume") != 0.35
		or sanitized_settings.get("muted") != false
		or sanitized_settings.get("display_mode") != "Windowed"
		or sanitized_settings.get("reduced_flash") != true
		or sanitized_settings.get("guidance_mode") != "Full"
	):
		_fail("M22 settings do not recover deterministic safe defaults")
		get_tree().quit(1)
		return
	var validation_settings_path := "user://m22-settings-validation.cfg"
	var persisted_candidate := sanitized_settings.duplicate(true)
	persisted_candidate["guidance_mode"] = "Off"
	if _settings_store.call("save_settings", persisted_candidate, validation_settings_path) != OK:
		_fail("M22 settings cannot persist local validated values")
		get_tree().quit(1)
		return
	var persisted_settings: Dictionary = _settings_store.call("load_settings", validation_settings_path)
	DirAccess.remove_absolute(ProjectSettings.globalize_path(validation_settings_path))
	if persisted_settings.get("guidance_mode") != "Off" or persisted_settings.get("master_volume") != 1.0:
		_fail("M22 settings persistence does not round-trip validated values")
		get_tree().quit(1)
		return
	var settings_focus_source: Control = _entry_shell.call("settings_focus_source")
	_open_settings(settings_focus_source)
	var settings_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_SETTINGS")
	if not settings_capture_path.is_empty():
		await get_tree().process_frame
		await get_tree().process_frame
		var settings_capture_error := await _save_review_capture(settings_capture_path)
		if settings_capture_error != OK:
			_fail("M22 settings validation capture could not be saved")
			get_tree().quit(1)
			return
	var settings_state: Dictionary = _settings_panel.call("presentation_state")
	if (
		not settings_state.get("visible", false)
		or settings_state.get("slider_count", 0) != 4
		or not settings_state.get("has_mute", false)
		or not settings_state.get("has_reduced_flash", false)
		or settings_state.get("display_options", 0) != 2
		or settings_state.get("guidance_options", 0) != 3
		or not settings_state.get("mouse_captured", false)
		or settings_state.get("focus_owner") != "Apply"
	):
		_fail("M22 settings panel is incomplete or not keyboard reachable")
		get_tree().quit(1)
		return
	_settings_panel.call("close_panel")
	entry_state = _entry_shell.call("presentation_state")
	if entry_state.get("focus_owner") != "Settings":
		_fail("M22 settings do not restore focus to their invoking control")
		get_tree().quit(1)
		return
	_entry_shell.call("dismiss")
	_hud_canvas.visible = true
	_status_label.text = "RELAY READY  •  WAITING FOR ACTIVITY"
	_controls_label.text = "WASD / ARROWS MOVE  •  AIM + CLICK / SPACE ATTACK  •  H HELP  •  ESC SETTINGS"
	_refresh_onboarding()
	var onboarding_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_ONBOARDING")
	if not onboarding_capture_path.is_empty():
		var onboarding_capture_error := await _save_review_capture(onboarding_capture_path)
		if onboarding_capture_error != OK:
			_fail("M22 onboarding validation capture could not be saved")
			get_tree().quit(1)
			return
	_apply_settings({
		"master_volume": 0.6,
		"ambience_volume": 0.5,
		"effects_volume": 0.7,
		"interface_volume": 0.4,
		"muted": true,
		"display_mode": "Windowed",
		"reduced_flash": true,
		"guidance_mode": "Compact",
	}, false)
	var audio_state: Dictionary = _settings_store.call("audio_state")
	var buses: Dictionary = audio_state.get("buses", {})
	if (
		not buses.get("Master", {}).get("present", false)
		or not buses.get("Master", {}).get("muted", false)
		or not buses.get("Ambience", {}).get("present", false)
		or not buses.get("Effects", {}).get("present", false)
		or not buses.get("Interface", {}).get("present", false)
		or _guidance_mode != "Compact"
		or not _presentation_polish.call("presentation_state").get("reduced_flash", false)
	):
		_fail("M22 local settings do not apply bounded buses and accessibility state")
		get_tree().quit(1)
		return
	_apply_settings(_settings_store.call("defaults"), false)
	var required_actions := ["move_forward", "move_back", "move_left", "move_right", "attack"]
	for action in required_actions:
		if not InputMap.has_action(action):
			_fail("M17 input action is missing: %s" % action)
			get_tree().quit(1)
			return
	if (
		_camera == null
		or _door == null
		or _status_label == null
		or _health_label == null
		or _enemy_health_label == null
		or _position_label == null
		or _objective_label == null
		or _crosshair == null
		or _guidance_label == null
		or _input_log_label == null
		or _inventory_label == null
		or _progression_label == null
		or _equipment_label == null
		or _hud_frame == null
		or _presentation_polish == null
		or _player_health_bar == null
		or _enemy_health_bar == null
		or _movement_buttons.size() != 4
		or _attack_button == null
		or _weapon_buttons.size() != 2
		or _crosshair.mouse_filter != Control.MOUSE_FILTER_IGNORE
	):
		_fail("M17 playable scene composition is incomplete")
		get_tree().quit(1)
		return
	_presentation_polish.call("play_confirmed_player_damage")
	_presentation_polish.call("play_authoritative_completion")
	var polish_state: Dictionary = _presentation_polish.call("presentation_state")
	if (
		polish_state.get("confirmed_damage_cues", 0) != 1
		or polish_state.get("completion_cues", 0) != 1
		or polish_state.get("edge_overlays", 0) != 4
		or polish_state.get("maximum_screen_coverage", 1.0) > 0.06
	):
		_fail("M21 polished slice exceeds its visual consistency or performance budget")
		get_tree().quit(1)
		return
	var hud_state: Dictionary = _hud_frame.call("presentation_state")
	if (
		hud_state.get("panel_count", 0) != 4
		or hud_state.get("semantic_accent_count", 0) < 4
		or not hud_state.get("mouse_passthrough", false)
		or _player_health_bar.value != 100.0
		or _enemy_health_bar.value != 100.0
	):
		_fail("M21 Operator HUD does not preserve its hierarchy, semantic roles or authoritative health state")
		get_tree().quit(1)
		return
	var codec_validation: RefCounted = MESSAGEPACK_CODEC.new()
	if codec_validation.call("encode_array", [-12, 0, 12]) != PackedByteArray([0x93, 0xf4, 0x00, 0x0c]):
		_fail("M17 movement encoder does not support signed relay-hub coordinates")
		get_tree().quit(1)
		return
	var codec_fixture := {"type": "MoveIntent", "position": [6, 0, 0]}
	var codec_payload: PackedByteArray = codec_validation.call("encode_map", codec_fixture)
	var codec_decoded: Array = codec_validation.call("decode_value", codec_payload)
	if (
		codec_validation.call("has_failed")
		or codec_decoded.is_empty()
		or codec_decoded[0] != codec_fixture
		or codec_decoded[1] != codec_payload.size()
	):
		_fail("M23 extracted MessagePack codec does not preserve the supported wire subset")
		get_tree().quit(1)
		return
	var session_state: Dictionary = _session.call("presentation_state")
	var transport_state: Dictionary = session_state.get("transport", {})
	if (
		_session.name != "SessionController"
		or session_state.get("protocol_version") != 2
		or session_state.get("message_deadline_ms") != 5000
		or transport_state.get("maximum_frame_size") != 64 * 1024
		or transport_state.get("buffered_bytes") != 0
	):
		_fail("M23 session controller does not preserve its protocol and bounded transport contract")
		get_tree().quit(1)
		return
	var projection_validation := AUTHORITATIVE_STATE.new()
	projection_validation.call("join_world", {"player_actor_id": 7})
	projection_validation.call("apply_actor_spawn", {"actor_id": 7, "health": 100, "max_health": 100, "position": [0, 0, 0]})
	projection_validation.call("apply_damage", {"target_actor_id": 7, "remaining_health": 90})
	projection_validation.call("apply_inventory_snapshot", {"items": [{"item_id": "pulse_rifle", "quantity": 1}]})
	projection_validation.call("apply_loot_grant", {"item_id": "relay_core_fragment", "resulting_quantity": 2})
	projection_validation.call("apply_progression", {"level": 7, "experience": 3200, "experience_to_next_level": 800})
	projection_validation.call("apply_equipment_snapshot", {"equipped_weapon_item_id": "pulse_rifle", "weapons": [{"item_id": "pulse_rifle", "damage": 25}]})
	projection_validation.call("apply_objective", {"objective_id": "clear_drone_group", "state": "Active"})
	projection_validation.call("apply_activity_complete", {"activity_id": "relay_awakening"})
	var projection_state: Dictionary = projection_validation.call("presentation_state")
	if (
		projection_state.get("player_actor_id") != 7
		or projection_state.get("actor_count") != 1
		or projection_validation.actor_health.get(7) != 90
		or projection_state.get("inventory", {}).get("relay_core_fragment") != 2
		or projection_state.get("progression", {}).get("experience") != 3200
		or projection_state.get("equipped_weapon_item_id") != "pulse_rifle"
		or projection_state.get("weapon_profile_count") != 1
		or projection_state.get("objective_count") != 1
		or not projection_state.get("activity_complete", false)
	):
		_fail("M23 authoritative state projection does not preserve server-confirmed domain facts")
		get_tree().quit(1)
		return
	var intent_validation := PLAYER_INTENT_CONTROLLER.new()
	intent_validation.call("set_ui_movement", Vector2.RIGHT)
	intent_validation.call("request_attack", true)
	var intent_state: Dictionary = intent_validation.call("presentation_state")
	var attack_intent: Dictionary = intent_validation.call("take_attack", 1000)
	var hud_validation := HUD_PROJECTION.new()
	if (
		intent_validation.call("movement", 1000) != Vector2.RIGHT
		or intent_state.get("move_cooldown_ms") != 120
		or intent_state.get("attack_cooldown_ms") != 260
		or not attack_intent.get("requested", false)
		or not attack_intent.get("use_active_target", false)
		or hud_validation.call("inventory_text", {"relay_core_fragment": 2}) != "INVENTORY\nRELAY CORE FRAGMENT  x2"
		or hud_validation.call("equipment_text", "pulse_rifle", {"damage": 25, "range": 8, "cooldown_ms": 260}) != "WEAPON  •  PULSE RIFLE\nDMG 25  RANGE 8  COOLDOWN 260 MS"
	):
		_fail("M23 input and HUD coordination does not preserve intent-only controls and authoritative text")
		get_tree().quit(1)
		return
	var environment_state: Dictionary = _environment.call("presentation_state")
	var closed_door_state: Dictionary = environment_state.get("door", {})
	if (
		environment_state.get("visual_bounds") != 12.0
		or environment_state.get("door_position") != Vector3(6.5, 0.0, 0.0)
		or not environment_state.get("terminal_present", false)
		or environment_state.get("terminal_interactive", true)
		or not environment_state.get("damaged_section_present", false)
		or environment_state.get("mesh_count", 0) > 60
		or environment_state.get("material_count", 0) > 6
		or environment_state.get("light_count", 0) != 5
		or environment_state.get("shadow_light_count", 1) != 0
		or closed_door_state.get("open", true)
	):
		_fail("M21 relay-hub modular environment exceeds its composition or performance contract")
		get_tree().quit(1)
		return
	_environment.call("set_core_door_open", true, false)
	var open_door_state: Dictionary = _environment.call("presentation_state").get("door", {})
	if not open_door_state.get("open", false) or open_door_state.get("amber_visible", true):
		_fail("M21 relay-core door does not expose its authoritative open state")
		get_tree().quit(1)
		return
	_environment.call("set_core_door_open", false, false)
	var operator: Node3D = OPERATOR_SCENE.instantiate()
	operator.name = "OperatorValidation"
	add_child(operator)
	await get_tree().process_frame
	var operator_state: Dictionary = operator.call("presentation_state")
	if (
		operator_state.get("equipped_weapon") != "pulse_rifle"
		or operator_state.get("last_animation") != "idle"
		or not operator_state.get("pulse_rifle_visible", false)
		or operator_state.get("arc_sidearm_visible", true)
		or operator_state.get("part_count", 0) < 15
	):
		_fail("M21 Operator initial presentation is incomplete")
		get_tree().quit(1)
		return
	operator.call("set_weapon", "arc_sidearm")
	operator_state = operator.call("presentation_state")
	if operator_state.get("equipped_weapon") != "arc_sidearm" or not operator_state.get("arc_sidearm_visible", false):
		_fail("M21 Operator weapon silhouettes do not follow authoritative equipment")
		get_tree().quit(1)
		return
	var drone: Node3D = RELAY_DRONE_SCENE.instantiate()
	drone.name = "RelayDroneValidation"
	drone.position = Vector3(-2.0, 0.0, 2.5)
	add_child(drone)
	var warden: Node3D = WARDEN_SCENE.instantiate()
	warden.name = "WardenValidation"
	warden.position = Vector3(3.0, 0.0, 2.5)
	add_child(warden)
	await get_tree().process_frame
	var drone_state: Dictionary = drone.call("presentation_state")
	var warden_state: Dictionary = warden.call("presentation_state")
	if (
		drone_state.get("family") != "relay_drone"
		or drone_state.get("mesh_count", 0) > 12
		or drone_state.get("material_count", 0) > 3
		or warden_state.get("family") != "warden"
		or warden_state.get("mesh_count", 0) > 18
		or warden_state.get("material_count", 0) > 4
		or warden_state.get("mesh_count", 0) <= drone_state.get("mesh_count", 0)
	):
		_fail("M21 enemy families are not distinct or exceed their presentation budgets")
		get_tree().quit(1)
		return
	var capture_directory := OS.get_environment("REVENANT_CAPTURE_M21_DIR")
	if not capture_directory.is_empty():
		var overview_error := await _save_review_capture(capture_directory.path_join(M21_CAPTURE_FILENAMES[0]))
		if overview_error != OK:
			_fail("M21 overview capture could not be saved")
			get_tree().quit(1)
			return
	drone.call("set_targeted", true)
	drone.call("set_danger_close", false)
	drone_state = drone.call("presentation_state")
	if not drone_state.get("targeted", false) or drone_state.get("danger_close", true):
		_fail("M21 enemy target and danger telegraphs are not independent")
		get_tree().quit(1)
		return
	drone.call("set_danger_close", true)
	if not capture_directory.is_empty():
		var telegraph_error := await _save_review_capture(capture_directory.path_join(M21_CAPTURE_FILENAMES[1]))
		if telegraph_error != OK:
			_fail("M21 telegraph capture could not be saved")
			get_tree().quit(1)
			return
	operator.call("play_authoritative_move", Vector3(-1.0, 0.0, 0.0))
	operator.call("play_confirmed_attack")
	operator.call("play_confirmed_hit")
	operator.call("play_defeat")
	operator_state = operator.call("presentation_state")
	if operator_state.get("last_animation") != "defeat" or not operator_state.get("defeated", false):
		_fail("M21 Operator authoritative animation states are incomplete")
		get_tree().quit(1)
		return
	drone.call("play_authoritative_move", Vector3(-1.0, 0.0, -1.0))
	drone.call("play_confirmed_attack")
	drone.call("play_confirmed_hit")
	drone.call("retire")
	warden.call("play_confirmed_attack")
	warden.call("play_confirmed_hit")
	warden.call("retire")
	_combat_vfx.call("play_local_cooldown", operator.global_position, 260)
	_combat_vfx.call("play_confirmed_exchange", operator, drone, false)
	_combat_vfx.call("play_confirmed_exchange", warden, operator, true)
	var vfx_state: Dictionary = _combat_vfx.call("presentation_state")
	if (
		vfx_state.get("confirmed_exchanges", 0) != 2
		or vfx_state.get("cooldown_cues", 0) != 1
		or vfx_state.get("active_effects", 0) <= 0
		or vfx_state.get("active_effects", 0) > vfx_state.get("maximum_active_effects", 0)
		or vfx_state.get("permanent_particles", 1) != 0
	):
		_fail("M21 combat VFX does not preserve its bounded authoritative feedback contract")
		get_tree().quit(1)
		return
	drone_state = drone.call("presentation_state")
	warden_state = warden.call("presentation_state")
	if not drone_state.get("retired", false) or not warden_state.get("retired", false):
		_fail("M21 enemy defeat does not retire confirmed targets")
		get_tree().quit(1)
		return
	var scene_budget: Dictionary = _presentation_polish.call("scene_budget", self)
	if (
		scene_budget.get("meshes", 0) > 120
		or scene_budget.get("materials", 0) > 32
		or scene_budget.get("lights", 0) != 5
		or scene_budget.get("shadow_lights", 1) != 0
		or scene_budget.get("particles", 1) != 0
		or scene_budget.get("audio_nodes", 0) != 14
	):
		_fail("M21 representative combat peak exceeds its whole-scene performance budget")
		get_tree().quit(1)
		return
	print("M21 scene budget measured: %d meshes, %d materials, %d lights, %d particles, %d audio nodes" % [
		scene_budget.get("meshes", 0),
		scene_budget.get("materials", 0),
		scene_budget.get("lights", 0),
		scene_budget.get("particles", 0),
		scene_budget.get("audio_nodes", 0),
	])
	if not capture_directory.is_empty():
		var combat_error := await _save_review_capture(capture_directory.path_join(M21_CAPTURE_FILENAMES[2]))
		if combat_error != OK:
			_fail("M21 combat capture could not be saved")
			get_tree().quit(1)
			return
	var runtime_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_RUNTIME")
	if not runtime_capture_path.is_empty():
		_status_label.text = "ENGAGED  •  CONFIRMED COMBAT FEEDBACK"
		_objective_label.text = "OBJECTIVE  •  DEFEAT THE WARDEN"
		_enemy_health_label.text = "ENEMY  WARDEN  •  020 / 120 HP"
		_enemy_health_bar.max_value = 120
		_enemy_health_bar.value = 20
		_onboarding.call("confirm", "warden_spawn")
		_refresh_onboarding()
		var runtime_capture_error := await _save_review_capture(runtime_capture_path)
		if runtime_capture_error != OK:
			_fail("M22 runtime validation capture could not be saved")
			get_tree().quit(1)
			return
	var capture_path := OS.get_environment("REVENANT_CAPTURE_SLICE")
	if not capture_path.is_empty():
		await get_tree().process_frame
		await get_tree().process_frame
		var capture_image := get_viewport().get_texture().get_image()
		if capture_image == null:
			_fail("M21 visual validation capture requires a graphical renderer")
			get_tree().quit(1)
			return
		var capture_error := capture_image.save_png(capture_path)
		if capture_error != OK:
			_fail("M21 visual validation capture could not be saved")
			get_tree().quit(1)
			return
	if OS.get_environment("REVENANT_MEASURE_M22") == "1":
		var measurement := await _measure_m22_presentation(operator, drone, warden)
		if (
			measurement.get("captured_frames", 0) <= 0
			or measurement.get("peak_dbfs", 0.0) > -3.0
			or measurement.get("peak_voices", 0) > 14
		):
			_fail("M22 graphical presentation exceeds its measured output or voice budget: %s" % measurement)
			get_tree().quit(1)
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
	_audio_director.call("shutdown")
	_audio_director.queue_free()
	_audio_director = null
	await get_tree().process_frame
	await get_tree().create_timer(0.1).timeout
	get_tree().quit(0)


func _measure_m22_presentation(operator: Node3D, drone: Node3D, warden: Node3D) -> Dictionary:
	var master_bus := AudioServer.get_bus_index("Master")
	var effect_index := AudioServer.get_bus_effect_count(master_bus)
	var capture := AudioEffectCapture.new()
	AudioServer.add_bus_effect(master_bus, capture, effect_index)
	await get_tree().process_frame
	capture.clear_buffer()
	_audio_director.call("play_confirmed_move", operator.global_position)
	_audio_director.call("play_confirmed_attack", "pulse_rifle", operator.global_position)
	_audio_director.call("play_enemy_presence", "relay-drone", drone.global_position)
	_audio_director.call("play_enemy_presence", "warden", warden.global_position)
	_audio_director.call("play_confirmed_impact", drone.global_position)
	_audio_director.call("play_player_damage")
	_audio_director.call("play_completion")
	var frame_times_ms: Array[float] = []
	var peak_voices := 0
	for _frame in range(30):
		var started_at := Time.get_ticks_usec()
		await get_tree().process_frame
		frame_times_ms.append(float(Time.get_ticks_usec() - started_at) / 1000.0)
		peak_voices = maxi(peak_voices, int(_audio_director.call("presentation_state").get("active_voices", 0)))
	var captured_frames := capture.get_frames_available()
	var samples := capture.get_buffer(captured_frames)
	var peak := 0.0
	var square_sum := 0.0
	for sample in samples:
		peak = maxf(peak, maxf(absf(sample.x), absf(sample.y)))
		square_sum += sample.x * sample.x + sample.y * sample.y
	AudioServer.remove_bus_effect(master_bus, effect_index)
	var frame_sum := 0.0
	var frame_max := 0.0
	for frame_time in frame_times_ms:
		frame_sum += frame_time
		frame_max = maxf(frame_max, frame_time)
	var rms := sqrt(square_sum / float(samples.size() * 2)) if not samples.is_empty() else 0.0
	var mean_frame_ms := frame_sum / float(frame_times_ms.size())
	var peak_dbfs := linear_to_db(peak) if peak > 0.0 else -80.0
	var rms_dbfs := linear_to_db(rms) if rms > 0.0 else -80.0
	print("M22 graphical presentation measured: mean %.3f ms/frame, max %.3f ms/frame, mixed peak %.2f dBFS, RMS %.2f dBFS, %d captured stereo frames, %d peak voices" % [mean_frame_ms, frame_max, peak_dbfs, rms_dbfs, captured_frames, peak_voices])
	return {
		"mean_frame_ms": mean_frame_ms,
		"max_frame_ms": frame_max,
		"peak_dbfs": peak_dbfs,
		"rms_dbfs": rms_dbfs,
		"captured_frames": captured_frames,
		"peak_voices": peak_voices,
	}


func _save_review_capture(path: String) -> Error:
	await get_tree().process_frame
	var capture_image := get_viewport().get_texture().get_image()
	if capture_image == null:
		return ERR_CANT_CREATE
	return capture_image.save_png(path)


func _fail(message: String) -> void:
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
