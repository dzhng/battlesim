extends Node3D

## The scenery as the browser's scenery layer draws it (`frame/sceneryLayer.ts`),
## from the instances it exported (`manifest.scenery`, `sceneryTables.ts`):
## native places nothing, and ports only the per-frame rules.
##
## - **Trees and shrubs** (the forest, its understorey, and the backdrop past
##   the map) are bucketed in square chunks (`scenery/lod.ts`). A chunk in view
##   whose tallest instance takes the coarsest tier from its nearest point draws
##   whole at it; a nearer chunk's instances each take the tier their own
##   projected height calls for. The forest and understorey cast, a tier
##   coarser; the backdrop casts nothing.
## - **Dressing** (`scenery/dressing.ts`) is laid a cell at a time: a cell in
##   view, nearer than where its tallest piece shrinks away, draws whole at its
##   tier, nearest first while the browser's slots last. A piece on ground the
##   side has seen cleared is laid with no size and not drawn.
## - **State** is the side's (`scenery/felled.ts`): a forest tree it saw fall
##   leaves the forest, falls on the clock from `(tick − 1) / tickHz` and lies
##   beside its stump; a shrub on ground it saw cleared is gone.
##
## The browser lays a cut's dressing over a few frames; native, stepping
## records far apart, lays every wanted cell at once. Not ported: the leaf
## clumps' shading and the dressing's shrink as it nears a few pixels.

const ModelBatches = preload("res://world/model_batches.gd")
const DetailView = preload("res://world/detail_view.gd")
const WordTables = preload("res://core/word_tables.gd")
const Coords = preload("res://core/coords.gd")
const Felled = preload("res://world/felled.gd")

const OWNER := "native/scenery_layer"
const TIERS := 4
const COARSEST := TIERS - 1
const POPULATIONS := ["forest", "understorey", "backdrop"]
const FLOATS := ModelBatches.INSTANCE_FLOATS

var _batches: ModelBatches
var _kinds: Array
var _appearances := PackedInt32Array()
var _lod_px: Array
var _dressing_rules: Dictionary
## Per population: its records (`WordTables.records`) and its id in the walk
## (`StaticSelection`, `native/godot-binding`), which chunks, culls and tiers
## the trees and shrubs.
var _records := {}
var _selection
var _pops := {}
## Each forest tree's record, by its trunk's body id.
var _forest_at := {}
## The dressing: its records, and per cell its box, first piece and count.
var _dressing: Dictionary
var _cells := {}
var _cell_list: Array[Dictionary] = []
## The felled trees and their stumps (`felled.gd`).
var _felled: Felled
## The trunk ids of the trees the forest stands without, and the learned
## ground's `cleared_version` the shrubs and dressing stand by.
var _fallen_ids := {}
var _learned
var _cleared_version := -1
## The dressing the last frame drew (`_plan_dressing`), and the keys
## (`kind · 4 + tier`) it drew into.
var _last_dressing: Array = []
var _dressing_keys := {}
## Population name -> per kind and tier (`kind · 4 + tier`), what the last
## frame drew; and the stumps per kind.
var counts := {}

## `tables` are the manifest's scenery tables read, `scenery` its section,
## `sun` and `max_far_m` the light's, `tick_hz` the capture's.
func _init(tables: Dictionary, scenery: Dictionary, sun: Vector3, max_far_m: float, tick_hz: float) -> void:
	name = "Scenery"
	_batches = ModelBatches.new("SceneryBatches", sun, max_far_m)
	add_child(_batches)
	_kinds = scenery.kinds
	var tiers: Dictionary = scenery.tiers
	_lod_px = tiers.lodPx
	_dressing_rules = tiers.dressing
	_selection = ClassDB.instantiate("StaticSelection")
	_selection.set_kinds(_kinds.size())
	for population in POPULATIONS:
		var table: Dictionary = tables[population]
		_records[population] = WordTables.records(table)
		_pops[population] = _selection.add_scenery(table.words, int(table.layout.stride), _records[population].at, float(tiers.chunkM), PackedFloat64Array(_lod_px), population != "backdrop")
	var forest: Dictionary = _records.forest
	for r in forest.count:
		_forest_at[forest.i[r * forest.stride + forest.at.id] & 0xffffffff] = r
	_dressing = WordTables.records(tables.dressing)
	var cells := WordTables.records(tables.dressingCells)
	var m := float(_dressing_rules.cellM)
	var past := float(_dressing_rules.reachM)
	var ground: Array = _dressing_rules.ground
	for c in cells.count:
		var o: int = c * cells.stride
		var i: int = cells.i[o + cells.at.cell]
		var j: int = cells.i[o + cells.at.cell + 1]
		_cells[Vector2i(i, j)] = _cell_list.size()
		_cell_list.append({
			"box": DetailView.sim_box(Vector3(i * m - past, j * m - past, float(ground[0])), Vector3((i + 1) * m + past, (j + 1) * m + past, float(ground[1]) + float(_dressing_rules.topM))),
			"first": cells.i[o + cells.at.first],
			"count": cells.i[o + cells.at.count],
			"cleared": PackedByteArray(),
			"pack": {},
			# The cleared version it was last laid by, and a count of its layings.
			"laid": -1,
			"version": 0,
		})
	_felled = Felled.new(scenery.felled, tick_hz)
	for key in POPULATIONS + ["dressing", "felled"]:
		counts[key] = PackedInt32Array()
		counts[key].resize(_kinds.size() * TIERS)
	counts.stumps = PackedInt32Array()
	counts.stumps.resize(_kinds.size())

