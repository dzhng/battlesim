extends RefCounted

## The `godot-render-report` a parity run writes. It names every budget (what
## was admitted, rendered and dropped, by whom and why), every blocker that
## stopped the run, and every plate written with its measured GPU time.
## Nothing native drops or approximates may be left out of it.

const SCHEMA := "godot-render-report"

## The manifest identity of the battle rendered, or empty before one is admitted.
var identity: Dictionary = {}
## The frame selection the run was asked for.
var frames := ""
var _budgets: Array = []
var _blockers: Array = []
var _plates: Array = []
var _grass: Array = []

func add_budget(owner: String, reason: String, source_id: String, admitted: int, rendered: int, dropped: int) -> void:
	_budgets.append({"owner": owner, "reason": reason, "sourceId": source_id, "admitted": admitted, "rendered": rendered, "dropped": dropped})

func add_blocker(name: String, detail: String) -> void:
	_blockers.append({"name": name, "detail": detail})

## `gpu_ms` is null when the renderer measured no GPU time.
func add_plate(index: int, path: String, gpu_ms: Variant) -> void:
	_plates.append({"index": index, "path": path, "gpuMs": gpu_ms})

## The grass a plate's frame grew: native's clumps per screen cell beside the
## browser's (`grass_layer.gd` `counts`).
func add_grass(counts: Dictionary) -> void:
	_grass.append(counts)

func ok() -> bool:
	return _blockers.is_empty()

func to_dictionary() -> Dictionary:
	return {
		"schema": SCHEMA,
		"identity": identity,
		"renderer": {
			"method": ProjectSettings.get_setting("rendering/renderer/rendering_method"),
			"driver": RenderingServer.get_current_rendering_driver_name(),
			"display": DisplayServer.get_name(),
		},
		"frames": frames,
		"budgets": _budgets,
		"blockers": _blockers,
		"plates": _plates,
		"grass": _grass,
	}

func write(path: String) -> Error:
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return FileAccess.get_open_error()
	file.store_string(JSON.stringify(to_dictionary(), "  ") + "\n")
	file.close()
	return OK
