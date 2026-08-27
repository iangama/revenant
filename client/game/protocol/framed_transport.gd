extends Node

const MAX_FRAME_SIZE := 64 * 1024
const MESSAGEPACK_CODEC := preload("res://protocol/messagepack_codec.gd")

var _peer := StreamPeerTCP.new()
var _receive_buffer := PackedByteArray()
var _messagepack: RefCounted = MESSAGEPACK_CODEC.new()


func reset_connection() -> void:
	_peer = StreamPeerTCP.new()
	_receive_buffer.clear()


func connect_to_host(host: String, port: int) -> Error:
	return _peer.connect_to_host(host, port)


func connection_status() -> StreamPeerTCP.Status:
	return _peer.get_status()


func poll() -> void:
	_peer.poll()


func send_message(value: Dictionary) -> bool:
	var payload: PackedByteArray = _messagepack.call("encode_map", value)
	if _messagepack.call("has_failed") or payload.is_empty():
		return false
	var frame := PackedByteArray([
		(payload.size() >> 24) & 0xff,
		(payload.size() >> 16) & 0xff,
		(payload.size() >> 8) & 0xff,
		payload.size() & 0xff,
	])
	frame.append_array(payload)
	return _peer.put_data(frame) == OK


func receive_message(deadline: int) -> Dictionary:
	while _receive_buffer.size() < 4 and Time.get_ticks_msec() < deadline:
		_peer.poll()
		_receive_buffer.append_array(_read_available())
		await get_tree().process_frame
	if _receive_buffer.size() < 4:
		return {}

	var payload_size := _payload_size()
	if payload_size > MAX_FRAME_SIZE:
		return {}
	while _receive_buffer.size() < payload_size + 4 and Time.get_ticks_msec() < deadline:
		_peer.poll()
		_receive_buffer.append_array(_read_available())
		await get_tree().process_frame
	if _receive_buffer.size() < payload_size + 4:
		return {}
	return _consume_message(payload_size)


func try_receive_message() -> Dictionary:
	_peer.poll()
	_receive_buffer.append_array(_read_available())
	if _receive_buffer.size() < 4:
		return {}
	var payload_size := _payload_size()
	if payload_size > MAX_FRAME_SIZE or _receive_buffer.size() < payload_size + 4:
		return {}
	return _consume_message(payload_size)


func presentation_state() -> Dictionary:
	return {
		"maximum_frame_size": MAX_FRAME_SIZE,
		"buffered_bytes": _receive_buffer.size(),
		"connection_status": _peer.get_status(),
	}


func _payload_size() -> int:
	return (_receive_buffer[0] << 24) | (_receive_buffer[1] << 16) | (_receive_buffer[2] << 8) | _receive_buffer[3]


func _consume_message(payload_size: int) -> Dictionary:
	var decoded: Array = _messagepack.call("decode_value", _receive_buffer.slice(4, payload_size + 4))
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
