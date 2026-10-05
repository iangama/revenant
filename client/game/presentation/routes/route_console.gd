extends Control

signal state_requested
signal choice_requested(route_id: String)
signal close_requested

const GRAPHITE := Color("10151d")
const BLUE_PETROL := Color("12313a")
const CYAN := Color("35d0d0")
const AMBER := Color("f5a524")
const MAGENTA := Color("d93678")
const NEUTRAL := Color("a9b8cc")
const PANEL_RECT := Rect2(Vector2(100, 42), Vector2(1080, 636))
const ROUTE_ORDER := ["breach", "stabilize"]

var _state := {}
var _pending := {}
var _result := {}
var _summary := {}
var _player_actor_id := 0
var _reduced_flash := false
var _authority: Label
var _instruction: Label
var _route_labels := {}
var _route_buttons := {}
var _lifecycle: Label
var _refresh_button: Button
var _close_button: Button


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build_surface()
	visible = false


func open_console() -> void:
	visible = true
	_refresh()
	var focus := focus_start()
	if focus != null:
		focus.grab_focus()


func close_console() -> void:
	visible = false


func reset_for_connection() -> void:
	_state.clear()
	_pending.clear()
	_result.clear()
	_summary.clear()
	_player_actor_id = 0
	visible = false
	_refresh()


func present(
	state: Dictionary,
	pending: Dictionary,
	result: Dictionary,
	summary: Dictionary,
	player_actor_id: int
) -> void:
	_state = state.duplicate(true)
	_pending = pending.duplicate(true)
	_result = result.duplicate(true)
	_summary = summary.duplicate(true)
	_player_actor_id = player_actor_id
	_refresh()


func set_reduced_flash(enabled: bool) -> void:
	_reduced_flash = enabled


func select_for_validation(route_id: String) -> bool:
	var button: Button = _route_buttons.get(route_id)
	if button == null or button.disabled:
		return false
	choice_requested.emit(route_id)
	return true


func focus_start() -> Control:
	for route_id in ROUTE_ORDER:
		var button: Button = _route_buttons.get(route_id)
		if button != null and not button.disabled:
			return button
	return _refresh_button


func presentation_state() -> Dictionary:
	return {
		"visible": visible,
		"panel_rect": PANEL_RECT,
		"authority_text": _authority.text,
		"instruction_text": _instruction.text,
		"breach_text": (_route_labels.get("breach") as Label).text,
		"stabilize_text": (_route_labels.get("stabilize") as Label).text,
		"breach_disabled": (_route_buttons.get("breach") as Button).disabled,
		"stabilize_disabled": (_route_buttons.get("stabilize") as Button).disabled,
		"lifecycle_text": _lifecycle.text,
		"focus_names": ["Breach", "Stabilize", "Refresh", "Close"],
		"focus_start": focus_start().name if focus_start() != null else "",
		"reduced_flash": _reduced_flash,
		"option_count": int(not _state.is_empty()) * _state.get("routes", []).size(),
	}


func _build_surface() -> void:
	var shade := ColorRect.new()
	shade.color = Color("071016", 0.9)
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	shade.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(shade)

	var layout := preload("res://presentation/settings/menu_layout.gd")
	var body := layout.panel(self, PANEL_RECT, _panel_style())
	_label(body, Vector2.ZERO, Vector2.ZERO, 24, CYAN, "ROUTE CONSOLE  •  SERVER AUTHORITY")
	_authority = _label(body, Vector2.ZERO, Vector2.ZERO, 15, NEUTRAL, "NO AUTHORITATIVE ROUTE STATE")
	_instruction = _label(body, Vector2.ZERO, Vector2.ZERO, 15, AMBER, "Request server state to inspect route engineering tradeoffs.")
	var columns := layout.columns(body)
	_build_route_card(columns, "breach", Vector2.ZERO)
	_build_route_card(columns, "stabilize", Vector2.ZERO)
	_lifecycle = _label(body, Vector2.ZERO, Vector2.ZERO, 15, NEUTRAL, "UNAVAILABLE  •  NO ROUTE REQUEST SENT\nNo route, event, objective, reward, or timer has been projected.")
	var actions := layout.columns(body)
	_refresh_button = _action_button(actions, "Refresh", "REFRESH SERVER STATE", Vector2.ZERO, Vector2.ZERO, CYAN)
	_refresh_button.pressed.connect(func() -> void: state_requested.emit())
	_close_button = _action_button(actions, "Close", "CLOSE [ESC / R]", Vector2.ZERO, Vector2.ZERO, NEUTRAL)
	_close_button.pressed.connect(func() -> void: close_requested.emit())
	_refresh()


