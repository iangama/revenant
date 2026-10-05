extends RefCounted

const LOGICAL_BOUNDS := Rect2(Vector2.ZERO, Vector2(1280, 720))
const INPUT_PANEL_BOUNDS := Rect2(894, 214, 362, 188)
const INVENTORY_PANEL_BOUNDS := Rect2(894, 422, 362, 184)
const ENTRY_FOCUS_NAMES := ["Username", "Connect", "Settings", "Quit"]
const SETTINGS_FOCUS_NAMES := [
	"MasterVolume",
	"AmbienceVolume",
	"EffectsVolume",
	"InterfaceVolume",
	"Mute",
	"ReducedFlash",
	"DisplayMode",
	"GuidanceMode",
	"Apply",
	"Cancel",
]


func validate(fixtures: Dictionary) -> String:
	if ProjectSettings.get_setting("display/window/stretch/mode") != "canvas_items":
		return "M24 display scaling is not anchored to the 1280x720 canvas"
	if ProjectSettings.get_setting("display/window/stretch/aspect") != "keep":
		return "M24 display scaling does not preserve the authored aspect ratio"

	var entry_shell: Control = fixtures.entry_shell
	var settings_panel: Control = fixtures.settings_panel
	var settings_store: RefCounted = fixtures.settings_store
	var initial_settings: Dictionary = fixtures.current_settings.call()
	var entry_state: Dictionary = entry_shell.call("presentation_state")
	var settings_state: Dictionary = settings_panel.call("presentation_state")
	if not _rect_inside(entry_state.get("panel_rect", Rect2()), LOGICAL_BOUNDS):
		return "M24 entry panel escapes the logical desktop canvas"
	if not _rect_inside(settings_state.get("panel_rect", Rect2()), LOGICAL_BOUNDS):
		return "M24 settings panel escapes the logical desktop canvas"
	if fixtures.input_log_label.visible and not _rect_inside(fixtures.input_log_label.get_rect(), INPUT_PANEL_BOUNDS):
		return "M24 input history escapes its logical HUD panel"
	if not _rect_inside(fixtures.inventory_label.get_rect(), fixtures.inventory_panel.get_rect()):
		return "M24 inventory text escapes its logical HUD panel"
	if not _rect_inside(fixtures.progression_label.get_rect(), fixtures.inventory_panel.get_rect()):
		return "M24 progression text escapes its logical HUD panel"
	if fixtures.inventory_label.get_rect().intersects(fixtures.progression_label.get_rect()):
		return "M24 inventory and progression text overlap"

	var environment_error := _validate_expected_environment(initial_settings)
	if not environment_error.is_empty():
		return environment_error
	var malformed_error := _validate_malformed_settings(settings_store)
	if not malformed_error.is_empty():
		return malformed_error
	var focus_error := await _validate_keyboard_focus(fixtures)
	if not focus_error.is_empty():
		return focus_error
	var preference_error := _validate_guidance_and_accessibility(fixtures, initial_settings)
	if not preference_error.is_empty():
		return preference_error
	var focus_recovery_error := _validate_focus_recovery(fixtures)
	if not focus_recovery_error.is_empty():
		return focus_recovery_error
	var mode_error := await _validate_mode_switch(fixtures, initial_settings)
	if not mode_error.is_empty():
		return mode_error

	fixtures.apply_settings.call(initial_settings, false)
	entry_shell.call("show_entry", "revenant-godot")
	await fixtures.tree.process_frame
	print("M24 display case validated: %dx%d %s guidance=%s muted=%s reduced_flash=%s" % [
		DisplayServer.window_get_size().x,
		DisplayServer.window_get_size().y,
		_display_mode(),
		initial_settings.get("guidance_mode", "Full"),
		initial_settings.get("muted", false),
		initial_settings.get("reduced_flash", false),
	])
	return ""


