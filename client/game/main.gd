extends Node

const PROTOCOL_VERSION := 2
const MAX_FRAME_SIZE := 64 * 1024
const DEFAULT_HOST := "127.0.0.1"
const DEFAULT_PORT := 7000

var _peer := StreamPeerTCP.new()
var _receive_buffer := PackedByteArray()
var _actors := {}
var _actor_health := {}
var _actor_max_health := {}
var _player_actor_id := 0
var _current_enemy_id := 0
var _player_health := 100
var _next_move_at := 0
var _next_attack_at := 0
var _activity_complete := false
var _inventory := {}
var _progression := {"level": 1, "experience": 0, "experience_to_next_level": 500}
var _equipped_weapon_item_id := "pulse_rifle"
var _weapon_profiles := {}
var _camera: Camera3D
var _door: MeshInstance3D
var _status_label: Label
var _health_label: Label
var _enemy_health_label: Label
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
var _attack_requested := false
var _ui_attack_requested := false
var _encode_failed := false
var _ui_movement := Vector2.ZERO
var _movement_buttons: Array[Button] = []
var _attack_button: Button
var _weapon_buttons: Array[Button] = []


func _ready() -> void:
	_build_playable_scene()
	if OS.get_environment("REVENANT_VALIDATE_SLICE") == "1":
		call_deferred("_validate_playable_slice")
		return
	call_deferred("_run_handshake")


func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		if event.physical_keycode in [KEY_W, KEY_A, KEY_S, KEY_D, KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT]:
			_append_input_log("KEY %s detected" % event.as_text_physical_keycode())
		if event.is_action_pressed("attack"):
			_attack_requested = true
			_append_input_log("SPACE attack detected")
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		_attack_requested = true
		_append_input_log("LEFT CLICK detected")


