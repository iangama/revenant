extends RefCounted

var player_actor_id := 0
var actors := {}
var actor_health := {}
var actor_max_health := {}
var inventory := {}
var progression := {"level": 1, "experience": 0, "experience_to_next_level": 500}
var equipped_weapon_item_id := "pulse_rifle"
var weapon_profiles := {}
var objectives := {}
var activity_complete := false


func join_world(message: Dictionary) -> void:
	player_actor_id = message.get("player_actor_id", 0)


func apply_actor_spawn(message: Dictionary) -> void:
	var actor_id: int = message.get("actor_id", 0)
	actors[actor_id] = message.duplicate(true)
	actor_health[actor_id] = message.get("health", 100)
	actor_max_health[actor_id] = message.get("max_health", 100)


func apply_actor_update(message: Dictionary) -> void:
	var actor_id: int = message.get("actor_id", 0)
	if not actors.has(actor_id):
		return
	var actor: Dictionary = actors[actor_id]
	actor["position"] = message.get("position", actor.get("position", [0, 0, 0]))


func apply_actor_destroy(message: Dictionary) -> void:
	var actor_id: int = message.get("actor_id", 0)
	actors.erase(actor_id)
	actor_health.erase(actor_id)
	actor_max_health.erase(actor_id)


func apply_damage(message: Dictionary) -> void:
	actor_health[message.get("target_actor_id", 0)] = message.get("remaining_health", 0)


func apply_objective(message: Dictionary) -> void:
	objectives[message.get("objective_id", "unknown")] = message.duplicate(true)


func apply_inventory_snapshot(message: Dictionary) -> void:
	inventory.clear()
	for item in message.get("items", []):
		inventory[item.get("item_id", "unknown")] = item.get("quantity", 0)


func apply_loot_grant(message: Dictionary) -> void:
	inventory[message.get("item_id", "unknown")] = message.get("resulting_quantity", 0)


func apply_progression(message: Dictionary) -> void:
	progression["level"] = message.get("level", 1)
	progression["experience"] = message.get("experience", 0)
	progression["experience_to_next_level"] = message.get("experience_to_next_level", 500)


func apply_equipment_snapshot(message: Dictionary) -> void:
	weapon_profiles.clear()
	for weapon in message.get("weapons", []):
		weapon_profiles[weapon.get("item_id", "unknown")] = weapon.duplicate(true)
	equipped_weapon_item_id = message.get("equipped_weapon_item_id", "pulse_rifle")


func apply_equipment_change(message: Dictionary) -> bool:
	if not message.get("accepted", false):
		return false
	equipped_weapon_item_id = message.get("equipped_weapon_item_id", equipped_weapon_item_id)
	return true


func apply_activity_complete(_message: Dictionary) -> void:
	activity_complete = true


func presentation_state() -> Dictionary:
	return {
		"player_actor_id": player_actor_id,
		"actor_count": actors.size(),
		"inventory": inventory.duplicate(true),
		"progression": progression.duplicate(true),
		"equipped_weapon_item_id": equipped_weapon_item_id,
		"weapon_profile_count": weapon_profiles.size(),
		"objective_count": objectives.size(),
		"activity_complete": activity_complete,
	}
