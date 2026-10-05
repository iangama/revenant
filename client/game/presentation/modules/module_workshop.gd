extends Control

signal preview_requested(modules: Array[String])
signal combine_requested(module_id: String)
signal loadout_requested(expected_revision: int, modules: Array[String])
signal claim_requested(arc_id: String)
signal close_requested

const GRAPHITE := Color("10151d")
const BLUE_PETROL := Color("12313a")
const CYAN := Color("35d0d0")
const AMBER := Color("f5a524")
const MAGENTA := Color("d93678")
const NEUTRAL := Color("a9b8cc")
const MODULE_ORDER := [
	"module_force_matrix",
	"module_tempo_regulator",
	"module_reach_lattice",
	"module_ward_capacitor",
	"module_focus_lens",
	"module_cycle_bypass",
	"module_breach_shunt",
	"module_standoff_optic",
	"module_skirmish_drive",
	"module_ablative_shell",
]

var _state := {}
var _preview := {}
var _pending := {}
var _result := {}
var _activity_complete := false
var _reduced_flash := false
var _selection_initialized := false
var _primary_module := ""
var _module_buttons := {}
var _title: Label
var _balance: Label
var _lifecycle: Label
var _preset_choice: OptionButton
var _preset_description: Label
var _acquisition: Label
var _profile_weapon: OptionButton
var _selected_weapon := "pulse_rifle"
var _current_profiles: Label
var _preview_profiles: Label
var _result_label: Label
var _preview_button: Button
var _combine_button: Button
var _loadout_button: Button
var _close_button: Button
var _commissions := {}
var _commission_toggle: Button
var _commission_panel: VBoxContainer
var _commission_rows := {}


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build_surface()
	visible = false


func preview_preset(index: int) -> void:
	var presets: Array = preload("res://presentation/modules/build_presets.gd").PRESETS
	if index < 0 or index >= presets.size() or not _pending.is_empty():
		return
	var preset: Dictionary = presets[index]
	_preset_choice.select(index + 1)
	_set_selected_modules(preset.modules)
	_selection_initialized = true
	select_profile_weapon(preset.weapon)
	_preset_description.text = preset.description + "\nPreview only. Save modules after completion; equip the suggested weapon next session."
	preview_requested.emit(selected_modules())


func select_profile_weapon(item_id: String) -> void:
	_selected_weapon = item_id
	if _profile_weapon != null:
		for index in _profile_weapon.item_count:
			if _profile_weapon.get_item_metadata(index) == item_id:
				_profile_weapon.select(index)
	_refresh()


func open_workshop() -> void:
	visible = true
	_refresh()
	if _module_buttons.has(MODULE_ORDER[0]):
		(_module_buttons[MODULE_ORDER[0]] as CheckButton).grab_focus()


func close_workshop() -> void:
	visible = false


func reset_for_connection() -> void:
	_commissions.clear()
	if _commission_panel != null:
		_commission_panel.visible = false
	_state.clear()
	_preview.clear()
	_pending.clear()
	_result.clear()
	_activity_complete = false
	_selection_initialized = false
	_primary_module = ""
	if _preset_choice != null:
		_preset_choice.select(0)
		_preset_description.text = "Choose a suggested setup, then compare the server preview."
	if not _module_buttons.is_empty():
		_set_selected_modules([])
	visible = false
	_refresh()


func present(state: Dictionary, preview: Dictionary, pending: Dictionary, result: Dictionary, activity_complete: bool) -> void:
	var state_changed := state != _state
	_state = state.duplicate(true)
	_preview = preview.duplicate(true)
	_pending = pending.duplicate(true)
	_result = result.duplicate(true)
	_activity_complete = activity_complete
	if state_changed and not _selection_initialized and not _state.is_empty():
		_set_selected_modules(_state.get("equipped_modules", []))
		_selection_initialized = true
	_refresh()


func set_reduced_flash(enabled: bool) -> void:
	_reduced_flash = enabled


