extends "res://tests/check.gd"

## The ground native draws: the simulation's triangles seated exactly on the
## height field, facing up, and a shader header built from the run's own
## table layouts and tuning.

const HeightField = preload("res://terrain/height_field.gd")
const TerrainLayer = preload("res://terrain/terrain_layer.gd")
const ParityRun = preload("res://parity_run.gd")
const Coords = preload("res://core/coords.gd")

func run() -> bool:
	var vectors: Dictionary = read_json("res://tests/fixtures/height_vectors.json")
	var words := PackedByteArray()
	words.resize(vectors.words.size() * 4)
	for i in vectors.words.size():
		words.encode_u32(i * 4, int(vectors.words[i]))
	var heights = HeightField.decode(words, vectors.layout).heights
	var built := TerrainLayer.ground_mesh(heights)
	var vertices: PackedVector3Array = built.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = built.mesh.surface_get_arrays(0)[Mesh.ARRAY_NORMAL]
	var seated := true
	var facing_up := true
	var area := 0.0
	for i in range(0, vertices.size(), 3):
		var a := vertices[i]
		# Godot draws clockwise triangles as front faces: seen from above, an
		# up-facing triangle's (b - a) x (c - a) points down.
		var cross := (vertices[i + 1] - a).cross(vertices[i + 2] - a)
		facing_up = facing_up and cross.y < 0.0 and normals[i].dot(-cross.normalized()) > 0.9999
		area += absf(cross.y) * 0.5
		for k in 3:
			var v := vertices[i + k]
			seated = seated and is_equal_approx(v.y, heights.height_at(v.x, -v.z))
		# Its centre too: the triangle lies on the cell's own triangle.
		var centre := (vertices[i] + vertices[i + 1] + vertices[i + 2]) / 3.0
		seated = seated and absf(centre.y - heights.height_at(centre.x, -centre.z)) < 1e-4
	check(seated, "every vertex and triangle centre lies on the height field")
	check(facing_up, "every ground triangle faces up, with its own normal")
	var width: float = (heights.nx - 1) * heights.spacing
	var depth: float = (heights.ny - 1) * heights.spacing
	check(is_equal_approx(area, width * depth), "the triangles cover the grid once (area %s of %s)" % [area, width * depth])
	check(built.triangles < 2 * (heights.nx - 1) * (heights.ny - 1), "flat rectangles are coalesced")

	# The header names every field the shader reads from the run's own layouts.
	var fixture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json"))
	var ground := ParityRun.read_ground(FIXTURE_RUN.path_join(FIXTURE_SCENE), fixture)
	var header := TerrainLayer.shader_header(ground.tables, fixture.tuning.terrain)
	check(header.ok, "the fixture's tables make a header: %s" % header.get("detail", ""))
	for define in ["#define PLOTS_rows 4", "#define PARAMS_roads_stride 14", "#define PARAMS_roads_count 4", "#define SURFACEINDEX_kindShift 30u", "#define TUNING_grain_weights_2 0.35"]:
		check(header.get("defines", "").contains(define + "\n"), "the header defines %s" % define)
	var missing: Dictionary = ground.tables.duplicate()
	missing.erase("plots")
	check(not TerrainLayer.shader_header(missing, {}).ok, "a missing table is refused")
	check(header.get("defines", "").contains("#define SCARS_grass 12\n"), "the header names the scar params' fields")

	# The water surface lies flat on each cell the browser laid, at its height.
	var water_words := PackedFloat32Array([16.0, 24.0, -1.5, 8.0, 24.0, 24.0, -1.5, 8.0]).to_byte_array()
	var fields := [{"name": "corner", "word": 0, "type": "f32", "count": 2}, {"name": "z", "word": 2, "type": "f32", "count": 1}, {"name": "size", "word": 3, "type": "f32", "count": 1}]
	var cells := TerrainLayer.water_cells({"words": water_words, "layout": {"kind": "records", "stride": 4, "count": 2, "fields": fields}})
	check(cells.ok and cells.cells.size() == 2, "the water table reads as two cells")
	var water := TerrainLayer.water_mesh(cells.cells)
	var corners: PackedVector3Array = water.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var box := AABB(corners[0], Vector3.ZERO)
	for corner in corners:
		box = box.expand(corner)
	check(box.is_equal_approx(AABB(Coords.sim_to_godot(Vector3(16.0, 32.0, -1.5)), Vector3(16.0, 0.0, 8.0))), "the cells cover 16..32 east, 24..32 north at 1.5 m below datum, not %s" % box)
	check(corners.size() == 12, "each cell is two triangles")
	return true