func _build_route_card(parent: Control, route_id: String, _position: Vector2) -> void:
	var layout := preload("res://presentation/settings/menu_layout.gd")
	var body := layout.panel(parent, Rect2(), _card_style())
	body.get_parent().size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var detail := _label(body, Vector2.ZERO, Vector2.ZERO, 14, Color.WHITE, "SERVER OPTION\nAWAITING AUTHORITATIVE ROUTE STATE")
	detail.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_route_labels[route_id] = detail
	var button := _action_button(body, route_id.to_pascal_case(), "WAITING FOR SERVER", Vector2.ZERO, Vector2.ZERO, AMBER)
	button.disabled = true
	button.pressed.connect(func() -> void: choice_requested.emit(route_id))
	_route_buttons[route_id] = button


func _refresh() -> void:
	if _authority == null:
		return
	var initial_pending: bool = _pending.get("kind") == "state" and _state.is_empty()
	if initial_pending:
		_authority.text = "CAPABILITY REQUEST PENDING  •  AWAITING SERVER"
		_instruction.text = "No route candidate or selection is displayed before the authoritative response."
		_clear_options()
	else:
		_refresh_authoritative_state()
	_refresh_lifecycle()
	_refresh_buttons()


func _refresh_authoritative_state() -> void:
	if _state.is_empty():
		_authority.text = "NO AUTHORITATIVE ROUTE STATE"
		_instruction.text = "Request server state to inspect route engineering tradeoffs."
		_clear_options()
		return
	var participants: Array = _state.get("participant_actor_ids", [])
	var capable: Array = _state.get("capable_actor_ids", [])
	var leader := int(_state.get("leader_actor_id", 0))
	var phase := str(_state.get("phase", "unknown"))
	if _summary.get("outcome") == "succeeded":
		phase = "succeeded"
	elif _summary.get("outcome") == "failed_timeout":
		phase = "failed"
	_authority.text = "PHASE %s  •  LEADER ACTOR %d  •  CAPABLE %d/%d" % [
		phase.replace("_", " ").to_upper(),
		leader,
		capable.size(),
		participants.size(),
	]
	if leader == _player_actor_id:
		_instruction.text = "YOU ARE THE SERVER-DECLARED LEADER  •  Choices unlock only in open phase with complete capability."
	else:
		_instruction.text = "ACTOR %d DECIDES  •  This participant can inspect the same server facts but cannot submit a choice." % leader
	var options_by_id := {}
	for option in _state.get("routes", []):
		options_by_id[option.get("route_id", "")] = option
	for route_id in ROUTE_ORDER:
		var detail: Label = _route_labels.get(route_id)
		var option: Dictionary = options_by_id.get(route_id, {})
		detail.text = _route_option_text(option) if not option.is_empty() else "SERVER OPTION\nAUTHORITATIVE OPTION MISSING"


func _clear_options() -> void:
	for route_id in ROUTE_ORDER:
		(_route_labels.get(route_id) as Label).text = "SERVER OPTION\nAWAITING AUTHORITATIVE ROUTE STATE"
		var button: Button = _route_buttons.get(route_id)
		button.text = "WAITING FOR SERVER"
		button.disabled = true


func _refresh_buttons() -> void:
	var can_choose: bool = (
		not _state.is_empty()
		and _state.get("phase") == "choice_open"
		and _state.get("all_capable", false)
		and int(_state.get("leader_actor_id", 0)) == _player_actor_id
		and _state.get("selection", null) == null
		and _result.get("selection", null) == null
		and _pending.is_empty()
		and _summary.is_empty()
	)
	for route_id in ROUTE_ORDER:
		var button: Button = _route_buttons.get(route_id)
		button.text = "SELECT %s" % route_id.to_upper() if not _state.is_empty() else "WAITING FOR SERVER"
		button.disabled = not can_choose


func _refresh_lifecycle() -> void:
	if not _summary.is_empty():
		_lifecycle.text = _summary_text(_summary)
		_lifecycle.add_theme_color_override("font_color", CYAN if _summary.get("outcome") == "succeeded" else MAGENTA)
		return
	if not _pending.is_empty():
		_lifecycle.add_theme_color_override("font_color", NEUTRAL)
		if _pending.get("kind") == "choice":
			_lifecycle.text = "CHOICE PENDING  •  REQUESTED %s\nNo route, event, objective, reward, effect, or timer is accepted yet." % str(_pending.get("route_id", "")).to_upper()
		else:
			_lifecycle.text = "STATE PENDING  •  AWAITING SERVER\nNo candidate or capability is inferred locally."
		return
	if not _result.is_empty():
		_lifecycle.text = _result_text(_result)
		_lifecycle.add_theme_color_override("font_color", CYAN if _result.get("accepted", false) else MAGENTA)
		return
	_lifecycle.add_theme_color_override("font_color", NEUTRAL)
	_lifecycle.text = "AVAILABLE FOR EXPLICIT REQUEST\nNo route, event, objective, reward, or timer has been projected."


