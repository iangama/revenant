extends "res://presentation/enemies/enemy_presentation.gd"

func family_name() -> String:
	return "signal_sentinel"


func _build_body() -> void:
	var armor := _material(Color("3d5268"), 0.7, 0.38)
	var dark := _material(Color("182b38"), 0.6, 0.4)
	var signal_light := _emissive_material(Color("ff7958"), 1.6)
	_set_core_material(signal_light)
	_add_cylinder("HeavyBase", 0.8, 0.35, Vector3(0, 0.18, 0), dark)
	_add_box("ArmoredColumn", Vector3(0.75, 1.5, 0.8), Vector3(0, 1, 0), armor)
	_add_box("TurretHead", Vector3(1.5, 0.5, 0.9), Vector3(0, 1.8, 0), armor)
	for side in [-1, 1]:
		_add_box("LongBarrel%d" % side, Vector3(0.24, 0.25, 1.8), Vector3(side * 0.5, 1.8, 0.9), dark)
		_add_box("Muzzle%d" % side, Vector3(0.28, 0.28, 0.14), Vector3(side * 0.5, 1.8, 1.82), signal_light)
	_add_sphere("TargetLens", 0.22, Vector3(0, 1.86, 0.5), signal_light)
	_add_box("Antenna", Vector3(0.07, 0.9, 0.07), Vector3(0, 2.47, 0), armor)


func track_target(world_position: Vector3) -> void:
	var direction := world_position - global_position
	_visual_root.rotation.y = atan2(direction.x, direction.z)
