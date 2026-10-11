extends RefCounted

## The browser's physical sky (`shaders/physicalSky.ts`), ported by formula into
## shader text with the manifest's numbers written in, as the browser writes
## its own. Every function takes and returns sim-space (z up) directions. The
## text is plain GLSL that both Godot's shading language and RenderingDevice
## compute shaders accept.

const Light = preload("res://environment/light.gd")
const ShaderText = preload("res://core/shader_text.gd")
const StrictJson = preload("res://core/strict_json.gd")

## Multiple-scattering floor and its sky-blue tint, the low-sun aureole, the
## below-horizon ground bounce, the overcast dome and the sun disc: the sky
## model's own constants, kept beside the formula as the browser keeps them.
const MS_FLOOR := 0.32
const MS_TINT := Vector3(0.5, 0.7, 1.0)
const LOW_SUN_AUREOLE_RADIANCE := 0.55
const LOW_SUN_AUREOLE_COS_OUTER := -0.12
const LOW_SUN_AUREOLE_COS_INNER := 0.76
const GROUND_BOUNCE_TINT := Vector3(0.34, 0.3, 0.25)
const OVERCAST_ZENITH_RADIANCE := Vector3(1.02, 1.05, 1.1)
const SUN_DISC_COS_INNER := 0.99985
const SUN_DISC_COS_OUTER := 0.99955
const SUN_DISC_RADIANCE := 2.5

static func low_sun_aureole_strength(sun_z: float, overcast: float) -> float:
	return pow(1.0 - overcast, 2.0) * (1.0 - smoothstep(0.25, 0.85, sun_z)) * LOW_SUN_AUREOLE_RADIANCE

## `vec3 sky_radiance(vec3 dir_in)`: the clear sky's single-scattered radiance
## plus a multiple-scattering floor, the low-sun aureole, the overcast dome by
## turbidity, and the ground bounce below the horizon. This is what the
## browser bakes into its sky LUT: no clouds and no sun disc.
static func radiance_function(light: Dictionary) -> String:
	var sun := Light.sun_direction(light)
	var turbidity := float(light.sky.turbidity)
	var mie := Light.mie_scale(turbidity)
	var overcast := Light.overcast(turbidity)
	var transmit := Light.transmittance_to_sun(sun, turbidity)
	var r0 := Light.PLANET_RADIUS_KM + Light.EYE_ALTITUDE_KM
	var g2 := Light.MIE_G * Light.MIE_G
	var top2 := Light.ATMOSPHERE_TOP_KM * Light.ATMOSPHERE_TOP_KM
	var bounce := GROUND_BOUNCE_TINT + (Vector3.ONE * 0.88 - GROUND_BOUNCE_TINT) * overcast
	return """
vec3 sky_radiance(vec3 dir_in) {
	vec3 beta_r = {beta_r};
	float beta_m_scatter = {beta_m_scatter};
	float beta_m_extinction = {beta_m_extinction};
	vec3 sun_dir = {sun};
	float mu = max(dir_in.z, 0.004);
	vec3 dir = normalize(vec3(dir_in.xy, mu));
	float t_top = sqrt({r0_2} * (mu * mu - 1.0) + {top2}) - mu * {r0};
	float dt = t_top / 32.0;
	float cos_theta = dot(dir, sun_dir);
	float phase_r = (cos_theta * cos_theta + 1.0) * {phase_r};
	float phase_m = {phase_m} / pow({one_g2} - cos_theta * {two_g}, 1.5);
	vec3 radiance = vec3(0.0);
	vec3 od_view = vec3(0.0);
	for (int i = 0; i < 32; i++) {
		float t = (float(i) + 0.5) * dt;
		float px = dir.x * t;
		float py = dir.y * t;
		float pz = dir.z * t + {r0};
		float rp = sqrt(px * px + py * py + pz * pz);
		float h = rp - {planet};
		float rho_r = exp(h / {neg_rayleigh});
		float rho_m = exp(h / {neg_mie});
		od_view += (beta_r * rho_r + rho_m * beta_m_extinction) * dt;
		vec3 t_view = exp(-od_view);
		float mu_s = (px * sun_dir.x + py * sun_dir.y + pz * sun_dir.z) / rp;
		float t_sun_top = sqrt(rp * rp * (mu_s * mu_s - 1.0) + {top2}) - mu_s * rp;
		float dts = t_sun_top / 6.0;
		vec3 od_sun = vec3(0.0);
		for (int j = 0; j < 6; j++) {
			float ts = (float(j) + 0.5) * dts;
			float sx = px + sun_dir.x * ts;
			float sy = py + sun_dir.y * ts;
			float sz = pz + sun_dir.z * ts;
			float hs = sqrt(sx * sx + sy * sy + sz * sz) - {planet};
			od_sun += (beta_r * exp(hs / {neg_rayleigh}) + exp(hs / {neg_mie}) * beta_m_extinction) * dts;
		}
		float horizon_mu = -sqrt(max(1.0 - {planet2} / (rp * rp), 0.0));
		float shadow = smoothstep(horizon_mu - 0.008, horizon_mu + 0.004, mu_s);
		vec3 t_sun = exp(-od_sun) * shadow;
		vec3 scatter_r = beta_r * rho_r;
		float scatter_m = rho_m * beta_m_scatter;
		vec3 single = t_sun * (scatter_r * phase_r + scatter_m * phase_m);
		vec3 multiple = (scatter_r + scatter_m) * {ms_tint} * {ms};
		radiance += t_view * (single + multiple) * dt;
	}
	float aureole = smoothstep({aureole_outer}, {aureole_inner}, cos_theta) * {aureole};
	vec3 clear_sky = radiance * {radiance_scale} + {transmit} * aureole;
	float overcast_gradient = 1.05 - max(dir_in.z, 0.0) * 0.22;
	vec3 overcast_sky = {overcast_zenith} * overcast_gradient * {overcast_sun};
	vec3 result = mix(clear_sky, overcast_sky, {overcast});
	float ground = smoothstep(0.0, 0.35, -dir_in.z);
	return mix(result, result * {bounce}, ground);
}
""".format({
		"beta_r": ShaderText.rgb(Light.BETA_RAYLEIGH),
		"beta_m_scatter": ShaderText.number(Light.BETA_MIE_SCATTER * mie),
		"beta_m_extinction": ShaderText.number(Light.BETA_MIE_EXTINCTION * mie),
		"sun": ShaderText.rgb(sun),
		"r0": ShaderText.number(r0),
		"r0_2": ShaderText.number(r0 * r0),
		"top2": ShaderText.number(top2),
		"phase_r": ShaderText.number(3.0 / (16.0 * PI)),
		"phase_m": ShaderText.number((1.0 - g2) / (4.0 * PI)),
		"one_g2": ShaderText.number(1.0 + g2),
		"two_g": ShaderText.number(2.0 * Light.MIE_G),
		"planet": ShaderText.number(Light.PLANET_RADIUS_KM),
		"planet2": ShaderText.number(Light.PLANET_RADIUS_KM * Light.PLANET_RADIUS_KM),
		"neg_rayleigh": ShaderText.number(-Light.RAYLEIGH_SCALE_KM),
		"neg_mie": ShaderText.number(-Light.MIE_SCALE_KM),
		"ms_tint": ShaderText.rgb(MS_TINT),
		"ms": ShaderText.number(MS_FLOOR * sqrt(maxf(sun.z, 0.0)) / (4.0 * PI)),
		"aureole_outer": ShaderText.number(LOW_SUN_AUREOLE_COS_OUTER),
		"aureole_inner": ShaderText.number(LOW_SUN_AUREOLE_COS_INNER),
		"aureole": ShaderText.number(low_sun_aureole_strength(sun.z, overcast)),
		"radiance_scale": ShaderText.number(float(light.sky.radiance)),
		"transmit": ShaderText.rgb(transmit),
		"overcast_zenith": ShaderText.rgb(OVERCAST_ZENITH_RADIANCE),
		"overcast_sun": ShaderText.number(sqrt(maxf(sun.z, 0.05))),
		"overcast": ShaderText.number(overcast),
		"bounce": ShaderText.rgb(bounce),
	})

