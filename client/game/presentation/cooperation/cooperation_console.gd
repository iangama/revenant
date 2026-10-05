extends Control

signal state_requested
signal start_requested
signal ping_requested
signal revive_requested
signal close_requested

const GRAPHITE := Color("10151d")
const BLUE_PETROL := Color("12313a")
const CYAN := Color("35d0d0")
const AMBER := Color("f5a524")
const MAGENTA := Color("d93678")
const NEUTRAL := Color("a9b8cc")
const PANEL_RECT := Rect2(Vector2(62, 22), Vector2(1156, 676))
const CONTRIBUTION_ORDER := [
	["anchor_arrived", "01  ANCHOR ARRIVED"],
	["pinged", "02  RELAY CONSOLE PINGED"],
	["runner_arrived", "03  RUNNER ARRIVED"],
	["revived", "04  RUNNER REVIVED"],
	["warden_completed", "05  WARDEN COMPLETED"],
]

var _state := {}
var _pending := {}
var _result := {}
var _life_states := {}
var _summary := {}
var _player_actor_id := 0
var _reduced_flash := false
var _authority: Label
var _instruction: Label
var _role_labels: Array[Label] = []
var _operation_detail: Label
var _contributions: Label
var _lifecycle: Label
var _start_button: Button
var _ping_button: Button
var _revive_button: Button
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
	_life_states.clear()
	_summary.clear()
	_player_actor_id = 0
	visible = false
	_refresh()


func present(
	state: Dictionary,
	pending: Dictionary,
	result: Dictionary,
	life_states: Dictionary,
	summary: Dictionary,
	player_actor_id: int
) -> void:
	_state = state.duplicate(true)
	_pending = pending.duplicate(true)
	_result = result.duplicate(true)
	_life_states = life_states.duplicate(true)
	_summary = summary.duplicate(true)
	_player_actor_id = player_actor_id
	_refresh()


func set_reduced_flash(enabled: bool) -> void:
	_reduced_flash = enabled


func activate_for_validation(action: String) -> bool:
	var buttons := {
		"start": _start_button,
		"ping": _ping_button,
		"revive": _revive_button,
		"refresh": _refresh_button,
	}
	var button: Button = buttons.get(action)
	if button == null or button.disabled:
		return false
	button.pressed.emit()
	return true


func focus_start() -> Control:
	for button in [_start_button, _ping_button, _revive_button, _refresh_button]:
		if button != null and not button.disabled:
			return button
	return _close_button


func presentation_state() -> Dictionary:
	return {
		"visible": visible,
		"panel_rect": PANEL_RECT,
		"authority_text": _authority.text,
		"instruction_text": _instruction.text,
		"anchor_text": _role_labels[0].text,
		"runner_text": _role_labels[1].text,
		"operation_text": _operation_detail.text,
		"contribution_text": _contributions.text,
		"lifecycle_text": _lifecycle.text,
		"start_disabled": _start_button.disabled,
		"ping_disabled": _ping_button.disabled,
		"revive_disabled": _revive_button.disabled,
		"focus_names": ["Start", "Ping", "Revive", "Refresh", "Close"],
		"focus_start": focus_start().name if focus_start() != null else "",
		"reduced_flash": _reduced_flash,
		"mouse_blocking": mouse_filter == Control.MOUSE_FILTER_STOP,
		"participant_card_count": _role_labels.size(),
	}


