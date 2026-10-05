extends Node3D

const FLOOR_PANEL_SCENE := preload("res://presentation/environment/modules/floor_panel.tscn")
const WALL_PANEL_SCENE := preload("res://presentation/environment/modules/wall_panel.tscn")
const COLUMN_SCENE := preload("res://presentation/environment/modules/structural_column.tscn")
const DOOR_SCENE := preload("res://presentation/environment/modules/relay_core_door.tscn")
const TERMINAL_SCENE := preload("res://presentation/environment/modules/relay_terminal.tscn")
const DAMAGED_SECTION_SCENE := preload("res://presentation/environment/modules/damaged_section.tscn")
const GRAPHITE_MATERIAL := preload("res://presentation/environment/materials/graphite.tres")
const BLUE_PETROL_MATERIAL := preload("res://presentation/environment/materials/blue_petrol.tres")
const INTACT_CYAN_MATERIAL := preload("res://presentation/environment/materials/intact_cyan.tres")
const OBJECTIVE_AMBER_MATERIAL := preload("res://presentation/environment/materials/objective_amber.tres")
const DAMAGED_METAL_MATERIAL := preload("res://presentation/environment/materials/damaged_metal.tres")
const THREAT_MAGENTA_MATERIAL := preload("res://presentation/environment/materials/threat_magenta.tres")

const VISUAL_BOUNDS := 12.0
const DOOR_POSITION := Vector3(6.5, 0.0, 0.0)
const ARRIVAL_POSITION := Vector3.ZERO
const COOPERATION_ANCHOR_POSITION := Vector3(3.0, 0.0, 3.0)
const COOPERATION_CONSOLE_POSITION := Vector3(4.0, 0.0, 3.0)
const ENRICHMENT_GROUP_NAMES := [
	"ArrivalPlatform",
	"CooperationLane",
	"BreachThreshold",
	"RelayInfrastructure",
	"RelayCoreChamber",
	"CorruptionTrace",
]
const BACKGROUND_COLOR := Color("101a26")
const AMBIENT_LIGHT_COLOR := Color("8fa4b8")
const AMBIENT_LIGHT_ENERGY := 0.90
const KEY_LIGHT_ENERGY := 1.80
const MINIMUM_PRACTICAL_RANGE := 5.5
const PRESENTATION_PHASES := ["arrival", "breach", "core", "secured"]

var _core_door: Node3D
var _terminal: Node3D
var _damaged_section: Node3D
var _enrichment_root: Node3D
var _arrival_signal_ring: MeshInstance3D
var _core_rotor_outer: MeshInstance3D
var _core_rotor_inner: MeshInstance3D
var _core_orb: MeshInstance3D
var _environment_resource: Environment
var _key_light: DirectionalLight3D
var _practical_lights := {}
var _presentation_phase := "arrival"
var _phase_change_count := 0
var _animation_clock := 0.0
var _meridian_access := false
var _meridian_walls: Node3D
var _meridian_inside := false


func set_meridian_inside(inside: bool) -> void:
	if _meridian_inside == inside:
		return
	_meridian_inside = inside
	for child in get_children():
		if child is Node3D and not child is Light3D:
			child.visible = not inside
	_environment_resource.background_color = Color("243d4d") if inside else BACKGROUND_COLOR
	# Restore the threshold's individual wall visibility after returning.
	var access := _meridian_access
	_meridian_access = not access
	set_meridian_access(access)


func set_meridian_access(open: bool) -> void:
	if open == _meridian_access:
		return
	_meridian_access = open
	for wall: Node3D in get_node("BoundaryWalls").get_children():
		if wall.position.x < -12.1 and absf(wall.position.z) == 3.0:
			wall.visible = not open
	if open and _meridian_walls == null:
		_meridian_walls = Node3D.new()
		add_child(_meridian_walls)
		for z in [-4.3, 4.3]:
			var wall := WALL_PANEL_SCENE.instantiate()
			wall.position = Vector3(-12.35, 0, z)
			wall.rotation_degrees.y = 90
			wall.scale.x = 0.55
			_meridian_walls.add_child(wall)
	if _meridian_walls != null:
		_meridian_walls.visible = open


func _ready() -> void:
	_build_floor()
	_build_walls()
	_build_landmarks()
	_build_environment_story()
	_build_lighting()
	set_presentation_phase("arrival", false)


var _reduced_motion := false


