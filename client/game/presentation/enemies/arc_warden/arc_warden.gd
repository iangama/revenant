extends "res://presentation/enemies/warden/warden.gd"

# A Warden variant shown only for the confirmed Arc Surge route event.
func _build_body() -> void:
	super()
	_core_material.albedo_color = Color("35d0d0")
	_core_material.emission = Color("35d0d0")
	_containment_material.albedo_color = Color("907bff")
	_containment_material.emission = Color("907bff")
	var armor := _material(Color("294d64"), 0.72, 0.36)
	var arc := _emissive_material(Color("35d0d0"), 1.8)
	for side in [-1, 1]:
		_add_box("ArcFin%d" % side, Vector3(0.28, 1.5, 0.42), Vector3(side * 1.18, 1.6, 0.1), armor, Vector3(0, 0, side * -28))
		_add_box("ArcFinLight%d" % side, Vector3(0.10, 1.05, 0.44), Vector3(side * 1.32, 1.86, 0.1), arc, Vector3(0, 0, side * -28))
	_add_torus("ArcCrown", 0.72, 0.05, Vector3(0, 2.25, 0), arc)


func play_authoritative_move(local_offset: Vector3) -> void:
	super(local_offset)
	if _retired or _reduced_motion or local_offset.length() < 0.1:
		return
	var trail := MeshInstance3D.new()
	var mesh := BoxMesh.new()
	mesh.size = Vector3(0.06, 0.06, local_offset.length())
	trail.mesh = mesh
	var material := _emissive_material(Color("35d0d0"), 1.4)
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	trail.material_override = material
	add_child(trail)
	trail.position = local_offset * 0.5 + Vector3(0, 1, 0)
	trail.look_at(to_global(Vector3(0, 1, 0)), Vector3.UP)
	var fade := create_tween()
	fade.tween_property(material, "albedo_color:a", 0.0, 0.22)
	fade.tween_callback(trail.queue_free)


func presentation_state() -> Dictionary:
	var state := super()
	state["variant"] = "arc_surge"
	return state