func _build_surface() -> void:
	var shade := ColorRect.new()
	shade.color = Color("071016", 0.92)
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	shade.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(shade)

	var layout := preload("res://presentation/settings/menu_layout.gd")
	var body := layout.panel(self, PANEL_RECT, _panel_style())
	_label(body, Vector2.ZERO, Vector2.ZERO, 23, CYAN, "COOPERATION LINK  •  SERVER AUTHORITY")
	_authority = _label(body, Vector2.ZERO, Vector2.ZERO, 15, NEUTRAL, "NO AUTHORITATIVE COOPERATION STATE")
	_instruction = _label(body, Vector2.ZERO, Vector2.ZERO, 15, AMBER, "Request server capability. No role or operation is inferred locally.")
	var columns := layout.columns(body)
	_build_role_card(columns, "ANCHOR", Vector2.ZERO, CYAN)
	_build_role_card(columns, "RUNNER", Vector2.ZERO, AMBER)
	_operation_detail = _label(body, Vector2.ZERO, Vector2.ZERO, 14, NEUTRAL, "NO OPERATION  •  TARGETS AND BUDGETS ARE SERVER-OWNED")
	_contributions = _label(body, Vector2.ZERO, Vector2.ZERO, 14, Color.WHITE, "CONTRIBUTIONS  •  AWAITING AUTHORITATIVE OPERATION")
	_lifecycle = _label(body, Vector2.ZERO, Vector2.ZERO, 15, NEUTRAL, "AVAILABLE FOR EXPLICIT REQUEST\nNo cooperation role, timer, life state, contribution, or reward is projected.")
	var actions := GridContainer.new()
	actions.columns = 3
	actions.add_theme_constant_override("h_separation", 10)
	actions.add_theme_constant_override("v_separation", 10)
	body.add_child(actions)
	_start_button = _action_button(actions, "Start", "START OPERATION", Vector2.ZERO, Vector2.ZERO, CYAN)
	_start_button.pressed.connect(func() -> void: start_requested.emit())
	_ping_button = _action_button(actions, "Ping", "PING RELAY CONSOLE", Vector2.ZERO, Vector2.ZERO, AMBER)
	_ping_button.pressed.connect(func() -> void: ping_requested.emit())
	_revive_button = _action_button(actions, "Revive", "REVIVE RUNNER", Vector2.ZERO, Vector2.ZERO, MAGENTA)
	_revive_button.pressed.connect(func() -> void: revive_requested.emit())
	_refresh_button = _action_button(actions, "Refresh", "REFRESH", Vector2.ZERO, Vector2.ZERO, CYAN)
	_refresh_button.pressed.connect(func() -> void: state_requested.emit())
	_close_button = _action_button(actions, "Close", "CLOSE [ESC / C]", Vector2.ZERO, Vector2.ZERO, NEUTRAL)
	_close_button.pressed.connect(func() -> void: close_requested.emit())
	_refresh()


func _build_role_card(parent: Control, role_name: String, _position: Vector2, accent: Color) -> void:
	var body := preload("res://presentation/settings/menu_layout.gd").panel(parent, Rect2(), _card_style(accent))
	body.get_parent().size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var detail := _label(body, Vector2.ZERO, Vector2.ZERO, 14, Color.WHITE, "%s  •  WAITING FOR SERVER\nNo actor, health, life, or responsibility assigned." % role_name)
	_role_labels.append(detail)


func _refresh() -> void:
	if _authority == null:
		return
	if _pending.get("kind") == "state" and _state.is_empty():
		_authority.text = "CAPABILITY REQUEST PENDING  •  AWAITING SERVER"
		_instruction.text = "No participant, role, target, timer, contribution, life state, or reward is displayed before acceptance."
		_clear_authoritative_details()
	else:
		_refresh_authoritative_state()
	_refresh_lifecycle()
	_refresh_buttons()


