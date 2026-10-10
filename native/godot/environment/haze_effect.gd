extends CompositorEffect

## The browser's aerial haze, applied once to the lit frame from its depth:
## every surface is seen through the air between it and the eye, toward the
## ground focus `observer`. The sky (cleared depth) is left as it is. It runs
## after transparent geometry and before the post chain.

const Coords = preload("res://core/coords.gd")
const Aerial = preload("res://environment/aerial.gd")
const SkyModel = preload("res://environment/sky_model.gd")
const ComputePass = preload("res://environment/compute_pass.gd")

## The camera's ground focus in sim space, and its near and far planes, set
## each frame. (A reverse-Z `Projection` doesn't report its planes.)
var observer := Vector3.ZERO
var near := 1.0
var far := 1.0

var _source: String
var _pass: ComputePass
var _nearest: RID

func _init(light: Dictionary) -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT
	# Under MSAA, read the resolved colour and depth.
	access_resolved_color = true
	access_resolved_depth = true
	_source = SHADER.replace("{sky}", SkyModel.radiance_function(light)).replace("{aerial}", Aerial.function(light))

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE and _pass != null:
		var rd := RenderingServer.get_rendering_device()
		_pass.free_on(rd)
		rd.free_rid(_nearest)

func _render_callback(_type: int, render_data: RenderData) -> void:
	var rd := RenderingServer.get_rendering_device()
	if _pass == null:
		_pass = ComputePass.new(rd, _source, "aerial haze")
		_nearest = ComputePass.sampler(rd, RenderingDevice.SAMPLER_FILTER_NEAREST)
	if not _pass.ready():
		return
	var buffers: RenderSceneBuffersRD = render_data.get_render_scene_buffers()
	var scene: RenderSceneData = render_data.get_render_scene_data()
	var size := buffers.get_internal_size()
	var view := scene.get_cam_transform()
	var projection := scene.get_cam_projection()
	var push := PackedFloat32Array()
	for row in [[Coords.godot_to_sim(view.origin), near], [observer, far], [Coords.godot_to_sim(view.basis.x), 1.0 / projection.x.x], [Coords.godot_to_sim(view.basis.y), 1.0 / projection.y.y], [Coords.godot_to_sim(-view.basis.z), 0.0]]:
		push.append_array([row[0].x, row[0].y, row[0].z, row[1]])
	for view_index in buffers.get_view_count():
		var uniforms := [
			ComputePass.image(0, buffers.get_color_layer(view_index)),
			ComputePass.sampled(1, _nearest, buffers.get_depth_layer(view_index)),
		]
		_pass.dispatch(rd, uniforms, push, size)

const SHADER := """
layout(rgba16f, set = 0, binding = 0) uniform restrict image2D color;
layout(set = 0, binding = 1) uniform sampler2D depth;
layout(push_constant, std430) uniform Frame {
	vec4 eye;      // sim position, near plane
	vec4 focus;    // sim ground focus, far plane
	vec4 right;    // sim camera right, tan of half the horizontal field
	vec4 up;       // sim camera up, tan of half the vertical field
	vec4 forward;  // sim view direction
} frame;

{sky}
{aerial}

void main() {
	ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
	ivec2 size = imageSize(color);
	if (pixel.x >= size.x || pixel.y >= size.y) {
		return;
	}
	// Reverse Z: 1 at the near plane, 0 (cleared) at the far plane and sky.
	float z = texelFetch(depth, pixel, 0).r;
	if (z <= 0.0) {
		return;
	}
	float near = frame.eye.w;
	float far = frame.focus.w;
	float view_depth = far * near / (z * (far - near) + near);
	vec2 ndc = (vec2(pixel) + 0.5) / vec2(size) * 2.0 - 1.0;
	vec3 ray = frame.forward.xyz + frame.right.xyz * ndc.x * frame.right.w - frame.up.xyz * ndc.y * frame.up.w;
	vec3 position = frame.eye.xyz + ray * view_depth;
	vec4 surface = imageLoad(color, pixel);
	imageStore(color, pixel, vec4(apply_aerial(surface.rgb, position, frame.eye.xyz, frame.focus.xyz), surface.a));
}
"""
