extends RefCounted

const PRODUCT_VERSION := "0.2.0"
const SCHEMA_VERSION := 1
const MAX_ENCODED_BYTES := 16 * 1024
const MAX_COOLDOWN_ACKNOWLEDGEMENTS := 65535
const DEFAULT_STORAGE_DIRECTORY := "user://playtest"
const OBSERVATION_KEYS := [
	"entry_ready",
	"connect_requested",
	"connect_outcome",
	"first_movement_attempt",
	"first_attack_attempt",
	"settings_opened",
	"completion_observed",
	"disconnect_observed",
	"quit_requested",
]
const CONNECTION_OUTCOMES := [
	"not_attempted",
	"connected",
	"rejected",
	"timeout",
	"transport_failure",
	"session_unavailable",
]
const TERMINAL_OUTCOMES := ["running", "completed", "failed", "disconnected", "quit"]

var _active := false
var _diagnostic := "playtest observation disabled"
var _retention := false
var _started_at_ms := 0
var _storage_directory := DEFAULT_STORAGE_DIRECTORY
var _report := {}


func configure_from_environment(preferences: Dictionary, viewport_size: Vector2i) -> void:
	configure({
		"mode": OS.get_environment("REVENANT_PLAYTEST_MODE"),
		"participant_code": OS.get_environment("REVENANT_PLAYTEST_PARTICIPANT"),
		"build_id": OS.get_environment("REVENANT_PLAYTEST_BUILD_ID"),
		"observation_consent": OS.get_environment("REVENANT_PLAYTEST_OBSERVATION_CONSENT"),
		"retention_consent": OS.get_environment("REVENANT_PLAYTEST_RETENTION_CONSENT"),
	}, preferences, _environment(viewport_size))


func configure(options: Dictionary, preferences: Dictionary, environment: Dictionary, storage_directory := DEFAULT_STORAGE_DIRECTORY) -> void:
	_active = false
	_report = {}
	_storage_directory = storage_directory
	if options.get("mode", "") != "1":
		_diagnostic = "playtest observation disabled: mode is not active"
		return
	var participant_code := str(options.get("participant_code", ""))
	if not _valid_participant_code(participant_code):
		_diagnostic = "playtest observation disabled: participant code must match PT-[A-Z0-9]{4}"
		return
	var build_id := str(options.get("build_id", ""))
	if not _valid_build_id(build_id):
		_diagnostic = "playtest observation disabled: build ID is missing or invalid"
		return
	if options.get("observation_consent", "") != "1":
		_diagnostic = "playtest observation disabled: observation consent is not confirmed"
		return
	_retention = options.get("retention_consent", "") == "1"
	var report_id := str(options.get("report_id", ""))
	if report_id.is_empty():
		report_id = _random_report_id()
	if not _valid_report_id(report_id):
		_diagnostic = "playtest observation disabled: report ID generation failed"
		return
	_started_at_ms = Time.get_ticks_msec()
	var observed_at := {}
	for key in OBSERVATION_KEYS:
		observed_at[key] = null
	observed_at["entry_ready"] = 0
	_report = {
		"schema_version": SCHEMA_VERSION,
		"report_id": report_id,
		"participant_code": participant_code,
		"product_version": PRODUCT_VERSION,
		"build_id": build_id,
		"environment": _sanitize_environment(environment),
		"preferences": _sanitize_preferences(preferences),
		"observed_at_ms": observed_at,
		"connection_outcome": "not_attempted",
		"terminal_outcome": "running",
		"cooldown_acknowledgement_count": 0,
		"consent": {"observation": true, "retention": _retention},
	}
	_active = true
	_diagnostic = "playtest observation active; local retention %s" % ("confirmed" if _retention else "declined")
	var persist_error := persist()
	if persist_error != OK:
		_active = false
		_diagnostic = "playtest observation disabled: local report could not be created"


func is_active() -> bool:
	return _active


func diagnostic() -> String:
	return _diagnostic


func report() -> Dictionary:
	return _report.duplicate(true)


func record_first(key: String, elapsed_override := -1) -> Error:
	if not _active or key not in OBSERVATION_KEYS:
		return ERR_UNAVAILABLE
	if _report["observed_at_ms"].get(key) != null:
		return OK
	_report["observed_at_ms"][key] = maxi(0, elapsed_override if elapsed_override >= 0 else Time.get_ticks_msec() - _started_at_ms)
	return persist()


func set_connection_outcome(outcome: String) -> Error:
	if not _active or outcome not in CONNECTION_OUTCOMES:
		return ERR_INVALID_PARAMETER
	_report["connection_outcome"] = outcome
	if outcome != "not_attempted":
		record_first("connect_outcome")
	return persist()


func set_terminal_outcome(outcome: String) -> Error:
	if not _active or outcome not in TERMINAL_OUTCOMES:
		return ERR_INVALID_PARAMETER
	_report["terminal_outcome"] = outcome
	return persist()


func set_session_id(session_id: String) -> Error:
	if not _active or session_id.is_empty() or not _canonical_session_id(session_id):
		return ERR_INVALID_PARAMETER
	_report["session_id"] = session_id
	return persist()


