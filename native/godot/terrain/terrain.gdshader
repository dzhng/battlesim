shader_type spatial;
// The ground, drawn with the browser's ground material (`ground.gdshaderinc`),
// as its terrain fragment and its backdrop fragment draw it (`worldPass.ts`).
// `terrain_layer.gd` compiles it with the manifest's defines in place of the
// marker below, and one of these:
// - BACKDROP: the flat land past the map's edge, never shadowed and lit flat;
// - CLASS_VIEW: the ground's classes (`groundClasses`) as exact bytes, unlit,
//   in place of its colour; the backdrop is black there, "not ground".
// MANIFEST_DEFINES

#include "res://terrain/ground.gdshaderinc"

varying vec3 sim_position;

vec3 sim_to_godot(vec3 v) {
	return vec3(v.x, v.z, -v.y);
}

vec3 godot_to_sim(vec3 v) {
	return vec3(v.x, -v.z, v.y);
}

void vertex() {
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	sim_position = godot_to_sim(world);
}

void fragment() {
	vec2 xy = sim_position.xy;
	float footprint = length(fwidth(xy));
#if defined(CLASS_VIEW) && defined(BACKDROP)
	ALBEDO = vec3(0.0);
#else
	uvec4 cell = ground_cell(xy, footprint);
	GroundPaved paved = ground_paved(xy, cell);
	vec4 site = ground_site(xy, cell, paved);
	float water = ground_water(xy, cell);
#ifdef CLASS_VIEW
	// The bytes are data: undo the output's sRGB encoding so they land exact.
	vec3 bytes = ground_classes(xy, footprint, site, water);
	ALBEDO = mix(pow((bytes + 0.055) / 1.055, vec3(2.4)), bytes / 12.92, lessThan(bytes, vec3(0.04045)));
#else
	vec4 surface = ground_colour(xy, footprint, site, paved, water);
	ALBEDO = surface.rgb;
	ROUGHNESS = surface.a;
#ifndef BACKDROP
	// A river's bank is lit as the cross-section it was cut to, and a road's
	// ruts and curb tilt it (`groundBank`, `groundRoadRelief`).
	vec3 n = godot_to_sim(normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz));
	vec3 bank = ground_bank(xy, cell, footprint);
	if (bank.z > 0.0) {
		n = normalize(mix(n, normalize(vec3(-bank.x, -bank.y, 1.0)), bank.z));
	}
	vec2 relief = ground_road_relief(xy, footprint, paved);
	n = normalize(n - vec3(relief, 0.0) * n.z);
	NORMAL = normalize((VIEW_MATRIX * vec4(sim_to_godot(n), 0.0)).xyz);
#endif
#endif
#endif
}
