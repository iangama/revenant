extends RefCounted

const AUTHORITATIVE_STATE := preload("res://projection/authoritative_state.gd")
const LOGICAL_BOUNDS := Rect2(Vector2.ZERO, Vector2(1280, 720))


func validate(fixtures: Dictionary) -> String:
	var console: Control = fixtures.console
	var projection := AUTHORITATIVE_STATE.new()
	projection.call("join_world", {"player_actor_id": 71})
	console.call("reset_for_connection")
	console.call("open_console")
	if not projection.call("begin_route_state_request"):
		return "M27 route capability request did not enter pending state"
	console.call("present", projection.route_state, projection.route_pending, projection.route_result, projection.route_summary, 71)
	var state: Dictionary = console.call("presentation_state")
	if (
		state.get("option_count") != 0
		or "CAPABILITY REQUEST PENDING" not in state.get("authority_text", "")
		or "BREACH" in state.get("breach_text", "")
		or "STABILIZE" in state.get("stabilize_text", "")
		or not state.get("breach_disabled", false)
		or not state.get("stabilize_disabled", false)
	):
		return "M27 route console exposed candidate or selection truth before the server response"

	var authoritative := _route_state(71, [71], [71], "choice_open", true, null)
	if not projection.call("apply_route_state", authoritative):
		return "M27 accepted authoritative route state was rejected"
	console.call("present", projection.route_state, projection.route_pending, projection.route_result, projection.route_summary, 71)
	state = console.call("presentation_state")
	if (
		state.get("option_count") != 2
		or state.get("breach_disabled", true)
		or state.get("stabilize_disabled", true)
		or "12000 BP" not in state.get("breach_text", "")
		or "100 XP" not in state.get("breach_text", "")
		or "150 XP" not in state.get("stabilize_text", "")
		or "ONE RESOLVES AFTER ACCEPTANCE" not in state.get("stabilize_text", "")
		or _contains_banned_claim(state)
	):
		return "M27 route options are not an exact neutral rendering of server engineering facts"

	var non_leader := AUTHORITATIVE_STATE.new()
	non_leader.call("join_world", {"player_actor_id": 72})
	if not non_leader.call("apply_route_state", _route_state(71, [71, 72], [71, 72], "choice_open", true, null)):
		return "M27 two-participant server route state was rejected"
	console.call("present", non_leader.route_state, {}, {}, {}, 72)
	state = console.call("presentation_state")
	if (
		not state.get("breach_disabled", false)
		or not state.get("stabilize_disabled", false)
		or "ACTOR 71 DECIDES" not in state.get("instruction_text", "")
		or non_leader.call("begin_route_choice", "godot-route-peer", "breach")
	):
		return "M27 non-leader can submit a choice or cannot see who decides"

	console.call("present", projection.route_state, projection.route_pending, projection.route_result, projection.route_summary, 71)
	var before_choice := projection.route_state.duplicate(true)
	if not projection.call("begin_route_choice", "godot-route-accepted", "breach"):
		return "M27 leader choice did not enter neutral pending state"
	if projection.route_state != before_choice:
		return "M27 pending route intent changed authoritative route state"
	console.call("present", projection.route_state, projection.route_pending, projection.route_result, projection.route_summary, 71)
	state = console.call("presentation_state")
	if (
		"CHOICE PENDING" not in state.get("lifecycle_text", "")
		or "ACCEPTED" in state.get("lifecycle_text", "")
		or not state.get("breach_disabled", false)
		or not state.get("stabilize_disabled", false)
	):
		return "M27 pending route choice looks accepted or permits another intent"

	var selection := _selection(71, [71], "breach", 777, "arc_surge")
	if not projection.call("apply_route_choice_result", _choice_result(true, false, "route selected", "godot-route-accepted", selection)):
		return "M27 authoritative accepted route selection did not project"
	if projection.route_result.get("selection", {}) != selection:
		return "M27 accepted route selection was recomputed instead of copied"
	console.call("present", projection.route_state, projection.route_pending, projection.route_result, projection.route_summary, 71)
	state = console.call("presentation_state")
	if (
		"ACCEPTED" not in state.get("lifecycle_text", "")
		or "ARC SURGE" not in state.get("lifecycle_text", "")
		or "SEED 777" not in state.get("lifecycle_text", "")
		or "COUNTER 20" not in state.get("lifecycle_text", "")
		or not state.get("breach_disabled", false)
	):
		return "M27 accepted route display lost exact server selection facts"

	var replay_projection := AUTHORITATIVE_STATE.new()
	replay_projection.call("join_world", {"player_actor_id": 71})
	replay_projection.call("apply_route_state", authoritative)
	replay_projection.route_pending = {"kind": "choice", "operation_id": "godot-route-accepted", "route_id": "breach"}
	if not replay_projection.call("apply_route_choice_result", _choice_result(true, true, "route selection replayed", "godot-route-accepted", selection)):
		return "M27 same-input accepted replay did not project"
	console.call("present", replay_projection.route_state, replay_projection.route_pending, replay_projection.route_result, {}, 71)
	if "ACCEPTED REPLAY" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M27 accepted replay is not distinct from first acceptance"

	var rejected_projection := AUTHORITATIVE_STATE.new()
	rejected_projection.call("join_world", {"player_actor_id": 71})
	rejected_projection.call("apply_route_state", authoritative)
	rejected_projection.call("begin_route_choice", "godot-route-conflict", "stabilize")
	if rejected_projection.call("apply_route_choice_result", _choice_result(false, false, "operation identifier conflict", "godot-route-conflict", null)):
		return "M27 server route rejection was presented as accepted"
	if rejected_projection.route_state != authoritative:
		return "M27 rejected route choice changed authoritative state"
	console.call("present", rejected_projection.route_state, {}, rejected_projection.route_result, {}, 71)
	if "REJECTED" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M27 route conflict lacks a distinct rejection state"

	var selected_state := _route_state(71, [71], [71], "routed", true, selection)
	if not projection.call("apply_route_state", selected_state):
		return "M27 routed state did not preserve accepted selection"
	var success := _success_summary(selection)
	if not projection.call("apply_route_summary", success):
		return "M27 authoritative route success summary did not project"
	console.call("present", projection.route_state, projection.route_pending, projection.route_result, projection.route_summary, 71)
	state = console.call("presentation_state")
	if (
		"SUCCEEDED" not in state.get("lifecycle_text", "")
		or "4321 / 90000 MS" not in state.get("lifecycle_text", "")
		or "5 SERVER TRANSITIONS" not in state.get("lifecycle_text", "")
		or "2 FRAGMENT(S) / 100 XP" not in state.get("lifecycle_text", "")
	):
		return "M27 success summary does not preserve exact terminal server truth"

	var timeout_projection := AUTHORITATIVE_STATE.new()
	timeout_projection.call("join_world", {"player_actor_id": 71})
	timeout_projection.call("apply_route_state", authoritative)
	timeout_projection.route_pending = {"kind": "choice", "operation_id": "godot-route-timeout", "route_id": "breach"}
	timeout_projection.call("apply_route_choice_result", _choice_result(true, false, "route selected", "godot-route-timeout", selection))
	timeout_projection.call("apply_route_state", selected_state)
	if not timeout_projection.call("apply_route_summary", _timeout_summary(selection)):
		return "M27 authoritative route timeout summary did not project"
	console.call("present", timeout_projection.route_state, {}, timeout_projection.route_result, timeout_projection.route_summary, 71)
	state = console.call("presentation_state")
	if "FAILED  •  DEADLINE EXCEEDED" not in state.get("lifecycle_text", "") or "NO ROUTE REWARD" not in state.get("lifecycle_text", ""):
		return "M27 timeout is not visibly distinct from success or rejection"

	var disconnected := AUTHORITATIVE_STATE.new()
	disconnected.call("join_world", {"player_actor_id": 71})
	disconnected.call("begin_route_state_request")
	disconnected.call("fail_route_request", "route state request lost", true)
	console.call("present", disconnected.route_state, disconnected.route_pending, disconnected.route_result, {}, 71)
	if "CONNECTION LOST  •  ROUTE NOT RESUMED" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M27 route connection loss is not distinct from server rejection"
	disconnected.call("join_world", {"player_actor_id": 73})
	if (
		not disconnected.route_state.is_empty()
		or not disconnected.route_result.is_empty()
		or not disconnected.route_summary.is_empty()
	):
		return "M27 reconnect retained stale route selection or terminal truth"

	console.call("present", authoritative, {}, {}, {}, 71)
	console.call("open_console")
	state = console.call("presentation_state")
	if not LOGICAL_BOUNDS.encloses(state.get("panel_rect", Rect2())):
		return "M27 route console escapes the logical 1280x720 canvas"
	if state.get("focus_names", []) != ["Breach", "Stabilize", "Refresh", "Close"] or state.get("focus_start") != "Breach":
		return "M27 route controls do not expose the complete leader keyboard focus set"
	for name in state.get("focus_names", []):
		var control: Control = console.find_child(name, true, false)
		if control == null or control.focus_mode != Control.FOCUS_ALL:
			return "M27 route control is not keyboard focusable: %s" % name
	console.call("set_reduced_flash", true)
	if not console.call("presentation_state").get("reduced_flash", false):
		return "M27 route console does not retain Reduced Flash state"
	console.call("close_console")
	if console.visible:
		return "M27 route console did not close for Escape/focus restoration"
	return ""


