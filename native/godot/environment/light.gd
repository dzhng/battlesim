extends RefCounted

## The manifest's `light`: the browser's `presentation.light` exactly as the
## battle frame was built with it, plus `sunDirection`, the unit vector toward
## the sun in sim space. This module checks its shape (every field present,
## nothing extra, every number finite; no defaults) and ports the formulas the
## browser derives from it (`packages/battle-renderer/src/light/`). Only the
## formulas live here; every tuning number is read from the manifest.

## The shape of `light`. A number is `0`, an rgb or a pair is its length, and a
## dictionary is a nested section.
const RGB := 3
const PAIR := 2
const SCHEMA := {
	"sun_azimuth": 0,
	"sun_elevation": 0,
	"sun_intensity": 0,
	"shadow_floor": 0,
	"sky": {
		"turbidity": 0,
		"radiance": 0,
		"fill": RGB,
		"clouds": {"coverage": 0, "height_m": 0, "scale_m": 0, "softness": 0, "opacity": 0, "radiance": 0, "shade": RGB},
	},
	"haze": {
		"distance_scale": 0,
		"clear_radius_m": 0,
		"overview_from_m": 0,
		"range_near_m": 0,
		"range_far_m": 0,
		"range_power": 0,
		"range_strength": 0,
		"sun_tint": RGB,
		"sun_strength": 0,
		"sun_power": 0,
		"mist_color": RGB,
		"mist_height_m": PAIR,
		"mist_distance_m": PAIR,
		"mist_color_strength": 0,
		"mist_opacity": 0,
	},
	"backdrop": {"reach_m": 0},
	"exposure": 0,
	"grade": {"saturation": 0, "contrast": 0, "split_tone": 0, "shadow_lift": 0, "shadow_tint": RGB, "highlight_tint": RGB, "lift": RGB},
	"bloom": {"strength": 0, "radius": 0, "threshold": 0, "smooth_width": 0},
	"cascades": {"count": 0, "map_size": 0, "max_far_m": 0, "split_lambda": 0, "normal_bias_texels": 0, "normal_bias_min_m": 0, "depth_bias": 0, "softness_m": 0},
	"sunDirection": RGB,
}
## Godot's directional light has at most four PSSM splits, and the browser's
## receiver layout has exactly four (`CSM_CASCADES`).
const CASCADES := 4

## An empty string when `light` has exactly the manifest's light shape, or the
## first problem, named by its path.
static func error(light: Variant) -> String:
	var shape := _shape_error(light, SCHEMA, "light")
	if not shape.is_empty():
		return shape
	if vector(light.sunDirection).is_zero_approx():
		return "light.sunDirection must point toward the sun"
	if int(light.cascades.count) != CASCADES or float(light.cascades.count) != CASCADES:
		return "light.cascades.count must be %d" % CASCADES
	if not int(light.cascades.map_size) in [512, 1024, 2048, 4096] or float(light.cascades.map_size) != int(light.cascades.map_size):
		return "light.cascades.map_size must be 512, 1024, 2048 or 4096"
	if float(light.backdrop.reach_m) < 0.0:
		return "light.backdrop.reach_m must not be negative"
	return ""

static func _shape_error(value: Variant, schema: Variant, path: String) -> String:
	if typeof(schema) == TYPE_DICTIONARY:
		if typeof(value) != TYPE_DICTIONARY:
			return "%s must be an object" % path
		for key in schema:
			if not value.has(key):
				return "%s.%s is missing" % [path, key]
		for key in value:
			if not schema.has(key):
				return "%s.%s is not a light field" % [path, key]
		for key in schema:
			var inner := _shape_error(value[key], schema[key], "%s.%s" % [path, key])
			if not inner.is_empty():
				return inner
		return ""
	if int(schema) == 0:
		return "" if _finite(value) else "%s must be a finite number" % path
	if typeof(value) != TYPE_ARRAY or value.size() != int(schema) or not value.all(_finite):
		return "%s must be %d finite numbers" % [path, int(schema)]
	return ""

static func _finite(value: Variant) -> bool:
	return (typeof(value) == TYPE_FLOAT or typeof(value) == TYPE_INT) and is_finite(float(value))

## A JSON triple as a vector, in whatever space it was written in.
static func vector(values: Array) -> Vector3:
	return Vector3(float(values[0]), float(values[1]), float(values[2]))

## The unit vector toward the sun, sim space (z up).
static func sun_direction(light: Dictionary) -> Vector3:
	return vector(light.sunDirection).normalized()

# Sky model (`light/skyParameters.ts`) -----------------------------------------
# Earth atmosphere constants (Hillaire, EGSR 2020), km units, z up. These are
# the formula's, not tuning: the browser keeps them in code beside it.

const PLANET_RADIUS_KM := 6360.0
const ATMOSPHERE_TOP_KM := 6460.0
const RAYLEIGH_SCALE_KM := 8.0
const MIE_SCALE_KM := 1.2
const BETA_RAYLEIGH := Vector3(5.802e-3, 13.558e-3, 33.1e-3)
const BETA_MIE_SCATTER := 3.996e-3
const BETA_MIE_EXTINCTION := 4.44e-3
const MIE_G := 0.8
const EYE_ALTITUDE_KM := 0.2

