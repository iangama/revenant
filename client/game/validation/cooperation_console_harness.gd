extends RefCounted

const AUTHORITATIVE_STATE := preload("res://projection/authoritative_state.gd")
const LOGICAL_BOUNDS := Rect2(Vector2.ZERO, Vector2(1280, 720))
const ANCHOR_ID := 81
const RUNNER_ID := 82


func validate(fixtures: Dictionary) -> String:
	var console: Control = fixtures.get("console")
	if console == null:
		return "M28 cooperation console fixture is missing"
	var projection := AUTHORITATIVE_STATE.new()
	projection.call("join_world", {"player_actor_id": ANCHOR_ID})
	console.call("reset_for_connection")
	console.call("present", {}, {}, {}, {}, {}, ANCHOR_ID)
	var state: Dictionary = console.call("presentation_state")
	if (
		state.get("participant_card_count") != 2
		or not state.get("start_disabled", false)
		or not state.get("ping_disabled", false)
		or not state.get("revive_disabled", false)
		or "NO AUTHORITATIVE" not in state.get("authority_text", "")
	):
		return "M28 empty cooperation surface invents participants, roles, or actions"

	if not projection.call("begin_cooperation_state_request"):
		return "M28 cooperation capability request did not enter pending"
	console.call("present", projection.cooperation_state, projection.cooperation_pending, projection.cooperation_result, projection.cooperation_life_states, projection.cooperation_summary, ANCHOR_ID)
	state = console.call("presentation_state")
	if (
		"CAPABILITY REQUEST PENDING" not in state.get("authority_text", "")
		or "NO ACTOR" in state.get("anchor_text", "")
		or "ACTOR 81" in state.get("anchor_text", "")
		or "60000" in state.get("operation_text", "")
	):
		return "M28 pending capability exposes authoritative facts before acceptance"

	var eligible := _state("eligible", null)
	if not projection.call("apply_cooperation_state", eligible):
		return "M28 valid eligible cooperation state was rejected"
	var invalid_rejection := _state_rejection("x".repeat(129))
	var rejected_projection := AUTHORITATIVE_STATE.new()
	rejected_projection.call("join_world", {"player_actor_id": ANCHOR_ID})
	if rejected_projection.call("apply_cooperation_state", invalid_rejection):
		return "M28 oversized cooperation capability rejection was accepted"
	if "INVALID SERVER DATA" not in str(rejected_projection.cooperation_result.get("message", "")):
		return "M28 invalid capability rejection lacks explicit protocol failure"
	console.call("present", projection.cooperation_state, projection.cooperation_pending, projection.cooperation_result, projection.cooperation_life_states, projection.cooperation_summary, ANCHOR_ID)
	state = console.call("presentation_state")
	if (
		state.get("start_disabled", true)
		or state.get("focus_start") != "Start"
		or "CAPABLE 2/2" not in state.get("authority_text", "")
		or "NO SERVER-DECLARED LOCAL ROLE" not in state.get("instruction_text", "")
	):
		return "M28 eligible leader cannot start or is assigned a role before operation admission"

	var runner_projection := AUTHORITATIVE_STATE.new()
	runner_projection.call("join_world", {"player_actor_id": RUNNER_ID})
	if not runner_projection.call("apply_cooperation_state", eligible):
		return "M28 runner fixture rejected shared eligible state"
	console.call("present", runner_projection.cooperation_state, {}, {}, {}, {}, RUNNER_ID)
	if not console.call("presentation_state").get("start_disabled", false) or runner_projection.call("begin_cooperation_start", "runner-start"):
		return "M28 second participant can submit the leader-only start"

	console.call("present", projection.cooperation_state, {}, projection.cooperation_result, {}, {}, ANCHOR_ID)
	var before_start := projection.cooperation_state.duplicate(true)
	if not projection.call("begin_cooperation_start", "godot-start"):
		return "M28 authoritative leader start did not enter pending"
	if projection.cooperation_state != before_start:
		return "M28 pending start changed authoritative cooperation state"
	console.call("present", projection.cooperation_state, projection.cooperation_pending, projection.cooperation_result, {}, {}, ANCHOR_ID)
	if "START PENDING" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M28 start pending is not neutral and explicit"
	var awaiting_anchor := _operation("awaiting_anchor")
	if not projection.call("apply_cooperation_start_result", _start_result(true, false, "godot-start", awaiting_anchor)):
		return "M28 accepted cooperation start did not project"
	console.call("present", projection.cooperation_state, projection.cooperation_pending, projection.cooperation_result, {}, {}, ANCHOR_ID)
	state = console.call("presentation_state")
	if (
		"START ACCEPTED" not in state.get("lifecycle_text", "")
		or "ANCHOR  •  ACTOR 81  •  YOU" not in state.get("anchor_text", "")
		or "RUNNER  •  ACTOR 82" not in state.get("runner_text", "")
		or "OP 60000" not in state.get("operation_text", "")
		or "PING 5000" not in state.get("operation_text", "")
		or "CHANNEL 2000 MS" not in state.get("operation_text", "")
		or "RANGE² 4" not in state.get("operation_text", "")
		or "2 FRAGMENT(S) / 125 XP EACH" not in state.get("operation_text", "")
	):
		return "M28 accepted start lost exact roles, targets, timing, or reward plan"

	var replayed := AUTHORITATIVE_STATE.new()
	replayed.call("join_world", {"player_actor_id": ANCHOR_ID})
	replayed.call("apply_cooperation_state", eligible)
	replayed.call("begin_cooperation_start", "godot-start")
	if not replayed.call("apply_cooperation_start_result", _start_result(true, true, "godot-start", awaiting_anchor)):
		return "M28 same start replay was rejected"
	console.call("present", replayed.cooperation_state, {}, replayed.cooperation_result, {}, {}, ANCHOR_ID)
	if "START ACCEPTED REPLAY" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M28 accepted start replay is not distinct"

	var rejected := AUTHORITATIVE_STATE.new()
	rejected.call("join_world", {"player_actor_id": ANCHOR_ID})
	rejected.call("apply_cooperation_state", eligible)
	rejected.call("begin_cooperation_start", "bad-start")
	if rejected.call("apply_cooperation_start_result", _start_result(false, false, "bad-start", null)):
		return "M28 rejected start was presented as accepted"
	console.call("present", rejected.cooperation_state, {}, rejected.cooperation_result, {}, {}, ANCHOR_ID)
	if "REJECTED" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M28 start rejection is not visibly distinct"

	var awaiting_ping := _operation("awaiting_ping")
	if not projection.call("apply_cooperation_state", _state("awaiting_ping", awaiting_ping)):
		return "M28 anchor-arrival state was rejected"
	console.call("present", projection.cooperation_state, {}, {}, {}, {}, ANCHOR_ID)
	state = console.call("presentation_state")
	if state.get("ping_disabled", true) or "[DONE] 01  ANCHOR ARRIVED" not in state.get("contribution_text", ""):
		return "M28 anchor cannot send the phase-authorized ping or contribution is hidden"
	var before_ping := projection.cooperation_state.duplicate(true)
	if not projection.call("begin_cooperation_ping", "godot-ping"):
		return "M28 valid ping did not enter pending"
	if projection.cooperation_state != before_ping:
		return "M28 pending ping changed authoritative state"
	var ping := _ping("godot-ping", true)
	if not projection.call("apply_cooperation_ping_result", _ping_result(true, false, "godot-ping", ping)):
		return "M28 accepted ping did not project"
	console.call("present", projection.cooperation_state, {}, projection.cooperation_result, {}, {}, ANCHOR_ID)
	state = console.call("presentation_state")
	if "PING ACCEPTED  •  LIVE" not in state.get("lifecycle_text", "") or not state.get("ping_disabled", false):
		return "M28 accepted ping is not explicit or still permits a duplicate intent"
	projection.cooperation_pending = {"kind": "ping", "operation_id": "godot-ping"}
	if not projection.call("apply_cooperation_ping_result", _ping_result(true, true, "godot-ping", ping)):
		return "M28 exact ping replay did not project"
	console.call("present", projection.cooperation_state, {}, projection.cooperation_result, {}, {}, ANCHOR_ID)
	if "PING ACCEPTED REPLAY" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M28 ping replay is not distinct from first acceptance"
	projection.cooperation_pending = {"kind": "ping", "operation_id": "second-ping"}
	if projection.call("apply_cooperation_ping_result", _ping_result(false, false, "second-ping", null)):
		return "M28 rejected ping was presented as accepted"

	var awaiting_runner := _operation("awaiting_runner")
	if not projection.call("apply_cooperation_state", _state("awaiting_runner", awaiting_runner)):
		return "M28 live ping state was rejected"
	var downed_life := _life("relay_feedback", null, "active", "downed", 100, 0)
	if not projection.call("apply_cooperation_life_state", downed_life):
		return "M28 authoritative runner downing did not project"
	console.call("present", projection.cooperation_state, {}, {}, projection.cooperation_life_states, {}, ANCHOR_ID)
	state = console.call("presentation_state")
	if "LIFE DOWNED" not in state.get("runner_text", "") or "RELAY FEEDBACK" not in state.get("runner_text", ""):
		return "M28 immediate downed feedback is missing or inferred from color"

	var runner_downed := _operation("runner_downed")
	if not projection.call("apply_cooperation_state", _state("runner_downed", runner_downed)):
		return "M28 runner-downed operation state was rejected"
	console.call("present", projection.cooperation_state, {}, {}, projection.cooperation_life_states, {}, ANCHOR_ID)
	if console.call("presentation_state").get("revive_disabled", true):
		return "M28 anchor cannot start revive in authoritative runner-downed phase"
	if not projection.call("begin_cooperation_revive", "godot-revive"):
		return "M28 valid revive did not enter pending"
	console.call("present", projection.cooperation_state, projection.cooperation_pending, {}, projection.cooperation_life_states, {}, ANCHOR_ID)
	if "REVIVE PENDING" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M28 revive pending is not neutral and explicit"
	var revive_started := _revive("godot-revive", "started", 1)
	if not projection.call("apply_cooperation_revive_result", _revive_result(true, false, "godot-revive", "started", revive_started)):
		return "M28 accepted revive start did not project"
	console.call("present", projection.cooperation_state, {}, projection.cooperation_result, projection.cooperation_life_states, {}, ANCHOR_ID)
	state = console.call("presentation_state")
	if "REVIVE STARTED" not in state.get("lifecycle_text", "") or "CHANNEL 2000 MS" not in state.get("lifecycle_text", "") or not state.get("revive_disabled", false):
		return "M28 revive channel lost exact accepted server facts"
	for status in ["replayed", "pending", "completed"]:
		var replay_flag: bool = status == "replayed"
		if not projection.call("apply_cooperation_revive_result", _revive_result(true, replay_flag, "godot-revive", status, _revive("godot-revive", status, 1))):
			return "M28 valid revive %s state was rejected" % status
	var cancelled := _revive("cancelled-revive", "cancelled", 9)
	if not projection.call("apply_cooperation_revive_result", _revive_result(true, false, "cancelled-revive", "cancelled", cancelled)):
		return "M28 bounded out-of-range revive cancellation was rejected"
	if projection.call("apply_cooperation_revive_result", _revive_result(false, false, "rejected-revive", "rejected", null)):
		return "M28 rejected revive was presented as accepted"

	var revived_life := _life("revive", ANCHOR_ID, "downed", "active", 0, 50)
	if not projection.call("apply_cooperation_life_state", revived_life):
		return "M28 authoritative revive life transition did not project"
	var encounter := _operation("encounter_active")
	if not projection.call("apply_cooperation_state", _state("encounter_active", encounter)):
		return "M28 encounter-active cooperation state was rejected"
	console.call("present", projection.cooperation_state, {}, projection.cooperation_result, projection.cooperation_life_states, {}, ANCHOR_ID)
	state = console.call("presentation_state")
	if "[DONE] 04  RUNNER REVIVED" not in state.get("contribution_text", "") or not state.get("ping_disabled", false) or not state.get("revive_disabled", false):
		return "M28 encounter state loses revive truth or leaves repeated actions enabled"

	var success := _summary("succeeded", null, null, _flags(5), 4321)
	if not projection.call("apply_cooperation_summary", success):
		return "M28 authoritative cooperation success summary was rejected"
	console.call("present", projection.cooperation_state, {}, projection.cooperation_result, projection.cooperation_life_states, projection.cooperation_summary, ANCHOR_ID)
	state = console.call("presentation_state")
	if "SUCCEEDED" not in state.get("lifecycle_text", "") or "2 ORDERED GRANTS" not in state.get("lifecycle_text", "") or "125 XP" not in state.get("lifecycle_text", "") or "[DONE] 05  WARDEN COMPLETED" not in state.get("contribution_text", ""):
		return "M28 success summary loses terminal time, contributions, or equal reward"

	var failures := [
		["failed_ping_timeout", null, "ping_timeout", _flags(1)],
		["failed_revive_timeout", null, "revive_timeout", _flags(3)],
		["failed_operation_timeout", null, "operation_timeout", _flags(4)],
		["failed_participant_defeated", "runner", "participant_defeated", _flags(4)],
		["abandoned_disconnect", "anchor", "participant_disconnected", _flags(2)],
	]
	for failure in failures:
		var failed := AUTHORITATIVE_STATE.new()
		failed.call("join_world", {"player_actor_id": ANCHOR_ID})
		failed.call("apply_cooperation_state", _state("encounter_active", encounter))
		if not failed.call("apply_cooperation_summary", _summary(failure[0], failure[1], failure[2], failure[3], 60001)):
			return "M28 valid terminal family was rejected: %s" % failure[0]
		console.call("present", failed.cooperation_state, {}, failed.cooperation_result, {}, failed.cooperation_summary, ANCHOR_ID)
		var terminal_text := str(console.call("presentation_state").get("lifecycle_text", ""))
		if "NO REWARD" not in terminal_text or str(failure[2]).replace("_", " ").to_upper() not in terminal_text:
			return "M28 terminal family lacks typed non-color no-reward presentation: %s" % failure[0]

	var invalid := AUTHORITATIVE_STATE.new()
	invalid.call("join_world", {"player_actor_id": ANCHOR_ID})
	invalid.call("apply_cooperation_state", eligible)
	var invalid_operation := _operation("awaiting_anchor")
	invalid_operation.timing.operation_duration_ms = 59_999
	if invalid.call("apply_cooperation_state", _state("awaiting_anchor", invalid_operation)):
		return "M28 invalid timing contract was accepted"
	console.call("present", invalid.cooperation_state, {}, invalid.cooperation_result, {}, {}, ANCHOR_ID)
	if "INVALID SERVER DATA" not in console.call("presentation_state").get("lifecycle_text", "") or not invalid.cooperation_state.is_empty():
		return "M28 invalid server state retained partial truth or lacks explicit failure"

	var disconnected := AUTHORITATIVE_STATE.new()
	disconnected.call("join_world", {"player_actor_id": ANCHOR_ID})
	disconnected.call("apply_cooperation_state", eligible)
	disconnected.call("begin_cooperation_start", "lost-start")
	disconnected.call("fail_cooperation_request", "relay connection closed; reconnect from the beginning", true)
	console.call("present", disconnected.cooperation_state, disconnected.cooperation_pending, disconnected.cooperation_result, {}, {}, ANCHOR_ID)
	if "CONNECTION LOST  •  OPERATION NOT RESUMED" not in console.call("presentation_state").get("lifecycle_text", ""):
		return "M28 connection loss is not distinct from a server rejection"
	disconnected.call("join_world", {"player_actor_id": 83})
	if not disconnected.cooperation_state.is_empty() or not disconnected.cooperation_life_states.is_empty() or not disconnected.cooperation_summary.is_empty():
		return "M28 reconnect retained cooperation operation, life, or terminal truth"

	console.call("present", eligible, {}, {}, {}, {}, ANCHOR_ID)
	console.call("open_console")
	state = console.call("presentation_state")
	if (
		not LOGICAL_BOUNDS.encloses(state.get("panel_rect", Rect2()))
		or state.get("focus_names", []) != ["Start", "Ping", "Revive", "Refresh", "Close"]
		or state.get("focus_start") != "Start"
		or not state.get("mouse_blocking", false)
	):
		return "M28 cooperation console escapes 1280x720 or lacks bounded keyboard/input behavior"
	for name in state.get("focus_names", []):
		var control: Control = console.find_child(name, true, false)
		if control == null or control.focus_mode != Control.FOCUS_ALL:
			return "M28 cooperation control is not keyboard focusable: %s" % name
	console.call("set_reduced_flash", true)
	if not console.call("presentation_state").get("reduced_flash", false):
		return "M28 cooperation console does not retain Reduced Flash"
	console.call("close_console")
	if console.visible:
		return "M28 cooperation console did not close for Escape/focus restoration"
	console.call("reset_for_connection")
	return ""