## The sources the appearances are drawn from (`catalog`'s summer state).
func required_assets(catalog) -> Array:
	return _kinds.map(func(kind): return catalog.source(kind, "summer"))

## Registers every appearance from `catalog`; one it cannot draw is a
## blocker in `report`.
func build(catalog, report) -> void:
	for kind in _kinds:
		var node: Node = catalog.node(kind, "summer")
		if node == null:
			report.add_blocker("scenery", "appearance %s has no staged summer state" % kind)
			_appearances.append(-1)
			continue
		_appearances.append(_batches.add_appearance(node, true))
	var forest_kinds := {}
	var forest: Dictionary = _records.forest
	for r in forest.count:
		forest_kinds[forest.i[r * forest.stride + forest.at.kind]] = true
	var barks := _kinds.map(func(kind): return Felled.bark_of(catalog.node(kind, "summer")) if catalog.node(kind, "summer") != null else null)
	_felled.build(_batches, _appearances, self, forest_kinds, barks)
	for population in POPULATIONS:
		report.add_budget(OWNER, "the browser's %s instances" % population, "scenery." + population, int(_records[population].count), int(_records[population].count), 0)
	report.add_budget(OWNER, "approximation: dressing drawn full size where the browser shrinks a piece away below %s px" % _dressing_rules.fadePx, "scenery.dressing", int(_dressing.count), int(_dressing.count), 0)
	report.add_budget(OWNER, "approximation: crowns without the browser's leaf clumps", "scenery.leaves", 1, 1, 0)

## Follows what the side knows: the forest trees it saw fall
## (`fallenBodies`), and the ground it saw cleared (`learned`,
## `learned_ground.gd`), under which shrubs and dressing are gone.
func set_known(fallen_bodies: Array, learned) -> void:
	var felled := {}
	for body in fallen_bodies:
		felled[int(body.prop)] = true
	if felled != _fallen_ids:
		_fallen_ids = felled
		_selection.hide_keys(_pops.forest, PackedInt64Array(felled.keys()))
	var version: int = learned.cleared_version if learned != null else 0
	if learned != _learned or version != _cleared_version:
		_learned = learned
		_cleared_version = version
		var shrubs: Dictionary = _records.understorey
		var hidden := PackedByteArray()
		hidden.resize(shrubs.count)
		for r in shrubs.count:
			var o: int = r * shrubs.stride + shrubs.at.pose
			hidden[r] = 1 if learned != null and learned.cleared_at(shrubs.f[o], shrubs.f[o + 1]) else 0
		_selection.set_hidden(_pops.understorey, hidden)
	_felled.show(fallen_bodies, _tree)

## Forest tree `id` as the felled trees read it, or empty.
func _tree(id: int) -> Dictionary:
	var r = _forest_at.get(id)
	if r == null:
		return {}
	var forest: Dictionary = _records.forest
	var pack := PackedFloat32Array()
	pack_instance(pack, forest, r)
	var o: int = r * forest.stride
	return {"kind": forest.i[o + forest.at.kind], "height": forest.f[o + forest.at.height], "reach": forest.f[o + forest.at.reach], "pack": pack}

