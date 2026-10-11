extends RefCounted

## The browser's sky lookup (`world/sky.ts`): the clear sky's radiance
## (`SkyModel.radiance_function`, a 32-step integral) baked once into a
## 384×192 equirectangular texture. The shown sky, the sky's light and the
## haze read it through `SkyModel.lookup_function` with linear filtering, as
## the browser's background, environment light and aerial term read its LUT,
## rather than integrating the sky again for every pixel.

const SkyModel = preload("res://environment/sky_model.gd")
const ComputePass = preload("res://core/compute_pass.gd")
const ShaderText = preload("res://core/shader_text.gd")

## `SKY_LUT_WIDTH` and `SKY_LUT_HEIGHT` (`light/skyParameters.ts`).
const WIDTH := 384
const HEIGHT := 192

## The baked texture on the rendering device, for compute passes.
var rid: RID
## The same texture for Godot shaders.
var texture := Texture2DRD.new()
## The linear, edge-clamped sampler a compute pass reads it with.
var sampler: RID

var _rd: RenderingDevice
var _bake: ComputePass

## Bakes `light`'s clear sky before the first frame draws. A headless run
## has no rendering device and draws nothing, so it bakes nothing.
func _init(light: Dictionary) -> void:
	_rd = RenderingServer.get_rendering_device()
	if _rd == null:
		return
	var format := RDTextureFormat.new()
	format.width = WIDTH
	format.height = HEIGHT
	format.format = RenderingDevice.DATA_FORMAT_R16G16B16A16_SFLOAT
	format.usage_bits = RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT | RenderingDevice.TEXTURE_USAGE_STORAGE_BIT
	rid = _rd.texture_create(format, RDTextureView.new())
	sampler = ComputePass.sampler(_rd, RenderingDevice.SAMPLER_FILTER_LINEAR)
	texture.texture_rd_rid = rid
	_bake = ComputePass.new(_rd, BAKE.replace("{radiance}", SkyModel.radiance_function(light)).replace("{tau}", ShaderText.number(TAU)).replace("{pi}", ShaderText.number(PI)), "sky lut")
	RenderingServer.call_on_render_thread(_dispatch)

func _dispatch() -> void:
	if _bake.ready():
		_bake.dispatch(_rd, [ComputePass.image(0, rid)], PackedFloat32Array([WIDTH, HEIGHT, 0.0, 0.0]), Vector2i(WIDTH, HEIGHT))

func free_device() -> void:
	if _rd == null:
		return
	texture.texture_rd_rid = RID()
	_bake.free_on(_rd)
	_rd.free_rid(sampler)
	_rd.free_rid(rid)

## Texel `(x, y)` holds the radiance toward the browser's equirectangular
## direction of its centre (`equirectDirectionWgsl`), clamped at zero.
const BAKE := """
layout(rgba16f, set = 0, binding = 0) uniform restrict writeonly image2D lut;
layout(push_constant, std430) uniform Size {
	vec4 size;
} frame;

{radiance}

void main() {
	ivec2 texel = ivec2(gl_GlobalInvocationID.xy);
	if (texel.x >= int(frame.size.x) || texel.y >= int(frame.size.y)) {
		return;
	}
	vec2 uv = (vec2(texel) + 0.5) / frame.size.xy;
	float theta = (uv.x - 0.5) * {tau};
	float phi = (uv.y - 0.5) * {pi};
	vec3 dir = vec3(cos(phi) * cos(theta), sin(phi), cos(phi) * sin(theta));
	imageStore(lut, texel, vec4(max(sky_radiance(dir), vec3(0.0)), 1.0));
}
"""
