extends Node3D

const FLOOR_PANEL_SCENE := preload("res://presentation/environment/modules/floor_panel.tscn")
const WALL_PANEL_SCENE := preload("res://presentation/environment/modules/wall_panel.tscn")
const COLUMN_SCENE := preload("res://presentation/environment/modules/structural_column.tscn")
const DOOR_SCENE := preload("res://presentation/environment/modules/relay_core_door.tscn")
const TERMINAL_SCENE := preload("res://presentation/environment/modules/relay_terminal.tscn")
const DAMAGED_SECTION_SCENE := preload("res://presentation/environment/modules/damaged_section.tscn")

const VISUAL_BOUNDS := 12.0
const DOOR_POSITION := Vector3(6.5, 0.0, 0.0)

var _core_door: Node3D
var _terminal: Node3D
var _damaged_section: Node3D


func _ready() -> void:
	_build_floor()
	_build_walls()
	_build_landmarks()
	_build_lighting()


func set_core_door_open(open: bool, animated := true) -> void:
	_core_door.call("set_open", open, animated)


func get_core_door() -> Node3D:
	return _core_door


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
		"mesh_count": _count_nodes(self, "MeshInstance3D"),
		"material_count": materials.size(),
		"light_count": _count_lights(self),
		"shadow_light_count": _count_shadow_lights(self),
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


func _build_lighting() -> void:
	var world_environment := WorldEnvironment.new()
	world_environment.name = "RelayEnvironment"
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color("070b11")
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color("12313a")
	environment.ambient_light_energy = 0.62
	environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	world_environment.environment = environment
	add_child(world_environment)

	var key_light := DirectionalLight3D.new()
	key_light.name = "IndustrialKey"
	key_light.rotation_degrees = Vector3(-58.0, -28.0, 0.0)
	key_light.light_color = Color("a9b8cc")
	key_light.light_energy = 1.35
	key_light.shadow_enabled = false
	add_child(key_light)
	_add_practical_light("CyanServiceLeft", Vector3(-8.5, 2.4, -2.0), Color("35d0d0"), 2.2, 5.5)
	_add_practical_light("CyanServiceRight", Vector3(8.5, 2.4, 6.0), Color("35d0d0"), 1.8, 5.0)
	_add_practical_light("AmberObjective", Vector3(5.2, 2.6, 0.0), Color("f5a524"), 2.8, 6.0)
	_add_practical_light("MagentaDamage", Vector3(-7.0, 1.3, 7.5), Color("d93678"), 1.35, 3.5)


func _add_practical_light(light_name: String, position: Vector3, color: Color, energy: float, range_value: float) -> void:
	var light := OmniLight3D.new()
	light.name = light_name
	light.position = position
	light.light_color = color
	light.light_energy = energy
	light.omni_range = range_value
	light.shadow_enabled = false
	add_child(light)


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