## `vec3 sky_radiance(vec3 dir)` read from the baked lookup (`sky_lut.gd`),
## a `sampler2D sky_lut` the shader declares: the browser's equirectangular
## coordinates (`equirectUvWgsl`), sampled linearly.
static func lookup_function() -> String:
	return """
vec2 sky_lut_uv(vec3 dir) {
	return vec2(atan(dir.z, dir.x) * {inv_tau} + 0.5, asin(clamp(dir.y, -1.0, 1.0)) * {inv_pi} + 0.5);
}

vec3 sky_radiance(vec3 dir) {
	return textureLod(sky_lut, sky_lut_uv(dir), 0.0).rgb;
}
""".format({"inv_tau": ShaderText.number(1.0 / TAU), "inv_pi": ShaderText.number(1.0 / PI)})

## `vec3 sky_disc(vec3 dir)`: the sun's disc, added to the shown sky only.
static func disc_function(light: Dictionary) -> String:
	var sun := Light.sun_direction(light)
	var overcast := Light.overcast(float(light.sky.turbidity))
	return """
vec3 sky_disc(vec3 dir) {
	float disc = smoothstep({outer}, {inner}, dot(dir, {sun})) * smoothstep(-0.015, 0.01, dir.z) * {radiance};
	return {transmit} * disc;
}
""".format({
		"outer": ShaderText.number(SUN_DISC_COS_OUTER),
		"inner": ShaderText.number(SUN_DISC_COS_INNER),
		"sun": ShaderText.rgb(sun),
		"radiance": ShaderText.number(SUN_DISC_RADIANCE * (1.0 - overcast)),
		"transmit": ShaderText.rgb(Light.transmittance_to_sun(sun, float(light.sky.turbidity))),
	})

