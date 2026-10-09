extends SceneTree
const CaptureDecoder = preload("res://presentation_capture.gd")
func _init() -> void:
	var path := OS.get_environment("GODOT_PRESENTATION_CAPTURE")
	if path.is_empty():
		push_error("GODOT_PRESENTATION_CAPTURE is required")
		quit(2)
		return
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_error("capture missing")
		quit(2)
		return
	var decoded := CaptureDecoder.decode_json(file.get_as_text())
	if not decoded.valid:
		push_error("capture invalid")
		quit(2)
	var baselines: Array = []
	var unit_count := 0
	var valid_publications := 0
	var fog_publications := 0
	var ground_packed_invalid := 0
	var sample_limit := int(OS.get_environment("GODOT_SEMANTIC_SAMPLE_LIMIT"))
	if sample_limit <= 0:
		sample_limit = decoded.capture.samples.size()
	for sample_index in mini(sample_limit, decoded.capture.samples.size()):
		var sample = decoded.capture.samples[sample_index]
		var result := CaptureDecoder.decode_publication(decoded, sample.publication, baselines)
		if not result.valid:
			push_error("semantic publication decode failed at %d tick %s: %s" % [sample_index, sample.tick, result.get("error", "unknown")])
			quit(2)
		baselines = result.baselines
		if result.has("fog") and int(result.fog.words) > 0:
			fog_publications += 1
		if result.has("ground") and not result.ground.packedValid:
			ground_packed_invalid += 1
		unit_count += result.units.size()
		valid_publications += 1
	print(JSON.stringify({"candidate":"battle-presentation-semantic-decoder","valid":true,"publications":valid_publications,"units":unit_count,"fog_publications":fog_publications,"ground_packed_invalid":ground_packed_invalid}))
	quit(0)
func mini(a: int, b: int) -> int:
	return a if a < b else b
