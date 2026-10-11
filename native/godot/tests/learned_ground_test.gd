extends "res://tests/check.gd"

## The capture side's learned ground, replayed from the fixture capture's
## publications, marks the same bare (crater, scorch or tracks) cells at each
## tick as the browser's `GroundView` (`fixtures/learned_ground_vectors.json`),
## and keeps the cells the market-town window's runs clear.

const Capture = preload("res://capture/presentation_capture.gd")
const LearnedGround = preload("res://capture/learned_ground.gd")

func run() -> bool:
	var vectors: Dictionary = read_json("res://tests/fixtures/learned_ground_vectors.json")
	var parsed := Capture.parse(FileAccess.get_file_as_string(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("capture.json")))
	var decoded := Capture.decode_samples(parsed)
	check(decoded.ok, "the fixture's samples decode")
	var ground := LearnedGround.new(parsed.layout.ground)
	var seen_bare := false
	for expected in vectors.ticks:
		ground.advance_to(decoded.samples, int(expected.tick))
		var bare := {}
		for cell in expected.bare:
			bare[Vector2i(int(cell[0]), int(cell[1]))] = true
		seen_bare = seen_bare or not bare.is_empty()
		# Every browser-bare cell is bare here, and its neighbours are only
		# when the browser's are too.
		for cell in bare:
			for offset in [Vector2i.ZERO, Vector2i.LEFT, Vector2i.RIGHT, Vector2i.UP, Vector2i.DOWN]:
				var at: Vector2i = cell + offset
				var got := ground.bare_at(at.x + 0.5, at.y + 0.5)
				check(got == bare.has(at), "tick %d cell %s: bare is %s, not %s" % [int(expected.tick), at, got, bare.has(at)])
	check(seen_bare, "the fixture's ground holds bare cells")

	# A run's cleared byte survives the replay, even on a run with no other mark.
	var window := Capture.parse(FileAccess.get_file_as_string("res://tests/fixtures/publication/market-town-capture.json"))
	var samples: Array = Capture.decode_samples(window).samples
	var cleared := LearnedGround.new(window.layout.ground)
	cleared.advance_to(samples, int(samples.back().tick))
	var expected: Array = read_json("res://tests/fixtures/publication/market-town-observations.json").observations
	var tiles_x := ceili(float(window.layout.ground.cols) / 16)
	var cleared_cells := 0
	for observation in expected:
		var runs: Array = observation.groundPatch.runs
		for at in range(0, runs.size(), 4):
			var tile := int(runs[at])
			var span := int(runs[at + 1])
			var want := int(runs[at + 3]) >= 65536
			for local in range(span % 256, span % 256 + span / 256):
				var x := (tile % tiles_x) * 16 + local % 16 + 0.5
				var y := (tile / tiles_x) * 16 + local / 16 + 0.5
				cleared_cells += int(want)
				check(cleared.cleared_at(x, y) == want, "cell (%d, %d): cleared is %s, not %s" % [x, y, not want, want])
	check(cleared_cells > 0, "the market-town window clears cells")
	return true
