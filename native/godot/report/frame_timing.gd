extends RefCounted

## Records a bench run's frames (`parity_run.gd --bench`) raw: after a
## warm-up, each frame's interval from the end of one frame's draw to the
## next, and its CPU time. The browser's own summary code
## (`web/fps-compare.mjs`) turns both clients' raw frames into numbers, so
## only the clock differs between them.

## Frames drawn before timing starts.
var warmup: int
var _drawn := 0
var _last_usec := 0
var _intervals: Array = []
var _cpu: Array = []
var _counts := {}

func _init(warmup_frames: int) -> void:
	warmup = warmup_frames

## A frame finished drawing at `now_usec` (`Time.get_ticks_usec`), having
## cost `cpu_ms` of CPU time and drawn `counts` (name to number, such as
## draw calls).
func drawn(now_usec: int, cpu_ms: float, counts: Dictionary) -> void:
	_drawn += 1
	if _drawn > warmup:
		_intervals.append((now_usec - _last_usec) / 1000.0)
		_cpu.append(cpu_ms)
		for name in counts:
			if not _counts.has(name):
				_counts[name] = []
			_counts[name].append(counts[name])
	_last_usec = now_usec

## The timed frames. `gpuMs` is null: Godot 4.7's Metal driver records no
## GPU timestamps. `drawn` gives each count's mean and peak.
func to_dictionary() -> Dictionary:
	var drawn := {}
	for name in _counts:
		var values: Array = _counts[name]
		var total := 0.0
		for value in values:
			total += value
		drawn[name] = {"mean": total / values.size(), "max": values.max()}
	return {"warmupFrames": warmup, "intervalsMs": _intervals, "cpuMs": _cpu, "gpuMs": null, "drawn": drawn}
