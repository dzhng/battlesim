extends "res://tests/check.gd"

## The building layer's contract (`world/building_layer.gd`, the port of
## `buildingPlacements.ts`): a chunk in view draws its coarse rows, or its rows
## at the tier its nearest point calls for while the pool holds them; a
## building the side knows destroyed draws its ruin's or its gutted shell's
## rows instead, by what its parts became.

const BuildingLayer = preload("res://world/building_layer.gd")
const DetailView = preload("res://world/detail_view.gd")
const Records = preload("res://tests/records.gd")

const VIEWPORT := Vector2(1280, 720)
## A metre at 85 m covers 10 px (851 px per metre a metre away): tier 0 nearer.
const TIERS := {"lodPxPerM": [10.0, 4.0, 1.2], "chunkM": 64.0, "poolRecords": 1024, "expandRows": 40000, "casterCoarser": 1}

func run() -> bool:
	var root := Node3D.new()
	get_root().add_child(root)
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.fov = rad_to_deg(0.8)
	# At the origin, 2 m up, looking north (sim +y, Godot -z).
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3(0, 2, -10))

	# Building 0 stands 40 m ahead (chunk 0), building 1 1.5 km ahead (chunk 1).
	# Each has a coarse row (module 0, tier 3) and a fine row (module 1, tier 0),
	# a ruin row (module 2, every tier) and a gutted row (module 3, every tier).
	var layer := _layer([[0.0, 40.0], [0.0, 1500.0]], TIERS)
	root.add_child(layer)
	layer.set_parts([[100, 101], [102]], ["gutted_shell"])
	var view = DetailView.of(camera, VIEWPORT, Vector3(0, 1, 0), 2600.0)
	layer.prepare(view, preload("res://report/render_report.gd").new())
	check(_drawn(layer) == {"1/0": 1, "0/3": 1}, "the near chunk draws its tier-0 row, the far one its coarse row: %s" % _drawn(layer))

	# The side sees building 0's parts replaced by gutted shells: it draws its
	# gutted rows at the chunk's tier, and building 1 is untouched.
	var report := preload("res://report/render_report.gd").new()
	layer.set_known([_known(100, "gutted_shell"), _known(101, "gutted_shell")], report)
	layer.prepare(view, report)
	check(_drawn(layer) == {"3/0": 1, "0/3": 1}, "a building known gutted draws its gutted rows instead: %s" % _drawn(layer))
	# One part rubble: the building is a ruin.
	layer.set_known([_known(100, "gutted_shell"), _known(101, "rubble")], report)
	layer.prepare(view, report)
	check(_drawn(layer) == {"2/0": 1, "0/3": 1}, "a part come down makes it a ruin: %s" % _drawn(layer))
	# A part the side never saw fall leaves it standing; a prop of no building changes nothing.
	layer.set_known([_known(999, "rubble")], report)
	layer.prepare(view, report)
	check(_drawn(layer) == {"1/0": 1, "0/3": 1}, "an unseen fall leaves the building intact: %s" % _drawn(layer))
	check(report.ok(), "no blocker")

	# Turned away: neither chunk is in view, nothing draws.
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3(0, 2, 10))
	layer.prepare(DetailView.of(camera, VIEWPORT, Vector3(0, 1, 0), 2600.0), report)
	check(_drawn(layer).is_empty(), "chunks out of view draw nothing: %s" % _drawn(layer))

	# A pool with room for no fine row: the near chunk is refused, and draws coarse.
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3(0, 2, -10))
	var full := _layer([[0.0, 40.0], [0.0, 60.0]], TIERS.merged({"poolRecords": 1}, true))
	root.add_child(full)
	full.set_parts([[100], [101]], [])
	full.prepare(DetailView.of(camera, VIEWPORT, Vector3(0, 1, 0), 2600.0), preload("res://report/render_report.gd").new())
	check(_drawn(full) == {"1/0": 1, "0/3": 1}, "the nearest chunk takes the pool's room; the next draws coarse: %s" % _drawn(full))
	root.free()
	return true

## A layer of buildings at sim `(x, y)` each, in a chunk of their own, under `tiers`.
static func _layer(places: Array, tiers: Dictionary) -> BuildingLayer:
	var buildings := []
	var intact := []
	var ruin := []
	var gutted := []
	for b in places.size():
		var at: Array = places[b]
		buildings.append({"frame": [at[0], at[1], 0.0, 0.0], "min": [at[0] - 5, at[1] - 5, 0.0], "max": [at[0] + 5, at[1] + 5, 10.0], "owner": b, "chunk": b, "rows": [2, 1, 1]})
		for row in [[0, 8], [1, 1]]:
			intact.append(_row(at, row[0], row[1], b))
		ruin.append(_row(at, 2, 15, b))
		gutted.append(_row(at, 3, 15, b))
	var tables := {
		"buildings": Records.table(BUILDING_FIELDS, buildings),
		"intact": Records.table(MODULE_FIELDS, intact),
		"ruin": Records.table(MODULE_FIELDS, ruin),
		"gutted": Records.table(MODULE_FIELDS, gutted),
	}
	var modules := range(4).map(func(m): return {"kit": "kit", "module": "m%d" % m})
	return BuildingLayer.new(tables, {"tiers": tiers, "modules": modules}, Vector3(0, 1, 0), 2600.0)

static func _row(at: Array, module: int, tiers: int, building: int) -> Dictionary:
	return {"position": [at[0], at[1], 0.0], "yaw": 0.0, "scale": [1.0, 1.0, 1.0], "kitModule": module, "tint": [1.0, 1.0, 1.0], "tiers": tiers, "building": building, "chunk": building}

static func _known(prop: int, kind: String) -> Dictionary:
	return {"authoredProp": prop, "kind": kind, "destroyed": false}

## The layer's last counts as `{"module/tier": count}`, zeros left out.
static func _drawn(layer: BuildingLayer) -> Dictionary:
	var out := {}
	for k in layer.counts.size():
		if layer.counts[k] > 0:
			out["%d/%d" % [k / 4, k % 4]] = layer.counts[k]
	return out

const BUILDING_FIELDS := [["frame", "f32", 4], ["min", "f32", 3], ["owner", "u32", 1], ["max", "f32", 3], ["chunk", "i32", 1], ["rows", "i32", 3]]
const MODULE_FIELDS := [["position", "f32", 3], ["yaw", "f32", 1], ["scale", "f32", 3], ["kitModule", "u32", 1], ["tint", "f32", 3], ["tiers", "u32", 1], ["building", "u32", 1], ["chunk", "i32", 1]]
