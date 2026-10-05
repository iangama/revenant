extends Node3D

const STATIONS := [Vector3(-1, 0, -5), Vector3(4, 0, -5), Vector3(4, 0, -1)]
const NAMES := ["1 • COOLANT INTAKE", "2 • TRANSFER", "3 • DELIVERY"]
const RECOVERY_NAMES := ["1 • INTAKE", "2 • TRANSFER", "3 • DELIVERY"]
var _labels: Array[Label3D] = []


func _ready() -> void:
	var material := StandardMaterial3D.new()
	material.albedo_color = Color("35d0d0")
	material.emission_enabled = true
	material.emission = Color("35d0d0")
	material.emission_energy_multiplier = 0.5
	for index in STATIONS.size():
		var terminal := preload("res://presentation/environment/modules/relay_terminal.tscn").instantiate()
		terminal.position = STATIONS[index] + Vector3(0.0, 0.0, -0.7)
		terminal.scale = Vector3.ONE * 0.55
		add_child(terminal)
		var ring := MeshInstance3D.new()
		var mesh := TorusMesh.new()
		mesh.inner_radius = 0.48
		mesh.outer_radius = 0.62
		ring.mesh = mesh
		ring.position = STATIONS[index] + Vector3(0, 0.09, 0)
		ring.material_override = material
		add_child(ring)
		var label := Label3D.new()
		label.position = STATIONS[index] + Vector3(0, 2.0, 0)
		label.font_size = 30
		label.pixel_size = 0.008
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.text = NAMES[index]
		add_child(label)
		_labels.append(label)
	visible = false


func present(available: bool, phase: String, untimed: bool = false) -> void:
	visible = available or not phase.is_empty()
	var active := 0 if phase in ["", "expired"] else (1 if phase == "transfer" else 2)
	for index in _labels.size():
		var label := _labels[index]
		label.modulate = Color("f5a524") if index == active else Color("a9b8cc")
		label.text = tr(RECOVERY_NAMES[index]) if untimed else NAMES[index]
		label.pixel_size = 0.016 if untimed else 0.008
		label.no_depth_test = untimed
		if phase == "completed" or index < active:
			label.text += "\n" + (tr("✓ CONFIRMED") if untimed else "✓ CHARGED")
			label.modulate = Color("35d0d0")
		elif index == active:
			if untimed:
				label.text += "\n" + (tr("HOLD HERE • 1.2 S") if index > 0 else tr("RETRIEVE • UNTIMED"))
			else:
				label.text += "\nHOLD HERE • 1.2 S" if index > 0 else "\nOPTIONAL • 8 SECOND RUN"
