extends "res://validation/campaign_journey_flow.gd"

var _observations := preload("res://validation/final_observations.gd").new()
var _review_stage := ""
var _review_profiles := []


func _initialize() -> void:
	process_frame.connect(func() -> void: _observations.sample(game, _review_stage))
	super._initialize()


func _check_campaign_controls() -> void:
	super._check_campaign_controls()
	var index: int = CHAPTERS.find(game.get("_campaign_chapter"))
	_review_stage = CHAPTERS[index]
	_observations.start()
	var settings: Dictionary = game.get("_settings").duplicate(true)
	settings.bindings = preload("res://input/input_bindings.gd").defaults()
	settings.language = "pt_BR" if index % 2 else "en"
	settings.ui_scale = [1.0, 1.5, 1.25, 1.5, 1.0, 1.5][index]
	settings.muted = index == 1
	settings.captions = index != 2
	settings.high_contrast = index in [1, 3, 5]
	settings.reduced_motion = index in [1, 3, 5]
	settings.reduced_flash = index != 0
	settings.guidance_mode = ["Full", "Full", "Compact", "Off", "Compact", "Full"][index]
	settings.display_mode = "Fullscreen" if index == 4 else "Windowed"
	game.call("_apply_settings", settings, false)
	_review_profiles.append({"chapter": _review_stage, "settings": settings.duplicate(true)})
	_check(game.get("_sound_captions").call("presentation_state").enabled == settings.captions, "caption setting reaches campaign chapter")
	_check(game.get("_audio_director").call("presentation_state").silent == settings.muted, "mute reaches campaign chapter")
	if DisplayServer.get_name() != "headless":
		_check(DisplayServer.window_get_mode() == (DisplayServer.WINDOW_MODE_FULLSCREEN if index == 4 else DisplayServer.WINDOW_MODE_WINDOWED), "display setting applies")


func _capture(filename: String) -> void:
	await super._capture(filename)
	if filename != "journey-default-ending.png": return
	var directory := OS.get_environment("REVENANT_CAPTURE_ARC_DIR")
	var report: Dictionary = _observations.finish(directory, "journey-observations")
	_check(report.stages.size() == 6, "all campaign chapters observed")
	for row in report.stages.values():
		_check(row.voices_max <= 14, "bounded audio pool throughout journey")
		_check(row.audio_peak <= 0.708, "sampled campaign mix keeps at least 3 dB headroom")
		_check(row.audio_bytes_max < 2 * 1024 * 1024, "retained audio remains bounded")
	_check(report.stages.prism_core.audio_samples > 0 and report.stages.prism_core.audio_peak > 0.0, "boss mix captured")
	var file := FileAccess.open(directory.path_join("journey-profiles.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(_review_profiles, "  ") + "\n")
	print("M38 final journey observations saved: six chapters, EN/PT, scale, contrast, motion/flash, captions/mute, guidance and display profiles; sampled audio headroom and fixed voices.")