func present_commissions(state: Dictionary) -> void:
	_commissions = state.duplicate(true)
	_refresh_commissions()


func select_modules_for_validation(modules: Array) -> void:
	_set_selected_modules(modules)
	_selection_initialized = true
	_refresh()


func selected_modules() -> Array[String]:
	var selected: Array[String] = []
	for module_id in MODULE_ORDER:
		var button: CheckButton = _module_buttons.get(module_id)
		if button != null and button.button_pressed:
			selected.append(module_id)
	return selected


func focus_start() -> Control:
	return _module_buttons.get(MODULE_ORDER[0])


func presentation_state() -> Dictionary:
	return {
		"visible": visible,
		"panel_rect": Rect2(Vector2(150, 55), Vector2(980, 610)),
		"state_loaded": not _state.is_empty(),
		"selected_modules": selected_modules(),
		"preview_disabled": _preview_button.disabled,
		"combine_disabled": _combine_button.disabled,
		"loadout_disabled": _loadout_button.disabled,
		"current_text": _current_profiles.text,
		"comparison_weapon": _selected_weapon,
		"preview_text": _preview_profiles.text,
		"result_text": _result_label.text,
		"lifecycle_text": _lifecycle.text,
		"focus_names": _focus_names(),
		"reduced_flash": _reduced_flash,
	}


func _build_surface() -> void:
	var shade := ColorRect.new()
	shade.color = Color("071016", 0.88)
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	shade.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(shade)

	var layout := preload("res://presentation/settings/menu_layout.gd")
	var body := layout.panel(self, Rect2(150, 55, 980, 610), _panel_style())
	_title = _label(body, Vector2.ZERO, Vector2.ZERO, 24, CYAN, "MODULE WORKSHOP  •  SERVER AUTHORITY")
	_balance = _label(body, Vector2.ZERO, Vector2.ZERO, 16, NEUTRAL, "WAITING FOR MODULE SNAPSHOT")
	_lifecycle = _label(body, Vector2.ZERO, Vector2.ZERO, 14, AMBER, "CHANGES LOCKED UNTIL ACTIVITY COMPLETE")
	_build_commissions(body)
	var columns := layout.columns(body)
	var selection := layout.column(columns)
	var profiles := layout.column(columns)
	_label(selection, Vector2.ZERO, Vector2.ZERO, 15, AMBER, "CANDIDATE INPUT  •  SELECT UP TO THREE")
	for module_id: String in MODULE_ORDER:
		var button := CheckButton.new()
		button.name = module_id.replace("module_", "").to_pascal_case()
		button.custom_minimum_size.y = 48
		button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		button.add_theme_font_size_override("font_size", 14)
		_style_button(button, CYAN)
		button.toggled.connect(_on_module_toggled.bind(module_id))
		selection.add_child(button)
		_module_buttons[module_id] = button
	_preset_choice = OptionButton.new()
	_preset_choice.name = "BuildPreview"
	_preset_choice.custom_minimum_size.y = 40
	_preset_choice.add_theme_font_size_override("font_size", 15)
	_style_button(_preset_choice, AMBER)
	_preset_choice.add_item("PREVIEW A PLAYSTYLE")
	for preset in preload("res://presentation/modules/build_presets.gd").PRESETS:
		_preset_choice.add_item(preset.name)
	_preset_choice.item_selected.connect(func(index: int) -> void:
		if index > 0: preview_preset(index - 1))
	profiles.add_child(_preset_choice)
	_preset_description = _label(profiles, Vector2.ZERO, Vector2.ZERO, 14, AMBER, "Choose a suggested setup, then compare the server preview.")
	_acquisition = _label(profiles, Vector2.ZERO, Vector2.ZERO, 14, NEUTRAL, "")
	_profile_weapon = OptionButton.new()
	_profile_weapon.name = "ComparisonWeapon"
	_profile_weapon.custom_minimum_size.y = 40
	_profile_weapon.add_theme_font_size_override("font_size", 15)
	_style_button(_profile_weapon, CYAN)
	_profile_weapon.item_selected.connect(func(index: int) -> void:
		_selected_weapon = str(_profile_weapon.get_item_metadata(index))
		_refresh())
	profiles.add_child(_profile_weapon)
	_current_profiles = _label(profiles, Vector2.ZERO, Vector2.ZERO, 15, Color.WHITE, "CURRENT SERVER STATE\nWAITING FOR SNAPSHOT")
	_preview_profiles = _label(profiles, Vector2.ZERO, Vector2.ZERO, 15, NEUTRAL, "SERVER PREVIEW  •  NOT ACTIVE\nNO PREVIEW REQUESTED")
	var actions := layout.columns(body)
	_preview_button = _action_button(actions, "Preview", "PREVIEW", Vector2.ZERO, Vector2.ZERO, CYAN)
	_preview_button.pressed.connect(_request_preview)
	_combine_button = _action_button(actions, "CombineSelected", "COMBINE SELECTED", Vector2.ZERO, Vector2.ZERO, AMBER)
	_combine_button.pressed.connect(_request_combine)
	_loadout_button = _action_button(actions, "ApplyLoadout", "APPLY LOADOUT", Vector2.ZERO, Vector2.ZERO, MAGENTA)
	_loadout_button.pressed.connect(_request_loadout)
	_close_button = _action_button(actions, "Close", "CLOSE", Vector2.ZERO, Vector2.ZERO, NEUTRAL)
	_close_button.pressed.connect(_close)
	_result_label = _label(body, Vector2.ZERO, Vector2.ZERO, 16, NEUTRAL, "NO REQUEST PENDING\nAccepted changes apply next session; this actor never changes in place.")
	_refresh()


