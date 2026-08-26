extends Node3D

const AUDIO_ROOT := "res://audio/m22/"
const FOUNDATION_PATHS := {
	"ambience": "relay_hub_ambience.wav",
	"system_ready": "system_ready.wav",
	"door_unlock": "relay_door_unlock.wav",
}
const CUE_PATHS := {
	"operator_move": "operator_servo.wav",
	"pulse_rifle": "pulse_rifle_confirmed.wav",
	"arc_sidearm": "arc_sidearm_confirmed.wav",
	"impact": "confirmed_impact.wav",
	"relay_drone": "relay_drone_cue.wav",
	"warden": "warden_cue.wav",
	"defeat": "enemy_defeat.wav",
	"player_damage": "player_damage.wav",
	"cooldown": "cooldown_tick.wav",
	"completion": "completion.wav",
}
const SYSTEM_POOL_SIZE := 2
const COMBAT_POOL_SIZE := 8
const CRITICAL_POOL_SIZE := 2
const MAX_VOICES := 14
const MIN_RETRIGGER_MS := 70

var _ambience: AudioStreamPlayer
var _door: AudioStreamPlayer3D
var _system_pool: Array[AudioStreamPlayer] = []
var _combat_pool: Array[AudioStreamPlayer3D] = []
var _critical_pool: Array[AudioStreamPlayer] = []
var _streams := {}
var _next_system := 0
var _next_combat := 0
var _next_critical := 0
var _last_played_at := {}
var _silent := false
var _requests := {}
var _played := {}
var _suppressed := 0


func _ready() -> void:
	for cue in FOUNDATION_PATHS:
		_streams[cue] = _load_pcm_wav(AUDIO_ROOT + FOUNDATION_PATHS[cue])
	for cue in CUE_PATHS:
		_streams[cue] = _load_pcm_wav(AUDIO_ROOT + CUE_PATHS[cue])
	for cue in _streams:
		_requests[cue] = 0
		_played[cue] = 0

	_ambience = AudioStreamPlayer.new()
	_ambience.name = "RelayHubAmbience"
	_streams["ambience"].loop_mode = AudioStreamWAV.LOOP_FORWARD
	_ambience.stream = _streams["ambience"]
	add_child(_ambience)

	_door = AudioStreamPlayer3D.new()
	_door.name = "RelayDoorCue"
	_door.stream = _streams["door_unlock"]
	_configure_spatial(_door)
	add_child(_door)

	for index in range(SYSTEM_POOL_SIZE):
		var system_player := AudioStreamPlayer.new()
		system_player.name = "SystemCue%d" % index
		system_player.stream = _streams["system_ready"]
		add_child(system_player)
		_system_pool.append(system_player)

	for index in range(COMBAT_POOL_SIZE):
		var combat_player := AudioStreamPlayer3D.new()
		combat_player.name = "CombatCue%d" % index
		_configure_spatial(combat_player)
		add_child(combat_player)
		_combat_pool.append(combat_player)

	for index in range(CRITICAL_POOL_SIZE):
		var critical_player := AudioStreamPlayer.new()
		critical_player.name = "CriticalCue%d" % index
		add_child(critical_player)
		_critical_pool.append(critical_player)


func configure_routes() -> void:
	_ambience.bus = "Ambience"
	_door.bus = "Effects"
	for player in _system_pool:
		player.bus = "Interface"
	for player in _combat_pool:
		player.bus = "Effects"
	for player in _critical_pool:
		player.bus = "Effects"


func set_silent(value: bool) -> void:
	_silent = value
	if _silent:
		_stop_all()
	else:
		start_ambience()


func shutdown() -> void:
	_stop_all()
	_ambience.stream = null
	_door.stream = null
	for pool in [_system_pool, _combat_pool, _critical_pool]:
		for player in pool:
			player.stream = null
	_streams.clear()


func start_ambience() -> void:
	_request("ambience")
	if _reject("ambience", false):
		return
	if not _ambience.playing:
		_ambience.play()
		_mark_played("ambience")


func play_system_ready() -> void:
	_request("system_ready")
	if _reject("system_ready", true):
		return
	var player := _system_pool[_next_system]
	_next_system = (_next_system + 1) % _system_pool.size()
	player.play()
	_mark_played("system_ready")


