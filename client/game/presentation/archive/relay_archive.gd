extends Control

signal close_requested

const TERMINAL := preload("res://presentation/environment/modules/relay_terminal.tscn")
const CYAN := Color("35d0d0")
const AMBER := Color("f5a524")
const PAPER := Color("d5deea")
const READING_DISTANCE := 1.75
const RECORDS := [
	{
		"id": "last_departure", "title": "The last departure", "source": "01 / ARRIVAL PLATFORM",
		"position": Vector3(-2, 0, -1), "location": "Arrival platform · west of the landing ring",
		"body": "The departure board still says ON TIME. Someone has crossed out the destination and written HOME beneath it.\n\nSHIFT LOG — E. VALE\nThe last shuttle left with seventeen seats and sixteen passengers. We kept one empty for the operator still below the relay. The station continued to answer their badge long after the lift stopped moving.\n\nI asked Control to cut the signal. They told me the Warden was already guarding it.\n\nIf someone returns through this platform, leave the lights on. They may not remember which way they came.",
	},
	{
		"id": "two_signals", "title": "Two signals in the dark", "source": "02 / MAINTENANCE BENCH",
		"position": Vector3(2, 0, 4), "location": "Maintenance bench · beside the stabilizer lane",
		"body": "Two mugs sit beside the relay controls. One is tied to the workbench with a length of copper wire.\n\nMAINTENANCE NOTE\nThe feedback used to knock us out before we could reach the second contact. Vale stayed at the anchor while I crossed. One pulse to say ready. Two to say come back.\n\nWhen the relay dropped me, she held the connection until I could stand. We left the marks on the floor for the next pair.\n\nThe breach circuit is faster. Stabilizing gives you time to listen. Neither route explains why a third signal keeps answering from inside the core.",
	},
	{
		"id": "custodian_order", "title": "The custodian's order", "source": "03 / CORE MEMORY — SIGNAL RESTORED",
		"position": Vector3(7, 0, -2), "location": "Core memory · recovered when the Warden falls",
		"body": "With the Warden silent, the relay finishes a transmission it has been repeating for years.\n\nCUSTODIAN ORDER 04\nKeep the carrier inside. Preserve the return signal. Do not interpret a familiar badge as proof of identity.\n\nThe last entry is not an order. It is a recording from the empty shuttle seat:\n\n“If you make it back, find the others. We kept the route lit.”\n\nOutside the chamber, a distant relay answers once. This station was never the end of the line.",
	},
	{
		"id": "meridian_memory", "title": "A name in the glass", "source": "04 / MERIDIAN — SKY GALLERY",
		"body": "“Vale, we kept your window facing home.” A name scratched beneath the last intact star.",
	},
]

var _read := {}
var _core_available := false
var _meridian_available := false
var _safe := false
var _nearest := -1
var _selected := -1
var _buttons: Array[Button] = []
var _markers: Array[Label3D] = []
var _heading: Label
var _source: Label
var _body: RichTextLabel
var _counter: Label
var _close: Button


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build_surface()
	visible = false


func build_terminals(parent: Node3D) -> void:
	var group := Node3D.new()
	group.name = "FieldArchiveTerminals"
	parent.add_child(group)
	for index in 3:
		var terminal := TERMINAL.instantiate()
		terminal.name = RECORDS[index].id.to_pascal_case()
		terminal.position = RECORDS[index].position
		terminal.scale = Vector3.ONE * 0.7
		group.add_child(terminal)
		var marker := Label3D.new()
		marker.text = "ARCHIVE %02d" % (index + 1)
		marker.position = Vector3(0, 2.05, 0)
		marker.font_size = 40
		marker.pixel_size = 0.012
		marker.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		marker.modulate = AMBER
		terminal.add_child(marker)
		_markers.append(marker)


func reset_for_connection() -> void:
	_read.clear()
	_core_available = false
	_meridian_available = false
	_safe = false
	_nearest = -1
	_selected = -1
	visible = false
	_refresh()


func update_meridian_memory(confirmed: bool) -> void:
	if _meridian_available == confirmed:
		return
	_meridian_available = confirmed
	if not confirmed:
		_read.erase(3)
		if _selected == 3:
			_selected = -1
	_refresh()


func _record_count() -> int:
	return 4 if _meridian_available else 3


func update_context(player_position: Vector3, safe: bool, complete: bool, suspended: bool) -> void:
	var previous_nearest := _nearest
	var previous_core := _core_available
	_safe = safe and not suspended
	_core_available = complete
	_nearest = -1
	if _safe:
		for index in 2:
			if player_position.distance_to(RECORDS[index].position) <= READING_DISTANCE:
				_nearest = index
				break
		if complete:
			_nearest = 2
	if not _safe and visible:
		close_requested.emit()
	if visible and (previous_nearest != _nearest or previous_core != _core_available):
		_refresh()
	for index in _markers.size():
		var available := index < 2 or _core_available
		_markers[index].modulate = CYAN if _read.has(index) else AMBER
		_markers[index].text = "%s %02d" % ["READ" if _read.has(index) else "ARCHIVE", index + 1]
		_markers[index].visible = safe and available and not suspended


func prompt_text() -> String:
	if _nearest >= 0:
		return "READ RECORD [E]"
	return "ARCHIVE [J]  %d/%d" % [_read.size(), _record_count()]


func hint_text() -> String:
	if not _safe:
		return "Records available between encounters."
	if _nearest >= 0:
		return RECORDS[_nearest].title
	return "Find the amber archive markers."


func can_open() -> bool:
	return _safe


