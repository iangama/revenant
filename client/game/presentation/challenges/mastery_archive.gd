extends Control

signal close_requested
signal contract_selected(mode: String)

const CATALOG = preload("res://projection/challenge_mastery.gd")
var _scaler := preload("res://presentation/settings/surface_scaler.gd").new()
var _panel: PanelContainer
var _summary: Label
var _badges: Label
var _result: Label
var _choice: OptionButton
var _rule: Label
var _record: Label
var _close: Button
var _archive := {}


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var shade := ColorRect.new()
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	shade.color = Color("10151d")
	add_child(shade)
	_panel = PanelContainer.new()
	_panel.size = Vector2(900, 630)
	var style := StyleBoxFlat.new()
	style.bg_color = Color("12313a")
	style.border_color = Color("35d0d0")
	style.set_border_width_all(1)
	style.set_content_margin_all(24)
	_panel.add_theme_stylebox_override("panel", style)
	_panel.minimum_size_changed.connect(_fit.call_deferred)
	add_child(_panel)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 14)
	_panel.add_child(column)
	_label(column, "MASTERY & ARCHIVE", 26)
	_summary = _label(column, "", 18)
	_label(column, "Archive titles grant no combat power, items or XP. Earlier qualifying attempts count.", 16)
	_badges = _label(column, "", 18)
	_label(column, "LATEST ATTEMPT", 20)
	_result = _label(column, "", 17)
	_label(column, "CHOOSE A MASTERY", 20)
	_choice = OptionButton.new()
	_choice.custom_minimum_size.y = 44
	_choice.add_theme_font_size_override("font_size", 18)
	_choice.item_selected.connect(func(_index: int) -> void: _refresh_goal())
	column.add_child(_choice)
	_rule = _label(column, "", 18)
	_record = _label(column, "", 16)
	var select := _button(column, "SELECT THIS CONTRACT")
	select.pressed.connect(func() -> void: contract_selected.emit(CATALOG.GOALS[CATALOG.GOALS.keys()[_choice.selected]].mode))
	_label(column, "Build goals allow an extra owned module. In a standalone operation, equip your weapon before completion and save your modules at the workshop afterward.", 16)
	_close = _button(column, "BACK TO CHALLENGE BOARD")
	_close.pressed.connect(func() -> void: close_requested.emit())
	visible = false


func open_archive(snapshot: Dictionary, factor: float) -> void:
	_archive = snapshot.mastery.duplicate(true)
	_summary.text = tr("Masteries: %d / 6 • Titles: %d / 3") % [_archive.records.size(), _archive.badges.size()]
	var lines := PackedStringArray()
	for badge in CATALOG.BADGES:
		var definition: Dictionary = CATALOG.BADGES[badge]
		var count: int = definition.goals.filter(func(goal: String) -> bool: return _find_record(goal) != null).size()
		lines.append("%s • %d/%d" % [tr(definition.name), count, definition.goals.size()])
	_badges.text = "\n".join(lines)
	_choice.clear()
	for goal in CATALOG.GOALS:
		_choice.add_item(("✓ " if _find_record(goal) != null else "○ ") + tr(CATALOG.GOALS[goal].name))
	_choice.select(0)
	_result.text = tr("Complete a contract to see its result here.")
	var attempt: Variant = _archive.get("last_attempt")
	if attempt is Dictionary:
		_result.text = tr("Attempt #%d") % attempt.run_id
		if attempt.assessments.is_empty(): _result.text += "\n" + tr("This contract has no mastery objective. Its completion record is saved separately.")
		for assessment in attempt.assessments:
			_result.text += "\n" + tr(CATALOG.GOALS[assessment.goal].name) + ": " + tr(CATALOG.REASONS[assessment.reason])
			_choice.select(CATALOG.GOALS.keys().find(assessment.goal))
		if attempt.route_moves != null: _result.text += "\n" + tr("Route: %d / 40 moves") % attempt.route_moves
		if attempt.prism_damage != null: _result.text += "\n" + tr("Pattern damage taken: %d") % attempt.prism_damage
	_refresh_goal()
	visible = true
	_scaler.apply(self, factor)
	_fit.call_deferred()
	_choice.grab_focus()


func _refresh_goal() -> void:
	var goal: String = CATALOG.GOALS.keys()[_choice.selected]
	_rule.text = tr(CATALOG.GOALS[goal].rule)
	var record: Variant = _find_record(goal)
	_record.text = tr("First achieved in attempt #%d") % record.run_id if record != null else tr("Not yet achieved. A missed mastery never invalidates contract completion.")


func _unhandled_input(event: InputEvent) -> void:
	if is_visible_in_tree() and event.is_action_pressed("ui_cancel"):
		close_requested.emit()
		get_viewport().set_input_as_handled()


func _find_record(goal: String) -> Variant:
	for record in _archive.get("records", []):
		if record.goal == goal: return record
	return null


func _fit() -> void:
	_panel.size.y = maxf(630, _panel.get_combined_minimum_size().y)


func _label(parent: Control, text: String, font_size: int) -> Label:
	var label := Label.new()
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.add_theme_font_size_override("font_size", font_size)
	parent.add_child(label)
	return label


func _button(parent: Control, text: String) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size.y = 44
	button.add_theme_font_size_override("font_size", 18)
	parent.add_child(button)
	return button
