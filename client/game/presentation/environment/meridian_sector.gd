extends Node3D

# The renderer and server consume the same authored floor/station coordinates.
const MAP_PATH := "res://world/meridian.json"
const TITLES := {"arrival": "1 • ARRIVAL LOCK", "lens": "2 • LENS CISTERN", "gallery": "3 • SKY GALLERY", "bridge": "KEEPER'S WALK"}
const STATION_TITLES := ["1 • ARRIVAL LOCK", "2 • ALIGN THE LENS", "3 • SURVEY THE SKY", "4 • KEEPER'S LOG", "5 • RETURN THE LOG", "A NAME IN THE GLASS"]
var layout: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(MAP_PATH))
var _geometry: Node3D
var _threshold: Node3D
var _labels := {}
var _materials: Array[StandardMaterial3D] = []
var _built := false


func _ready() -> void:
	visible = false
	for color in [Color("475e68"), Color("afc9c0"), Color("edcf88"), Color("27454e")]:
		var material := StandardMaterial3D.new()
		material.albedo_color = color
		material.roughness = 0.86
		_materials.append(material)
	_threshold = Node3D.new()
	add_child(_threshold)
	_box(_threshold, Vector3(-11.7, 0.08, 0), Vector3(0.4, 0.15, 2.3), 2)
	_label(_threshold, Vector3(-11, 2.0, 0), "MERIDIAN ANNEX\nOPTIONAL • WEST", 36)


func _build() -> void:
	_built = true
	_geometry = Node3D.new()
	add_child(_geometry)
	for rect: Array in layout.floors:
		_box(_geometry, Vector3((rect[0] + rect[1]) / 2.0, -0.2, (rect[2] + rect[3]) / 2.0), Vector3(rect[1] - rect[0] + 1, 0.35, rect[3] - rect[2] + 1), 0)
	# Boundary rails exactly follow the authoritative cell edges. A single
	# MultiMesh keeps the perimeter bounded without hiding navigable openings.
	var transforms: Array[Transform3D] = []
	for x in range(-32, -11):
		for z in range(-10, 11):
			if not walkable(Vector3i(x, 0, z)):
				continue
			for direction in [Vector3i(1, 0, 0), Vector3i(-1, 0, 0), Vector3i(0, 0, 1), Vector3i(0, 0, -1)]:
				var next: Vector3i = Vector3i(x, 0, z) + direction
				if walkable(next) or next.x >= -12:
					continue
				var scale := Vector3(0.12, 0.38, 1.0) if direction.x else Vector3(1.0, 0.38, 0.12)
				transforms.append(Transform3D(Basis.IDENTITY.scaled(scale), Vector3(x, 0.25, z) + Vector3(direction) * 0.5))
	var perimeter := MultiMeshInstance3D.new()
	var batch := MultiMesh.new()
	batch.transform_format = MultiMesh.TRANSFORM_3D
	batch.mesh = BoxMesh.new()
	batch.instance_count = transforms.size()
	for index in transforms.size():
		batch.set_instance_transform(index, transforms[index])
	perimeter.multimesh = batch
	perimeter.material_override = _materials[1]
	_geometry.add_child(perimeter)
	# Arrival: open pale ribs and a low wayfinding strip.
	for x in [-14.5, -17.5, -20.0]:
		for z in [-3.65, 3.65]:
			_box(_geometry, Vector3(x, 1.6, z), Vector3(0.24, 3.2, 0.24), 1)
		_box(_geometry, Vector3(x, 3.25, 0), Vector3(0.24, 0.24, 7.3), 1)
	_box(_geometry, Vector3(-19, 0.015, 0), Vector3(12, 0.04, 0.12), 2)
	# Cistern: a recessed circular lens, with four static segmented vanes.
	_cylinder(Vector3(-28, -0.02, -5), 1.45, 0.1, 3)
	for offset in [Vector3(-1.5, 0, 0), Vector3(1.5, 0, 0), Vector3(0, 0, -1.5), Vector3(0, 0, 1.5)]:
		_box(_geometry, Vector3(-28, 0.15, -5) + offset, Vector3(0.5, 0.22, 0.5), 2)
	var lens := MeshInstance3D.new()
	var ring := TorusMesh.new()
	ring.inner_radius = 1.1
	ring.outer_radius = 1.3
	lens.mesh = ring
	lens.position = Vector3(-28, 2.6, -9.2)
	lens.rotation_degrees.x = 65
	lens.material_override = _materials[2]
	_geometry.add_child(lens)
	# Gallery: three broken astronomical frames against the open sky.
	for x in [-31, -28, -25]:
		_box(_geometry, Vector3(x, 2.5, 10.35), Vector3(0.2, 5, 0.25), 1)
		_box(_geometry, Vector3(x, 4.9, 9.7), Vector3(1.8, 0.2, 0.25), 2)
	for index in layout.stations.size():
		var station: Dictionary = layout.stations[index]
		if station.id == "meridian_return":
			continue
		var p := Vector3(station.position[0], 0, station.position[2])
		_cylinder(p + Vector3(0, 0.04, 0), 0.45, 0.08, 2)
		_labels[station.id] = _label(_geometry, p + Vector3(0, 1.8, -0.5), STATION_TITLES[index], 30)
	_label(_geometry, Vector3(-21, 1.7, 0), "HUB →\nRETURN EAST", 34)
	_label(_geometry, Vector3(-24, 1.7, 1), "2 NORTH / 3 SOUTH\n4 WEST", 30)