func _run_handshake() -> void:
	var host := OS.get_environment("REVENANT_GAME_HOST")
	if host.is_empty():
		host = DEFAULT_HOST
	var port_text := OS.get_environment("REVENANT_GAME_PORT")
	var port := DEFAULT_PORT if port_text.is_empty() else int(port_text)

	var connect_error := _peer.connect_to_host(host, port)
	if connect_error != OK:
		_fail("connection could not start: %s" % error_string(connect_error))
		return

	var deadline := Time.get_ticks_msec() + 5000
	while _peer.get_status() == StreamPeerTCP.STATUS_CONNECTING and Time.get_ticks_msec() < deadline:
		_peer.poll()
		await get_tree().process_frame

	if _peer.get_status() != StreamPeerTCP.STATUS_CONNECTED:
		_fail("connection to %s:%d timed out" % [host, port])
		return

	var hello := {
		"type": "ClientHello",
		"protocol_version": PROTOCOL_VERSION,
		"client_name": "revenant-godot",
		"client_build": "0.2.0",
	}
	if not _send_message(hello):
		_fail("ClientHello send failed")
		return

	var server_hello := await _receive_message(Time.get_ticks_msec() + 5000)
	if server_hello.get("type") != "ServerHello":
		_fail("expected ServerHello")
		return
	if not server_hello.get("accepted", false):
		_fail("server rejected handshake: %s" % server_hello.get("message", "unknown error"))
		return
	if server_hello.get("protocol_version") != PROTOCOL_VERSION:
		_fail("server selected an unexpected protocol version")
		return

	print("handshake accepted by %s using protocol v%d" % [server_hello.get("server_name"), PROTOCOL_VERSION])

	if not _send_message({"type": "AuthRequest", "username": "revenant-godot"}):
		_fail("AuthRequest send failed")
		return
	var auth_response := await _receive_message(Time.get_ticks_msec() + 5000)
	if auth_response.get("type") != "AuthResponse":
		_fail("expected AuthResponse")
		return
	if not auth_response.get("authenticated", false):
		_fail("local authentication rejected: %s" % auth_response.get("message", "unknown error"))
		return
	print("authenticated local account %s" % auth_response.get("account_id"))

	if not _send_message({"type": "CharacterListRequest"}):
		_fail("CharacterListRequest send failed")
		return
	var character_response := await _receive_message(Time.get_ticks_msec() + 5000)
	if character_response.get("type") != "CharacterListResponse":
		_fail("expected CharacterListResponse")
		return
	var characters: Array = character_response.get("characters", [])
	if characters.is_empty():
		_fail("server returned no local character")
		return
	var character: Dictionary = characters[0]
	print("received character %s (%s, level %d)" % [character.get("display_name"), character.get("class_name"), character.get("level")])

	if not _send_message({"type": "WorldJoinRequest", "character_id": character.get("character_id")}):
		_fail("WorldJoinRequest send failed")
		return
	var world_response := await _receive_message(Time.get_ticks_msec() + 5000)
	if world_response.get("type") != "WorldJoinResponse":
		_fail("expected WorldJoinResponse")
		return
	if not world_response.get("accepted", false):
		_fail("world join rejected: %s" % world_response.get("message", "unknown error"))
		return
	print("joined world %s as player actor %d" % [world_response.get("world_id"), world_response.get("player_actor_id")])
	_player_actor_id = world_response.get("player_actor_id")
	var inventory_snapshot := await _receive_message(Time.get_ticks_msec() + 5000)
	if inventory_snapshot.get("type") != "InventorySnapshot":
		_fail("expected InventorySnapshot")
		return
	_apply_inventory_snapshot(inventory_snapshot)
	var progression_snapshot := await _receive_message(Time.get_ticks_msec() + 5000)
	if progression_snapshot.get("type") != "ProgressionSnapshot":
		_fail("expected ProgressionSnapshot")
		return
	_apply_progression(progression_snapshot)
	var equipment_snapshot := await _receive_message(Time.get_ticks_msec() + 5000)
	if equipment_snapshot.get("type") != "EquipmentSnapshot":
		_fail("expected EquipmentSnapshot")
		return
	_apply_equipment_snapshot(equipment_snapshot)
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
	_apply_loot_grant(loot)
	_apply_progression(progression_grant)
	print("activity %s completed" % activity_complete.get("activity_id"))
	if _should_exit_after_flow():
		get_tree().quit(0)


func _send_message(value: Dictionary) -> bool:
	_encode_failed = false
	var payload := _encode_map(value)
	if _encode_failed or payload.is_empty():
		_append_input_log("Protocol encode FAILED: %s" % value.get("type", "unknown"))
		return false
	var frame := PackedByteArray([
		(payload.size() >> 24) & 0xff,
		(payload.size() >> 16) & 0xff,
		(payload.size() >> 8) & 0xff,
		payload.size() & 0xff,
	])
	frame.append_array(payload)
	return _peer.put_data(frame) == OK


func _receive_message(deadline: int) -> Dictionary:
	while _receive_buffer.size() < 4 and Time.get_ticks_msec() < deadline:
		_peer.poll()
		_receive_buffer.append_array(_read_available())
		await get_tree().process_frame
	if _receive_buffer.size() < 4:
		return {}

	var payload_size := (_receive_buffer[0] << 24) | (_receive_buffer[1] << 16) | (_receive_buffer[2] << 8) | _receive_buffer[3]
	if payload_size > MAX_FRAME_SIZE:
		return {}
	while _receive_buffer.size() < payload_size + 4 and Time.get_ticks_msec() < deadline:
		_peer.poll()
		_receive_buffer.append_array(_read_available())
		await get_tree().process_frame
	if _receive_buffer.size() < payload_size + 4:
		return {}

	var decoded := _decode_value(_receive_buffer.slice(4, payload_size + 4), 0)
	_receive_buffer = _receive_buffer.slice(payload_size + 4)
	if decoded.is_empty() or not (decoded[0] is Dictionary):
		return {}
	return decoded[0]