func _refresh() -> void:
	if _title == null:
		return
	var loaded := not _state.is_empty()
	var fragments := int(_state.get("fragments", 0))
	var revision := int(_state.get("loadout_revision", 0))
	var owned: Array = _state.get("owned_modules", [])
	var equipped: Array = _state.get("equipped_modules", [])
	_balance.text = "FRAGMENTS %d  •  OWNED %d/%d  •  LOADOUT %d/3  •  REVISION %d" % [fragments, owned.size(), _state.get("catalog", []).size(), equipped.size(), revision] if loaded else "WAITING FOR MODULE SNAPSHOT"
	_lifecycle.text = "ACTIVITY COMPLETE\nAPPLIES NEXT SESSION AFTER ACCEPTANCE" if _activity_complete else "ACTIVITY ACTIVE\nCOMBINE / LOADOUT LOCKED"
	_update_module_rows(owned)
	_update_acquisition(owned)
	_preset_choice.visible = _state.get("catalog_revision") == "m35-v2"
	_preset_description.visible = _preset_choice.visible
	_preset_choice.disabled = not _pending.is_empty()
	_profile_weapon.visible = _state.get("catalog_revision") in ["m35-v1", "m35-v2"]
	if _profile_weapon.visible and _profile_weapon.item_count != _state.get("weapons", []).size():
		_profile_weapon.clear()
		for weapon in _state.get("weapons", []):
			var item_id: String = weapon.get("item_id", "")
			_profile_weapon.add_item(item_id.replace("_", " ").to_upper())
			var index := _profile_weapon.item_count - 1
			_profile_weapon.set_item_metadata(index, item_id)
			if item_id == _selected_weapon:
				_profile_weapon.select(index)
	_current_profiles.text = _profile_text("CURRENT SERVER STATE", _state.get("weapons", []), int(_state.get("max_health", 0)), equipped) if loaded else "CURRENT SERVER STATE\nWAITING FOR SNAPSHOT"
	_preview_profiles.text = _profile_text("SERVER PREVIEW  •  NOT ACTIVE", _preview.get("weapons", []), int(_preview.get("max_health", 0)), _preview.get("requested_modules", [])) if not _preview.is_empty() else "SERVER PREVIEW  •  NOT ACTIVE\nSelect a candidate and request server preview.\nNo local stat calculation is shown."
	if _pending.get("kind") == "preview":
		_preview_profiles.text = "SERVER PREVIEW  •  NOT ACTIVE\nWAITING FOR SERVER"
	var pending := not _pending.is_empty()
	_preview_button.disabled = not loaded or pending
	_combine_button.disabled = not _can_combine(owned, fragments, pending)
	_loadout_button.disabled = not _can_apply_loadout(owned, equipped, pending)
	_update_result(pending)
	_refresh_commissions()


