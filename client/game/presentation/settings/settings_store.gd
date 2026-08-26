extends RefCounted

const SETTINGS_PATH := "user://revenant-settings.cfg"
const BUS_NAMES := ["Ambience", "Effects", "Interface"]
const GUIDANCE_MODES := ["Full", "Compact", "Off"]
const DISPLAY_MODES := ["Windowed", "Fullscreen"]


func defaults() -> Dictionary:
	return {
		"master_volume": 0.8,
		"ambience_volume": 0.7,
		"effects_volume": 0.85,
		"interface_volume": 0.75,
		"muted": false,
		"display_mode": "Windowed",
		"reduced_flash": false,
		"guidance_mode": "Full",
	}


func load_settings(path := SETTINGS_PATH) -> Dictionary:
	var config := ConfigFile.new()
	if config.load(path) != OK:
		return defaults()
	var candidate := {}
	for key in defaults():
		candidate[key] = config.get_value("presentation", key, defaults()[key])
	return sanitize(candidate)


func save_settings(settings: Dictionary, path := SETTINGS_PATH) -> Error:
	var sanitized := sanitize(settings)
	var config := ConfigFile.new()
	for key in sanitized:
		config.set_value("presentation", key, sanitized[key])
	return config.save(path)


func sanitize(candidate: Dictionary) -> Dictionary:
	var result := defaults()
	for key in ["master_volume", "ambience_volume", "effects_volume", "interface_volume"]:
		var value = candidate.get(key, result[key])
		if value is float or value is int:
			result[key] = clampf(float(value), 0.0, 1.0)
	for key in ["muted", "reduced_flash"]:
		var value = candidate.get(key, result[key])
		if value is bool:
			result[key] = value
	var display_mode = candidate.get("display_mode", result["display_mode"])
	if display_mode is String and display_mode in DISPLAY_MODES:
		result["display_mode"] = display_mode
	var guidance_mode = candidate.get("guidance_mode", result["guidance_mode"])
	if guidance_mode is String and guidance_mode in GUIDANCE_MODES:
		result["guidance_mode"] = guidance_mode
	return result


func apply(settings: Dictionary) -> Dictionary:
	var sanitized := sanitize(settings)
	_ensure_audio_buses()
	_set_bus_volume("Master", sanitized["master_volume"])
	_set_bus_volume("Ambience", sanitized["ambience_volume"])
	_set_bus_volume("Effects", sanitized["effects_volume"])
	_set_bus_volume("Interface", sanitized["interface_volume"])
	AudioServer.set_bus_mute(AudioServer.get_bus_index("Master"), sanitized["muted"])
	if DisplayServer.get_name() != "headless":
		DisplayServer.window_set_mode(
			DisplayServer.WINDOW_MODE_FULLSCREEN
			if sanitized["display_mode"] == "Fullscreen"
			else DisplayServer.WINDOW_MODE_WINDOWED
		)
	return sanitized


func audio_state() -> Dictionary:
	var state := {"bus_count": AudioServer.bus_count, "buses": {}}
	for bus_name in ["Master"] + BUS_NAMES:
		var index := AudioServer.get_bus_index(bus_name)
		state["buses"][bus_name] = {
			"present": index >= 0,
			"muted": AudioServer.is_bus_mute(index) if index >= 0 else false,
			"volume": db_to_linear(AudioServer.get_bus_volume_db(index)) if index >= 0 else -1.0,
		}
	return state


func _ensure_audio_buses() -> void:
	for bus_name in BUS_NAMES:
		if AudioServer.get_bus_index(bus_name) >= 0:
			continue
		AudioServer.add_bus()
		var index := AudioServer.bus_count - 1
		AudioServer.set_bus_name(index, bus_name)
		AudioServer.set_bus_send(index, "Master")


func _set_bus_volume(bus_name: String, linear: float) -> void:
	var index := AudioServer.get_bus_index(bus_name)
	if index >= 0:
		AudioServer.set_bus_volume_db(index, linear_to_db(maxf(linear, 0.0001)))
