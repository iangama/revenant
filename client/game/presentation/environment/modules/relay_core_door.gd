extends Node3D

var _left_panel: MeshInstance3D
var _right_panel: MeshInstance3D
var _amber_strips: Node3D
var _open := false
var _transition: Tween


func _ready() -> void:
	_left_panel = $LeftPanel
	_right_panel = $RightPanel
	_amber_strips = $AmberStrips
	_apply_state(false, false)


func set_open(open: bool, animated := true) -> void:
	if _open == open and animated:
		return
	_open = open
	_apply_state(open, animated)


func presentation_state() -> Dictionary:
	return {
		"open": _open,
		"left_panel_z": _left_panel.position.z,
		"right_panel_z": _right_panel.position.z,
		"amber_visible": _amber_strips.visible,
	}


func _apply_state(open: bool, animated: bool) -> void:
	if _transition != null and _transition.is_valid():
		_transition.kill()
	var left_target := -2.65 if open else -1.12
	var right_target := 2.65 if open else 1.12
	_amber_strips.visible = not open
	if not animated:
		_left_panel.position.z = left_target
		_right_panel.position.z = right_target
		return
	_transition = create_tween().set_parallel(true)
	_transition.tween_property(_left_panel, "position:z", left_target, 0.28)
	_transition.tween_property(_right_panel, "position:z", right_target, 0.28)