func _build_commissions(body: Control) -> void:
	_commission_toggle = _action_button(body, "Commissions", "ACQUISITION COMMISSIONS", Vector2.ZERO, Vector2.ZERO, AMBER)
	_commission_panel = VBoxContainer.new()
	_commission_panel.add_theme_constant_override("separation", 12)
	body.add_child(_commission_panel)
	_commission_panel.visible = false
	_commission_toggle.pressed.connect(func() -> void: _commission_panel.visible = not _commission_panel.visible)
	_label(_commission_panel, Vector2.ZERO, Vector2.ZERO, 15, NEUTRAL, "Each commission grants 2 fragments once per character. Spend them on any module recipe. Claim after completing a mission.")
	for arc_id: String in ["meridian", "routes", "prism"]:
		var label := _label(_commission_panel, Vector2.ZERO, Vector2.ZERO, 15, Color.WHITE, "")
		var button := _action_button(_commission_panel, "Claim" + arc_id.to_pascal_case(), "CLAIM 2 FRAGMENTS", Vector2.ZERO, Vector2.ZERO, CYAN)
		button.pressed.connect(func() -> void: claim_requested.emit(arc_id))
		_commission_rows[arc_id] = {"label": label, "button": button}


func _refresh_commissions() -> void:
	if _commission_toggle == null:
		return
	_commission_toggle.visible = not _commissions.is_empty()
	if _commissions.is_empty():
		_commission_panel.visible = false
		return
	var names := {"meridian": "MERIDIAN RECOVERY", "routes": "TWO APPROACHES", "prism": "PRISM BREAKER"}
	var requirements := {
		"meridian_recovered": "Survey Meridian, recover the log and finish that mission.",
		"breach_completed": "Win a mission through the Breach route.",
		"stabilize_completed": "Win a mission through the Stabilize route.",
		"prism_completed": "Defeat Prism Warden and finish that mission.",
	}
	for arc in _commissions.get("arcs", []):
		var row: Dictionary = _commission_rows[arc.arc_id]
		var status: String = {"locked": "IN PROGRESS", "ready": "READY TO CLAIM", "claimed": "CLAIMED"}[arc.status]
		var lines: Array[String] = ["%s  •  %s" % [names[arc.arc_id], status]]
		for requirement in arc.requirements:
			lines.append("%s  •  %s" % ["DONE" if requirement.completed else "TO DO", requirements[requirement.milestone]])
		row.label.text = "\n".join(lines)
		row.button.text = "CLAIMED" if arc.status == "claimed" else ("CLAIM 2 FRAGMENTS" if _commissions.can_claim else "FINISH MISSION TO CLAIM")
		row.button.disabled = arc.status != "ready" or not _commissions.can_claim or not _pending.is_empty()


func _update_module_rows(owned: Array) -> void:
	var definitions := {}
	for definition in _state.get("catalog", []):
		definitions[definition.get("module_id", "")] = definition
	for module_id in MODULE_ORDER:
		var button: CheckButton = _module_buttons[module_id]
		var definition: Dictionary = definitions.get(module_id, {})
		button.visible = not definition.is_empty()
		var ownership := "OWNED" if module_id in owned else "LOCKED"
		button.text = "%s  •  %s  •  COST %d\nDMG %+d%%  CD %+d%%  RANGE %+d  HP %+d" % [
			module_id.replace("module_", "").replace("_", " ").to_upper(),
			ownership,
			int(definition.get("recipe_fragments", 0)),
			int(definition.get("damage_basis_points", 0)) / 100,
			int(definition.get("cooldown_basis_points", 0)) / 100,
			int(definition.get("range_delta", 0)),
			int(definition.get("max_health_delta", 0)),
		]