func _refresh_authoritative_state() -> void:
	if _state.is_empty():
		_authority.text = "NO AUTHORITATIVE COOPERATION STATE"
		_instruction.text = "Request server capability. No role or operation is inferred locally."
		_clear_authoritative_details()
		return
	var participants: Array = _state.get("participant_actor_ids", [])
	var capable: Array = _state.get("capable_actor_ids", [])
	var phase := str(_state.get("phase", "unknown"))
	if not _summary.is_empty():
		phase = "succeeded" if _summary.get("outcome") == "succeeded" else "failed"
	_authority.text = "PHASE %s  •  SESSION %s  •  CAPABLE %d/%d" % [
		phase.replace("_", " ").to_upper(),
		str(_state.get("session_id", "unknown")),
		capable.size(),
		participants.size(),
	]
	var local_role := _local_role()
	if local_role.is_empty():
		_instruction.text = "NO SERVER-DECLARED LOCAL ROLE  •  Inspect state or wait for both participants."
	else:
		_instruction.text = "YOU ARE %s  •  ACTOR %d  •  Only the matching server-authorized actions can be sent." % [local_role.to_upper(), _player_actor_id]
	var operation_value = _state.get("operation", null)
	if not operation_value is Dictionary:
		_clear_operation_details()
		return
	var operation: Dictionary = operation_value
	_refresh_participants(operation)
	var timing: Dictionary = operation.get("timing", {})
	var reward: Dictionary = operation.get("reward", {})
	_operation_detail.text = "TARGETS  •  ANCHOR [3,0,3]  •  CONSOLE [4,0,3]  •  OBSERVED %d MS\nBUDGETS  •  OP %d  •  PING %d  •  REVIVE WINDOW %d  •  CHANNEL %d MS  •  RANGE² %d  •  RESTORE %d HP  •  REWARD %d FRAGMENT(S) / %d XP EACH\n%s" % [
		int(operation.get("observed_elapsed_ms", 0)),
		int(timing.get("operation_duration_ms", 0)),
		int(timing.get("ping_ttl_ms", 0)),
		int(timing.get("revive_window_ms", 0)),
		int(timing.get("revive_channel_ms", 0)),
		int(timing.get("maximum_distance_squared", 0)),
		int(timing.get("revive_health", 0)),
		int(reward.get("item_quantity", 0)),
		int(reward.get("experience", 0)),
		_ping_text(operation),
	]
	_refresh_contributions(_summary.get("contributions", operation.get("contributions", {})))


func _refresh_participants(operation: Dictionary) -> void:
	var participants: Array = operation.get("participants", [])
	for index in 2:
		var participant: Dictionary = participants[index]
		var actor_id := int(participant.get("actor_id", 0))
		var life: Dictionary = _life_states.get(actor_id, {})
		var current_health := int(life.get("health_after", participant.get("current_health", 0)))
		var life_name := str(life.get("life_after", participant.get("life", "unknown")))
		var role_name := str(participant.get("role", "unknown")).to_upper()
		var responsibility := "ARRIVE [3,0,3]  →  PING  →  REVIVE RUNNER" if role_name == "ANCHOR" else "WAIT FOR PING  →  ARRIVE [4,0,3]  →  SURVIVE REVIVE"
		_role_labels[index].text = "%s  •  ACTOR %d%s\nLIFE %s  •  HP %d/%d\nREQUIRED  •  %s\n%s" % [
			role_name,
			actor_id,
			"  •  YOU" if actor_id == _player_actor_id else "",
			life_name.to_upper(),
			current_health,
			int(participant.get("max_health", 0)),
			responsibility,
			_life_detail(life),
		]


func _refresh_contributions(contributions: Dictionary) -> void:
	var items: Array[String] = []
	for entry in CONTRIBUTION_ORDER:
		items.append("%s %s" % ["[DONE]" if contributions.get(entry[0], false) else "[WAIT]", entry[1]])
	items.append("REVIVE COUNT %d / 1" % int(contributions.get("revive_count", 0)))
	_contributions.text = "CONTRIBUTIONS  •  %s\n%s" % ["  |  ".join(items.slice(0, 3)), "  |  ".join(items.slice(3))]


func _clear_authoritative_details() -> void:
	for index in 2:
		var role_name := "ANCHOR" if index == 0 else "RUNNER"
		_role_labels[index].text = "%s  •  WAITING FOR SERVER\nNo actor, health, life, or responsibility assigned." % role_name
	_clear_operation_details()