func _route_state(leader: int, participants: Array, capable: Array, phase: String, all_capable: bool, selection: Variant) -> Dictionary:
	return {
		"type": "RouteState",
		"schema_version": 1,
		"accepted": true,
		"message": "route state fixture",
		"session_id": "session-m27-godot",
		"activity_id": "relay_awakening",
		"phase": phase,
		"leader_actor_id": leader,
		"participant_actor_ids": participants,
		"capable_actor_ids": capable,
		"all_capable": all_capable,
		"routes": [_breach_option(), _stabilize_option()],
		"selection": selection,
	}


func _breach_option() -> Dictionary:
	return {
		"route_id": "breach",
		"objective_path": ["clear_drone_group", "reach_relay_door", "defeat_warden"],
		"duration_budget_ms": 90_000,
		"reward": _reward(2, 100),
		"events": [
			{"event_id": "overcharged_armor", "effect": _effect(12_000, 15)},
			{"event_id": "arc_surge", "effect": _effect(10_000, 20)},
		],
	}


func _stabilize_option() -> Dictionary:
	return {
		"route_id": "stabilize",
		"objective_path": ["clear_drone_group", "reach_relay_stabilizer", "reach_relay_door", "defeat_warden"],
		"duration_budget_ms": 90_000,
		"reward": _reward(1, 150),
		"events": [
			{"event_id": "shielded_channel", "effect": _effect(10_000, 10)},
			{"event_id": "residual_feedback", "effect": _effect(11_000, 15)},
		],
	}