func _validate_expected_environment(settings: Dictionary) -> String:
	if DisplayServer.get_name() == "headless":
		return ""
	var expected_size := OS.get_environment("REVENANT_EXPECT_WINDOW_SIZE")
	if not expected_size.is_empty():
		var parts := expected_size.to_lower().split("x", false, 1)
		if parts.size() != 2:
			return "M24 expected window size is malformed"
		var expected := Vector2i(int(parts[0]), int(parts[1]))
		if DisplayServer.window_get_size() != expected:
			return "M24 window size mismatch: expected %s, got %s" % [expected, DisplayServer.window_get_size()]
	var expectations := {
		"guidance_mode": OS.get_environment("REVENANT_EXPECT_GUIDANCE_MODE"),
		"display_mode": OS.get_environment("REVENANT_EXPECT_DISPLAY_MODE"),
	}
	for key in expectations:
		if not expectations[key].is_empty() and settings.get(key) != expectations[key]:
			return "M24 persisted %s mismatch" % key
	for pair in [
		["muted", "REVENANT_EXPECT_MUTED"],
		["reduced_flash", "REVENANT_EXPECT_REDUCED_FLASH"],
	]:
		var value := OS.get_environment(pair[1])
		if not value.is_empty() and settings.get(pair[0]) != (value == "1"):
			return "M24 persisted %s mismatch" % pair[0]
	var expected_mode := OS.get_environment("REVENANT_EXPECT_DISPLAY_MODE")
	if not expected_mode.is_empty() and _display_mode() != expected_mode:
		return "M24 applied display mode mismatch: expected %s, got %s" % [expected_mode, _display_mode()]
	return ""


func _validate_malformed_settings(settings_store: RefCounted) -> String:
	if OS.get_environment("REVENANT_TEST_MALFORMED_SETTINGS") != "1":
		return ""
	var malformed_path := "user://m24-malformed-settings.cfg"
	var file := FileAccess.open(malformed_path, FileAccess.WRITE)
	if file == null:
		return "M24 malformed settings fixture could not be created"
	file.store_string("[presentation\nmaster_volume=this is not a ConfigFile")
	file.close()
	var recovered: Dictionary = settings_store.call("load_settings", malformed_path)
	DirAccess.remove_absolute(ProjectSettings.globalize_path(malformed_path))
	if recovered != settings_store.call("defaults"):
		return "M24 malformed settings do not recover to deterministic defaults"
	return ""


func _validate_keyboard_focus(fixtures: Dictionary) -> String:
	var entry_shell: Control = fixtures.entry_shell
	var settings_panel: Control = fixtures.settings_panel
	entry_shell.call("show_entry", "revenant-godot")
	await fixtures.tree.process_frame
	var entry_focus := _focus_cycle(entry_shell.call("username_control"), ENTRY_FOCUS_NAMES.size() + 2)
	if not _contains_all(entry_focus, ENTRY_FOCUS_NAMES):
		return "M24 entry controls are not all reachable through forward focus navigation"

	fixtures.open_settings.call(entry_shell.call("settings_focus_source"))
	await fixtures.tree.process_frame
	var settings_focus: Array[String] = []
	var tabs: TabContainer = settings_panel.get("_tabs")
	for index in tabs.get_tab_count():
		tabs.current_tab = index
		await fixtures.tree.process_frame
		for control_name in _focus_cycle(settings_panel.call("focus_start"), 64):
			if control_name not in settings_focus:
				settings_focus.append(control_name)
	tabs.current_tab = 0
	if not _contains_all(settings_focus, SETTINGS_FOCUS_NAMES):
		return "Settings controls are not all reachable through forward focus navigation: %s" % [settings_focus]
	var escape := InputEventKey.new()
	escape.keycode = KEY_ESCAPE
	escape.pressed = true
	Input.parse_input_event(escape)
	await fixtures.tree.process_frame
	escape.pressed = false
	Input.parse_input_event(escape)
	var state: Dictionary = settings_panel.call("presentation_state")
	if state.get("visible", true):
		return "M24 Escape does not close the settings overlay"
	if entry_shell.call("presentation_state").get("focus_owner") != "Settings":
		return "M24 settings Escape does not restore entry focus"
	return ""


