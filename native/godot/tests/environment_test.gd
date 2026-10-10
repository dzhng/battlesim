extends "res://tests/check.gd"

## The sun's cascades are split over the part of the map the camera sees, as
## the browser's are: a high camera spends none of them on the air above the
## map, and no shadow reaches past the light's shadow reach.

const EnvironmentAdapter = preload("res://environment/environment_adapter.gd")
const Coords = preload("res://core/coords.gd")

const VIEWPORT := Vector2(1280, 720)

func run() -> bool:
	var light: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json")).light
	var receivers := AABB(Vector3(0, -1, -1000), Vector3(1000, 26, 1000))
	var environment := EnvironmentAdapter.new(light, true, receivers)
	root.add_child(environment)
	var sun: DirectionalLight3D = environment.find_child("Sun", true, false)
	check(root.msaa_3d == Viewport.MSAA_4X, "the world is drawn at the browser's 4x MSAA")
	check(is_equal_approx(sun.shadow_opacity, 1.0 - float(light.shadow_floor)), "a sun-shadowed surface keeps the shadow floor's share of the sun")

	var camera := Camera3D.new()
	root.add_child(camera)
	camera.fov = 45.0
	camera.near = 1.0

	# Straight down from 1 km: the map's top is 975 m away.
	camera.look_at_from_position(Coords.sim_to_godot(Vector3(500, 500, 1000)), Coords.sim_to_godot(Vector3(500, 500, 0)), Coords.sim_to_godot(Vector3(0, 1, 0)))
	environment.prepare(camera, VIEWPORT, Vector3.ZERO)
	var first_split := sun.directional_shadow_split_1 * sun.directional_shadow_max_distance
	check(first_split > 975.0, "a camera 1 km up starts its first cascade at the map, not the near plane: it ends at %.1f m" % first_split)
	# The browser keeps the range at least half again its start.
	check(sun.directional_shadow_max_distance <= 1.5 * 1001.0, "the cascades end near the map: %.1f m" % sun.directional_shadow_max_distance)

	# A low oblique view past the shadow reach is cut at the reach.
	camera.look_at_from_position(Coords.sim_to_godot(Vector3(0, 0, 30)), Coords.sim_to_godot(Vector3(1000, 1000, 0)), Coords.sim_to_godot(Vector3(0, 0, 1)))
	environment.prepare(camera, VIEWPORT, Vector3.ZERO)
	check(sun.directional_shadow_max_distance <= float(light.cascades.max_far_m), "no shadow past the reach: %.1f m" % sun.directional_shadow_max_distance)
	var splits := [sun.directional_shadow_split_1, sun.directional_shadow_split_2, sun.directional_shadow_split_3, 1.0]
	for i in 3:
		check(splits[i] > 0.0 and splits[i] < splits[i + 1], "the splits rise toward the reach: %s" % [splits])
	check(sun.shadow_blur > 0.0, "the penumbra is soft")
	environment.free()
	camera.free()
	return true
