extends CompositorEffect

## The browser's effect pass (`packages/battle-renderer/src/effects/effectPass.ts`),
## ported as a raster pass on Godot's rendering device: it draws the frame's
## effect instances (`effect_frame.gd`, the browser's 16-float layout in sim
## space) into the resolved, hazed world, before the post chain, as the
## browser draws them after its world pass and before post. Four shapes: a
## streak (tracers, flash tongues, sparks; a lit smoke ribbon when its colour
## carries an opacity), a glow sprite (a solid disc with an opacity), a
## flipbook sprite from the effect atlas (fire carries its own colour; smoke
## and dust an albedo lit by the sun, the sky and the cast lights), and a
## rotor blur. Glows and light streaks add light; ribbons, discs, rotors and
## flipbooks blend over, premultiplied. Each tests against the world's depth
## by reading it and fades into what it meets.
##
## The sky light a lit sprite takes is the sky's radiance averaged over the
## hemisphere about its top and its side, cosine-weighted, times the fill;
## the browser reads its prefiltered environment at full roughness there
## (reported as an approximation).

const ComputePass = preload("res://core/compute_pass.gd")
const Coords = preload("res://core/coords.gd")
const SkyModel = preload("res://environment/sky_model.gd")
const SkyLut = preload("res://environment/sky_lut.gd")
const Light = preload("res://environment/light.gd")
const ShaderText = preload("res://core/shader_text.gd")
const StrictJson = preload("res://core/strict_json.gd")
const Staging = preload("res://assets/staging.gd")
const EffectFrame = preload("res://effects/effect_frame.gd")

## The effect atlas's sheets, one layer each in `EffectFrame` layer order
## (`flipbooks.ts`): a fireball burning out, and a lit puff turning.
const FLIPBOOKS := ["assets/third-party/effects/explosion00_5x5.png", "assets/third-party/effects/cloud01_8x8.png"]
const FLIPBOOK_SIZE := 1024
## Lights the effects' smoke takes, as the world's (`CAST_LIGHTS_MAX`).
const CAST_LIGHTS_MAX := 24
## How fast a cast light falls off inside its radius (`CAST_FALLOFF`).
const CAST_FALLOFF := 6.0
## Floats in the frame uniform.
const FRAME_FLOATS := 48

## The camera's near and far planes, set each frame. (A reverse-Z
## `Projection` doesn't report its planes.)
var near := 1.0
var far := 1.0

var _light: Dictionary
var _sky_lut: SkyLut
var _instances := PackedFloat32Array()
var _count := 0
var _casts := PackedFloat32Array()
var _capacity: int
var _shader: RID
var _pipeline: RID
var _format := -1
var _framebuffers := {}
var _owned: Array[RID] = []
var _instance_buffer: RID
var _frame_buffer: RID
var _cast_buffer: RID
var _atlas: RID
var _linear: RID
var _nearest: RID
var _error := ""

## `light` is the manifest's, `sky_lut` the baked sky the smoke is lit by,
## `capacity` the instances a frame holds.
func _init(light: Dictionary, sky_lut: SkyLut, capacity: int) -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT
	access_resolved_color = true
	access_resolved_depth = true
	_light = light
	_sky_lut = sky_lut
	_capacity = capacity
	_casts.resize(4 + CAST_LIGHTS_MAX * 8)

## A blocker the pass met building its resources, or "".
func error() -> String:
	return _error

## The frame's instances (`count` of them in `data`) and its packed cast
## lights (`cast_lights.gd`, sim space), for the next draw.
func set_instances(data: PackedFloat32Array, count: int, casts: PackedFloat32Array) -> void:
	_instances = data
	_count = count
	_casts = casts

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE:
		var rd := RenderingServer.get_rendering_device()
		if rd == null:
			return
		# Dependents first: the pipeline before its shader.
		for k in range(_owned.size() - 1, -1, -1):
			if _owned[k].is_valid() and not (_owned[k] == _pipeline and not _shader.is_valid()):
				rd.free_rid(_owned[k])
		for framebuffer in _framebuffers.values():
			if rd.framebuffer_is_valid(framebuffer):
				rd.free_rid(framebuffer)

