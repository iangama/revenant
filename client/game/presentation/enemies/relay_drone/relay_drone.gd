extends "res://presentation/enemies/enemy_presentation.gd"

var _chassis: MeshInstance3D
var _arms: Array[MeshInstance3D] = []
var _signal_pods: Array[MeshInstance3D] = []


func family_name() -> String:
	return "relay_drone"


func _process(delta: float) -> void:
	super(delta)
	if _chassis == null:
		return
	_chassis.rotation_degrees.y = fmod(_idle_clock * 34.0, 360.0)
	for index in _arms.size():
		_arms[index].rotation_degrees.y = sin(_idle_clock * 2.2 + float(index) * 1.57) * 7.0
	for index in _signal_pods.size():
		_signal_pods[index].position.y = 0.71 + sin(_idle_clock * 3.0 + float(index) * 1.57) * 0.075
		_signal_pods[index].rotation_degrees.y = fmod(_idle_clock * 48.0 + float(index) * 90.0, 360.0)


func play_confirmed_attack() -> void:
	super()
	for pod in _signal_pods:
		var pulse := create_tween()
		pulse.tween_property(pod, "scale", Vector3.ONE * 1.55, 0.08)
		pulse.tween_property(pod, "scale", Vector3.ONE, 0.16)


func play_confirmed_hit() -> void:
	super()
	if _chassis != null:
		var recoil := create_tween()
		recoil.tween_property(_chassis, "scale", Vector3(1.18, 0.72, 1.18), 0.06)
		recoil.tween_property(_chassis, "scale", Vector3.ONE, 0.13)


func _build_body() -> void:
	_visual_root.scale = Vector3.ONE * 1.18
	var armor := _material(Color("3a6072"), 0.7, 0.38)
	var corruption := _emissive_material(Color("d93678"), 2.35)
	_set_core_material(corruption)
	_chassis = _add_cylinder("RadialChassis", 0.56, 0.32, Vector3(0.0, 0.72, 0.0), armor)
	_add_sphere("CorruptedCore", 0.30, Vector3(0.0, 0.73, -0.24), corruption)
	for arm_data in [
		["ArmNorth", Vector3(0.0, 0.65, -0.74), Vector3(-22.0, 0.0, 0.0)],
		["ArmSouth", Vector3(0.0, 0.65, 0.74), Vector3(22.0, 0.0, 0.0)],
		["ArmWest", Vector3(-0.74, 0.65, 0.0), Vector3(0.0, 0.0, -22.0)],
		["ArmEast", Vector3(0.74, 0.65, 0.0), Vector3(0.0, 0.0, 22.0)],
	]:
		_arms.append(_add_box(arm_data[0], Vector3(0.82, 0.18, 0.20), arm_data[1], armor, arm_data[2]))
	for pod_data in [
		["SignalPodNorth", Vector3(0.0, 0.71, -1.12)],
		["SignalPodSouth", Vector3(0.0, 0.71, 1.12)],
		["SignalPodWest", Vector3(-1.12, 0.71, 0.0)],
		["SignalPodEast", Vector3(1.12, 0.71, 0.0)],
	]:
		_signal_pods.append(_add_sphere(pod_data[0], 0.12, pod_data[1], corruption))
