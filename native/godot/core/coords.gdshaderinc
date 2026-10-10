// `coords.gd`'s conversion in shader text: sim (x, y, z), Z up, to Godot
// (x, z, -y), Y up, and back. A rotation; nothing is mirrored.

vec3 sim_to_godot(vec3 v) {
	return vec3(v.x, v.z, -v.y);
}

vec3 godot_to_sim(vec3 v) {
	return vec3(v.x, -v.z, v.y);
}
