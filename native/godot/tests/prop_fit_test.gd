extends "res://tests/check.gd"

## The known props' fit (`world/prop_layer.gd` `fit`, the port of
## `PropAppearances.fit`): a wreck is its own unit's wreck, anything else the
## candidate whose footprint is nearest its box, scaled to it; a module repeats
## along the box's long side; a kind with no art stands in as a tinted box; a
## car's paint follows the authored prop it is, as the browser draws it.

const PropLayer = preload("res://world/prop_layer.gd")

## Twelve paints, each its index in red, so a model's red names its paint.
var paints := range(12).map(func(i): return [i / 100.0, 0.0, 0.0])
var table := {
	"kinds": {
		"parked_car": {"modular": false, "wreck": false, "standIn": null, "candidates": [
			{"look": 0, "footprint": [2.2, 0.9, 0.72], "paints": paints},
			{"look": 1, "footprint": [4.0, 1.2, 1.5], "paints": null},
		]},
		"heavy_wreck": {"modular": false, "wreck": true, "standIn": null, "candidates": []},
		"wall": {"modular": true, "wreck": false, "standIn": null, "candidates": [{"look": 2, "footprint": [2.0, 0.3, 0.8], "paints": null}]},
		"kiosk": {"modular": false, "wreck": false, "standIn": [0.5, 0.25, 0.0], "candidates": []},
		"trunk": {"modular": false, "wreck": false, "standIn": null, "candidates": []},
	},
	"wrecks": {"t72": {"look": 3, "footprint": [3.4, 1.8, 1.1], "paints": null}},
	"standIn": 4,
}

func run() -> bool:
	var car := _box("parked_car", [2.2, 0.9, 0.72], 7)
	var fitted := PropLayer.fit(table, car)
	check(fitted.size() == 1 and fitted[0][0] == 0, "a car takes the candidate its box fits best")
	# `paintOf` for authored prop 7 picks paint 0 of 12, 12345 paint 11
	# (the browser's Mulberry32, computed from `math/random`).
	check(is_equal_approx(fitted[0][2].r, 0.0), "authored prop 7 wears paint 0: %s" % fitted[0][2])
	check(is_equal_approx(PropLayer.fit(table, _box("parked_car", [2.2, 0.9, 0.72], 12345))[0][2].r, 0.11), "authored prop 12345 wears paint 11")
	# With no authored prop, the paint follows where it stands:
	# round(123.4 · 10) · 73856093 + round(567.8 · 10) picks paint 7.
	var dynamic := _box("parked_car", [2.2, 0.9, 0.72], null)
	dynamic.center = [123.4, 567.8]
	check(is_equal_approx(PropLayer.fit(table, dynamic)[0][2].r, 0.07), "a body with no authored prop wears the paint its place picks")
	var van := PropLayer.fit(table, _box("parked_car", [4.2, 1.2, 1.4], null))
	check(van[0][0] == 1 and is_equal_approx(van[0][1].basis.get_scale().x, 4.2 / 4.0), "a longer box takes the longer candidate, stretched to it")

	var wreck := _box("heavy_wreck", [3.5, 1.8, 1.2], null)
	wreck.wreckOf = "t72"
	check(PropLayer.fit(table, wreck)[0][0] == 3, "a wreck is its own unit's wreck")
	wreck.wreckOf = "unknown"
	check(PropLayer.fit(table, wreck).is_empty(), "a wreck of a unit with no wreck art draws nothing")

	# A 16 m wall of 4 m modules (footprints are half extents): four, each
	# centred on its quarter of the length.
	var wall := _box("wall", [8.0, 0.3, 0.8], null)
	var modules := PropLayer.fit(table, wall)
	check(modules.size() == 4, "a modular wall repeats its module along its length: %d" % modules.size())
	check(modules.map(func(m): return snappedf(m[1].origin.x, 0.01)) == [-6.0, -2.0, 2.0, 6.0], "evenly: %s" % [modules.map(func(m): return m[1].origin.x)])

	var kiosk := PropLayer.fit(table, _box("kiosk", [1.0, 1.5, 1.2], null))
	check(kiosk.size() == 1 and kiosk[0][0] == 4 and kiosk[0][2] == Color(0.5, 0.25, 0.0), "a kind with no art stands in, tinted")
	check(is_equal_approx(kiosk[0][1].basis.get_scale().y, 2.4), "the stand-in box is the prop's own box (2.4 m tall)")
	check(PropLayer.fit(table, _box("trunk", [0.3, 0.3, 5.0], null)).is_empty(), "a kind drawn by something else draws nothing here")
	check(PropLayer.fit(table, _box("unbound", [1, 1, 1], null)).is_empty(), "nor does a kind with no binding")
	return true

static func _box(kind: String, half: Array, authored) -> Dictionary:
	return {"kind": kind, "center": [0.0, 0.0], "yaw": 0.0, "half": half, "baseZ": 0.0, "authoredProp": authored, "wreckOf": null, "destroyed": false}
