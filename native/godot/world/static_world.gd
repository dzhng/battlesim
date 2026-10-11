extends Node3D

## The map's buildings, scenery and props, drawn from the placement the
## browser exported (`manifest.buildings`, `.scenery`, `.props`) in the state
## the side knows from each frame's publication, and counted per plate against
## what the browser drew (`manifest.instanceCounts`): equal, or a blocker.
## Each listed layer is its own owner: `building_layer.gd` (`buildings`),
## `scenery_layer.gd` (`scenery`) and `prop_layer.gd` (`structures`, the
## browser's fitted props).

const Manifest = preload("res://capture/visual_manifest.gd")
const AppearanceCatalog = preload("res://world/appearance_catalog.gd")
const BuildingLayer = preload("res://world/building_layer.gd")
const SceneryLayer = preload("res://world/scenery_layer.gd")
const PropLayer = preload("res://world/prop_layer.gd")
const DetailView = preload("res://world/detail_view.gd")
const Coords = preload("res://core/coords.gd")

var buildings: BuildingLayer
var scenery: SceneryLayer
var props: PropLayer
var _sun: Vector3
var _max_far_m: float

## The layers `manifest` lists, from its tables beside it in `scene_dir`, with
## the capture's `tick_hz`. Blockers go to `report`; call `build` next.
func _init(scene_dir: String, manifest: Dictionary, tick_hz: float, report) -> void:
	name = "StaticWorld"
	_sun = Coords.sim_array_to_godot(manifest.light.sunDirection)
	_max_far_m = float(manifest.light.cascades.max_far_m)
	var layers: Array = manifest.layers
	if layers.has("buildings"):
		var read := Manifest.read_tables(scene_dir, manifest.buildings.tables, "buildings")
		if not read.ok:
			report.add_blocker(read.blocker.name, read.blocker.detail)
			return
		buildings = BuildingLayer.new(read.tables, manifest.buildings, _sun, _max_far_m)
		buildings.set_parts(manifest.props.buildings.parts, manifest.props.buildings.shells)
		add_child(buildings)
	if layers.has("scenery"):
		var read := Manifest.read_tables(scene_dir, manifest.scenery.tables, "scenery")
		if not read.ok:
			report.add_blocker(read.blocker.name, read.blocker.detail)
			return
		scenery = SceneryLayer.new(read.tables, manifest.scenery, _sun, _max_far_m, tick_hz)
		add_child(scenery)
	if layers.has("structures"):
		var read := Manifest.read_tables(scene_dir, [manifest.props.map], "props")
		if not read.ok:
			report.add_blocker(read.blocker.name, read.blocker.detail)
			return
		props = PropLayer.new(manifest.props, read.tables.map, _sun, _max_far_m)
		add_child(props)

## The layers there are.
func _layers() -> Array:
	return [buildings, scenery, props].filter(func(layer): return layer != null)

## Whether the manifest listed any of them: only then is a plate counted.
func listed() -> bool:
	return not _layers().is_empty()

## Stages every asset the layers draw and registers their appearances.
func build(report) -> void:
	var catalog := AppearanceCatalog.new()
	var sources := {}
	for layer in _layers():
		for source in layer.required_assets(catalog):
			sources[source] = true
	catalog.stage(sources.keys(), report)
	if not report.ok():
		return
	for layer in _layers():
		layer.build(catalog, report)
	catalog.free_scenes()

## Draws what `sample` (the frame's publication) says the side knows, with
## the side's learned ground replayed to its tick (`learned`), at presentation
## second `clock`, as `camera` sees it into a viewport `size` pixels big.
func present(sample: Dictionary, learned, clock: float, camera: Camera3D, size: Vector2, report) -> void:
	var view = DetailView.of(camera, size, _sun, _max_far_m)
	if buildings != null:
		buildings.set_known(sample.knownProps, report)
		buildings.prepare(view, report)
	if scenery != null:
		scenery.set_known(sample.fallenBodies, learned)
		scenery.prepare(view, clock)
	if props != null:
		props.set_known(sample.knownProps)
		props.prepare(view)
	for layer in _layers():
		layer.prepare_batches(camera, size)

## What the last frame drew, in the browser's `instanceCounts` shape (less
## the plate's name).
func counts() -> Dictionary:
	var scenery_counts := {}
	if scenery != null:
		for key in scenery.counts:
			scenery_counts[key] = Array(scenery.counts[key])
	return {
		"buildings": Array(buildings.counts) if buildings != null else [],
		"scenery": scenery_counts,
		"knownProps": props.drawn() if props != null else [],
	}

## How `native` (`counts`) differs from the browser's row for the same plate:
## one line per kit module, scenery population and appearance, or known prop
## appearance whose counts differ, naming both. Empty when equal.
static func differences(native: Dictionary, browser: Dictionary, modules: Array, kinds: Array) -> Array[String]:
	var out: Array[String] = []
	if not native.buildings.is_empty():
		for m in modules.size():
			var ours: Array = native.buildings.slice(m * 4, m * 4 + 4)
			var theirs: Array = browser.buildings.slice(m * 4, m * 4 + 4).map(func(n): return int(n))
			if ours != theirs:
				out.append("module %s/%s per tier: native %s, browser %s" % [modules[m].kit, modules[m].module, ours, theirs])
	for population in native.scenery:
		var width := 1 if population == "stumps" else 4
		for k in kinds.size():
			var ours: Array = native.scenery[population].slice(k * width, k * width + width)
			var theirs: Array = browser.scenery[population].slice(k * width, k * width + width).map(func(n): return int(n))
			if ours != theirs:
				out.append("%s %s per tier: native %s, browser %s" % [population, kinds[k], ours, theirs])
	var known := {}
	for row in browser.knownProps:
		known[row.appearance] = row.levels.map(func(n): return int(n))
	for row in native.knownProps:
		if known.get(row.appearance) != row.levels:
			out.append("known %s per tier and card: native %s, browser %s" % [row.appearance, row.levels, known.get(row.appearance)])
		known.erase(row.appearance)
	for appearance in known:
		out.append("known %s per tier and card: native none, browser %s" % [appearance, known[appearance]])
	return out
