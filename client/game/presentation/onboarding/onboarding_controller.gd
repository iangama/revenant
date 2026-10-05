extends RefCounted

const CONTENT := {
	"Movement": ["STEP 1  •  MOVE", "Move with the arrow keys, left stick, or on-screen pad."],
	"Attack": ["STEP 2  •  ATTACK", "Attack the highlighted drone. Mouse clicks aim at the cursor."],
	"Loadout": ["OPTIONAL  •  LOADOUT", "Choose a weapon from the action bar or use the shoulder buttons."],
	"Door": ["STEP 3  •  OPEN THE CORE", "Head east to the amber core door at [6, 0]."],
	"Warden": ["STEP 4  •  DEFEAT THE WARDEN", "Keep attacking the Warden until its health reaches zero."],
	"Completion": ["MISSION COMPLETE", "Reward received. Open Modules to prepare your next run."],
}
var mode := "Full"
var step := "Movement"
var dismissed := false
var local_movement_attempted := false
var local_attack_attempted := false
var confirmed := {}

func reset(new_mode := "Full") -> void:
	mode = new_mode
	step = "Movement"
	dismissed = false
	local_movement_attempted = false
	local_attack_attempted = false
	confirmed.clear()

func set_mode(value: String) -> void:
	mode = value if value in ["Full", "Compact", "Off"] else "Full"

func note_local(kind: String) -> void:
	if kind == "movement": local_movement_attempted = true
	if kind == "attack": local_attack_attempted = true

func confirm(evidence: String) -> void:
	confirmed[evidence] = true
	if evidence == "movement" and step == "Movement": step = "Attack"
	elif evidence == "damage" and step in ["Movement", "Attack"]: step = "Loadout"
	elif evidence == "equipment" and step == "Loadout": step = "Door"
	elif evidence == "door_objective": step = "Door"
	elif evidence == "warden_spawn": step = "Warden"
	elif evidence == "completion": step = "Completion"
	dismissed = false

func guidance() -> Dictionary:
	return {"step": step, "title": CONTENT[step][0], "instructions": CONTENT[step][1], "visible": not dismissed and mode != "Off", "compact": mode == "Compact"}

func toggle() -> void:
	dismissed = not dismissed

func presentation_state() -> Dictionary:
	return {"mode": mode, "step": step, "dismissed": dismissed, "local_movement_attempted": local_movement_attempted, "local_attack_attempted": local_attack_attempted, "confirmed": confirmed.duplicate(true)}
