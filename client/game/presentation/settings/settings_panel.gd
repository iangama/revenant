extends Control

signal settings_applied(settings: Dictionary)
signal closed

const INPUT_BINDINGS := preload("res://input/input_bindings.gd")
const SETTINGS_STORE := preload("res://presentation/settings/settings_store.gd")

var _settings := {}
var _focus_return: Control
var _panel: PanelContainer
var _tabs: TabContainer
var _sliders := {}
var _toggles := {}
var _choices := {}
var _binding_buttons := {}
var _mute: CheckButton
var _reduced_flash: CheckButton
var _display: OptionButton
var _guidance: OptionButton
var _apply_button: Button
var _cancel_button: Button
var _capture := {}
var _notice: Label


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build_panel()
	visible = false


func open(settings: Dictionary, focus_return: Control = null) -> void:
	_settings = settings.duplicate(true)
	_focus_return = focus_return
	_capture.clear()
	_sync_controls()
	visible = true
	_apply_button.grab_focus()


func close_panel() -> void:
	_capture.clear()
	visible = false
	if is_instance_valid(_focus_return):
		_focus_return.grab_focus()
	closed.emit()


func is_capturing() -> bool:
	return not _capture.is_empty()


func presentation_state() -> Dictionary:
	return {
		"visible": visible, "slider_count": _sliders.size(),
		"has_mute": _mute != null, "has_reduced_flash": _reduced_flash != null,
		"display_options": _display.item_count, "guidance_options": _guidance.item_count,
		"focus_owner": get_viewport().gui_get_focus_owner().name if get_viewport().gui_get_focus_owner() != null else "",
		"mouse_captured": mouse_filter == Control.MOUSE_FILTER_STOP,
		"panel_rect": Rect2(_panel.position, _panel.size),
		"binding_count": _binding_buttons.size(), "capturing": is_capturing(),
		"tab_count": _tabs.get_tab_count(),
	}


func focus_start() -> Control:
	return _apply_button


func _input(event: InputEvent) -> void:
	if not visible or not event.is_pressed() or event.is_echo():
		return
	if is_capturing():
		get_viewport().set_input_as_handled()
		if event is InputEventKey and event.keycode == KEY_ESCAPE:
			_capture.clear()
			_notice.text = "Binding cancelled."
			return
		var channel: String = _capture.channel
		var code := -2
		if event is InputEventKey and channel == "key":
			code = event.physical_keycode if event.physical_keycode != 0 else event.keycode
		elif event is InputEventJoypadButton and channel == "button":
			code = event.button_index
		elif event is InputEventKey and channel == "button" and event.keycode == KEY_DELETE:
			code = -1
		if code != -2:
			if INPUT_BINDINGS.rebind(_settings.bindings, _capture.action, channel, code):
				_capture.clear()
				_notice.text = "Binding changed. Conflicts swap places. Apply to save."
				_sync_binding_buttons()
			else:
				_notice.text = "That input is reserved for menu navigation. Choose another."
		return
	if event.is_action_pressed("ui_cancel") or event.is_action_pressed("settings"):
		get_viewport().set_input_as_handled()
		close_panel()
	elif event is InputEventJoypadButton and event.button_index in [JOY_BUTTON_LEFT_SHOULDER, JOY_BUTTON_RIGHT_SHOULDER]:
		_tabs.current_tab = posmod(_tabs.current_tab + (-1 if event.button_index == JOY_BUTTON_LEFT_SHOULDER else 1), _tabs.get_tab_count())
		_tabs.get_tab_bar().grab_focus()
		get_viewport().set_input_as_handled()


