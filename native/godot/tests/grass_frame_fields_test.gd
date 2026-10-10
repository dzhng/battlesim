extends "res://tests/check.gd"

## The grass build's view fields, recomputed from a frame record, are the ones
## the browser wrote for that frame: the fixture's grass params hold the
## fields of its plate frame's regrow (`grassTables.ts`).

const GrassLayer = preload("res://grass/grass_layer.gd")
const ParityRun = preload("res://parity_run.gd")
const WordTables = preload("res://core/word_tables.gd")

func run() -> bool:
	var dir := FIXTURE_RUN.path_join(FIXTURE_SCENE)
	var fixture: Dictionary = read_json(dir.path_join("manifest.json"))
	var capture: Dictionary = read_json(dir.path_join("capture.json"))
	var ground := ParityRun.read_ground(dir, fixture)
	var params: Dictionary = ground.grass.params
	var at := GrassLayer.field_vec4s(params.layout)
	var vectors := WordTables.vec4s(params)
	var words: PackedByteArray = params.words
	var plate: Dictionary = capture.frames.filter(func(frame): return frame.plate != null)[0]
	var fields := GrassLayer.frame_fields(plate, float(fixture.lens.fovY), ground.heights.lowest(), vectors[at.density].w, float(fixture.grass.constants.tileM))

	var eye: Vector4 = vectors[at.eye]
	check((fields.eye - eye).abs() < Vector4(1e-3, 1e-3, 1e-3, 1e-9), "the eye and its metres per pixel are the browser's: %s, not %s" % [fields.eye, eye])
	var window: Vector4 = vectors[at.window]
	check(fields.window == Vector2(window.x, window.y), "the window's corner is the browser's: %s, not %s" % [fields.window, Vector2(window.x, window.y)])
	var grid := Vector2i(words.decode_u32(at.grid * 16), words.decode_u32(at.grid * 16 + 4))
	check(fields.grid == grid, "the window's tiles are the browser's: %s, not %s" % [fields.grid, grid])
	# The browser lists the side planes in its own order; each is one of ours.
	for i in 4:
		var browser: Vector4 = vectors[at.planes + i]
		var nearest := INF
		for plane in fields.planes:
			nearest = minf(nearest, Vector3(plane.x - browser.x, plane.y - browser.y, plane.z - browser.z).length() + absf(plane.w - browser.w) / 1000.0)
		check(nearest < 1e-4, "side plane %s is one of ours (off by %f)" % [browser, nearest])
	return true
