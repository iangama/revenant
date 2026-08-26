extends Node3D

const AMBIENCE_PATH := "res://audio/m22/relay_hub_ambience.wav"
const SYSTEM_READY_PATH := "res://audio/m22/system_ready.wav"
const DOOR_UNLOCK_PATH := "res://audio/m22/relay_door_unlock.wav"
const SYSTEM_POOL_SIZE := 2
const MAX_VOICES := 4
const MIN_SYSTEM_RETRIGGER_MS := 500

var _ambience: AudioStreamPlayer
var _ambience_stream: AudioStreamWAV
var _system_stream: AudioStreamWAV
var _door_stream: AudioStreamWAV
var _door: AudioStreamPlayer3D
var _system_pool: Array[AudioStreamPlayer] = []
var _next_system_player := 0
var _last_system_at := -MIN_SYSTEM_RETRIGGER_MS
var _silent := false
var _requests := {"ambience": 0, "system_ready": 0, "door_unlock": 0}
var _played := {"ambience": 0, "system_ready": 0, "door_unlock": 0}
var _suppressed := 0


func _ready() -> void:
	_ambience = AudioStreamPlayer.new()
	_ambience.name = "RelayHubAmbience"
	_ambience.bus = "Ambience"
	_ambience_stream = _load_pcm_wav(AMBIENCE_PATH)
	_ambience_stream.loop_mode = AudioStreamWAV.LOOP_FORWARD
	_ambience.stream = _ambience_stream
	add_child(_ambience)

	_door = AudioStreamPlayer3D.new()
	_door.name = "RelayDoorCue"
	_door.bus = "Effects"
	_door_stream = _load_pcm_wav(DOOR_UNLOCK_PATH)
	_door.stream = _door_stream
	_door.max_distance = 24.0
	_door.unit_size = 5.0
	add_child(_door)

	_system_stream = _load_pcm_wav(SYSTEM_READY_PATH)
	for index in range(SYSTEM_POOL_SIZE):
		var player := AudioStreamPlayer.new()
		player.name = "SystemCue%d" % index
		player.bus = "Interface"
		player.stream = _system_stream
		add_child(player)
		_system_pool.append(player)


func configure_routes() -> void:
	_ambience.bus = "Ambience"
	_door.bus = "Effects"
	for player in _system_pool:
		player.bus = "Interface"


func set_silent(value: bool) -> void:
	_silent = value
	if _silent:
		_stop_all()
	else:
		start_ambience()


func start_ambience() -> void:
	_requests["ambience"] += 1
	if _silent:
		_suppressed += 1
		return
	if not _ambience.playing:
		_ambience.play()
		_played["ambience"] += 1


func play_system_ready() -> void:
	_requests["system_ready"] += 1
	var now := Time.get_ticks_msec()
	if _silent or now - _last_system_at < MIN_SYSTEM_RETRIGGER_MS:
		_suppressed += 1
		return
	var player := _system_pool[_next_system_player]
	_next_system_player = (_next_system_player + 1) % _system_pool.size()
	player.play()
	_last_system_at = now
	_played["system_ready"] += 1


func apply_door_state(open: bool, world_position: Vector3) -> void:
	if not open:
		return
	_requests["door_unlock"] += 1
	if _silent:
		_suppressed += 1
		return
	_door.global_position = world_position
	_door.play()
	_played["door_unlock"] += 1


func presentation_state() -> Dictionary:
	var active_voices := int(_ambience.playing) + int(_door.playing)
	for player in _system_pool:
		active_voices += int(player.playing)
	return {
		"fixed_nodes": 2 + _system_pool.size(),
		"maximum_voices": MAX_VOICES,
		"active_voices": active_voices,
		"silent": _silent,
		"ambience_looping": _ambience_stream.loop_mode == AudioStreamWAV.LOOP_FORWARD,
		"decoded_bytes": _ambience_stream.data.size() + _door_stream.data.size() + _system_stream.data.size(),
		"routes": {"ambience": str(_ambience.bus), "door": str(_door.bus), "system": str(_system_pool[0].bus)},
		"requests": _requests.duplicate(true),
		"played": _played.duplicate(true),
		"suppressed": _suppressed,
	}


func _stop_all() -> void:
	_ambience.stop()
	_door.stop()
	for player in _system_pool:
		player.stop()


func _load_pcm_wav(path: String) -> AudioStreamWAV:
	var bytes := FileAccess.get_file_as_bytes(path)
	var stream := AudioStreamWAV.new()
	stream.format = AudioStreamWAV.FORMAT_16_BITS
	stream.mix_rate = 48000
	stream.stereo = false
	stream.data = bytes.slice(44) if bytes.size() >= 44 else PackedByteArray()
	return stream
