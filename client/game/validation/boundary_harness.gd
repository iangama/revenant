extends RefCounted

const MESSAGEPACK_CODEC := preload("res://protocol/messagepack_codec.gd")
const AUTHORITATIVE_STATE := preload("res://projection/authoritative_state.gd")
const PLAYER_INTENT_CONTROLLER := preload("res://input/player_intent_controller.gd")
const HUD_PROJECTION := preload("res://presentation/hud_projection.gd")


func validate(session: Node) -> String:
	var codec_validation: RefCounted = MESSAGEPACK_CODEC.new()
	if codec_validation.call("encode_array", [-12, 0, 12]) != PackedByteArray([0x93, 0xf4, 0x00, 0x0c]):
		return "M17 movement encoder does not support signed relay-hub coordinates"
	var codec_fixture := {"type": "MoveIntent", "position": [6, 0, 0]}
	var codec_payload: PackedByteArray = codec_validation.call("encode_map", codec_fixture)
	var codec_decoded: Array = codec_validation.call("decode_value", codec_payload)
	if (
		codec_validation.call("has_failed")
		or codec_decoded.is_empty()
		or codec_decoded[0] != codec_fixture
		or codec_decoded[1] != codec_payload.size()
	):
		return "M23 extracted MessagePack codec does not preserve the supported wire subset"
	var session_state: Dictionary = session.call("presentation_state")
	var transport_state: Dictionary = session_state.get("transport", {})
	if (
		session.name != "SessionController"
		or session_state.get("protocol_version") != 2
		or session_state.get("message_deadline_ms") != 5000
		or transport_state.get("maximum_frame_size") != 64 * 1024
		or transport_state.get("buffered_bytes") != 0
	):
		return "M23 session controller does not preserve its protocol and bounded transport contract"
	var projection_validation := AUTHORITATIVE_STATE.new()
	projection_validation.call("join_world", {"player_actor_id": 7})
	projection_validation.call("apply_actor_spawn", {"actor_id": 7, "health": 100, "max_health": 100, "position": [0, 0, 0]})
	projection_validation.call("apply_damage", {"target_actor_id": 7, "remaining_health": 90})
	projection_validation.call("apply_inventory_snapshot", {"items": [{"item_id": "pulse_rifle", "quantity": 1}]})
	projection_validation.call("apply_loot_grant", {"item_id": "relay_core_fragment", "resulting_quantity": 2})
	projection_validation.call("apply_progression", {"level": 7, "experience": 3200, "experience_to_next_level": 800})
	projection_validation.call("apply_equipment_snapshot", {"equipped_weapon_item_id": "pulse_rifle", "weapons": [{"item_id": "pulse_rifle", "damage": 25}]})
	projection_validation.call("apply_objective", {"objective_id": "clear_drone_group", "state": "Active"})
	projection_validation.call("apply_activity_complete", {"activity_id": "relay_awakening"})
	var projection_state: Dictionary = projection_validation.call("presentation_state")
	if (
		projection_state.get("player_actor_id") != 7
		or projection_state.get("actor_count") != 1
		or projection_validation.actor_health.get(7) != 90
		or projection_state.get("inventory", {}).get("relay_core_fragment") != 2
		or projection_state.get("progression", {}).get("experience") != 3200
		or projection_state.get("equipped_weapon_item_id") != "pulse_rifle"
		or projection_state.get("weapon_profile_count") != 1
		or projection_state.get("objective_count") != 1
		or not projection_state.get("activity_complete", false)
	):
		return "M23 authoritative state projection does not preserve server-confirmed domain facts"
	var intent_validation := PLAYER_INTENT_CONTROLLER.new()
	intent_validation.call("set_ui_movement", Vector2.RIGHT)
	intent_validation.call("request_attack", true)
	var intent_state: Dictionary = intent_validation.call("presentation_state")
	var attack_intent: Dictionary = intent_validation.call("take_attack", 1000)
	var hud_validation := HUD_PROJECTION.new()
	if (
		intent_validation.call("movement", 1000) != Vector2.RIGHT
		or intent_state.get("move_cooldown_ms") != 120
		or intent_state.get("attack_cooldown_ms") != 260
		or not attack_intent.get("requested", false)
		or not attack_intent.get("use_active_target", false)
		or hud_validation.call("inventory_text", {"relay_core_fragment": 2}) != "INVENTORY\nRELAY CORE FRAGMENT  x2"
		or hud_validation.call("equipment_text", "pulse_rifle", {"damage": 25, "range": 8, "cooldown_ms": 260}) != "WEAPON  •  PULSE RIFLE\nDMG 25  RANGE 8  COOLDOWN 260 MS"
	):
		return "M23 input and HUD coordination does not preserve intent-only controls and authoritative text"
	return ""
