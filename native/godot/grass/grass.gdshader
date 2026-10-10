shader_type spatial;
render_mode world_vertex_coords, cull_disabled;
// The browser's grass draw (`grassPass.ts` `grassVertexOf` and its fragment),
// ported as text: each instance is one clump the build wrote, each vertex one
// vertex of its kind's blade strips. `grass_layer.gd` compiles it with the
// grass params' field vec4s (`GRASS_<field>`) and `TUNING_<path>` for
// `tuning.grass` in place of the marker below.
// MANIFEST_DEFINES

uniform vec4 grass_params[GRASS_vec4s];
uniform sampler2D grass_shapes : filter_nearest;
uniform int tier;
uniform int blade_vertices;

varying vec3 albedo;

// Word `i` of a table (`core/data_texture.gd`).
uint shape_word(int i) {
	int width = textureSize(grass_shapes, 0).x;
	uvec4 b = uvec4(round(texelFetch(grass_shapes, ivec2(i % width, i / width), 0) * 255.0));
	return b.r | (b.g << 8u) | (b.b << 16u) | (b.a << 24u);
}

vec4 shape_vec4(int vertex, int field) {
	int at = vertex * 16 + field * 4;
	return vec4(uintBitsToFloat(shape_word(at)), uintBitsToFloat(shape_word(at + 1)), uintBitsToFloat(shape_word(at + 2)), uintBitsToFloat(shape_word(at + 3)));
}

vec3 sim_to_godot(vec3 v) {
	return vec3(v.x, v.z, -v.y);
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
		float scale = height / max(row.x, 1e-4);
		float cs = cos(yaw);
		float sn = sin(yaw);
		vec3 sp = spine4.xyz * scale;
		vec3 sd = side4.xyz * scale * width;
		vec3 spine = vec3(sp.x * cs - sp.y * sn, sp.x * sn + sp.y * cs, sp.z);
		vec3 side = vec3(sd.x * cs - sd.y * sn, sd.x * sn + sd.y * cs, sd.z);
		vec3 world = root + spine + side;
		vec3 n = normal4.xyz;
		VERTEX = sim_to_godot(world);
		NORMAL = sim_to_godot(vec3(n.x * cs - n.y * sn, n.x * sn + n.y * cs, n.z));
		albedo = r2.xyz * r2.xyz;
	}
}

void fragment() {
	ALBEDO = albedo;
	ROUGHNESS = TUNING_blade_roughness;
}