func _state(phase: String, operation: Variant) -> Dictionary:
	return {
		"type": "CooperationState",
		"schema_version": 1,
		"accepted": true,
		"message": "cooperation state refreshed",
		"session_id": "session-m28-godot",
		"activity_id": "relay_awakening",
		"phase": phase,
		"participant_actor_ids": [ANCHOR_ID, RUNNER_ID],
		"capable_actor_ids": [ANCHOR_ID, RUNNER_ID],
		"all_capable": true,
		"operation": operation,
	}


func _state_rejection(message: String) -> Dictionary:
	return {
		"type": "CooperationState",
		"schema_version": 1,
		"accepted": false,
		"message": message,
		"session_id": "session-m28-godot",
		"activity_id": "relay_awakening",
		"phase": "unavailable",
		"participant_actor_ids": [ANCHOR_ID, RUNNER_ID],
		"capable_actor_ids": [ANCHOR_ID],
		"all_capable": false,
		"operation": null,
	}


func _operation(phase: String) -> Dictionary:
	var count: int = int({
		"awaiting_anchor": 0,
		"awaiting_ping": 1,
		"awaiting_runner": 2,
		"runner_downed": 3,
		"revive_channel": 3,
		"encounter_active": 4,
		"succeeded": 5,
	}.get(phase, 0))
	var runner_downed: bool = phase in ["runner_downed", "revive_channel"]
	var ping: Variant = _ping("godot-ping", phase not in ["encounter_active", "succeeded"]) if count >= 2 else null
	var revive: Variant = null
	if phase == "revive_channel":
		revive = _revive("godot-revive", "started", 1)
	elif phase in ["encounter_active", "succeeded"]:
		revive = _revive("godot-revive", "completed", 1)
	return {
		"catalog_revision": "m28-v1",
		"start_operation_id": "godot-start",
		"phase": phase,
		"participants": [
			{"actor_id": ANCHOR_ID, "role": "anchor", "current_health": 100, "max_health": 100, "life": "active"},
			{"actor_id": RUNNER_ID, "role": "runner", "current_health": 0 if runner_downed else 50 if count >= 4 else 100, "max_health": 100, "life": "downed" if runner_downed else "active"},
		],
		"targets": [
			{"target": "relay_anchor", "position": [3, 0, 3]},
			{"target": "relay_console", "position": [4, 0, 3]},
		],
		"timing": {
			"operation_duration_ms": 60_000,
			"ping_ttl_ms": 5_000,
			"revive_window_ms": 15_000,
			"revive_channel_ms": 2_000,
			"revive_health": 50,
			"maximum_distance_squared": 4,
		},
		"reward": {"item_id": "relay_core_fragment", "item_quantity": 2, "experience": 125},
		"contributions": _flags(count),
		"observed_elapsed_ms": 3200,
		"ping": ping,
		"revive": revive,
		"terminal_outcome": "succeeded" if phase == "succeeded" else null,
	}


