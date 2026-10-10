extends CompositorEffect

## The browser's post chain (`world/post.ts`, `shaders/post.ts`), ported by
## formula: a bloom (high-pass at half size, five blurred levels, one
## composite), then the grade and AgX tone mapping with the manifest's
## exposure. It writes linear display colour; Godot's own output step, set to
## its linear tone mapper, applies the same sRGB transfer the browser's post
## ends with. It runs last, after the haze.

const ComputePass = preload("res://environment/compute_pass.gd")
const SkyModel = preload("res://environment/sky_model.gd")
const Light = preload("res://environment/light.gd")

## The bloom's blur radius, in texels of its level, per level (`BLOOM_KERNEL_RADII`).
const BLOOM_KERNEL_RADII := [6, 10, 14, 18, 22]
## Each level's weight in the composite at radius 0; the radius moves it toward
## `1.2 - weight` (`bloomCompositeWgsl`).
const BLOOM_LEVEL_WEIGHTS := [1.0, 0.8, 0.6, 0.4, 0.2]

var _light: Dictionary
var _passes := {}
var _linear: RID
var _size := Vector2i.ZERO
## Owned textures: `bright` and `[horizontal, vertical]` per bloom level.
var _bright: RID
var _levels: Array = []

func _init(light: Dictionary) -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT
	# Under MSAA, read the resolved colour.
	access_resolved_color = true
	_light = light

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE and not _passes.is_empty():
		var rd := RenderingServer.get_rendering_device()
		for texture in [_bright] + _levels.reduce(func(all, pair): return all + pair, []):
			rd.free_rid(texture)
		for pass_ in _passes.values():
			pass_.free_on(rd)
		rd.free_rid(_linear)

func _render_callback(_type: int, render_data: RenderData) -> void:
	var rd := RenderingServer.get_rendering_device()
	if _passes.is_empty():
		_passes = {
			"highpass": ComputePass.new(rd, highpass_source(_light), "bloom high-pass"),
			"blur": ComputePass.new(rd, BLUR_SOURCE, "bloom blur"),
			"composite": ComputePass.new(rd, composite_source(_light), "bloom composite"),
			"final": ComputePass.new(rd, final_source(_light), "grade and tone map"),
		}
		_linear = ComputePass.sampler(rd, RenderingDevice.SAMPLER_FILTER_LINEAR)
	if not _passes.values().all(func(pass_): return pass_.ready()):
		return
	var buffers: RenderSceneBuffersRD = render_data.get_render_scene_buffers()
	var size := buffers.get_internal_size()
	if size != _size:
		_free_targets(rd)
		_make_targets(rd, size)
	for view in buffers.get_view_count():
		var scene := buffers.get_color_layer(view)
		var half := size / 2
		_passes.highpass.dispatch(rd, [ComputePass.image(0, _bright), ComputePass.sampled(1, _linear, scene)], _push([half.x, half.y]), half)
		var source := _bright
		var level_size := half
		for i in BLOOM_KERNEL_RADII.size():
			var pair: Array = _levels[i]
			for axis in 2:
				var delta := Vector2(1.0 / level_size.x, 0.0) if axis == 0 else Vector2(0.0, 1.0 / level_size.y)
				_passes.blur.dispatch(rd, [ComputePass.image(0, pair[axis]), ComputePass.sampled(1, _linear, source)], _push([level_size.x, level_size.y, delta.x, delta.y, BLOOM_KERNEL_RADII[i]]), level_size)
				source = pair[axis]
			level_size /= 2
		# The composite lands in the first level's horizontal target, which
		# nothing reads any more.
		var composite := [ComputePass.image(0, _levels[0][0])]
		for i in BLOOM_KERNEL_RADII.size():
			composite.append(ComputePass.sampled(i + 1, _linear, _levels[i][1]))
		_passes.composite.dispatch(rd, composite, _push([half.x, half.y]), half)
		_passes.final.dispatch(rd, [ComputePass.image(0, scene), ComputePass.sampled(1, _linear, _levels[0][0])], _push([size.x, size.y]), size)

## `values` padded to whole 16-byte rows.
static func _push(values: Array) -> PackedFloat32Array:
	var push := PackedFloat32Array(values)
	while push.size() % 4 != 0:
		push.append(0.0)
	return push

func _make_targets(rd: RenderingDevice, size: Vector2i) -> void:
	_size = size
	var level_size := size / 2
	_bright = _target(rd, level_size)
	for _radius in BLOOM_KERNEL_RADII:
		_levels.append([_target(rd, level_size), _target(rd, level_size)])
		level_size /= 2

