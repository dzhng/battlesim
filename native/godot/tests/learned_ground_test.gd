extends "res://tests/check.gd"

## The capture side's learned ground, replayed from the fixture capture's
## publications, marks the same bare (crater, scorch or tracks) cells at each
## tick as the browser's `GroundView` (`fixtures/learned_ground_vectors.json`).

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
	return true
