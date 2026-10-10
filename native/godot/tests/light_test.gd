extends "res://tests/check.gd"

## The manifest's light is read without defaults, and the ported formulas give
## the browser's numbers. The expected values come from the TypeScript owners
## (`skyParameters.ts`, `cascadePolicy.ts`) evaluated on the same inputs.

const Light = preload("res://environment/light.gd")
const Aerial = preload("res://environment/aerial.gd")

func run() -> bool:
	var light: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json")).light
	check(Light.error(light).is_empty(), "the fixture's light is admitted: %s" % Light.error(light))

	# Every field is required, by its path; nothing is filled in.
	for path in _leaf_paths(Light.SCHEMA, []):
		var broken := light.duplicate(true)
		_parent(broken, path).erase(path[-1])
		var why := Light.error(broken)
		check(why.contains(".".join(path)), "a light without %s is refused by name, not defaulted: %s" % [".".join(path), why])
	var extra := light.duplicate(true)
	extra.grade["gamma"] = 2.2
	check(Light.error(extra).contains("grade.gamma"), "a field the browser does not write is refused")
	var wrong := light.duplicate(true)
	wrong.haze.mist_height_m = [8]
	check(not Light.error(wrong).is_empty(), "a pair with one number is refused")
	wrong = light.duplicate(true)
	wrong.sky.turbidity = "2"
	check(Light.error(wrong).contains("sky.turbidity"), "a string where a number belongs is refused")
	wrong = light.duplicate(true)
	wrong.cascades.count = 3
	check(Light.error(wrong).contains("cascades.count"), "a cascade count other than the receiver layout's four is refused")

	# The sun's colour is the browser's: a clear noon-ish sun is warm white, a
	# low sun in thicker air is orange and partly greyed by overcast.
	var sun := _with_sun(light, -0.35, 0.7, 2.0)
	_near(Light.sun_color(sun), Vector3(1, 0.9106228791334504, 0.7192635955758356), "the sun colour at elevation 0.7 and turbidity 2")
	_near(Light.sun_color(_with_sun(light, -0.35, 0.1, 5.0)), Vector3(0.905710983444657, 0.5929681303720696, 0.2724214870373374), "the sun colour at elevation 0.1 and turbidity 5")
	_near(Light.transmittance_to_sun(Vector3(0.6, 0, 0.8), 7.0), Vector3(1, 0.9273048152243014, 0.7667193749879522), "the transmittance toward the sun at turbidity 7")

	# The haze thickens with turbidity (and fog above its onset), and clear air
	# widens the horizon blend.
	var hazy := _with_sun(light, -0.35, 0.7, 2.0)
	hazy.haze.distance_scale = 3.0
	_near(Aerial.extinction(hazy), Vector3(0.030726, 0.053994, 0.11262), "the extinction at turbidity 2")
	check(absf(Aerial.visibility_km(hazy) - 59.4709638187899) < 1e-6, "the visibility at turbidity 2 is %f km" % Aerial.visibility_km(hazy))
	check(Aerial.horizon_fade(hazy).is_equal_approx(Vector2(-0.38, 0.14)), "clear air widens the horizon blend fully")
	hazy.sky.turbidity = 6.0
	_near(Aerial.extinction(hazy), Vector3(0.188006, 0.211274, 0.2699), "the extinction at turbidity 6, with fog")
	check(Aerial.horizon_fade(hazy).distance_to(Vector2(-0.25948235153471416, 0.09179294061388565)) < 1e-6, "hazier air narrows the horizon blend: %s" % Aerial.horizon_fade(hazy))

	var breaks := Light.cascade_breaks(40.0, 900.0, 4, 0.5)
	var expected := [0.15239392063596352, 0.3370561974477342, 0.5919969723250337, 1.0]
	for i in 4:
		check(absf(breaks[i] - expected[i]) < 1e-9, "cascade break %d is the browser's practical split: %s" % [i, breaks])
	return true

## `light` with the sun at `azimuth` and `elevation` and the given turbidity;
## the direction is written the way the browser derives it.
static func _with_sun(light: Dictionary, azimuth: float, elevation: float, turbidity: float) -> Dictionary:
	var result := light.duplicate(true)
	result.sun_azimuth = azimuth
	result.sun_elevation = elevation
	result.sky.turbidity = turbidity
	result.sunDirection = [cos(elevation) * cos(azimuth), cos(elevation) * sin(azimuth), sin(elevation)]
	return result

func _near(actual: Vector3, expected: Vector3, what: String) -> void:
	check(actual.distance_to(expected) < 1e-5, "%s is %s, not %s" % [what, actual, expected])

static func _leaf_paths(schema: Dictionary, prefix: Array) -> Array:
	var paths := []
	for key in schema:
		var path := prefix + [key]
		paths.append(path)
		if typeof(schema[key]) == TYPE_DICTIONARY:
			paths.append_array(_leaf_paths(schema[key], path))
	return paths

static func _parent(root: Dictionary, path: Array) -> Dictionary:
	var at := root
	for key in path.slice(0, -1):
		at = at[key]
	return at
