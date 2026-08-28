extends RefCounted


func validate(audio_director: Node3D, door: Node3D) -> String:
	var initial_audio_state: Dictionary = audio_director.call("presentation_state")
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
		return "M22 audio foundation is not fixed, bounded or loop-ready: %s" % initial_audio_state
	audio_director.call("set_silent", true)
	audio_director.call("play_system_ready")
	audio_director.call("apply_door_state", false, Vector3.ZERO)
	audio_director.call("apply_door_state", true, Vector3.ZERO)
	audio_director.call("play_cooldown_acknowledgement")
	audio_director.call("play_confirmed_attack", "pulse_rifle", Vector3.ZERO)
	audio_director.call("play_enemy_presence", "warden", Vector3.ZERO)
	audio_director.call("play_player_damage")
	audio_director.call("play_completion")
	var silent_audio_state: Dictionary = audio_director.call("presentation_state")
	if (
		silent_audio_state.get("active_voices") != 0
		or silent_audio_state.get("suppressed", 0) < 7
		or silent_audio_state.get("requests", {}).get("door_unlock") != 1
	):
		return "M22 silent mode permits audible or queued cues"
	audio_director.call("set_silent", false)
	audio_director.call("play_system_ready")
	audio_director.call("apply_door_state", true, door.global_position)
	audio_director.call("play_cooldown_acknowledgement")
	var local_ack_state: Dictionary = audio_director.call("presentation_state")
	if local_ack_state.get("played", {}).get("cooldown") != 1 or local_ack_state.get("played", {}).get("pulse_rifle") != 0:
		return "M22 local attack acknowledgement manufactures a confirmed combat cue"
	audio_director.call("play_confirmed_move", Vector3.ZERO)
	audio_director.call("play_confirmed_attack", "pulse_rifle", Vector3.ZERO)
	audio_director.call("play_confirmed_attack", "arc_sidearm", Vector3.ZERO)
	audio_director.call("play_enemy_presence", "relay-drone", Vector3.ZERO)
	audio_director.call("play_enemy_presence", "warden", Vector3.ZERO)
	audio_director.call("play_confirmed_impact", Vector3.ZERO)
	audio_director.call("play_confirmed_defeat", Vector3.ZERO)
	audio_director.call("play_player_damage")
	audio_director.call("play_completion")
	if audio_director.call("presentation_state").get("active_voices", 0) > 14:
		return "M22 audio presentation exceeds its voice budget"
	return ""


func measure(tree: SceneTree, audio_director: Node3D, operator: Node3D, drone: Node3D, warden: Node3D) -> Dictionary:
	var master_bus := AudioServer.get_bus_index("Master")
	var effect_index := AudioServer.get_bus_effect_count(master_bus)
	var capture := AudioEffectCapture.new()
	AudioServer.add_bus_effect(master_bus, capture, effect_index)
	await tree.process_frame
	capture.clear_buffer()
	audio_director.call("play_confirmed_move", operator.global_position)
	audio_director.call("play_confirmed_attack", "pulse_rifle", operator.global_position)
	audio_director.call("play_enemy_presence", "relay-drone", drone.global_position)
	audio_director.call("play_enemy_presence", "warden", warden.global_position)
	audio_director.call("play_confirmed_impact", drone.global_position)
	audio_director.call("play_player_damage")
	audio_director.call("play_completion")
	var frame_times_ms: Array[float] = []
	var peak_voices := 0
	for _frame in range(30):
		var started_at := Time.get_ticks_usec()
		await tree.process_frame
		frame_times_ms.append(float(Time.get_ticks_usec() - started_at) / 1000.0)
		peak_voices = maxi(peak_voices, int(audio_director.call("presentation_state").get("active_voices", 0)))
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


func validate_measurement(measurement: Dictionary) -> String:
	if (
		measurement.get("captured_frames", 0) <= 0
		or measurement.get("peak_dbfs", 0.0) > -3.0
		or measurement.get("peak_voices", 0) > 14
	):
		return "M22 graphical presentation exceeds its measured output or voice budget: %s" % measurement
	return ""
