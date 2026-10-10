extends RefCounted

## One full-screen compute pass on Godot's rendering device: GLSL source in,
## a pipeline out, dispatched over a target size in 8×8 groups. The screen
## effects (haze, post) are built from these. Its static helpers are every
## native compute shader's: compiling, and binding images, sampled textures
## and storage buffers.

const GROUP := 8

var shader: RID
var pipeline: RID

## Compiles `body` (everything after the `#version` and workgroup lines).
func _init(rd: RenderingDevice, body: String, label: String) -> void:
	shader = compile(rd, "#version 450\nlayout(local_size_x = %d, local_size_y = %d, local_size_z = 1) in;\n%s" % [GROUP, GROUP, body], label)
	if shader.is_valid():
		pipeline = rd.compute_pipeline_create(shader)

func ready() -> bool:
	return pipeline.is_valid()

## Runs the pass over `size` texels with `uniforms` in set 0 and `push` as its
## push constant (a multiple of 16 bytes).
func dispatch(rd: RenderingDevice, uniforms: Array, push: PackedFloat32Array, size: Vector2i) -> void:
	var list := rd.compute_list_begin()
	rd.compute_list_bind_compute_pipeline(list, pipeline)
	rd.compute_list_bind_uniform_set(list, UniformSetCacheRD.get_cache(shader, 0, uniforms), 0)
	var bytes := push.to_byte_array()
	rd.compute_list_set_push_constant(list, bytes, bytes.size())
	rd.compute_list_dispatch(list, ceili(size.x / float(GROUP)), ceili(size.y / float(GROUP)), 1)
	rd.compute_list_end()

func free_on(rd: RenderingDevice) -> void:
	if shader.is_valid():
		rd.free_rid(shader)

## Compiles a GLSL compute shader on `rd`, or returns an invalid RID. A compile
## error is pushed as an engine error, which the parity run turns into a
## blocker.
static func compile(rd: RenderingDevice, source: String, label: String) -> RID:
	var shader_source := RDShaderSource.new()
	shader_source.language = RenderingDevice.SHADER_LANGUAGE_GLSL
	shader_source.source_compute = source
	var spirv := rd.shader_compile_spirv_from_source(shader_source)
	if not spirv.compile_error_compute.is_empty():
		push_error("%s does not compile: %s" % [label, spirv.compile_error_compute])
		return RID()
	return rd.shader_create_from_spirv(spirv, label)

static func image(binding: int, texture: RID) -> RDUniform:
	var uniform := RDUniform.new()
	uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
	uniform.binding = binding
	uniform.add_id(texture)
	return uniform

static func sampled(binding: int, sampler: RID, texture: RID) -> RDUniform:
	var uniform := RDUniform.new()
	uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
	uniform.binding = binding
	uniform.add_id(sampler)
	uniform.add_id(texture)
	return uniform

static func storage(binding: int, buffer: RID) -> RDUniform:
	var uniform := RDUniform.new()
	uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	uniform.binding = binding
	uniform.add_id(buffer)
	return uniform

static func sampler(rd: RenderingDevice, filter: RenderingDevice.SamplerFilter) -> RID:
	var state := RDSamplerState.new()
	state.min_filter = filter
	state.mag_filter = filter
	state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	return rd.sampler_create(state)