func apply_door_state(open: bool, world_position: Vector3) -> void:
	if not open:
		return
	_request("door_unlock")
	if _reject("door_unlock", true):
		return
	_door.global_position = world_position
	_door.play()
	_mark_played("door_unlock")


func play_confirmed_move(world_position: Vector3) -> void:
	_play_spatial("operator_move", world_position)


func play_enemy_presence(family: String, world_position: Vector3) -> void:
	_play_spatial("warden" if family == "warden" else "relay_drone", world_position)


func play_confirmed_attack(profile: String, world_position: Vector3) -> void:
	_play_spatial("arc_sidearm" if profile == "arc_sidearm" else "pulse_rifle", world_position)


func play_enemy_attack(family: String, world_position: Vector3) -> void:
	_play_spatial("warden" if family == "warden" else "relay_drone", world_position)


func play_confirmed_impact(world_position: Vector3) -> void:
	_play_spatial("impact", world_position)


func play_confirmed_defeat(world_position: Vector3) -> void:
	_play_spatial("defeat", world_position)


func play_player_damage() -> void:
	_play_critical("player_damage", "Effects")


func play_completion() -> void:
	_play_critical("completion", "Effects")


func play_cooldown_acknowledgement() -> void:
	_play_critical("cooldown", "Interface")


func presentation_state() -> Dictionary:
	var active_voices := int(_ambience.playing) + int(_door.playing)
	for pool in [_system_pool, _combat_pool, _critical_pool]:
		for player in pool:
			active_voices += int(player.playing)
	var decoded_bytes := 0
	for stream in _streams.values():
		decoded_bytes += stream.data.size()
	return {
		"fixed_nodes": 2 + _system_pool.size() + _combat_pool.size() + _critical_pool.size(),
		"maximum_voices": MAX_VOICES,
		"foundation_voices": 2 + _system_pool.size(),
		"combat_voices": _combat_pool.size(),
		"interface_critical_voices": _system_pool.size() + _critical_pool.size(),
		"active_voices": active_voices,
		"silent": _silent,
		"ambience_looping": _streams["ambience"].loop_mode == AudioStreamWAV.LOOP_FORWARD,
		"decoded_bytes": decoded_bytes,
		"routes": {"ambience": str(_ambience.bus), "door": str(_door.bus), "system": str(_system_pool[0].bus), "combat": str(_combat_pool[0].bus)},
		"requests": _requests.duplicate(true),
		"played": _played.duplicate(true),
		"suppressed": _suppressed,
	}


func _play_spatial(cue: String, world_position: Vector3) -> void:
	_request(cue)
	if _reject(cue, true):
		return
	var player := _combat_pool[_next_combat]
	_next_combat = (_next_combat + 1) % _combat_pool.size()
	player.stream = _streams[cue]
	player.global_position = world_position
	player.play()
	_mark_played(cue)


func _play_critical(cue: String, bus: String) -> void:
	_request(cue)
	if _reject(cue, true):
		return
	var player := _critical_pool[_next_critical]
	_next_critical = (_next_critical + 1) % _critical_pool.size()
	player.bus = bus
	player.stream = _streams[cue]
	player.play()
	_mark_played(cue)


func _request(cue: String) -> void:
	_requests[cue] += 1


func _reject(cue: String, retrigger_bounded: bool) -> bool:
	var now := Time.get_ticks_msec()
	if _silent or (retrigger_bounded and now - int(_last_played_at.get(cue, -MIN_RETRIGGER_MS)) < MIN_RETRIGGER_MS):
		_suppressed += 1
		return true
	return false


func _mark_played(cue: String) -> void:
	_played[cue] += 1
	_last_played_at[cue] = Time.get_ticks_msec()


func _configure_spatial(player: AudioStreamPlayer3D) -> void:
	player.max_distance = 24.0
	player.unit_size = 5.0


func _stop_all() -> void:
	_ambience.stop()
	_door.stop()
	for pool in [_system_pool, _combat_pool, _critical_pool]:
		for player in pool:
			player.stop()


func _load_pcm_wav(path: String) -> AudioStreamWAV:
	var bytes := FileAccess.get_file_as_bytes(path)
	var stream := AudioStreamWAV.new()
	stream.format = AudioStreamWAV.FORMAT_16_BITS
	stream.mix_rate = 48000
	stream.stereo = false
	stream.data = bytes.slice(44) if bytes.size() >= 44 else PackedByteArray()
	return stream