func _validate_guidance_and_accessibility(fixtures: Dictionary, initial_settings: Dictionary) -> String:
	var onboarding: RefCounted = fixtures.onboarding
	for mode in ["Full", "Compact", "Off"]:
		onboarding.call("reset", mode)
		var guidance: Dictionary = onboarding.call("guidance")
		if mode == "Full" and (not guidance.get("visible", false) or guidance.get("compact", true)):
			return "M24 Full guidance is not visible and expanded"
		if mode == "Compact" and (not guidance.get("visible", false) or not guidance.get("compact", false)):
			return "M24 Compact guidance is not visible and concise"
		if mode == "Off" and guidance.get("visible", true):
			return "M24 Off guidance remains visible"
	var combined := initial_settings.duplicate(true)
	combined["muted"] = true
	combined["reduced_flash"] = true
	combined["guidance_mode"] = "Compact"
	fixtures.apply_settings.call(combined, false)
	var master: Dictionary = fixtures.settings_store.call("audio_state").get("buses", {}).get("Master", {})
	if not master.get("muted", false):
		return "M24 combined accessibility path does not mute Master"
	if not fixtures.presentation_polish.call("presentation_state").get("reduced_flash", false):
		return "M24 combined accessibility path does not preserve reduced flash"
	if fixtures.guidance_mode.call() != "Compact":
		return "M24 combined accessibility path does not apply Compact guidance"
	if not fixtures.status_label.visible or not fixtures.objective_label.visible or not fixtures.action_grid.visible:
		return "M24 guidance preferences hide critical authoritative status"
	fixtures.apply_settings.call(initial_settings, false)
	onboarding.call("reset", initial_settings.get("guidance_mode", "Full"))
	return ""


func _validate_focus_recovery(fixtures: Dictionary) -> String:
	fixtures.player_intents.call("set_ui_movement", Vector2.RIGHT)
	fixtures.root.notification(MainLoop.NOTIFICATION_APPLICATION_FOCUS_OUT)
	if fixtures.player_intents.call("presentation_state").get("ui_movement") != Vector2.ZERO:
		return "M24 focus loss leaves on-screen movement latched"
	fixtures.root.notification(MainLoop.NOTIFICATION_APPLICATION_FOCUS_IN)
	var input_text: String = fixtures.input_log_label.text
	if "WINDOW FOCUS LOST" not in input_text or "WINDOW FOCUS RESTORED" not in input_text:
		return "M24 focus loss and recovery are not visibly acknowledged"
	return ""


func _validate_mode_switch(fixtures: Dictionary, initial_settings: Dictionary) -> String:
	if DisplayServer.get_name() == "headless" or OS.get_environment("REVENANT_TEST_DISPLAY_SWITCH") != "1":
		return ""
	var alternate := initial_settings.duplicate(true)
	alternate["display_mode"] = "Windowed" if initial_settings.get("display_mode") == "Fullscreen" else "Fullscreen"
	fixtures.apply_settings.call(alternate, false)
	await fixtures.tree.process_frame
	await fixtures.tree.process_frame
	if _display_mode() != alternate["display_mode"]:
		return "M24 display mode does not switch to %s" % alternate["display_mode"]
	fixtures.apply_settings.call(initial_settings, false)
	await fixtures.tree.process_frame
	await fixtures.tree.process_frame
	if _display_mode() != initial_settings.get("display_mode"):
		return "M24 display mode does not restore to %s" % initial_settings.get("display_mode")
	return ""


func _focus_cycle(start: Control, limit: int) -> Array[String]:
	var names: Array[String] = []
	var current := start
	for _index in range(limit):
		if current == null:
			break
		var current_name := str(current.name)
		if current_name in names:
			break
		names.append(current_name)
		current = current.find_next_valid_focus()
	return names


func _contains_all(actual: Array[String], expected: Array) -> bool:
	for name in expected:
		if name not in actual:
			return false
	return true


func _rect_inside(candidate: Rect2, bounds: Rect2) -> bool:
	return candidate.size.x > 0.0 and candidate.size.y > 0.0 and bounds.encloses(candidate)


func _display_mode() -> String:
	return "Fullscreen" if DisplayServer.window_get_mode() == DisplayServer.WINDOW_MODE_FULLSCREEN else "Windowed"
