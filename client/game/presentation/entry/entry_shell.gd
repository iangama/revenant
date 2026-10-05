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
var _panel: PanelContainer
var _username: LineEdit
var _endpoint_label: Label
var _state_label: Label
var _detail_label: Label
var _validation_label: Label
var _connect_button: Button
var _settings_button: Button
var _quit_button: Button
var _mode: OptionButton
var _mastery_button: Button
var _mastery_archive: Control
var _campaign_username := ""
var _campaign_snapshot := {}
var _challenge_username := ""
var _challenge_snapshot := {}
const CHALLENGE_VIEW = preload("res://projection/challenge_state.gd")
const ENTRY_MODES := ["standalone", "campaign", "practice_return_signal", "practice_meridian_readings", "practice_broken_supply_line", "practice_counter_signal", "practice_the_breach", "practice_prism_core", "challenges", "challenge_close_quarters", "challenge_meridian_circuit", "challenge_coolant_recovery", "challenge_bastion_link", "challenge_prism_discipline", "challenge_distant_signal", "challenge_last_reserve", "challenge_relay_gauntlet", "challenge_meridian_circuit@west_approach", "challenge_meridian_circuit@pace", "challenge_meridian_circuit@west_approach_pace", "challenge_relay_gauntlet@bastion_first", "challenge_relay_gauntlet@single_reserve", "challenge_relay_gauntlet@pace", "challenge_relay_gauntlet@bastion_first_single_reserve", "challenge_relay_gauntlet@single_reserve_pace"]
const CHAPTER_NAMES := ["Return signal", "Meridian readings", "Broken supply line", "Counter-signal", "The breach", "Prism core"]


func entry_mode() -> String:
	return ENTRY_MODES[_mode.selected].split("@")[0]


func entry_preset() -> String:
	var parts: PackedStringArray = ENTRY_MODES[_mode.selected].split("@")
	return parts[1] if parts.size() > 1 else "baseline"


func select_mode(mode: String) -> void:
	var index := ENTRY_MODES.find(mode)
	_mode.select(maxi(index, 0))
	_refresh_campaign_menu()


func set_challenge_progress(username: String, snapshot: Dictionary) -> void:
	_challenge_username = username
	_challenge_snapshot = snapshot.duplicate(true)
	_refresh_campaign_menu()


func _refresh_challenge_menu() -> void:
	_connect_button.disabled = not is_username_valid(_username.text)
	var known := not _challenge_snapshot.is_empty() and _challenge_username == _username.text
	var contract := entry_mode().trim_prefix("challenge_")
	_state_label.text = tr("CHALLENGES")
	if entry_mode() == "challenges":
		_connect_button.text = tr("REFRESH CHALLENGE BOARD")
		_detail_label.text = tr("Load your records, then choose a contract in Play mode. Challenges use your current equipment and grant no items or XP.")
		if known:
			_detail_label.text = tr("Completed contracts: %d / %d. Choose a contract in Play mode. Every attempt starts from the beginning.") % [_completed_contract_count(), _challenge_snapshot.contracts.size()]
		return
	var previous: Variant = _challenge_snapshot.get("active") if known else null
	if previous == null and known and _challenge_snapshot.get("last_result") is Dictionary:
		previous = _challenge_snapshot.last_result.run
	var retry: bool = previous is Dictionary and previous.contract.contract_id == contract and CHALLENGE_VIEW.preset_id(previous.contract) == entry_preset()
	var completed := false
	if known:
		for record in _challenge_snapshot.records:
			if record.contract.contract_id == contract and CHALLENGE_VIEW.preset_id(record.contract) == entry_preset(): completed = true
	_connect_button.text = tr("RESTART CONTRACT") if retry else tr("START CONTRACT")
	var goal := tr("Defeat the Glass Lancer and Relay Mender. Equipment is locked during the attempt.") if contract == "close_quarters" else tr("Visit the lens, gallery and log in any order, then return to the Meridian entrance.")
	if contract == "coolant_recovery":
		goal = tr("Collect, transfer, deliver. Hold 1.2 s at stations 2 and 3. No deadline or rewards. Retry starts over.")
	if contract == "bastion_link":
		goal = tr("Defeat the Mender and Bulwark. Flank the shield. No items or XP; retry starts over.")
	if contract == "prism_discipline":
		goal = tr("Defeat both Prism phases. Follow the floor warnings. No items or XP; retry restarts the boss.")
	if contract == "distant_signal":
		goal = tr("Defeat the Sentinel around cover. Range 4+ required; Rail Driver suggested. No items or XP; retry starts over.")
	if contract == "last_reserve":
		goal = tr("Hold west then east for 3.6 s; return alive. Reserve pad: +36 HP once. Weapons offline. No items, XP or deadline.")
	if contract == "relay_gauntlet":
		goal = tr("Close Quarters → Bastion → Prism. +24 HP between stages. Pulse Rifle. No items or XP; retry restarts all.")
	var preset := entry_preset()
	if preset != "baseline":
		goal = tr(CHALLENGE_VIEW.PRESET_NAMES[preset]) + ". " + (tr("Read all three stations; return west [-31, 0].") if preset.begins_with("west_approach") else tr("Bastion → Close Quarters → Prism.") if preset.begins_with("bastion_first") else "")
		if "single_reserve" in preset: goal += " " + tr("Reserve: +24 HP once, after either early stage.")
		if "pace" in preset: goal += " " + tr("Optional goal: %d s. Late completion still counts.") % (60 if contract == "meridian_circuit" else 180)
	_detail_label.text = (tr("Record: complete. ") if completed else "") + goal + ("" if contract in ["coolant_recovery", "bastion_link", "prism_discipline", "distant_signal", "last_reserve", "relay_gauntlet"] else " " + tr("No items or XP. Restart resets all progress."))
	if known:
		for record in _challenge_snapshot.records:
			if record.contract.contract_id == contract and CHALLENGE_VIEW.preset_id(record.contract) == preset and record.get("elapsed_ms") != null:
				_detail_label.text += " " + tr("Best: %.1f s.") % (float(record.elapsed_ms) / 1000.0)
	if known and _challenge_snapshot.get("active") is Dictionary and not retry:
		_detail_label.text = tr("Restart the interrupted contract before choosing another one.")
		_connect_button.disabled = true


