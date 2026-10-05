extends RefCounted

const CONTRACTS := {
	"relay_gauntlet": {"contract_revision": "relay-gauntlet-v1", "seed": 37008, "objective_count": 3, "gameplay_revision": "m37-gauntlet-v1"},
	"last_reserve": {"contract_revision": "last-reserve-v1", "seed": 37007, "objective_count": 3, "gameplay_revision": "m37-survival-v1"},
	"distant_signal": {"contract_revision": "distant-signal-v1", "seed": 37006, "objective_count": 1, "gameplay_revision": "m37-signal-v1"},
	"prism_discipline": {"contract_revision": "prism-discipline-v1", "seed": 37005, "objective_count": 1, "gameplay_revision": "m37-prism-v1"},
	"close_quarters": {"contract_revision": "close-quarters-v1", "seed": 37001, "objective_count": 2, "gameplay_revision": "m37-baseline-v1"},
	"meridian_circuit": {"contract_revision": "meridian-circuit-v1", "seed": 37002, "objective_count": 4, "gameplay_revision": "m37-baseline-v1"},
	"coolant_recovery": {"contract_revision": "coolant-recovery-v1", "seed": 37003, "objective_count": 3, "gameplay_revision": "m37-recovery-v1"},
	"bastion_link": {"contract_revision": "bastion-link-v1", "seed": 37004, "objective_count": 2, "gameplay_revision": "m37-elite-v1"},
}
const PRESETS := {
	"meridian_circuit": {"west_approach": null, "pace": 60000, "west_approach_pace": 60000},
	"relay_gauntlet": {"bastion_first": null, "single_reserve": null, "pace": 180000, "bastion_first_single_reserve": null, "single_reserve_pace": 180000},
}
const PRESET_NAMES := {"baseline": "Standard rules", "west_approach": "West approach", "bastion_first": "Bastion first", "single_reserve": "Single reserve", "pace": "Optional time goal", "west_approach_pace": "West approach + time goal", "bastion_first_single_reserve": "Bastion first + single reserve", "single_reserve_pace": "Single reserve + time goal"}
const POLICIES := {
	"m37-challenges-v8": ["close_quarters", "meridian_circuit", "coolant_recovery", "bastion_link", "prism_discipline", "distant_signal", "last_reserve", "relay_gauntlet"],
	"m37-challenges-v7": ["close_quarters", "meridian_circuit", "coolant_recovery", "bastion_link", "prism_discipline", "distant_signal", "last_reserve", "relay_gauntlet"],
	"m37-challenges-v6": ["close_quarters", "meridian_circuit", "coolant_recovery", "bastion_link", "prism_discipline", "distant_signal", "last_reserve"],
	"m37-challenges-v5": ["close_quarters", "meridian_circuit", "coolant_recovery", "bastion_link", "prism_discipline", "distant_signal"],
	"m37-challenges-v4": ["close_quarters", "meridian_circuit", "coolant_recovery", "bastion_link", "prism_discipline"],
	"m37-challenges-v1": ["close_quarters", "meridian_circuit"],
	"m37-challenges-v2": ["close_quarters", "meridian_circuit", "coolant_recovery"],
	"m37-challenges-v3": ["close_quarters", "meridian_circuit", "coolant_recovery", "bastion_link"],
}
var snapshot := {}


static func preset_id(contract: Dictionary) -> String:
	return str(contract.get("preset_id", "baseline"))


static func time_goal(contract: Dictionary) -> Variant:
	return PRESETS.get(contract.get("contract_id"), {}).get(preset_id(contract))



