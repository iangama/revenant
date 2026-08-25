extends CanvasLayer

const DAMAGE_RED := Color("e8505b")
const INTACT_CYAN := Color("35d0d0")
const EDGE_DEPTH := 10.0

var _edge_bars: Array[ColorRect] = []
var _confirmed_damage_cues := 0
var _completion_cues := 0


func _ready() -> void:
	layer = 2
	_build_edge_feedback()


func play_confirmed_player_damage() -> void:
	_confirmed_damage_cues += 1
	_pulse_edges(DAMAGE_RED, 0.72, 0.28)


func play_authoritative_completion() -> void:
	_completion_cues += 1
	_pulse_edges(INTACT_CYAN, 0.58, 0.55)


func scene_budget(root: Node) -> Dictionary:
	var materials := {}
	var counts := {
		"meshes": 0,
		"lights": 0,
		"shadow_lights": 0,
		"particles": 0,
		"audio_nodes": 0,
	}
	_collect_scene(root, counts, materials)
	counts["materials"] = materials.size()
	counts["edge_overlays"] = _edge_bars.size()
	return counts


func presentation_state() -> Dictionary:
	return {
		"confirmed_damage_cues": _confirmed_damage_cues,
		"completion_cues": _completion_cues,
		"edge_overlays": _edge_bars.size(),
		"maximum_screen_coverage": 0.06,
	}


func _build_edge_feedback() -> void:
	var definitions := [
		Rect2(0, 0, 1280, EDGE_DEPTH),
		Rect2(0, 710, 1280, EDGE_DEPTH),
		Rect2(0, EDGE_DEPTH, EDGE_DEPTH, 700),
		Rect2(1270, EDGE_DEPTH, EDGE_DEPTH, 700),
	]
	for index in range(definitions.size()):
		var bar := ColorRect.new()
		bar.name = "FeedbackEdge%d" % index
		bar.position = definitions[index].position
		bar.size = definitions[index].size
		bar.color = Color(DAMAGE_RED, 0.0)
		bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
		add_child(bar)
		_edge_bars.append(bar)


func _pulse_edges(color: Color, peak_alpha: float, duration: float) -> void:
	for bar in _edge_bars:
		bar.color = Color(color, peak_alpha)
		var tween := create_tween()
		tween.tween_property(bar, "color:a", 0.0, duration).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)


func _collect_scene(node: Node, counts: Dictionary, materials: Dictionary) -> void:
	if node is MeshInstance3D:
		counts["meshes"] += 1
		var mesh_instance := node as MeshInstance3D
		if mesh_instance.material_override != null:
			materials[mesh_instance.material_override.get_instance_id()] = true
	elif node is Light3D:
		counts["lights"] += 1
		if (node as Light3D).shadow_enabled:
			counts["shadow_lights"] += 1
	elif node is GPUParticles3D or node is CPUParticles3D:
		counts["particles"] += 1
	elif node is AudioStreamPlayer or node is AudioStreamPlayer2D or node is AudioStreamPlayer3D:
		counts["audio_nodes"] += 1
	for child in node.get_children():
		_collect_scene(child, counts, materials)