func set_campaign_progress(username: String, snapshot: Dictionary) -> void:
	_campaign_username = username
	_campaign_snapshot = snapshot.duplicate(true)
	_refresh_campaign_menu()


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build_shell()
	show_entry("revenant-godot")
	_mastery_archive = preload("res://presentation/challenges/mastery_archive.gd").new()
	add_child(_mastery_archive)
	_mastery_archive.close_requested.connect(_close_mastery)
	_mastery_archive.contract_selected.connect(func(mode: String) -> void: _close_mastery(); select_mode(mode); _connect_button.grab_focus())


func _open_mastery() -> void:
	if _challenge_snapshot.get("mastery") is Dictionary:
		_mastery_archive.call("open_archive", _challenge_snapshot, _panel.scale.x)
		_panel.hide()


func _close_mastery() -> void:
	_mastery_archive.visible = false
	_panel.show()
	_mastery_button.grab_focus()


func configure_endpoint(host: String, port: int) -> void:
	_endpoint = "%s:%d" % [host, port]
	if _endpoint_label != null:
		_endpoint_label.text = "LOCAL RELAY  •  %s" % _endpoint


func show_entry(username: String) -> void:
	visible = true
	_username.text = username
	_set_state("Entry", "Review your local identity, then connect to the relay.", false)
	_validate_username()
	_refresh_campaign_menu()
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
		"panel_rect": _visible_panel_rect(),
	}


func _visible_panel_rect() -> Rect2:
	var bounds := _panel.get_global_rect()
	var ancestor := _panel.get_parent()
	while ancestor is Control:
		if ancestor.clip_contents:
			bounds = bounds.intersection(ancestor.get_global_rect())
		ancestor = ancestor.get_parent()
	return bounds


func settings_focus_source() -> Control:
	return _settings_button


func username_control() -> Control:
	return _username