func _build_panel() -> void:
	var backdrop := ColorRect.new()
	backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	backdrop.color = Color("10151d", 0.96)
	add_child(backdrop)
	_panel = PanelContainer.new()
	_panel.name = "SettingsPanel"
	_panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_panel.offset_left = 56
	_panel.offset_right = -56
	_panel.offset_top = 24
	_panel.offset_bottom = -24
	var style := StyleBoxFlat.new()
	style.bg_color = Color("12313a")
	style.border_color = Color("f5a524")
	style.set_border_width_all(2)
	style.content_margin_left = 24
	style.content_margin_right = 24
	style.content_margin_top = 16
	style.content_margin_bottom = 16
	_panel.add_theme_stylebox_override("panel", style)
	add_child(_panel)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 12)
	_panel.add_child(column)
	_label(column, "SETTINGS", "SettingsTitle")
	_tabs = TabContainer.new()
	_tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(_tabs)
	var audio := _page("Audio and display")
	for definition in [["master_volume", "Master volume"], ["ambience_volume", "Ambience volume"], ["effects_volume", "Effects volume"], ["interface_volume", "Interface volume"]]:
		var row := HBoxContainer.new()
		audio.add_child(row)
		var label := _label(row, definition[1])
		label.custom_minimum_size.x = 290
		var slider := HSlider.new()
		slider.name = definition[0].to_pascal_case()
		slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		slider.custom_minimum_size.y = 48
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.05
		row.add_child(slider)
		var value := _label(row, "100%")
		value.custom_minimum_size.x = 90
		slider.value_changed.connect(func(amount: float) -> void: value.text = "%d%%" % roundi(amount * 100))
		_sliders[definition[0]] = slider
	_mute = _toggle(audio, "muted", "Mute all audio")
	_mute.name = "Mute"
	_display = _choice(audio, "display_mode", "Display", ["Windowed", "Fullscreen"], ["Windowed", "Fullscreen"])
	_label(audio, "Changes take effect when you select Apply. Sound is optional.")
	var access := _page("Accessibility")
	_reduced_flash = _toggle(access, "reduced_flash", "Reduced flash")
	_toggle(access, "reduced_motion", "Reduced motion")
	_toggle(access, "high_contrast", "High contrast")
	_toggle(access, "captions", "Sound captions")
	_choice(access, "ui_scale", "Interface size", ["100%", "125%", "150%"], [1.0, 1.25, 1.5])
	_choice(access, "language", "Language", ["English", "Português (Brasil)", "Expanded text preview"], ["en", "pt_BR", "pseudo"])
	_guidance = _choice(access, "guidance_mode", "Guidance", ["Full", "Compact", "Off"], ["Full", "Compact", "Off"])
	var controls := _page("Controls")
	_label(controls, "Select a binding, then press a key or controller button. Conflicts swap.\nEsc cancels capture. Delete clears a controller binding. Left stick moves.\nMenus: Tab / arrows / D-pad, Enter / South to select, Esc / East to close.")
	for action in INPUT_BINDINGS.ACTIONS:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 12)
		controls.add_child(row)
		var label := _label(row, INPUT_BINDINGS.ACTIONS[action])
		label.custom_minimum_size.x = 290
		_binding_buttons[action] = {}
		for channel in ["key", "button"]:
			var button := _button(row, "", action.to_pascal_case() + channel.to_pascal_case())
			button.pressed.connect(_begin_capture.bind(action, channel))
			_binding_buttons[action][channel] = button
	_notice = _label(column, "")
	_notice.custom_minimum_size.y = 44
	var footer := HBoxContainer.new()
	footer.add_theme_constant_override("separation", 12)
	column.add_child(footer)
	_apply_button = _button(footer, "APPLY", "Apply")
	_apply_button.pressed.connect(_apply)
	_cancel_button = _button(footer, "CANCEL", "Cancel")
	_cancel_button.pressed.connect(close_panel)
	var reset := _button(footer, "RESTORE DEFAULTS", "RestoreDefaults")
	reset.pressed.connect(func() -> void:
		_settings = SETTINGS_STORE.new().defaults()
		_sync_controls()
		_notice.text = "Defaults restored in this form. Apply to save."
	)


func _page(title: String) -> VBoxContainer:
	var scroll := ScrollContainer.new()
	scroll.name = title
	scroll.follow_focus = true
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_tabs.add_child(scroll)
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 12)
	scroll.add_child(column)
	return column


func _toggle(parent: Node, key: String, text: String) -> CheckButton:
	var button := CheckButton.new()
	button.name = key.to_pascal_case()
	button.text = text
	button.custom_minimum_size.y = 48
	parent.add_child(button)
	_toggles[key] = button
	return button


func _choice(parent: Node, key: String, title: String, labels: Array, values: Array) -> OptionButton:
	var row := HBoxContainer.new()
	parent.add_child(row)
	var label := _label(row, title)
	label.custom_minimum_size.x = 290
	var choice := OptionButton.new()
	choice.name = key.to_pascal_case()
	choice.custom_minimum_size.y = 48
	choice.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for index in labels.size():
		choice.add_item(labels[index])
		choice.set_item_metadata(index, values[index])
	row.add_child(choice)
	_choices[key] = choice
	return choice


func _sync_controls() -> void:
	for key in _sliders:
		_sliders[key].value = _settings.get(key, 0.0)
	for key in _toggles:
		_toggles[key].button_pressed = _settings.get(key, false)
	for key in _choices:
		var choice: OptionButton = _choices[key]
		for index in choice.item_count:
			if choice.get_item_metadata(index) == _settings.get(key):
				choice.select(index)
	_sync_binding_buttons()
	_notice.text = ""


func _sync_binding_buttons() -> void:
	for action in _binding_buttons:
		_binding_buttons[action].key.text = INPUT_BINDINGS.key_label(_settings.bindings, action)
		_binding_buttons[action].button.text = INPUT_BINDINGS.button_label(_settings.bindings, action)


func _begin_capture(action: String, channel: String) -> void:
	_capture = {"action": action, "channel": channel}
	_notice.text = "Press a %s for %s. Esc cancels." % [channel, INPUT_BINDINGS.ACTIONS[action]]


func _apply() -> void:
	for key in _sliders:
		_settings[key] = _sliders[key].value
	for key in _toggles:
		_settings[key] = _toggles[key].button_pressed
	for key in _choices:
		var choice: OptionButton = _choices[key]
		_settings[key] = choice.get_item_metadata(choice.selected)
	settings_applied.emit(_settings.duplicate(true))
	close_panel()


func _label(parent: Node, text: String, node_name := "") -> Label:
	var label := Label.new()
	label.text = text
	if not node_name.is_empty():
		label.name = node_name
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	parent.add_child(label)
	return label


func _button(parent: Node, text: String, node_name: String) -> Button:
	var button := Button.new()
	button.text = text
	button.name = node_name
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	button.custom_minimum_size = Vector2(190, 48)
	parent.add_child(button)
	return button
