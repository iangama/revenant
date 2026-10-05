extends CanvasLayer

const PANEL_BOUNDS := Rect2(360.0, 504.0, 560.0, 88.0)
const GRAPHITE := Color("10151d")
const NEUTRAL := Color("d5deea")

var _bounds := PANEL_BOUNDS
var _panel: Panel
var _style: StyleBoxFlat
var _rail: ColorRect
var _sector_label: Label
var _title_label: Label
var _transition_count := 0
var _last_sector := ""
var _last_title := ""
var _active := false
var _reduced_motion := false
var _generation := 0


func _ready() -> void:
	layer = 1
	_build_banner()


func set_bottom(bottom: float) -> void:
	_bounds = Rect2(360, bottom - 76, 560, 76)
	_panel.position = _bounds.position
	_panel.size = _bounds.size
	_rail.size.y = _bounds.size.y


func present(sector: String, title: String, accent: Color) -> bool:
	if sector.is_empty() or title.is_empty() or sector.length() > 24 or title.length() > 40:
		return false
	_transition_count += 1
	_generation += 1
	_last_sector = sector
	_last_title = title
	_active = true
	_sector_label.text = sector
	_sector_label.add_theme_color_override("font_color", accent)
	_title_label.text = title
	_style.border_color = Color(accent, 0.88)
	_rail.color = accent
	_panel.visible = true
	_panel.position = _bounds.position + Vector2(0.0, 8.0 if not _reduced_motion else 0.0)
	_panel.modulate.a = 0.0 if not _reduced_motion else 0.94

	var generation := _generation
	var tween := create_tween()
	if not _reduced_motion:
		tween.tween_property(_panel, "modulate:a", 0.94, 0.18)
		tween.parallel().tween_property(_panel, "position", _bounds.position, 0.18).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
	tween.tween_interval(1.35)
	tween.tween_property(_panel, "modulate:a", 0.0, 0.28 if not _reduced_motion else 0.12)
	tween.finished.connect(_finish.bind(generation))
	return true


func set_reduced_motion(enabled: bool) -> void:
	_reduced_motion = enabled


func presentation_state() -> Dictionary:
	return {
		"transition_count": _transition_count,
		"last_sector": _last_sector,
		"last_title": _last_title,
		"active": _active,
		"bounds": _bounds,
		"mouse_passthrough": _panel.mouse_filter == Control.MOUSE_FILTER_IGNORE,
		"reduced_motion": _reduced_motion,
	}


func _build_banner() -> void:
	_panel = Panel.new()
	_panel.name = "SectorTransitionBanner"
	_panel.position = _bounds.position
	_panel.size = PANEL_BOUNDS.size
	_panel.visible = false
	_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_style = StyleBoxFlat.new()
	_style.bg_color = Color(GRAPHITE, 0.94)
	_style.border_color = Color("35d0d0")
	_style.set_border_width_all(1)
	_style.set_corner_radius_all(3)
	_panel.add_theme_stylebox_override("panel", _style)
	add_child(_panel)

	_rail = ColorRect.new()
	_rail.name = "SectorRail"
	_rail.position = Vector2(0.0, 0.0)
	_rail.size = Vector2(5.0, PANEL_BOUNDS.size.y)
	_rail.color = Color("35d0d0")
	_rail.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_panel.add_child(_rail)

	_sector_label = Label.new()
	_sector_label.position = Vector2(24.0, 13.0)
	_sector_label.add_theme_font_size_override("font_size", 15)
	_panel.add_child(_sector_label)

	_title_label = Label.new()
	_title_label.position = Vector2(24.0, 36.0)
	_title_label.add_theme_font_size_override("font_size", 25)
	_title_label.add_theme_color_override("font_color", NEUTRAL)
	_panel.add_child(_title_label)


func _finish(generation: int) -> void:
	if generation != _generation:
		return
	_active = false
	_panel.visible = false