func _build_shell() -> void:
	var backdrop := ColorRect.new()
	backdrop.name = "Backdrop"
	backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	backdrop.color = Color(GRAPHITE, 0.96)
	backdrop.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(backdrop)

	_panel = PanelContainer.new()
	_panel.name = "EntryPanel"
	_panel.position = Vector2(330, 72)
	_panel.size = Vector2(620, 576)
	var panel_style := StyleBoxFlat.new()
	panel_style.bg_color = Color(BLUE_PETROL, 0.94)
	panel_style.border_color = Color(CYAN, 0.82)
	panel_style.set_border_width_all(1)
	panel_style.border_width_left = 5
	panel_style.set_corner_radius_all(4)
	panel_style.content_margin_left = 42
	panel_style.content_margin_right = 42
	panel_style.content_margin_top = 30
	panel_style.content_margin_bottom = 28
	_panel.add_theme_stylebox_override("panel", panel_style)
	_panel.minimum_size_changed.connect(_fit_panel.call_deferred)
	add_child(_panel)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 12)
	_panel.add_child(column)

	_add_label(column, "REVENANT", 36, Color.WHITE)
	_add_label(column, "DEAD NETWORKS REMAIN OPERATIONAL", 14, NEUTRAL)
	_state_label = _add_label(column, "ENTRY", 18, CYAN)
	_detail_label = _add_label(column, "", 16, Color.WHITE)
	_detail_label.custom_minimum_size.y = 58
	_detail_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_mastery_button = _make_button(column, "MASTERY & ARCHIVE", "MasteryArchive", 0, CYAN)
	_mastery_button.visible = false
	_mastery_button.pressed.connect(_open_mastery)

	_add_label(column, "LOCAL OPERATOR ID", 14, AMBER)
	_username = LineEdit.new()
	_username.name = "Username"
	_username.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	_username.custom_minimum_size.y = 46
	_username.max_length = MAX_USERNAME_LENGTH
	_username.placeholder_text = "1-32 letters, digits, _, . or -"
	_username.add_theme_font_size_override("font_size", 18)
	_username.text_changed.connect(_on_username_changed)
	_username.text_submitted.connect(_on_username_submitted)
	column.add_child(_username)

	_validation_label = _add_label(column, "", 13, NEUTRAL)
	_add_label(column, "PLAY MODE", 14, AMBER)
	_mode = OptionButton.new()
	_mode.name = "PlayMode"
	_mode.custom_minimum_size.y = 46
	for text in ["Standalone operation", "Campaign — continue / resume", "Practice — Return signal", "Practice — Meridian readings", "Practice — Broken supply line", "Practice — Counter-signal", "Practice — The breach", "Practice — Prism core", "Challenge board", "Challenge — Close quarters", "Challenge — Meridian circuit", "Challenge — Coolant recovery", "Challenge — Bastion link", "Challenge — Prism discipline", "Challenge — Distant signal", "Challenge — Last reserve", "Challenge — Relay gauntlet"]:
		_mode.add_item(text)
	for label in ["Meridian — West approach", "Meridian — Time goal", "Meridian — West + time goal", "Gauntlet — Bastion first", "Gauntlet — Single reserve", "Gauntlet — Time goal", "Gauntlet — Bastion + reserve", "Gauntlet — Reserve + time goal"]:
		_mode.add_item(label)
	_mode.tooltip_text = "Campaign progress saves at checkpoints. Practice requires a cleared chapter and grants no reward."
	_mode.item_selected.connect(func(_index: int) -> void: _refresh_campaign_menu())
	column.add_child(_mode)

	var buttons := HBoxContainer.new()
	buttons.add_theme_constant_override("separation", 14)
	column.add_child(buttons)
	_connect_button = _make_button(buttons, "CONNECT TO RELAY", "Connect", 240, CYAN)
	_connect_button.pressed.connect(_request_connection)
	_settings_button = _make_button(buttons, "SETTINGS", "Settings", 160, AMBER)
	_settings_button.pressed.connect(_request_settings)
	_quit_button = _make_button(buttons, "QUIT", "Quit", 108, NEUTRAL)
	_quit_button.pressed.connect(quit_requested.emit)

	_add_label(column, "ENTER  CONNECT    •    TAB  NAVIGATE    •    ESC  RETURN", 13, NEUTRAL)
	_endpoint_label = _add_label(column, "LOCAL RELAY  •  %s" % _endpoint, 14, NEUTRAL)
	_connect_button.focus_neighbor_left = _username.get_path()
	_connect_button.focus_neighbor_top = _username.get_path()
	_quit_button.focus_neighbor_right = _username.get_path()


func _fit_panel() -> void:
	# Wrapped labels can report a tall minimum before receiving their width.
	_panel.size.y = maxf(576, _panel.get_combined_minimum_size().y)