func apply(message: Dictionary, character_id: String = "") -> bool:
	if message.get("type") != "ChallengeSnapshot" or not POLICIES.has(message.get("policy_revision")): return false
	if not message.get("character_id") is String or message.character_id.is_empty(): return false
	if not character_id.is_empty() and message.character_id != character_id: return false
	if not snapshot.is_empty() and message.character_id != snapshot.character_id: return false
	var revision := int(message.get("state_revision", -1))
	if revision < 0 or revision < int(snapshot.get("state_revision", 0)): return false
	var available: Array = POLICIES[message.policy_revision]
	var modified: bool = message.policy_revision == "m37-challenges-v8"
	if not _valid_variants(message.get("variants", []), modified): return false
	var elapsed: Variant = message.get("elapsed_ms")
	if elapsed != null and (typeof(elapsed) not in [TYPE_INT, TYPE_FLOAT] or elapsed != int(elapsed) or elapsed < 0): return false
	if not message.get("contracts") is Array or message.contracts.size() != available.size(): return false
	var ids := []
	for contract in message.contracts:
		if not _valid_contract(contract) or contract.contract_id not in available or contract.contract_id in ids: return false
		ids.append(contract.contract_id)
	var active: Variant = message.get("active")
	if active != null and not _valid_run(active, revision, available, modified): return false
	if active != null and _complete(active): return false
	var result: Variant = message.get("last_result")
	if result != null:
		if not result is Dictionary or result.get("outcome") not in ["completed", "abandoned", "interrupted", "defeated"]: return false
		if not _valid_run(result.get("run"), revision, available, modified): return false
		if result.outcome == "completed" and not _complete(result.run): return false
	if not message.get("records") is Array or message.records.size() > (16 if modified else available.size()): return false
	ids.clear()
	for record in message.records:
		if not _valid_run(record, revision, available, modified) or not _complete(record) or (record.contract.contract_id + ":" + preset_id(record.contract)) in ids: return false
		ids.append(record.contract.contract_id + ":" + preset_id(record.contract))
	if message.get("mastery") != null and not preload("res://projection/challenge_mastery.gd").valid(message.mastery, message): return false
	snapshot = message.duplicate(true)
	return true


func _valid_contract(value: Variant, modified: bool = false) -> bool:
	if not value is Dictionary or not CONTRACTS.has(value.get("contract_id")): return false
	var expected: Dictionary = CONTRACTS[value.contract_id]
	var preset := preset_id(value)
	var gameplay: String = expected.gameplay_revision
	if preset != "baseline":
		if not modified or not PRESETS.get(value.contract_id, {}).has(preset): return false
		gameplay = "m37-modifiers-v1"
	return value.get("contract_revision") == expected.contract_revision and value.get("seed") == expected.seed and value.get("objective_count") == expected.objective_count and value.get("gameplay_revision") == gameplay


func _valid_run(value: Variant, revision: int, available: Array, modified: bool) -> bool:
	if not value is Dictionary or not _valid_contract(value.get("contract"), modified): return false
	if value.contract.contract_id not in available: return false
	if value.contract.contract_id in ["coolant_recovery", "last_reserve", "relay_gauntlet"] and int(value.get("objectives", -1)) not in [0, 1, 3, 7]: return false
	if int(value.get("run_id", 0)) < 1 or int(value.run_id) > revision: return false
	var mask := (1 << int(value.contract.objective_count)) - 1
	if int(value.get("objectives", -1)) < 0 or int(value.objectives) > mask: return false
	var elapsed: Variant = value.get("elapsed_ms")
	if (elapsed != null) != (_complete(value) and time_goal(value.contract) != null): return false
	if elapsed != null and (typeof(elapsed) not in [TYPE_INT, TYPE_FLOAT] or elapsed != int(elapsed) or elapsed < 0): return false
	var equipment: Variant = value.get("equipment")
	return equipment is Dictionary and equipment.get("catalog_revision") == "m35-v2" and int(equipment.get("loadout_revision", -1)) >= 0 and equipment.get("weapon_id") is String and equipment.get("modules") is Array


func _complete(run: Dictionary) -> bool:
	return int(run.objectives) == (1 << int(run.contract.objective_count)) - 1


func _valid_variants(value: Variant, modified: bool) -> bool:
	if not value is Array or value.size() != (8 if modified else 0): return false
	var seen := []
	for variant in value:
		if not variant is Dictionary: return false
		var presets: Dictionary = PRESETS.get(variant.get("contract_id"), {})
		var preset: String = variant.get("preset_id", "")
		var identity := str(variant.get("contract_id")) + ":" + preset
		if not presets.has(preset) or variant.get("time_goal_ms") != presets[preset] or identity in seen: return false
		seen.append(identity)
	return true
