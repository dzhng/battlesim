shader_type spatial;
render_mode world_vertex_coords, cull_disabled;
// The browser's grass draw (`grassPass.ts` `grassVertexOf` and its fragment),
// ported as text: each instance is one clump the build wrote, each vertex one
// vertex of its kind's blade strips, placed, scaled, turned and bent in the
// one wind. `grass_layer.gd` compiles it with the grass params' field vec4s
// (`GRASS_<field>`) and `TUNING_<path>` for `tuning.grass` in place of the
// marker below. Positions are in sim metres until they are handed to Godot.
// MANIFEST_DEFINES

uniform vec4 grass_params[GRASS_vec4s];
uniform sampler2D grass_shapes : filter_nearest;
uniform int tier;
uniform int blade_vertices;
// The frame's interpolated sim time (the wind's clock) and metres per pixel
// per metre of distance.
uniform float sim_time;
uniform float pixel_scale;

varying vec3 albedo;
varying vec3 blade_normal;
varying vec3 world_position;
// How far the fragment is lit as the ground under it, not as a blade.
varying float ground;

#include "res://core/word_tables.gdshaderinc"
#include "res://core/coords.gdshaderinc"
#include "res://core/wgsl.gdshaderinc"
#include "res://effects/cast_lights.gdshaderinc"

// Field `field` (a vec4) of shape vertex `vertex`.
vec4 shape_vec4(int vertex, int field) {
	return table_vec4(grass_shapes, vertex * 16 + field * 4);
}

void vertex() {
	// The clump: the build's three rows, which the MultiMesh hands over as the
	// instance transform's rows.
	vec4 r0 = vec4(MODEL_MATRIX[0][0], MODEL_MATRIX[1][0], MODEL_MATRIX[2][0], MODEL_MATRIX[3][0]);
	vec4 r1 = vec4(MODEL_MATRIX[0][1], MODEL_MATRIX[1][1], MODEL_MATRIX[2][1], MODEL_MATRIX[3][1]);
	vec4 r2 = vec4(MODEL_MATRIX[0][2], MODEL_MATRIX[1][2], MODEL_MATRIX[2][2], MODEL_MATRIX[3][2]);
	vec3 root = r0.xyz;
	float height = r0.w;
	float yaw = r1.x;
	float width = r1.y;
	int kind = int(r1.z);
	vec4 row = grass_params[GRASS_kinds + kind];
	int vid = VERTEX_ID;
	if (vid / blade_vertices >= int(row.y)) {
		// A blade this kind does not have: collapsed to a point, never drawn.
		VERTEX = vec3(0.0);
	} else {
		vec4 bases = grass_params[GRASS_bases + kind];
		int base = int(tier == 0 ? bases.x : bases.y);
		vec4 spine4 = shape_vec4(base + vid, 0);
		vec4 side4 = shape_vec4(base + vid, 1);
		vec4 normal4 = shape_vec4(base + vid, 2);
		vec4 tint4 = shape_vec4(base + vid, 3);
		float scale = height / max(row.x, 1e-4);
		float cs = cos(yaw);
		float sn = sin(yaw);
		vec3 sp = spine4.xyz * scale;
		vec3 sd = side4.xyz * scale * width;
		vec3 spine = vec3(sp.x * cs - sp.y * sn, sp.x * sn + sp.y * cs, sp.z);
		vec3 side = vec3(sd.x * cs - sd.y * sn, sd.x * sn + sd.y * cs, sd.z);
		// The one wind: a steady lean, gust fronts rolling downwind, a flutter.
		vec4 wind = grass_params[GRASS_wind];
		vec4 gusts = grass_params[GRASS_gusts];
		float t = sim_time;
		vec2 dir = wind.xy;
		float along = dot(root.xy, dir);
		float front = sin(6.2831853 * (along - t * gusts.y) * gusts.x);
		float gust = ss(TUNING_wind_gustFrom, 1.0, front);
		float flutter = gusts.z * sin(gusts.w * t + side4.w * 6.2831853 + along * TUNING_wind_flutterPhase);
		float bend = spine4.w * spine4.w;
		// Laid over by tracks or trampling (the clump's last row word; a stiff
		// kind answers the wind less, its shape's own response).
		float flat_share = 1.0 - r1.w;
		float answer = tint4.w * (1.0 - flat_share);
		float lean = (wind.z + wind.w * gust + flutter) * answer;
		vec2 push = dir * lean + vec2(-dir.y, dir.x) * flutter * TUNING_wind_flutterAcross * answer;
		vec2 laid = vec2(cs, sn) * flat_share * TUNING_laid_lean;
		vec3 world = root + vec3(spine.xy, spine.z * (1.0 - flat_share * TUNING_laid_sink)) + side + vec3((push + laid) * height * bend, 0.0);
		world.z -= 0.5 * dot(push, push) * height * bend;
		vec3 n = normal4.xyz;
		VERTEX = sim_to_godot(world);
		world_position = world;
		blade_normal = vec3(n.x * cs - n.y * sn, n.x * sn + n.y * cs, n.z);
		POSITION = PROJECTION_MATRIX * (VIEW_MATRIX * vec4(VERTEX, 1.0));
		// Blades cross: a per-blade nudge of up to a two-thousandth of the
		// depth separates every such pair the same way each frame.
		uint tie = ((floatBitsToUint(root.x) * 2654435761u) ^ (floatBitsToUint(root.y) * 2246822519u)) + uint(vid / blade_vertices) * 374761393u;
		POSITION.z *= 1.0 + 1e-3 * (float(tie >> 8u) / 16777216.0 - 0.5);
		// A clump a few pixels tall shows its tops, not its roots: its tint and
		// its lighting give way to the ground's.
		vec4 shading = grass_params[GRASS_shading];
		vec4 tiers = grass_params[GRASS_tiers];
		float footprint = pixel_scale * distance(godot_to_sim(CAMERA_POSITION_WORLD), root);
		float plain = shading.x * ss(tiers.z, tiers.w, footprint);
		albedo = r2.xyz * r2.xyz * mix(tint4.xyz, vec3(1.0), plain);
		ground = mix(1.0 - shading.y, 1.0, plain);
	}
}

void fragment() {
	vec3 eye = godot_to_sim(CAMERA_POSITION_WORLD);
	vec3 up = vec3(0.0, 0.0, 1.0);
	vec3 n = normalize(blade_normal);
	if (dot(n, eye - world_position) < 0.0) {
		n = -n;
	}
	// A blade's shading normal leans to the ground's, so the field lights like
	// the ground it grows on and never glitters.
	vec3 shading = normalize(mix(n, up, ground));
	NORMAL = normalize((VIEW_MATRIX * vec4(sim_to_godot(shading), 0.0)).xyz);
	ALBEDO = albedo;
	ROUGHNESS = TUNING_blade_roughness;
	EMISSION = cast_light(ALBEDO, NORMAL, VERTEX, INV_VIEW_MATRIX);
}
