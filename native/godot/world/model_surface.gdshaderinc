// A model's surface as the browser's models and scenery layers shade it
// (`modelSurface` in `models/modelLayer.ts`, `tierMesh` in
// `scenery/appearance.ts`): albedo is the material's base colour times the
// vertex colour, times its colour scale and its albedo texture, and the
// instance's tint mixed in by the material's tint share. `model_surface.gd`
// prepends the render modes of the material it stands for.
// The instance's tint rides a MultiMesh's custom data. A posed instance's
// palette base rides its alpha: from there each matrix of its palette image
// is three texels, the rows of its 3×4 affine part
// (`units/pose_kernel.compute`). A skinned vertex blends four joints
// (`CUSTOM0`, weights `CUSTOM1`); a rigid one follows one node, `CUSTOM0.x`
// being its slot plus 256 times its track side (1 left, 2 right), and on a
// track scrolls its texture by the palette's first texel, its links' travel
// (`modelVertex`).

#include "res://effects/cast_lights.gdshaderinc"

uniform vec4 base : source_color = vec4(1.0);
uniform float colour_scale = 1.0;
uniform float tint_share = 0.0;
uniform bool has_albedo = false;
uniform sampler2D albedo_texture : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool has_normal = false;
uniform sampler2D normal_texture : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform float normal_scale = 1.0;
uniform float roughness = 1.0;
uniform float metallic = 0.0;
uniform bool has_roughness = false;
uniform sampler2D roughness_texture : hint_default_white, filter_linear_mipmap_anisotropic, repeat_enable;
uniform vec4 roughness_channel = vec4(0.0, 1.0, 0.0, 0.0);
uniform bool has_metallic = false;
uniform sampler2D metallic_texture : hint_default_white, filter_linear_mipmap_anisotropic, repeat_enable;
uniform vec4 metallic_channel = vec4(0.0, 0.0, 1.0, 0.0);
uniform float alpha_cutoff = 0.5;
#if defined(POSED_SKINNED) || defined(POSED_RIGID)
uniform sampler2D palette : filter_nearest;
uniform int palette_width = 1024;

mat4 palette_matrix(int index) {
	int texel = index * 3;
	vec4 r0 = texelFetch(palette, ivec2(texel % palette_width, texel / palette_width), 0);
	vec4 r1 = texelFetch(palette, ivec2((texel + 1) % palette_width, (texel + 1) / palette_width), 0);
	vec4 r2 = texelFetch(palette, ivec2((texel + 2) % palette_width, (texel + 2) / palette_width), 0);
	return mat4(vec4(r0.x, r1.x, r2.x, 0.0), vec4(r0.y, r1.y, r2.y, 0.0), vec4(r0.z, r1.z, r2.z, 0.0), vec4(r0.w, r1.w, r2.w, 1.0));
}
#endif

varying vec3 tint;

void vertex() {
	tint = INSTANCE_CUSTOM.rgb;
#if defined(POSED_SKINNED) || defined(POSED_RIGID)
	int base_matrix = int(INSTANCE_CUSTOM.a + 0.5);
#endif
#ifdef POSED_SKINNED
	mat4 m = palette_matrix(base_matrix + int(CUSTOM0.x)) * CUSTOM1.x
		+ palette_matrix(base_matrix + int(CUSTOM0.y)) * CUSTOM1.y
		+ palette_matrix(base_matrix + int(CUSTOM0.z)) * CUSTOM1.z
		+ palette_matrix(base_matrix + int(CUSTOM0.w)) * CUSTOM1.w;
	VERTEX = (m * vec4(VERTEX, 1.0)).xyz;
	NORMAL = normalize(mat3(m) * NORMAL);
	TANGENT = normalize(mat3(m) * TANGENT);
#endif
#ifdef POSED_RIGID
	int code = int(CUSTOM0.x + 0.5);
	mat4 m = palette_matrix(base_matrix + code % 256);
	VERTEX = (m * vec4(VERTEX, 1.0)).xyz;
	NORMAL = normalize(mat3(m) * NORMAL);
	TANGENT = normalize(mat3(m) * TANGENT);
	int track = code / 256;
	if (track > 0) {
		int texel = base_matrix * 3;
		vec4 scroll = texelFetch(palette, ivec2(texel % palette_width, texel / palette_width), 0);
		UV.x -= track == 1 ? scroll.x : scroll.y;
	}
#endif
}

void fragment() {
	vec4 sampled = has_albedo ? texture(albedo_texture, UV) : vec4(1.0);
	ALBEDO = base.rgb * COLOR.rgb * colour_scale * sampled.rgb * mix(vec3(1.0), tint, tint_share);
	ROUGHNESS = roughness * (has_roughness ? dot(texture(roughness_texture, UV), roughness_channel) : 1.0);
	METALLIC = metallic * (has_metallic ? dot(texture(metallic_texture, UV), metallic_channel) : 1.0);
	EMISSION = cast_light(ALBEDO, NORMAL, VERTEX, INV_VIEW_MATRIX);
	if (has_normal) {
		NORMAL_MAP = texture(normal_texture, UV).rgb;
		NORMAL_MAP_DEPTH = normal_scale;
	}
#ifdef CUTOUT
	// A cutout's coverage rides its normal map's alpha (the browser's
	// `coverageValue`), times the base colour's, sampled through the mip
	// chain and scaled so the cutoff is half there (`cutoutCoverage`). A
	// surface too far to resolve its holes thins by the share of it that is
	// there: the browser dithers that share over its samples, native over
	// pixels, against interleaved gradient noise.
	float coverage = base.a * (has_normal ? texture(normal_texture, UV).a : 1.0);
	ALPHA = clamp(coverage * 0.5 / max(alpha_cutoff, 0.001), 0.0, 1.0);
	ALPHA_SCISSOR_THRESHOLD = max(fract(52.9829189 * fract(dot(FRAGCOORD.xy, vec2(0.06711056, 0.00583715)))), 0.001);
#endif
#ifdef TRANSLUCENT
	ALPHA = base.a * sampled.a;
#endif
}