func _read_available() -> PackedByteArray:
	if _peer.get_status() != StreamPeerTCP.STATUS_CONNECTED:
		return PackedByteArray()
	var available := _peer.get_available_bytes()
	if available <= 0:
		return PackedByteArray()
	var result := _peer.get_data(available)
	if result[0] != OK:
		return PackedByteArray()
	return result[1]


func _render_actor(actor: Dictionary) -> void:
	var instance := MeshInstance3D.new()
	instance.name = "Actor_%s" % actor.get("actor_id")
	if actor.get("actor_kind") == "enemy":
		instance.mesh = BoxMesh.new()
		instance.material_override = _material(Color("e8505b") if actor.get("archetype") != "warden" else Color("a855f7"))
	else:
		instance.mesh = CapsuleMesh.new()
		instance.material_override = _material(Color("35d0ba"))
	var position: Array = actor.get("position", [0, 0, 0])
	instance.position = Vector3(position[0], position[1], position[2])
	add_child(instance)
	_actors[actor.get("actor_id")] = instance
	_actor_health[actor.get("actor_id")] = actor.get("health", 100)
	_actor_max_health[actor.get("actor_id")] = actor.get("max_health", 100)
	if actor.get("actor_kind") == "enemy" and _enemy_health_label != null:
		_enemy_health_label.text = "ENEMY  %s  •  %03d / %03d HP" % [str(actor.get("archetype")).to_upper(), actor.get("health"), actor.get("max_health")]
	print("rendered %s actor %d (%s)" % [actor.get("actor_kind"), actor.get("actor_id"), actor.get("archetype")])


func _destroy_actor(actor_id: int) -> void:
	var actor: Node = _actors.get(actor_id)
	if actor != null:
		actor.queue_free()
		_actors.erase(actor_id)
	_actor_health.erase(actor_id)
	_actor_max_health.erase(actor_id)
	if actor_id == _current_enemy_id and _enemy_health_label != null:
		_enemy_health_label.text = "ENEMY  •  DEFEATED"
	print("enemy actor %d destroyed" % actor_id)


func _update_actor(update: Dictionary) -> void:
	var actor: Node3D = _actors.get(update.get("actor_id"))
	if actor == null:
		return
	var position: Array = update.get("position", [0, 0, 0])
	actor.position = Vector3(position[0], position[1], position[2])
	if update.get("actor_id") == _player_actor_id and _position_label != null:
		var door_distance := maxi(0, 6 - int(position[0]))
		_position_label.text = "POSITION  [%d, %d]  •  DOOR %d STEPS" % [position[0], position[2], door_distance]
	print("actor %d moved to %s" % [update.get("actor_id"), position])


func _run_manual_activity() -> void:
	_controls_label.text = "WASD / ARROWS  MOVE    •    MOUSE / SPACE  ATTACK"
	while not _activity_complete and _peer.get_status() == StreamPeerTCP.STATUS_CONNECTED:
		_handle_manual_input()
		var message := _try_receive_message()
		if not message.is_empty():
			_handle_manual_message(message)
		await get_tree().process_frame