## How much Mie the turbidity adds (1 is pure Rayleigh air).
static func mie_scale(turbidity: float) -> float:
	return maxf(0.05, turbidity - 1.0)

## How overcast the sky is, from turbidity alone.
static func overcast(turbidity: float) -> float:
	return smoothstep(4.0, 7.0, turbidity)

## Beer-Lambert transmittance from the eye toward `direction` (sim space,
## unit) through the exponential atmosphere, normalised to its largest channel:
## the sun's tint.
static func transmittance_to_sun(direction: Vector3, turbidity: float) -> Vector3:
	var mie_extinction := mie_scale(turbidity) * BETA_MIE_EXTINCTION
	var r0 := PLANET_RADIUS_KM + EYE_ALTITUDE_KM
	var mu := maxf(direction.z, 0.0)
	var t_top := -r0 * mu + sqrt(r0 * r0 * (mu * mu - 1.0) + ATMOSPHERE_TOP_KM * ATMOSPHERE_TOP_KM)
	var steps := 64
	var dt := t_top / steps
	var depth := Vector3.ZERO
	for i in steps:
		var t := (i + 0.5) * dt
		var h := sqrt(r0 * r0 + t * t + 2.0 * r0 * t * mu) - PLANET_RADIUS_KM
		depth += (BETA_RAYLEIGH * exp(-h / RAYLEIGH_SCALE_KM) + Vector3.ONE * mie_extinction * exp(-h / MIE_SCALE_KM)) * dt
	var transmit := Vector3(exp(-depth.x), exp(-depth.y), exp(-depth.z))
	return transmit / maxf(maxf(transmit.x, transmit.y), maxf(transmit.z, 1e-6))

## The sun's linear rgb colour: its transmittance, greyed toward its own
## luminance as the sky turns overcast.
static func sun_color(light: Dictionary) -> Vector3:
	var transmit := transmittance_to_sun(sun_direction(light), float(light.sky.turbidity))
	var luminance := transmit.dot(Vector3(0.2126, 0.7152, 0.0722))
	return transmit.lerp(Vector3.ONE * luminance, overcast(float(light.sky.turbidity)))

# Cascades (`light/cascadePolicy.ts`, `shadowData.ts`, `frame/receiverRange.ts`)

## Practical split: each internal break is the uniform and logarithmic depths
## mixed by `lambda`, as a fraction of the receiver range [near, far]. The last
## break is 1.
static func cascade_breaks(near: float, far: float, count: int, lambda: float) -> PackedFloat64Array:
	var breaks := PackedFloat64Array()
	for i in range(1, count):
		var uniform := float(i) / count
		var logarithmic := (near * pow(far / near, float(i) / count) - near) / (far - near)
		breaks.append(uniform + (logarithmic - uniform) * lambda)
	breaks.append(1.0)
	return breaks

## The PCF radius in the browser's cascade texels: the fixture's penumbra over
## the texel, between one and four texels.
static func pcf_radius_texels(cascades: Dictionary, world_per_texel: float) -> float:
	return clampf(float(cascades.softness_m) / world_per_texel, 1.0, 4.0)

## Headroom above the highest ground for what stands on it.
const STANDING_M := 25.0
## Screen rays per side of the receiver sampling grid.
const RECEIVER_GRID := 12

## The view-depth range [near, far] where the camera's screen rays cross
## `box` (Godot space), sampled on the browser's ray grid. Collapses to just
## past the near plane when no ray meets it.
static func receiver_range(camera: Camera3D, viewport: Vector2, box: AABB) -> Vector2:
	var near := INF
	var far := 0.0
	var to_view := camera.global_transform.affine_inverse()
	for iy in RECEIVER_GRID + 1:
		for ix in RECEIVER_GRID + 1:
			var pixel := Vector2(ix, iy) / RECEIVER_GRID * viewport
			var origin := camera.project_ray_origin(pixel)
			var direction := camera.project_ray_normal(pixel)
			var span := _ray_box(origin, direction, box)
			if span.y < span.x:
				continue
			near = minf(near, -(to_view * (origin + direction * span.x)).z)
			far = maxf(far, -(to_view * (origin + direction * span.y)).z)
	if is_inf(near):
		return Vector2(camera.near, camera.near * 2.0)
	return Vector2(maxf(camera.near, near), far)

## The ray's [enter, exit] parameters through `box` (exit < enter when it
## misses), clipped to t >= 0.
static func _ray_box(origin: Vector3, direction: Vector3, box: AABB) -> Vector2:
	var enter := 0.0
	var exit := INF
	for axis in 3:
		if absf(direction[axis]) < 1e-12:
			if origin[axis] < box.position[axis] or origin[axis] > box.end[axis]:
				return Vector2(1.0, 0.0)
			continue
		var a := (box.position[axis] - origin[axis]) / direction[axis]
		var b := (box.end[axis] - origin[axis]) / direction[axis]
		enter = maxf(enter, minf(a, b))
		exit = minf(exit, maxf(a, b))
	return Vector2(enter, exit)