func present(available: bool, active: bool, player: Vector3, objectives: Dictionary, settings: Dictionary) -> String:
	visible = available or active
	if active and not _built:
		_build()
	var zone := zone_at(player) if active and player.x < -12 else ""
	if _geometry != null:
		_geometry.visible = not zone.is_empty()
	_threshold.visible = zone.is_empty()
	for id: String in _labels:
		var label: Label3D = _labels[id]
		var state: String = objectives.get(id, {}).get("state", "Pending")
		var index := 0
		for station: Dictionary in layout.stations:
			if station.id == id:
				break
			index += 1
		label.text = STATION_TITLES[index] + ("\n✓ RECORDED" if state == "Completed" else ("\n◇ APPROACH" if state == "Active" else ""))
		if id == "meridian_arrival" and objectives.get("meridian_return", {}).get("state") in ["Active", "Completed"]:
			label.text = STATION_TITLES[4] + ("\n✓ RECORDED" if objectives.meridian_return.state == "Completed" else "\n◇ APPROACH")
		label.modulate = Color.WHITE if settings.get("high_contrast", false) else Color("fff0cb")
		if id == "meridian_arrival":
			var target: Array = objectives.get("meridian_return", {}).get("position", [-16, 0, 0]) if objectives.get("meridian_return", {}).get("state") in ["Active", "Completed"] else [-16, 0, 0]
			label.position = Vector3(target[0], 1.8, target[2] - 0.5)
		label.visible = Vector2(label.position.x - player.x, label.position.z + 0.5 - player.z).length() <= 2.5
		if objectives.get("meridian_return", {}).get("position", [-16, 0, 0])[0] == -31:
			var returning: bool = objectives.meridian_return.state in ["Active", "Completed"]
			if id == "meridian_arrival":
				label.text = "RETURN WEST" + ("\n✓ RECORDED" if objectives.meridian_return.state == "Completed" else "\n◇ APPROACH")
				label.visible = label.visible and returning
			elif id == "meridian_log":
				label.visible = label.visible and not returning
	return zone


func zone_at(p: Vector3) -> String:
	if p.x >= -21:
		return "arrival"
	if p.z <= -2:
		return "lens"
	if p.z >= 2:
		return "gallery"
	return "bridge"


func walkable(p: Vector3i) -> bool:
	for r: Array in layout.floors:
		if p.y == 0 and p.x >= r[0] and p.x <= r[1] and p.z >= r[2] and p.z <= r[3]:
			return true
	return false


func _box(parent: Node3D, p: Vector3, dimensions: Vector3, material: int) -> void:
	var instance := MeshInstance3D.new()
	var mesh := BoxMesh.new()
	mesh.size = dimensions
	instance.mesh = mesh
	instance.position = p
	instance.material_override = _materials[material]
	parent.add_child(instance)


func _cylinder(p: Vector3, radius: float, height: float, material: int) -> void:
	var instance := MeshInstance3D.new()
	var mesh := CylinderMesh.new()
	mesh.top_radius = radius
	mesh.bottom_radius = radius
	mesh.height = height
	mesh.radial_segments = 24
	instance.mesh = mesh
	instance.position = p
	instance.material_override = _materials[material]
	_geometry.add_child(instance)


func _label(parent: Node3D, p: Vector3, text: String, font_size: int) -> Label3D:
	var label := Label3D.new()
	label.position = p
	label.text = text
	label.font_size = font_size
	label.pixel_size = 0.014
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.outline_size = 8
	parent.add_child(label)
	return label