func _render_callback(_type: int, render_data: RenderData) -> void:
	if _count == 0 or not _error.is_empty():
		return
	var rd := RenderingServer.get_rendering_device()
	if not _shader.is_valid() and not _build(rd):
		return
	var buffers: RenderSceneBuffersRD = render_data.get_render_scene_buffers()
	var scene: RenderSceneData = render_data.get_render_scene_data()
	var size := buffers.get_internal_size()
	var bytes := _instances.slice(0, _count * EffectFrame.FLOATS).to_byte_array()
	rd.buffer_update(_instance_buffer, 0, bytes.size(), bytes)
	var casts := _casts.to_byte_array()
	rd.buffer_update(_cast_buffer, 0, casts.size(), casts)
	var frame := _frame(scene, size).to_byte_array()
	rd.buffer_update(_frame_buffer, 0, frame.size(), frame)
	for view in buffers.get_view_count():
		var color := buffers.get_color_layer(view)
		var framebuffer: RID = _framebuffers.get(color, RID())
		if not framebuffer.is_valid() or not rd.framebuffer_is_valid(framebuffer):
			framebuffer = rd.framebuffer_create([color])
			_framebuffers[color] = framebuffer
		if _format < 0:
			_format = rd.framebuffer_get_format(framebuffer)
			_pipeline = _own(rd.render_pipeline_create(_shader, _format, RenderingDevice.INVALID_FORMAT_ID, RenderingDevice.RENDER_PRIMITIVE_TRIANGLES, RDPipelineRasterizationState.new(), RDPipelineMultisampleState.new(), RDPipelineDepthStencilState.new(), _blend()))
		var uniforms := [
			ComputePass.storage(0, _instance_buffer),
			_uniform_buffer(1, _frame_buffer),
			ComputePass.sampled(2, _nearest, buffers.get_depth_layer(view)),
			ComputePass.sampled(3, _linear, _atlas),
			ComputePass.sampled(4, _sky_lut.sampler, _sky_lut.rid),
			ComputePass.storage(5, _cast_buffer),
		]
		rd.draw_command_begin_label("effects", Color.WHITE)
		var list := rd.draw_list_begin(framebuffer)
		rd.draw_list_bind_render_pipeline(list, _pipeline)
		rd.draw_list_bind_uniform_set(list, UniformSetCacheRD.get_cache(_shader, 0, uniforms), 0)
		rd.draw_list_draw(list, false, _count, 6)
		rd.draw_list_end()
		rd.draw_command_end_label()

## The frame's uniform: the sim-to-clip matrix, the canvas and planes, the
## projection's scale, the camera's frame in sim space, and the sun.
func _frame(scene: RenderSceneData, size: Vector2i) -> PackedFloat32Array:
	var view := scene.get_cam_transform()
	var projection := scene.get_cam_projection()
	# Godot's projection already turns y down for the device; the browser's
	# screen arithmetic runs y up, as `emit` turns it back.
	var flip := Projection(Vector4(1, 0, 0, 0), Vector4(0, -1, 0, 0), Vector4(0, 0, 1, 0), Vector4(0, 0, 0, 1))
	var sim_to_clip := flip * projection * Projection(view.affine_inverse() * Transform3D(Coords.TO_GODOT, Vector3.ZERO))
	var out := PackedFloat32Array()
	for c in 4:
		var column: Vector4 = sim_to_clip[c]
		out.append_array([column.x, column.y, column.z, column.w])
	var right := Coords.godot_to_sim(view.basis.x).normalized()
	var up := Coords.godot_to_sim(view.basis.y).normalized()
	var toward := right.cross(up)
	var sun := Light.sun_direction(_light)
	var radiance := Light.sun_color(_light) * float(_light.sun_intensity)
	var fill := StrictJson.vector3(_light.sky.fill)
	out.append_array([size.x, size.y, near, far])
	out.append_array([absf(projection.x.x), absf(projection.y.y), 0.0, 0.0])
	for v in [right, up, toward, Vector3(sun.dot(right), sun.dot(up), sun.dot(toward)), radiance, fill]:
		out.append_array([v.x, v.y, v.z, 0.0])
	return out

