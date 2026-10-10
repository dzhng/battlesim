extends "res://tests/check.gd"

## Display-backed: the native grass build samples where the browser's does.
## For fixed tiles and ranks under the fixture's grass params, the build's own
## functions (`grass_build.compute`) give the browser's hash words and tile
## offsets bit for bit, and admit exactly the candidates the browser's density
## admits; the point each candidate falls at agrees within `FELL_ULPS`. The
## expected values are the browser's own build (`fixtures/grass_sampling_vectors.json`, written by
## `throwaway/grass-tools/sampling-vectors-gpu.ts`, the browser's WGSL run on WebGPU).

const ParityRun = preload("res://parity_run.gd")
const GrassLayer = preload("res://grass/grass_layer.gd")

## Where the build's entry point starts: the probe below replaces it.
const ENTRY := "layout(local_size_x = 64) in;"
const WORDS_PER_ROW := 8
## The two GPU compilers (Tint's MSL in the browser; glslang, SPIRV-Cross and
## MSL here) round the fall point's f32 arithmetic differently, each off
## strict f32 at different candidates: up to 8 ulps measured, a tenth of a
## millimetre 250 m out. Twice that bounds it; the hashes it is built from are
## exact.
const FELL_ULPS := 16

func run() -> bool:
	check(DisplayServer.get_name() != "headless", "the test needs a display")
	var vectors: Dictionary = read_json("res://tests/fixtures/grass_sampling_vectors.json")
	var fixture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json"))
	var ground := ParityRun.read_ground(FIXTURE_RUN.path_join(FIXTURE_SCENE), fixture)
	var layer = GrassLayer.new(ground, fixture.grass, fixture.tuning, fixture.lens)
	var built: Dictionary = layer.build_source()
	check(built.ok, "the build's source assembles: %s" % built.get("detail", ""))
	var source: String = built.source
	var entry := source.find(ENTRY)
	check(entry > 0, "the build's source has its entry point")
	var rows: Array = vectors.rows
	var inputs := PackedInt32Array()
	for row in rows:
		inputs.append_array([int(row.tile[0]), int(row.tile[1]), int(row.j), 0])
	source = source.substr(0, entry) + """
layout(local_size_x = 1) in;
layout(set = 0, binding = 13, std430) restrict readonly buffer Inputs { ivec4 inputs[]; };
layout(set = 0, binding = 14, std430) restrict writeonly buffer Outputs { uint outputs[]; };
void main() {
	uint at = gl_GlobalInvocationID.x;
	ivec2 tile = inputs[at].xy;
	uint j = uint(inputs[at].z);
	float T = gp(GRASS_window).z;
	vec2 lo = vec2(tile) * T;
	ivec2 cell = grass_tile_cell(lo);
	vec2 offset = grass_hash(uint(cell.x), uint(cell.y)).xy;
	vec3 h;
	vec2 fell = grass_fell(lo, cell, offset, j, h);
	vec3 eye = gp(GRASS_eye).xyz;
	float dist = max(distance(eye, vec3(fell, 0.0)), 0.5);
	float rank = (float(j) + 0.5) / (T * T);
	bool admitted = gp(GRASS_eye).w * dist < gp(GRASS_density).w && rank < grass_density(dist, max(eye.z, 0.0));
	uint o = at * %du;
	outputs[o + 0u] = uint(offset.x * 16777216.0);
	outputs[o + 1u] = uint(offset.y * 16777216.0);
	outputs[o + 2u] = uint(h.x * 16777216.0);
	outputs[o + 3u] = uint(h.y * 16777216.0);
	outputs[o + 4u] = uint(h.z * 16777216.0);
	outputs[o + 5u] = floatBitsToUint(fell.x);
	outputs[o + 6u] = floatBitsToUint(fell.y);
	outputs[o + 7u] = admitted ? 1u : 0u;
}
""" % WORDS_PER_ROW
	var rd := RenderingServer.create_local_rendering_device()
	var shader := GrassLayer.compile(rd, source, "grass sampling probe")
	check(shader.is_valid(), "the probe compiles against the build's functions")
	if not shader.is_valid():
		rd.free()
		return true
	var buffers := {
		0: rd.storage_buffer_create(ground.grass.params.words.size(), ground.grass.params.words),
		13: rd.storage_buffer_create(inputs.size() * 4, inputs.to_byte_array()),
		14: rd.storage_buffer_create(rows.size() * WORDS_PER_ROW * 4),
	}
	# The build's other bindings: the probe reads none of them.
	for binding in [1, 2, 3, 4, 5, 6, 11, 12]:
		buffers[binding] = rd.storage_buffer_create(16, PackedByteArray([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))
	var format := RDTextureFormat.new()
	format.format = RenderingDevice.DATA_FORMAT_R8G8B8A8_UNORM
	format.usage_bits = RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
	var texture := rd.texture_create(format, RDTextureView.new(), [PackedByteArray([0, 0, 0, 0])])
	var sampler := rd.sampler_create(RDSamplerState.new())
	var uniforms: Array[RDUniform] = []
	for binding in [7, 8, 9, 10]:
		var table := RDUniform.new()
		table.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
		table.binding = binding
		table.add_id(sampler)
		table.add_id(texture)
		uniforms.append(table)
	for binding in buffers:
		var uniform := RDUniform.new()
		uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
		uniform.binding = binding
		uniform.add_id(buffers[binding])
		uniforms.append(uniform)
	var pipeline := rd.compute_pipeline_create(shader)
	var set := rd.uniform_set_create(uniforms, shader, 0)
	var list := rd.compute_list_begin()
	rd.compute_list_bind_compute_pipeline(list, pipeline)
	rd.compute_list_bind_uniform_set(list, set, 0)
	rd.compute_list_dispatch(list, rows.size(), 1, 1)
	rd.compute_list_end()
	rd.submit()
	rd.sync()
	var out := Array(rd.buffer_get_data(buffers[14]).to_int32_array()).map(func(word): return word & 0xFFFFFFFF)
	var admitted := 0
	var worst := 0
	for i in rows.size():
		var row: Dictionary = rows[i]
		var o := i * WORDS_PER_ROW
		var what := "tile %s rank %d" % [row.tile, row.j]
		var got := [out[o], out[o + 1]]
		check(got == row.offsetWords.map(func(w): return int(w)), "%s: the tile's offset words are %s, not %s" % [what, got, row.offsetWords])
		got = [out[o + 2], out[o + 3], out[o + 4]]
		check(got == row.hashWords.map(func(w): return int(w)), "%s: the hash words are %s, not %s" % [what, got, row.hashWords])
		for k in 2:
			var ulps := absi(out[o + 5 + k] - int(row.fellBits[k]))
			worst = maxi(worst, ulps)
			check(ulps <= FELL_ULPS, "%s: it falls %d ulps from the browser's point %s" % [what, ulps, row.fell])
		check((out[o + 7] == 1) == row.admitted, "%s: admitted is %s, not %s" % [what, out[o + 7] == 1, row.admitted])
		admitted += int(row.admitted)
	check(admitted > 0 and admitted < rows.size(), "the vectors hold both admitted and refused candidates")
	print("grass sampling: %d candidates, fall points at most %d ulps from the browser's" % [rows.size(), worst])
	rd.free_rid(shader)
	rd.free_rid(texture)
	rd.free_rid(sampler)
	for binding in buffers:
		rd.free_rid(buffers[binding])
	rd.free()
	layer.free()
	return true
