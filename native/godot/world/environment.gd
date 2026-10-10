extends Node3D

## The light the parity run draws under: a sun with shadows, a sky (when the
## manifest lists the `sky` layer) and the ambient term. The values are the
## native approximation the light slices replace.

func _init(sky_listed: bool) -> void:
	name = "Environment"
	var env := Environment.new()
	if sky_listed:
		env.background_mode = Environment.BG_SKY
		var sky := Sky.new()
		var sky_material := ShaderMaterial.new()
		sky_material.shader = Shader.new()
		sky_material.shader.code = SKY_SHADER
		sky.sky_material = sky_material
		env.sky = sky
	else:
		env.background_mode = Environment.BG_CLEAR_COLOR
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color("8aa4bc")
	env.ambient_light_energy = 0.8
	env.fog_enabled = true
	env.fog_light_color = Color("a9b7b0")
	env.fog_light_energy = 0.32
	env.fog_density = 0.00065
	env.fog_sky_affect = 0.28
	var world := WorldEnvironment.new()
	world.environment = env
	add_child(world)
	var sun := DirectionalLight3D.new()
	sun.name = "Sun"
	sun.rotation_degrees = Vector3(-42.0, 20.0, 0.0)
	sun.light_color = Color("fff0d0")
	sun.light_energy = 2.2
	sun.shadow_enabled = true
	add_child(sun)

const SKY_SHADER := """
shader_type sky;
render_mode use_debanding;

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
    float horizon = smoothstep(-0.10, 0.58, direction.y);
    vec3 horizon_color = vec3(0.71, 0.82, 0.80);
    vec3 zenith_color = vec3(0.17, 0.42, 0.62);
    vec3 clear = mix(horizon_color, zenith_color, horizon);
    // Browser fixture: coverage .42, height 1600 m, scale 600 m,
    // softness .05 and opacity .95. The projection is view-direction based
    // because the native sky has no terrain plot texture to anchor clouds.
    float cloud_band = smoothstep(0.10, 0.72, direction.y) * (1.0 - smoothstep(0.80, 0.98, direction.y));
    vec2 cloud_uv = direction.xz / max(direction.y + 0.08, 0.18) * 0.92;
    float cloud = cloud_fbm(cloud_uv * 1.65 + vec2(4.2, -2.7));
    float cloud_mask = smoothstep(0.53, 0.61, cloud) * cloud_band;
    vec3 cloud_color = vec3(0.96, 0.95, 0.88);
    clear = mix(clear, cloud_color, cloud_mask * 0.95);
    COLOR = clear;
}
"""
