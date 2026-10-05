extends RefCounted

# Suggestions only. Every preview and mutation still goes through the server.
const PRESETS := [
	{"name": "BREACHER", "weapon": "scatter_caster", "modules": ["module_focus_lens", "module_breach_shunt"], "description": "Pair with Scatter Caster. Strong close burst; short reach and slow follow-up."},
	{"name": "MARKSMAN", "weapon": "rail_driver", "modules": ["module_standoff_optic", "module_skirmish_drive"], "description": "Pair with Rail Driver. Long reach and quicker cycling; lower health and a close dead zone."},
	{"name": "GUARD", "weapon": "pulse_rifle", "modules": ["module_cycle_bypass", "module_ablative_shell"], "description": "Pair with Pulse Rifle. More health and steady fire; weaker hits and short reach."},
]
