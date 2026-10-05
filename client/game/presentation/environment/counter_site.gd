extends Node3D

var _markers: Array[Node3D] = []
var _labels: Array[Label3D] = []


func _ready() -> void:
	for point in [Vector3(2, 0, 6), Vector3(7, 0, 6), Vector3(5, 0, -6), Vector3(9, 0, -6)]:
		var marker := Node3D.new()
		marker.position = point
		add_child(marker)
		var mesh := MeshInstance3D.new()
		var ring := TorusMesh.new()
		ring.inner_radius = 0.65
		ring.outer_radius = 0.85
		mesh.mesh = ring
		mesh.position.y = 0.1
		mesh.material_override = preload("res://presentation/environment/materials/objective_amber.tres")
		marker.add_child(mesh)
		var label := Label3D.new()
		label.position.y = 1.4
		label.font_size = 32
		label.pixel_size = 0.01
		label.outline_size = 8
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		marker.add_child(label)
		_markers.append(marker)
		_labels.append(label)
	visible = false


func present(active: bool, checkpoint: int, complete: bool) -> void:
	visible = active and not complete
	var selected: int = {0: 0, 2: 1, 3: 2, 5: 3}.get(checkpoint, -1)
	var titles := ["REPAIR LINK", "DECODE TRANSMISSION", "BASTION LINK", "ISOLATE EMITTER"]
	for i in range(_markers.size()):
		_markers[i].visible = i == selected
		_labels[i].text = tr(titles[i])
