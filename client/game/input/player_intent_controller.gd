extends RefCounted

const MOVE_COOLDOWN_MS := 120
const DEFAULT_ATTACK_COOLDOWN_MS := 260
const MAX_ATTACK_COOLDOWN_MS := 5000

var _ui_movement := Vector2.ZERO
var _attack_requested := false
var _ui_attack_requested := false
var _next_move_at := 0
var _next_attack_at := 0
var _attack_cooldown_ms := DEFAULT_ATTACK_COOLDOWN_MS
var _suppress_attack_through_frame := -1


func set_ui_movement(direction: Vector2) -> void:
	_ui_movement = direction


func clear_ui_movement() -> void:
	_ui_movement = Vector2.ZERO


func clear_pending_input() -> void:
	clear_ui_movement()
	_attack_requested = false
	_ui_attack_requested = false
	# A click or Space used to close a modal must not become an attack.
	_suppress_attack_through_frame = Engine.get_process_frames() + 1


func request_attack(use_active_target: bool) -> void:
	_attack_requested = true
	_ui_attack_requested = use_active_target


func set_attack_cooldown_ms(cooldown_ms: int) -> bool:
	if cooldown_ms < 1 or cooldown_ms > MAX_ATTACK_COOLDOWN_MS:
		return false
	_attack_cooldown_ms = cooldown_ms
	return true


func movement(now: int) -> Vector2:
	if now < _next_move_at:
		return Vector2.ZERO
	return _ui_movement if _ui_movement != Vector2.ZERO else Input.get_vector("move_left", "move_right", "move_forward", "move_back")


func consume_movement(now: int) -> void:
	_next_move_at = now + MOVE_COOLDOWN_MS


func take_attack(now: int) -> Dictionary:
	if Engine.get_process_frames() <= _suppress_attack_through_frame:
		return {"requested": false, "cooling": false}
	if not _attack_requested:
		return {"requested": false, "cooling": false}
	if now < _next_attack_at:
		return {"requested": false, "cooling": _attack_requested, "remaining_ms": _next_attack_at - now}
	var use_active_target := _ui_attack_requested
	_attack_requested = false
	_ui_attack_requested = false
	return {"requested": true, "cooling": false, "use_active_target": use_active_target}


func consume_attack(now: int) -> void:
	_next_attack_at = now + _attack_cooldown_ms


func presentation_state() -> Dictionary:
	return {
		"move_cooldown_ms": MOVE_COOLDOWN_MS,
		"attack_cooldown_ms": _attack_cooldown_ms,
		"ui_movement": _ui_movement,
		"attack_requested": _attack_requested,
	}
