extends Control

signal close_requested
signal action_requested(action: String)

const POINTS := {
	"empty_seat_memory": [-31, 0, 8], "empty_seat_returned": [-18, 0, 0],
	"held_connection_note": [-6, 0, -3], "held_connection_returned": [2, 0, 4],
	"supply_covered": [-18, 0, 0], "supply_service": [-18, 0, 0],
	"core_grounded": [7, 0, -6], "core_direct": [7, 0, -6],
}
const ACTIONS := {
	"empty_seat_memory": "Recover the seat memory", "empty_seat_returned": "Return the memory to the departure board",
	"held_connection_note": "Recover the maintenance note", "held_connection_returned": "Restore the note at the bench",
	"supply_covered": "Use the covered supply approach", "supply_service": "Use the western service path",
	"core_grounded": "Ground the relay before fighting", "core_direct": "Breach directly and fight first",
}
const EMPTY_FOUND := "A name survives beneath the glass: Vale. The shuttle left her seat empty. Take the memory to Meridian's departure board; someone left a route there."
const EMPTY_RETURNED := "THE EMPTY SEAT\n\nThe board recognizes Vale's last message: 'Do not wait for my badge. Keep the service path lit for whoever comes next.'\n\nThe empty seat was a promise to a person, not to the signal wearing her name. A maintenance route leads around the west end of the supply barrier. You can take it, or stay on the covered approach."
const HELD_FOUND := "The note records two pulses: ready, then come back. Its missing ending belongs at the maintenance bench near the relay entrance."
const HELD_RETURNED := "THE HELD CONNECTION\n\n'When the relay dropped me, Vale held the contact until I could stand. She did not ask the signal who it was. She waited for my answer.'\n\nThe bench restores the final instruction: grounding the relay drains the Warden's shield before the fight. A direct breach bypasses the shield feed, but the relay must still be drained after the guard falls."
const ENDINGS := {
	"signal_silent": "The false signal is silent. You return by the covered route, leaving the relay grounded. For the first time, the station waits for a real answer.",
	"route_lit": "The false signal is silent. The service lights remain on, and the grounded relay carries one clear pulse. Whoever comes next will have a way home.",
	"open_passage": "The false signal is silent. The Warden fell before the circuit was drained; the passage is open. Beyond the core, a distant relay answers once. You leave the choice of a reply to those who follow.",
}

var _snapshot := {}
var _position: Array = []
var _safe := false
var _pending := false
var _selected := ""
var _requested := ""
var _buttons := {}
var _body: RichTextLabel
var _summary: Label
var _status: Label
var _close: Button
var _records: Array[Button] = []


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var shade := ColorRect.new()
	shade.color = Color(0.02, 0.05, 0.08, 0.95)
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(shade)
	var panel := PanelContainer.new()
	panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	panel.offset_left = -500
	panel.offset_right = 500
	panel.offset_top = -315
	panel.offset_bottom = 315
	var style := StyleBoxFlat.new()
	style.bg_color = Color("101c26")
	style.set_content_margin_all(24)
	panel.add_theme_stylebox_override("panel", style)
	add_child(panel)
	var layout := VBoxContainer.new()
	layout.add_theme_constant_override("separation", 12)
	panel.add_child(layout)
	_label(layout, "CAMPAIGN JOURNAL", 26)
	_summary = _label(layout, "", 18)
	_label(layout, "Optional records grant no rewards. Choices can be changed here during chapter practice; they affect later attempts and ending text.", 17)
	var tabs := HBoxContainer.new()
	layout.add_child(tabs)
	for entry in [["empty_seat", "The empty seat"], ["held_connection", "The held connection"]]:
		var button := _button(tabs, entry[1])
		button.pressed.connect(_select_record.bind(entry[0]))
		_records.append(button)
	_body = RichTextLabel.new()
	_body.bbcode_enabled = false
	_body.focus_mode = Control.FOCUS_ALL
	_body.custom_minimum_size.y = 90
	_body.fit_content = true
	_body.scroll_active = false
	_body.add_theme_font_size_override("normal_font_size", 20)
	layout.add_child(_body)
	var actions := VBoxContainer.new()
	layout.add_child(actions)
	for id in ACTIONS:
		var button := _button(actions, ACTIONS[id])
		button.name = id.to_pascal_case()
		button.pressed.connect(_request_action.bind(id))
		_buttons[id] = button
	_status = _label(layout, "", 17)
	_close = _button(layout, "Return to the chapter")
	_close.pressed.connect(func() -> void: close_requested.emit())
	visible = false


func update_context(snapshot: Dictionary, position: Array, safe: bool, from_menu := false) -> void:
	_close.text = tr("Return to menu") if from_menu else tr("Return to the chapter")
	var changed: bool = _snapshot != snapshot or _position != position or _safe != safe
	_snapshot = snapshot.duplicate(true)
	_position = position.duplicate()
	_safe = safe
	if visible and not safe:
		close_requested.emit()
	if changed:
		_refresh()


