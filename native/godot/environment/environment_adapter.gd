extends Node3D

## The light the parity run draws under, built from the manifest's light: the
## browser's sun with its cascaded shadows, and the sky's light scaled by the
## fill as the ambient and reflected light. Each frame, `fit(camera)` splits
## the cascades over the part of the map the camera sees, as the browser does.

const Coords = preload("res://core/coords.gd")
const Light = preload("res://environment/light.gd")
const SkyModel = preload("res://environment/sky_model.gd")

## Godot's PCF filter whose four Vogel taps are closest to the browser's five,
## and its kernel radius in shadow-atlas texels per unit of `shadow_blur`
## (`directional_soft_shadow_filter_set_quality`).
const SHADOW_FILTER := RenderingServer.SHADOW_QUALITY_SOFT_LOW
const SHADOW_FILTER_RADIUS := 2.0
## The browser's camera far when its own is infinite; it only enters the
## cascade extent's fade margin.
const FINITE_CAMERA_FAR := 1e7

var _light: Dictionary
var _receivers: AABB
var _sun: DirectionalLight3D

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
	env.fog_enabled = true
	env.fog_light_color = Color("a9b7b0")
	env.fog_light_energy = 0.32
	env.fog_density = 0.00065
	env.fog_sky_affect = 0.28
	var world := WorldEnvironment.new()
	world.environment = env
	add_child(world)
	_sun = build_sun(light)
	add_child(_sun)

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

## Splits the cascades over the receivers `camera` sees and sets the penumbra,
## as the browser's cascade fit does for the frame it is about to draw.
func fit(camera: Camera3D, viewport: Vector2) -> void:
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
	# Godot has one PCF radius per light, in its own split's texels; the
	# penumbra is the browser's nearest cascade's, in metres.
	var aspect := viewport.x / viewport.y
	var texel := browser_extent(near, depths[0], far, camera.fov, aspect) / float(cascades.map_size)
	var penumbra_m := Light.pcf_radius_texels(cascades, texel) * texel
	var godot_texel := godot_extent(camera.near, depths[0], camera.fov, aspect) / float(cascades.map_size)
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

## The sky shader. Godot lights with what the shader writes in its cubemap
## pass: the browser's sky radiance times the fill. The shown background stays
## the pre-parity sky until slice 04b ports the browser's.
static func sky_shader(light: Dictionary) -> String:
	return SKY_SHADER.replace("{radiance}", SkyModel.radiance_function(light)).replace("{fill}", SkyModel.rgb(Light.vector(light.sky.fill)))

const SKY_SHADER := """
shader_type sky;
render_mode use_debanding;

{radiance}

float hash21(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float cloud_noise(vec2 p) {
	vec2 cell = floor(p);
	vec2 local = fract(p);
	local = local * local * (3.0 - 2.0 * local);
	float a = hash21(cell);
	float b = hash21(cell + vec2(1.0, 0.0));
	float c = hash21(cell + vec2(0.0, 1.0));
	float d = hash21(cell + vec2(1.0, 1.0));
	return mix(mix(a, b, local.x), mix(c, d, local.x), local.y);
}

float cloud_fbm(vec2 p) {
	float value = 0.0;
	float weight = 0.58;
	for (int i = 0; i < 4; i++) {
		value += cloud_noise(p) * weight;
		p = p * 2.03 + vec2(17.0, 9.0);
		weight *= 0.5;
	}
	return value;
}

void sky() {
	vec3 direction = normalize(EYEDIR);
	// Sim (x, y, z) from Godot (x, y, z) is (x, -z, y).
	vec3 sim = vec3(direction.x, -direction.z, direction.y);
	if (AT_CUBEMAP_PASS) {
		COLOR = max(sky_radiance(sim), vec3(0.0)) * {fill};
	} else {
		float horizon = smoothstep(-0.10, 0.58, direction.y);
		vec3 clear = mix(vec3(0.71, 0.82, 0.80), vec3(0.17, 0.42, 0.62), horizon);
		float cloud_band = smoothstep(0.10, 0.72, direction.y) * (1.0 - smoothstep(0.80, 0.98, direction.y));
		vec2 cloud_uv = sim.xy / max(direction.y + 0.08, 0.18) * 0.92;
		float cloud = cloud_fbm(cloud_uv * 1.65 + vec2(4.2, -2.7));
		COLOR = mix(clear, vec3(0.96, 0.95, 0.88), smoothstep(0.53, 0.61, cloud) * cloud_band * 0.95);
	}
}
"""