func _selection(leader: int, participants: Array, route_id: String, seed: int, event_id: String) -> Dictionary:
	var option := _breach_option() if route_id == "breach" else _stabilize_option()
	var effect: Dictionary = _effect(10_000, 20) if event_id == "arc_surge" else option.events[0].effect
	return {
		"leader_actor_id": leader,
		"participant_actor_ids": participants,
		"route_id": route_id,
		"seed": seed,
		"event_id": event_id,
		"effect": effect,
		"objective_path": option.objective_path,
		"duration_budget_ms": option.duration_budget_ms,
		"reward": option.reward,
	}


func _choice_result(accepted: bool, replayed: bool, message: String, operation_id: String, selection: Variant) -> Dictionary:
	return {
		"type": "RouteChoiceResult",
		"schema_version": 1,
		"accepted": accepted,
		"replayed": replayed,
		"message": message,
		"session_id": "session-m27-godot",
		"operation_id": operation_id,
		"selection": selection,
	}


func _success_summary(selection: Dictionary) -> Dictionary:
	return {
		"type": "RouteOperationSummary",
		"schema_version": 1,
		"session_id": "session-m27-godot",
		"authoring_revision": "m27-v1",
		"catalog_revision": "m27-v1",
		"resolver_revision": "m27-permute63-v1",
		"operation_id": "godot-route-accepted",
		"leader_actor_id": selection.leader_actor_id,
		"participant_actor_ids": selection.participant_actor_ids,
		"route_id": selection.route_id,
		"seed": selection.seed,
		"event_id": selection.event_id,
		"effect": selection.effect,
		"objective_path": selection.objective_path,
		"transitions": _success_transitions(),
		"duration_budget_ms": 90_000,
		"elapsed_ms": 4321,
		"outcome": "succeeded",
		"reward": selection.reward,
		"grants": [{"actor_id": 71, "item_id": "relay_core_fragment", "item_quantity": 2, "experience": 100}],
		"no_reward_reason": null,
	}


