extends Control

var layout: Dictionary
var player := Vector3.ZERO
var objectives := {}
var _title: Label
var _legend: Label
var _factor := 1.0
var _style: StyleBoxFlat


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_style = _background()
	_title = Label.new()
	_title.position = Vector2(12, 8)
	_title.add_theme_font_size_override("font_size", 16)
	_title.text = "MERIDIAN • N ↑"
	add_child(_title)
	_legend = Label.new()
	_legend.add_theme_font_size_override("font_size", 14)
	_legend.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_legend.text = "1 Arrival • 2 Lens • 3 Sky\n4 Log • Return east →"
	add_child(_legend)
	for label in [_title, _legend]:
		label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		label.add_theme_color_override("font_color", Color.WHITE)


func present(data: Dictionary, position_3d: Vector3, state: Dictionary, settings: Dictionary, bounds: Rect2) -> void:
	layout = data
	player = position_3d
	objectives = state
	var factor: float = settings.get("ui_scale", 1.0)
	_factor = 1.0 + (factor - 1.0) * 0.5
	position = bounds.position
	size = bounds.size
	_title.add_theme_font_size_override("font_size", roundi(16 * factor))
	_legend.add_theme_font_size_override("font_size", roundi(14 * factor))
	_legend.position = Vector2(218 * _factor, 42 * _factor)
	_legend.size = Vector2(size.x - _legend.position.x - 12, size.y - 50)
	_legend.text = "1 Arrival\n2 Lens\n3 Sky\n4 Log\nReturn east →"
	if objectives.get("meridian_return", {}).get("state") == "Active":
		_legend.text = "5 Return\n2 Lens\n3 Sky\n4 Log\nReturn east →"
	if objectives.get("meridian_return", {}).get("position", [-16, 0, 0])[0] == -31:
		_legend.text = "1 Arrival\n2 Lens\n3 Sky\n4 Log\n← Return west"
	if objectives.get("meridian_memory", {}).get("state") == "Completed":
		_legend.text += "\n✓ Memory"
	queue_redraw()


func _point(x: float, z: float) -> Vector2:
	return Vector2(20 + (x + 33) * 8, 40 + (z + 11) * 6.6) * _factor


func _draw() -> void:
	if layout == null or layout.is_empty():
		return
	draw_style_box(_style, Rect2(Vector2.ZERO, size))
	for r: Array in layout.floors:
		var corner := _point(r[0] - 0.5, r[2] - 0.5)
		var end := _point(r[1] + 0.5, r[3] + 0.5)
		draw_rect(Rect2(corner, end - corner), Color("496775"))
	var number := 0
	for station: Dictionary in layout.stations:
		number += 1
		if number > 4:
			continue
		if station.id == "meridian_arrival" and objectives.get("meridian_return", {}).get("position", [-16, 0, 0])[0] == -31:
			continue # The west return shares the log station's marker.
		var target: Array = station.position
		if station.id == "meridian_arrival" and objectives.get("meridian_return", {}).get("state") in ["Active", "Completed"]:
			target = objectives.get("meridian_return", {}).get("position", target)
		var point := _point(target[0], target[2])
		var done: bool = objectives.get(station.id, {}).get("state") == "Completed"
		var marker := number
		if number == 1 and objectives.get("meridian_return", {}).get("state") == "Active":
			marker = 5
			done = false
		draw_circle(point, 8 * _factor, Color("e5d8ab"))
		if done:
			draw_circle(point, 10 * _factor, Color.WHITE, false, 1)
		draw_string(ThemeDB.fallback_font, point + Vector2(-4, 5) * _factor, str(marker), HORIZONTAL_ALIGNMENT_LEFT, -1, roundi(13 * _factor), Color("10151d"))
	var p := _point(player.x, player.z)
	draw_polyline(PackedVector2Array([p + Vector2(0, -6), p + Vector2(5, 5), p + Vector2(-5, 5), p + Vector2(0, -6)]), Color.WHITE, 2)


func _background() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color("101923")
	style.border_color = Color("afc9c0")
	style.set_border_width_all(1)
	return style