func _handle_manual_input() -> void:
	var now := Time.get_ticks_msec()
	_crosshair.position = get_viewport().get_mouse_position() - (_crosshair.size * 0.5)
	var movement := _ui_movement if _ui_movement != Vector2.ZERO else Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	if movement.length() > 0.2 and now >= _next_move_at:
		var player: Node3D = _actors.get(_player_actor_id)
		if player != null:
			var step := Vector3(roundi(movement.x), 0, roundi(movement.y))
			var target := player.position + step
			target.x = clampi(roundi(target.x), -12, 12)
			target.z = clampi(roundi(target.z), -12, 12)
			var position := [roundi(target.x), 0, roundi(target.z)]
			var sent := _send_message({"type": "MoveIntent", "position": position})
			_append_input_log("MoveIntent %s %s" % [position, "sent" if sent else "FAILED"])
			_next_move_at = now + 120
	if (_attack_requested or Input.is_action_just_pressed("attack")) and now >= _next_attack_at:
		var use_active_target := _ui_attack_requested
		_attack_requested = false
		_ui_attack_requested = false
		var target_id := _current_enemy_id if use_active_target else _aimed_enemy()
		if target_id != 0:
			var sent := _send_message({"type": "AttackIntent", "target_actor_id": target_id})
			_status_label.text = "ATTACK SENT  •  TARGET %d" % target_id if sent else "ATTACK FAILED  •  CONNECTION ERROR"
			_append_input_log("AttackIntent target %d %s" % [target_id, "sent" if sent else "FAILED"])
			_next_attack_at = now + 260
		else:
			_append_input_log("Attack blocked: aim at active enemy")
	elif _attack_requested:
		_status_label.text = "WEAPON COOLING  •  %d MS" % (_next_attack_at - now)


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
				_status_label.text = "BOSS ONLINE  •  WARDEN"
				_set_guidance("STEP 3  •  DEFEAT THE WARDEN", "Aim at the purple Warden and attack until its HP reaches zero.")
		"ActivityComplete":
			_activity_complete = true
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
		_:
			_fail("unexpected manual gameplay message: %s" % message.get("type"))


func _try_receive_message() -> Dictionary:
	_peer.poll()
	_receive_buffer.append_array(_read_available())
	if _receive_buffer.size() < 4:
		return {}
	var payload_size := (_receive_buffer[0] << 24) | (_receive_buffer[1] << 16) | (_receive_buffer[2] << 8) | _receive_buffer[3]
	if payload_size > MAX_FRAME_SIZE or _receive_buffer.size() < payload_size + 4:
		return {}
	var decoded := _decode_value(_receive_buffer.slice(4, payload_size + 4), 0)
	_receive_buffer = _receive_buffer.slice(payload_size + 4)
	if decoded.is_empty() or not (decoded[0] is Dictionary):
		return {}
	return decoded[0]


func _aimed_enemy() -> int:
	if _current_enemy_id == 0:
		return 0
	var enemy: Node3D = _actors.get(_current_enemy_id)
	if enemy == null or _camera.is_position_behind(enemy.global_position):
		return 0
	var cursor := get_viewport().get_mouse_position()
	var enemy_screen := _camera.unproject_position(enemy.global_position)
	if cursor.distance_to(enemy_screen) <= 110.0 or Input.is_key_pressed(KEY_SPACE):
		return _current_enemy_id
	_status_label.text = "AIM AT THE HIGHLIGHTED TARGET"
	return 0


func _handle_damage_feedback(message: Dictionary) -> void:
	var target_id: int = message.get("target_actor_id")
	var remaining: int = message.get("remaining_health")
	_actor_health[target_id] = remaining
	if target_id == _player_actor_id:
		_player_health = remaining
		_health_label.text = "HP  %03d / 100  •  STABLE" % _player_health
		_health_label.modulate = Color("ff6b6b")
		var health_tween := create_tween()
		health_tween.tween_property(_health_label, "modulate", Color.WHITE, 0.3)
	else:
		_status_label.text = "HIT %d  •  %d HP REMAINS" % [message.get("damage"), remaining]
		if _enemy_health_label != null:
			var maximum: int = _actor_max_health.get(target_id, remaining)
			_enemy_health_label.text = "ENEMY  •  %03d / %03d HP" % [remaining, maximum]
		_append_input_log("Server confirmed %d damage" % message.get("damage"))
	var actor: MeshInstance3D = _actors.get(target_id)
	if actor != null:
		var original := actor.scale
		actor.scale = original * 1.25
		var hit_tween := create_tween()
		hit_tween.tween_property(actor, "scale", original, 0.12)


