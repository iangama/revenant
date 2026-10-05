extends RefCounted

var _surfaces := {}


func apply(surface: Control, factor: float) -> void:
	if surface == null:
		return
	var id := surface.get_instance_id()
	if not _surfaces.has(id):
		var panel: Control
		for child in surface.get_children():
			if child is Panel or child is PanelContainer:
				panel = child
				break
		if panel == null:
			return
		_surfaces[id] = {"panel": panel, "rect": panel.get_rect(), "scroll": null}
	var state: Dictionary = _surfaces[id]
	var panel: Control = state.panel
	var authored: Rect2 = state.rect
	if state.scroll == null:
		var scroll := ScrollContainer.new()
		scroll.name = "AccessibleSurfaceScroll"
		scroll.follow_focus = true
		scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		scroll.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		scroll.offset_left = 24
		scroll.offset_right = -24
		scroll.offset_top = 24
		scroll.offset_bottom = -24
		surface.add_child(scroll)
		var extent := Control.new()
		extent.name = "ScaledSurfaceExtent"
		extent.mouse_filter = Control.MOUSE_FILTER_PASS
		scroll.add_child(extent)
		panel.reparent(extent)
		state.scroll = scroll
		state.extent = extent
		panel.resized.connect(_resize_extent.bind(state))
	state.factor = factor
	panel.set_anchors_preset(Control.PRESET_TOP_LEFT)
	var available := Vector2(1216, 672)
	panel.size = Vector2(minf(authored.size.x, available.x / factor), authored.size.y)
	panel.scale = Vector2.ONE * factor
	_resize_extent(state)
	state.scroll.scroll_horizontal = 0
	state.scroll.scroll_vertical = 0


func _resize_extent(state: Dictionary) -> void:
	var factor: float = state.get("factor", 1.0)
	var scaled: Vector2 = state.panel.size * factor
	state.extent.custom_minimum_size = Vector2(1216, maxf(scaled.y, 672))
	state.panel.position = (Vector2(1216, 672) - scaled).max(Vector2.ZERO) * 0.5
