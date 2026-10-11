extends "res://tests/check.gd"

const FrameTiming = preload("res://report/frame_timing.gd")

func run() -> bool:
	# Frames end at 0, 5, 10, 20, 40 ms: with two warm-up frames the intervals
	# timed are the third frame's onward.
	var timing := FrameTiming.new(2)
	for end_ms in [0, 5, 10, 20, 40]:
		timing.drawn(end_ms * 1000, end_ms / 10.0, {"drawCalls": end_ms, "primitives": 1})
	var report: Dictionary = timing.to_dictionary()
	check(report.warmupFrames == 2, "the report names its warm-up: %s" % [report])
	check(report.intervalsMs == [5.0, 10.0, 20.0], "each timed interval runs from the previous frame's end, raw and in order: %s" % [report.intervalsMs])
	check(report.cpuMs == [1.0, 2.0, 4.0], "each timed frame's CPU time is kept raw beside its interval: %s" % [report.cpuMs])
	check(report.drawn.drawCalls == {"mean": 70.0 / 3, "max": 40}, "what the timed frames drew is averaged and its peak kept: %s" % [report.drawn])
	check(report.gpuMs == null, "no GPU time is reported as null, never a zero")
	check(FrameTiming.new(5).to_dictionary().intervalsMs.is_empty(), "frames inside the warm-up time nothing")
	return true
