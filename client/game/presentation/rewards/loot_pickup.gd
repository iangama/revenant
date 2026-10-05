extends Node3D

const INTACT_CYAN_MATERIAL := preload("res://presentation/environment/materials/intact_cyan.tres")
const OBJECTIVE_AMBER_MATERIAL := preload("res://presentation/environment/materials/objective_amber.tres")
const MAX_ACTIVE_PICKUPS := 1
const PRESENTATION_DURATION_SECONDS := 2.03

var _reduced_motion := false
var _active_pickups := 0
var _confirmed_pickups := 0
var _last_item_id := ""
var _last_quantity := 0


func set_reduced_motion(enabled: bool) -> void:
	_reduced_motion = enabled


func play_confirmed_pickup(item_id: String, quantity: int, origin: Vector3, target: Vector3) -> bool:
	if item_id != "relay_core_fragment" or quantity <= 0 or _active_pickups >= MAX_ACTIVE_PICKUPS:
		return false
	_confirmed_pickups += 1
	_active_pickups += 1
	_last_item_id = item_id
	_last_quantity = quantity

	var pickup := Node3D.new()
	pickup.name = "ConfirmedRelayCoreFragment"
	add_child(pickup)
	pickup.global_position = origin + Vector3(0.0, 0.75, 0.0)
	pickup.scale = Vector3.ONE * 0.18
	_add_fragment(pickup, "FragmentCore", Vector3(0.24, 0.62, 0.20), Vector3.ZERO, OBJECTIVE_AMBER_MATERIAL, Vector3(0.0, 0.0, 18.0))
	_add_fragment(pickup, "FragmentShardLeft", Vector3(0.12, 0.34, 0.11), Vector3(-0.28, 0.02, 0.0), INTACT_CYAN_MATERIAL, Vector3(0.0, 0.0, -28.0))
	_add_fragment(pickup, "FragmentShardRight", Vector3(0.12, 0.32, 0.11), Vector3(0.27, -0.04, 0.0), INTACT_CYAN_MATERIAL, Vector3(0.0, 0.0, 32.0))
	_add_world_label(pickup, quantity)
	if _reduced_motion:
		pickup.scale = Vector3.ONE
		get_tree().create_timer(1.5).timeout.connect(_expire.bind(pickup))
		return true

	var destination := target + Vector3(0.0, 1.15, 0.0)
	var reveal_position := pickup.global_position + Vector3(0.0, 0.72, 0.0)
	var midpoint := reveal_position.lerp(destination, 0.48) + Vector3(0.0, 1.65, 0.0)
	var tween := create_tween()
	tween.tween_property(pickup, "global_position", reveal_position, 0.26).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	tween.parallel().tween_property(pickup, "scale", Vector3.ONE * 1.18, 0.26)
	tween.tween_interval(0.55)
	tween.tween_property(pickup, "global_position", midpoint, 0.48).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
	tween.parallel().tween_property(pickup, "rotation_degrees", Vector3(35.0, 210.0, 12.0), 0.48)
	tween.tween_property(pickup, "global_position", destination, 0.58).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_IN)
	tween.parallel().tween_property(pickup, "scale", Vector3.ONE * 0.40, 0.58)
	tween.tween_property(pickup, "scale", Vector3.ZERO, 0.16)
	tween.finished.connect(_expire.bind(pickup))
	return true


func presentation_state() -> Dictionary:
	return {
		"active_pickups": _active_pickups,
		"confirmed_pickups": _confirmed_pickups,
		"last_item_id": _last_item_id,
		"last_quantity": _last_quantity,
		"maximum_active_pickups": MAX_ACTIVE_PICKUPS,
		"maximum_meshes_per_pickup": 3,
		"permanent_particles": 0,
		"server_confirmed_only": true,
		"world_label_present": true,
		"presentation_duration_seconds": PRESENTATION_DURATION_SECONDS,
	}


func _add_fragment(parent: Node3D, fragment_name: String, size: Vector3, position: Vector3, material: Material, rotation: Vector3) -> void:
	var mesh := PrismMesh.new()
	mesh.size = size
	var fragment := MeshInstance3D.new()
	fragment.name = fragment_name
	fragment.mesh = mesh
	fragment.position = position
	fragment.rotation_degrees = rotation
	fragment.material_override = material
	parent.add_child(fragment)


func _add_world_label(parent: Node3D, quantity: int) -> void:
	var label := Label3D.new()
	label.name = "ConfirmedRewardLabel"
	label.text = "RELAY CORE FRAGMENT  +%d" % quantity
	label.position = Vector3(0.0, 1.02, 0.0)
	label.font_size = 26
	label.pixel_size = 0.0065
	label.outline_size = 8
	label.modulate = Color("f5d27a")
	label.outline_modulate = Color("071016")
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.fixed_size = false
	label.no_depth_test = true
	label.render_priority = 3
	parent.add_child(label)


func _expire(pickup: Node3D) -> void:
	_active_pickups = maxi(0, _active_pickups - 1)
	if is_instance_valid(pickup):
		pickup.queue_free()