func _flags(count: int) -> Dictionary:
	return {
		"anchor_arrived": count >= 1,
		"pinged": count >= 2,
		"runner_arrived": count >= 3,
		"revived": count >= 4,
		"warden_completed": count >= 5,
		"revive_count": 1 if count >= 4 else 0,
	}


func _ping(operation_id: String, active: bool) -> Dictionary:
	return {
		"operation_id": operation_id,
		"source_actor_id": ANCHOR_ID,
		"target": "relay_console",
		"accepted_elapsed_ms": 500,
		"expires_elapsed_ms": 5500,
		"active": active,
	}


func _revive(operation_id: String, status: String, distance_squared: int) -> Dictionary:
	return {
		"operation_id": operation_id,
		"source_actor_id": ANCHOR_ID,
		"target_actor_id": RUNNER_ID,
		"started_elapsed_ms": 1200,
		"observed_elapsed_ms": 3300 if status == "completed" else 1500,
		"required_duration_ms": 2000,
		"maximum_distance_squared": 4,
		"distance_squared": distance_squared,
		"status": status,
	}


func _start_result(accepted: bool, replayed: bool, operation_id: String, operation: Variant) -> Dictionary:
	return {
		"type": "CooperationStartResult",
		"schema_version": 1,
		"accepted": accepted,
		"replayed": replayed,
		"message": "cooperation started" if accepted else "operation identifier conflict",
		"session_id": "session-m28-godot",
		"operation_id": operation_id,
		"operation": operation,
	}


