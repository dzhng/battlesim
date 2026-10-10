extends RefCounted

## Places the camera exactly where a frame record says the browser drew from:
## its eye, target and screen up, through the manifest's lens. Native ports no
## rig logic; clamping, clearance and follow lag are already in the record.

const Coords = preload("res://core/coords.gd")

static func apply(camera: Camera3D, record: Dictionary, lens: Dictionary) -> void:
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.fov = rad_to_deg(float(lens.fovY))
	camera.near = float(lens.near)
	camera.look_at_from_position(Coords.sim_array_to_godot(record.eye), Coords.sim_array_to_godot(record.target), Coords.sim_array_to_godot(record.up))

## Godot has no infinite far plane, so the browser's is replaced by the farthest
## anything is drawn: across the map plus the backdrop's reach beyond it.
static func far_plane(map_size: Array, backdrop_reach_m: float) -> float:
	return Vector2(float(map_size[0]), float(map_size[1])).length() + backdrop_reach_m