func _build(rd: RenderingDevice) -> bool:
	var source := RDShaderSource.new()
	source.language = RenderingDevice.SHADER_LANGUAGE_GLSL
	var common := COMMON.replace("{sky}", SkyModel.lookup_function()).replace("{falloff}", ShaderText.number(CAST_FALLOFF))
	source.source_vertex = "#version 450\n" + common + VERTEX
	source.source_fragment = "#version 450\n" + common + FRAGMENT
	var spirv := rd.shader_compile_spirv_from_source(source)
	if not spirv.compile_error_vertex.is_empty() or not spirv.compile_error_fragment.is_empty():
		_error = "the effect pass does not compile: %s %s" % [spirv.compile_error_vertex, spirv.compile_error_fragment]
		push_error(_error)
		return false
	_shader = _own(rd.shader_create_from_spirv(spirv, "effects"))
	_instance_buffer = _own(rd.storage_buffer_create(_capacity * EffectFrame.FLOATS * 4))
	_frame_buffer = _own(rd.uniform_buffer_create(FRAME_FLOATS * 4))
	_cast_buffer = _own(rd.storage_buffer_create((4 + CAST_LIGHTS_MAX * 8) * 4))
	_linear = _own(_sampler(rd, RenderingDevice.SAMPLER_FILTER_LINEAR))
	_nearest = _own(_sampler(rd, RenderingDevice.SAMPLER_FILTER_NEAREST))
	_atlas = _own(_load_atlas(rd))
	return _atlas.is_valid()

## Every flipbook as a layer of an sRGB 2D array with its mip chain,
## premultiplied, as the browser's atlas holds them.
func _load_atlas(rd: RenderingDevice) -> RID:
	var layers: Array[PackedByteArray] = []
	var mips := 0
	for path in FLIPBOOKS:
		var image := Image.load_from_file(Staging.repo_path(path))
		if image == null or image.get_width() != FLIPBOOK_SIZE or image.get_height() != FLIPBOOK_SIZE:
			_error = "effect flipbook %s is not a %d² image (an LFS pointer?)" % [path, FLIPBOOK_SIZE]
			push_error(_error)
			return RID()
		image.convert(Image.FORMAT_RGBA8)
		image.premultiply_alpha()
		image.generate_mipmaps()
		mips = image.get_mipmap_count() + 1
		layers.append(image.get_data())
	var format := RDTextureFormat.new()
	format.texture_type = RenderingDevice.TEXTURE_TYPE_2D_ARRAY
	format.width = FLIPBOOK_SIZE
	format.height = FLIPBOOK_SIZE
	format.array_layers = FLIPBOOKS.size()
	format.mipmaps = mips
	format.format = RenderingDevice.DATA_FORMAT_R8G8B8A8_SRGB
	format.usage_bits = RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT | RenderingDevice.TEXTURE_USAGE_CAN_UPDATE_BIT
	return rd.texture_create(format, RDTextureView.new(), layers)

func _blend() -> RDPipelineColorBlendState:
	var attachment := RDPipelineColorBlendStateAttachment.new()
	attachment.enable_blend = true
	attachment.src_color_blend_factor = RenderingDevice.BLEND_FACTOR_ONE
	attachment.dst_color_blend_factor = RenderingDevice.BLEND_FACTOR_ONE_MINUS_SRC_ALPHA
	attachment.color_blend_op = RenderingDevice.BLEND_OP_ADD
	# The lit world's alpha is kept.
	attachment.src_alpha_blend_factor = RenderingDevice.BLEND_FACTOR_ZERO
	attachment.dst_alpha_blend_factor = RenderingDevice.BLEND_FACTOR_ONE
	attachment.alpha_blend_op = RenderingDevice.BLEND_OP_ADD
	var state := RDPipelineColorBlendState.new()
	state.attachments = [attachment]
	return state

static func _sampler(rd: RenderingDevice, filter: RenderingDevice.SamplerFilter) -> RID:
	var state := RDSamplerState.new()
	state.min_filter = filter
	state.mag_filter = filter
	state.mip_filter = filter
	state.repeat_u = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	state.repeat_v = RenderingDevice.SAMPLER_REPEAT_MODE_CLAMP_TO_EDGE
	return rd.sampler_create(state)

static func _uniform_buffer(binding: int, buffer: RID) -> RDUniform:
	var uniform := RDUniform.new()
	uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_UNIFORM_BUFFER
	uniform.binding = binding
	uniform.add_id(buffer)
	return uniform

