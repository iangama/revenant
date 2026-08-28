extends RefCounted

const ONBOARDING_CONTROLLER := preload("res://presentation/onboarding/onboarding_controller.gd")


func validate(fixtures: Dictionary) -> String:
	var tree: SceneTree = fixtures.tree
	var entry_shell: Control = fixtures.entry_shell
	var settings_store: RefCounted = fixtures.settings_store
	var settings_panel: Control = fixtures.settings_panel
	var onboarding: RefCounted = fixtures.onboarding
	var hud_canvas: CanvasLayer = fixtures.hud_canvas
	var status_label: Label = fixtures.status_label
	var controls_label: Label = fixtures.controls_label
	var presentation_polish: CanvasLayer = fixtures.presentation_polish
	var open_settings: Callable = fixtures.open_settings
	var apply_settings: Callable = fixtures.apply_settings
	var refresh_onboarding: Callable = fixtures.refresh_onboarding
	var save_capture: Callable = fixtures.save_capture
	var onboarding_validation := ONBOARDING_CONTROLLER.new()
	onboarding_validation.call("reset", "Full")
	onboarding_validation.call("note_local", "movement")
	onboarding_validation.call("note_local", "attack")
	var onboarding_state: Dictionary = onboarding_validation.call("presentation_state")
	if onboarding_state.get("step") != "Movement":
		return "M22 local onboarding attempts manufacture authoritative progress"
	onboarding_validation.call("confirm", "movement")
	onboarding_validation.call("confirm", "damage")
	onboarding_validation.call("confirm", "equipment")
	onboarding_validation.call("confirm", "door_objective")
	onboarding_validation.call("confirm", "warden_spawn")
	onboarding_validation.call("toggle")
	onboarding_validation.call("toggle")
	onboarding_validation.call("confirm", "completion")
	onboarding_state = onboarding_validation.call("presentation_state")
	if onboarding_state.get("step") != "Completion" or onboarding_state.get("dismissed", true):
		return "M22 onboarding does not follow confirmed evidence or revisitable guidance"
	var entry_state: Dictionary = entry_shell.call("presentation_state")
	if (
		entry_state.get("state") != "Entry"
		or not entry_state.get("username_valid", false)
		or not entry_state.get("connect_enabled", false)
		or not entry_state.get("mouse_captured", false)
		or not entry_shell.call("is_username_valid", "Echo.Runner-1")
		or entry_shell.call("is_username_valid", "bad user")
		or entry_shell.call("is_username_valid", "")
	):
		return "M22 entry shell does not expose a safe explicit connection state"
	var entry_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_ENTRY")
	if not entry_capture_path.is_empty():
		await tree.process_frame
		await tree.process_frame
		if await save_capture.call(entry_capture_path) != OK:
			return "M22 entry shell validation capture could not be saved"
	entry_shell.call("set_connection_state", "Connecting", "Validation")
	entry_state = entry_shell.call("presentation_state")
	if entry_state.get("state") != "Connecting" or entry_state.get("connect_enabled", true):
		return "M22 entry shell does not prevent duplicate connection actions"
	entry_shell.call("show_failure", "Validation failure")
	entry_state = entry_shell.call("presentation_state")
	if entry_state.get("state") != "Failed" or not entry_state.get("connect_enabled", false):
		return "M22 entry shell does not provide a keyboard-reachable retry state"
	var sanitized_settings: Dictionary = settings_store.call("sanitize", {
		"master_volume": 4.0,
		"ambience_volume": -2.0,
		"effects_volume": "invalid",
		"interface_volume": 0.35,
		"muted": "invalid",
		"display_mode": "Borderless",
		"reduced_flash": true,
		"guidance_mode": "Unknown",
	})
	if (
		sanitized_settings.get("master_volume") != 1.0
		or sanitized_settings.get("ambience_volume") != 0.0
		or sanitized_settings.get("effects_volume") != 0.85
		or sanitized_settings.get("interface_volume") != 0.35
		or sanitized_settings.get("muted") != false
		or sanitized_settings.get("display_mode") != "Windowed"
		or sanitized_settings.get("reduced_flash") != true
		or sanitized_settings.get("guidance_mode") != "Full"
	):
		return "M22 settings do not recover deterministic safe defaults"
	var validation_settings_path := "user://m22-settings-validation.cfg"
	var persisted_candidate := sanitized_settings.duplicate(true)
	persisted_candidate["guidance_mode"] = "Off"
	if settings_store.call("save_settings", persisted_candidate, validation_settings_path) != OK:
		return "M22 settings cannot persist local validated values"
	var persisted_settings: Dictionary = settings_store.call("load_settings", validation_settings_path)
	DirAccess.remove_absolute(ProjectSettings.globalize_path(validation_settings_path))
	if persisted_settings.get("guidance_mode") != "Off" or persisted_settings.get("master_volume") != 1.0:
		return "M22 settings persistence does not round-trip validated values"
	var settings_focus_source: Control = entry_shell.call("settings_focus_source")
	open_settings.call(settings_focus_source)
	var settings_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_SETTINGS")
	if not settings_capture_path.is_empty():
		await tree.process_frame
		await tree.process_frame
		if await save_capture.call(settings_capture_path) != OK:
			return "M22 settings validation capture could not be saved"
	var settings_state: Dictionary = settings_panel.call("presentation_state")
	if (
		not settings_state.get("visible", false)
		or settings_state.get("slider_count", 0) != 4
		or not settings_state.get("has_mute", false)
		or not settings_state.get("has_reduced_flash", false)
		or settings_state.get("display_options", 0) != 2
		or settings_state.get("guidance_options", 0) != 3
		or not settings_state.get("mouse_captured", false)
		or settings_state.get("focus_owner") != "Apply"
	):
		return "M22 settings panel is incomplete or not keyboard reachable"
	settings_panel.call("close_panel")
	entry_state = entry_shell.call("presentation_state")
	if entry_state.get("focus_owner") != "Settings":
		return "M22 settings do not restore focus to their invoking control"
	entry_shell.call("dismiss")
	hud_canvas.visible = true
	status_label.text = "RELAY READY  •  WAITING FOR ACTIVITY"
	controls_label.text = "WASD / ARROWS MOVE  •  AIM + CLICK / SPACE ATTACK  •  H HELP  •  ESC SETTINGS"
	refresh_onboarding.call()
	var onboarding_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_ONBOARDING")
	if not onboarding_capture_path.is_empty() and await save_capture.call(onboarding_capture_path) != OK:
		return "M22 onboarding validation capture could not be saved"
	apply_settings.call({
		"master_volume": 0.6,
		"ambience_volume": 0.5,
		"effects_volume": 0.7,
		"interface_volume": 0.4,
		"muted": true,
		"display_mode": "Windowed",
		"reduced_flash": true,
		"guidance_mode": "Compact",
	}, false)
	var audio_state: Dictionary = settings_store.call("audio_state")
	var buses: Dictionary = audio_state.get("buses", {})
	if (
		not buses.get("Master", {}).get("present", false)
		or not buses.get("Master", {}).get("muted", false)
		or not buses.get("Ambience", {}).get("present", false)
		or not buses.get("Effects", {}).get("present", false)
		or not buses.get("Interface", {}).get("present", false)
		or fixtures.guidance_mode.call() != "Compact"
		or not presentation_polish.call("presentation_state").get("reduced_flash", false)
	):
		return "M22 local settings do not apply bounded buses and accessibility state"
	apply_settings.call(settings_store.call("defaults"), false)
	return ""