## `vec3 sky_clouds(vec3 dir, vec3 sky)`: the view's cumulus layer over the
## shown sky `sky`, a warped value-noise fBm thresholded to the coverage and
## lit as if its thickness were height. Its opacity gives way to the overcast
## dome. Never part of the light the sky casts.
static func clouds_function(light: Dictionary) -> String:
	var sun := Light.sun_direction(light)
	var clouds: Dictionary = light.sky.clouds
	var turbidity := float(light.sky.turbidity)
	var opacity := float(clouds.opacity) * (1.0 - Light.overcast(turbidity))
	var across := Vector2(sun.x, sun.y).length()
	var to_sun := Vector2(sun.x, sun.y) / across if across > 1e-6 else Vector2(1, 0)
	var lo := 1.0 - float(clouds.coverage)
	var softness := float(clouds.softness)
	var radiance := float(clouds.radiance)
	return """
float cloud_hash(ivec2 i) {
	uint h = (uint(i.x) * 0x8da6b343u) ^ (uint(i.y) * 0xd8163841u);
	h = h ^ (h >> 13u);
	h = h * 0x5bd1e995u;
	h = h ^ (h >> 15u);
	return float(h & 0xffffffu) / 16777216.0;
}

float cloud_noise(vec2 p) {
	ivec2 i = ivec2(floor(p));
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(cloud_hash(i), cloud_hash(i + ivec2(1, 0)), u.x), mix(cloud_hash(i + ivec2(0, 1)), cloud_hash(i + ivec2(1, 1)), u.x), u.y);
}

float cloud_fbm(vec2 p0) {
	// Warped, so masses round into heaps rather than streaks.
	vec2 p = p0 + vec2(cloud_noise(p0 * 0.5), cloud_noise(p0 * 0.5 + vec2(5.2, 1.3))) * 0.9;
	float sum = 0.0;
	float amp = 0.55;
	float norm = 0.0;
	for (int k = 0; k < 5; k++) {
		sum += amp * cloud_noise(p);
		norm += amp;
		p = vec2(p.x * 1.7 - p.y * 1.1, p.x * 1.1 + p.y * 1.7) + vec2(17.3, 9.1);
		amp *= 0.45;
	}
	return sum / norm;
}

vec3 sky_clouds(vec3 dir, vec3 sky) {
	if (dir.z < 0.012) {
		return sky;
	}
	// A dome, not a plane: far clouds keep their bulk.
	vec2 at = dir.xy * ({height_over_scale} / (dir.z + 0.12));
	float n = cloud_fbm(at);
	float density = smoothstep({lo_minus}, {lo_plus}, n);
	float e = 0.02;
	vec2 slope = vec2(cloud_fbm(at + vec2(e, 0.0)) - n, cloud_fbm(at + vec2(0.0, e)) - n) / e;
	vec3 normal = normalize(vec3(-slope * 0.35, 1.0));
	float facing = clamp(dot(normal, {sun}) * 0.5 + 0.5, 0.0, 1.0);
	float toward_sun = cloud_fbm(at + vec2({to_sun_x}, {to_sun_y}));
	float shadow = smoothstep({lo}, {lo_shadow}, toward_sun) * (1.0 - facing);
	float core = smoothstep({lo}, {lo_core}, n);
	vec3 lit = {transmit} * {radiance};
	vec3 base = {shade} * {base_radiance} + sky * 0.25;
	vec3 cloud = mix(base, lit, clamp(facing * 1.2 - shadow * 0.5 - core * 0.3, 0.0, 1.0));
	// Thin edges toward the sun glow (forward scattering).
	float silver = pow(max(dot(dir, {sun}), 0.0), 6.0) * (1.0 - core);
	cloud += {transmit} * silver * {silver_radiance};
	// Far clouds pile toward the horizon and take its haze.
	float near = smoothstep(0.0, 0.3, dir.z);
	cloud = mix(mix(sky, cloud, 0.55), cloud, near);
	float alpha = density * {opacity} * smoothstep(0.012, 0.04, dir.z);
	return mix(sky, cloud, alpha);
}
""".format({
		"height_over_scale": ShaderText.number(float(clouds.height_m) / float(clouds.scale_m)),
		"lo_minus": ShaderText.number(lo - softness),
		"lo_plus": ShaderText.number(lo + softness),
		"sun": ShaderText.rgb(sun),
		"to_sun_x": ShaderText.number(to_sun.x * 0.07),
		"to_sun_y": ShaderText.number(to_sun.y * 0.07),
		"lo": ShaderText.number(lo),
		"lo_shadow": ShaderText.number(lo + 0.3),
		"lo_core": ShaderText.number(lo + 0.35),
		"transmit": ShaderText.rgb(Light.transmittance_to_sun(sun, turbidity)),
		"radiance": ShaderText.number(radiance),
		"shade": ShaderText.rgb(StrictJson.vector3(clouds.shade)),
		"base_radiance": ShaderText.number(radiance * 0.7),
		"silver_radiance": ShaderText.number(radiance * 0.6),
		"opacity": ShaderText.number(opacity),
	})
