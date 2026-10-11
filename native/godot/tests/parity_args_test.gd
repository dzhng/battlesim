extends "res://tests/check.gd"

const ParityRun = preload("res://parity_run.gd")

func run() -> bool:
	var plates := ParityRun.parse_args(PackedStringArray(["--run", "throwaway/r", "--scene", "paris-corner"]))
	check(plates.bench == -1 and plates.frames == "plates" and plates.out.ends_with("throwaway/r/paris-corner/native"), "a plate run shoots the plate records into native/: %s" % [plates])
	var bench := ParityRun.parse_args(PackedStringArray(["--run", "throwaway/r", "--scene", "paris-corner", "--bench", "60"]))
	check(bench.bench == 60 and bench.frames == "all" and bench.pace == "uncapped" and bench.out.ends_with("throwaway/r/paris-corner/native-bench"), "a bench run times every record uncapped after its warm-up, and reports beside the plates, not over them: %s" % [bench])
	var paced := ParityRun.parse_args(PackedStringArray(["--run", "throwaway/r", "--scene", "paris-corner", "--bench", "60", "--pace", "display", "--frames", "1,2,3"]))
	check(paced.pace == "display" and paced.frames == "1,2,3", "a bench run can be paced to the display and time a subset: %s" % [paced])
	for refused in [["--bench", "-1"], ["--bench", "many"], ["--pace", "display"], ["--bench", "60", "--pace", "vsync"]]:
		var args := PackedStringArray(["--run", "throwaway/r", "--scene", "paris-corner"])
		args.append_array(PackedStringArray(refused))
		check(ParityRun.parse_args(args).has("error"), "%s is refused" % [refused])
	return true
