extends "res://tests/check.gd"

## The building layer's contract (`world/building_layer.gd`) on paris-corner's
## own buildings, resolved in Rust from the repository's files: the chunk
## nearest the camera draws its rows at tier 0; a building the side knows
## destroyed draws its gutted shell's or its ruin's rows, by what its parts
## became, and a state its template has no rows for is refused by name. The
## walk's tiers and pool are the extension's own tests (`static_layers`).

const BuildingLayer = preload("res://world/building_layer.gd")
const DetailView = preload("res://world/detail_view.gd")
const RenderReport = preload("res://report/render_report.gd")
const Staging = preload("res://assets/staging.gd")

const VIEWPORT := Vector2(1280, 720)
## Building 0 stands at sim (262.5, 179), its template with gutted rows and
## no ruin; building 3's has a ruin and no gutted shell.
const GUTTED := 0
const RUINED := 3

func run() -> bool:
	var root := Node3D.new()
	get_root().add_child(root)
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.fov = rad_to_deg(0.8)
	# 40 m south of building 0, 2 m up, looking north (sim +y, Godot -z).
	camera.look_at_from_position(Vector3(262.5, 2, -139), Vector3(262.5, 2, -179))

	var report := RenderReport.new()
	var layer := BuildingLayer.new("paris-corner", Vector3(0, 1, 0), 2600.0, report)
	check(report.ok(), "paris-corner's buildings resolve: %s" % JSON.stringify(report.to_dictionary().blockers))
	root.add_child(layer)
	var map: Dictionary = read_json(Staging.repo_path("fixtures/maps/paris-corner/map.json"))
	layer.set_parts(map.buildings.map(func(b): return b.parts.map(func(p): return p.prop)), ["gutted_shell"])
	var view = DetailView.of(camera, VIEWPORT, Vector3(0, 1, 0), 2600.0)
	layer.prepare(view, report)
	var intact := _drawn(layer)
	check(intact.keys().any(func(key): return key.ends_with("/0")), "the near chunk draws at tier 0: %s" % intact)

	# The side sees building 0's body replaced by a gutted shell.
	layer.set_known([_known(GUTTED, "gutted_shell")], report)
	layer.prepare(view, report)
	check(report.ok() and _drawn(layer) != intact, "a building known gutted draws its gutted rows instead")
	# Building 3's body came down: a ruin, which its template has.
	layer.set_known([_known(RUINED, "rubble")], report)
	layer.prepare(view, report)
	check(report.ok(), "a building known ruined draws its ruin's rows")
	# A part the side never saw fall leaves every building standing.
	layer.set_known([_known(999, "rubble")], report)
	layer.prepare(view, report)
	check(_drawn(layer) == intact, "an unseen fall leaves the buildings intact: %s" % _drawn(layer))
	check(report.ok(), "no blocker")

	# Building 0's body came down, and its template has no ruin.
	var refused := RenderReport.new()
	layer.set_known([_known(GUTTED, "rubble")], refused)
	var blockers: Array = refused.to_dictionary().blockers
	check(blockers.any(func(row): return row.name == "buildings" and row.detail.contains("no ruin rows")), "a state the template lacks is refused by name: %s" % JSON.stringify(blockers))
	root.free()
	return true

static func _known(prop: int, kind: String) -> Dictionary:
	return {"authoredProp": prop, "kind": kind, "destroyed": false}

## The layer's last counts as `{"module/tier": count}`, zeros left out.
static func _drawn(layer: BuildingLayer) -> Dictionary:
	var out := {}
	for k in layer.counts.size():
		if layer.counts[k] > 0:
			out["%d/%d" % [k / 4, k % 4]] = layer.counts[k]
	return out
