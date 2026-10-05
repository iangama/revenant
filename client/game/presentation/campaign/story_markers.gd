extends Node3D

var _labels: Array[Label3D] = []


func _ready() -> void:
	for point in [Vector3(-31, 0, 8), Vector3(-18, 0, 0), Vector3(-6, 0, -3), Vector3(2, 0, 4), Vector3(7, 0, -6)]:
		var label := Label3D.new()
		label.position = point + Vector3(0, 2.5, 0)
		label.font_size = 32
		label.pixel_size = 0.012
		label.outline_size = 8
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.modulate = Color("35d0d0")
		add_child(label)
		_labels.append(label)
	visible = false


func present(snapshot: Dictionary, shown: bool) -> void:
	visible = shown
	for label in _labels: label.visible = false
	var active: Variant = snapshot.get("active")
	if not active is Dictionary: return
	var story: Dictionary = snapshot.get("story", {})
	var cp: int = active.get("checkpoint", -1)
	match active.get("chapter_id"):
		"meridian_readings":
			_labels[0].visible = cp in [1, 2] and story.get("empty_seat") == "unseen"
			_labels[0].text = tr("OPTIONAL MEMORY")
			_labels[1].visible = cp == 2 or (cp == 1 and story.get("empty_seat") == "found")
			_labels[1].text = tr("DEPARTURE BOARD")
		"broken_supply_line":
			_labels[2].visible = cp in [2, 3] and story.get("held_connection") == "unseen"
			_labels[2].text = tr("MAINTENANCE NOTE")
			_labels[3].visible = cp in [2, 3] and story.get("held_connection") == "found"
			_labels[3].text = tr("RETURN THE NOTE")
		"counter_signal":
			_labels[4].visible = cp == 5
			_labels[4].text = tr("CORE APPROACH")
