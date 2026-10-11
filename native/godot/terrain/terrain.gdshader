shader_type spatial;
// The ground, drawn with the browser's ground material (`ground.gdshaderinc`),
// as its terrain fragment and its backdrop fragment draw it (`worldPass.ts`):
// the terrain under the side's learned scars, the backdrop without them.
// `terrain_layer.gd` compiles it with the manifest's defines in place of the
// marker below, and with BACKDROP for the flat land past the map's edge,
// never shadowed and lit flat.
// MANIFEST_DEFINES

#include "res://core/coords.gdshaderinc"
#include "res://terrain/ground.gdshaderinc"
#include "res://effects/cast_lights.gdshaderinc"

varying vec3 sim_position;

void vertex() {
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	sim_position = godot_to_sim(world);
}

void fragment() {
	vec2 xy = sim_position.xy;
	float footprint = length(fwidth(xy));
	uvec4 cell = ground_cell(xy, footprint);
	GroundPaved paved = ground_paved(xy, cell);
	vec4 site = ground_site(xy, cell, paved);
	float water = ground_water(xy, cell);
	vec4 surface = ground_colour(xy, footprint, site, paved, water);
#ifndef BACKDROP
	ScarSample scar = ground_scars_seen(sim_position, godot_to_sim(CAMERA_POSITION_WORLD), footprint);
	surface = scarred_surface(surface, scar);
#endif
	ALBEDO = surface.rgb;
	ROUGHNESS = surface.a;
#ifndef BACKDROP
	// A river's bank is lit as the cross-section it was cut to, and a road's
	// ruts and curb tilt it, as the scars do (`groundBank`,
	// `groundRoadRelief`, `scarredNormal`).
	vec3 n = godot_to_sim(normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz));
	vec3 bank = ground_bank(xy, cell, footprint);
	if (bank.z > 0.0) {
		n = normalize(mix(n, normalize(vec3(-bank.x, -bank.y, 1.0)), bank.z));
	}
	vec2 relief = ground_road_relief(xy, footprint, paved);
	n = normalize(n - vec3(relief, 0.0) * n.z);
	n = scarred_normal(n, scar);
	NORMAL = normalize((VIEW_MATRIX * vec4(sim_to_godot(n), 0.0)).xyz);
#endif
	EMISSION = cast_light(ALBEDO, NORMAL, VERTEX, INV_VIEW_MATRIX);
}
