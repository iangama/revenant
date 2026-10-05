extends Node3D

var _label: Label3D
var _pad: Node3D
var _edges: Array
var bulwark := false
var support := false
var elite_id := ""


func _ready() -> void:
	var north := bulwark or not elite_id.is_empty()
	var material := preload("res://presentation/environment/materials/objective_amber.tres")
	var marker := MeshInstance3D.new()
	var ring := TorusMesh.new()
	ring.inner_radius = 0.65
	ring.outer_radius = 0.85
	marker.mesh = ring
	marker.material_override = material
	marker.position = Vector3(6, 0.09, -8) if north else Vector3(4, 0.09, 8)
	if support:
		marker.position = Vector3(2, 0.09, 6)
	if not elite_id.is_empty():
		marker.position = Vector3(5, 0.09, -6 if elite_id == "bastion_link" else -10)
	add_child(marker)
	_pad = Node3D.new()
	add_child(_pad)
	_edges = [Vector4(7.5, -4.5, 6, 0.1), Vector4(7.5, -10.5, 6, 0.1), Vector4(4.5, -7.5, 0.1, 6), Vector4(10.5, -7.5, 0.1, 6)] if north else [Vector4(6, 4.5, 9, 0.1), Vector4(6, 10.5, 9, 0.1), Vector4(1.5, 7.5, 0.1, 6), Vector4(10.5, 7.5, 0.1, 6)]
	if not support and elite_id.is_empty():
		_build_pad()
	_label = Label3D.new()
	_label.text = "GLASS LANCER\nOPTIONAL • APPROACH TO TRAIN"
	_label.position = Vector3(4, 2.0, 8)
	_label.font_size = 32
	_label.pixel_size = 0.01
	_label.outline_size = 8
	_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	add_child(_label)
	visible = false


func _build_pad() -> void:
	for edge: Vector4 in _edges:
		var stripe := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(edge.z, 0.03, edge.w)
		stripe.mesh = box
		stripe.position = Vector3(edge.x, 0.14, edge.y)
		stripe.material_override = preload("res://presentation/environment/materials/objective_amber.tres")
		_pad.add_child(stripe)


func present(available: bool, fighting: bool, bounded_contract := false) -> void:
	visible = available or fighting
	_pad.visible = fighting
	_label.visible = not bounded_contract
	if support or not elite_id.is_empty():
		if fighting and _pad.get_child_count() == 0:
			_build_pad()
		elif not fighting:
			for stripe in _pad.get_children():
				_pad.remove_child(stripe)
				stripe.queue_free()
		if not elite_id.is_empty():
			_label.position = Vector3(5, 3.4, -5) if fighting else Vector3(4, 2.8, -6 if elite_id == "bastion_link" else -10)
			_label.text = "RETREAT SOUTH\nLEAVE THE TRAINING PAD" if fighting else "BASTION LINK\nBULWARK + MENDER" if elite_id == "bastion_link" else "CROSSED GUARD\nBULWARK + LANCER"
			return
		_label.position = Vector3(2, 2.8, 4.5) if fighting else Vector3(2, 2, 6)
		_label.text = "RETREAT NORTH\nLEAVE THE TRAINING PAD" if fighting else "MENDER + LANCER\nOPTIONAL • APPROACH TO TRAIN"
		return
	if bulwark:
		_label.position = Vector3(4.5, 3.4, -5) if fighting else Vector3(6, 2, -8)
		_label.text = "RETREAT SOUTH\nLEAVE THE TRAINING PAD" if fighting else "STEEL BULWARK\nOPTIONAL • APPROACH TO TRAIN"
		return
	_label.position = Vector3(6, 1.6, 4.5) if fighting else Vector3(4, 2, 8)
	_label.text = "RETREAT NORTH\nLEAVE THE TRAINING PAD" if fighting else "GLASS LANCER\nOPTIONAL • APPROACH TO TRAIN"
