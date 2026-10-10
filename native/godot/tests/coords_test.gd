extends "res://tests/check.gd"

const Coords = preload("res://core/coords.gd")
const CameraRig = preload("res://reel/camera_rig.gd")

func run() -> bool:
	# Asymmetric known points: sim is Z-up and right-handed, Godot Y-up and right-handed.
	check(Coords.sim_to_godot(Vector3(1, 2, 3)) == Vector3(1, 3, -2), "(1, 2, 3) lands at (1, 3, -2)")
	check(Coords.sim_to_godot(Vector3(5, 0, 0)) == Vector3(5, 0, 0), "east stays +x")
	check(Coords.sim_to_godot(Vector3(0, 7, 0)) == Vector3(0, 0, -7), "north becomes -z")
	check(Coords.sim_to_godot(Vector3(0, 0, 4)) == Vector3(0, 4, 0), "up becomes +y")
	var basis := Basis(Coords.sim_to_godot(Vector3(1, 0, 0)), Coords.sim_to_godot(Vector3(0, 1, 0)), Coords.sim_to_godot(Vector3(0, 0, 1)))
	check(is_equal_approx(basis.determinant(), 1.0), "the conversion is a rotation, not a reflection (determinant %f)" % basis.determinant())

	# A sim yaw turns +x toward +y about +z; its basis turns the converted point the same way.
	var point := Vector3(3, 1, 2)
	for yaw in [0.3, -1.2, 2.9]:
		var turned := Vector3(point.x * cos(yaw) - point.y * sin(yaw), point.x * sin(yaw) + point.y * cos(yaw), point.z)
		check((Coords.yaw_basis(yaw) * Coords.sim_to_godot(point)).is_equal_approx(Coords.sim_to_godot(turned)), "yaw %.1f turns a point as the sim does" % yaw)

	# The browser's screen right is forward x up. Looking north from the origin,
	# a point to the east must land on the right of the frame and one above on top.
	var root_node := Node3D.new()
	root.add_child(root_node)
	var camera := Camera3D.new()
	root_node.add_child(camera)
	var lens := {"fovY": 0.8, "near": 1.0}
	CameraRig.apply(camera, {"eye": [0.0, 0.0, 2.0], "target": [0.0, 10.0, 2.0], "up": [0.0, 0.0, 1.0]}, lens)
	var to_view := camera.global_transform.affine_inverse()
	check((to_view * Coords.sim_to_godot(Vector3(3, 10, 2))).x > 0.0, "a point east of a north-facing eye is on the right")
	check((to_view * Coords.sim_to_godot(Vector3(0, 10, 5))).y > 0.0, "a point above the target is at the top")
	check((to_view * Coords.sim_to_godot(Vector3(0, 10, 2))).z < 0.0, "the target is in front of the camera")
	check(is_equal_approx(camera.fov, rad_to_deg(0.8)) and camera.keep_aspect == Camera3D.KEEP_HEIGHT, "the lens fovY is the vertical field of view")
	check(camera.near == 1.0, "the near plane is the lens near")
	root_node.free()

	# A flat ring is wound to face up, with up normals, whatever its input order.
	var LegacyMapLayer = load("res://world/legacy_map_layer.gd")
	for ring in [[[0, 0], [4, 0], [4, 3], [0, 3]], [[0, 0], [0, 3], [4, 3], [4, 0]]]:
		var mesh: ArrayMesh = LegacyMapLayer._polygon_mesh([ring])
		var arrays := mesh.surface_get_arrays(0)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		var facing_up := true
		for i in range(0, indices.size(), 3):
			var a := vertices[indices[i]]
			# Godot draws clockwise triangles as front faces: seen from above,
			# an up-facing triangle's (b - a) x (c - a) points down.
			facing_up = facing_up and (vertices[indices[i + 1]] - a).cross(vertices[indices[i + 2]] - a).y < 0.0
		check(facing_up, "every ring triangle faces up for %s" % [ring])
		check(Array(arrays[Mesh.ARRAY_NORMAL]).all(func(n): return n.dot(Vector3.UP) > 0.9999), "ring normals point up")
		check(vertices.has(Vector3(4, 0, -3)), "ring corners are converted points")
	return true
