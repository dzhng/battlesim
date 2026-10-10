extends Logger

## Collects every error Godot logs while a run is registered. A GDScript
## runtime error returns null to its caller and lets the run go on, so without
## this a broken run would still write plates and a report with no blocker.
## Warnings are not collected.

var _mutex := Mutex.new()
var _errors: Array[String] = []

func _log_error(function: String, file: String, line: int, code: String, rationale: String, _editor_notify: bool, error_type: int, _script_backtraces: Array[ScriptBacktrace]) -> void:
	if error_type == ERROR_TYPE_WARNING:
		return
	_mutex.lock()
	_errors.append("%s at %s:%d (%s)" % [rationale if not rationale.is_empty() else code, file, line, function])
	_mutex.unlock()

## Every error logged so far.
func errors() -> Array[String]:
	_mutex.lock()
	var copy := _errors.duplicate()
	_mutex.unlock()
	return copy
