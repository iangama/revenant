extends "res://presentation/enemies/enemy_presentation.gd"


func family_name() -> String:
	return "warden"


func _build_body() -> void:
	var armor := _material(Color("10151d"), 0.76, 0.34)
	var corruption := _emissive_material(Color("d93678"), 2.15)
	var containment := _emissive_material(Color("f5a524"), 1.15)
	_set_core_material(corruption)
	_add_box("ContainmentBody", Vector3(1.1, 1.3, 0.75), Vector3(0.0, 1.08, 0.0), armor)
	_add_sphere("WardenCore", 0.31, Vector3(0.0, 1.14, -0.43), corruption)
	for pylon_data in [
		["PylonFrontLeft", Vector3(-0.72, 0.62, -0.48), Vector3(0.0, 8.0, -8.0)],
		["PylonFrontRight", Vector3(0.72, 0.62, -0.48), Vector3(0.0, -8.0, 8.0)],
		["PylonBackLeft", Vector3(-0.72, 0.62, 0.48), Vector3(0.0, -8.0, -8.0)],
		["PylonBackRight", Vector3(0.72, 0.62, 0.48), Vector3(0.0, 8.0, 8.0)],
	]:
		_add_box(pylon_data[0], Vector3(0.32, 1.15, 0.32), pylon_data[1], armor, pylon_data[2])
	for plate_data in [
		["PlateNorth", Vector3(0.0, 1.7, -0.68), Vector3(-12.0, 0.0, 0.0)],
		["PlateSouth", Vector3(0.0, 1.7, 0.68), Vector3(12.0, 0.0, 0.0)],
		["PlateWest", Vector3(-0.68, 1.7, 0.0), Vector3(0.0, 0.0, -12.0)],
		["PlateEast", Vector3(0.68, 1.7, 0.0), Vector3(0.0, 0.0, 12.0)],
	]:
		_add_box(plate_data[0], Vector3(0.72, 0.22, 0.48), plate_data[1], containment, plate_data[2])