func _profile_text(title: String, weapons: Array, max_health: int, modules: Array) -> String:
	var lines: Array[String] = [title, "MODULES  •  %s" % ("NONE" if modules.is_empty() else ", ".join(modules).replace("module_", "").replace("_", " ").to_upper())]
	for weapon in weapons:
		if _profile_weapon.visible and weapon.get("item_id") != _selected_weapon:
			continue
		lines.append("%s  •  DMG %d  RANGE %d  CD %d MS" % [
			str(weapon.get("item_id", "unknown")).replace("_", " ").to_upper(),
			int(weapon.get("effective_damage", 0)),
			int(weapon.get("effective_range", 0)),
			int(weapon.get("effective_cooldown_ms", 0)),
		])
		if weapon.get("item_id") == "scatter_caster":
			lines.append("Full damage within 3 units; half damage beyond.")
		elif weapon.get("item_id") == "rail_driver":
			lines.append("Cannot fire within 4 units; switch weapons when rushed.")
	lines.append("OPERATOR MAX HP  •  %d" % max_health)
	return "\n".join(lines)


func _update_result(pending: bool) -> void:
	if pending:
		_result_label.add_theme_color_override("font_color", NEUTRAL)
		_result_label.text = "REQUEST PENDING  •  %s\nNo fragment, ownership, loadout, stat, or actor change is projected yet." % str(_pending.get("kind", "server")).to_upper()
		return
	if _result.is_empty():
		_result_label.add_theme_color_override("font_color", NEUTRAL)
		_result_label.text = "NO REQUEST PENDING\nAccepted changes apply next session; this actor never changes in place."
		return
	var accepted: bool = _result.get("accepted", false)
	_result_label.add_theme_color_override("font_color", CYAN if accepted else MAGENTA)
	var disposition := "ACCEPTED"
	if accepted and _result.get("replayed", false):
		disposition = "ACCEPTED REPLAY"
	elif not accepted and _result.get("connection_lost", false):
		disposition = "CONNECTION LOST"
	elif not accepted:
		disposition = "REJECTED"
	_result_label.text = "%s  •  %s\n%s" % [disposition, str(_result.get("kind", "server")).to_upper(), str(_result.get("message", ""))]
	if accepted and _result.get("kind") in ["combine", "loadout"]:
		_result_label.text += "\nAPPLIES NEXT SESSION  •  CURRENT ACTOR UNCHANGED"


func _can_combine(owned: Array, fragments: int, pending: bool) -> bool:
	var target := _craft_target(owned)
	if not _activity_complete or pending or target.is_empty():
		return false
	for definition in _state.get("catalog", []):
		if definition.get("module_id") == target:
			return fragments >= int(definition.get("recipe_fragments", 0))
	return false


func _craft_target(owned: Array) -> String:
	var selected := selected_modules()
	if _primary_module in selected and _primary_module not in owned:
		return _primary_module
	for module_id in selected:
		if module_id not in owned:
			return module_id
	return ""


func _update_acquisition(owned: Array) -> void:
	_acquisition.visible = _state.get("catalog_revision") == "m35-v2"
	if not _acquisition.visible:
		return
	var missing := 0
	var total_cost := 0
	var next_cost := 0
	var target := _craft_target(owned)
	for definition in _state.get("catalog", []):
		var module_id: String = definition.get("module_id", "")
		if module_id in selected_modules() and module_id not in owned:
			missing += 1
			total_cost += int(definition.get("recipe_fragments", 0))
		if module_id == target:
			next_cost = int(definition.get("recipe_fragments", 0))
	var lines: Array[String] = []
	if missing > 0:
		lines.append("TO ACQUIRE  •  %d MODULES / %d FRAGMENTS" % [missing, total_cost])
		lines.append("NEXT CRAFT  •  %s / %d FRAGMENTS" % [target.replace("module_", "").replace("_", " ").to_upper(), next_cost])
		lines.append("After clearing the drone, compare mission rewards in Routes.")
	elif not selected_modules().is_empty():
		lines.append("ALL SELECTED MODULES OWNED")
		lines.append("Save this loadout after mission completion. It becomes active next session.")
	_acquisition.text = "\n".join(lines)


