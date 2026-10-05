extends "res://presentation/enemies/enemy_presentation.gd"

var _link: MeshInstance3D
var _label: Label3D
var _repair_target: WeakRef
var _repair_until := 0


func family_name() -> String:
	return "relay_mender"


func _build_body() -> void:
	var armor := preload("res://presentation/environment/materials/graphite.tres")
	var core := _emissive_material(Color("61e8b8"), 1.4)
	_set_core_material(core)
	_add_box("RelayBody", Vector3(0.85, 0.65, 0.7), Vector3(0, 1.2, 0), armor)
	for side in [-1, 1]:
		_add_box("Arm%d" % side, Vector3(0.2, 0.25, 1.1), Vector3(side * 0.65, 1.2, 0.2), armor)
		_add_sphere("Emitter%d" % side, 0.15, Vector3(side * 0.65, 1.2, 0.75), core)
	_add_box("RepairVertical", Vector3(0.13, 0.6, 0.12), Vector3(0, 1.4, 0.42), core)
	_add_box("RepairHorizontal", Vector3(0.55, 0.13, 0.12), Vector3(0, 1.4, 0.42), core)
	_link = MeshInstance3D.new()
	_link.mesh = BoxMesh.new()
	_link.material_override = core
	_link.visible = false
	add_child(_link)
	_label = Label3D.new()
	_label.text = "RELAY MENDER"
	_label.position = Vector3(0, 2.6, 0)
	_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	_label.font_size = 32
	_label.pixel_size = 0.01
	_label.outline_size = 10
	add_child(_label)


func present_repair(target: Node3D, amount: int) -> void:
	_repair_target = weakref(target)
	_repair_until = Time.get_ticks_msec() + 700
	_label.text = tr("REPAIR +%d") % amount


func _process(delta: float) -> void:
	super._process(delta)
	var target: Node3D = _repair_target.get_ref() if _repair_target != null else null
	_link.visible = not _retired and target != null and Time.get_ticks_msec() < _repair_until
	if not _link.visible:
		_label.text = "RELAY MENDER"
		return
	var displacement := target.global_position - global_position
	_link.position = displacement * 0.5 + Vector3(0, 2.0, 0)
	_link.mesh.size = Vector3(0.12, 0.12, maxf(0.1, displacement.length()))
	_link.rotation.y = atan2(displacement.x, displacement.z)


func presentation_state() -> Dictionary:
	var state := super.presentation_state()
	state["repair_visible"] = _link.visible
	return state


func retire() -> void:
	_link.visible = false
	_label.visible = false
	super.retire()
