extends Control

const GRAPHITE := Color("10151d")
const BLUE_PETROL := Color("12313a")
const CYAN := Color("35d0d0")
const AMBER := Color("f5a524")
const MAGENTA := Color("d93678")
const NEUTRAL := Color("a9b8cc")

var _panels := 0
var _semantic_accents := {}


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)


func add_panel(rect: Rect2, title: String, accent: Color) -> Panel:
	var panel := Panel.new()
	panel.name = title.to_pascal_case().replace(" ", "") + "Panel"
	panel.position = rect.position
	panel.size = rect.size
	panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var style := StyleBoxFlat.new()
	style.bg_color = Color(GRAPHITE, 0.91)
	style.border_color = Color(accent, 0.72)
	style.set_border_width_all(1)
	style.set_corner_radius_all(3)
	style.content_margin_left = 16.0
	style.content_margin_right = 16.0
	panel.add_theme_stylebox_override("panel", style)
	add_child(panel)
	_panels += 1
	_semantic_accents[accent.to_html(false)] = true

	var rail := ColorRect.new()
	rail.name = "SemanticRail"
	rail.color = accent
	rail.position = Vector2(0.0, 0.0)
	rail.size = Vector2(4.0, rect.size.y)
	rail.mouse_filter = Control.MOUSE_FILTER_IGNORE
	panel.add_child(rail)
	return panel


func style_button(button: Button, accent: Color, compact := false) -> void:
	button.add_theme_font_size_override("font_size", 16 if compact else 18)
	button.add_theme_color_override("font_color", NEUTRAL)
	button.add_theme_color_override("font_hover_color", Color.WHITE)
	button.add_theme_color_override("font_pressed_color", GRAPHITE)
	button.add_theme_stylebox_override("normal", _button_style(BLUE_PETROL, accent, 0.72))
	button.add_theme_stylebox_override("hover", _button_style(BLUE_PETROL.lightened(0.12), accent, 1.0))
	button.add_theme_stylebox_override("pressed", _button_style(accent, accent, 1.0))
	button.add_theme_stylebox_override("focus", _button_style(BLUE_PETROL.lightened(0.08), accent, 1.0))


func make_bar(rect: Rect2, fill_color: Color) -> ProgressBar:
	var bar := ProgressBar.new()
	bar.position = rect.position
	bar.size = rect.size
	bar.min_value = 0.0
	bar.max_value = 100.0
	bar.value = 100.0
	bar.show_percentage = false
	var background := StyleBoxFlat.new()
	background.bg_color = Color("071016")
	background.border_color = Color(NEUTRAL, 0.3)
	background.set_border_width_all(1)
	var fill := StyleBoxFlat.new()
	fill.bg_color = fill_color
	fill.corner_radius_top_right = 2
	fill.corner_radius_bottom_right = 2
	bar.add_theme_stylebox_override("background", background)
	bar.add_theme_stylebox_override("fill", fill)
	add_child(bar)
	_semantic_accents[fill_color.to_html(false)] = true
	return bar


func presentation_state() -> Dictionary:
	return {
		"panel_count": _panels,
		"semantic_accent_count": _semantic_accents.size(),
		"mouse_passthrough": mouse_filter == Control.MOUSE_FILTER_IGNORE,
	}


func _button_style(background: Color, border: Color, opacity: float) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(background, opacity)
	style.border_color = border
	style.set_border_width_all(1)
	style.set_corner_radius_all(3)
	return style
