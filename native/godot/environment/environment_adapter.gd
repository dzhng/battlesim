extends Node3D

## The light the parity run draws under, built from the manifest's light: the
## browser's sun with its cascaded shadows, its sky (shown when the manifest
## lists `sky`) whose light scaled by the fill is the ambient and reflected
## light, its aerial haze and its post chain (bloom, grade, AgX). Each frame,
## `prepare` splits the cascades over the part of the map the camera sees and
## focuses the haze, as the browser's environment frame does.

const Coords = preload("res://core/coords.gd")
const Light = preload("res://environment/light.gd")
const StrictJson = preload("res://core/strict_json.gd")
const SkyModel = preload("res://environment/sky_model.gd")
const ShaderText = preload("res://core/shader_text.gd")
const HazeEffect = preload("res://environment/haze_effect.gd")
const PostEffect = preload("res://environment/post_effect.gd")

## Godot's PCF filter whose four Vogel taps are closest to the browser's five,
## and its kernel radius in shadow-atlas texels per unit of `shadow_blur`
## (`directional_soft_shadow_filter_set_quality`).
const SHADOW_FILTER := RenderingServer.SHADOW_QUALITY_SOFT_LOW
const SHADOW_FILTER_RADIUS := 2.0
## The browser's camera far when its own is infinite; it only enters the
## cascade extent's fade margin.
const FINITE_CAMERA_FAR := 1e7
## The browser draws its lit world at 4× MSAA (`FRAME_MSAA`), shades and hazes
## each sample, and resolves before post.
const FRAME_MSAA := Viewport.MSAA_4X

var _light: Dictionary
var _receivers: AABB
var _sun: DirectionalLight3D
var _haze: CompositorEffect

## `receivers` is the box the map's shadow receivers lie in, in Godot space.
func _init(light: Dictionary, sky_listed: bool, receivers: AABB) -> void:
	name = "Environment"
	_light = light
	_receivers = receivers
	var env := Environment.new()
	var sky := Sky.new()
	var sky_material := ShaderMaterial.new()
	sky_material.shader = Shader.new()
	sky_material.shader.code = sky_shader(light)
	sky.sky_material = sky_material
	env.sky = sky
	env.background_mode = Environment.BG_SKY if sky_listed else Environment.BG_CLEAR_COLOR
	# The browser lights every surface with its sky times the fill (diffuse
	# and specular alike); the sky shader writes that product into the
	# radiance cubemap Godot lights with.
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_sky_contribution = 1.0
	env.ambient_light_energy = 1.0
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	var world := WorldEnvironment.new()
	world.environment = env
	_haze = HazeEffect.new(light)
	world.compositor = Compositor.new()
	world.compositor.compositor_effects = [_haze, PostEffect.new(light)]
	# The post chain tone maps; Godot's output step only applies the sRGB
	# transfer the browser's post ends with.
	env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	env.tonemap_exposure = 1.0
	env.glow_enabled = false
	add_child(world)
	_sun = build_sun(light)
	add_child(_sun)

func _enter_tree() -> void:
	get_viewport().msaa_3d = FRAME_MSAA

## The browser's sun as a Godot light: its direction through the one
## conversion, its colour from the sky model and its intensity, keeping
## `shadow_floor` of itself in shadow.
static func build_sun(light: Dictionary) -> DirectionalLight3D:
	var sun := DirectionalLight3D.new()
	sun.name = "Sun"
	# A directional light shines along its -Z, away from the sun.
	sun.basis = Basis.looking_at(Coords.sim_to_godot(-Light.sun_direction(light)))
	# Godot reads a light's colour as sRGB and converts it to linear.
	var color := Light.sun_color(light)
	sun.light_color = Color(color.x, color.y, color.z).linear_to_srgb()
	# Godot multiplies a directional light's energy by pi and its Lambert
	# term divides by pi; the browser's sun radiance is divided by pi once.
	sun.light_energy = float(light.sun_intensity) / PI
	sun.shadow_enabled = true
	sun.shadow_opacity = 1.0 - float(light.shadow_floor)
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_blend_splits = true
	# Godot packs four splits two by two into one atlas.
	RenderingServer.directional_shadow_atlas_set_size(2 * int(light.cascades.map_size), true)
	RenderingServer.directional_soft_shadow_filter_set_quality(SHADOW_FILTER)
	return sun