func set_reduced_motion(enabled: bool) -> void:
	_reduced_motion = enabled


func _process(delta: float) -> void:
	if _reduced_motion:
		return
	_animation_clock += delta
	if _arrival_signal_ring != null:
		_arrival_signal_ring.rotation_degrees.y = fmod(_animation_clock * 18.0, 360.0)
	if _core_rotor_outer != null:
		_core_rotor_outer.rotation_degrees.x = fmod(_animation_clock * 26.0, 360.0)
	if _core_rotor_inner != null:
		_core_rotor_inner.rotation_degrees.x = fmod(-_animation_clock * 38.0, 360.0)
	if _core_orb != null:
		var pulse := 1.0 + sin(_animation_clock * 2.6) * (0.08 if _presentation_phase in ["core", "secured"] else 0.035)
		_core_orb.scale = Vector3.ONE * pulse


func set_prism_encounter(active: bool) -> void:
	# Clear the playable square during this encounter; restore the core dressing
	# on retreat, victory or retry. These are presentation props, not collision.
	_core_door.visible = not active
	for part in ["CorePylonNorth", "CorePylonSouth", "CoreCrossbeam", "CoreStatusRail", "ReactorPylonNorth", "ReactorPylonSouth", "ReactorHeadNorth", "ReactorHeadSouth", "CoreSpine", "CoreOrb", "CoreRotorOuter", "CoreRotorInner"]:
		var node := find_child(part, true, false) as Node3D
		if node != null: node.visible = not active


func set_core_door_open(open: bool, animated := true) -> void:
	_core_door.call("set_open", open, animated and not _reduced_motion)


func get_core_door() -> Node3D:
	return _core_door


func set_presentation_phase(phase: String, animated := true) -> bool:
	if phase not in PRESENTATION_PHASES:
		return false
	if phase != _presentation_phase:
		_phase_change_count += 1
	_presentation_phase = phase
	var profile := _lighting_profile(phase)
	_apply_light_energy(_key_light, float(profile.key), animated)
	_apply_light_energy(_practical_lights.get("cyan_left"), float(profile.cyan_left), animated)
	_apply_light_energy(_practical_lights.get("cyan_right"), float(profile.cyan_right), animated)
	_apply_light_energy(_practical_lights.get("amber"), float(profile.amber), animated)
	_apply_light_energy(_practical_lights.get("magenta"), float(profile.magenta), animated)
	if _environment_resource != null:
		if animated:
			create_tween().tween_property(_environment_resource, "ambient_light_energy", float(profile.ambient), 0.55)
		else:
			_environment_resource.ambient_light_energy = float(profile.ambient)
	return true


func presentation_state() -> Dictionary:
	var materials := {}
	_collect_materials(self, materials)
	return {
		"visual_bounds": VISUAL_BOUNDS,
		"door_position": _core_door.position,
		"door": _core_door.call("presentation_state"),
		"terminal_present": _terminal != null,
		"terminal_interactive": false,
		"damaged_section_present": _damaged_section != null,
		"enrichment_group_count": _enrichment_root.get_child_count(),
		"enrichment_group_names": ENRICHMENT_GROUP_NAMES.duplicate(),
		"enrichment_mesh_count": _count_nodes(_enrichment_root, "MeshInstance3D"),
		"presentation_phase": _presentation_phase,
		"phase_change_count": _phase_change_count,
		"core_rotor_count": 2,
		"animated_landmark_count": 4,
		"arrival_position": ARRIVAL_POSITION,
		"cooperation_anchor_position": COOPERATION_ANCHOR_POSITION,
		"cooperation_console_position": COOPERATION_CONSOLE_POSITION,
		"mesh_count": _count_nodes(self, "MeshInstance3D"),
		"material_count": materials.size(),
		"light_count": _count_lights(self),
		"shadow_light_count": _count_shadow_lights(self),
		"ambient_light_energy": AMBIENT_LIGHT_ENERGY,
		"ambient_light_luminance": AMBIENT_LIGHT_COLOR.get_luminance(),
		"graphite_luminance": GRAPHITE_MATERIAL.albedo_color.get_luminance(),
		"minimum_practical_range": MINIMUM_PRACTICAL_RANGE,
	}


func _build_floor() -> void:
	var floor_root := Node3D.new()
	floor_root.name = "ModularFloor"
	add_child(floor_root)
	for x in [-9.0, -3.0, 3.0, 9.0]:
		for z in [-9.0, -3.0, 3.0, 9.0]:
			var panel := FLOOR_PANEL_SCENE.instantiate()
			panel.position = Vector3(x, 0.0, z)
			floor_root.add_child(panel)