func _update_objective_hud(objective: Dictionary) -> void:
	var label := str(objective.get("objective_id", "unknown")).replace("_", " ").to_upper()
	var progress: int = objective.get("progress", 0)
	var target: int = objective.get("target", 0)
	_objective_label.text = "OBJECTIVE  •  %s  [%d/%d]  %s" % [label, progress, target, objective.get("state", "")]
	if objective.get("objective_type") == "ReachArea" and objective.get("state") == "Active":
		_status_label.text = "RELAY DOOR UNLOCKED  •  MOVE TO X=6"
		_set_guidance("STEP 2  •  OPEN THE CORE", "Hold D to move right until you reach the orange relay door at x=6.")


func _apply_inventory_snapshot(message: Dictionary) -> void:
	_inventory.clear()
	for item in message.get("items", []):
		_inventory[item.get("item_id", "unknown")] = item.get("quantity", 0)
	_update_inventory_hud()


func _apply_loot_grant(message: Dictionary) -> void:
	var item_id: String = message.get("item_id", "unknown")
	_inventory[item_id] = message.get("resulting_quantity", 0)
	_update_inventory_hud()
	_status_label.text = "LOOT SECURED  •  %s +%d" % [item_id.replace("_", " ").to_upper(), message.get("quantity", 0)]
	print("loot granted: %s x%d" % [item_id, message.get("quantity", 0)])


func _update_inventory_hud() -> void:
	var lines: Array[String] = ["INVENTORY"]
	var item_ids := _inventory.keys()
	item_ids.sort()
	for item_id in item_ids:
		lines.append("%s  x%d" % [str(item_id).replace("_", " ").to_upper(), _inventory[item_id]])
	_inventory_label.text = "\n".join(lines)


func _apply_progression(message: Dictionary) -> void:
	_progression["level"] = message.get("level", 1)
	_progression["experience"] = message.get("experience", 0)
	_progression["experience_to_next_level"] = message.get("experience_to_next_level", 500)
	_update_progression_hud()
	if message.get("type") == "ProgressionGranted":
		_status_label.text = "PROGRESSION SECURED  •  +%d XP" % message.get("experience_granted", 0)
		print("progression granted: +%d XP, level %d" % [message.get("experience_granted", 0), message.get("level", 1)])


func _update_progression_hud() -> void:
	_progression_label.text = "PROGRESSION  •  LEVEL %d\nXP %d  •  %d TO NEXT LEVEL" % [
		_progression.get("level", 1),
		_progression.get("experience", 0),
		_progression.get("experience_to_next_level", 500),
	]


func _apply_equipment_snapshot(message: Dictionary) -> void:
	_weapon_profiles.clear()
	for weapon in message.get("weapons", []):
		_weapon_profiles[weapon.get("item_id", "unknown")] = weapon
	_equipped_weapon_item_id = message.get("equipped_weapon_item_id", "pulse_rifle")
	_update_equipment_hud()


func _apply_equipment_change(message: Dictionary) -> void:
	if not message.get("accepted", false):
		_status_label.text = "EQUIP REJECTED  •  %s" % message.get("message", "SERVER REJECTED ITEM")
		return
	_equipped_weapon_item_id = message.get("equipped_weapon_item_id", _equipped_weapon_item_id)
	_update_equipment_hud()
	_status_label.text = "WEAPON EQUIPPED  •  %s" % _equipped_weapon_item_id.replace("_", " ").to_upper()
	print("authoritative weapon equipped: %s" % _equipped_weapon_item_id)


func _update_equipment_hud() -> void:
	var profile: Dictionary = _weapon_profiles.get(_equipped_weapon_item_id, {})
	_equipment_label.text = "WEAPON  •  %s\nDMG %d  RANGE %d  COOLDOWN %d MS" % [
		_equipped_weapon_item_id.replace("_", " ").to_upper(),
		profile.get("damage", 0),
		profile.get("range", 0),
		profile.get("cooldown_ms", 0),
	]


