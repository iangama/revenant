extends "res://presentation/enemies/enemy_presentation.gd"

var _containment_material: StandardMaterial3D
var _containment_halo: MeshInstance3D
var _containment_crown: MeshInstance3D
var _containment_plates: Array[MeshInstance3D] = []


func family_name() -> String:
	return "warden"


func _process(delta: float) -> void:
	super(delta)
	if _containment_halo == null:
		return
	_containment_halo.rotation_degrees.y = fmod(_idle_clock * 42.0, 360.0)
	_containment_crown.rotation_degrees.y = fmod(-_idle_clock * 31.0, 360.0)
	_containment_material.emission_energy_multiplier = 1.55 + sin(_idle_clock * 2.4) * 0.30


func play_confirmed_attack() -> void:
	super()
	_containment_material.emission_energy_multiplier = 4.6
	var halo_pulse := create_tween()
	halo_pulse.tween_property(_containment_halo, "scale", Vector3.ONE * 1.45, 0.10)
	halo_pulse.tween_property(_containment_halo, "scale", Vector3.ONE, 0.22)
	for plate in _containment_plates:
		var plate_pulse := create_tween()
		plate_pulse.tween_property(plate, "scale", Vector3(1.15, 1.55, 1.15), 0.08)
		plate_pulse.tween_property(plate, "scale", Vector3.ONE, 0.22)


func play_confirmed_hit() -> void:
	super()
	var ring_recoil := create_tween()
	ring_recoil.tween_property(_containment_halo, "rotation_degrees:z", 112.0, 0.07)
	ring_recoil.tween_property(_containment_halo, "rotation_degrees:z", 90.0, 0.14)


func _build_body() -> void:
	_visual_root.scale = Vector3.ONE * 1.12
	var armor := _material(Color("3a4858"), 0.76, 0.34)
	var corruption := _emissive_material(Color("d93678"), 2.55)
	_containment_material = _emissive_material(Color("f5a524"), 1.55)
	_set_core_material(corruption)
	_add_box("ContainmentBody", Vector3(1.25, 1.42, 0.86), Vector3(0.0, 1.08, 0.0), armor)
	_add_sphere("WardenCore", 0.36, Vector3(0.0, 1.16, -0.50), corruption)
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
		_containment_plates.append(_add_box(plate_data[0], Vector3(0.72, 0.22, 0.48), plate_data[1], corruption, plate_data[2]))
	_add_box("BaseLeft", Vector3(0.58, 0.28, 1.05), Vector3(-0.42, 0.22, 0.0), armor, Vector3(0.0, 0.0, -5.0))
	_add_box("BaseRight", Vector3(0.58, 0.28, 1.05), Vector3(0.42, 0.22, 0.0), armor, Vector3(0.0, 0.0, 5.0))
	_add_box("ShoulderGuardLeft", Vector3(0.48, 0.34, 0.82), Vector3(-0.82, 1.42, 0.0), armor, Vector3(0.0, 0.0, -12.0))
	_add_box("ShoulderGuardRight", Vector3(0.48, 0.34, 0.82), Vector3(0.82, 1.42, 0.0), armor, Vector3(0.0, 0.0, 12.0))
	_containment_crown = _add_cylinder("ContainmentCrown", 0.44, 0.16, Vector3(0.0, 1.94, 0.0), _containment_material)
	_containment_halo = _add_torus("ContainmentHalo", 0.92, 0.055, Vector3(0.0, 1.18, 0.0), _containment_material, Vector3(0.0, 0.0, 90.0))
