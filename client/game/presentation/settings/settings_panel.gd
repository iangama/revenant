extends Control

signal settings_applied(settings: Dictionary)
signal closed

const GRAPHITE := Color("10151d")
const BLUE_PETROL := Color("12313a")
const CYAN := Color("35d0d0")
const AMBER := Color("f5a524")
const NEUTRAL := Color("a9b8cc")

var _settings := {}
var _focus_return: Control
var _sliders := {}
var _mute: CheckButton
var _reduced_flash: CheckButton
var _display: OptionButton
var _guidance: OptionButton
var _apply_button: Button
var _cancel_button: Button


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build_panel()
	visible = false


func open(settings: Dictionary, focus_return: Control = null) -> void:
	_settings = settings.duplicate(true)
	_focus_return = focus_return
	_sync_controls()
	visible = true
	_apply_button.grab_focus()


func close_panel() -> void:
	visible = false
	if is_instance_valid(_focus_return):
		_focus_return.grab_focus()
	closed.emit()


func presentation_state() -> Dictionary:
	return {
		"visible": visible,
		"slider_count": _sliders.size(),
		"has_mute": _mute != null,
		"has_reduced_flash": _reduced_flash != null,
		"display_options": _display.item_count,
		"guidance_options": _guidance.item_count,
		"focus_owner": get_viewport().gui_get_focus_owner().name if get_viewport().gui_get_focus_owner() != null else "",
		"mouse_captured": mouse_filter == Control.MOUSE_FILTER_STOP,
	}


func _unhandled_key_input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_ESCAPE:
		get_viewport().set_input_as_handled()
		close_panel()


func _build_panel() -> void:
	var backdrop := ColorRect.new()
	backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	backdrop.color = Color(GRAPHITE, 0.88)
	backdrop.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(backdrop)

	var panel := Panel.new()
	panel.position = Vector2(300, 48)
	panel.size = Vector2(680, 624)
	var style := StyleBoxFlat.new()
	style.bg_color = Color(BLUE_PETROL, 0.98)
	style.border_color = Color(AMBER, 0.9)
	style.set_border_width_all(1)
	style.set_corner_radius_all(4)
	panel.add_theme_stylebox_override("panel", style)
	add_child(panel)

	var title := _label(panel, "PRESENTATION SETTINGS", Vector2(38, 28), 26, Color.WHITE)
	title.name = "SettingsTitle"
	_label(panel, "LOCAL ONLY  •  NEVER SENT TO THE SERVER", Vector2(40, 66), 13, NEUTRAL)

	var y := 108.0
	for definition in [
		["master_volume", "MASTER", CYAN],
		["ambience_volume", "AMBIENCE", Color("35d0ba")],
		["effects_volume", "EFFECTS", Color("d93678")],
		["interface_volume", "INTERFACE", AMBER],
	]:
		_label(panel, definition[1], Vector2(40, y), 14, definition[2])
		var slider := HSlider.new()
		slider.name = definition[1].to_pascal_case() + "Volume"
		slider.position = Vector2(190, y - 6)
		slider.size = Vector2(430, 28)
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.05
		panel.add_child(slider)
		_sliders[definition[0]] = slider
		y += 50.0

	_mute = CheckButton.new()
	_mute.name = "Mute"
	_mute.text = "MUTE ALL AUDIO"
	_mute.position = Vector2(40, 308)
	_mute.size = Vector2(270, 40)
	panel.add_child(_mute)

	_reduced_flash = CheckButton.new()
	_reduced_flash.name = "ReducedFlash"
	_reduced_flash.text = "REDUCED FLASH"
	_reduced_flash.position = Vector2(340, 308)
	_reduced_flash.size = Vector2(280, 40)
	panel.add_child(_reduced_flash)

	_label(panel, "DISPLAY", Vector2(40, 374), 14, AMBER)
	_display = OptionButton.new()
	_display.name = "DisplayMode"
	_display.position = Vector2(190, 364)
	_display.size = Vector2(180, 42)
	_display.add_item("Windowed")
	_display.add_item("Fullscreen")
	panel.add_child(_display)

	_label(panel, "GUIDANCE", Vector2(390, 374), 14, CYAN)
	_guidance = OptionButton.new()
	_guidance.name = "GuidanceMode"
	_guidance.position = Vector2(500, 364)
	_guidance.size = Vector2(120, 42)
	_guidance.add_item("Full")
	_guidance.add_item("Compact")
	_guidance.add_item("Off")
	panel.add_child(_guidance)

	var note := _label(panel, "Volumes and mute apply immediately to the bounded audio layer.\nReduced Flash preserves text and state while lowering peripheral pulses.", Vector2(40, 430), 14, NEUTRAL)
	note.size = Vector2(580, 58)
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	_apply_button = _button(panel, "APPLY", "Apply", Vector2(40, 520), Vector2(260, 54), CYAN)
	_apply_button.pressed.connect(_apply)
	_cancel_button = _button(panel, "CANCEL", "Cancel", Vector2(320, 520), Vector2(260, 54), NEUTRAL)
	_cancel_button.pressed.connect(close_panel)
	_label(panel, "TAB  NAVIGATE    •    ENTER  SELECT    •    ESC  CANCEL", Vector2(40, 590), 12, NEUTRAL)


func _sync_controls() -> void:
	for key in _sliders:
		_sliders[key].value = _settings.get(key, 0.0)
	_mute.button_pressed = _settings.get("muted", false)
	_reduced_flash.button_pressed = _settings.get("reduced_flash", false)
	_display.select(1 if _settings.get("display_mode") == "Fullscreen" else 0)
	_guidance.select(["Full", "Compact", "Off"].find(_settings.get("guidance_mode", "Full")))


func _apply() -> void:
	for key in _sliders:
		_settings[key] = _sliders[key].value
	_settings["muted"] = _mute.button_pressed
	_settings["reduced_flash"] = _reduced_flash.button_pressed
	_settings["display_mode"] = _display.get_item_text(_display.selected)
	_settings["guidance_mode"] = _guidance.get_item_text(_guidance.selected)
	settings_applied.emit(_settings.duplicate(true))
	close_panel()


func _label(parent: Node, text: String, position: Vector2, font_size: int, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.position = position
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", color)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	parent.add_child(label)
	return label


func _button(parent: Node, text: String, name: String, position: Vector2, size: Vector2, accent: Color) -> Button:
	var button := Button.new()
	button.text = text
	button.name = name
	button.position = position
	button.size = size
	button.add_theme_font_size_override("font_size", 16)
	var normal := StyleBoxFlat.new()
	normal.bg_color = Color(GRAPHITE, 0.72)
	normal.border_color = accent
	normal.set_border_width_all(1)
	normal.set_corner_radius_all(3)
	button.add_theme_stylebox_override("normal", normal)
	button.add_theme_stylebox_override("focus", normal.duplicate())
	parent.add_child(button)
	return button