func _set_door_open(open: bool) -> void:
	_door.visible = not open
	_status_label.text = "RELAY CORE OPEN  •  WARDEN INBOUND" if open else "RELAY CORE SEALED"
	if open:
		_set_guidance("CORE OPEN", "The Warden is spawning. Get ready to aim and attack.")


func _build_playable_scene() -> void:
	_camera = Camera3D.new()
	_camera.name = "GameplayCamera"
	_camera.position = Vector3(8, 13, 15)
	add_child(_camera)
	_camera.look_at_from_position(_camera.position, Vector3(4, 0, 0), Vector3.UP)

	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-55, -25, 0)
	light.light_energy = 1.4
	add_child(light)

	var ground := MeshInstance3D.new()
	var ground_mesh := PlaneMesh.new()
	ground_mesh.size = Vector2(26, 26)
	ground.mesh = ground_mesh
	ground.material_override = _material(Color("101927"))
	add_child(ground)

	_door = MeshInstance3D.new()
	_door.name = "RelayCoreDoor"
	var door_mesh := BoxMesh.new()
	door_mesh.size = Vector3(0.6, 3.5, 5.0)
	_door.mesh = door_mesh
	_door.position = Vector3(6.5, 1.75, 0)
	_door.material_override = _material(Color("f5a524"))
	add_child(_door)

	var canvas := CanvasLayer.new()
	canvas.name = "HUD"
	add_child(canvas)
	var panel := ColorRect.new()
	panel.color = Color(0.025, 0.045, 0.075, 0.92)
	panel.position = Vector2(24, 24)
	panel.size = Vector2(620, 198)
	canvas.add_child(panel)
	_status_label = _hud_label(canvas, Vector2(44, 38), 22, Color("35d0ba"), "CONNECTING TO REVENANT CORE")
	_health_label = _hud_label(canvas, Vector2(44, 72), 26, Color.WHITE, "HP  100 / 100")
	_objective_label = _hud_label(canvas, Vector2(44, 112), 18, Color("f5a524"), "OBJECTIVE  •  WAITING FOR ACTIVITY")
	_enemy_health_label = _hud_label(canvas, Vector2(44, 146), 18, Color("ff6b6b"), "ENEMY  •  WAITING FOR ENCOUNTER")
	_position_label = _hud_label(canvas, Vector2(44, 176), 16, Color("a9b8cc"), "POSITION  [0, 0]  •  DOOR 6 STEPS")
	_controls_label = _hud_label(canvas, Vector2(24, 674), 16, Color("a9b8cc"), "CONNECTING...")
	_crosshair = _hud_label(canvas, Vector2(632, 344), 28, Color("f5a524"), "+")
	_crosshair.mouse_filter = Control.MOUSE_FILTER_IGNORE

	var briefing := ColorRect.new()
	briefing.color = Color(0.025, 0.045, 0.075, 0.92)
	briefing.position = Vector2(894, 24)
	briefing.size = Vector2(362, 170)
	canvas.add_child(briefing)
	_hud_label(canvas, Vector2(918, 42), 16, Color("f5a524"), "MISSION GUIDE")
	_guidance_label = _hud_label(canvas, Vector2(918, 72), 18, Color.WHITE, "CONNECTING\n\nWait for the relay-hub connection.")
	_guidance_label.size = Vector2(314, 104)
	_guidance_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	var input_console := ColorRect.new()
	input_console.color = Color(0.025, 0.045, 0.075, 0.92)
	input_console.position = Vector2(894, 214)
	input_console.size = Vector2(362, 188)
	input_console.mouse_filter = Control.MOUSE_FILTER_IGNORE
	canvas.add_child(input_console)
	_hud_label(canvas, Vector2(918, 232), 16, Color("35d0ba"), "INPUT MONITOR")
	_input_log_label = _hud_label(canvas, Vector2(918, 262), 14, Color("a9b8cc"), "Click inside the game window.\nWaiting for keyboard or mouse input...")
	_input_log_label.size = Vector2(314, 126)
	_input_log_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	var inventory_panel := ColorRect.new()
	inventory_panel.color = Color(0.025, 0.045, 0.075, 0.92)
	inventory_panel.position = Vector2(894, 422)
	inventory_panel.size = Vector2(362, 176)
	inventory_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	canvas.add_child(inventory_panel)
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
	_attack_button.pressed.connect(_request_ui_attack)
	canvas.add_child(_attack_button)
	_create_weapon_button(canvas, "RIFLE", "pulse_rifle", Vector2(280, 642))
	_create_weapon_button(canvas, "SIDEARM", "arc_sidearm", Vector2(400, 642))


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
	button.button_down.connect(_set_ui_movement.bind(direction))
	button.button_up.connect(_clear_ui_movement)
	parent.add_child(button)
	_movement_buttons.append(button)