func _timeout_summary(selection: Dictionary) -> Dictionary:
	return {
		"type": "RouteOperationSummary",
		"schema_version": 1,
		"session_id": "session-m27-godot",
		"authoring_revision": "m27-v1",
		"catalog_revision": "m27-v1",
		"resolver_revision": "m27-permute63-v1",
		"operation_id": "godot-route-timeout",
		"leader_actor_id": selection.leader_actor_id,
		"participant_actor_ids": selection.participant_actor_ids,
		"route_id": selection.route_id,
		"seed": selection.seed,
		"event_id": selection.event_id,
		"effect": selection.effect,
		"objective_path": selection.objective_path,
		"transitions": [{"ordinal": 0, "objective_id": "clear_drone_group", "from": "active", "to": "failed", "progress": 0, "target": 1}],
		"duration_budget_ms": 90_000,
		"elapsed_ms": 90_001,
		"outcome": "failed_timeout",
		"reward": null,
		"grants": [],
		"no_reward_reason": "deadline_exceeded",
	}


func _success_transitions() -> Array:
	return [
		{"ordinal": 0, "objective_id": "clear_drone_group", "from": "active", "to": "completed", "progress": 1, "target": 1},
		{"ordinal": 1, "objective_id": "reach_relay_door", "from": "pending", "to": "active", "progress": 0, "target": 1},
		{"ordinal": 2, "objective_id": "reach_relay_door", "from": "active", "to": "completed", "progress": 1, "target": 1},
		{"ordinal": 3, "objective_id": "defeat_warden", "from": "pending", "to": "active", "progress": 0, "target": 1},
		{"ordinal": 4, "objective_id": "defeat_warden", "from": "active", "to": "completed", "progress": 1, "target": 1},
	]


func _reward(quantity: int, experience: int) -> Dictionary:
	return {"item_id": "relay_core_fragment", "item_quantity": quantity, "experience": experience}


func _effect(health_basis_points: int, counter_damage: int) -> Dictionary:
	return {"warden_health_basis_points": health_basis_points, "warden_counter_damage": counter_damage}


func _contains_banned_claim(state: Dictionary) -> bool:
	var combined := " ".join([
		str(state.get("authority_text", "")),
		str(state.get("instruction_text", "")),
		str(state.get("breach_text", "")),
		str(state.get("stabilize_text", "")),
		str(state.get("lifecycle_text", "")),
	]).to_lower()
	var words := combined.replace("•", " ").replace("/", " ").replace("\n", " ").split(" ", false)
	for banned in ["best", "recommended", "safer", "fun", "preferred", "optimal", "valuable"]:
		if banned in words:
			return true
	return false
