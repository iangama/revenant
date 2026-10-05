extends Node

signal connection_state_requested(state: String, detail: String)
signal campaign_snapshot_loaded(username: String, snapshot: Dictionary)
signal challenge_snapshot_loaded(username: String, snapshot: Dictionary)

const PROTOCOL_VERSION := 2
const CLIENT_NAME := "revenant-godot"
const CLIENT_BUILD := "0.2.0"
const MESSAGE_DEADLINE_MS := 5000
const FRAMED_TRANSPORT := preload("res://protocol/framed_transport.gd")

var _transport: Node
var exploration_capable := false
var encounter_capable := false
var bulwark_capable := false
var support_capable := false
var elite_capable := false
var prism_capable := false
var acquisition_capable := false
var entry_mode := "standalone"
var challenge_preset := "baseline"
var campaign_capable := false
var challenge_capable := false


func _ready() -> void:
	_transport = FRAMED_TRANSPORT.new()
	_transport.name = "FramedTransport"
	add_child(_transport)


func reset_connection() -> void:
	exploration_capable = false
	encounter_capable = false
	bulwark_capable = false
	support_capable = false
	elite_capable = false
	prism_capable = false
	acquisition_capable = false
	campaign_capable = false
	challenge_capable = false
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
		var failure_outcome := classify_transport_failure(Time.get_ticks_msec() >= deadline)
		if failure_outcome == "timeout":
			return _failure("connection to %s:%d timed out" % [host, port], failure_outcome)
		return _failure("connection to %s:%d was refused or closed" % [host, port], failure_outcome)

	connection_state_requested.emit("Negotiating", "Relay transport connected. Verifying Protocol V2 compatibility.")
	var content_revision := "m37-v9" if entry_mode.begins_with("challenge") else "m35-v3" if entry_mode == "standalone" else "m36-v2"
	if not send_message({"type": "ClientHello", "protocol_version": PROTOCOL_VERSION, "client_name": CLIENT_NAME, "client_build": CLIENT_BUILD, "content_revision": content_revision}):
		return _failure("ClientHello send failed")
	var server_hello := await receive_message(_deadline())
	if server_hello.get("type") != "ServerHello":
		return _failure("expected ServerHello")
	if not server_hello.get("accepted", false):
		return _failure(
			"server rejected handshake: %s" % server_hello.get("message", "unknown error"),
			classify_rejection("handshake", str(server_hello.get("message", "")))
		)
	if server_hello.get("protocol_version") != PROTOCOL_VERSION:
		return _failure(
			"server selected an unexpected protocol version",
			classify_rejection("handshake", "unexpected protocol version")
		)
	print("handshake accepted by %s using protocol v%d" % [server_hello.get("server_name"), PROTOCOL_VERSION])
	challenge_capable = server_hello.get("content_revision", "") in ["m37-v1", "m37-v2", "m37-v3", "m37-v4", "m37-v5", "m37-v6", "m37-v7", "m37-v8", "m37-v9"]
	campaign_capable = challenge_capable or server_hello.get("content_revision", "") == "m36-v2"
	acquisition_capable = campaign_capable or server_hello.get("content_revision", "") == "m35-v3"
	prism_capable = acquisition_capable or server_hello.get("content_revision", "") in ["m34-v5", "m35-v1", "m35-v2"]
	elite_capable = prism_capable or server_hello.get("content_revision", "") == "m34-v4"
	support_capable = elite_capable or server_hello.get("content_revision", "") == "m34-v3"
	bulwark_capable = support_capable or server_hello.get("content_revision", "") == "m34-v2"
	encounter_capable = bulwark_capable or server_hello.get("content_revision", "") == "m34-v1"
	exploration_capable = encounter_capable or server_hello.get("content_revision", "") == "m33-v1"

	connection_state_requested.emit("Authenticating", "Protocol accepted. Requesting the local Operator identity.")
	if not send_message({"type": "AuthRequest", "username": username}):
		return _failure("AuthRequest send failed")
	var auth_response := await receive_message(_deadline())
	if auth_response.get("type") != "AuthResponse":
		return _failure("expected AuthResponse")
	if not auth_response.get("authenticated", false):
		return _failure(
			"local authentication rejected: %s" % auth_response.get("message", "unknown error"),
			classify_rejection("authentication", str(auth_response.get("message", "")))
		)
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

	var join_request := {"type": "WorldJoinRequest", "character_id": character.get("character_id")}
	if entry_mode.begins_with("challenge"):
		if not challenge_capable:
			return _failure(tr("This server does not support challenges."))
		if not send_message({"type": "ChallengeStateRequest", "character_id": character.get("character_id")}):
			return _failure("ChallengeStateRequest send failed")
		var board := await receive_message(_deadline())
		var projection := preload("res://projection/challenge_state.gd").new()
		if not projection.apply(board, str(character.get("character_id"))):
			return _failure("expected valid challenge board")
		challenge_snapshot_loaded.emit(username, board)
		if entry_mode == "challenges":
			reset_connection()
			return {"ok": true, "challenge_board": true}
		var contract := entry_mode.trim_prefix("challenge_")
		var previous: Variant = board.get("active")
		if previous == null and board.get("last_result") is Dictionary:
			previous = board.last_result.run
		var same_variant: bool = previous is Dictionary and previous.contract.contract_id == contract and projection.preset_id(previous.contract) == challenge_preset
		if board.get("active") is Dictionary and not same_variant:
			return _failure(tr("Restart the interrupted contract before choosing another one."))
		var retry: Variant = previous.run_id if same_variant else null
		join_request.merge({"type": "ChallengeJoinRequest", "operation_id": "godot-start-%d-%d" % [int(Time.get_unix_time_from_system() * 1000000), Time.get_ticks_usec()], "expected_revision": board.state_revision, "contract_id": contract, "retry_run_id": retry, "preset_id": challenge_preset if challenge_preset != "baseline" else null}, true)
	elif entry_mode != "standalone":
		if not campaign_capable:
			return _failure(tr("This server does not support campaign entry."))
		if not send_message({"type": "CampaignStateRequest", "character_id": character.get("character_id")}):
			return _failure("CampaignStateRequest send failed")
		var saved := await receive_message(_deadline())
		if saved.get("type") != "CampaignSnapshot" or saved.get("revision") not in ["m36-campaign-v1", "m36-campaign-v2", "m36-campaign-v3", "m36-campaign-v4", "m36-campaign-v5", "m36-campaign-v6"]:
			return _failure("expected campaign snapshot")
		campaign_snapshot_loaded.emit(username, saved)
		var active: Dictionary = saved.get("active") if saved.get("active") is Dictionary else {}
		var chapter := ""
		var mode := "progress"
		if not active.is_empty():
			chapter = active.get("chapter_id", "")
			mode = "resume"
		elif entry_mode.begins_with("practice_"):
			chapter = entry_mode.trim_prefix("practice_")
			mode = "practice"
			var index := ["return_signal", "meridian_readings", "broken_supply_line", "counter_signal", "the_breach", "prism_core"].find(chapter)
			if index < 0 or index >= int(saved.get("cleared_chapters", 0)):
				return _failure(tr("Complete this chapter before entering practice."))
		else:
			var cleared := int(saved.get("cleared_chapters", 0))
			if cleared >= int(saved.get("available_chapters", 0)) or cleared >= 6:
				if cleared == 6:
					reset_connection()
					return {"ok": true, "campaign_complete": true, "campaign_snapshot": saved}
				return _failure(tr("The available chapters are complete. Choose practice or a standalone operation."))
			chapter = ["return_signal", "meridian_readings", "broken_supply_line", "counter_signal", "the_breach", "prism_core"][cleared]
		join_request.merge({"type": "CampaignJoinRequest", "chapter_id": chapter, "mode": mode, "expected_revision": saved.get("state_revision", 0)}, true)
	if not send_message(join_request):
		return _failure("WorldJoinRequest send failed")
	var world_response := await receive_message(_deadline())
	if world_response.get("type") != "WorldJoinResponse":
		return _failure("expected WorldJoinResponse")
	if not world_response.get("accepted", false):
		var rejection := str(world_response.get("message", "unknown error"))
		var rejection_kind := classify_rejection("world_join", rejection)
		return _failure("world join rejected: %s" % rejection, rejection_kind)
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
		"campaign": entry_mode != "standalone" and not entry_mode.begins_with("challenge"),
		"challenge": entry_mode.trim_prefix("challenge_") if entry_mode.begins_with("challenge_") else "",
		"campaign_chapter": join_request.get("chapter_id", ""),
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


static func classify_transport_failure(deadline_expired: bool) -> String:
	return "timeout" if deadline_expired else "transport_failure"


static func classify_rejection(boundary: String, message: String) -> String:
	if boundary in ["handshake", "authentication"]:
		return "rejected"
	if boundary == "world_join" and "already active" in message.to_lower():
		return "session_unavailable"
	return "transport_failure"


func _failure(message: String, connection_outcome := "transport_failure") -> Dictionary:
	return {"ok": false, "error": message, "connection_outcome": connection_outcome}
