extends "res://presentation/enemies/enemy_presentation.gd"

var _charge_line: MeshInstance3D
var _charge_label: Label3D
var _warning := false


func family_name() -> String:
	return "glass_lancer"


func _build_body() -> void:
	var armor := _material(Color("abc9ce"), 0.65, 0.3)
	var dark := _material(Color("253847"), 0.7, 0.4)
	var amber := _emissive_material(Color("f6c667"), 1.3)
	_set_core_material(amber)
	_add_box("Spine", Vector3(0.5, 1.5, 0.65), Vector3(0, 1.0, 0), dark)
	for side in [-1, 1]:
		_add_box("Blade%d" % side, Vector3(0.3, 1.25, 0.7), Vector3(side * 0.52, 1.15, 0), armor, Vector3(0, 0, side * -22))
		_add_box("Runner%d" % side, Vector3(0.25, 0.3, 1.2), Vector3(side * 0.4, 0.2, 0), dark)
	_add_box("Lance", Vector3(0.16, 0.16, 1.75), Vector3(0, 1.3, 1), armor)
	_add_sphere("Lens", 0.22, Vector3(0, 1.5, 0.36), amber)
	_charge_line = MeshInstance3D.new()
	_charge_line.name = "ConfirmedChargeLane"
	_charge_line.mesh = BoxMesh.new()
	_charge_line.material_override = amber
	_charge_line.visible = false
	add_child(_charge_line)
	_charge_label = Label3D.new()
	_charge_label.position = Vector3(0, 2.5, 0)
	_charge_label.font_size = 36
	_charge_label.pixel_size = 0.01
	_charge_label.outline_size = 10
	_charge_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	_charge_label.modulate = Color.WHITE
	_charge_label.text = "GLASS LANCER"
	add_child(_charge_label)


func present_charge(cue: Dictionary) -> void:
	_warning = cue.get("winding_up", false)
	_charge_line.visible = _warning
	_charge_label.text = "CHARGE • STEP ASIDE" if _warning else "RECOVERING • ATTACK"
	if not _warning:
		return
	var target: Array = cue.get("target", [0, 0, 0])
	var displacement := Vector3(target[0], 0, target[2]) - global_position
	_charge_line.position = displacement * 0.5 + Vector3(0, 0.1, 0)
	_charge_line.mesh.size = Vector3(absf(displacement.x) + 0.6, 0.05, absf(displacement.z) + 0.6)
	_visual_root.rotation.y = atan2(displacement.x, displacement.z)


func presentation_state() -> Dictionary:
	var state := super.presentation_state()
	state["charge_warning"] = _warning
	state["charge_label"] = _charge_label.text
	return state


func retire() -> void:
	_charge_line.visible = false
	_charge_label.visible = false
	super.retire()
