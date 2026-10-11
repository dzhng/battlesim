extends "res://tests/check.gd"

## Display-backed: native's water surface formulas give the browser's. For
## fixed points, pixel footprints and water depths under the fixture's terrain
## params, `water_look` and `water_normal` (`terrain/ground.gdshaderinc`) agree
## with the browser's `waterSurface` and `waterNormal` run on WebGPU
## (`fixtures/water_vectors.json`, written by
## `throwaway/slice16/water-vectors-gpu.ts`). Neither reel map has water, so
## the depth `groundWater` reads from a map's rivers is fed in on both sides;
## no plate pair judges water.

const ParityRun = preload("res://parity_run.gd")
const GrassLayer = preload("res://grass/grass_layer.gd")
const ComputePass = preload("res://core/compute_pass.gd")
const ScarTexture = preload("res://terrain/scar_texture.gd")
const WordTables = preload("res://core/word_tables.gd")

## Where the grass build's entry point starts: the probe replaces it.
const ENTRY := "layout(local_size_x = 64) in;"
## The two GPU compilers round f32 maths differently (and the browser splices
## some reciprocals in as constants): a few ulps of a value near 1.
const TOLERANCE := 2e-6

func run() -> bool:
	check(DisplayServer.get_name() != "headless", "the test needs a display")
	var vectors: Dictionary = read_json("res://tests/fixtures/water_vectors.json")
	var fixture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json"))
	var ground := ParityRun.read_ground(FIXTURE_RUN.path_join(FIXTURE_SCENE), fixture)
	# The grass build's source carries the ground formulas with every uniform
	# bound for compute.
	var layer = GrassLayer.new(ground, fixture.grass, fixture.tuning, fixture.lens, ScarTexture.new(ground.tables.scars).texture)
	var built: Dictionary = layer.build_source()
	check(built.ok, "the ground formulas assemble: %s" % built.get("detail", ""))
	var source: String = built.source
	var entry := source.find(ENTRY)
	check(entry > 0, "the build's source has its entry point")
	var rows: Array = vectors.rows
	var inputs := PackedFloat32Array()
	for row in rows:
		inputs.append_array(row.input)
	source = source.substr(0, entry) + """
layout(local_size_x = 1) in;
layout(set = 0, binding = 15, std430) restrict readonly buffer Inputs { vec4 inputs[]; };
layout(set = 0, binding = 16, std430) restrict writeonly buffer Outputs { vec4 outputs[]; };
void main() {
	uint at = gl_GlobalInvocationID.x;
	vec4 p = inputs[at];
	outputs[at * 2u] = water_look(p.xy, p.z, p.w);
	outputs[at * 2u + 1u] = vec4(water_normal(p.xy, p.z), 0.0);
}
"""
	var rd := RenderingServer.create_local_rendering_device()
	var shader := ComputePass.compile(rd, source, "water vectors probe")
	check(shader.is_valid(), "the probe compiles against the ground formulas")
	if not shader.is_valid():
		rd.free()
		layer.free()
		return true
	var params := WordTables.vec4s(ground.tables.params).to_byte_array()
	var buffers := {
		11: rd.storage_buffer_create(params.size(), params),
		15: rd.storage_buffer_create(inputs.size() * 4, inputs.to_byte_array()),
		16: rd.storage_buffer_create(rows.size() * 32),
	}
	# The build's other bindings: the probe reads none of them.
	for binding in [0, 1, 2, 3, 4, 5, 6, 12, 14]:
		buffers[binding] = rd.storage_buffer_create(16, PackedByteArray([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))
	var format := RDTextureFormat.new()
	format.format = RenderingDevice.DATA_FORMAT_R8G8B8A8_UNORM
	format.usage_bits = RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
	var texture := rd.texture_create(format, RDTextureView.new(), [PackedByteArray([0, 0, 0, 0])])
	var sampler := rd.sampler_create(RDSamplerState.new())
	var uniforms: Array[RDUniform] = []
	for binding in [7, 8, 9, 10, 13]:
		uniforms.append(ComputePass.sampled(binding, sampler, texture))
	for binding in buffers:
		uniforms.append(ComputePass.storage(binding, buffers[binding]))
	var pipeline := rd.compute_pipeline_create(shader)
	var set := rd.uniform_set_create(uniforms, shader, 0)
	var list := rd.compute_list_begin()
	rd.compute_list_bind_compute_pipeline(list, pipeline)
	rd.compute_list_bind_uniform_set(list, set, 0)
	rd.compute_list_dispatch(list, rows.size(), 1, 1)
	rd.compute_list_end()
	rd.submit()
	rd.sync()
	var out := rd.buffer_get_data(buffers[16]).to_float32_array()
	var worst := 0.0
	var shown := 0
	for i in rows.size():
		var row: Dictionary = rows[i]
		var expected: Array = row.surface + row.normal
		var got := [out[i * 8], out[i * 8 + 1], out[i * 8 + 2], out[i * 8 + 3], out[i * 8 + 4], out[i * 8 + 5], out[i * 8 + 6]]
		for k in expected.size():
			var off := absf(got[k] - float(expected[k]))
			worst = maxf(worst, off)
			check(off <= TOLERANCE, "point %s: %s is %s, not the browser's %s" % [row.input, ["r", "g", "b", "alpha", "normal x", "normal y", "normal z"][k], got[k], expected[k]])
		shown += int(float(row.surface[3]) > 0.0)
	check(shown > 0 and shown < rows.size(), "the vectors hold water and no water")
	print("water: %d points, at most %s from the browser's" % [rows.size(), worst])
	rd.free_rid(shader)
	rd.free_rid(texture)
	rd.free_rid(sampler)
	for binding in buffers:
		rd.free_rid(buffers[binding])
	rd.free()
	layer.free()
	return true
