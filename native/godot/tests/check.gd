extends SceneTree

## Shared base for headless tests. A failing `assert` hangs a script-mode
## Godot run, so tests record failures and exit through `finish()`.

const FIXTURE_RUN := "res://tests/fixtures/run"
const FIXTURE_MAPS := "res://tests/fixtures/maps"
## A trimmed real browser capture: paris-corner through its first plate frame.
const FIXTURE_SCENE := "paris-corner"

var failures: Array[String] = []

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	# A script error aborts the body but not this caller, so a body only
	# counts when it returns true from its last line.
	if await run() != true:
		failures.append("the test body stopped before its end")
	finish()

## Override with the test body; its last line returns true.
func run() -> bool:
	return true

func check(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)
		push_error(message)

func finish() -> void:
	print(JSON.stringify({"test": get_script().resource_path.get_file(), "failures": failures}))
	quit(0 if failures.is_empty() else 1)

static func read_json(path: String) -> Variant:
	return JSON.parse_string(FileAccess.get_file_as_string(path))

## A fresh scratch directory under the repository's ignored `throwaway/`.
static func scratch(name: String) -> String:
	var path := ProjectSettings.globalize_path("res://").path_join("../../throwaway/godot-tests").path_join(name).simplify_path()
	if DirAccess.dir_exists_absolute(path):
		for file in DirAccess.get_files_at(path):
			DirAccess.remove_absolute(path.path_join(file))
	DirAccess.make_dir_recursive_absolute(path)
	return path