## Poses the light for the frame `camera` is about to draw: the cascades split
## over the receivers it sees, with the browser's penumbra, and the haze
## measured from `focus`, the camera's ground focus in sim space.
func prepare(camera: Camera3D, viewport: Vector2, focus: Vector3) -> void:
	_haze.observer = focus
	_haze.near = camera.near
	_haze.far = camera.far
	var cascades: Dictionary = _light.cascades
	var seen := Light.receiver_range(camera, viewport, _receivers)
	var near := maxf(camera.near, seen.x)
	var far := maxf(near * 1.5, minf(seen.y, float(cascades.max_far_m)))
	var breaks := Light.cascade_breaks(near, far, Light.CASCADES, float(cascades.split_lambda))
	# Godot's splits start at the camera and are fractions of the shadow
	# distance; the browser's are fractions of the receiver range.
	_sun.directional_shadow_max_distance = far
	var depths: Array[float] = []
	for b in breaks:
		depths.append(near + b * (far - near))
	_sun.directional_shadow_split_1 = depths[0] / far
	_sun.directional_shadow_split_2 = depths[1] / far
	_sun.directional_shadow_split_3 = depths[2] / far
	# The browser's last cascade fades to unshadowed over the last quarter of
	# the receiver range.
	_sun.directional_shadow_fade_start = (far - 0.25 * (far - near)) / far
	# The browser's penumbra is the same width in metres in every cascade
	# (a texel at least); Godot's is one radius in each split's own texels, so
	# it widens with distance. The two agree in the second cascade, where the
	# reel's subjects mostly stand: the first is slightly sharper, the last two
	# softer.
	var aspect := viewport.x / viewport.y
	var texel := browser_extent(depths[0], depths[1], far, camera.fov, aspect) / float(cascades.map_size)
	var penumbra_m := Light.pcf_radius_texels(cascades, texel) * texel
	var godot_texel := godot_extent(depths[0], depths[1], camera.fov, aspect) / float(cascades.map_size)
	_sun.shadow_blur = penumbra_m / godot_texel / SHADOW_FILTER_RADIUS

## The browser's square cascade extent for the slice between view depths
## `near` and `far`: the longer of its far diagonal and its near-to-far
## diagonal, widened by the fade margin (`cascadeExtent`).
static func browser_extent(near: float, far: float, receiver_far: float, fov_degrees: float, aspect: float) -> float:
	var slope := tan(deg_to_rad(fov_degrees) * 0.5)
	var far_half := Vector2(far * slope * aspect, far * slope)
	var near_half := Vector2(near * slope * aspect, near * slope)
	var across := 2.0 * far_half.length()
	var diagonal := Vector3(far_half.x + near_half.x, far_half.y + near_half.y, far - near).length()
	var span := maxf(FINITE_CAMERA_FAR, receiver_far) - near
	return maxf(across, diagonal) + 0.25 * far * far / span

## Godot's stable split extent: the diameter of the sphere around the slice's
## eight corners, centred on their mean.
static func godot_extent(near: float, far: float, fov_degrees: float, aspect: float) -> float:
	var slope := tan(deg_to_rad(fov_degrees) * 0.5)
	var corners: Array[Vector3] = []
	for depth in [near, far]:
		for sx in [-1.0, 1.0]:
			for sy in [-1.0, 1.0]:
				corners.append(Vector3(sx * depth * slope * aspect, sy * depth * slope, -depth))
	var centre := Vector3.ZERO
	for corner in corners:
		centre += corner / corners.size()
	var radius := 0.0
	for corner in corners:
		radius = maxf(radius, corner.distance_to(centre))
	return 2.0 * radius

## The sky shader. The shown sky is the browser's: its sky radiance with the
## horizon's haze below the horizon, the sun's disc and the cloud layer. Godot
## lights with what the shader writes in its cubemap pass: the sky radiance
## (with its ground bounce, without clouds or disc) times the fill.
static func sky_shader(light: Dictionary) -> String:
	return SKY_SHADER.replace("{functions}", SkyModel.radiance_function(light) + SkyModel.disc_function(light) + SkyModel.clouds_function(light)).replace("{fill}", ShaderText.rgb(StrictJson.vector3(light.sky.fill)))

const SKY_SHADER := """
shader_type sky;
render_mode use_debanding;

#include "res://core/coords.gdshaderinc"

{functions}

void sky() {
	vec3 dir = godot_to_sim(normalize(EYEDIR));
	if (AT_CUBEMAP_PASS) {
		COLOR = max(sky_radiance(dir), vec3(0.0)) * {fill};
	} else {
		vec3 shown = normalize(vec3(dir.xy, max(dir.z, 0.004)));
		COLOR = max(sky_clouds(dir, sky_radiance(shown) + sky_disc(dir)), vec3(0.0));
	}
}
"""