## Chooses what `view` (`detail_view.gd`) draws and casts at presentation
## second `clock`, counts it, and writes what changed.
func prepare(view, clock: float) -> void:
	var changed := {}
	for key in _selection.select(view.eye, view.sides_packed(), view.pixels_per_metre, view.reach, view.fall):
		changed[key] = true
	for population in POPULATIONS:
		counts[population] = _selection.counts(_pops[population])
	var dressing := _plan_dressing(view, counts.dressing)
	var additions := {}
	for entry in dressing:
		var pack: Dictionary = _cell_list[entry[0]].pack
		for kind in pack:
			var key: int = kind * TIERS + entry[1]
			if not additions.has(key):
				additions[key] = PackedFloat32Array()
			additions[key].append_array(pack[kind])
	if dressing != _last_dressing:
		_last_dressing = dressing
		changed.merge(_dressing_keys)
		changed.merge(additions)
		_dressing_keys = additions
	for key in changed:
		var kind: int = key / TIERS
		if kind < _appearances.size() and _appearances[kind] >= 0:
			var drawn: PackedFloat32Array = _selection.view(key)
			if additions.has(key):
				drawn.append_array(additions[key])
			_batches.choose(_appearances[kind], key % TIERS, drawn, _selection.cast(key))
	_felled.prepare(view, clock, _lod_px, counts.felled, counts.stumps)
	_batches.write()

## The tier an instance `height` metres tall draws at from `distance` (`tierFor`).
static func tier_for(height: float, distance: float, pixels_per_metre: float, lod_px: Array) -> int:
	var px := height * pixels_per_metre / maxf(distance, 1e-3)
	return 0 if px > float(lod_px[0]) else 1 if px > float(lod_px[1]) else 2 if px > float(lod_px[2]) else COARSEST

## The dressing cells `view` draws, nearest first while the browser's slots
## last, counted into `drawn`: `[cell, tier, the cell's pack version]` each.
func _plan_dressing(view, drawn: PackedInt32Array) -> Array:
	drawn.fill(0)
	if _cell_list.is_empty():
		return []
	var ppm: float = view.pixels_per_metre
	var top := float(_dressing_rules.topM)
	var reach := top * ppm / (float(_dressing_rules.fadePx) * float(_dressing_rules.gone))
	var m := float(_dressing_rules.cellM)
	var eye := Coords.godot_to_sim(view.eye)
	var wanted := []
	for j in range(floori((eye.y - reach) / m), floori((eye.y + reach) / m) + 1):
		for i in range(floori((eye.x - reach) / m), floori((eye.x + reach) / m) + 1):
			var at = _cells.get(Vector2i(i, j))
			if at == null or _cell_list[at].count == 0:
				continue
			var cell: Dictionary = _cell_list[at]
			if not view.box_meets(cell.box):
				continue
			var distance: float = view.distance_to(cell.box)
			if distance < reach:
				wanted.append([distance, at])
	wanted.sort_custom(func(a, b): return a[0] < b[0])
	var plan := []
	for entry in wanted.slice(0, int(_dressing_rules.slots)):
		var tier := tier_for(top, entry[0], ppm, _dressing_rules.lodPx)
		var cell: Dictionary = _cell_list[entry[1]]
		var pack := _dressing_pack(cell)
		for kind in pack:
			drawn[kind * TIERS + tier] += pack[kind].size() / FLOATS
		plan.append([entry[1], tier, cell.version])
	return plan

## A cell's pieces standing on ground the side has not seen cleared, by
## kind; laid again when what is cleared under it changes.
func _dressing_pack(cell: Dictionary) -> Dictionary:
	if cell.laid == _cleared_version and not cell.pack.is_empty():
		return cell.pack
	cell.laid = _cleared_version
	var cleared := PackedByteArray()
	cleared.resize(cell.count)
	var learned = _learned
	var records := _dressing
	for p in cell.count:
		var o: int = (cell.first + p) * records.stride + records.at.pose
		cleared[p] = 1 if learned != null and learned.cleared_at(records.f[o], records.f[o + 1]) else 0
	if cleared != cell.cleared or cell.pack.is_empty():
		cell.cleared = cleared
		cell.version += 1
		var pack := {}
		for p in cell.count:
			if cleared[p]:
				continue
			var r: int = cell.first + p
			var kind: int = records.i[r * records.stride + records.at.kind]
			if not pack.has(kind):
				pack[kind] = PackedFloat32Array()
			pack_instance(pack[kind], records, r)
		cell.pack = pack
	return cell.pack

## Appends record `r` of `records` (a `SceneryInstance` table) as an instance.
static func pack_instance(out: PackedFloat32Array, records: Dictionary, r: int) -> void:
	var o: int = r * records.stride
	var at: Dictionary = records.at
	var f: PackedFloat32Array = records.f
	var basis := Coords.yaw_basis(f[o + at.pose + 3]).scaled_local(Vector3(f[o + at.shape], f[o + at.shape + 2], f[o + at.shape + 1]))
	var foot := Coords.sim_to_godot(Vector3(f[o + at.pose], f[o + at.pose + 1], f[o + at.pose + 2]))
	ModelBatches.pack(out, Transform3D(basis, foot), Color(f[o + at.tint], f[o + at.tint + 1], f[o + at.tint + 2]))
