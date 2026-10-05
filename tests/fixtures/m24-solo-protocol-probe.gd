extends SceneTree

const FRAMED_TRANSPORT := preload("res://protocol/framed_transport.gd")
const SESSION_CONTROLLER := preload("res://session/session_controller.gd")


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var mode := OS.get_environment("M24_PROBE_MODE")
	var host := OS.get_environment("M24_PROBE_HOST")
	var port := int(OS.get_environment("M24_PROBE_PORT"))
	if host.is_empty():
		host = "127.0.0.1"
	if port <= 0:
		push_error("M24 probe port is required")
		quit(1)
		return
	if mode == "handshake":
		await _probe_rejected_handshake(host, port)
		return
	if mode == "admission":
		await _probe_admission(host, port)
		return
	push_error("unknown M24 protocol probe mode: %s" % mode)
	quit(1)


func _probe_rejected_handshake(host: String, port: int) -> void:
	var transport := FRAMED_TRANSPORT.new()
	get_root().add_child(transport)
	if transport.connect_to_host(host, port) != OK:
		push_error("M24 handshake probe could not start a connection")
		quit(1)
		return
	var deadline := Time.get_ticks_msec() + 5000
	while transport.connection_status() == StreamPeerTCP.STATUS_CONNECTING and Time.get_ticks_msec() < deadline:
		transport.poll()
		await process_frame
	if transport.connection_status() != StreamPeerTCP.STATUS_CONNECTED:
		push_error("M24 handshake probe did not connect")
		quit(1)
		return
	if not transport.send_message({
		"type": "ClientHello",
		"protocol_version": 99,
		"client_name": "m24-solo-probe",
		"client_build": "0.2.0",
	}):
		push_error("M24 handshake probe could not send ClientHello")
		quit(1)
		return
	var response: Dictionary = await transport.receive_message(Time.get_ticks_msec() + 5000)
	var outcome := SESSION_CONTROLLER.classify_rejection("handshake", str(response.get("message", "")))
	if response.get("type") != "ServerHello" or response.get("accepted", true):
		push_error("M24 incompatible handshake was not rejected")
		quit(1)
		return
	if response.get("message") != "unsupported protocol version" or outcome != "rejected":
		push_error("M24 incompatible handshake classification is not structured")
		quit(1)
		return
	print("M24 solo handshake probe outcome=rejected protocol=99")
	quit(0)


func _probe_admission(host: String, port: int) -> void:
	var controller := SESSION_CONTROLLER.new()
	get_root().add_child(controller)
	await process_frame
	var username := OS.get_environment("M24_PROBE_USERNAME")
	if username.is_empty():
		username = "m24-solo-probe"
	var expected := OS.get_environment("M24_PROBE_EXPECTED_OUTCOME")
	var started := Time.get_ticks_msec()
	var result: Dictionary = await controller.join_initial_session(username, host, port)
	var elapsed := Time.get_ticks_msec() - started
	if result.get("ok", false):
		push_error("M24 admission probe unexpectedly joined")
		quit(1)
		return
	if result.get("connection_outcome") != expected:
		push_error("M24 admission outcome mismatch: expected %s, got %s" % [
			expected,
			result.get("connection_outcome"),
		])
		quit(1)
		return
	print("M24 solo admission probe outcome=%s elapsed_ms=%d" % [expected, elapsed])
	quit(0)
