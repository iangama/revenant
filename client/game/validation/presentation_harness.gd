extends RefCounted

const OPERATOR_SCENE := preload("res://presentation/operator/operator.tscn")
const RELAY_DRONE_SCENE := preload("res://presentation/enemies/relay_drone/relay_drone.tscn")
const WARDEN_SCENE := preload("res://presentation/enemies/warden/warden.tscn")
const M21_CAPTURE_FILENAMES := [
	"01-relay-hub-overview.png",
	"02-enemy-telegraphs.png",
	"03-combat-feedback.png",
]


func validate_foundation(fixtures: Dictionary) -> String:
	var required_actions := ["move_forward", "move_back", "move_left", "move_right", "attack"]
	for action in required_actions:
		if not InputMap.has_action(action):
			return "M17 input action is missing: %s" % action
	if (
		fixtures.camera == null
		or fixtures.door == null
		or fixtures.status_label == null
		or fixtures.health_label == null
		or fixtures.enemy_health_label == null
		or fixtures.position_label == null
		or fixtures.objective_label == null
		or fixtures.crosshair == null
		or fixtures.guidance_label == null
		or fixtures.input_log_label == null
		or fixtures.inventory_label == null
		or fixtures.progression_label == null
		or fixtures.equipment_label == null
		or fixtures.hud_frame == null
		or fixtures.presentation_polish == null
		or fixtures.loot_pickup == null
		or fixtures.sector_transition == null
		or fixtures.player_health_bar == null
		or fixtures.enemy_health_bar == null
		or fixtures.movement_buttons.size() != 4
		or fixtures.attack_button == null
		or fixtures.weapon_buttons.size() != 5
		or fixtures.cooperation_button == null
		or fixtures.cooperation_button.focus_mode != Control.FOCUS_ALL
		or fixtures.crosshair.mouse_filter != Control.MOUSE_FILTER_IGNORE
	):
		return "M17 playable scene composition is incomplete"
	fixtures.presentation_polish.call("play_confirmed_player_damage")
	fixtures.presentation_polish.call("play_authoritative_completion")
	var polish_state: Dictionary = fixtures.presentation_polish.call("presentation_state")
	if (
		polish_state.get("confirmed_damage_cues", 0) != 1
		or polish_state.get("completion_cues", 0) != 1
		or polish_state.get("edge_overlays", 0) != 4
		or polish_state.get("maximum_screen_coverage", 1.0) > 0.06
	):
		return "M21 polished slice exceeds its visual consistency or performance budget"
	var hud_state: Dictionary = fixtures.hud_frame.call("presentation_state")
	if (
		hud_state.get("panel_count", 0) != 4
		or hud_state.get("semantic_accent_count", 0) < 4
		or not hud_state.get("mouse_passthrough", false)
		or fixtures.player_health_bar.value != 100.0
		or fixtures.enemy_health_bar.value != 100.0
	):
		return "M21 Operator HUD does not preserve its hierarchy, semantic roles or authoritative health state"
	var pickup_state: Dictionary = fixtures.loot_pickup.call("presentation_state")
	var sector_state: Dictionary = fixtures.sector_transition.call("presentation_state")
	if (
		pickup_state.get("maximum_active_pickups", 0) != 1
		or pickup_state.get("maximum_meshes_per_pickup", 0) != 3
		or pickup_state.get("permanent_particles", 1) != 0
		or not pickup_state.get("server_confirmed_only", false)
		or not pickup_state.get("world_label_present", false)
		or float(pickup_state.get("presentation_duration_seconds", 0.0)) < 2.0
		or not Rect2(0, 0, 1280, 720).encloses(sector_state.get("bounds", Rect2()))
		or sector_state.get("bounds", Rect2()).size.x > 560
		or sector_state.get("bounds", Rect2()).size.y > 88
		or not sector_state.get("mouse_passthrough", false)
	):
		return "Relay Hub reward or sector presentation exceeds its bounded contract"
	return ""