func _route_option_text(option: Dictionary) -> String:
	var reward: Dictionary = option.get("reward", {})
	var lines: Array[String] = [
		"%s  •  SERVER CANDIDATE" % str(option.get("route_id", "")).to_upper(),
		"OBJECTIVES  •  %s" % _path_text(option.get("objective_path", [])),
		"BUDGET  •  %d MS" % int(option.get("duration_budget_ms", 0)),
		"REWARD PLAN  •  %d FRAGMENT(S)  •  %d XP" % [
			int(reward.get("item_quantity", 0)),
			int(reward.get("experience", 0)),
		],
		"POSSIBLE SERVER EVENTS  •  ONE RESOLVES AFTER ACCEPTANCE",
	]
	for event in option.get("events", []):
		var effect: Dictionary = event.get("effect", {})
		lines.append("%s  •  WARDEN HP %d BP  •  COUNTER %d" % [
			str(event.get("event_id", "")).replace("_", " ").to_upper(),
			int(effect.get("warden_health_basis_points", 0)),
			int(effect.get("warden_counter_damage", 0)),
		])
		if event.get("event_id") == "arc_surge":
			lines.append("ARC WARDEN  •  Repositions after each hit. Track its jumps.")
	return "\n".join(lines)


func _result_text(result: Dictionary) -> String:
	if not result.get("accepted", false):
		if result.get("connection_lost", false):
			return "CONNECTION LOST  •  ROUTE NOT RESUMED\n%s" % str(result.get("message", "Reconnect from the entry screen."))
		return "REJECTED  •  NO ROUTE CHANGE\n%s" % str(result.get("message", "Server rejected the request."))
	if result.get("kind") == "state":
		return "SERVER STATE RECEIVED\n%s" % str(result.get("message", "Capability state is authoritative."))
	var selection: Dictionary = result.get("selection", {})
	var reward: Dictionary = selection.get("reward", {})
	var effect: Dictionary = selection.get("effect", {})
	return "%s  •  %s  •  EVENT %s  •  SEED %d\nWARDEN HP %d BP  •  COUNTER %d  •  PATH %s  •  REWARD %d FRAGMENT(S) / %d XP" % [
		"ACCEPTED REPLAY" if result.get("replayed", false) else "ACCEPTED",
		str(selection.get("route_id", "")).to_upper(),
		str(selection.get("event_id", "")).replace("_", " ").to_upper(),
		int(selection.get("seed", 0)),
		int(effect.get("warden_health_basis_points", 0)),
		int(effect.get("warden_counter_damage", 0)),
		_path_text(selection.get("objective_path", [])),
		int(reward.get("item_quantity", 0)),
		int(reward.get("experience", 0)),
	]


func _summary_text(summary: Dictionary) -> String:
	if summary.get("outcome") == "failed_timeout":
		return "FAILED  •  DEADLINE EXCEEDED  •  %d / %d MS\nNO ROUTE REWARD  •  %d SERVER TRANSITIONS  •  %s" % [
			int(summary.get("elapsed_ms", 0)),
			int(summary.get("duration_budget_ms", 0)),
			summary.get("transitions", []).size(),
			str(summary.get("no_reward_reason", "server failure")),
		]
	var reward: Dictionary = summary.get("reward", {})
	return "SUCCEEDED  •  %s / %s  •  %d / %d MS\n%d SERVER TRANSITIONS  •  GRANT %d FRAGMENT(S) / %d XP" % [
		str(summary.get("route_id", "")).to_upper(),
		str(summary.get("event_id", "")).replace("_", " ").to_upper(),
		int(summary.get("elapsed_ms", 0)),
		int(summary.get("duration_budget_ms", 0)),
		summary.get("transitions", []).size(),
		int(reward.get("item_quantity", 0)),
		int(reward.get("experience", 0)),
	]


func _path_text(path: Array) -> String:
	var labels: Array[String] = []
	for objective in path:
		labels.append(str(objective).replace("_", " ").to_upper())
	return " → ".join(labels)


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
	button.focus_mode = Control.FOCUS_ALL
	button.add_theme_color_override("font_color", NEUTRAL)
	button.add_theme_color_override("font_hover_color", Color.WHITE)
	button.add_theme_stylebox_override("normal", _button_style(BLUE_PETROL, accent, 0.72))
	button.add_theme_stylebox_override("hover", _button_style(BLUE_PETROL.lightened(0.12), accent, 1.0))
	button.add_theme_stylebox_override("pressed", _button_style(accent, accent, 1.0))
	button.add_theme_stylebox_override("focus", _button_style(BLUE_PETROL.lightened(0.08), accent, 1.0))
	parent.add_child(button)
	return button


func _panel_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(GRAPHITE, 0.99)
	style.border_color = CYAN
	style.set_border_width_all(2)
	style.set_corner_radius_all(5)
	return style


func _card_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(BLUE_PETROL, 0.72)
	style.border_color = AMBER
	style.set_border_width_all(1)
	style.set_corner_radius_all(3)
	return style


func _button_style(background: Color, border: Color, opacity: float) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(background, opacity)
	style.border_color = border
	style.set_border_width_all(1)
	style.set_corner_radius_all(3)
	return style
