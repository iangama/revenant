extends Node3D

var _label: Label3D


func _ready() -> void:
	var marker := MeshInstance3D.new()
	var ring := TorusMesh.new()
	ring.inner_radius = 0.6
	ring.outer_radius = 0.8
	ring.rings = 16
	marker.mesh = ring
	marker.material_override = preload("res://presentation/environment/materials/objective_amber.tres")
	marker.position = Vector3(5, 0.13, 2)
	add_child(marker)
	_label = Label3D.new()
	_label.position = Vector3(5, 2.8, 2)
	_label.font_size = 30
	_label.pixel_size = 0.01
	_label.outline_size = 10
	_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	add_child(_label)
	visible = false


func present(available: bool, fighting: bool) -> void:
	visible = available or fighting
	_label.text = "PRISM WARDEN\nOPTIONAL CORE • ENTER HERE" if available else "RETREAT WEST\nLEAVE THE MARKED SQUARE"
	_label.position = Vector3(5, 2.8, 2) if available else Vector3(4.5, 2.4, 2)