func _set_ui_movement(direction: Vector2) -> void:
	_ui_movement = direction
	_append_input_log("ON-SCREEN MOVE %s" % direction)


func _clear_ui_movement() -> void:
	_ui_movement = Vector2.ZERO


func _request_ui_attack() -> void:
	_attack_requested = true
	_ui_attack_requested = true
	_append_input_log("ON-SCREEN ATTACK detected")


func _create_weapon_button(parent: Node, text: String, item_id: String, position: Vector2) -> void:
	var button := Button.new()
	button.text = text
	button.position = position
	button.size = Vector2(108, 40)
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
	while _equipped_weapon_item_id != "arc_sidearm" and Time.get_ticks_msec() < equip_deadline:
		await get_tree().process_frame
	if _equipped_weapon_item_id != "arc_sidearm":
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
		var player: Node3D = _actors.get(_player_actor_id)
		if player != null and player.position.x >= 6:
			break
		await get_tree().process_frame
	_clear_ui_movement()
	var player: Node3D = _actors.get(_player_actor_id)
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
	while not _activity_complete and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if not _activity_complete:
		_driver_fail("relay_awakening did not complete through manual controls")
		return
	print("M17 manual controls completed relay_awakening without user input")
	get_tree().quit(0)


func _driver_fail(message: String) -> void:
	_fail(message)
	get_tree().quit(1)


func _material(color: Color) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.metallic = 0.35
	material.roughness = 0.42
	return material


func _set_guidance(title: String, instructions: String) -> void:
	_guidance_label.text = "%s\n\n%s" % [title, instructions]


func _append_input_log(message: String) -> void:
	if _input_log_label == null:
		return
	_input_events.push_front("• %s" % message)
	if _input_events.size() > 6:
		_input_events.resize(6)
	_input_log_label.text = "WINDOW FOCUS: %s\n%s" % ["YES" if get_window().has_focus() else "NO", "\n".join(_input_events)]


func _validate_playable_slice() -> void:
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
		or _movement_buttons.size() != 4
		or _attack_button == null
		or _weapon_buttons.size() != 2
		or _crosshair.mouse_filter != Control.MOUSE_FILTER_IGNORE
	):
		_fail("M17 playable scene composition is incomplete")
		get_tree().quit(1)
		return
	if _encode_array([-12, 0, 12]) != PackedByteArray([0x93, 0xf4, 0x00, 0x0c]):
		_fail("M17 movement encoder does not support signed relay-hub coordinates")
		get_tree().quit(1)
		return
	print("M17 playable slice validated: camera, HUD, movement, aiming and attack inputs are ready")
	print("M18 inventory HUD validated: authoritative snapshot and loot presentation are ready")
	print("M19 progression HUD validated: authoritative experience and level presentation are ready")
	print("M20 loadout HUD validated: authoritative weapon selection and profiles are ready")
	get_tree().quit(0)


