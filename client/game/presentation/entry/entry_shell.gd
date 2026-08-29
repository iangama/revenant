extends Control

signal connect_requested(username: String)
signal settings_requested(focus_source: Control)
signal quit_requested

const GRAPHITE := Color("10151d")
const BLUE_PETROL := Color("12313a")
const CYAN := Color("35d0d0")
const AMBER := Color("f5a524")
const DAMAGE_RED := Color("e8505b")
const NEUTRAL := Color("a9b8cc")
const MAX_USERNAME_LENGTH := 32

var _state := "Entry"
var _endpoint := "127.0.0.1:7000"
var _panel: Panel
var _username: LineEdit
var _endpoint_label: Label
var _state_label: Label
var _detail_label: Label
var _validation_label: Label
var _connect_button: Button
var _settings_button: Button
var _quit_button: Button


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build_shell()
	show_entry("revenant-godot")


func configure_endpoint(host: String, port: int) -> void:
	_endpoint = "%s:%d" % [host, port]
	if _endpoint_label != null:
		_endpoint_label.text = "LOCAL RELAY  •  %s" % _endpoint


func show_entry(username: String) -> void:
	visible = true
	_username.text = username
	_set_state("Entry", "Review your local identity, then connect to the relay.", false)
	_validate_username()
	_username.grab_focus()
	_username.select_all()


func set_connection_state(state: String, detail: String) -> void:
	visible = true
	_set_state(state, detail, true)


func show_failure(detail: String) -> void:
	visible = true
	_set_state("Failed", detail, false)
	_connect_button.text = "RETRY CONNECTION"
	_connect_button.grab_focus()


func dismiss() -> void:
	visible = false


func is_username_valid(username: String) -> bool:
	if username.is_empty() or username.length() > MAX_USERNAME_LENGTH:
		return false
	for character in username:
		if not (
			character.to_ascii_buffer()[0] >= 48 and character.to_ascii_buffer()[0] <= 57
			or character.to_ascii_buffer()[0] >= 65 and character.to_ascii_buffer()[0] <= 90
			or character.to_ascii_buffer()[0] >= 97 and character.to_ascii_buffer()[0] <= 122
			or character in ["_", ".", "-"]
		):
			return false
	return true


func presentation_state() -> Dictionary:
	return {
		"state": _state,
		"endpoint": _endpoint,
		"username": _username.text,
		"username_valid": is_username_valid(_username.text),
		"connect_enabled": not _connect_button.disabled,
		"connect_text": _connect_button.text,
		"settings_enabled": not _settings_button.disabled,
		"quit_enabled": not _quit_button.disabled,
		"focus_owner": get_viewport().gui_get_focus_owner().name if get_viewport().gui_get_focus_owner() != null else "",
		"mouse_captured": mouse_filter == Control.MOUSE_FILTER_STOP,
	}


func settings_focus_source() -> Control:
	return _settings_button


