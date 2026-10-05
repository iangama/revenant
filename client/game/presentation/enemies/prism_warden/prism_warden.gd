extends "res://presentation/enemies/enemy_presentation.gd"

var _plates: Array[MeshInstance3D] = []
var _floor: MeshInstance3D
var _label: Label3D
var _cue := {}


func family_name() -> String:
	return "prism-warden"


func _build_body() -> void:
	var armor := preload("res://presentation/environment/materials/blue_petrol.tres")
	var dark := preload("res://presentation/environment/materials/graphite.tres")
	var core := _emissive_material(Color("91e6eb"), 1.4)
	_set_core_material(core)
	for side in [-1, 1]:
		var crystal := _add_cylinder("Crystal%d" % side, 0.62, 1.1, Vector3(0, 1.55 + side * 0.55, 0), core)
		crystal.mesh.radial_segments = 6
		crystal.mesh.top_radius = 0.0
		crystal.rotation.z = PI if side < 0 else 0.0
	_add_cylinder("Anchor", 0.72, 0.25, Vector3(0, 0.25, 0), dark)
	for index in 4:
		var angle := index * PI / 2 + PI / 4
		var plate := _add_box("Shell%d" % index, Vector3(0.5, 1.75, 0.35), Vector3(sin(angle), 1.25, cos(angle)), armor, Vector3(0, rad_to_deg(angle), 12))
		_plates.append(plate)
		_add_box("Crown%d" % index, Vector3(0.18, 0.4, 0.18), Vector3(sin(angle) * 0.5, 2.5, cos(angle) * 0.5), dark)
	_floor = MeshInstance3D.new()
	_floor.name = "ConfirmedPrismFloor"
	_floor.mesh = ImmediateMesh.new()
	var warning := preload("res://presentation/environment/materials/objective_amber.tres").duplicate() as StandardMaterial3D
	warning.cull_mode = BaseMaterial3D.CULL_DISABLED
	warning.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	warning.albedo_color = Color("ffc45a")
	_floor.material_override = warning
	_floor.position.y = 0.34
	add_child(_floor)
	_label = Label3D.new()
	_label.position = Vector3(0, 3.0, 0)
	_label.font_size = 32
	_label.pixel_size = 0.01
	_label.outline_size = 10
	_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	add_child(_label)
	present_prism({"phase": "Lanes", "mode": "Opening", "pattern": null})


func present_prism(cue: Dictionary) -> void:
	_cue = cue.duplicate(true)
	var open: bool = cue.get("mode") == "Recovery"
	for index in _plates.size():
		var angle := index * PI / 2 + PI / 4
		_plates[index].position = Vector3(sin(angle) * (1.4 if open else 0.85), 0.65 if open else 1.3, cos(angle) * (1.4 if open else 0.85))
	var title := "I • SHELL CLOSED"
	if cue.get("phase") == "Pulses": title = "II • SHELL CLOSED"
	if open: title = "SHELL OPEN • ATTACK"
	if cue.get("mode") == "Shifting": title = "PHASE II • CORE PULSES"
	var pattern: Dictionary = cue.get("pattern") if cue.get("pattern") is Dictionary else {}
	if cue.get("mode") == "Warning":
		title = {"AcrossX": "MOVE OFF THE STRIPE", "AcrossZ": "MOVE OFF THE STRIPE", "Center": "LEAVE THE CENTER", "Perimeter": "RETURN TO THE CENTER"}.get(pattern.get("shape"), title)
	_label.text = title
	var mesh := _floor.mesh as ImmediateMesh
	mesh.clear_surfaces()
	mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
	for edge in [Rect2(4.5,-3.5,7,0.07), Rect2(4.5,3.43,7,0.07), Rect2(4.5,-3.5,0.07,7), Rect2(11.43,-3.5,0.07,7)]:
		_rect(mesh, edge)
	if cue.get("mode") == "Warning":
		match pattern.get("shape"):
			"AcrossX": _rect(mesh, Rect2(4.5, float(pattern.z)-0.48, 7, 0.96))
			"AcrossZ": _rect(mesh, Rect2(float(pattern.x)-0.48, -3.5, 0.96, 7))
			"Center": _rect(mesh, Rect2(6.5,-1.5,3,3))
			"Perimeter":
				for area in [Rect2(4.5,-3.5,7,2), Rect2(4.5,1.5,7,2), Rect2(4.5,-1.5,2,3), Rect2(9.5,-1.5,2,3)]:
					_rect(mesh, area)
	mesh.surface_end()


func _rect(mesh: ImmediateMesh, area: Rect2) -> void:
	var a := Vector3(area.position.x - 8, 0, area.position.y)
	var b := a + Vector3(area.size.x, 0, 0)
	var c := a + Vector3(area.size.x, 0, area.size.y)
	var d := a + Vector3(0, 0, area.size.y)
	for vertex in [a, c, b, a, d, c]: mesh.surface_add_vertex(vertex)


func play_confirmed_block() -> void:
	_label.text = "SHELL CLOSED • WAIT FOR OPENING"


func set_danger_close(danger_close: bool) -> void:
	super.set_danger_close(danger_close)
	_danger_indicator.visible = false


func presentation_state() -> Dictionary:
	var state := super.presentation_state()
	state["prism"] = _cue.duplicate(true)
	state["floor_visible"] = _floor.visible
	return state


func retire() -> void:
	_floor.visible = false
	_label.visible = false
	super.retire()
