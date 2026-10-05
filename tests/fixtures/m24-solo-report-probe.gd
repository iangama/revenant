extends SceneTree

const LOCAL_OBSERVATION_REPORT := preload("res://playtest/local_observation_report.gd")


func _initialize() -> void:
	var report := LOCAL_OBSERVATION_REPORT.new()
	var options := {
		"mode": "1",
		"participant_code": "PT-S001",
		"build_id": "m24-solo-endurance",
		"observation_consent": "1",
		"retention_consent": "1",
		"report_id": "00112233445566778899aabbccddeeff",
	}
	var preferences := {"guidance_mode": "Compact", "muted": true, "reduced_flash": true}
	var environment := {
		"os_family": "linux",
		"viewport_width": 2560,
		"viewport_height": 1440,
		"display_mode": "Windowed",
	}
	report.configure(options, preferences, environment, "user://m24-solo-report-probe")
	report.record_first("connect_requested", 10)
	report.record_first("connect_outcome", 20)
	report.record_first("first_movement_attempt", 30)
	report.record_first("first_attack_attempt", 40)
	report.record_first("settings_opened", 50)
	report.record_first("completion_observed", 60)
	report.record_first("quit_requested", 70)
	report.set_connection_outcome("connected")
	report.increment_cooldown_acknowledgement(65535)
	report.set_terminal_outcome("completed")
	report.finalize()
	var size := report.encoded_size()
	if size <= 0 or size > 16 * 1024:
		push_error("M24 solo report size is outside its bounded contract: %d" % size)
		quit(1)
		return
	print("M24 solo report probe encoded_bytes=%d ceiling_bytes=%d" % [size, 16 * 1024])
	if DirAccess.remove_absolute(ProjectSettings.globalize_path(report.final_path())) != OK:
		push_error("M24 solo report probe could not delete its exact synthetic report")
		quit(1)
		return
	quit(0)
