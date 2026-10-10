extends RefCounted

## The one conversion from sim space to Godot space. The sim is Z-up and
## right-handed in metres; Godot is Y-up and right-handed. East stays +x,
## north becomes -z and up becomes +y: a rotation, so nothing is mirrored.
## Every placement, light direction and camera pose goes through here.

static func sim_to_godot(point: Vector3) -> Vector3:
	return Vector3(point.x, point.z, -point.y)

## The inverse of `sim_to_godot`.
static func godot_to_sim(point: Vector3) -> Vector3:
	return Vector3(point.x, -point.z, point.y)

## `sim_to_godot` of a JSON point `[x, y, z]`.
static func sim_array_to_godot(point: Array) -> Vector3:
	return sim_to_godot(Vector3(float(point[0]), float(point[1]), float(point[2])))

## A sim yaw (from +x toward +y, about +z) as a Godot basis: under this
## conversion it is the same angle about +y.
static func yaw_basis(yaw: float) -> Basis:
	return Basis(Vector3.UP, yaw)
