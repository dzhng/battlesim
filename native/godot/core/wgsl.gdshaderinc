// WGSL's smoothstep, which the browser's formulas rely on: defined for any
// edges, where GLSL's is undefined when low >= high.

float ss(float low, float high, float x) {
	float t = clamp((x - low) / (high - low), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}

vec2 ss2(vec2 low, vec2 high, vec2 x) {
	vec2 t = clamp((x - low) / (high - low), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}

vec4 ss4(vec4 low, vec4 high, vec4 x) {
	vec4 t = clamp((x - low) / (high - low), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}
