extends Node3D

@export var weapon_id := "pulse_rifle"
@export var body_length := 0.9
@export var body_height := 0.18
@export var grip_offset := 0.18

var _energy_material: StandardMaterial3D


func _ready() -> void:
	name = weapon_id
	_build_weapon()


func pulse() -> void:
	if _energy_material == null:
		return
	_energy_material.emission_energy_multiplier = 4.5
	var tween := create_tween()
	tween.tween_property(_energy_material, "emission_energy_multiplier", 1.8, 0.14)


func _build_weapon() -> void:
	var frame_material := _material(Color("172632"), 0.7, 0.3)
	_energy_material = _emissive_material(Color("35d0d0"), 1.8)
	_add_box("Frame", Vector3(body_length, body_height, 0.16), Vector3(body_length * 0.38, 0.0, 0.0), frame_material)
	_add_box("Barrel", Vector3(body_length * 0.38, body_height * 0.48, 0.1), Vector3(body_length * 0.92, 0.02, 0.0), frame_material)
	_add_box("EnergyCell", Vector3(body_length * 0.28, body_height * 0.55, 0.18), Vector3(body_length * 0.35, 0.0, 0.0), _energy_material)
	_add_box("Grip", Vector3(0.13, 0.34, 0.14), Vector3(grip_offset, -0.2, 0.0), frame_material, Vector3(0.0, 0.0, -12.0))
	if weapon_id == "pulse_rifle":
		_add_box("Stock", Vector3(0.34, 0.27, 0.2), Vector3(-0.22, -0.02, 0.0), frame_material)
	else:
		_add_box("ArcGuard", Vector3(0.28, 0.08, 0.2), Vector3(0.02, -0.34, 0.0), _energy_material, Vector3(0.0, 0.0, 18.0))


func _add_box(part_name: String, size: Vector3, position: Vector3, material: Material, rotation: Vector3 = Vector3.ZERO) -> void:
	var part := MeshInstance3D.new()
	part.name = part_name
	var mesh := BoxMesh.new()
	mesh.size = size
	part.mesh = mesh
	part.position = position
	part.rotation_degrees = rotation
	part.material_override = material
	add_child(part)


func _material(color: Color, metallic: float, roughness: float) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.metallic = metallic
	material.roughness = roughness
	return material


func _emissive_material(color: Color, energy: float) -> StandardMaterial3D:
	var material := _material(color.darkened(0.45), 0.25, 0.2)
	material.emission_enabled = true
	material.emission = color
	material.emission_energy_multiplier = energy
	return material
