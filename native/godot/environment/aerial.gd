extends RefCounted

## The browser's aerial haze (`light/aerialParameters.ts`, `shaders/aerial.ts`),
## ported by formula: extinction over distance from the camera's ground focus,
## a range term, a sunward Mie tint and a low valley mist, toward the sky's
## own radiance along the view. The curve's numbers are the manifest's
## `light.haze`; turbidity drives the physical extinction.

const Light = preload("res://environment/light.gd")
const StrictJson = preload("res://core/strict_json.gd")
const ShaderText = preload("res://core/shader_text.gd")

## Neutral ground-fog extinction (km^-1) per (turbidity - onset)^2, and the
## horizon blend's shape: the haze formula's own constants.
const FOG_COEFF_KM := 0.026
const FOG_TURBIDITY_ONSET := 4.0
const HORIZON_SKY_Z := 0.004
const HORIZON_FADE_START_Z := -0.18
const HORIZON_FADE_END_Z := 0.06
const HORIZON_CLEAR_VISIBILITY_KM := 8.0
const HORIZON_WIDE_VISIBILITY_KM := 32.0
const HORIZON_CLEAR_EXTRA_START_Z := 0.2
const HORIZON_CLEAR_EXTRA_END_Z := 0.08

## Per-channel extinction (km^-1) from turbidity, scaled by the haze's
## `distance_scale`.
static func extinction(light: Dictionary) -> Vector3:
	var turbidity := float(light.sky.turbidity)
	var mie := Light.mie_scale(turbidity) * Light.BETA_MIE_EXTINCTION
	var fog := FOG_COEFF_KM * pow(maxf(0.0, turbidity - FOG_TURBIDITY_ONSET), 2.0)
	return (Light.BETA_RAYLEIGH + Vector3.ONE * mie) * float(light.haze.distance_scale) + Vector3.ONE * fog

## Koschmieder meteorological visibility (km).
static func visibility_km(light: Dictionary) -> float:
	var sigma := extinction(light)
	return 3.912 / ((sigma.x + sigma.y + sigma.z) / 3.0)

## The view-direction heights where a downward view's sky light blends from
## the view's own sky to the horizon's; clearer air widens the blend.
static func horizon_fade(light: Dictionary) -> Vector2:
	var thin := clampf((visibility_km(light) - HORIZON_CLEAR_VISIBILITY_KM) / (HORIZON_WIDE_VISIBILITY_KM - HORIZON_CLEAR_VISIBILITY_KM), 0.0, 1.0)
	return Vector2(HORIZON_FADE_START_Z - HORIZON_CLEAR_EXTRA_START_Z * thin, HORIZON_FADE_END_Z + HORIZON_CLEAR_EXTRA_END_Z * thin)

## `vec3 apply_aerial(vec3 surface, vec3 position, vec3 camera, vec3 observer)`
## in sim space (metres, z up): `surface` behind the air between it and the
## eye. `observer` is the camera's ground focus. Needs `sky_radiance`.
static func function(light: Dictionary) -> String:
	var h: Dictionary = light.haze
	var fade := horizon_fade(light)
	var f := ShaderText.number
	var rgb := ShaderText.rgb
	return """
vec3 apply_aerial(vec3 surface, vec3 position, vec3 camera, vec3 observer) {
	vec3 reach = position - observer;
	float dist_m = length(reach) / max(1.0, distance(camera, observer) / {overview_from});
	float dist_km = max(dist_m / 1000.0 - {clear_radius_km}, 0.0);
	float range_depth = pow(max(dist_m - {range_near}, 0.0) / {range_far}, {range_power}) * {range_strength};
	float height_mist = 1.0 - smoothstep({mist_bottom}, {mist_top}, position.z);
	float far_mist = smoothstep({mist_start}, {mist_full}, dist_m);
	float mist_weight = height_mist * far_mist;
	vec3 transmit = exp(-({extinction} * dist_km + vec3(range_depth) + vec3(mist_weight * {mist_opacity})));
	vec3 view = normalize(position - camera);
	vec3 view_sky = sky_radiance(normalize(vec3(view.xy, max(view.z, {horizon_z}))));
	vec3 horizon_sky = sky_radiance(normalize(vec3(view.x, view.y, {horizon_z})));
	float below = smoothstep({fade_start}, 0.0, view.z);
	float above = 1.0 - smoothstep(0.0, {fade_end}, view.z);
	float sun_mie = pow(clamp(dot(view, {sun}), 0.0, 1.0), {sun_power}) * {sun_strength};
	vec3 sky_light = mix(mix(view_sky, horizon_sky, below * above), {sun_tint}, sun_mie);
	vec3 mist_sky_light = mix(sky_light, {mist_color}, mist_weight * {mist_color_strength});
	return surface * transmit + mist_sky_light * (vec3(1.0) - transmit);
}
""".format({
		"overview_from": f.call(float(h.overview_from_m)),
		"clear_radius_km": f.call(float(h.clear_radius_m) / 1000.0),
		"range_near": f.call(float(h.range_near_m)),
		"range_far": f.call(float(h.range_far_m)),
		"range_power": f.call(float(h.range_power)),
		"range_strength": f.call(float(h.range_strength)),
		"mist_bottom": f.call(float(h.mist_height_m[0])),
		"mist_top": f.call(float(h.mist_height_m[1])),
		"mist_start": f.call(float(h.mist_distance_m[0])),
		"mist_full": f.call(float(h.mist_distance_m[1])),
		"mist_opacity": f.call(float(h.mist_opacity)),
		"extinction": rgb.call(extinction(light)),
		"horizon_z": f.call(HORIZON_SKY_Z),
		"fade_start": f.call(fade.x),
		"fade_end": f.call(fade.y),
		"sun": rgb.call(Light.sun_direction(light)),
		"sun_power": f.call(float(h.sun_power)),
		"sun_strength": f.call(float(h.sun_strength)),
		"sun_tint": rgb.call(StrictJson.vector3(h.sun_tint)),
		"mist_color": rgb.call(StrictJson.vector3(h.mist_color)),
		"mist_color_strength": f.call(float(h.mist_color_strength)),
	})
