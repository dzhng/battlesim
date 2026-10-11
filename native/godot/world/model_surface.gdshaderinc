// A model's surface as the browser's models and scenery layers shade it
// (`modelSurface` in `models/modelLayer.ts`, `tierMesh` in
// `scenery/appearance.ts`): albedo is the material's base colour times the
// vertex colour, times its colour scale and its albedo texture, and the
// instance's tint mixed in by the material's tint share. `model_surface.gd`
// prepends the render modes of the material it stands for.
// The instance's tint rides a MultiMesh's custom data.

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

varying vec3 tint;

void vertex() {
	tint = INSTANCE_CUSTOM.rgb;
}

void fragment() {
	vec4 sampled = has_albedo ? texture(albedo_texture, UV) : vec4(1.0);
	ALBEDO = base.rgb * COLOR.rgb * colour_scale * sampled.rgb * mix(vec3(1.0), tint, tint_share);
	ROUGHNESS = roughness * (has_roughness ? dot(texture(roughness_texture, UV), roughness_channel) : 1.0);
	METALLIC = metallic * (has_metallic ? dot(texture(metallic_texture, UV), metallic_channel) : 1.0);
	if (has_normal) {
		NORMAL_MAP = texture(normal_texture, UV).rgb;
		NORMAL_MAP_DEPTH = normal_scale;
	}
#ifdef CUTOUT
	ALPHA = base.a * sampled.a;
	ALPHA_SCISSOR_THRESHOLD = alpha_cutoff;
#endif
#ifdef TRANSLUCENT
	ALPHA = base.a * sampled.a;
#endif
}