func _own(rid: RID) -> RID:
	_owned.append(rid)
	return rid

## Shared by both stages: the bindings, the sky lookup and the cast lights.
const COMMON := """
struct Instance { vec4 a; vec4 b; vec4 color; vec4 misc; };
layout(set = 0, binding = 0, std430) restrict readonly buffer Instances { Instance instances[]; };
layout(set = 0, binding = 1, std140) uniform Frame {
	mat4 view_proj;   // sim space to clip (GL-style)
	vec4 canvas;      // width, height, near, far
	vec4 scale;       // clip units per metre at unit depth, across and up
	vec4 right;       // the camera's right, up and toward the eye, sim space
	vec4 up;
	vec4 toward;
	vec4 sun;         // the sun in a sprite's frame (right, up, toward)
	vec4 sun_radiance;
	vec4 fill;
} frame;
layout(set = 0, binding = 2) uniform sampler2D scene_depth;
layout(set = 0, binding = 3) uniform sampler2DArray atlas;
layout(set = 0, binding = 4) uniform sampler2D sky_lut;
layout(set = 0, binding = 5, std430) restrict readonly buffer Casts { vec4 header; vec4 lights[]; } casts;

{sky}

const float PI = 3.14159265;

// The cast lights' irradiance at `p` in smoke, over pi: each one's falloff,
// with no facing (light scatters through smoke).
vec3 cast_at(vec3 p) {
	vec3 sum = vec3(0.0);
	int count = int(casts.header.x);
	for (int i = 0; i < count; i++) {
		vec4 at = casts.lights[i * 2];
		vec3 toward = at.xyz - p;
		float x2 = dot(toward, toward) * at.w;
		if (x2 < 1.0) {
			sum += casts.lights[i * 2 + 1].rgb * (1.0 - x2) * (1.0 - x2) / (1.0 + {falloff} * x2);
		}
	}
	return sum * 0.31830988;
}
"""

