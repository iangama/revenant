extends "res://presentation/enemies/enemy_presentation.gd"

var _shield: Node3D
var _slam_sector: MeshInstance3D
var _stance_label: Label3D
var _braced := true
var _facing: Array = [-1, 0]


func family_name() -> String:
	return "steel-bulwark"


func _build_body() -> void:
	var armor := preload("res://presentation/environment/materials/blue_petrol.tres")
	var dark := preload("res://presentation/environment/materials/graphite.tres")
	var trim := _emissive_material(Color("60d1df"), 1.4)
	_set_core_material(trim)
	_add_box("ArmoredBody", Vector3(1.2, 1.2, 0.8), Vector3(0, 0.9, 0), armor)
	_add_box("Crest", Vector3(0.65, 0.28, 0.6), Vector3(0, 1.68, 0), dark)
	_add_sphere("Sensor", 0.16, Vector3(0, 1.4, 0.44), trim)
	for side in [-1, 1]:
		_add_box("Foot%d" % side, Vector3(0.42, 0.3, 0.8), Vector3(side * 0.42, 0.18, 0), dark)
	_shield = Node3D.new()
	_shield.name = "ConfirmedShield"
	_visual_root.add_child(_shield)
	for side in [-1, 1]:
		var plate := _add_box("Shield%d" % side, Vector3(0.7, 1.65, 0.18), Vector3(side * 0.38, 0.92, 0.72), armor, Vector3(0, side * -12, 0))
		plate.reparent(_shield, false)
		var edge := _add_box("ShieldEdge%d" % side, Vector3(0.08, 1.65, 0.2), Vector3(side * 0.74, 0.92, 0.65), trim)
		edge.reparent(_shield, false)
	_slam_sector = MeshInstance3D.new()
	_slam_sector.name = "ConfirmedSlamSector"
	var mesh := ImmediateMesh.new()
	var danger := preload("res://presentation/environment/materials/objective_amber.tres").duplicate() as StandardMaterial3D
	danger.cull_mode = BaseMaterial3D.CULL_DISABLED
	danger.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	danger.albedo_color = Color("ffc45a")
	mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES, danger)
	for segment in 12:
		var start := -PI / 4 + segment * PI / 24
		var end := start + PI / 24
		mesh.surface_add_vertex(Vector3.ZERO)
		mesh.surface_add_vertex(Vector3(sin(start), 0, cos(start)) * 2.15)
		mesh.surface_add_vertex(Vector3(sin(end), 0, cos(end)) * 2.15)
	mesh.surface_end()
	_slam_sector.mesh = mesh
	_slam_sector.material_override = danger
	_slam_sector.position.y = 0.15
	add_child(_slam_sector)
	_stance_label = Label3D.new()
	_stance_label.position = Vector3(0, 2.15, 0)
	_stance_label.font_size = 36
	_stance_label.pixel_size = 0.01
	_stance_label.outline_size = 10
	_stance_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	add_child(_stance_label)
	present_defense({"facing": [-1, 0], "braced": true})


func present_defense(cue: Dictionary) -> void:
	_braced = cue.get("braced", false)
	_facing = cue.get("facing", [-1, 0])
	var facing_angle := atan2(float(_facing[0]), float(_facing[1]))
	_visual_root.rotation.y = facing_angle
	_slam_sector.rotation.y = facing_angle
	_slam_sector.visible = _braced
	_shield.position.y = 0 if _braced else -0.85
	_stance_label.text = "SHIELD UP • FLANK" if _braced else "SHIELD DOWN • ATTACK"


func play_confirmed_block() -> void:
	_stance_label.text = "BLOCKED • FLANK OR WAIT"


func set_danger_close(danger_close: bool) -> void:
	super.set_danger_close(danger_close)
	# Its confirmed wedge replaces the generic all-around melee danger disc.
	_danger_indicator.visible = false


func presentation_state() -> Dictionary:
	var state := super.presentation_state()
	state["braced"] = _braced
	state["facing"] = _facing
	state["stance_label"] = _stance_label.text
	return state


func retire() -> void:
	_slam_sector.visible = false
	_stance_label.visible = false
	super.retire()
