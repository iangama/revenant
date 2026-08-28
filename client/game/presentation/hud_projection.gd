extends RefCounted


func objective_text(objective: Dictionary) -> String:
	var label := str(objective.get("objective_id", "unknown")).replace("_", " ").to_upper()
	return "OBJECTIVE  •  %s  [%d/%d]  %s" % [label, objective.get("progress", 0), objective.get("target", 0), objective.get("state", "")]


func inventory_text(inventory: Dictionary) -> String:
	var lines: Array[String] = ["INVENTORY"]
	var item_ids := inventory.keys()
	item_ids.sort()
	for item_id in item_ids:
		lines.append("%s  x%d" % [str(item_id).replace("_", " ").to_upper(), inventory[item_id]])
	return "\n".join(lines)


func progression_text(progression: Dictionary) -> String:
	return "PROGRESSION  •  LEVEL %d\nXP %d  •  %d TO NEXT LEVEL" % [progression.get("level", 1), progression.get("experience", 0), progression.get("experience_to_next_level", 500)]


func equipment_text(item_id: String, profile: Dictionary) -> String:
	return "WEAPON  •  %s\nDMG %d  RANGE %d  COOLDOWN %d MS" % [item_id.replace("_", " ").to_upper(), profile.get("damage", 0), profile.get("range", 0), profile.get("cooldown_ms", 0)]
