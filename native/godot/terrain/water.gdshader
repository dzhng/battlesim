shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled;
// The water surface, as the browser's world pass draws it (`worldPass.ts`
// `waterFragment`): the ground material's `water_surface` and `water_normal`
// on the water cells, blended over the ground, reading depth and writing
// none. `terrain_layer.gd` compiles it with the manifest's defines in place
// of the marker below.
// MANIFEST_DEFINES

#include "res://core/coords.gdshaderinc"
#include "res://terrain/ground.gdshaderinc"

varying vec3 sim_position;

void vertex() {
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	sim_position = godot_to_sim(world);
}

void fragment() {
	vec2 xy = sim_position.xy;
	float footprint = length(fwidth(xy));
	vec4 surface = water_surface(xy, footprint);
	ALBEDO = surface.rgb;
	ALPHA = surface.a;
	ROUGHNESS = TUNING_water_roughness;
	NORMAL = normalize((VIEW_MATRIX * vec4(sim_to_godot(water_normal(xy, footprint)), 0.0)).xyz);
}
