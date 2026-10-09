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
	for sample in decoded.capture.samples.slice(0, mini(8, decoded.capture.samples.size())):
		var result := CaptureDecoder.decode_publication(decoded, sample.publication, baselines)
		if not result.valid:
			push_error("semantic publication decode failed")
			quit(2)
		baselines = result.baselines
		unit_count += result.units.size()
		valid_publications += 1
	print(JSON.stringify({"candidate":"battle-presentation-semantic-decoder","valid":true,"publications":valid_publications,"units":unit_count}))
	quit(0)
func mini(a: int, b: int) -> int:
	return a if a < b else b