func _ping_result(accepted: bool, replayed: bool, operation_id: String, ping: Variant) -> Dictionary:
	return {
		"type": "CooperationPingResult",
		"schema_version": 1,
		"accepted": accepted,
		"replayed": replayed,
		"message": "cooperation ping accepted" if accepted else "cooperation ping rejected",
		"session_id": "session-m28-godot",
		"operation_id": operation_id,
		"ping": ping,
	}


func _life(cause: String, source: Variant, before: String, after: String, health_before: int, health_after: int) -> Dictionary:
	return {
		"type": "CooperationLifeState",
		"schema_version": 1,
		"session_id": "session-m28-godot",
		"actor_id": RUNNER_ID,
		"role": "runner",
		"source_actor_id": source,
		"cause": cause,
		"elapsed_ms": 1100,
		"health_before": health_before,
		"health_after": health_after,
		"max_health": 100,
		"life_before": before,
		"life_after": after,
	}


func _revive_result(accepted: bool, replayed: bool, operation_id: String, status: String, revive: Variant) -> Dictionary:
	return {
		"type": "CooperationReviveResult",
		"schema_version": 1,
		"accepted": accepted,
		"replayed": replayed,
		"message": "cooperation revive updated" if accepted else "cooperation revive rejected",
		"session_id": "session-m28-godot",
		"operation_id": operation_id,
		"status": status,
		"revive": revive,
	}