func _build_walls() -> void:
	var walls := Node3D.new()
	walls.name = "BoundaryWalls"
	add_child(walls)
	for offset in [-9.0, -3.0, 3.0, 9.0]:
		_add_wall(walls, Vector3(offset, 0.0, -12.35), 0.0)
		_add_wall(walls, Vector3(offset, 0.0, 12.35), 0.0)
		_add_wall(walls, Vector3(-12.35, 0.0, offset), 90.0)
		_add_wall(walls, Vector3(12.35, 0.0, offset), 90.0)
	for corner in [Vector3(-12.0, 0.0, -12.0), Vector3(-12.0, 0.0, 12.0), Vector3(12.0, 0.0, -12.0), Vector3(12.0, 0.0, 12.0)]:
		var column := COLUMN_SCENE.instantiate()
		column.position = corner
		walls.add_child(column)


func _add_wall(parent: Node3D, position: Vector3, yaw: float) -> void:
	var wall := WALL_PANEL_SCENE.instantiate()
	wall.position = position
	wall.rotation_degrees.y = yaw
	parent.add_child(wall)


func _build_landmarks() -> void:
	_core_door = DOOR_SCENE.instantiate()
	_core_door.position = DOOR_POSITION
	add_child(_core_door)
	_terminal = TERMINAL_SCENE.instantiate()
	_terminal.name = "RelayTerminal"
	_terminal.position = Vector3(-7.5, 0.0, -7.5)
	_terminal.rotation_degrees.y = -35.0
	_terminal.scale = Vector3.ONE * 1.25
	add_child(_terminal)
	_damaged_section = DAMAGED_SECTION_SCENE.instantiate()
	_damaged_section.name = "DamagedSection"
	_damaged_section.position = Vector3(-7.0, 0.0, 7.5)
	_damaged_section.rotation_degrees.y = 18.0
	add_child(_damaged_section)


func _build_environment_story() -> void:
	_enrichment_root = Node3D.new()
	_enrichment_root.name = "RelayHubEnrichment"
	add_child(_enrichment_root)
	_build_arrival_platform()
	_build_cooperation_lane()
	_build_breach_threshold()
	_build_relay_infrastructure()
	_build_relay_core_chamber()
	_build_corruption_trace()


func _build_arrival_platform() -> void:
	var root := _add_story_group(ENRICHMENT_GROUP_NAMES[0])
	_arrival_signal_ring = _add_cylinder(root, "ArrivalSignalRing", ARRIVAL_POSITION + Vector3(0.0, 0.015, 0.0), 2.25, 0.06, INTACT_CYAN_MATERIAL)
	_add_cylinder(root, "ArrivalOuterPlate", ARRIVAL_POSITION + Vector3(0.0, 0.055, 0.0), 2.02, 0.09, BLUE_PETROL_MATERIAL)
	_add_cylinder(root, "ArrivalInnerPlate", ARRIVAL_POSITION + Vector3(0.0, 0.11, 0.0), 1.30, 0.11, GRAPHITE_MATERIAL)


func _build_cooperation_lane() -> void:
	var root := _add_story_group(ENRICHMENT_GROUP_NAMES[1])
	_add_floor_rail(root, "ArrivalToAnchor", Vector3(1.15, 0.08, 1.05), COOPERATION_ANCHOR_POSITION, 0.16, INTACT_CYAN_MATERIAL)
	_add_floor_rail(root, "AnchorToConsole", COOPERATION_ANCHOR_POSITION, COOPERATION_CONSOLE_POSITION, 0.20, OBJECTIVE_AMBER_MATERIAL)
	_add_floor_rail(root, "ConsoleToCore", COOPERATION_CONSOLE_POSITION, Vector3(5.9, 0.08, 0.45), 0.16, OBJECTIVE_AMBER_MATERIAL)

	_add_cylinder(root, "AnchorBase", COOPERATION_ANCHOR_POSITION + Vector3(0.0, 0.08, 0.0), 0.62, 0.16, BLUE_PETROL_MATERIAL)
	_add_cylinder(root, "AnchorCrown", COOPERATION_ANCHOR_POSITION + Vector3(0.0, 0.17, 0.0), 0.44, 0.07, INTACT_CYAN_MATERIAL)
	_add_cylinder(root, "AnchorBeacon", COOPERATION_ANCHOR_POSITION + Vector3(0.0, 0.78, 0.0), 0.055, 1.20, INTACT_CYAN_MATERIAL)

	var console := TERMINAL_SCENE.instantiate()
	console.name = "CooperationConsoleLandmark"
	console.position = COOPERATION_CONSOLE_POSITION
	console.rotation_degrees.y = 180.0
	console.scale = Vector3.ONE * 0.72
	root.add_child(console)