static func _target(rd: RenderingDevice, size: Vector2i) -> RID:
	var format := RDTextureFormat.new()
	format.format = RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT
	format.width = maxi(size.x, 1)
	format.height = maxi(size.y, 1)
	format.usage_bits = RenderingDevice.TEXTURE_USAGE_STORAGE_BIT | RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
	return rd.texture_create(format, RDTextureView.new())

func _free_targets(rd: RenderingDevice) -> void:
	if _bright.is_valid():
		rd.free_rid(_bright)
	for pair in _levels:
		for texture in pair:
			rd.free_rid(texture)
	_bright = RID()
	_levels = []
	_size = Vector2i.ZERO

const TARGET := """
layout(rgba16f, set = 0, binding = 0) uniform restrict writeonly image2D target;
"""

## Keeps what is brighter than the threshold, fading in over the smooth width.
static func highpass_source(light: Dictionary) -> String:
	var bloom: Dictionary = light.bloom
	return TARGET + """
layout(set = 0, binding = 1) uniform sampler2D source;
layout(push_constant, std430) uniform Frame { vec4 size; } frame;
void main() {
	ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
	if (pixel.x >= int(frame.size.x) || pixel.y >= int(frame.size.y)) {
		return;
	}
	vec4 c = textureLod(source, (vec2(pixel) + 0.5) / frame.size.xy, 0.0);
	float weight = smoothstep({threshold}, {threshold_end}, dot(c.rgb, vec3(0.2126, 0.7152, 0.0722)));
	imageStore(target, pixel, mix(vec4(0.0), c, weight));
}
""".format({"threshold": SkyModel.f(float(bloom.threshold)), "threshold_end": SkyModel.f(float(bloom.threshold) + float(bloom.smooth_width))})

## A separable Gaussian with sigma a third of the radius, one axis at a time.
const BLUR_SOURCE := TARGET + """
layout(set = 0, binding = 1) uniform sampler2D source;
layout(push_constant, std430) uniform Frame { vec4 size_delta; vec4 radius; } frame;
void main() {
	ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
	if (pixel.x >= int(frame.size_delta.x) || pixel.y >= int(frame.size_delta.y)) {
		return;
	}
	vec2 uv = (vec2(pixel) + 0.5) / frame.size_delta.xy;
	int radius = int(frame.radius.x);
	float sigma = float(radius) / 3.0;
	vec3 color = textureLod(source, uv, 0.0).rgb * (0.39894 / sigma);
	for (int i = 1; i < radius; i++) {
		float weight = 0.39894 * exp(-0.5 * float(i * i) / (sigma * sigma)) / sigma;
		vec2 offset = frame.size_delta.zw * float(i);
		color += (textureLod(source, uv + offset, 0.0).rgb + textureLod(source, uv - offset, 0.0).rgb) * weight;
	}
	imageStore(target, pixel, vec4(color, 1.0));
}
"""

## The five levels, weighted by the bloom's radius, times its strength.
static func composite_source(light: Dictionary) -> String:
	var bloom: Dictionary = light.bloom
	var radius := float(bloom.radius)
	var terms: Array[String] = []
	for i in BLOOM_LEVEL_WEIGHTS.size():
		var weight: float = BLOOM_LEVEL_WEIGHTS[i]
		terms.append("textureLod(level%d, uv, 0.0) * %s" % [i, SkyModel.f(weight * (1.0 - radius) + (1.2 - weight) * radius)])
	var bindings := ""
	for i in BLOOM_LEVEL_WEIGHTS.size():
		bindings += "layout(set = 0, binding = %d) uniform sampler2D level%d;\n" % [i + 1, i]
	return TARGET + bindings + """
layout(push_constant, std430) uniform Frame { vec4 size; } frame;
void main() {
	ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
	if (pixel.x >= int(frame.size.x) || pixel.y >= int(frame.size.y)) {
		return;
	}
	vec2 uv = (vec2(pixel) + 0.5) / frame.size.xy;
	imageStore(target, pixel, ({terms}) * {strength});
}
""".format({"terms": " + ".join(terms), "strength": SkyModel.f(float(bloom.strength))})

