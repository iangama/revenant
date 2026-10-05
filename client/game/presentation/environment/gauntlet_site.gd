extends Node3D

var _label: Label3D
var _reserve: MeshInstance3D
var _reserve_label: Label3D


func _ready() -> void:
	var terminal := preload("res://presentation/environment/modules/relay_terminal.tscn").instantiate()
	terminal.position = Vector3(0, 0, -0.8)
	terminal.scale = Vector3.ONE * 0.55
	add_child(terminal)
	var ring := MeshInstance3D.new()
	var mesh := TorusMesh.new()
	mesh.inner_radius = 0.48
	mesh.outer_radius = 0.65
	ring.mesh = mesh
	ring.position.y = 0.1
	var material := StandardMaterial3D.new()
	material.albedo_color = Color("35d0d0")
	material.emission_enabled = true
	material.emission = Color("35d0d0")
	material.emission_energy_multiplier = 0.5
	ring.material_override = material
	add_child(ring)
	_label = Label3D.new()
	_label.position.y = 2.0
	_label.font_size = 30
	_label.pixel_size = 0.016
	_label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	_label.no_depth_test = true
	add_child(_label)
	_reserve = MeshInstance3D.new()
	_reserve.mesh = mesh.duplicate()
	_reserve.position = Vector3(1, 0.12, 0)
	var reserve_material: StandardMaterial3D = material.duplicate()
	reserve_material.albedo_color = Color("f5a524")
	reserve_material.emission = Color("f5a524")
	_reserve.material_override = reserve_material
	add_child(_reserve)
	_reserve_label = _label.duplicate()
	_reserve_label.position = Vector3(1, 0.9, 0)
	_reserve_label.font_size = 24
	add_child(_reserve_label)
	visible = false


func present(stage: int, transferring: bool, holding: bool, modifier: Dictionary = {}) -> void:
	visible = transferring
	if not transferring: return
	position = Vector3(2, 0, 6) if stage == 1 else Vector3(5, 0, -6)
	_label.text = tr("TRANSFER • UP TO +24 HP") + "\n" + (tr("TRANSFER • STAY STILL") if holding else tr("HOLD HERE • 1.2 S"))

	var single := "single_reserve" in str(modifier.get("preset_id", ""))
	_reserve.visible = single and not modifier.get("reserve_used", false)
	_reserve_label.visible = _reserve.visible
	_reserve_label.text = tr("RESERVE • +24 HP ONCE")
	if single: _label.text = tr("TRANSFER • NO HEALING") + "\n" + tr("HOLD HERE • 1.2 S")