func _build_breach_threshold() -> void:
	var root := _add_story_group(ENRICHMENT_GROUP_NAMES[2])
	for marker in [4.55, 5.10, 5.65]:
		_add_box(root, "ApproachMarker%s" % str(marker).replace(".", ""), Vector3(marker, 0.075, 0.0), Vector3(0.28, 0.05, 2.25), OBJECTIVE_AMBER_MATERIAL, Vector3(0.0, 0.0, -14.0))
	_add_box(root, "ThresholdPadNorth", Vector3(6.05, 0.10, -2.05), Vector3(1.25, 0.12, 1.20), BLUE_PETROL_MATERIAL)
	_add_box(root, "ThresholdPadSouth", Vector3(6.05, 0.10, 2.05), Vector3(1.25, 0.12, 1.20), BLUE_PETROL_MATERIAL)


func _build_relay_infrastructure() -> void:
	var root := _add_story_group(ENRICHMENT_GROUP_NAMES[3])
	_add_box(root, "CorePylonNorth", Vector3(5.75, 1.45, -2.65), Vector3(0.52, 2.9, 0.62), BLUE_PETROL_MATERIAL)
	_add_box(root, "CorePylonSouth", Vector3(5.75, 1.45, 2.65), Vector3(0.52, 2.9, 0.62), BLUE_PETROL_MATERIAL)
	_add_box(root, "CoreCrossbeam", Vector3(5.75, 2.75, 0.0), Vector3(0.58, 0.42, 5.55), GRAPHITE_MATERIAL)
	_add_box(root, "CoreStatusRail", Vector3(5.43, 2.75, 0.0), Vector3(0.08, 0.14, 4.75), OBJECTIVE_AMBER_MATERIAL)

	_add_service_bank(root, "ServiceBankWest", Vector3(-8.8, 0.0, -10.8))
	_add_service_bank(root, "ServiceBankCenter", Vector3(-5.8, 0.0, -10.8))
	_add_service_bank(root, "ServiceBankEast", Vector3(-2.8, 0.0, -10.8))


func _build_relay_core_chamber() -> void:
	var root := _add_story_group(ENRICHMENT_GROUP_NAMES[4])
	var core_center := Vector3(9.55, 0.0, 0.0)
	_add_cylinder(root, "CoreDaisOuter", core_center + Vector3(0.0, 0.08, 0.0), 2.20, 0.16, BLUE_PETROL_MATERIAL)
	_add_cylinder(root, "CoreDaisInner", core_center + Vector3(0.0, 0.17, 0.0), 1.48, 0.10, GRAPHITE_MATERIAL)
	for side in [-1.0, 1.0]:
		var suffix := "North" if side < 0.0 else "South"
		_add_box(root, "ReactorPylon%s" % suffix, core_center + Vector3(0.75, 1.20, side * 2.20), Vector3(0.58, 2.40, 0.58), DAMAGED_METAL_MATERIAL)
		_add_box(root, "ReactorHead%s" % suffix, core_center + Vector3(0.62, 2.32, side * 2.20), Vector3(0.70, 0.18, 0.70), OBJECTIVE_AMBER_MATERIAL)
	_add_cylinder(root, "CoreSpine", core_center + Vector3(1.18, 1.45, 0.0), 0.18, 2.90, DAMAGED_METAL_MATERIAL)
	_core_orb = _add_sphere(root, "CoreOrb", core_center + Vector3(1.18, 1.60, 0.0), 0.42, INTACT_CYAN_MATERIAL)
	_core_rotor_outer = _add_torus(root, "CoreRotorOuter", core_center + Vector3(1.18, 1.60, 0.0), 1.00, 0.055, OBJECTIVE_AMBER_MATERIAL, Vector3(0.0, 0.0, 90.0))
	_core_rotor_inner = _add_torus(root, "CoreRotorInner", core_center + Vector3(1.18, 1.60, 0.0), 0.68, 0.045, INTACT_CYAN_MATERIAL, Vector3(90.0, 0.0, 0.0))