func _summary(outcome: String, subject: Variant, reason: Variant, contributions: Dictionary, elapsed: int) -> Dictionary:
	var success: bool = outcome == "succeeded"
	var runner_defeated: bool = outcome == "failed_participant_defeated" and subject == "runner"
	var anchor_defeated: bool = outcome == "failed_participant_defeated" and subject == "anchor"
	return {
		"type": "CooperationOperationSummary",
		"schema_version": 1,
		"session_id": "session-m28-godot",
		"catalog_revision": "m28-v1",
		"start_operation_id": "godot-start",
		"last_nonterminal_phase": "encounter_active",
		"participants": [
			{"actor_id": ANCHOR_ID, "role": "anchor", "current_health": 0 if anchor_defeated else 85, "max_health": 100, "life": "defeated" if anchor_defeated else "active"},
			{"actor_id": RUNNER_ID, "role": "runner", "current_health": 0 if runner_defeated else 50, "max_health": 100, "life": "defeated" if runner_defeated else "active"},
		],
		"contributions": contributions,
		"outcome": outcome,
		"subject_role": subject,
		"terminal_elapsed_ms": elapsed,
		"reward": {"item_id": "relay_core_fragment", "item_quantity": 2, "experience": 125} if success else null,
		"grants": [
			{"actor_id": ANCHOR_ID, "item_id": "relay_core_fragment", "item_quantity": 2, "experience": 125},
			{"actor_id": RUNNER_ID, "item_id": "relay_core_fragment", "item_quantity": 2, "experience": 125},
		] if success else [],
		"no_reward_reason": reason,
	}
