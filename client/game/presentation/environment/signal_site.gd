extends Node3D

var _approach_ring: MeshInstance3D
var _beacon: Label3D
var _terminal_label: Label3D
var _terminal: Node3D
var _challenge_bounds: Node3D
var _delivery: Node3D
var _delivery_label: Label3D
var _survival_stations: Node3D
var _survival_labels: Array[Label3D] = []


func _ready() -> void:
	var armor := StandardMaterial3D.new()
	armor.albedo_color = Color("34485b")
	armor.metallic = 0.65
	var amber := StandardMaterial3D.new()
	amber.albedo_color = Color("f5a524")
	amber.emission_enabled = true
	amber.emission = Color("f5a524")
	amber.emission_energy_multiplier = 0.6
	_box(Vector3(4, 1.8, 1), Vector3(-4, 0.9, 1), armor)
	_box(Vector3(4.03, 0.13, 1.03), Vector3(-4, 1.65, 1), amber)
	var ring := MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.inner_radius = 0.65
	torus.outer_radius = 0.82
	ring.mesh = torus
	_approach_ring = ring
	ring.material_override = amber
	ring.position = Vector3(-4, 0.08, 4)
	add_child(ring)
	_beacon = _label("LOST SIGNAL\nOPTIONAL • STANDARD REWARD", Vector3(-4, 2.2, 4))
	var terminal := preload("res://presentation/environment/modules/relay_terminal.tscn").instantiate()
	_terminal = terminal
	terminal.position = Vector3(-4, 0, -3)
	terminal.scale = Vector3.ONE * 0.7
	add_child(terminal)
	_terminal_label = _label("RECOVER SIGNAL", Vector3(-4, 2.2, -3))
	_terminal_label.visible = false
	_delivery = Node3D.new()
	add_child(_delivery)
	var intake := preload("res://presentation/environment/modules/relay_terminal.tscn").instantiate()
	intake.position = Vector3(-1, 0, -5.7)
	intake.scale = Vector3.ONE * 0.55
	_delivery.add_child(intake)
	var delivery_ring := ring.duplicate()
	delivery_ring.position = Vector3(-1, 0.08, -5)
	_delivery.add_child(delivery_ring)
	_delivery_label = _label("DELIVER POWER CELL", Vector3(-1, 2.0, -5))
	_delivery_label.reparent(_delivery)
	_delivery.visible = false
	_challenge_bounds = Node3D.new()
	add_child(_challenge_bounds)
	for segment in [Vector3(-4, 0.08, -0.45), Vector3(-4, 0.08, 5.45)]:
		var boundary := _box(Vector3(10.9, 0.08, 0.08), segment, amber)
		boundary.reparent(_challenge_bounds)
	for segment in [Vector3(-9.45, 0.08, 2.5), Vector3(1.45, 0.08, 2.5)]:
		var boundary := _box(Vector3(0.08, 0.08, 5.9), segment, amber)
		boundary.reparent(_challenge_bounds)
	_challenge_bounds.visible = false
	_survival_stations = Node3D.new()
	add_child(_survival_stations)
	for position in [Vector3(-8, 0, 0), Vector3(0, 0, 0), Vector3(-4, 0, 5)]:
		var marker := ring.duplicate()
		marker.position = position + Vector3(0, 0.09, 0)
		marker.scale = Vector3.ONE * 0.6
		_survival_stations.add_child(marker)
		var label := _label("", position + Vector3(0, 1.5, -0.5))
		label.font_size = 48
		label.pixel_size = 0.018
		label.no_depth_test = true
		label.reparent(_survival_stations)
		_survival_labels.append(label)
	_survival_stations.visible = false
	visible = false


func present(available: bool, objective: Dictionary) -> void:
	_survival_stations.visible = false
	_terminal.visible = true
	_challenge_bounds.visible = false
	_delivery.visible = false
	_beacon.position = Vector3(-4, 2.2, 4)
	_approach_ring.position = Vector3(-4, 0.08, 4)
	_beacon.text = "LOST SIGNAL\nOPTIONAL • STANDARD REWARD"
	visible = available or not objective.is_empty()
	_beacon.visible = available and objective.is_empty()
	_terminal_label.visible = objective.get("progress", 0) >= 1
	_terminal_label.text = "SIGNAL RECEIVED" if objective.get("state") == "Completed" else "RECOVER SIGNAL\nAPPROACH THE TERMINAL"
	_terminal_label.modulate = Color("35d0d0") if objective.get("state") == "Completed" else Color("f5a524")


func present_supply(checkpoint: int, complete: bool, service := false) -> void:
	_survival_stations.visible = false
	_terminal.visible = true
	_challenge_bounds.visible = false
	visible = true
	_beacon.visible = checkpoint == 0 and not complete
	_beacon.text = tr("WESTERN SERVICE PATH") if service else tr("WEST SUPPLY ROUTE")
	_beacon.position = Vector3(-8, 2.2, -3) if service else Vector3(-4, 2.2, 4)
	_approach_ring.position = Vector3(-8, 0.08, -3) if service else Vector3(-4, 0.08, 4)
	_terminal_label.visible = checkpoint >= 2 or complete
	_terminal_label.text = "CELL RECOVERED" if checkpoint >= 3 or complete else "RECOVER POWER CELL"
	_terminal_label.modulate = Color("35d0d0") if checkpoint >= 3 or complete else Color("f5a524")
	_delivery.visible = checkpoint >= 3 or complete
	_delivery_label.text = "NORTH RELAY POWERED" if complete else "DELIVER POWER CELL\nNO TIME LIMIT"
	_delivery_label.modulate = Color("35d0d0") if complete else Color("f5a524")


func present_challenge() -> void:
	_survival_stations.visible = false
	visible = true
	_terminal.visible = false
	_beacon.visible = false
	_terminal_label.visible = false
	_delivery.visible = false
	_challenge_bounds.visible = true
	_approach_ring.position = Vector3(-4, 0.08, 4)


func present_survival(state: Dictionary) -> void:
	present_challenge()
	_survival_stations.visible = true
	var phase: String = state.get("phase", "west")
	var names := [tr("WEST RELAY"), tr("EAST RELAY"), tr("RESERVE")]
	for index in 3:
		var complete: bool = (index == 0 and phase in ["east", "return", "completed"]) or (index == 1 and phase in ["return", "completed"]) or (index == 2 and state.get("reserve_used", false))
		_survival_labels[index].text = names[index] + (" ✓" if complete else "")
		_survival_labels[index].modulate = Color("35d0d0") if complete else Color("f5a524")


func _box(size: Vector3, position: Vector3, material: Material) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	var mesh := BoxMesh.new()
	mesh.size = size
	instance.mesh = mesh
	instance.material_override = material
	instance.position = position
	add_child(instance)
	return instance


func _label(text: String, position: Vector3) -> Label3D:
	var label := Label3D.new()
	label.text = text
	label.position = position
	label.font_size = 36
	label.pixel_size = 0.009
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.modulate = Color("f5a524")
	add_child(label)
	return label
