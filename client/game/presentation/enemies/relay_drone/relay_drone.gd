extends "res://presentation/enemies/enemy_presentation.gd"


func family_name() -> String:
	return "relay_drone"


func _build_body() -> void:
	var armor := _material(Color("172632"), 0.7, 0.38)
	var corruption := _emissive_material(Color("d93678"), 1.8)
	_set_core_material(corruption)
	_add_cylinder("RadialChassis", 0.48, 0.28, Vector3(0.0, 0.72, 0.0), armor)
	_add_sphere("CorruptedCore", 0.24, Vector3(0.0, 0.73, -0.2), corruption)
	for arm_data in [
		["ArmNorth", Vector3(0.0, 0.65, -0.68), Vector3(-22.0, 0.0, 0.0)],
		["ArmSouth", Vector3(0.0, 0.65, 0.68), Vector3(22.0, 0.0, 0.0)],
		["ArmWest", Vector3(-0.68, 0.65, 0.0), Vector3(0.0, 0.0, -22.0)],
		["ArmEast", Vector3(0.68, 0.65, 0.0), Vector3(0.0, 0.0, 22.0)],
	]:
		_add_box(arm_data[0], Vector3(0.7, 0.16, 0.18), arm_data[1], armor, arm_data[2])