func _build_corruption_trace() -> void:
	var root := _add_story_group(ENRICHMENT_GROUP_NAMES[5])
	var origin := Vector3(-7.0, 0.09, 7.5)
	_add_floor_rail(root, "CorruptionVeinOne", origin, Vector3(-9.3, 0.09, 8.8), 0.12, THREAT_MAGENTA_MATERIAL)
	_add_floor_rail(root, "CorruptionVeinTwo", origin + Vector3(0.3, 0.0, -0.2), Vector3(-5.0, 0.09, 9.8), 0.09, THREAT_MAGENTA_MATERIAL)
	_add_floor_rail(root, "CorruptionVeinThree", origin + Vector3(-0.2, 0.0, 0.15), Vector3(-8.5, 0.09, 5.4), 0.08, THREAT_MAGENTA_MATERIAL)
	_add_box(root, "DebrisOne", Vector3(-8.35, 0.20, 6.25), Vector3(0.85, 0.28, 0.48), DAMAGED_METAL_MATERIAL, Vector3(12.0, 28.0, -8.0))
	_add_box(root, "DebrisTwo", Vector3(-5.55, 0.17, 8.85), Vector3(0.55, 0.22, 0.92), DAMAGED_METAL_MATERIAL, Vector3(-7.0, -24.0, 10.0))
	_add_box(root, "DebrisThree", Vector3(-7.9, 0.14, 9.55), Vector3(0.45, 0.18, 0.62), GRAPHITE_MATERIAL, Vector3(8.0, 41.0, 5.0))


func _add_story_group(group_name: String) -> Node3D:
	var group := Node3D.new()
	group.name = group_name
	_enrichment_root.add_child(group)
	return group


func _add_service_bank(parent: Node3D, bank_name: String, bank_position: Vector3) -> void:
	_add_box(parent, bank_name, bank_position + Vector3(0.0, 0.95, 0.0), Vector3(2.15, 1.9, 0.62), BLUE_PETROL_MATERIAL)
	_add_box(parent, "%sStatus" % bank_name, bank_position + Vector3(0.0, 1.16, 0.34), Vector3(1.45, 0.32, 0.05), INTACT_CYAN_MATERIAL)


func _add_floor_rail(parent: Node3D, rail_name: String, start: Vector3, finish: Vector3, width: float, material: Material) -> void:
	var delta := finish - start
	var center := (start + finish) * 0.5
	center.y = maxf(start.y, finish.y)
	_add_box(
		parent,
		rail_name,
		center,
		Vector3(width, 0.055, Vector2(delta.x, delta.z).length()),
		material,
		Vector3(0.0, rad_to_deg(atan2(delta.x, delta.z)), 0.0)
	)


func _add_box(parent: Node3D, mesh_name: String, position: Vector3, size: Vector3, material: Material, rotation := Vector3.ZERO) -> MeshInstance3D:
	var box := BoxMesh.new()
	box.size = size
	var instance := MeshInstance3D.new()
	instance.name = mesh_name
	instance.position = position
	instance.rotation_degrees = rotation
	instance.mesh = box
	instance.material_override = material
	parent.add_child(instance)
	return instance


func _add_cylinder(parent: Node3D, mesh_name: String, position: Vector3, radius: float, height: float, material: Material) -> MeshInstance3D:
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = radius
	cylinder.bottom_radius = radius
	cylinder.height = height
	cylinder.radial_segments = 24
	var instance := MeshInstance3D.new()
	instance.name = mesh_name
	instance.position = position
	instance.mesh = cylinder
	instance.material_override = material
	parent.add_child(instance)
	return instance


func _add_sphere(parent: Node3D, mesh_name: String, position: Vector3, radius: float, material: Material) -> MeshInstance3D:
	var sphere := SphereMesh.new()
	sphere.radius = radius
	sphere.height = radius * 2.0
	sphere.radial_segments = 20
	sphere.rings = 10
	var instance := MeshInstance3D.new()
	instance.name = mesh_name
	instance.position = position
	instance.mesh = sphere
	instance.material_override = material
	parent.add_child(instance)
	return instance


