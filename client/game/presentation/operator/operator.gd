extends Node3D

const PULSE_RIFLE_SCENE := preload("res://presentation/operator/weapons/pulse_rifle.tscn")
const ARC_SIDEARM_SCENE := preload("res://presentation/operator/weapons/arc_sidearm.tscn")

var _visual_root: Node3D
var _motion_root: Node3D
var _weapon_mount: Node3D
var _core_material: StandardMaterial3D
var _pulse_rifle: Node3D
var _arc_sidearm: Node3D
var _equipped_weapon := "pulse_rifle"
var _last_animation := "idle"
var _idle_clock := 0.0
var _defeated := false


func _ready() -> void:
	_build_operator()
	set_weapon(_equipped_weapon)
	_last_animation = "idle"


func _process(delta: float) -> void:
	if _defeated:
		return
	_idle_clock += delta
	_visual_root.position.y = sin(_idle_clock * 2.4) * 0.018
	_core_material.emission_energy_multiplier = 1.9 + sin(_idle_clock * 3.2) * 0.35


func set_weapon(item_id: String) -> void:
	if item_id not in ["pulse_rifle", "arc_sidearm"]:
		return
	_equipped_weapon = item_id
	if _pulse_rifle != null:
		_pulse_rifle.visible = item_id == "pulse_rifle"
	if _arc_sidearm != null:
		_arc_sidearm.visible = item_id == "arc_sidearm"
	_last_animation = "equip"
	_weapon_mount.rotation_degrees = Vector3(0.0, 0.0, -18.0)
	var tween := create_tween()
	tween.tween_property(_weapon_mount, "rotation_degrees", Vector3.ZERO, 0.16)


func play_authoritative_move(local_offset: Vector3) -> void:
	if _defeated:
		return
	_last_animation = "move"
	_motion_root.position = local_offset
	_motion_root.rotation_degrees.z = clampf(-local_offset.x * 3.0, -5.0, 5.0)
	var tween := create_tween().set_parallel(true)
	tween.tween_property(_motion_root, "position", Vector3.ZERO, 0.12)
	tween.tween_property(_motion_root, "rotation_degrees", Vector3.ZERO, 0.12)


func play_confirmed_attack() -> void:
	if _defeated:
		return
	_last_animation = "attack_%s" % ("rifle" if _equipped_weapon == "pulse_rifle" else "sidearm")
	var weapon: Node3D = _pulse_rifle if _equipped_weapon == "pulse_rifle" else _arc_sidearm
	weapon.call("pulse")
	_weapon_mount.position.x = -0.1
	var tween := create_tween()
	tween.tween_property(_weapon_mount, "position", Vector3.ZERO, 0.12)


func play_confirmed_hit() -> void:
	if _defeated:
		return
	_last_animation = "hit"
	_motion_root.scale = Vector3(1.08, 0.92, 1.08)
	var tween := create_tween()
	tween.tween_property(_motion_root, "scale", Vector3.ONE, 0.14)


func play_defeat() -> void:
	if _defeated:
		return
	_defeated = true
	_last_animation = "defeat"
	_core_material.emission_energy_multiplier = 0.15
	var tween := create_tween().set_parallel(true)
	tween.tween_property(_motion_root, "rotation_degrees", Vector3(0.0, 0.0, 78.0), 0.32)
	tween.tween_property(_motion_root, "position", Vector3(0.0, 0.18, 0.0), 0.32)


func presentation_state() -> Dictionary:
	return {
		"equipped_weapon": _equipped_weapon,
		"last_animation": _last_animation,
		"pulse_rifle_visible": _pulse_rifle.visible,
		"arc_sidearm_visible": _arc_sidearm.visible,
		"defeated": _defeated,
		"part_count": _count_mesh_parts(self),
	}


