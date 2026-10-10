extends "res://tests/check.gd"

## Native's ground height is the browser's: `height_at` matches `groundHeight`
## on a non-flat paged grid packed by the browser's own packer
## (`fixtures/height_vectors.json`), across page edges, absent (flat) pages,
## both triangles of a cell, and the clamp past the grid's edge.

const HeightField = preload("res://terrain/height_field.gd")

func run() -> bool:
	var vectors: Dictionary = read_json("res://tests/fixtures/height_vectors.json")
	var words := PackedByteArray()
	words.resize(vectors.words.size() * 4)
	for i in vectors.words.size():
		words.encode_u32(i * 4, int(vectors.words[i]))
	var field := HeightField.decode(words, vectors.layout)
	check(field.ok, "the height pages decode: %s" % field.get("detail", ""))
	if not field.ok:
		return true
	var worst := 0.0
	for sample in vectors.samples:
		worst = maxf(worst, absf(field.heights.height_at(float(sample[0]), float(sample[1])) - float(sample[2])))
	# The browser reads f32 heights in f64 arithmetic; so does GDScript.
	check(worst < 1e-9, "every sample matches the browser's height, worst error %s m" % worst)
	check(vectors.samples.any(func(sample): return absf(float(sample[2])) > 1.0), "the grid has relief to match")

	var flat := PackedByteArray()
	flat.resize(12)
	for at in 3:
		flat.encode_u32(at * 4, [16, 1, 0][at])
	var empty := HeightField.decode(flat, {"kind": "height-pages", "nx": 11, "ny": 11, "spacing": 4, "pageSize": 16})
	check(empty.ok and empty.heights.height_at(13.0, 27.0) == 0.0, "ground with no pages is flat at zero")
	var short := HeightField.decode(words.slice(0, 40), vectors.layout)
	check(not short.ok, "a table shorter than its directory says is refused")
	return true
