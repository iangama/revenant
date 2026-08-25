extends Node3D

const MAX_ACTIVE_EFFECTS := 24

var _active_effects := 0
var _confirmed_exchanges := 0
var _cooldown_cues := 0
var _cyan_material: StandardMaterial3D
var _magenta_material: StandardMaterial3D
var _red_material: StandardMaterial3D
var _amber_material: StandardMaterial3D


func _ready() -> void:
	_cyan_material = _effect_material(Color("35d0d0"))
	_magenta_material = _effect_material(Color("d93678"))
	_red_material = _effect_material(Color("e8505b"))
	_amber_material = _effect_material(Color("f5a524"))


func play_confirmed_exchange(source: Node3D, target: Node3D, hostile: bool) -> void:
	if source == null or target == null:
		return
	_confirmed_exchanges += 1
	var source_position := source.global_position + Vector3(0.0, 1.05, 0.0)
	var target_position := target.global_position + Vector3(0.0, 0.82, 0.0)
	var shot_material := _magenta_material if hostile else _cyan_material
	_spawn_flash("MuzzleConfirmation", source_position, 0.13, shot_material, 0.11)
	_spawn_trail(source_position, target_position, shot_material)
	_spawn_flash("DamageImpact", target_position, 0.24, _red_material, 0.18)
	if not hostile:
		_spawn_corruption_shards(target_position)


func play_local_cooldown(origin: Vector3, duration_ms: int) -> void:
	_cooldown_cues += 1
	var cue := MeshInstance3D.new()
	cue.name = "LocalCooldownCue"
	var mesh := TorusMesh.new()
	mesh.inner_radius = 0.34
	mesh.outer_radius = 0.42
	mesh.rings = 20
	mesh.ring_segments = 8
	cue.mesh = mesh
	cue.material_override = _amber_material
	cue.scale = Vector3(0.2, 0.2, 0.2)
	_add_bounded(cue)
	cue.global_position = origin + Vector3(0.0, 0.06, 0.0)
	var lifetime := clampf(float(duration_ms) / 1000.0, 0.12, 1.2)
	var tween := create_tween().set_parallel(true)
	tween.tween_property(cue, "scale", Vector3.ONE, lifetime)
	tween.tween_property(cue, "transparency", 1.0, lifetime)
	tween.finished.connect(_expire.bind(cue))


func presentation_state() -> Dictionary:
	return {
		"active_effects": _active_effects,
		"confirmed_exchanges": _confirmed_exchanges,
		"cooldown_cues": _cooldown_cues,
		"maximum_active_effects": MAX_ACTIVE_EFFECTS,
		"permanent_particles": 0,
	}


func _spawn_trail(start: Vector3, finish: Vector3, material: Material) -> void:
	var direction := finish - start
	var length := direction.length()
	if length <= 0.01:
		return
	var trail := MeshInstance3D.new()
	trail.name = "ConfirmedShotTrail"
	var mesh := CylinderMesh.new()
	mesh.top_radius = 0.025
	mesh.bottom_radius = 0.055
	mesh.height = length
	mesh.radial_segments = 8
	trail.mesh = mesh
	trail.material_override = material
	_add_bounded(trail)
	trail.global_position = start.lerp(finish, 0.5)
	trail.quaternion = Quaternion(Vector3.UP, direction.normalized())
	var tween := create_tween()
	tween.tween_property(trail, "transparency", 1.0, 0.14)
	tween.finished.connect(_expire.bind(trail))


func _spawn_flash(effect_name: String, position: Vector3, radius: float, material: Material, lifetime: float) -> void:
	var flash := MeshInstance3D.new()
	flash.name = effect_name
	var mesh := SphereMesh.new()
	mesh.radius = radius
	mesh.height = radius * 2.0
	mesh.radial_segments = 12
	mesh.rings = 6
	flash.mesh = mesh
	flash.material_override = material
	flash.scale = Vector3(0.25, 0.25, 0.25)
	_add_bounded(flash)
	flash.global_position = position
	var tween := create_tween().set_parallel(true)
	tween.tween_property(flash, "scale", Vector3(1.7, 1.7, 1.7), lifetime)
	tween.tween_property(flash, "transparency", 1.0, lifetime)
	tween.finished.connect(_expire.bind(flash))


func _spawn_corruption_shards(position: Vector3) -> void:
	for index in range(4):
		var shard := MeshInstance3D.new()
		shard.name = "CorruptionFragment"
		var mesh := PrismMesh.new()
		mesh.size = Vector3(0.08, 0.22, 0.08)
		shard.mesh = mesh
		shard.material_override = _magenta_material
		shard.rotation_degrees.y = index * 90.0
		_add_bounded(shard)
		shard.global_position = position
		var direction := Vector3(cos(index * PI * 0.5), 0.65, sin(index * PI * 0.5))
		var tween := create_tween().set_parallel(true)
		tween.tween_property(shard, "global_position", position + direction * 0.55, 0.22)
		tween.tween_property(shard, "transparency", 1.0, 0.22)
		tween.finished.connect(_expire.bind(shard))


func _add_bounded(effect: MeshInstance3D) -> void:
	if _active_effects >= MAX_ACTIVE_EFFECTS:
		effect.queue_free()
		return
	add_child(effect)
	_active_effects += 1


func _expire(effect: Node) -> void:
	if is_instance_valid(effect) and effect.get_parent() == self:
		_active_effects = maxi(0, _active_effects - 1)
		effect.queue_free()


func _effect_material(color: Color) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = Color(color, 0.86)
	material.emission_enabled = true
	material.emission = color
	material.emission_energy_multiplier = 2.4
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	return material
