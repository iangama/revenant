extends RefCounted

const LOCAL_OBSERVATION_REPORT := preload("res://playtest/local_observation_report.gd")
const REPORT_ID := "0123456789abcdef0123456789abcdef"


func validate() -> String:
	var validation_directory := "user://m24-playtest-validation"
	_remove_tree(validation_directory)
	var preferences := {"guidance_mode": "Compact", "muted": true, "reduced_flash": true}
	var environment := {
		"os_family": "linux",
		"viewport_width": 1280,
		"viewport_height": 720,
		"display_mode": "Windowed",
	}
	var disabled := LOCAL_OBSERVATION_REPORT.new()
	disabled.configure({}, preferences, environment, validation_directory)
	if disabled.is_active() or DirAccess.dir_exists_absolute(ProjectSettings.globalize_path(validation_directory)):
		return "M24 local observation report is not disabled by default"
	var malformed := LOCAL_OBSERVATION_REPORT.new()
	malformed.configure(_options("participant", true), preferences, environment, validation_directory)
	if malformed.is_active():
		return "M24 local observation accepts a malformed participant code"
	var no_consent := LOCAL_OBSERVATION_REPORT.new()
	var no_consent_options := _options("PT-A1B2", true)
	no_consent_options["observation_consent"] = "0"
	no_consent.configure(no_consent_options, preferences, environment, validation_directory)
	if no_consent.is_active():
		return "M24 local observation activates without explicit observation consent"
	var oversized_build := LOCAL_OBSERVATION_REPORT.new()
	var oversized_options := _options("PT-A1B2", true)
	oversized_options["build_id"] = "x".repeat(65)
	oversized_build.configure(oversized_options, preferences, environment, validation_directory)
	if oversized_build.is_active():
		return "M24 local observation accepts an unbounded build identifier"
	var report := LOCAL_OBSERVATION_REPORT.new()
	report.configure(_options("PT-A1B2", true), preferences, environment, validation_directory)
	if not report.is_active() or not FileAccess.file_exists(report.final_path()):
		return "M24 retained local report was not created atomically"
	var top_level_keys: Array = report.report().keys()
	top_level_keys.sort()
	var expected_keys := [
		"build_id", "connection_outcome", "consent", "cooldown_acknowledgement_count",
		"environment", "observed_at_ms", "participant_code", "preferences", "product_version",
		"report_id", "schema_version", "terminal_outcome",
	]
	expected_keys.sort()
	if top_level_keys != expected_keys:
		return "M24 local observation report does not enforce its top-level allow-list"
	report.record_first("connect_requested", 10)
	report.set_connection_outcome("transport_failure")
	report.set_terminal_outcome("failed")
	var failure_state: Dictionary = report.report()
	if failure_state.has("session_id") or failure_state.get("connection_outcome") != "transport_failure":
		return "M24 pre-session failure manufactures an authoritative session identifier"
	var completed := LOCAL_OBSERVATION_REPORT.new()
	var completed_options := _options("PT-C3D4", true)
	completed_options["report_id"] = "abcdef0123456789abcdef0123456789"
	completed.configure(completed_options, preferences, environment, validation_directory)
	completed.record_first("completion_observed", 42)
	completed.record_first("completion_observed", 99)
	completed.set_terminal_outcome("completed")
	var completion_state: Dictionary = completed.report()
	if completion_state["observed_at_ms"]["settings_opened"] != null:
		return "M24 completion fixture invents optional settings interaction"
	if completion_state["observed_at_ms"]["completion_observed"] != 42:
		return "M24 local observations are not first-occurrence only"
	completed.increment_cooldown_acknowledgement(65540)
	if completed.report().get("cooldown_acknowledgement_count") != 65535:
		return "M24 cooldown acknowledgement count does not saturate"
	if completed.encoded_size() > 16 * 1024:
		return "M24 local observation report exceeds the 16 KiB ceiling"
	if not completed.reconciliation_state(false).get("contradiction", false):
		return "M24 local completion contradiction is not retained for reconciliation"
	var declined := LOCAL_OBSERVATION_REPORT.new()
	var declined_options := _options("PT-E5F6", false)
	declined_options["report_id"] = "fedcba9876543210fedcba9876543210"
	declined.configure(declined_options, preferences, environment, validation_directory)
	if not FileAccess.file_exists(declined.temporary_path()) or FileAccess.file_exists(declined.final_path()):
		return "M24 declined retention is not isolated to a temporary report"
	declined.record_first("quit_requested", 7)
	declined.set_terminal_outcome("quit")
	declined.finalize()
	if FileAccess.file_exists(declined.temporary_path()) or FileAccess.file_exists(declined.final_path()):
		return "M24 declined retention report was not deleted at closeout"
	_remove_tree(validation_directory)
	return ""


func _options(participant_code: String, retain: bool) -> Dictionary:
	return {
		"mode": "1",
		"participant_code": participant_code,
		"build_id": "m24-validation",
		"observation_consent": "1",
		"retention_consent": "1" if retain else "0",
		"report_id": REPORT_ID,
	}


func _remove_tree(path: String) -> void:
	var absolute := ProjectSettings.globalize_path(path)
	if not DirAccess.dir_exists_absolute(absolute):
		return
	var directory := DirAccess.open(path)
	if directory != null:
		for file_name in directory.get_files():
			DirAccess.remove_absolute("%s/%s" % [absolute, file_name])
	DirAccess.remove_absolute(absolute)