func _build_operator() -> void:
	_motion_root = Node3D.new()
	_motion_root.name = "AuthoritativeMotion"
	add_child(_motion_root)
	_visual_root = Node3D.new()
	_visual_root.name = "OperatorVisual"
	_motion_root.add_child(_visual_root)

	var armor := _material(Color("172632"), 0.72, 0.34)
	var secondary := _material(Color("294552"), 0.58, 0.42)
	var joint := _material(Color("0b1118"), 0.25, 0.62)
	var amber := _emissive_material(Color("f5a524"), 0.9)
	_core_material = _emissive_material(Color("35d0d0"), 2.0)

	_add_box("Torso", Vector3(0.72, 0.62, 0.36), Vector3(0.0, 1.18, 0.0), armor)
	_add_box("Waist", Vector3(0.42, 0.25, 0.3), Vector3(0.0, 0.78, 0.0), joint)
	_add_box("Helmet", Vector3(0.38, 0.4, 0.38), Vector3(0.0, 1.72, 0.0), armor)
	_add_box("Visor", Vector3(0.2, 0.12, 0.04), Vector3(0.0, 1.74, -0.21), _core_material)
	_add_sphere("CoreFront", 0.19, Vector3(0.0, 1.2, -0.22), _core_material)
	_add_sphere("CoreBack", 0.15, Vector3(0.0, 1.2, 0.22), _core_material)
	_add_box("ShoulderLeft", Vector3(0.3, 0.2, 0.42), Vector3(-0.5, 1.42, 0.0), secondary, Vector3(0.0, 0.0, -10.0))
	_add_box("ShoulderRight", Vector3(0.34, 0.23, 0.44), Vector3(0.52, 1.44, 0.0), secondary, Vector3(0.0, 0.0, 10.0))
	_add_limb("ArmLeft", Vector3(-0.49, 1.02, 0.0), Vector3(0.2, 0.56, 0.22), armor)
	_add_limb("ArmRight", Vector3(0.49, 1.02, 0.0), Vector3(0.2, 0.56, 0.22), armor)
	_add_limb("LegLeft", Vector3(-0.23, 0.36, 0.0), Vector3(0.27, 0.68, 0.32), secondary)
	_add_limb("LegRight", Vector3(0.23, 0.36, 0.0), Vector3(0.27, 0.68, 0.32), secondary)
	_add_box("MaintenanceMark", Vector3(0.08, 0.08, 0.03), Vector3(0.3, 1.48, -0.24), amber)

	_weapon_mount = Node3D.new()
	_weapon_mount.name = "WeaponMount"
	_weapon_mount.position = Vector3(0.4, 1.08, -0.32)
	_visual_root.add_child(_weapon_mount)
	_pulse_rifle = PULSE_RIFLE_SCENE.instantiate()
	_arc_sidearm = ARC_SIDEARM_SCENE.instantiate()
	_weapon_mount.add_child(_pulse_rifle)
	_weapon_mount.add_child(_arc_sidearm)


func _add_limb(part_name: String, position: Vector3, size: Vector3, material: Material) -> void:
	_add_box(part_name, size, position, material)


func _add_box(part_name: String, size: Vector3, position: Vector3, material: Material, rotation: Vector3 = Vector3.ZERO) -> void:
	var part := MeshInstance3D.new()
	part.name = part_name
	var mesh := BoxMesh.new()
	mesh.size = size
	part.mesh = mesh
	part.position = position
	part.rotation_degrees = rotation
	part.material_override = material
	_visual_root.add_child(part)


func _add_sphere(part_name: String, radius: float, position: Vector3, material: Material) -> void:
	var part := MeshInstance3D.new()
	part.name = part_name
	var mesh := SphereMesh.new()
	mesh.radius = radius
	mesh.height = radius * 2.0
	part.mesh = mesh
	part.position = position
	part.material_override = material
	_visual_root.add_child(part)


func _material(color: Color, metallic: float, roughness: float) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.metallic = metallic
	material.roughness = roughness
	return material


func _emissive_material(color: Color, energy: float) -> StandardMaterial3D:
	var material := _material(color.darkened(0.5), 0.2, 0.2)
	material.emission_enabled = true
	material.emission = color
	material.emission_energy_multiplier = energy
	return material


func _count_mesh_parts(node: Node) -> int:
	var count := 1 if node is MeshInstance3D else 0
	for child in node.get_children():
		count += _count_mesh_parts(child)
	return count