## The scene plus its bloom, graded and tone mapped by AgX at the exposure,
## in place. Linear display colour out.
static func final_source(light: Dictionary) -> String:
	var grade: Dictionary = light.grade
	return """
layout(rgba16f, set = 0, binding = 0) uniform restrict image2D scene;
layout(set = 0, binding = 1) uniform sampler2D bloom;
layout(push_constant, std430) uniform Frame { vec4 size; } frame;

const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);

vec3 grade_color(vec3 color) {
	vec3 base = max(color, vec3(0.0));
	float base_luma = max(dot(base, LUMA), 0.0001);
	float mapped = base_luma / (base_luma + 1.0);
	float curve = mapped * mapped * (3.0 - mapped * 2.0);
	float l = mix(mapped, curve, {contrast});
	float contrast_luma = l / max(0.0001, 1.0 - l);
	vec3 graded = base * (contrast_luma / base_luma);
	vec3 shadow_tint = mix(vec3(1.0), mix({shadow_tint}, vec3(1.0), smoothstep(0.0, 0.34, l)), {split_tone});
	vec3 highlight_tint = mix(vec3(1.0), mix(vec3(1.0), {highlight_tint}, smoothstep(0.44, 0.98, l)), {split_tone});
	vec3 tinted = graded * shadow_tint * highlight_tint;
	graded = tinted * (dot(graded, LUMA) / max(dot(tinted, LUMA), 0.0001));
	vec3 lift = {lift} * {shadow_lift} * (1.0 - smoothstep(0.08, 0.38, l)) * 0.45;
	graded = graded * (vec3(1.0) - lift) + lift;
	float midtone = smoothstep(0.1, 0.42, l) * (1.0 - smoothstep(0.62, 0.96, l));
	float saturation = 1.0 + {saturation} * (0.55 + midtone * 0.45);
	return max(mix(vec3(dot(graded, LUMA)), graded, saturation), vec3(0.0));
}

// AgX, as pinned from Three 0.185.1 (MIT).
vec3 agx(vec3 color) {
	mat3 to2020 = mat3(vec3(0.6274, 0.0691, 0.0164), vec3(0.3293, 0.9195, 0.0880), vec3(0.0433, 0.0113, 0.8956));
	mat3 inset = mat3(vec3(0.856627153315983, 0.137318972929847, 0.11189821299995), vec3(0.0951212405381588, 0.761241990602591, 0.0767994186031903), vec3(0.0482516061458583, 0.101439036467562, 0.811302368396859));
	mat3 outset = mat3(vec3(1.1271005818144368, -0.1413297634984383, -0.14132976349843826), vec3(-0.11060664309660323, 1.157823702216272, -0.11060664309660294), vec3(-0.016493938717834573, -0.016493938717834257, 1.2519364065950405));
	mat3 to_srgb = mat3(vec3(1.6605, -0.1246, -0.0182), vec3(-0.5876, 1.1329, -0.1006), vec3(-0.0728, -0.0083, 1.1187));
	vec3 x = clamp((log2(max(inset * (to2020 * (color * {exposure})), vec3(1e-10))) + 12.47393) / (4.026069 + 12.47393), vec3(0.0), vec3(1.0));
	vec3 x2 = x * x;
	vec3 x4 = x2 * x2;
	vec3 contrast = 15.5 * (x4 * x2) - 40.14 * (x4 * x) + (31.96 * x4 - 6.868 * (x2 * x) + (0.4298 * x2 + (0.1191 * x - 0.00232)));
	return clamp(to_srgb * pow(max(vec3(0.0), outset * contrast), vec3(2.2)), vec3(0.0), vec3(1.0));
}

void main() {
	ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
	if (pixel.x >= int(frame.size.x) || pixel.y >= int(frame.size.y)) {
		return;
	}
	vec4 hdr = imageLoad(scene, pixel) + textureLod(bloom, (vec2(pixel) + 0.5) / frame.size.xy, 0.0);
	imageStore(scene, pixel, vec4(agx(grade_color(hdr.rgb)), clamp(hdr.a, 0.0, 1.0)));
}
""".format({
		"contrast": SkyModel.f(float(grade.contrast)),
		"shadow_tint": SkyModel.rgb(Light.vector(grade.shadow_tint)),
		"highlight_tint": SkyModel.rgb(Light.vector(grade.highlight_tint)),
		"split_tone": SkyModel.f(float(grade.split_tone)),
		"lift": SkyModel.rgb(Light.vector(grade.lift)),
		"shadow_lift": SkyModel.f(float(grade.shadow_lift)),
		"saturation": SkyModel.f(float(grade.saturation)),
		"exposure": SkyModel.f(float(light.exposure)),
	})