func _add_torus(parent: Node3D, mesh_name: String, position: Vector3, radius: float, tube_radius: float, material: Material, rotation: Vector3) -> MeshInstance3D:
	var torus := TorusMesh.new()
	torus.inner_radius = radius - tube_radius
	torus.outer_radius = radius + tube_radius
	torus.rings = 24
	torus.ring_segments = 8
	var instance := MeshInstance3D.new()
	instance.name = mesh_name
	instance.position = position
	instance.rotation_degrees = rotation
	instance.mesh = torus
	instance.material_override = material
	parent.add_child(instance)
	return instance


func _build_lighting() -> void:
	var world_environment := WorldEnvironment.new()
	world_environment.name = "RelayEnvironment"
	_environment_resource = Environment.new()
	_environment_resource.background_mode = Environment.BG_COLOR
	_environment_resource.background_color = BACKGROUND_COLOR
	_environment_resource.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	_environment_resource.ambient_light_color = AMBIENT_LIGHT_COLOR
	_environment_resource.ambient_light_energy = AMBIENT_LIGHT_ENERGY
	_environment_resource.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	_environment_resource.tonemap_exposure = 1.12
	world_environment.environment = _environment_resource
	add_child(world_environment)

	_key_light = DirectionalLight3D.new()
	_key_light.name = "IndustrialKey"
	_key_light.rotation_degrees = Vector3(-58.0, -28.0, 0.0)
	_key_light.light_color = Color("b8c8dc")
	_key_light.light_energy = KEY_LIGHT_ENERGY
	_key_light.shadow_enabled = false
	add_child(_key_light)
	_practical_lights.cyan_left = _add_practical_light("CyanServiceLeft", Vector3(-8.5, 2.4, -2.0), Color("35d0d0"), 3.8, 7.5)
	_practical_lights.cyan_right = _add_practical_light("CyanServiceRight", Vector3(9.0, 2.8, 3.6), Color("35d0d0"), 2.6, 7.5)
	_practical_lights.amber = _add_practical_light("AmberObjective", Vector3(6.2, 2.8, 0.0), Color("f5a524"), 2.5, 7.5)
	_practical_lights.magenta = _add_practical_light("MagentaDamage", Vector3(-7.0, 1.3, 7.5), Color("d93678"), 1.8, MINIMUM_PRACTICAL_RANGE)


func _add_practical_light(light_name: String, position: Vector3, color: Color, energy: float, range_value: float) -> OmniLight3D:
	var light := OmniLight3D.new()
	light.name = light_name
	light.position = position
	light.light_color = color
	light.light_energy = energy
	light.omni_range = range_value
	light.shadow_enabled = false
	add_child(light)
	return light


func _lighting_profile(phase: String) -> Dictionary:
	match phase:
		"breach":
			return {"ambient": 0.94, "key": 1.82, "cyan_left": 2.8, "cyan_right": 3.0, "amber": 4.2, "magenta": 1.6}
		"core":
			return {"ambient": 1.00, "key": 1.88, "cyan_left": 2.2, "cyan_right": 3.7, "amber": 4.0, "magenta": 2.2}
		"secured":
			return {"ambient": 1.08, "key": 1.96, "cyan_left": 3.8, "cyan_right": 4.2, "amber": 3.2, "magenta": 1.0}
		_:
			return {"ambient": AMBIENT_LIGHT_ENERGY, "key": KEY_LIGHT_ENERGY, "cyan_left": 3.8, "cyan_right": 2.6, "amber": 2.5, "magenta": 1.8}


func _apply_light_energy(light: Light3D, energy: float, animated: bool) -> void:
	if light == null:
		return
	if animated:
		create_tween().tween_property(light, "light_energy", energy, 0.55)
	else:
		light.light_energy = energy


func _count_nodes(node: Node, class_name_value: String) -> int:
	var count := 1 if node.is_class(class_name_value) else 0
	for child in node.get_children():
		count += _count_nodes(child, class_name_value)
	return count


func _collect_materials(node: Node, materials: Dictionary) -> void:
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		if mesh_instance.material_override != null:
			materials[mesh_instance.material_override.get_instance_id()] = true
	for child in node.get_children():
		_collect_materials(child, materials)


func _count_lights(node: Node) -> int:
	var count := 1 if node is Light3D else 0
	for child in node.get_children():
		count += _count_lights(child)
	return count


func _count_shadow_lights(node: Node) -> int:
	var count := 1 if node is Light3D and (node as Light3D).shadow_enabled else 0
	for child in node.get_children():
		count += _count_shadow_lights(child)
	return count
