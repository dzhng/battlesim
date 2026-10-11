extends RefCounted

## What a frame's camera means for level of detail, as the browser's
## `frame/detailView.ts` and `frame/staticChunks.ts` read it: where the eye is,
## how many pixels a metre covers a metre away, the view's four side planes
## (near and far are not tested), and where the sun's shadows fall and are
## still received. Every layer that picks tiers or culls by chunk reads one.
## Godot space throughout; the sim's boxes go through `sim_box`.

const Coords = preload("res://core/coords.gd")

var eye: Vector3
## Viewport pixels per metre at one metre's distance: `height / (2·tan(fovY/2))`.
var pixels_per_metre: float
## The side planes through the eye, facing out: a point beyond one is out of view.
var sides: Array[Plane] = []
## The ground distance a metre of height throws its shadow, and the farthest
## distance from the eye at which a shadow is still received (the cascades'
## `max_far_m` in view depth, farther at the view's corners).
var fall: Vector3
var reach: float

## The view of `camera` drawing into a viewport `size` pixels big, under a sun
## in direction `sun` (unit, toward the sun, Godot space) whose cascades reach
## `max_far_m`.
static func of(camera: Camera3D, size: Vector2, sun: Vector3, max_far_m: float):
	var view = new()
	view.eye = camera.global_position
	var tan_v := tan(deg_to_rad(camera.fov) / 2.0)
	var tan_h := tan_v * size.x / size.y
	var basis := camera.global_basis
	# The camera looks down -z.
	for normal in [Vector3(-1, 0, tan_h), Vector3(1, 0, tan_h), Vector3(0, 1, tan_v), Vector3(0, -1, tan_v)]:
		view.sides.append(Plane((basis * normal).normalized(), view.eye))
	view.pixels_per_metre = size.y / (2.0 * tan_v)
	view.fall = Vector3(-sun.x, 0.0, -sun.z) / maxf(sun.y, 1e-3)
	view.reach = max_far_m * Vector3(1.0, tan_v, tan_h).length()
	return view

## Whether a box meets the view between its side planes: no plane has the
## whole box beyond it.
func box_meets(box: AABB) -> bool:
	for plane in sides:
		var nearest := Vector3(
			box.position.x if plane.normal.x > 0.0 else box.end.x,
			box.position.y if plane.normal.y > 0.0 else box.end.y,
			box.position.z if plane.normal.z > 0.0 else box.end.z)
		if plane.distance_to(nearest) > 0.0:
			return false
	return true

## Whether a sphere meets the view between its side planes.
func sphere_meets(center: Vector3, radius: float) -> bool:
	for plane in sides:
		if plane.distance_to(center) > radius:
			return false
	return true

## The eye's distance to the farthest corner of `box`.
func farthest(box: AABB) -> float:
	var far := Vector3(
		box.position.x if eye.x - box.position.x > box.end.x - eye.x else box.end.x,
		box.position.y if eye.y - box.position.y > box.end.y - eye.y else box.end.y,
		box.position.z if eye.z - box.position.z > box.end.z - eye.z else box.end.z)
	return eye.distance_to(far)

## Whether all of `box` lies within the view's side planes.
func box_within(box: AABB) -> bool:
	for plane in sides:
		var farthest_out := Vector3(
			box.end.x if plane.normal.x > 0.0 else box.position.x,
			box.end.y if plane.normal.y > 0.0 else box.position.y,
			box.end.z if plane.normal.z > 0.0 else box.position.z)
		if plane.distance_to(farthest_out) > 0.0:
			return false
	return true

## The eye's distance to the nearest point of `box`.
func distance_to(box: AABB) -> float:
	return eye.distance_to(eye.clamp(box.position, box.end))

## Whether a box out of view can cast into it: swept along the fall of a
## shadow `height` metres tall, it meets the view within the shadows' reach.
func casts_into_view(box: AABB, height: float) -> bool:
	var swept := box.merge(AABB(box.position + fall * height, box.size))
	return distance_to(swept) <= reach and box_meets(swept)

## The sim box `lo`..`hi` in Godot space.
static func sim_box(lo: Vector3, hi: Vector3) -> AABB:
	var a := Coords.sim_to_godot(lo)
	var b := Coords.sim_to_godot(hi)
	return AABB(a.min(b), (a - b).abs())
