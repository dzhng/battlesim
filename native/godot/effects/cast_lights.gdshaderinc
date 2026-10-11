// The light combat throws on what is round it (`effects/cast_lights.gd`, the
// browser's `castLight` in `world/environment.ts`): every lit world material
// adds it to its emission, so a flash, a burst or a burning wreck lights the
// ground, the grass and the models beside it alike. Each light's irradiance
// falls off inside its radius as `(1 - x²)² / (1 + 6x²)`, its facing wrapped
// by the effects' wrap, as Lambert on the albedo greyed toward its luminance
// and held above a floor. Godot space; the lights are the frame's
// (`cast_lights`, two texels each: position and 1/radius², colour).

global uniform float cast_light_count;
global uniform float cast_light_wrap;
global uniform sampler2D cast_lights : filter_nearest;

// The cast light on a surface of albedo `base` facing `view_normal` at
// `view_position` (view space, as a fragment's NORMAL and VERTEX).
vec3 cast_light(vec3 base, vec3 view_normal, vec3 view_position, mat4 inv_view) {
	int cl_count = int(cast_light_count);
	if (cl_count == 0) {
		return vec3(0.0);
	}
	// Locals are prefixed: the including shader's globals share the scope.
	vec3 cl_at = (inv_view * vec4(view_position, 1.0)).xyz;
	vec3 cl_n = normalize((inv_view * vec4(view_normal, 0.0)).xyz);
	vec3 cl_sum = vec3(0.0);
	for (int cl_i = 0; cl_i < cl_count; cl_i++) {
		vec4 cl_light = texelFetch(cast_lights, ivec2(cl_i * 2, 0), 0);
		vec3 cl_toward = cl_light.xyz - cl_at;
		float cl_d2 = dot(cl_toward, cl_toward);
		float cl_x2 = cl_d2 * cl_light.w;
		if (cl_x2 < 1.0) {
			float cl_falloff = (1.0 - cl_x2) * (1.0 - cl_x2) / (1.0 + 6.0 * cl_x2);
			float cl_facing = dot(cl_n, cl_toward) * inversesqrt(max(cl_d2, 1e-4));
			float cl_wrapped = clamp((cl_facing + cast_light_wrap) / (1.0 + cast_light_wrap), 0.0, 1.0);
			cl_sum += texelFetch(cast_lights, ivec2(cl_i * 2 + 1, 0), 0).rgb * cl_falloff * cl_wrapped;
		}
	}
	float cl_grey = dot(base, vec3(0.2126, 0.7152, 0.0722));
	vec3 cl_albedo = max(mix(base, vec3(cl_grey), 0.35), vec3(0.1));
	return cl_albedo * cl_sum * 0.31830988;
}