func _clear_operation_details() -> void:
	_operation_detail.text = "NO OPERATION  •  TARGETS AND BUDGETS ARE SERVER-OWNED"
	_contributions.text = "CONTRIBUTIONS  •  AWAITING AUTHORITATIVE OPERATION"


func _refresh_buttons() -> void:
	var operation: Dictionary = _state.get("operation", {}) if _state.get("operation", null) is Dictionary else {}
	var role := _local_role()
	var no_pending := _pending.is_empty() and _summary.is_empty()
	_start_button.disabled = not (
		no_pending
		and _state.get("phase") == "eligible"
		and _state.get("all_capable", false)
		and _state.get("operation", null) == null
		and not _state.get("participant_actor_ids", []).is_empty()
		and int(_state.get("participant_actor_ids", [0])[0]) == _player_actor_id
	)
	_ping_button.disabled = not (no_pending and operation.get("phase") == "awaiting_ping" and role == "anchor" and operation.get("ping", null) == null and not (_result.get("kind") == "ping" and _result.get("accepted", false)))
	_revive_button.disabled = not (no_pending and operation.get("phase") == "runner_downed" and role == "anchor" and operation.get("revive", null) == null and not (_result.get("kind") == "revive" and _result.get("accepted", false)))


func _refresh_lifecycle() -> void:
	if not _summary.is_empty():
		_lifecycle.text = _summary_text(_summary)
		_lifecycle.add_theme_color_override("font_color", CYAN if _summary.get("outcome") == "succeeded" else MAGENTA)
		return
	if not _pending.is_empty():
		_lifecycle.add_theme_color_override("font_color", NEUTRAL)
		_lifecycle.text = "%s PENDING  •  AWAITING SERVER\nNo phase, contribution, ping, life, channel, completion, or reward is accepted locally." % str(_pending.get("kind", "request")).to_upper()
		return
	if not _result.is_empty():
		_lifecycle.text = _result_text(_result)
		_lifecycle.add_theme_color_override("font_color", CYAN if _result.get("accepted", false) else MAGENTA)
		return
	_lifecycle.add_theme_color_override("font_color", NEUTRAL)
	_lifecycle.text = "AVAILABLE FOR EXPLICIT REQUEST\nNo cooperation operation, timer, life state, contribution, or reward is inferred locally."


func _result_text(result: Dictionary) -> String:
	if not result.get("accepted", false):
		if result.get("connection_lost", false):
			return "CONNECTION LOST  •  OPERATION NOT RESUMED\n%s" % str(result.get("message", "Reconnect from the entry screen."))
		if result.get("kind") == "protocol":
			return str(result.get("message", "INVALID SERVER DATA"))
		return "REJECTED  •  NO AUTHORITATIVE CHANGE\n%s" % str(result.get("message", "Server rejected cooperation request."))
	if result.get("kind") == "state":
		return "SERVER STATE RECEIVED\n%s" % str(result.get("message", "Capability state is authoritative."))
	var replay := " REPLAY" if result.get("replayed", false) else ""
	if result.get("kind") == "revive":
		var revive: Dictionary = result.get("revive", {})
		return "REVIVE %s%s  •  OBSERVED %d MS\nSOURCE %d  →  TARGET %d  •  CHANNEL %d MS  •  DISTANCE² %d/%d" % [
			str(result.get("status", "unknown")).to_upper(), replay,
			int(revive.get("observed_elapsed_ms", 0)), int(revive.get("source_actor_id", 0)),
			int(revive.get("target_actor_id", 0)), int(revive.get("required_duration_ms", 0)),
			int(revive.get("distance_squared", 0)), int(revive.get("maximum_distance_squared", 0)),
		]
	if result.get("kind") == "ping":
		var ping: Dictionary = result.get("ping", {})
		return "PING ACCEPTED%s  •  %s  •  TARGET RELAY CONSOLE\nSOURCE %d  •  ACCEPTED %d MS  •  EXPIRES %d MS" % [
			replay, "LIVE" if ping.get("active", false) else "EXPIRED", int(ping.get("source_actor_id", 0)),
			int(ping.get("accepted_elapsed_ms", 0)), int(ping.get("expires_elapsed_ms", 0)),
		]
	return "%s ACCEPTED%s  •  ID %s\n%s" % [
		str(result.get("kind", "request")).to_upper(), replay,
		str(result.get("operation_id", "")), str(result.get("message", "Server accepted request.")),
	]


