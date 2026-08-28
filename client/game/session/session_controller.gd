extends Node

signal connection_state_requested(state: String, detail: String)

const PROTOCOL_VERSION := 2
const CLIENT_NAME := "revenant-godot"
const CLIENT_BUILD := "0.2.0"
const MESSAGE_DEADLINE_MS := 5000
const FRAMED_TRANSPORT := preload("res://protocol/framed_transport.gd")

var _transport: Node


func _ready() -> void:
	_transport = FRAMED_TRANSPORT.new()
	_transport.name = "FramedTransport"
	add_child(_transport)


func reset_connection() -> void:
	_transport.call("reset_connection")


func join_initial_session(username: String, host: String, port: int) -> Dictionary:
	var connect_error: Error = _transport.call("connect_to_host", host, port)
	if connect_error != OK:
		return _failure("connection could not start: %s" % error_string(connect_error))

	var deadline := _deadline()
	while connection_status() == StreamPeerTCP.STATUS_CONNECTING and Time.get_ticks_msec() < deadline:
		_transport.call("poll")
		await get_tree().process_frame
	if connection_status() != StreamPeerTCP.STATUS_CONNECTED:
		return _failure("connection to %s:%d timed out" % [host, port])

	connection_state_requested.emit("Negotiating", "Relay transport connected. Verifying Protocol V2 compatibility.")
	if not send_message({"type": "ClientHello", "protocol_version": PROTOCOL_VERSION, "client_name": CLIENT_NAME, "client_build": CLIENT_BUILD}):
		return _failure("ClientHello send failed")
	var server_hello := await receive_message(_deadline())
	if server_hello.get("type") != "ServerHello":
		return _failure("expected ServerHello")
	if not server_hello.get("accepted", false):
		return _failure("server rejected handshake: %s" % server_hello.get("message", "unknown error"))
	if server_hello.get("protocol_version") != PROTOCOL_VERSION:
		return _failure("server selected an unexpected protocol version")
	print("handshake accepted by %s using protocol v%d" % [server_hello.get("server_name"), PROTOCOL_VERSION])

	connection_state_requested.emit("Authenticating", "Protocol accepted. Requesting the local Operator identity.")
	if not send_message({"type": "AuthRequest", "username": username}):
		return _failure("AuthRequest send failed")
	var auth_response := await receive_message(_deadline())
	if auth_response.get("type") != "AuthResponse":
		return _failure("expected AuthResponse")
	if not auth_response.get("authenticated", false):
		return _failure("local authentication rejected: %s" % auth_response.get("message", "unknown error"))
	print("authenticated local account %s" % auth_response.get("account_id"))

	connection_state_requested.emit("Joining", "Identity accepted. Loading the server-owned character and relay state.")
	if not send_message({"type": "CharacterListRequest"}):
		return _failure("CharacterListRequest send failed")
	var character_response := await receive_message(_deadline())
	if character_response.get("type") != "CharacterListResponse":
		return _failure("expected CharacterListResponse")
	var characters: Array = character_response.get("characters", [])
	if characters.is_empty():
		return _failure("server returned no local character")
	var character: Dictionary = characters[0]
	print("received character %s (%s, level %d)" % [character.get("display_name"), character.get("class_name"), character.get("level")])

	if not send_message({"type": "WorldJoinRequest", "character_id": character.get("character_id")}):
		return _failure("WorldJoinRequest send failed")
	var world_response := await receive_message(_deadline())
	if world_response.get("type") != "WorldJoinResponse":
		return _failure("expected WorldJoinResponse")
	if not world_response.get("accepted", false):
		return _failure("world join rejected: %s" % world_response.get("message", "unknown error"))
	print("joined world %s as player actor %d" % [world_response.get("world_id"), world_response.get("player_actor_id")])

	var inventory_snapshot := await receive_message(_deadline())
	if inventory_snapshot.get("type") != "InventorySnapshot":
		return _failure("expected InventorySnapshot")
	var progression_snapshot := await receive_message(_deadline())
	if progression_snapshot.get("type") != "ProgressionSnapshot":
		return _failure("expected ProgressionSnapshot")
	var equipment_snapshot := await receive_message(_deadline())
	if equipment_snapshot.get("type") != "EquipmentSnapshot":
		return _failure("expected EquipmentSnapshot")
	return {
		"ok": true,
		"world": world_response,
		"inventory": inventory_snapshot,
		"progression": progression_snapshot,
		"equipment": equipment_snapshot,
	}


func send_message(value: Dictionary) -> bool:
	return _transport.call("send_message", value)


func receive_message(deadline: int) -> Dictionary:
	return await _transport.call("receive_message", deadline)


func try_receive_message() -> Dictionary:
	return _transport.call("try_receive_message")


func connection_status() -> StreamPeerTCP.Status:
	return _transport.call("connection_status")


func presentation_state() -> Dictionary:
	return {
		"protocol_version": PROTOCOL_VERSION,
		"message_deadline_ms": MESSAGE_DEADLINE_MS,
		"transport": _transport.call("presentation_state"),
	}


func _deadline() -> int:
	return Time.get_ticks_msec() + MESSAGE_DEADLINE_MS


func _failure(message: String) -> Dictionary:
	return {"ok": false, "error": message}
