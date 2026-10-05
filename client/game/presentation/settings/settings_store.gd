extends RefCounted

const SETTINGS_PATH := "user://revenant-settings.cfg"
const BUS_NAMES := ["Ambience", "Effects", "Interface"]
const GUIDANCE_MODES := ["Full", "Compact", "Off"]
const DISPLAY_MODES := ["Windowed", "Fullscreen"]
const INPUT_BINDINGS := preload("res://input/input_bindings.gd")
const PRESENTATION_TRANSLATION := preload("res://presentation/localization/presentation_translation.gd")
static var _translation: Translation

var _windowed_size := Vector2i.ZERO


static func shutdown_translation() -> void:
	if _translation != null:
		TranslationServer.remove_translation(_translation)
		_translation = null


func defaults() -> Dictionary:
	return {
		"master_volume": 0.8,
		"ambience_volume": 0.7,
		"effects_volume": 0.85,
		"interface_volume": 0.75,
		"muted": false,
		"display_mode": "Windowed",
		"reduced_flash": false,
		"reduced_motion": false,
		"high_contrast": false,
		"captions": true,
		"ui_scale": 1.0,
		"language": "en",
		"bindings": INPUT_BINDINGS.defaults(),
		"guidance_mode": "Full",
	}


func load_settings(path := SETTINGS_PATH) -> Dictionary:
	var config := ConfigFile.new()
	if config.load(path) != OK:
		if config.load(path + ".bak") != OK:
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
	var pending := path + ".tmp"
	var error := config.save(pending)
	if error != OK:
		return error
	# Rotate only a readable primary. An invalid file must not replace the
	# last usable backup when recovering from a interrupted previous save.
	var previous := ConfigFile.new()
	if previous.load(path) == OK:
		error = DirAccess.rename_absolute(path, path + ".bak")
		if error != OK:
			DirAccess.remove_absolute(pending)
			return error
	error = DirAccess.rename_absolute(pending, path)
	if error != OK and not FileAccess.file_exists(path):
		DirAccess.copy_absolute(path + ".bak", path)
	return error


func sanitize(candidate: Dictionary) -> Dictionary:
	var result := defaults()
	for key in ["master_volume", "ambience_volume", "effects_volume", "interface_volume"]:
		var value = candidate.get(key, result[key])
		if (value is float or value is int) and is_finite(float(value)):
			result[key] = clampf(float(value), 0.0, 1.0)
	for key in ["muted", "reduced_flash", "reduced_motion", "high_contrast", "captions"]:
		var value = candidate.get(key, result[key])
		if value is bool:
			result[key] = value
	var display_mode = candidate.get("display_mode", result["display_mode"])
	if display_mode is String and display_mode in DISPLAY_MODES:
		result["display_mode"] = display_mode
	var guidance_mode = candidate.get("guidance_mode", result["guidance_mode"])
	if guidance_mode is String and guidance_mode in GUIDANCE_MODES:
		result["guidance_mode"] = guidance_mode
	var ui_scale: Variant = candidate.get("ui_scale", 1.0)
	if (ui_scale is float or ui_scale is int) and ui_scale in [1.0, 1.25, 1.5]:
		result["ui_scale"] = float(ui_scale)
	var language: Variant = candidate.get("language", "en")
	if language is String and language in ["en", "pt_BR", "pseudo"]:
		result["language"] = language
	result["bindings"] = INPUT_BINDINGS.sanitize(candidate.get("bindings", {}))
	return result


func apply(settings: Dictionary) -> Dictionary:
	var sanitized := sanitize(settings)
	INPUT_BINDINGS.apply(sanitized.bindings)
	if _translation == null:
		_translation = PRESENTATION_TRANSLATION.new()
		TranslationServer.add_translation(_translation)
	TranslationServer.set_locale("pt_BR" if sanitized.language == "pt_BR" else "en")
	TranslationServer.pseudolocalization_enabled = sanitized.language == "pseudo"
	_ensure_audio_buses()
	_set_bus_volume("Master", sanitized["master_volume"])
	_set_bus_volume("Ambience", sanitized["ambience_volume"])
	_set_bus_volume("Effects", sanitized["effects_volume"])
	_set_bus_volume("Interface", sanitized["interface_volume"])
	AudioServer.set_bus_mute(AudioServer.get_bus_index("Master"), sanitized["muted"])
	if DisplayServer.get_name() != "headless":
		if sanitized["display_mode"] == "Fullscreen":
			if DisplayServer.window_get_mode() == DisplayServer.WINDOW_MODE_WINDOWED:
				_windowed_size = DisplayServer.window_get_size()
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)
		else:
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
			if _windowed_size != Vector2i.ZERO:
				DisplayServer.window_set_size(_windowed_size)
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