func _encode_map(value: Dictionary) -> PackedByteArray:
	var bytes := PackedByteArray([0x80 | value.size()])
	for key in value:
		bytes.append_array(_encode_string(key))
		var item = value[key]
		if item is String:
			bytes.append_array(_encode_string(item))
		elif item is int:
			bytes.append_array(_encode_integer(item))
		elif item is Array:
			bytes.append_array(_encode_array(item))
		else:
			push_error("unsupported M1 MessagePack value")
			_encode_failed = true
	return bytes


func _encode_array(value: Array) -> PackedByteArray:
	var bytes := PackedByteArray([0x90 | value.size()])
	for item in value:
		if item is int:
			bytes.append_array(_encode_integer(item))
		else:
			push_error("unsupported M8 MessagePack array value")
			_encode_failed = true
	return bytes


func _encode_integer(value: int) -> PackedByteArray:
	if value >= 0 and value <= 127:
		return PackedByteArray([value])
	if value >= -32 and value < 0:
		return PackedByteArray([256 + value])
	push_error("unsupported MessagePack integer value: %d" % value)
	_encode_failed = true
	return PackedByteArray()


func _encode_string(value: String) -> PackedByteArray:
	var utf8 := value.to_utf8_buffer()
	var bytes := PackedByteArray()
	if utf8.size() <= 31:
		bytes.append(0xa0 | utf8.size())
	else:
		bytes.append(0xd9)
		bytes.append(utf8.size())
	bytes.append_array(utf8)
	return bytes


func _decode_value(bytes: PackedByteArray, offset: int) -> Array:
	if offset >= bytes.size():
		return []
	var marker := bytes[offset]
	if marker <= 0x7f:
		return [marker, offset + 1]
	if marker >= 0xa0 and marker <= 0xbf:
		return _decode_string(bytes, offset + 1, marker & 0x1f)
	if marker >= 0x80 and marker <= 0x8f:
		var result := {}
		var cursor := offset + 1
		for _entry in range(marker & 0x0f):
			var key := _decode_value(bytes, cursor)
			if key.is_empty():
				return []
			cursor = key[1]
			var item := _decode_value(bytes, cursor)
			if item.is_empty():
				return []
			cursor = item[1]
			result[key[0]] = item[0]
		return [result, cursor]
	if marker >= 0x90 and marker <= 0x9f:
		var result := []
		var cursor := offset + 1
		for _entry in range(marker & 0x0f):
			var item := _decode_value(bytes, cursor)
			if item.is_empty():
				return []
			cursor = item[1]
			result.append(item[0])
		return [result, cursor]
	if marker == 0xc2:
		return [false, offset + 1]
	if marker == 0xc3:
		return [true, offset + 1]
	if marker == 0xcc and offset + 1 < bytes.size():
		return [bytes[offset + 1], offset + 2]
	if marker == 0xcd and offset + 2 < bytes.size():
		return [(bytes[offset + 1] << 8) | bytes[offset + 2], offset + 3]
	if marker == 0xce and offset + 4 < bytes.size():
		var value32 := 0
		for index in range(1, 5):
			value32 = (value32 << 8) | bytes[offset + index]
		return [value32, offset + 5]
	if marker == 0xcf and offset + 8 < bytes.size():
		var value64 := 0
		for index in range(1, 9):
			value64 = (value64 << 8) | bytes[offset + index]
		return [value64, offset + 9]
	if marker == 0xd9 and offset + 1 < bytes.size():
		return _decode_string(bytes, offset + 2, bytes[offset + 1])
	return []


func _decode_string(bytes: PackedByteArray, offset: int, length: int) -> Array:
	if offset + length > bytes.size():
		return []
	return [bytes.slice(offset, offset + length).get_string_from_utf8(), offset + length]


func _fail(message: String) -> void:
	push_error("Revenant handshake failed: %s" % message)
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
		get_tree().quit(1)


func _should_exit_after_flow() -> bool:
	return OS.get_environment("REVENANT_EXIT_AFTER_FLOW") == "1"