func _add_label(parent: Control, text: String, font_size: int, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", color)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	parent.add_child(label)
	return label


func _make_button(parent: Control, text: String, name: String, width: float, accent: Color) -> Button:
	var button := Button.new()
	button.name = name
	button.text = text
	button.custom_minimum_size = Vector2(width, 54)
	button.add_theme_font_size_override("font_size", 16)
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART if name == "Connect" else TextServer.AUTOWRAP_OFF
	button.add_theme_color_override("font_color", NEUTRAL)
	button.add_theme_color_override("font_hover_color", Color.WHITE)
	button.add_theme_color_override("font_pressed_color", GRAPHITE)
	button.add_theme_stylebox_override("normal", _button_style(BLUE_PETROL.darkened(0.18), accent, 0.72))
	button.add_theme_stylebox_override("hover", _button_style(BLUE_PETROL.lightened(0.08), accent, 1.0))
	button.add_theme_stylebox_override("pressed", _button_style(accent, accent, 1.0))
	button.add_theme_stylebox_override("focus", _button_style(BLUE_PETROL.lightened(0.12), accent, 1.0))
	parent.add_child(button)
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
	_mode.disabled = busy
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
	_refresh_campaign_menu()


func _refresh_campaign_menu() -> void:
	if _mode == null: return
	_mastery_button.visible = entry_mode().begins_with("challenge") and _challenge_username == _username.text and _challenge_snapshot.get("mastery") is Dictionary
	var known := not _campaign_snapshot.is_empty() and _campaign_username == _username.text
	var active: Variant = _campaign_snapshot.get("active") if known else null
	var cleared: int = _campaign_snapshot.get("cleared_chapters", 0) if known else 0
	for index in 6:
		_mode.set_item_disabled(index + 2, known and (active is Dictionary or index >= cleared))
	if _state != "Entry": return
	if entry_mode().begins_with("challenge"):
		_refresh_challenge_menu()
		return
	_connect_button.disabled = not is_username_valid(_username.text)
	var chapter_index: int = _mode.selected - 2 if entry_mode().begins_with("practice_") else cleared
	if active is Dictionary:
		chapter_index = ["return_signal", "meridian_readings", "broken_supply_line", "counter_signal", "the_breach", "prism_core"].find(active.get("chapter_id"))
	_state_label.text = tr("ENTRY") if entry_mode() == "standalone" else tr("CAMPAIGN COMPLETE") if chapter_index == 6 else tr(["ACT I • TRACE THE RETURN", "ACT II • CUT THE FALSE SIGNAL", "ACT III • OPEN THE CORE"][clampi(chapter_index / 2, 0, 2)])
	if entry_mode() == "standalone":
		_detail_label.text = tr("Review your local identity, then connect to the relay.")
		_connect_button.text = tr("CONNECT TO RELAY")
	elif active is Dictionary:
		var index := ["return_signal", "meridian_readings", "broken_supply_line", "counter_signal", "the_breach", "prism_core"].find(active.get("chapter_id"))
		_detail_label.text = tr("Resume %s from checkpoint %d. The interrupted encounter restarts.") % [tr(CHAPTER_NAMES[index]), int(active.get("checkpoint", 0))]
		_connect_button.text = tr("RESUME CHAPTER")
	elif entry_mode().begins_with("practice_"):
		_detail_label.text = tr("Replay a cleared chapter. Choices affect later attempts; first-clear rewards are not repeated.")
		_connect_button.text = tr("START PRACTICE")
	elif known and cleared == 6:
		_detail_label.text = tr("Campaign complete. Continue to read your journal, or choose a chapter for practice.")
		_connect_button.text = tr("READ CAMPAIGN JOURNAL")
	elif known:
		_detail_label.text = tr("Next: %s. First clear: %d fragment and %d XP.") % [tr(CHAPTER_NAMES[cleared]), _campaign_snapshot.get("first_clear_fragments", 1), _campaign_snapshot.get("first_clear_experience", 100)]
		_connect_button.text = tr("START NEXT CHAPTER")
	else:
		_detail_label.text = tr("Follow the return signal through six chapters. Optional records can be skipped. Your saved chapter resumes automatically.")
		_connect_button.text = tr("CONTINUE CAMPAIGN")


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


func _completed_contract_count() -> int:
	var ids := {}
	for record in _challenge_snapshot.get("records", []): ids[record.contract.contract_id] = true
	return ids.size()