func _build_shell() -> void:
	var backdrop := ColorRect.new()
	backdrop.name = "Backdrop"
	backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	backdrop.color = Color(GRAPHITE, 0.96)
	backdrop.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(backdrop)

	_panel = Panel.new()
	_panel.name = "EntryPanel"
	_panel.position = Vector2(330, 104)
	_panel.size = Vector2(620, 512)
	var panel_style := StyleBoxFlat.new()
	panel_style.bg_color = Color(BLUE_PETROL, 0.94)
	panel_style.border_color = Color(CYAN, 0.82)
	panel_style.set_border_width_all(1)
	panel_style.set_corner_radius_all(4)
	_panel.add_theme_stylebox_override("panel", panel_style)
	add_child(_panel)

	var rail := ColorRect.new()
	rail.name = "IntegrityRail"
	rail.position = Vector2(0, 0)
	rail.size = Vector2(5, 512)
	rail.color = CYAN
	rail.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_panel.add_child(rail)

	_add_label("REVENANT", Vector2(40, 30), 36, Color.WHITE)
	_add_label("DEAD NETWORKS REMAIN OPERATIONAL", Vector2(42, 78), 14, NEUTRAL)
	_state_label = _add_label("ENTRY", Vector2(42, 122), 18, CYAN)
	_detail_label = _add_label("", Vector2(42, 154), 16, Color.WHITE)
	_detail_label.size = Vector2(536, 48)
	_detail_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	_add_label("LOCAL OPERATOR ID", Vector2(42, 218), 14, AMBER)
	_username = LineEdit.new()
	_username.name = "Username"
	_username.position = Vector2(42, 244)
	_username.size = Vector2(536, 46)
	_username.max_length = MAX_USERNAME_LENGTH
	_username.placeholder_text = "1-32 letters, digits, _, . or -"
	_username.add_theme_font_size_override("font_size", 18)
	_username.text_changed.connect(_on_username_changed)
	_username.text_submitted.connect(_on_username_submitted)
	_panel.add_child(_username)

	_validation_label = _add_label("", Vector2(42, 296), 13, NEUTRAL)
	_endpoint_label = _add_label("LOCAL RELAY  •  %s" % _endpoint, Vector2(42, 330), 14, NEUTRAL)

	_connect_button = _make_button("CONNECT TO RELAY", "Connect", Vector2(42, 374), Vector2(250, 54), CYAN)
	_connect_button.pressed.connect(_request_connection)
	_settings_button = _make_button("SETTINGS", "Settings", Vector2(306, 374), Vector2(132, 54), AMBER)
	_settings_button.pressed.connect(_request_settings)
	_quit_button = _make_button("QUIT", "Quit", Vector2(452, 374), Vector2(126, 54), NEUTRAL)
	_quit_button.pressed.connect(quit_requested.emit)

	_add_label("ENTER  CONNECT    •    TAB  NAVIGATE    •    ESC  RETURN", Vector2(42, 458), 13, NEUTRAL)
	_connect_button.focus_neighbor_left = _username.get_path()
	_connect_button.focus_neighbor_top = _username.get_path()
	_quit_button.focus_neighbor_right = _username.get_path()


func _add_label(text: String, position: Vector2, font_size: int, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.position = position
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", color)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_panel.add_child(label)
	return label


func _make_button(text: String, name: String, position: Vector2, size: Vector2, accent: Color) -> Button:
	var button := Button.new()
	button.name = name
	button.text = text
	button.position = position
	button.size = size
	button.add_theme_font_size_override("font_size", 16)
	button.add_theme_color_override("font_color", NEUTRAL)
	button.add_theme_color_override("font_hover_color", Color.WHITE)
	button.add_theme_color_override("font_pressed_color", GRAPHITE)
	button.add_theme_stylebox_override("normal", _button_style(BLUE_PETROL.darkened(0.18), accent, 0.72))
	button.add_theme_stylebox_override("hover", _button_style(BLUE_PETROL.lightened(0.08), accent, 1.0))
	button.add_theme_stylebox_override("pressed", _button_style(accent, accent, 1.0))
	button.add_theme_stylebox_override("focus", _button_style(BLUE_PETROL.lightened(0.12), accent, 1.0))
	_panel.add_child(button)
	return button


func _button_style(background: Color, border: Color, opacity: float) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(background, opacity)
	style.border_color = border
	style.set_border_width_all(1)
	style.set_corner_radius_all(3)
	return style


func _set_state(state: String, detail: String, busy: bool) -> void:
	_state = state
	_state_label.text = state.to_upper()
	_state_label.add_theme_color_override("font_color", DAMAGE_RED if state == "Failed" else CYAN)
	_detail_label.text = detail
	_username.editable = not busy
	_connect_button.disabled = busy or not is_username_valid(_username.text)
	_connect_button.text = "CONNECTING..." if busy else ("RETRY CONNECTION" if state == "Failed" else "CONNECT TO RELAY")
	_quit_button.disabled = false


func _validate_username() -> void:
	var valid := is_username_valid(_username.text)
	_validation_label.text = "IDENTITY FORMAT READY" if valid else "USE 1-32 ASCII LETTERS, DIGITS, _, . OR -"
	_validation_label.add_theme_color_override("font_color", CYAN if valid else DAMAGE_RED)
	_connect_button.disabled = not valid or _state not in ["Entry", "Failed"]


func _on_username_changed(_value: String) -> void:
	_validate_username()


func _on_username_submitted(_value: String) -> void:
	if not _connect_button.disabled:
		_request_connection()


func _request_connection() -> void:
	if not is_username_valid(_username.text):
		_validate_username()
		return
	connect_requested.emit(_username.text)


func _request_settings() -> void:
	settings_requested.emit(_settings_button)
