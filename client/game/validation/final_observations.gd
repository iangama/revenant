extends RefCounted

# Local review samples only; no telemetry, save changes or hardware certification.
var stages := {}
var _previous_usec := 0
var _capture: AudioEffectCapture
var _audio_index := -1
var _last_audio_ms := 0


func start() -> void:
	if _capture != null: return
	_capture = AudioEffectCapture.new()
	_capture.buffer_length = 0.25
	_audio_index = AudioServer.get_bus_effect_count(0)
	AudioServer.add_bus_effect(0, _capture)


func sample(game: Node, stage: String) -> void:
	var now := Time.get_ticks_usec()
	var frame_ms := float(now - _previous_usec) / 1000.0 if _previous_usec else 0.0
	_previous_usec = now
	if stage.is_empty() or not is_instance_valid(game): return
	if not stages.has(stage):
		stages[stage] = {"frames": 0, "frame_total_ms": 0.0, "frame_max_ms": 0.0, "memory_max_bytes": 0, "nodes_max": 0, "audio_peak": 0.0, "audio_samples": 0, "voices_max": 0, "audio_bytes_max": 0}
	var row: Dictionary = stages[stage]
	row.frames += 1
	row.frame_total_ms += frame_ms
	row.frame_max_ms = maxf(row.frame_max_ms, frame_ms)
	row.memory_max_bytes = maxi(row.memory_max_bytes, int(Performance.get_monitor(Performance.MEMORY_STATIC)))
	row.nodes_max = maxi(row.nodes_max, int(Performance.get_monitor(Performance.OBJECT_NODE_COUNT)))
	var director: Node = game.get("_audio_director")
	if director == null: return
	var audio: Dictionary = director.call("presentation_state")
	row.voices_max = maxi(row.voices_max, audio.active_voices)
	row.audio_bytes_max = maxi(row.audio_bytes_max, audio.decoded_bytes)
	if _capture != null and Time.get_ticks_msec() - _last_audio_ms >= 100:
		_last_audio_ms = Time.get_ticks_msec()
		var samples := _capture.get_buffer(_capture.get_frames_available())
		row.audio_samples += samples.size()
		for value in samples:
			row.audio_peak = maxf(row.audio_peak, maxf(absf(value.x), absf(value.y)))


func finish(directory: String, name: String) -> Dictionary:
	if _audio_index >= 0:
		AudioServer.remove_bus_effect(0, _audio_index)
		_audio_index = -1
	for row in stages.values():
		row["frame_mean_ms"] = row.frame_total_ms / maxi(1, row.frames)
	var report := {"renderer": RenderingServer.get_video_adapter_name(), "audio_driver": OS.get_environment("REVENANT_REVIEW_AUDIO_DRIVER"), "kind": "creator-operated local samples including loading; no hardware-wide performance claim", "stages": stages}
	var file := FileAccess.open(directory.path_join(name + ".json"), FileAccess.WRITE)
	if file != null: file.store_string(JSON.stringify(report, "  ") + "\n")
	return report
