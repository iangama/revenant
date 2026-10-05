extends Node3D

var _markers: Array[Node3D] = []
var _labels: Array[Label3D] = []
var _shield: MeshInstance3D


func _ready() -> void:
	for point in [Vector3(6, 0, 0), Vector3(3, 0, 3), Vector3(10, 0, 0)]:
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
	_shield = MeshInstance3D.new()
	var shell := SphereMesh.new()
	shell.radius = 1.5
	shell.height = 3
	_shield.mesh = shell
	var material := StandardMaterial3D.new()
	material.albedo_color = Color(0.2, 0.8, 1, 0.3)
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_shield.material_override = material
	add_child(_shield)
	visible = false


func present(active: bool, checkpoint: int, complete: bool, guard_position: Vector3, direct := false) -> void:
	visible = active and not complete
	var selected: int = ({0: 0, 2: 1, 3: 2} if direct else {0: 0, 1: 1, 3: 2}).get(checkpoint, -1)
	var titles := ["OPEN THE BREACH", "DRAIN THE RELAY", "CORE THRESHOLD"]
	for i in range(_markers.size()):
		_markers[i].visible = i == selected
		_labels[i].text = tr(titles[i])
	_shield.visible = checkpoint == 1 and not direct
	_shield.position = guard_position + Vector3(0, 1.2, 0)