func _summary_text(summary: Dictionary) -> String:
	var outcome := str(summary.get("outcome", "unknown"))
	if outcome == "succeeded":
		var reward: Dictionary = summary.get("reward", {})
		return "SUCCEEDED  •  %d MS  •  ALL 5 CONTRIBUTIONS  •  REVIVE 1/1\n2 ORDERED GRANTS  •  EACH %d RELAY CORE FRAGMENT(S) / %d XP" % [
			int(summary.get("terminal_elapsed_ms", 0)), int(reward.get("item_quantity", 0)), int(reward.get("experience", 0)),
		]
	var subject := "" if summary.get("subject_role", null) == null else "  •  %s" % str(summary.get("subject_role")).to_upper()
	return "%s%s  •  %d MS\nNO REWARD  •  %s" % [
		outcome.replace("_", " ").to_upper(), subject, int(summary.get("terminal_elapsed_ms", 0)),
		str(summary.get("no_reward_reason", "server failure")).replace("_", " ").to_upper(),
	]


func _local_role() -> String:
	var operation = _state.get("operation", null)
	if not operation is Dictionary:
		return ""
	for participant in operation.get("participants", []):
		if int(participant.get("actor_id", 0)) == _player_actor_id:
			return str(participant.get("role", ""))
	return ""


func _ping_text(operation: Dictionary) -> String:
	var ping = operation.get("ping", null)
	if not ping is Dictionary and _result.get("kind") == "ping" and _result.get("accepted", false):
		ping = _result.get("ping", null)
	if not ping is Dictionary:
		return "PING  •  NONE ACCEPTED"
	return "PING  •  %s  •  SOURCE %d  •  ACCEPTED %d MS  •  EXPIRES %d MS" % [
		"LIVE" if ping.get("active", false) else "EXPIRED", int(ping.get("source_actor_id", 0)),
		int(ping.get("accepted_elapsed_ms", 0)), int(ping.get("expires_elapsed_ms", 0)),
	]


func _life_detail(life: Dictionary) -> String:
	if life.is_empty():
		return "LATEST LIFE EVENT  •  NONE"
	var source = life.get("source_actor_id", null)
	return "LATEST  •  %s → %s  •  %s  •  SOURCE %s  •  %d MS" % [
		str(life.get("life_before", "")).to_upper(), str(life.get("life_after", "")).to_upper(),
		str(life.get("cause", "")).replace("_", " ").to_upper(), "SERVER" if source == null else str(source),
		int(life.get("elapsed_ms", 0)),
	]


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
	button.add_theme_stylebox_override("normal", _button_style(BLUE_PETROL, accent, 0.74))
	button.add_theme_stylebox_override("hover", _button_style(BLUE_PETROL.lightened(0.12), accent, 1.0))
	button.add_theme_stylebox_override("pressed", _button_style(accent, accent, 1.0))
	button.add_theme_stylebox_override("focus", _button_style(BLUE_PETROL.lightened(0.08), accent, 1.0))
	parent.add_child(button)
	return button


func _panel_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(GRAPHITE, 0.995)
	style.border_color = CYAN
	style.set_border_width_all(2)
	style.set_corner_radius_all(5)
	return style


func _card_style(accent: Color) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(BLUE_PETROL, 0.72)
	style.border_color = accent
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
