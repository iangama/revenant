extends RefCounted

const PLAYER_INTENT_CONTROLLER := preload("res://input/player_intent_controller.gd")
const OPERATOR_SCENE := preload("res://presentation/operator/operator.tscn")
const RELAY_DRONE_SCENE := preload("res://presentation/enemies/relay_drone/relay_drone.tscn")
const WARDEN_SCENE := preload("res://presentation/enemies/warden/warden.tscn")
const CAPTURE_FILENAMES := ["01-local-attempt.png", "02-server-confirmed-hit.png"]


func validate() -> String:
	var intents := PLAYER_INTENT_CONTROLLER.new()
	if intents.call("presentation_state").get("attack_cooldown_ms") != 260:
		return "M25 combat input does not preserve the safe pre-snapshot cooldown"
	if not intents.call("set_attack_cooldown_ms", 150):
		return "M25 combat input rejected the authoritative sidearm cooldown"
	intents.call("request_attack", true)
	var first: Dictionary = intents.call("take_attack", 1000)
	if not first.get("requested", false):
		return "M25 combat input did not expose the first sidearm intent"
	intents.call("consume_attack", 1000)
	intents.call("request_attack", true)
	var cooling: Dictionary = intents.call("take_attack", 1149)
	if not cooling.get("cooling", false) or cooling.get("remaining_ms") != 1:
		return "M25 sidearm cooldown boundary is not exact"
	var ready: Dictionary = intents.call("take_attack", 1150)
	if not ready.get("requested", false):
		return "M25 sidearm intent did not become ready at its authoritative boundary"
	intents.call("consume_attack", 1150)
	if not intents.call("set_attack_cooldown_ms", 250):
		return "M25 combat input rejected the authoritative rifle cooldown"
	intents.call("request_attack", true)
	if intents.call("take_attack", 1299).get("requested", false):
		return "M25 equipment change rewrote an already active sidearm cooldown"
	if not intents.call("take_attack", 1300).get("requested", false):
		return "M25 equipment change lost the original cooldown deadline"
	intents.call("consume_attack", 2000)
	if not intents.call("set_attack_cooldown_ms", 150):
		return "M25 combat input could not restore the sidearm profile"
	intents.call("request_attack", true)
	var switched_cooling: Dictionary = intents.call("take_attack", 2249)
	if not switched_cooling.get("cooling", false) or switched_cooling.get("remaining_ms") != 1:
		return "M25 faster equipment change bypassed the active rifle cooldown"
	if not intents.call("take_attack", 2250).get("requested", false):
		return "M25 switched intent did not become ready at the original rifle deadline"
	if intents.call("set_attack_cooldown_ms", 0) or intents.call("set_attack_cooldown_ms", 5001):
		return "M25 combat input accepted an invalid authoritative cooldown"
	if intents.call("presentation_state").get("attack_cooldown_ms") != 150:
		return "M25 invalid cooldown changed the last accepted profile"
	return ""