const VERTEX := """
layout(location = 0) out vec2 v_uv;
layout(location = 1) out vec4 v_color;
layout(location = 2) flat out vec4 v_misc;
layout(location = 3) flat out vec4 v_extra;
layout(location = 4) out float v_along;
layout(location = 5) out float v_view_depth;
layout(location = 6) flat out vec3 v_sun;
layout(location = 7) out vec2 v_offset;
layout(location = 8) flat out vec3 v_sky_top;
layout(location = 9) flat out vec3 v_sky_side;
layout(location = 10) out vec3 v_cast;

const float NEAR_W = 0.05;
const float ROTOR_SOFT_M = 0.05;

// The sky's radiance about `axis`, cosine-weighted over its hemisphere.
vec3 sky_about(vec3 axis) {
	vec3 t = normalize(abs(axis.z) < 0.9 ? cross(axis, vec3(0.0, 0.0, 1.0)) : cross(axis, vec3(1.0, 0.0, 0.0)));
	vec3 b = cross(axis, t);
	vec3 sum = vec3(0.0);
	const int TAPS = 32;
	for (int i = 0; i < TAPS; i++) {
		float u = (float(i) + 0.5) / float(TAPS);
		float r = sqrt(u);
		float a = float(i) * 2.39996323;
		vec3 dir = normalize(t * (r * cos(a)) + b * (r * sin(a)) + axis * sqrt(max(0.0, 1.0 - u)));
		sum += sky_radiance(dir);
	}
	return sum / float(TAPS) * frame.fill.rgb;
}

void sprite_light() {
	v_sun = frame.sun.xyz;
	vec3 side = normalize(vec3(frame.toward.x, frame.toward.y, 0.0) + vec3(0.0, 0.0, 1e-3));
	v_sky_top = sky_about(vec3(0.0, 0.0, 1.0));
	v_sky_side = sky_about(side);
}

void emit(vec4 clip) {
	// Back to the device's clip space, whose y runs down; nothing tests depth,
	// so z stays inside.
	gl_Position = vec4(clip.x, -clip.y, 0.5 * clip.w, clip.w);
}

void main() {
	vec2 corners[6] = vec2[](vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0));
	Instance v = instances[gl_InstanceIndex];
	vec2 c = corners[gl_VertexIndex];
	vec2 half_size = frame.canvas.xy * 0.5;
	vec2 s = frame.scale.xy;
	v_uv = c;
	v_color = v.color;
	v_misc = v.misc;
	v_extra = vec4(0.0);
	v_along = 0.0;
	v_view_depth = 1.0;
	v_sun = vec3(0.0, 0.0, 1.0);
	v_sky_top = vec3(0.0);
	v_sky_side = vec3(0.0);
	v_cast = vec3(0.0);
	v_offset = c;
	uint shape = uint(v.misc.x);
	if (shape == 0u) {
		vec4 ca = frame.view_proj * vec4(v.a.xyz, 1.0);
		vec4 cb = frame.view_proj * vec4(v.b.xyz, 1.0);
		if (ca.w < NEAR_W && cb.w < NEAR_W) {
			gl_Position = vec4(0.0, 0.0, -2.0, 1.0);
			return;
		}
		float along_a = v.misc.y;
		float along_b = v.misc.z;
		if (ca.w < NEAR_W) {
			float t = (NEAR_W - ca.w) / (cb.w - ca.w);
			ca = mix(ca, cb, t);
			along_a = mix(along_a, along_b, t);
		}
		if (cb.w < NEAR_W) {
			float t = (NEAR_W - cb.w) / (ca.w - cb.w);
			cb = mix(cb, ca, t);
			along_b = mix(along_b, along_a, t);
		}
		vec2 sa = ca.xy / ca.w * half_size;
		vec2 sb = cb.xy / cb.w * half_size;
		vec2 d = sb - sa;
		float len = length(d);
		vec2 dir = len > 1e-4 ? d / max(len, 1e-6) : vec2(1.0, 0.0);
		vec2 perp = vec2(-dir.y, dir.x);
		vec4 end = c.x > 0.0 ? cb : ca;
		bool fixed_line = v.a.w < 0.0;
		float true_px = fixed_line ? -v.a.w : v.a.w * s.y * half_size.y / end.w;
		float px = fixed_line ? true_px + 2.0 : max(true_px, v.b.w);
		float coverage = min(1.0, true_px / px);
		float boost = abs(v.misc.w);
		float k = boost > 1.0 ? mix(boost, 1.0, coverage) : coverage;
		float energy = fixed_line ? 1.0 : k;
		bool ribbon = v.color.a > 0.0;
		float overhang = ribbon ? 0.0 : 0.25;
		vec2 off = (perp * c.y + dir * c.x * overhang * 2.0) * px * 0.5;
		emit(vec4(end.xy + off / half_size * end.w, end.zw));
		v_color = ribbon ? v.color * energy : vec4(v.color.rgb * energy, 0.0);
		v_along = c.x > 0.0 ? along_b : along_a;
		v_view_depth = end.w;
		v_extra = vec4(len * 0.5 + px * overhang, px * overhang, v.a.w * 0.5, fixed_line ? true_px : 0.0);
		if (ribbon) {
			v_offset = perp * c.y;
			sprite_light();
			v_cast = cast_at(c.x > 0.0 ? v.b.xyz : v.a.xyz);
		}
		return;
	}
	if (shape == 3u) {
		// A rotor: the quad round its hub in the plane across its axis, its
		// first blade along +x of that plane.
		vec3 axis = normalize(v.b.xyz);
		vec3 u = normalize(v.color.xyz - axis * dot(v.color.xyz, axis));
		vec3 w = cross(axis, u);
		vec4 clip = frame.view_proj * vec4(v.a.xyz + (u * c.x + w * c.y) * v.a.w, 1.0);
		if (clip.w < NEAR_W) {
			gl_Position = vec4(0.0, 0.0, -2.0, 1.0);
			return;
		}
		emit(clip);
		v_view_depth = clip.w;
		v_color = vec4(vec3(v.color.w), 1.0);
		v_extra = vec4(v.b.w, v.misc.y, v.misc.z, ROTOR_SOFT_M);
		sprite_light();
		v_cast = cast_at(v.a.xyz);
		return;
	}
	vec4 clip = frame.view_proj * vec4(v.a.xyz, 1.0);
	if (clip.w < NEAR_W) {
		gl_Position = vec4(0.0, 0.0, -2.0, 1.0);
		return;
	}
	float cr = cos(v.b.x);
	float sr = sin(v.b.x);
	vec2 turned = vec2(c.x * cr - c.y * sr, c.x * sr + c.y * cr);
	float true_px = v.a.w * s.y * half_size.y / clip.w;
	float min_px = shape == 1u ? v.b.y : 0.0;
	float px = max(true_px, min_px);
	float radius = v.a.w * px / max(true_px, 1e-6);
	emit(vec4(clip.xy + turned * radius * s, clip.zw));
	if (shape == 1u) {
		// A glow dims as the square of its shrink below the least size; a
		// solid disc (opacity in alpha) thins its coverage with it.
		float k = true_px / px;
		v_color = v.color.a > 0.0 ? v.color * k : vec4(v.color.rgb * k * k, 0.0);
	}
	v_extra = vec4(v.b.y, v.b.z, v.b.w, 0.0);
	v_view_depth = clip.w;
	v_offset = turned;
	if (shape == 2u && v.misc.z > 0.5) {
		sprite_light();
		v_cast = cast_at(v.a.xyz);
	}
}
"""

