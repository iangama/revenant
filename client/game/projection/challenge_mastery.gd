extends RefCounted

const GOALS := {
	"breacher": {"name": "BREACHER", "contract": "close_quarters", "mode": "challenge_close_quarters", "rule": "Complete Close Quarters with Scatter Caster, Focus Lens and Breach Shunt."},
	"marksman": {"name": "MARKSMAN", "contract": "distant_signal", "mode": "challenge_distant_signal", "rule": "Complete Distant Signal with Rail Driver, Standoff Optic and Skirmish Drive."},
	"guard": {"name": "GUARD", "contract": "last_reserve", "mode": "challenge_last_reserve", "rule": "Complete Last Reserve with Pulse Rifle, Cycle Bypass and Ablative Shell. Keep the reserve unused."},
	"route_planner": {"name": "ROUTE PLANNER", "contract": "meridian_circuit", "mode": "challenge_meridian_circuit@west_approach", "rule": "Complete the west Meridian route in 40 moves or fewer. Take as much time as you need."},
	"target_priority": {"name": "TARGET PRIORITY", "contract": "bastion_link", "mode": "challenge_bastion_link", "rule": "Complete Bastion Link. Defeat the Mender before dealing any damage to the Bulwark."},
	"prism_execution": {"name": "PRISM EXECUTION", "contract": "prism_discipline", "mode": "challenge_prism_discipline", "rule": "Complete both Prism Discipline phases without taking pattern damage."},
}
const BADGES := {
	"arsenal_adept": {"name": "ARSENAL ADEPT", "goals": ["breacher", "marksman", "guard"]},
	"field_tactician": {"name": "FIELD TACTICIAN", "goals": ["route_planner", "target_priority"]},
	"prism_adept": {"name": "PRISM ADEPT", "goals": ["prism_execution"]},
}
const REASONS := {
	"achieved": "Mastery achieved. Its first successful attempt stays in your archive.",
	"complete_contract": "Finish the contract alive to earn this mastery.",
	"use_build": "Equip the listed weapon and both modules in the workshop, then start a new attempt.",
	"preserve_reserve": "Keep the reserve unused. Use cover between relay holds.",
	"use_west_route": "Choose West approach. This route goal has no time limit.",
	"reduce_route_moves": "Plan the two branches before leaving. Return west within 40 moves.",
	"mender_first": "Focus on the Mender first. Start damaging the Bulwark after the Mender falls.",
	"avoid_prism_damage": "Move out of each warning shape, including the phase-two pulses. Attack while the shell is open.",
}


static func valid(value: Variant, snapshot: Dictionary) -> bool:
	if not value is Dictionary or value.get("revision") != "m37-mastery-v1": return false
	if not value.get("records") is Array or value.records.size() > 6: return false
	if not value.get("badges") is Array or value.badges.size() > 3: return false
	var records := {}
	for record in value.records:
		if not record is Dictionary or not GOALS.has(record.get("goal")) or records.has(record.goal): return false
		if not _valid_run_id(record.get("run_id"), snapshot): return false
		if not snapshot.records.any(func(r: Dictionary) -> bool: return r.contract.contract_id == GOALS[record.goal].contract): return false
		records[record.goal] = record.run_id
	var badges := {}
	for badge in value.badges:
		if not badge is Dictionary or not BADGES.has(badge.get("badge")) or badges.has(badge.badge): return false
		var expected := 0
		for goal in BADGES[badge.badge].goals:
			if not records.has(goal): return false
			expected = maxi(expected, records[goal])
		if badge.get("run_id") != expected: return false
		badges[badge.badge] = true
	for id in BADGES:
		if BADGES[id].goals.all(func(goal: String) -> bool: return records.has(goal)) != badges.has(id): return false
	var attempt: Variant = value.get("last_attempt")
	if attempt == null: return records.is_empty()
	if not attempt is Dictionary or not _valid_run_id(attempt.get("run_id"), snapshot): return false
	var result: Variant = snapshot.get("last_result")
	if not result is Dictionary or attempt.run_id != result.run.run_id: return false
	if not attempt.get("assessments") is Array: return false
	var expected_goals := GOALS.keys().filter(func(goal: String) -> bool: return GOALS[goal].contract == result.run.contract.contract_id)
	if attempt.assessments.size() != expected_goals.size(): return false
	for assessment in attempt.assessments:
		if not assessment is Dictionary or assessment.get("goal") not in expected_goals or not REASONS.has(assessment.get("reason")): return false
		if assessment.reason == "achieved" and (result.outcome != "completed" or not records.has(assessment.goal) or records[assessment.goal] > attempt.run_id): return false
	for key in ["route_moves", "prism_damage"]:
		var number: Variant = attempt.get(key)
		if number != null and (typeof(number) not in [TYPE_INT, TYPE_FLOAT] or number != int(number) or number < 0): return false
	return true


static func _valid_run_id(value: Variant, snapshot: Dictionary) -> bool:
	return typeof(value) in [TYPE_INT, TYPE_FLOAT] and value == int(value) and value > 0 and value <= int(snapshot.state_revision)