func validate_presentation(fixtures: Dictionary) -> String:
	var root: Node = fixtures.root
	var tree: SceneTree = fixtures.tree
	var combat_vfx: Node3D = fixtures.combat_vfx
	var audio_director: Node3D = fixtures.audio_director
	var status_label: Label = fixtures.status_label
	var health_label: Label = fixtures.health_label
	var enemy_health_label: Label = fixtures.enemy_health_label
	var controls_label: Label = fixtures.controls_label
	var save_capture: Callable = fixtures.save_capture
	var original_text := {
		"status": status_label.text,
		"health": health_label.text,
		"enemy_health": enemy_health_label.text,
		"controls": controls_label.text,
	}
	var operator: Node3D = OPERATOR_SCENE.instantiate()
	operator.name = "M25Operator"
	operator.position = Vector3(-2.0, 0.0, 1.5)
	root.add_child(operator)
	var drone: Node3D = RELAY_DRONE_SCENE.instantiate()
	drone.name = "M25Drone"
	drone.position = Vector3(2.0, 0.0, 1.5)
	root.add_child(drone)
	var warden: Node3D = WARDEN_SCENE.instantiate()
	warden.name = "M25Warden"
	warden.position = Vector3(3.5, 0.0, 2.5)
	root.add_child(warden)
	await tree.process_frame

	var vfx_before: Dictionary = combat_vfx.call("presentation_state")
	var audio_before: Dictionary = audio_director.call("presentation_state")
	combat_vfx.call("play_local_attempt", operator.global_position)
	combat_vfx.call("play_local_cooldown", operator.global_position, 150)
	audio_director.call("play_attack_attempt")
	audio_director.call("play_cooldown_acknowledgement")
	status_label.text = "LOCAL ATTEMPT  •  AWAITING SERVER"
	health_label.text = "HP  100 / 100  •  NO DAMAGE CONFIRMED"
	enemy_health_label.text = "TARGET  •  AUTHORITATIVE HEALTH UNCHANGED"
	controls_label.text = "AMBER FLASH + RING  •  LOCAL INTENT / COOLDOWN ONLY"
	var capture_directory := OS.get_environment("REVENANT_CAPTURE_M25_DIR")
	if (
		not capture_directory.is_empty()
		and await save_capture.call(capture_directory.path_join(CAPTURE_FILENAMES[0])) != OK
	):
		return "M25 local-attempt A/B capture could not be saved"
	await tree.create_timer(0.3).timeout

	combat_vfx.call("play_confirmed_exchange", operator, drone, false)
	audio_director.call("play_confirmed_attack", "pulse_rifle", operator.global_position)
	audio_director.call("play_confirmed_impact", drone.global_position)
	status_label.text = "SERVER HIT CONFIRMED  •  40 DAMAGE"
	health_label.text = "HP  100 / 100  •  STABLE"
	enemy_health_label.text = "ENEMY  •  100 / 140 HP"
	controls_label.text = "CYAN TRAIL + RED IMPACT  •  AUTHORITATIVE RESULT"
	if (
		not capture_directory.is_empty()
		and await save_capture.call(capture_directory.path_join(CAPTURE_FILENAMES[1])) != OK
	):
		return "M25 authoritative-hit A/B capture could not be saved"

	combat_vfx.call("play_target_unavailable", operator.global_position)
	audio_director.call("play_target_unavailable")
	combat_vfx.call("play_confirmed_exchange", warden, operator, true)
	audio_director.call("play_enemy_attack", "warden", warden.global_position)
	audio_director.call("play_confirmed_impact", operator.global_position)
	audio_director.call("play_player_damage")
	combat_vfx.call("play_confirmed_defeat", drone.global_position)
	audio_director.call("play_confirmed_defeat", drone.global_position)
	var vfx_after: Dictionary = combat_vfx.call("presentation_state")
	if (
		vfx_after.get("local_attempts", 0) - vfx_before.get("local_attempts", 0) != 1
		or vfx_after.get("cooldown_cues", 0) - vfx_before.get("cooldown_cues", 0) != 1
		or vfx_after.get("unavailable_targets", 0) - vfx_before.get("unavailable_targets", 0) != 1
		or vfx_after.get("confirmed_player_hits", 0) - vfx_before.get("confirmed_player_hits", 0) != 1
		or vfx_after.get("confirmed_hostile_hits", 0) - vfx_before.get("confirmed_hostile_hits", 0) != 1
		or vfx_after.get("confirmed_defeats", 0) - vfx_before.get("confirmed_defeats", 0) != 1
		or vfx_after.get("active_effects", 0) > vfx_after.get("maximum_active_effects", 0)
		or vfx_after.get("permanent_particles", 1) != 0
	):
		return "M25 visual combat semantics are not distinct and bounded: %s" % vfx_after
	combat_vfx.call("set_reduced_flash", true)
	var reduced_state: Dictionary = combat_vfx.call("presentation_state")
	if not reduced_state.get("reduced_flash", false) or reduced_state.get("flash_scale", 1.0) > 0.6:
		return "M25 combat VFX does not honor Reduced Flash"
	combat_vfx.call("set_reduced_flash", vfx_before.get("reduced_flash", false))

	var audio_after: Dictionary = audio_director.call("presentation_state")
	var requested_before: Dictionary = audio_before.get("requests", {})
	var requested_after: Dictionary = audio_after.get("requests", {})
	for cue in ["attack_attempt", "cooldown", "target_unavailable", "pulse_rifle", "impact", "warden", "player_damage", "defeat"]:
		if int(requested_after.get(cue, 0)) <= int(requested_before.get(cue, 0)):
			return "M25 audio semantic cue was not requested: %s" % cue
	var pitches: Dictionary = audio_after.get("semantic_pitch_scales", {})
	if pitches != {"attack_attempt": 1.3, "cooldown": 1.0, "target_unavailable": 0.72}:
		return "M25 local combat cues are not acoustically distinct: %s" % pitches
	if audio_after.get("active_voices", 0) > audio_after.get("maximum_voices", 0):
		return "M25 combat cues exceed the fixed audio pool"
	var suppressed_before := int(audio_after.get("suppressed", 0))
	audio_director.call("set_silent", true)
	audio_director.call("play_attack_attempt")
	audio_director.call("play_cooldown_acknowledgement")
	audio_director.call("play_target_unavailable")
	var muted_state: Dictionary = audio_director.call("presentation_state")
	if muted_state.get("active_voices", 1) != 0 or int(muted_state.get("suppressed", 0)) != suppressed_before + 3:
		return "M25 silent mode does not suppress every local combat semantic cue"
	audio_director.call("set_silent", false)

	status_label.text = original_text.status
	health_label.text = original_text.health
	enemy_health_label.text = original_text.enemy_health
	controls_label.text = original_text.controls
	operator.queue_free()
	drone.queue_free()
	warden.queue_free()
	await tree.create_timer(0.35).timeout
	return ""