func available_actions() -> Array:
	var active: Variant = _snapshot.get("active")
	if not _safe or not active is Dictionary:
		return []
	var result := []
	var chapter: String = active.get("chapter_id", "")
	var cp: int = active.get("checkpoint", -1)
	var story: Dictionary = _snapshot.get("story", {})
	for id in ACTIONS:
		if _position != POINTS[id]: continue
		if id.begins_with("empty_seat") and chapter == "meridian_readings" and cp in [1, 2]:
			if (id.ends_with("memory") and story.get("empty_seat", "unseen") == "unseen") or (id.ends_with("returned") and story.get("empty_seat") == "found"):
				result.append(id)
		if id.begins_with("held_connection") and chapter == "broken_supply_line" and cp in [2, 3]:
			if (id.ends_with("note") and story.get("held_connection", "unseen") == "unseen") or (id.ends_with("returned") and story.get("held_connection") == "found"):
				result.append(id)
		if id.begins_with("supply_") and chapter == "meridian_readings" and cp == 2: result.append(id)
		if id.begins_with("core_") and chapter == "counter_signal" and cp == 5: result.append(id)
	return result


func open_journal(nearby := false) -> bool:
	if not _safe or (nearby and available_actions().is_empty()): return false
	_selected = ""
	_status.text = ""
	visible = true
	_refresh()
	_body.grab_focus()
	_scroll_to_top.call_deferred()
	return true


func set_pending(pending: bool) -> void:
	_pending = pending
	_status.text = tr("Saving the record…") if pending else ""
	_refresh()


func show_result(status: String) -> void:
	_pending = false
	if status == "accepted" and (_requested.begins_with("empty_seat") or _requested.begins_with("held_connection")):
		_selected = "empty_seat" if _requested.begins_with("empty_seat") else "held_connection"
		_scroll_to_top.call_deferred()
	_status.text = tr({"accepted": "Record saved.", "rejected": "This interaction is not available here.", "unconfirmed": "Save not confirmed. Return to the chapter menu and resume to check the record."}.get(status, "Save not confirmed. Return to the chapter menu and resume to check the record."))
	_refresh()


func reset_for_connection() -> void:
	visible = false
	_snapshot.clear()
	_pending = false
	_selected = ""


func _refresh() -> void:
	if _body == null: return
	var story: Dictionary = _snapshot.get("story", {})
	var supply := tr("Service path") if story.get("supply") == "service" else tr("Covered approach")
	var core := tr("Direct breach") if story.get("core") == "direct" else tr("Grounded relay")
	_summary.text = supply + " · " + core
	var options := available_actions()
	for id in _buttons:
		_buttons[id].visible = id in options
		_buttons[id].disabled = _pending or (id.begins_with("supply_") and story.get("supply") == id.trim_prefix("supply_")) or (id.begins_with("core_") and story.get("core") == id.trim_prefix("core_"))
	_records[0].disabled = story.get("empty_seat", "unseen") == "unseen"
	_records[1].disabled = story.get("held_connection", "unseen") == "unseen"
	if _selected == "empty_seat":
		_body.text = tr(EMPTY_RETURNED if story.get("empty_seat") == "returned" else EMPTY_FOUND)
	elif _selected == "held_connection":
		_body.text = tr(HELD_RETURNED if story.get("held_connection") == "returned" else HELD_FOUND)
	elif int(_snapshot.get("cleared_chapters", 0)) == 6 and not _snapshot.get("active") is Dictionary:
		_body.text = tr(ENDINGS.get(story.get("epilogue"), ENDINGS.signal_silent))
	elif "supply_covered" in options:
		_body.text = tr("The departure board offers two supply approaches. Covered: approach the barrier, then flank west. Service: circle farther west and meet the sentinel near its terminal. Both recover the same power cell.")
	elif "core_grounded" in options:
		_body.text = tr("The isolated emitter reveals two ways in. Grounded: drain the shield under the Warden's pressure, then fight. Direct: fight without the shield, then drain the relay before entering the core.")
	elif "empty_seat_returned" in options or "empty_seat_memory" in options:
		_body.text = tr(EMPTY_FOUND)
	elif "held_connection_returned" in options or "held_connection_note" in options:
		_body.text = tr(HELD_FOUND)
	else:
		_body.text = tr("Meridian's gallery holds a memory of the empty seat. The supply line holds a maintenance note after its guard falls. Recover either record, then return it to its marked terminal. You can continue the campaign without them.")


func _label(parent: Node, text: String, size: int) -> Label:
	var label := Label.new()
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.add_theme_font_size_override("font_size", size)
	parent.add_child(label)
	return label


func _button(parent: Node, text: String) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size.y = 42
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(button)
	return button


func _select_record(id: String) -> void:
	_selected = id
	_refresh()
	_scroll_to_top.call_deferred()


func _request_action(id: String) -> void:
	_requested = id
	action_requested.emit(id)


func _scroll_to_top() -> void:
	var scroll := find_child("AccessibleSurfaceScroll", true, false) as ScrollContainer
	if scroll != null: scroll.scroll_vertical = 0
