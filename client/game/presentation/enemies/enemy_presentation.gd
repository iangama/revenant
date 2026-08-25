extends Node3D

var _motion_root: Node3D
var _visual_root: Node3D
var _danger_indicator: MeshInstance3D
var _target_indicator: MeshInstance3D
var _core_material: StandardMaterial3D
var _last_animation := "spawn"
var _targeted := false
var _danger_close := false
var _retired := false
var _idle_clock := 0.0


func _ready() -> void:
	_motion_root = Node3D.new()
	_motion_root.name = "AuthoritativeMotion"
	add_child(_motion_root)
	_visual_root = Node3D.new()
	_visual_root.name = "EnemyVisual"
	_motion_root.add_child(_visual_root)
	_build_indicators()
	_build_body()
	play_spawn()


func _process(delta: float) -> void:
	if _retired:
		return
	_idle_clock += delta
	_visual_root.position.y = sin(_idle_clock * 2.8) * 0.035
	if _core_material != null:
		_core_material.emission_energy_multiplier = 1.65 + sin(_idle_clock * 3.6) * 0.28


func play_spawn() -> void:
	_last_animation = "spawn"
	_motion_root.scale = Vector3(0.15, 0.15, 0.15)
	var tween := create_tween()
	tween.tween_property(_motion_root, "scale", Vector3.ONE, 0.24).set_trans(Tween.TRANS_BACK)


func play_authoritative_move(local_offset: Vector3) -> void:
	if _retired:
		return
	_last_animation = "chase"
	_motion_root.position = local_offset
	if absf(local_offset.x) + absf(local_offset.z) > 0.01:
		_visual_root.rotation.y = atan2(-local_offset.x, -local_offset.z)
		_visual_root.rotation_degrees.z = clampf(local_offset.x * 4.0, -7.0, 7.0)
	var tween := create_tween().set_parallel(true)
	tween.tween_property(_motion_root, "position", Vector3.ZERO, 0.15)
	tween.tween_property(_visual_root, "rotation_degrees:z", 0.0, 0.15)


func play_confirmed_attack() -> void:
	if _retired:
		return
	_last_animation = "attack_confirmed"
	var original_scale := _visual_root.scale
	_visual_root.scale = original_scale * 1.12
	if _core_material != null:
		_core_material.emission_energy_multiplier = 4.2
	var tween := create_tween().set_parallel(true)
	tween.tween_property(_visual_root, "scale", original_scale, 0.13)
	if _core_material != null:
		tween.tween_property(_core_material, "emission_energy_multiplier", 1.8, 0.16)


func play_confirmed_hit() -> void:
	if _retired:
		return
	_last_animation = "hit_confirmed"
	_motion_root.scale = Vector3(1.18, 0.82, 1.18)
	var tween := create_tween()
	tween.tween_property(_motion_root, "scale", Vector3.ONE, 0.14)


func retire() -> void:
	if _retired:
		return
	_retired = true
	_last_animation = "defeat_confirmed"
	_danger_indicator.visible = false
	_target_indicator.visible = false
	if _core_material != null:
		_core_material.emission_energy_multiplier = 0.08
	var tween := create_tween().set_parallel(true)
	tween.tween_property(_motion_root, "scale", Vector3(1.25, 0.08, 1.25), 0.24)
	tween.tween_property(_motion_root, "position:y", 0.06, 0.24)
	await get_tree().create_timer(0.26).timeout
	queue_free()


func set_targeted(targeted: bool) -> void:
	_targeted = targeted
	_target_indicator.visible = targeted and not _retired


func set_danger_close(danger_close: bool) -> void:
	_danger_close = danger_close
	_danger_indicator.visible = danger_close and not _retired


func presentation_state() -> Dictionary:
	var materials := {}
	_collect_materials(self, materials)
	return {
		"family": family_name(),
		"last_animation": _last_animation,
		"targeted": _targeted,
		"danger_close": _danger_close,
		"retired": _retired,
		"mesh_count": _count_meshes(self),
		"material_count": materials.size(),
	}


func family_name() -> String:
	return "enemy"


func _build_body() -> void:
	pass


func _set_core_material(material: StandardMaterial3D) -> void:
	_core_material = material


func _add_box(part_name: String, size: Vector3, position: Vector3, material: Material, rotation: Vector3 = Vector3.ZERO) -> MeshInstance3D:
	var part := MeshInstance3D.new()
	part.name = part_name
	var mesh := BoxMesh.new()
	mesh.size = size
	part.mesh = mesh
	part.position = position
	part.rotation_degrees = rotation
	part.material_override = material
	_visual_root.add_child(part)
	return part


func _add_sphere(part_name: String, radius: float, position: Vector3, material: Material) -> MeshInstance3D:
	var part := MeshInstance3D.new()
	part.name = part_name
	var mesh := SphereMesh.new()
	mesh.radius = radius
	mesh.height = radius * 2.0
	part.mesh = mesh
	part.position = position
	part.material_override = material
	_visual_root.add_child(part)
	return part


func _add_cylinder(part_name: String, radius: float, height: float, position: Vector3, material: Material) -> MeshInstance3D:
	var part := MeshInstance3D.new()
	part.name = part_name
	var mesh := CylinderMesh.new()
	mesh.top_radius = radius
	mesh.bottom_radius = radius
	mesh.height = height
	mesh.radial_segments = 16
	part.mesh = mesh
	part.position = position
	part.material_override = material
	_visual_root.add_child(part)
	return part


func _material(color: Color, metallic: float, roughness: float) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.metallic = metallic
	material.roughness = roughness
	return material


func _emissive_material(color: Color, energy: float) -> StandardMaterial3D:
	var material := _material(color.darkened(0.58), 0.22, 0.25)
	material.emission_enabled = true
	material.emission = color
	material.emission_energy_multiplier = energy
	return material


func _build_indicators() -> void:
	var indicator_material := StandardMaterial3D.new()
	indicator_material.albedo_color = Color(0.851, 0.212, 0.471, 0.18)
	indicator_material.emission_enabled = true
	indicator_material.emission = Color("d93678")
	indicator_material.emission_energy_multiplier = 0.75
	indicator_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	indicator_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_danger_indicator = _indicator("DangerZone", 2.0, indicator_material, 0.025)
	_target_indicator = _indicator("TargetSelection", 0.72, indicator_material, 0.04)
	_danger_indicator.visible = false
	_target_indicator.visible = false


func _indicator(indicator_name: String, radius: float, material: Material, height: float) -> MeshInstance3D:
	var indicator := MeshInstance3D.new()
	indicator.name = indicator_name
	var mesh := CylinderMesh.new()
	mesh.top_radius = radius
	mesh.bottom_radius = radius
	mesh.height = 0.012
	mesh.radial_segments = 32
	indicator.mesh = mesh
	indicator.position.y = height
	indicator.material_override = material
	add_child(indicator)
	return indicator


func _count_meshes(node: Node) -> int:
	var count := 1 if node is MeshInstance3D else 0
	for child in node.get_children():
		count += _count_meshes(child)
	return count


func _collect_materials(node: Node, materials: Dictionary) -> void:
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		if mesh_instance.material_override != null:
			materials[mesh_instance.material_override.get_instance_id()] = true
	for child in node.get_children():
		_collect_materials(child, materials)
