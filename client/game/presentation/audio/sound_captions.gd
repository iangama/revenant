extends PanelContainer

const CAPTIONS := {
	"scatter_caster": "Scatter Caster fires", "rail_driver": "Rail Driver fires",
	"prism-warden": "Prism Warden signal", "prism_lane": "Prism firing lane locked",
	"prism_center": "Center pulse charging", "prism_perimeter": "Outer pulse charging",
	"prism_shift": "Prism enters phase two", "prism_recovery": "Prism shell opens",

	"meridian": "Wind through the Meridian glass",
	"ambience": "Relay hum", "system_ready": "Relay connected", "door_unlock": "Core door opens",
	"operator_move": "Operator footsteps", "pulse_rifle": "Pulse Rifle fires", "arc_sidearm": "Arc Sidearm fires", "coil_lance": "Coil Lance fires",
	"impact": "Hit impact", "defeat": "Enemy falls", "player_damage": "Operator hit",
	"cooldown": "Weapon cooling", "completion": "Mission complete",
	"attack_attempt": "Attack requested", "target_unavailable": "Target unavailable",
	"relay-drone": "Relay drone signal", "warden": "Warden signal", "signal-sentinel": "Signal sentinel signal",
	"glass-lancer": "Glass Lancer signal", "lancer_charge": "Lancer charging — step off the line",
	"relay-mender": "Relay Mender signal", "mender_repair": "Mender repairs its ally",
	"reserve_recovery": "Reserve restores health",
	"gauntlet_recovery": "Transfer restores health",
	"steel-bulwark": "Steel Bulwark signal", "bulwark_slam": "Bulwark preparing slam — step aside", "shield_block": "Shield blocks the shot",
}
var _lines: Array[Dictionary] = []
var _last_at := {}
var _label: Label
var _enabled := true
var listener_position := Vector3.ZERO


func _ready() -> void:
	name = "SoundCaptions"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	position = Vector2(280, 460)
	size = Vector2(600, 1)
	custom_minimum_size.x = 600
	var style := StyleBoxFlat.new()
	style.bg_color = Color("071016", 0.97)
	style.content_margin_left = 12
	style.content_margin_right = 12
	style.content_margin_top = 8
	style.content_margin_bottom = 8
	add_theme_stylebox_override("panel", style)
	_label = Label.new()
	_label.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	_label.add_theme_color_override("font_color", Color.WHITE)
	_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_label.vertical_alignment = VERTICAL_ALIGNMENT_TOP
	_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_label)
	visible = false


func configure(enabled: bool, ui_scale: float) -> void:
	_enabled = enabled
	_label.add_theme_font_size_override("font_size", roundi(16 * ui_scale))
	if not _enabled:
		_lines.clear()
	_refresh()


func request(cue: String, source_position: Vector3) -> void:
	if not _enabled or not CAPTIONS.has(cue):
		return
	var now := Time.get_ticks_msec()
	if now - int(_last_at.get(cue, -1500)) < 1000:
		return
	_last_at[cue] = now
	var direction := ""
	if source_position.is_finite() and source_position.distance_to(listener_position) > 2.0:
		var offset := source_position - listener_position
		direction = ("N" if offset.z < -1 else ("S" if offset.z > 1 else "")) + ("E" if offset.x > 1 else ("W" if offset.x < -1 else ""))
	# Low-priority repeated effects cannot evict a recent damage/completion cue.
	var critical := cue in ["player_damage", "completion"] or cue.begins_with("prism_")
	if _lines.size() >= 3:
		var removable := -1
		for index in _lines.size():
			if not _lines[index].critical:
				removable = index
				break
		if removable < 0 and not critical:
			return
		_lines.remove_at(maxi(removable, 0))
	_lines.append({"cue": cue, "direction": direction, "until": now + (3500 if critical and not cue.begins_with("prism_") else 2200), "critical": critical})
	_refresh()


func _process(_delta: float) -> void:
	var now := Time.get_ticks_msec()
	var count := _lines.size()
	_lines = _lines.filter(func(line: Dictionary) -> bool: return line.until > now)
	if _lines.size() != count:
		_refresh()


func _refresh() -> void:
	visible = _enabled and not _lines.is_empty()
	var texts: Array[String] = []
	for line in _lines:
		texts.append("♪ %s%s" % [tr(CAPTIONS[line.cue]), " • " + str(line.direction) if not line.direction.is_empty() else ""])
	_label.text = "\n".join(texts)
	call_deferred("reset_size")


func presentation_state() -> Dictionary:
	return {"enabled": _enabled, "line_count": _lines.size(), "maximum_lines": 3, "text": _label.text}