func validate_scene(fixtures: Dictionary) -> Dictionary:
	var root: Node = fixtures.root
	var tree: SceneTree = fixtures.tree
	var environment: Node3D = fixtures.environment
	var presentation_polish: CanvasLayer = fixtures.presentation_polish
	var combat_vfx: Node3D = fixtures.combat_vfx
	var loot_pickup: Node3D = fixtures.loot_pickup
	var sector_transition: CanvasLayer = fixtures.sector_transition
	var save_capture: Callable = fixtures.save_capture
	var environment_state: Dictionary = environment.call("presentation_state")
	var closed_door_state: Dictionary = environment_state.get("door", {})
	if (
		environment_state.get("visual_bounds") != 12.0
		or environment_state.get("door_position") != Vector3(6.5, 0.0, 0.0)
		or not environment_state.get("terminal_present", false)
		or environment_state.get("terminal_interactive", true)
		or not environment_state.get("damaged_section_present", false)
		or environment_state.get("enrichment_group_count", 0) != 6
		or environment_state.get("enrichment_group_names", []) != ["ArrivalPlatform", "CooperationLane", "BreachThreshold", "RelayInfrastructure", "RelayCoreChamber", "CorruptionTrace"]
		or environment_state.get("enrichment_mesh_count", 0) > 44
		or environment_state.get("presentation_phase") != "arrival"
		or environment_state.get("core_rotor_count", 0) != 2
		or environment_state.get("animated_landmark_count", 0) < 4
		or environment_state.get("arrival_position") != Vector3.ZERO
		or environment_state.get("cooperation_anchor_position") != Vector3(3.0, 0.0, 3.0)
		or environment_state.get("cooperation_console_position") != Vector3(4.0, 0.0, 3.0)
		or environment_state.get("mesh_count", 0) > 103
		or environment_state.get("material_count", 0) > 6
		or environment_state.get("light_count", 0) != 5
		or environment_state.get("shadow_light_count", 1) != 0
		or float(environment_state.get("ambient_light_energy", 0.0)) < 0.88
		or float(environment_state.get("ambient_light_luminance", 0.0)) < 0.55
		or float(environment_state.get("graphite_luminance", 0.0)) < 0.20
		or float(environment_state.get("minimum_practical_range", 0.0)) < 5.5
		or closed_door_state.get("open", true)
	):
		return {"error": "M21 relay-hub modular environment exceeds its composition, visibility or performance contract"}
	environment.call("set_core_door_open", true, false)
	var open_door_state: Dictionary = environment.call("presentation_state").get("door", {})
	if not open_door_state.get("open", false) or open_door_state.get("amber_visible", true):
		return {"error": "M21 relay-core door does not expose its authoritative open state"}
	environment.call("set_core_door_open", false, false)
	for phase in ["breach", "core", "secured", "arrival"]:
		if not environment.call("set_presentation_phase", phase, false):
			return {"error": "Relay Hub did not accept its bounded presentation phase: %s" % phase}
	if environment.call("set_presentation_phase", "untrusted", false):
		return {"error": "Relay Hub accepted an unbounded presentation phase"}
	var restored_environment_state: Dictionary = environment.call("presentation_state")
	if restored_environment_state.get("presentation_phase") != "arrival" or restored_environment_state.get("phase_change_count", 0) != 4:
		return {"error": "Relay Hub phase presentation is not deterministic or restorable"}
	var operator: Node3D = OPERATOR_SCENE.instantiate()
	operator.name = "OperatorValidation"
	root.add_child(operator)
	await tree.process_frame
	var operator_state: Dictionary = operator.call("presentation_state")
	if (
		operator_state.get("equipped_weapon") != "pulse_rifle"
		or operator_state.get("last_animation") != "idle"
		or not operator_state.get("pulse_rifle_visible", false)
		or operator_state.get("arc_sidearm_visible", true)
		or operator_state.get("part_count", 0) < 15
	):
		return {"error": "M21 Operator initial presentation is incomplete"}
	operator.call("set_weapon", "arc_sidearm")
	operator_state = operator.call("presentation_state")
	if operator_state.get("equipped_weapon") != "arc_sidearm" or not operator_state.get("arc_sidearm_visible", false):
		return {"error": "M21 Operator weapon silhouettes do not follow authoritative equipment"}
	operator.call("set_authoritative_life", "downed")
	operator_state = operator.call("presentation_state")
	if operator_state.get("authoritative_life") != "downed" or operator_state.get("last_animation") != "downed":
		return {"error": "M28 Operator does not expose the authoritative downed state"}
	operator.call("set_authoritative_life", "active")
	operator_state = operator.call("presentation_state")
	if operator_state.get("authoritative_life") != "active" or operator_state.get("last_animation") != "revived":
		return {"error": "M28 Operator does not expose the authoritative revive state"}
	var drone: Node3D = RELAY_DRONE_SCENE.instantiate()
	drone.name = "RelayDroneValidation"
	drone.position = Vector3(-2.0, 0.0, 2.5)
	root.add_child(drone)
	var warden: Node3D = WARDEN_SCENE.instantiate()
	warden.name = "WardenValidation"
	warden.position = Vector3(3.0, 0.0, 2.5)
	root.add_child(warden)
	await tree.process_frame
	var drone_state: Dictionary = drone.call("presentation_state")
	var warden_state: Dictionary = warden.call("presentation_state")
	if (
		drone_state.get("family") != "relay_drone"
		or drone_state.get("mesh_count", 0) > 12
		or drone_state.get("material_count", 0) > 3
		or warden_state.get("family") != "warden"
		or warden_state.get("mesh_count", 0) > 18
		or warden_state.get("material_count", 0) > 4
		or warden_state.get("mesh_count", 0) <= drone_state.get("mesh_count", 0)
	):
		return {"error": "M21 enemy families are not distinct or exceed their presentation budgets"}
	var capture_directory := OS.get_environment("REVENANT_CAPTURE_M21_DIR")
	if not capture_directory.is_empty() and await save_capture.call(capture_directory.path_join(M21_CAPTURE_FILENAMES[0])) != OK:
		return {"error": "M21 overview capture could not be saved"}
	drone.call("set_targeted", true)
	drone.call("set_danger_close", false)
	drone_state = drone.call("presentation_state")
	if not drone_state.get("targeted", false) or drone_state.get("danger_close", true):
		return {"error": "M21 enemy target and danger telegraphs are not independent"}
	drone.call("set_danger_close", true)
	if not capture_directory.is_empty() and await save_capture.call(capture_directory.path_join(M21_CAPTURE_FILENAMES[1])) != OK:
		return {"error": "M21 telegraph capture could not be saved"}
	operator.call("play_authoritative_move", Vector3(-1.0, 0.0, 0.0))
	operator.call("play_confirmed_attack")
	operator.call("play_confirmed_hit")
	operator.call("play_defeat")
	operator_state = operator.call("presentation_state")
	if operator_state.get("last_animation") != "defeat" or not operator_state.get("defeated", false):
		return {"error": "M21 Operator authoritative animation states are incomplete"}
	drone.call("play_authoritative_move", Vector3(-1.0, 0.0, -1.0))
	drone.call("play_confirmed_attack")
	drone.call("play_confirmed_hit")
	drone.call("retire")
	warden.call("play_confirmed_attack")
	warden.call("play_confirmed_hit")
	warden.call("retire")
	var vfx_before: Dictionary = combat_vfx.call("presentation_state")
	combat_vfx.call("play_local_cooldown", operator.global_position, 260)
	combat_vfx.call("play_confirmed_exchange", operator, drone, false)
	combat_vfx.call("play_confirmed_exchange", warden, operator, true)
	var vfx_state: Dictionary = combat_vfx.call("presentation_state")
	if (
		vfx_state.get("confirmed_exchanges", 0) - vfx_before.get("confirmed_exchanges", 0) != 2
		or vfx_state.get("cooldown_cues", 0) - vfx_before.get("cooldown_cues", 0) != 1
		or vfx_state.get("active_effects", 0) <= 0
		or vfx_state.get("active_effects", 0) > vfx_state.get("maximum_active_effects", 0)
		or vfx_state.get("permanent_particles", 1) != 0
	):
		return {"error": "M21 combat VFX does not preserve its bounded authoritative feedback contract"}
	drone_state = drone.call("presentation_state")
	warden_state = warden.call("presentation_state")
	if not drone_state.get("retired", false) or not warden_state.get("retired", false):
		return {"error": "M21 enemy defeat does not retire confirmed targets"}
	var scene_budget: Dictionary = presentation_polish.call("scene_budget", root)
	if (
		# M31 keeps inactive weapons/excursion geometry resident. Rendering keeps
		# the original 170-mesh cap; resident authored geometry has a 220 cap.
		scene_budget.get("meshes", 0) > 220
		or scene_budget.get("visible_meshes", 0) > 170
		or scene_budget.get("materials", 0) > 32
		or scene_budget.get("lights", 0) != 5
		or scene_budget.get("shadow_lights", 1) != 0
		or scene_budget.get("particles", 1) != 0
		or scene_budget.get("audio_nodes", 0) != 14
	):
		return {"error": "Representative combat peak exceeds its whole-scene performance budget: %s" % scene_budget}
	print("M21 scene budget measured: %d meshes, %d materials, %d lights, %d particles, %d audio nodes" % [
		scene_budget.get("meshes", 0),
		scene_budget.get("materials", 0),
		scene_budget.get("lights", 0),
		scene_budget.get("particles", 0),
		scene_budget.get("audio_nodes", 0),
	])
	if not capture_directory.is_empty() and await save_capture.call(capture_directory.path_join(M21_CAPTURE_FILENAMES[2])) != OK:
		return {"error": "M21 combat capture could not be saved"}
	var runtime_capture_path := OS.get_environment("REVENANT_CAPTURE_M22_RUNTIME")
	if not runtime_capture_path.is_empty():
		fixtures.status_label.text = "ENGAGED  •  CONFIRMED COMBAT FEEDBACK"
		fixtures.objective_label.text = "OBJECTIVE  •  DEFEAT THE WARDEN"
		fixtures.enemy_health_label.text = "ENEMY  WARDEN  •  020 / 120 HP"
		fixtures.enemy_health_bar.max_value = 120
		fixtures.enemy_health_bar.value = 20
		fixtures.onboarding.call("confirm", "warden_spawn")
		fixtures.refresh_onboarding.call()
		if await save_capture.call(runtime_capture_path) != OK:
			return {"error": "M22 runtime validation capture could not be saved"}
	var capture_path := OS.get_environment("REVENANT_CAPTURE_SLICE")
	if not capture_path.is_empty():
		await tree.process_frame
		if await save_capture.call(capture_path) != OK:
			return {"error": "M21 visual validation capture could not be saved"}
	if not loot_pickup.call("play_confirmed_pickup", "relay_core_fragment", 1, Vector3(5.0, 0.0, 0.0), Vector3.ZERO):
		return {"error": "confirmed Relay Core Fragment presentation did not start"}
	if loot_pickup.call("play_confirmed_pickup", "relay_core_fragment", 1, Vector3(5.0, 0.0, 0.0), Vector3.ZERO):
		return {"error": "confirmed reward presentation allowed overlapping duplicate pickup"}
	var pickup_state: Dictionary = loot_pickup.call("presentation_state")
	if (
		pickup_state.get("active_pickups", 0) != 1
		or pickup_state.get("confirmed_pickups", 0) != 1
		or pickup_state.get("last_item_id") != "relay_core_fragment"
		or pickup_state.get("last_quantity", 0) != 1
	):
		return {"error": "confirmed reward presentation diverged from accepted LootGranted data"}
	sector_transition.call("set_reduced_motion", true)
	if not sector_transition.call("present", "SECTOR 02  •  RELAY HUB", "RELAY CORE", Color("f5a524")):
		return {"error": "authoritative Relay Core sector transition did not render"}
	var sector_state: Dictionary = sector_transition.call("presentation_state")
	if (
		sector_state.get("transition_count", 0) != 1
		or sector_state.get("last_sector") != "SECTOR 02  •  RELAY HUB"
		or sector_state.get("last_title") != "RELAY CORE"
		or not sector_state.get("active", false)
		or not sector_state.get("reduced_motion", false)
	):
		return {"error": "Relay Core transition lost its bounded non-color or Reduced Flash state"}
	return {"error": "", "operator": operator, "drone": drone, "warden": warden}
