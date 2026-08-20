extends Node

const PROTOCOL_VERSION := 2
const MAX_FRAME_SIZE := 64 * 1024
const DEFAULT_HOST := "127.0.0.1"
const DEFAULT_PORT := 7000

var _peer := StreamPeerTCP.new()
var _receive_buffer := PackedByteArray()
var _actors := {}


func _ready() -> void:
	call_deferred("_run_handshake")


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
		"client_build": "0.1.0",
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
	var activity := await _receive_message(Time.get_ticks_msec() + 5000)
	if activity.get("type") != "ActivityStart":
		_fail("expected ActivityStart")
		return
	var initial_objective := await _receive_message(Time.get_ticks_msec() + 5000)
	if initial_objective.get("type") != "ObjectiveUpdate" or initial_objective.get("state") != "Active":
		_fail("expected active initial objective")
		return
	print("activity %s started with objective %s" % [activity.get("activity_id"), initial_objective.get("objective_id")])

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
			break
		else:
			_fail("unexpected message while observing enemy AI")
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
	var activity_complete := await _receive_message(Time.get_ticks_msec() + 5000)
	if boss_complete.get("state") != "Completed" or activity_complete.get("type") != "ActivityComplete":
		_fail("activity did not complete after boss death")
		return
	print("activity %s completed" % activity_complete.get("activity_id"))
	if _should_exit_after_flow():
		get_tree().quit(0)


func _send_message(value: Dictionary) -> bool:
	var payload := _encode_map(value)
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
	else:
		instance.mesh = CapsuleMesh.new()
	var position: Array = actor.get("position", [0, 0, 0])
	instance.position = Vector3(position[0], position[1], position[2])
	add_child(instance)
	_actors[actor.get("actor_id")] = instance
	print("rendered %s actor %d (%s)" % [actor.get("actor_kind"), actor.get("actor_id"), actor.get("archetype")])


func _destroy_actor(actor_id: int) -> void:
	var actor: Node = _actors.get(actor_id)
	if actor != null:
		actor.queue_free()
		_actors.erase(actor_id)
	print("enemy actor %d destroyed" % actor_id)


func _update_actor(update: Dictionary) -> void:
	var actor: Node3D = _actors.get(update.get("actor_id"))
	if actor == null:
		return
	var position: Array = update.get("position", [0, 0, 0])
	actor.position = Vector3(position[0], position[1], position[2])
	print("actor %d moved to %s" % [update.get("actor_id"), position])


func _encode_map(value: Dictionary) -> PackedByteArray:
	var bytes := PackedByteArray([0x80 | value.size()])
	for key in value:
		bytes.append_array(_encode_string(key))
		var item = value[key]
		if item is String:
			bytes.append_array(_encode_string(item))
		elif item is int and item >= 0 and item <= 127:
			bytes.append(item)
		elif item is Array:
			bytes.append_array(_encode_array(item))
		else:
			push_error("unsupported M1 MessagePack value")
	return bytes


func _encode_array(value: Array) -> PackedByteArray:
	var bytes := PackedByteArray([0x90 | value.size()])
	for item in value:
		if item is int and item >= 0 and item <= 127:
			bytes.append(item)
		else:
			push_error("unsupported M8 MessagePack array value")
	return bytes


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
	if marker == 0xd9 and offset + 1 < bytes.size():
		return _decode_string(bytes, offset + 2, bytes[offset + 1])
	return []


func _decode_string(bytes: PackedByteArray, offset: int, length: int) -> Array:
	if offset + length > bytes.size():
		return []
	return [bytes.slice(offset, offset + length).get_string_from_utf8(), offset + length]


func _fail(message: String) -> void:
	push_error("Revenant handshake failed: %s" % message)
	if _should_exit_after_flow():
		get_tree().quit(1)


func _should_exit_after_flow() -> bool:
	return OS.get_environment("REVENANT_EXIT_AFTER_FLOW") == "1"