func update_preferences(preferences: Dictionary) -> Error:
	if not _active:
		return ERR_UNAVAILABLE
	_report["preferences"] = _sanitize_preferences(preferences)
	return persist()


func increment_cooldown_acknowledgement(amount := 1) -> Error:
	if not _active:
		return ERR_UNAVAILABLE
	_report["cooldown_acknowledgement_count"] = mini(
		MAX_COOLDOWN_ACKNOWLEDGEMENTS,
		int(_report["cooldown_acknowledgement_count"]) + maxi(0, amount)
	)
	return persist()


func reconciliation_state(authoritative_completed: bool) -> Dictionary:
	var local_completed: bool = (
		_report.get("terminal_outcome") == "completed"
		or _report.get("observed_at_ms", {}).get("completion_observed") != null
	)
	return {
		"local_completed": local_completed,
		"authoritative_completed": authoritative_completed,
		"contradiction": local_completed != authoritative_completed,
	}


func encoded_size() -> int:
	return JSON.stringify(_report).to_utf8_buffer().size()


func persist() -> Error:
	if not _active:
		return ERR_UNAVAILABLE
	var encoded := JSON.stringify(_report)
	if encoded.to_utf8_buffer().size() > MAX_ENCODED_BYTES:
		return ERR_FILE_CANT_WRITE
	var directory_error := DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(_storage_directory))
	if directory_error != OK:
		return directory_error
	var temporary_path := _temporary_path()
	var file := FileAccess.open(temporary_path, FileAccess.WRITE)
	if file == null:
		return FileAccess.get_open_error()
	file.store_string(encoded)
	file.flush()
	file.close()
	if not _retention:
		return OK
	var rename_error := DirAccess.rename_absolute(temporary_path, _final_path())
	if rename_error == OK:
		return OK
	DirAccess.remove_absolute(_final_path())
	return DirAccess.rename_absolute(temporary_path, _final_path())


func finalize() -> Error:
	if not _active:
		return OK
	if not _retention:
		DirAccess.remove_absolute(_temporary_path())
		DirAccess.remove_absolute(_final_path())
		return OK
	return persist()


func temporary_path() -> String:
	return _temporary_path()


func final_path() -> String:
	return _final_path()


func _temporary_path() -> String:
	return "%s/m24-%s.json.tmp" % [_storage_directory, _report.get("report_id", "invalid")]


func _final_path() -> String:
	return "%s/m24-%s.json" % [_storage_directory, _report.get("report_id", "invalid")]


func _random_report_id() -> String:
	return Crypto.new().generate_random_bytes(16).hex_encode()


func _valid_participant_code(value: String) -> bool:
	if value.length() != 7 or not value.begins_with("PT-"):
		return false
	for character in value.substr(3):
		if not (character >= "A" and character <= "Z") and not (character >= "0" and character <= "9"):
			return false
	return true


func _valid_build_id(value: String) -> bool:
	if value.is_empty() or value.length() > 64:
		return false
	for character in value:
		if not _ascii_alphanumeric(character) and character not in ["-", "_", "."]:
			return false
	return true


func _valid_report_id(value: String) -> bool:
	if value.length() != 32:
		return false
	for character in value:
		if character not in "0123456789abcdef":
			return false
	return true


func _canonical_session_id(value: String) -> bool:
	for character in value:
		if not _ascii_alphanumeric(character) and character != "-":
			return false
	return true


func _ascii_alphanumeric(character: String) -> bool:
	return (
		(character >= "A" and character <= "Z")
		or (character >= "a" and character <= "z")
		or (character >= "0" and character <= "9")
	)


func _sanitize_preferences(candidate: Dictionary) -> Dictionary:
	var guidance_mode := str(candidate.get("guidance_mode", "Full"))
	if guidance_mode not in ["Full", "Compact", "Off"]:
		guidance_mode = "Full"
	return {
		"guidance_mode": guidance_mode,
		"muted": candidate.get("muted", false) == true,
		"reduced_flash": candidate.get("reduced_flash", false) == true,
	}


func _sanitize_environment(candidate: Dictionary) -> Dictionary:
	var os_family := str(candidate.get("os_family", "other"))
	if os_family not in ["windows", "linux", "macos", "other"]:
		os_family = "other"
	var display_mode := str(candidate.get("display_mode", "Windowed"))
	if display_mode not in ["Windowed", "Fullscreen"]:
		display_mode = "Windowed"
	return {
		"os_family": os_family,
		"viewport_width": clampi(int(candidate.get("viewport_width", 0)), 0, 16384),
		"viewport_height": clampi(int(candidate.get("viewport_height", 0)), 0, 16384),
		"display_mode": display_mode,
	}


func _environment(viewport_size: Vector2i) -> Dictionary:
	var os_name := OS.get_name().to_lower()
	var os_family := "other"
	if "windows" in os_name:
		os_family = "windows"
	elif "linux" in os_name:
		os_family = "linux"
	elif "macos" in os_name:
		os_family = "macos"
	return {
		"os_family": os_family,
		"viewport_width": viewport_size.x,
		"viewport_height": viewport_size.y,
		"display_mode": "Fullscreen" if DisplayServer.window_get_mode() == DisplayServer.WINDOW_MODE_FULLSCREEN else "Windowed",
	}