func open_archive(nearby := false) -> bool:
	if not _safe or (nearby and _nearest < 0):
		return false
	visible = true
	if nearby:
		read_record(_nearest)
	elif _core_available and not _read.has(2):
		read_record(2)
	elif _meridian_available and not _read.has(3):
		read_record(3)
	elif _selected >= 0 and _read.has(_selected):
		read_record(_selected)
	else:
		_refresh()
	_close.grab_focus()
	return true


func read_record(index: int) -> bool:
	if not _safe or index < 0 or index >= _record_count():
		return false
	if not _read.has(index) and index != _nearest and not (index == 2 and _core_available) and not (index == 3 and _meridian_available):
		return false
	_read[index] = true
	_selected = index
	_refresh()
	_body.scroll_to_line(0)
	return true


func close_archive() -> void:
	visible = false


func presentation_state() -> Dictionary:
	return {"visible": visible, "safe": _safe, "nearest": _nearest,
		"read_count": _read.size(), "selected": _selected,
		"core_available": _core_available, "title": _heading.text,
		"body": _body.text, "record_count": _record_count(), "meridian_available": _meridian_available}


func _build_surface() -> void:
	var shade := ColorRect.new()
	shade.color = Color(0.02, 0.05, 0.08, 0.94)
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(shade)
	var panel := PanelContainer.new()
	panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	panel.offset_left = -540
	panel.offset_top = -305
	panel.offset_right = 540
	panel.offset_bottom = 305
	var style := StyleBoxFlat.new()
	style.bg_color = Color("101c26")
	style.border_color = CYAN.darkened(0.4)
	style.set_border_width_all(1)
	style.set_corner_radius_all(5)
	style.content_margin_left = 28
	style.content_margin_right = 28
	style.content_margin_top = 24
	style.content_margin_bottom = 24
	panel.add_theme_stylebox_override("panel", style)
	add_child(panel)
	var layout := VBoxContainer.new()
	layout.add_theme_constant_override("separation", 14)
	panel.add_child(layout)
	_label(layout, "RELAY FIELD ARCHIVE", 26, CYAN)
	_counter = _label(layout, "", 15, AMBER)
	var columns := HBoxContainer.new()
	columns.add_theme_constant_override("separation", 28)
	columns.size_flags_vertical = Control.SIZE_EXPAND_FILL
	layout.add_child(columns)
	var index_column := VBoxContainer.new()
	index_column.custom_minimum_size.x = 270
	index_column.add_theme_constant_override("separation", 12)
	var index_scroll := ScrollContainer.new()
	index_scroll.custom_minimum_size.x = 286
	index_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	index_scroll.follow_focus = true
	columns.add_child(index_scroll)
	index_scroll.add_child(index_column)
	for index in RECORDS.size():
		var button := Button.new()
		button.name = "Record%d" % (index + 1)
		button.custom_minimum_size = Vector2(270, 64)
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		button.add_theme_font_size_override("font_size", 16)
		button.pressed.connect(read_record.bind(index))
		index_column.add_child(button)
		_buttons.append(button)
	var note := _label(index_column, "Field notes from this run.\nExplore between encounters.\n\nE · read nearby record\nJ · revisit the archive\nEsc · return to the hub", 15, PAPER.darkened(0.2))
	note.custom_minimum_size.x = 270
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	var page := VBoxContainer.new()
	page.add_theme_constant_override("separation", 12)
	page.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	columns.add_child(page)
	_source = _label(page, "", 14, AMBER)
	_heading = _label(page, "", 26, PAPER)
	_heading.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_body = RichTextLabel.new()
	_body.bbcode_enabled = false
	_body.add_theme_font_size_override("normal_font_size", 20)
	_body.add_theme_color_override("default_color", PAPER)
	_body.add_theme_constant_override("normal_line_separation", 5)
	_body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_body.custom_minimum_size.y = 240
	_body.focus_mode = Control.FOCUS_ALL
	page.add_child(_body)
	_close = Button.new()
	_close.name = "CloseArchive"
	_close.text = "RETURN TO THE HUB [ESC / J]"
	_close.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_close.custom_minimum_size.y = 42
	_close.pressed.connect(func() -> void: close_requested.emit())
	layout.add_child(_close)
	_refresh()


func _refresh() -> void:
	_counter.text = "%d / %d RECORDS READ" % [_read.size(), _record_count()]
	for index in RECORDS.size():
		_buttons[index].visible = index < _record_count()
		var available: bool = _read.has(index) or index == _nearest or (index == 2 and _core_available) or (index == 3 and _meridian_available)
		_buttons[index].disabled = not available
		_buttons[index].text = "%02d  %s\n%s" % [index + 1,
			RECORDS[index].title if available else "UNRECOVERED RECORD",
			"READ" if _read.has(index) else ("AVAILABLE" if available else "FIND THE SIGNAL")]
	if _selected < 0:
		_source.text = "TRACES OF THE LAST SHIFT"
		_heading.text = "Someone kept the lights on."
		_body.text = "After clearing the drone, look for the amber archive markers near the arrival platform and maintenance bench.\n\nApproach a terminal and press E to read it. The core memory becomes available when the Warden is defeated.\n\nRecovered records can be revisited here during this run."
	else:
		_source.text = RECORDS[_selected].source
		_heading.text = RECORDS[_selected].title
		_body.text = RECORDS[_selected].body


func _label(parent: Node, text: String, font_size: int, color: Color) -> Label:
	var label := Label.new()
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.text = text
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", color)
	parent.add_child(label)
	return label