const FRAGMENT := """
layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec4 v_color;
layout(location = 2) flat in vec4 v_misc;
layout(location = 3) flat in vec4 v_extra;
layout(location = 4) in float v_along;
layout(location = 5) in float v_view_depth;
layout(location = 6) flat in vec3 v_sun;
layout(location = 7) in vec2 v_offset;
layout(location = 8) flat in vec3 v_sky_top;
layout(location = 9) flat in vec3 v_sky_side;
layout(location = 10) in vec3 v_cast;
layout(location = 0) out vec4 out_color;

const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);

// The world's light on smoke at `o` on a lit sprite (-1..1, screen aligned):
// a soft ball's (or across a ribbon, a tube's) normal against the sun,
// wrapped, the sun scattered on toward the eye, and the sky's light.
vec3 smoke_light(vec2 o) {
	vec3 n = normalize(vec3(o, sqrt(max(0.0, 1.0 - dot(o, o))) + 0.3));
	float wrap = clamp((dot(n, v_sun) + 0.3) / 1.3, 0.0, 1.0);
	float g = 0.4;
	float forward = (1.0 - g * g) / pow(1.0 + g * g + 2.0 * g * v_sun.z, 1.5);
	vec3 sky = mix(v_sky_side, v_sky_top, clamp(n.y * 0.5 + 0.5, 0.0, 1.0));
	return frame.sun_radiance.rgb * ((0.6 * wrap + 0.4 * forward) / PI) + sky + v_cast;
}

vec2 cell(vec2 tuv, float frame_index, float cols) {
	vec2 at = vec2(mod(frame_index, cols), floor(frame_index / cols));
	return (at + tuv) / cols;
}

// WGSL's smoothstep, defined at its edges.
float smooth_step(float e0, float e1, float x) {
	float t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}

void main() {
	vec2 tuv = clamp(vec2(v_uv.x, -v_uv.y) * 0.5 + 0.5, vec2(0.004), vec2(0.996));
	vec2 gx = dFdx(tuv);
	vec2 gy = dFdy(tuv);
	vec2 offset_width = fwidth(v_offset);
	uint shape = uint(v_misc.x);
	float raw = texelFetch(scene_depth, ivec2(gl_FragCoord.xy), 0).r;
	float near = frame.canvas.z;
	float far = frame.canvas.w;
	float scene = raw > 0.0 ? far * near / (raw * (far - near) + near) : 1e9;
	float gap = scene - v_view_depth;
	if (gap < 0.0) {
		discard;
	}
	vec3 rgb = vec3(0.0);
	float alpha = 0.0;
	if (shape == 0u && v_color.a > 0.0) {
		// A smoke ribbon: thick through its middle, thinning to its edges.
		float across = v_uv.y * v_uv.y;
		float cover = exp(-across * 2.0) * (1.0 - across) * clamp(gap / max(v_extra.z, 1e-3), 0.0, 1.0);
		alpha = v_color.a * cover;
		rgb = v_color.rgb * smoke_light(v_offset) * alpha;
	} else if (shape == 0u) {
		float across = v_uv.y * v_uv.y;
		float a = clamp(v_along, 0.0, 1.0);
		float from_end = (1.0 - abs(v_uv.x)) * v_extra.x;
		float cap = smooth_step(0.0, max(v_extra.y, 1e-3) * 2.0, from_end);
		float sharp = exp(-across * 9.0) + 0.3 * exp(-across * 2.5);
		float soft = exp(-across * 3.0) * (1.0 - across);
		float distance_px = abs(v_uv.y) * (v_extra.w + 2.0) * 0.5;
		float line_coverage = clamp((v_extra.w + 1.0) * 0.5 - distance_px, 0.0, 1.0);
		float profile = v_extra.w > 0.0 ? line_coverage : ((v_misc.w == 1.0 || v_misc.w < 0.0) ? soft : sharp);
		rgb = v_color.rgb * profile * a * a * cap * clamp(gap / 0.1, 0.0, 1.0);
	} else if (shape == 1u && v_color.a > 0.0) {
		// A solid disc: the round seen as an object.
		float cover = (1.0 - smooth_step(0.6, 1.0, length(v_uv))) * clamp(gap / 0.1, 0.0, 1.0);
		rgb = v_color.rgb * cover;
		alpha = v_color.a * cover;
	} else if (shape == 3u) {
		// A rotor, from its hub (0) to its blade tips (1): each blade covers
		// the wedge it swept, for the share of the exposure it spent there.
		float r = length(v_offset);
		float ang = atan(v_offset.y, v_offset.x);
		float aa = length(offset_width) / max(r, 1e-3);
		int n = int(v_extra.z + 0.5);
		float cover = n == 0 ? v_misc.w : 0.0;
		float chord_ang = max(v_extra.y, v_misc.w * length(offset_width)) / max(r, 1e-3);
		float half_width = (v_extra.x + chord_ang) * 0.5;
		float share = chord_ang / (v_extra.x + chord_ang);
		for (int k = 0; k < n; k++) {
			float mid = 2.0 * PI * float(k) / float(n) - v_extra.x * 0.5;
			float d = abs(atan(sin(ang - mid), cos(ang - mid)));
			cover += (1.0 - smooth_step(half_width - aa, half_width + aa, d)) * share;
		}
		float edge = 1.0 - smooth_step(1.0 - length(offset_width), 1.0, r);
		cover = min(cover, 1.0) * edge * clamp(gap / v_extra.w, 0.0, 1.0);
		alpha = cover;
		rgb = v_color.rgb * smoke_light(vec2(0.0)) * alpha;
	} else if (shape == 1u) {
		float r = length(v_uv);
		float ang = atan(v_uv.y, v_uv.x);
		float rays = pow(abs(cos(ang * 2.0)), 16.0) * exp(-r * 2.5) * 0.6 * v_extra.y;
		float edge = 1.0 - smooth_step(0.75, 1.0, r);
		rgb = v_color.rgb * (exp(-r * r * 8.0) + rays) * edge * clamp(gap / 0.5, 0.0, 1.0);
	} else {
		int layer = int(v_extra.y + 0.5);
		float cols = layer == 1 ? 8.0 : 5.0;
		float frames = cols * cols;
		float frame_index = clamp(v_extra.x, 0.0, frames - 1.0);
		float f0 = floor(frame_index);
		float f1 = min(f0 + 1.0, frames - 1.0);
		vec4 s0 = textureGrad(atlas, vec3(cell(tuv, f0, cols), float(layer)), gx / cols, gy / cols);
		vec4 s1 = textureGrad(atlas, vec3(cell(tuv, f1, cols), float(layer)), gx / cols, gy / cols);
		vec4 tex = mix(s0, s1, frame_index - f0);
		vec3 straight = tex.rgb / max(tex.a, 1e-3);
		float lum = dot(straight, LUMA);
		float opacity = v_color.a * clamp(gap / max(v_extra.z, 1e-3), 0.0, 1.0);
		if (v_misc.z > 0.5) {
			// Lit, the sheet's own relief on top.
			float relief = mix(0.65, 1.25, smooth_step(0.08, 0.45, lum));
			rgb = v_color.rgb * smoke_light(v_offset) * relief * tex.a * opacity;
		} else {
			float boost = 1.0 + v_misc.y * lum * lum;
			rgb = tex.rgb * v_color.rgb * boost * opacity;
		}
		alpha = tex.a * opacity;
	}
	out_color = vec4(rgb, alpha);
}
"""
