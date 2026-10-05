extends RefCounted

const MESSAGEPACK_CODEC := preload("res://protocol/messagepack_codec.gd")
const AUTHORITATIVE_STATE := preload("res://projection/authoritative_state.gd")
const PLAYER_INTENT_CONTROLLER := preload("res://input/player_intent_controller.gd")
const HUD_PROJECTION := preload("res://presentation/hud_projection.gd")


func validate(session: Node) -> String:
	var codec_validation: RefCounted = MESSAGEPACK_CODEC.new()
	if codec_validation.call("encode_array", [-12, 0, 12]) != PackedByteArray([0x93, 0xf4, 0x00, 0x0c]):
		return "M17 movement encoder does not support signed relay-hub coordinates"
	var signed_module_fixture := PackedByteArray([0x93, 0xd1, 0xfa, 0x24, 0xd1, 0xfc, 0x18, 0xf4])
	var signed_module_decoded: Array = codec_validation.call("decode_value", signed_module_fixture)
	if signed_module_decoded != [[-1500, -1000, -12], signed_module_fixture.size()]:
		return "M26 MessagePack decoder does not preserve signed module modifiers"
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
	var module_codec_fixture := {
		"type": "ModulePreviewRequest",
		"modules": ["module_force_matrix", "module_ward_capacitor"],
	}
	var module_codec_payload: PackedByteArray = codec_validation.call("encode_map", module_codec_fixture)
	var module_codec_decoded: Array = codec_validation.call("decode_value", module_codec_payload)
	if (
		codec_validation.call("has_failed")
		or module_codec_decoded.is_empty()
		or module_codec_decoded[0] != module_codec_fixture
		or module_codec_decoded[1] != module_codec_payload.size()
	):
		return "M26 MessagePack codec does not preserve bounded module request arrays"
	var route_map16_fixture := {
		"type": "RouteOperationSummary",
		"schema_version": 1,
		"session_id": "session-m27",
		"authoring_revision": "m27-v1",
		"catalog_revision": "m27-v1",
		"resolver_revision": "m27-permute63-v1",
		"operation_id": "godot-route-1",
		"leader_actor_id": 7,
		"participant_actor_ids": [7],
		"route_id": "breach",
		"seed": 777,
		"event_id": "arc_surge",
		"effect": {"warden_health_basis_points": 10_000, "warden_counter_damage": 20},
		"objective_path": ["clear_drone_group", "reach_relay_door", "defeat_warden"],
		"transitions": [],
		"duration_budget_ms": 90_000,
		"elapsed_ms": 90_001,
		"outcome": "failed_timeout",
		"reward": null,
		"grants": [],
		"no_reward_reason": "deadline_exceeded",
	}
	var route_map16_payload: PackedByteArray = codec_validation.call("encode_map", route_map16_fixture)
	var route_map16_decoded: Array = codec_validation.call("decode_value", route_map16_payload)
	if (
		codec_validation.call("has_failed")
		or route_map16_payload[0] != 0xde
		or route_map16_decoded.is_empty()
		or route_map16_decoded[0] != route_map16_fixture
		or route_map16_decoded[1] != route_map16_payload.size()
	):
		return "M27 MessagePack codec does not preserve map16 or nil route terminal fields"
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
