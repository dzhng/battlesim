extends RefCounted

## The browser's physical sky (`shaders/physicalSky.ts`), ported by formula into
## shader text with the manifest's numbers written in, as the browser writes
## its own. Every function takes and returns sim-space (z up) directions. The
## text is plain GLSL that both Godot's shading language and RenderingDevice
## compute shaders accept.

const Light = preload("res://environment/light.gd")

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

static func f(value: float) -> String:
	return "%.12f" % value

static func rgb(value: Vector3) -> String:
	return "vec3(%s, %s, %s)" % [f(value.x), f(value.y), f(value.z)]

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
		"beta_r": rgb(Light.BETA_RAYLEIGH),
		"beta_m_scatter": f(Light.BETA_MIE_SCATTER * mie),
		"beta_m_extinction": f(Light.BETA_MIE_EXTINCTION * mie),
		"sun": rgb(sun),
		"r0": f(r0),
		"r0_2": f(r0 * r0),
		"top2": f(top2),
		"phase_r": f(3.0 / (16.0 * PI)),
		"phase_m": f((1.0 - g2) / (4.0 * PI)),
		"one_g2": f(1.0 + g2),
		"two_g": f(2.0 * Light.MIE_G),
		"planet": f(Light.PLANET_RADIUS_KM),
		"planet2": f(Light.PLANET_RADIUS_KM * Light.PLANET_RADIUS_KM),
		"neg_rayleigh": f(-Light.RAYLEIGH_SCALE_KM),
		"neg_mie": f(-Light.MIE_SCALE_KM),
		"ms_tint": rgb(MS_TINT),
		"ms": f(MS_FLOOR * sqrt(maxf(sun.z, 0.0)) / (4.0 * PI)),
		"aureole_outer": f(LOW_SUN_AUREOLE_COS_OUTER),
		"aureole_inner": f(LOW_SUN_AUREOLE_COS_INNER),
		"aureole": f(low_sun_aureole_strength(sun.z, overcast)),
		"radiance_scale": f(float(light.sky.radiance)),
		"transmit": rgb(transmit),
		"overcast_zenith": rgb(OVERCAST_ZENITH_RADIANCE),
		"overcast_sun": f(sqrt(maxf(sun.z, 0.05))),
		"overcast": f(overcast),
		"bounce": rgb(bounce),
	})