func _can_apply_loadout(owned: Array, equipped: Array, pending: bool) -> bool:
	var selected := selected_modules()
	if not _activity_complete or pending or selected.size() > 3 or selected == equipped:
		return false
	for module_id in selected:
		if module_id not in owned:
			return false
	return true


func _on_module_toggled(enabled: bool, module_id: String) -> void:
	if enabled:
		_primary_module = module_id
	elif _primary_module == module_id:
		var remaining := selected_modules()
		_primary_module = "" if remaining.is_empty() else remaining.back()
	if selected_modules().size() > 3:
		(_module_buttons[module_id] as CheckButton).set_pressed_no_signal(false)
		var remaining := selected_modules()
		_primary_module = "" if remaining.is_empty() else remaining.back()
		_result = {"kind": "selection", "accepted": false, "message": "select at most three modules; no request was sent"}
	_refresh()


func _set_selected_modules(modules: Array) -> void:
	for module_id in MODULE_ORDER:
		(_module_buttons[module_id] as CheckButton).set_pressed_no_signal(module_id in modules)
	_primary_module = "" if modules.is_empty() else str(modules.back())


func _request_preview() -> void:
	preview_requested.emit(selected_modules())


func _request_combine() -> void:
	var target := _craft_target(_state.get("owned_modules", []))
	if not target.is_empty():
		combine_requested.emit(target)


func _request_loadout() -> void:
	loadout_requested.emit(int(_state.get("loadout_revision", 0)), selected_modules())


func _close() -> void:
	close_requested.emit()


func _focus_names() -> Array[String]:
	var names: Array[String] = []
	for module_id in MODULE_ORDER:
		var button: CheckButton = _module_buttons[module_id]
		if button.visible:
			names.append(button.name)
	if _preset_choice.visible:
		names.append(_preset_choice.name)
	if _profile_weapon.visible:
		names.append(_profile_weapon.name)
	for button in [_preview_button, _combine_button, _loadout_button, _close_button]:
		names.append(button.name)
	return names


func _label(parent: Control, position: Vector2, size: Vector2, font_size: int, color: Color, text: String) -> Label:
	var label := Label.new()
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.position = position
	label.size = size
	label.text = text
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", color)
	parent.add_child(label)
	return label


func _action_button(parent: Control, name_value: String, text: String, position: Vector2, size: Vector2, accent: Color) -> Button:
	var button := Button.new()
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	button.custom_minimum_size.y = 44
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	button.name = name_value
	button.text = text
	button.position = position
	button.size = size
	_style_button(button, accent)
	parent.add_child(button)
	return button


func _style_button(button: BaseButton, accent: Color) -> void:
	button.focus_mode = Control.FOCUS_ALL
	button.add_theme_color_override("font_color", NEUTRAL)
	button.add_theme_color_override("font_hover_color", Color.WHITE)
	button.add_theme_stylebox_override("normal", _button_style(BLUE_PETROL, accent, 0.72))
	button.add_theme_stylebox_override("hover", _button_style(BLUE_PETROL.lightened(0.12), accent, 1.0))
	button.add_theme_stylebox_override("pressed", _button_style(accent, accent, 1.0))
	button.add_theme_stylebox_override("focus", _button_style(BLUE_PETROL.lightened(0.08), accent, 1.0))


func _panel_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(GRAPHITE, 0.98)
	style.border_color = CYAN
	style.set_border_width_all(2)
	style.set_corner_radius_all(5)
	return style


func _button_style(background: Color, border: Color, opacity: float) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(background, opacity)
	style.border_color = border
	style.set_border_width_all(1)
	style.set_corner_radius_all(3)
	return style
