extends RefCounted

const MOVE_COOLDOWN_MS := 120
const ATTACK_COOLDOWN_MS := 260

var _ui_movement := Vector2.ZERO
var _attack_requested := false
var _ui_attack_requested := false
var _next_move_at := 0
var _next_attack_at := 0


func set_ui_movement(direction: Vector2) -> void:
	_ui_movement = direction


func clear_ui_movement() -> void:
	_ui_movement = Vector2.ZERO


func request_attack(use_active_target: bool) -> void:
	_attack_requested = true
	_ui_attack_requested = use_active_target


func movement(now: int) -> Vector2:
	if now < _next_move_at:
		return Vector2.ZERO
	return _ui_movement if _ui_movement != Vector2.ZERO else Input.get_vector("move_left", "move_right", "move_forward", "move_back")


func consume_movement(now: int) -> void:
	_next_move_at = now + MOVE_COOLDOWN_MS


func take_attack(now: int) -> Dictionary:
	var keyboard_requested := Input.is_action_just_pressed("attack")
	if not _attack_requested and not keyboard_requested:
		return {"requested": false, "cooling": false}
	if now < _next_attack_at:
		return {"requested": false, "cooling": _attack_requested, "remaining_ms": _next_attack_at - now}
	var use_active_target := _ui_attack_requested
	_attack_requested = false
	_ui_attack_requested = false
	return {"requested": true, "cooling": false, "use_active_target": use_active_target}


func consume_attack(now: int) -> void:
	_next_attack_at = now + ATTACK_COOLDOWN_MS


func presentation_state() -> Dictionary:
	return {
		"move_cooldown_ms": MOVE_COOLDOWN_MS,
		"attack_cooldown_ms": ATTACK_COOLDOWN_MS,
		"ui_movement": _ui_movement,
		"attack_requested": _attack_requested,
	}
