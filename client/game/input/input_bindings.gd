extends RefCounted

# Menu navigation remains available even after gameplay bindings are changed.
const ACTIONS := {
	"move_forward": "Move north", "move_back": "Move south",
	"move_left": "Move west", "move_right": "Move east",
	"attack": "Attack active enemy", "target_next": "Next enemy", "interact": "Read nearby archive",
	"archive": "Archive", "modules": "Modules", "routes": "Routes",
	"cooperation": "Cooperation", "help": "Guidance",
	"weapon_1": "Pulse Rifle", "weapon_2": "Arc Sidearm", "weapon_3": "Coil Lance",
	"weapon_previous": "Previous weapon", "weapon_next": "Next weapon",
	"menu_focus": "Focus action bar", "settings": "Settings",
}
const KEYS := {
	"move_forward": KEY_W, "move_back": KEY_S, "move_left": KEY_A, "move_right": KEY_D,
	"attack": KEY_SPACE, "target_next": KEY_V, "interact": KEY_E, "archive": KEY_J, "modules": KEY_M,
	"routes": KEY_R, "cooperation": KEY_C, "help": KEY_H,
	"weapon_1": KEY_1, "weapon_2": KEY_2, "weapon_3": KEY_3,
	"weapon_previous": KEY_Q, "weapon_next": KEY_F, "menu_focus": KEY_T, "settings": KEY_P,
}
const BUTTONS := {
	"move_forward": JOY_BUTTON_DPAD_UP, "move_back": JOY_BUTTON_DPAD_DOWN,
	"move_left": JOY_BUTTON_DPAD_LEFT, "move_right": JOY_BUTTON_DPAD_RIGHT,
	"attack": JOY_BUTTON_A, "target_next": JOY_BUTTON_RIGHT_STICK, "interact": JOY_BUTTON_B, "archive": JOY_BUTTON_X,
	"modules": JOY_BUTTON_Y, "weapon_previous": JOY_BUTTON_LEFT_SHOULDER,
	"weapon_next": JOY_BUTTON_RIGHT_SHOULDER, "settings": JOY_BUTTON_START,
	"menu_focus": JOY_BUTTON_BACK,
}
const BUTTON_NAMES := ["South / A", "East / B", "West / X", "North / Y", "Back", "Guide", "Start", "Left stick", "Right stick", "LB", "RB", "D-pad up", "D-pad down", "D-pad left", "D-pad right"]
const RESERVED_KEYS := [KEY_ESCAPE, KEY_TAB, KEY_ENTER, KEY_KP_ENTER]


static func defaults() -> Dictionary:
	var result := {}
	for action in ACTIONS:
		result[action] = {"key": KEYS[action], "button": BUTTONS.get(action, -1)}
	return result


static func sanitize(candidate: Variant) -> Dictionary:
	var result := defaults()
	if not candidate is Dictionary:
		return result
	# M34 adds target cycling. Preserve older custom bindings when V or the
	# right-stick button already belongs to another action.
	if not candidate.has("target_next"):
		candidate = candidate.duplicate(true)
		var added: Dictionary = result.target_next.duplicate()
		for channel in ["key", "button"]:
			var occupied: Array = []
			for entry in candidate.values():
				if entry is Dictionary:
					occupied.append(entry.get(channel, -1))
			if added[channel] in occupied:
				if channel == "button":
					added[channel] = -1
				else:
					for key in range(KEY_A, KEY_Z + 1):
						if key not in occupied and key not in KEYS.values():
							added[channel] = key
							break
		candidate.target_next = added
	# Apply each channel as a whole. A corrupt duplicate must not strand an
	# action or silently discard another valid binding.
	for channel in ["key", "button"]:
		var values := {}
		var seen := {}
		var valid := true
		for action in ACTIONS:
			var entry: Variant = candidate.get(action, result[action])
			var value: Variant = entry.get(channel, result[action][channel]) if entry is Dictionary else null
			if not value is int or not valid_code(channel, value) or (value != -1 and seen.has(value)):
				valid = false
				break
			values[action] = value
			seen[value] = true
		if valid:
			for action in ACTIONS:
				result[action][channel] = values[action]
	return result


static func valid_code(channel: String, code: int) -> bool:
	if channel == "button":
		return code == -1 or (code >= 0 and code < BUTTON_NAMES.size() and code != JOY_BUTTON_GUIDE)
	return code > 0 and code <= KEY_SPECIAL + 255 and code not in RESERVED_KEYS and not OS.get_keycode_string(code).is_empty()


static func rebind(bindings: Dictionary, action: String, channel: String, code: int) -> bool:
	if not ACTIONS.has(action) or channel not in ["key", "button"] or not valid_code(channel, code):
		return false
	var previous: int = bindings[action][channel]
	for other in ACTIONS:
		if other != action and code != -1 and bindings[other][channel] == code:
			bindings[other][channel] = previous
	bindings[action][channel] = code
	return true


static func apply(candidate: Dictionary) -> void:
	var bindings := sanitize(candidate)
	for action in {"ui_accept": JOY_BUTTON_A, "ui_cancel": JOY_BUTTON_B, "ui_up": JOY_BUTTON_DPAD_UP, "ui_down": JOY_BUTTON_DPAD_DOWN, "ui_left": JOY_BUTTON_DPAD_LEFT, "ui_right": JOY_BUTTON_DPAD_RIGHT}:
		var button := InputEventJoypadButton.new()
		button.button_index = {"ui_accept": JOY_BUTTON_A, "ui_cancel": JOY_BUTTON_B, "ui_up": JOY_BUTTON_DPAD_UP, "ui_down": JOY_BUTTON_DPAD_DOWN, "ui_left": JOY_BUTTON_DPAD_LEFT, "ui_right": JOY_BUTTON_DPAD_RIGHT}[action]
		if not InputMap.action_has_event(action, button):
			InputMap.action_add_event(action, button)
	for action in ACTIONS:
		if not InputMap.has_action(action):
			InputMap.add_action(action, 0.25)
		InputMap.action_erase_events(action)
		var key := InputEventKey.new()
		key.physical_keycode = bindings[action].key
		InputMap.action_add_event(action, key)
		if bindings[action].button >= 0:
			var button := InputEventJoypadButton.new()
			button.button_index = bindings[action].button
			InputMap.action_add_event(action, button)
	# Arrow-key alternatives only occupy keys not explicitly rebound elsewhere.
	for action in ["move_forward", "move_back", "move_left", "move_right"]:
		var arrow: int = {"move_forward": KEY_UP, "move_back": KEY_DOWN, "move_left": KEY_LEFT, "move_right": KEY_RIGHT}[action]
		var occupied := false
		for entry in bindings.values():
			occupied = occupied or entry.key == arrow
		if not occupied:
			var key := InputEventKey.new()
			key.physical_keycode = arrow
			InputMap.action_add_event(action, key)
		var axis := InputEventJoypadMotion.new()
		axis.axis = JOY_AXIS_LEFT_Y if action in ["move_forward", "move_back"] else JOY_AXIS_LEFT_X
		axis.axis_value = -1.0 if action in ["move_forward", "move_left"] else 1.0
		InputMap.action_add_event(action, axis)
	var mouse := InputEventMouseButton.new()
	mouse.button_index = MOUSE_BUTTON_LEFT
	InputMap.action_add_event("attack", mouse)


static func key_label(bindings: Dictionary, action: String) -> String:
	return OS.get_keycode_string(bindings[action].key)


static func button_label(bindings: Dictionary, action: String) -> String:
	var code: int = bindings[action].button
	return "Unassigned" if code < 0 else BUTTON_NAMES[code]
