extends Node3D

## The map's buildings as the browser's building layer draws them
## (`models/buildingPlacements.ts`), from the module instances it exported
## (`manifest.buildings`, `buildingTables.ts`): native places nothing, and
## ports only the per-frame rules.
##
## - **Coarse.** Every chunk in view draws its intact buildings' coarsest rows
##   (the rows whose tier mask has tier 3), and casts them within the shadows'
##   reach; a chunk out of view whose shadow lands in it casts them.
## - **Resident.** A chunk in view whose nearest point calls for a finer tier
##   draws its rows at that tier instead, nearest first while the pool has
##   room, and casts them a tier coarser. A chunk the pool has no room for
##   draws coarse.
## - **Fallen.** A building the side knows destroyed leaves both and draws its
##   template's rows for the state it is known in (a ruin or a gutted shell),
##   chunked by where it was placed, at every tier its rows have.
##
## The browser expands residents over a few frames and packs its pool by
## kind; native, stepping records far apart, settles each frame at once and
## counts the pool by records. A frame whose residents would pass the share of
## the pool packing always leaves free is a blocker, not a guess.

const ModelBatches = preload("res://world/model_batches.gd")
const DetailView = preload("res://world/detail_view.gd")
const WordTables = preload("res://core/word_tables.gd")

const OWNER := "native/building_layer"
const TIERS := 4
const STATES := ["intact", "ruin", "gutted"]
## The browser's pool refuses a record once packing cannot give every kind a
## quarter to spare (`placementPool.ts`): residents past this share of its
## records are never all drawn there.
const POOL_SHARE := 0.75

var _batches: ModelBatches
## The tier rule (`manifest.buildings.tiers`).
var _lod_px_per_m: Array
var _chunk_m: float
var _pool_records: int
## Per exported module: its appearance in `_batches`.
var _modules: Array = []
var _module_names: Array = []
## The manifest's tables: `buildings` and each state's rows.
var _tables: Dictionary
## Per building: its frame (x, y, z, yaw), its bounds in Godot space and its
## height, and per state its first row and row count (-1: its template has
## none).
var _frames := PackedFloat32Array()
var _boxes: Array[AABB] = []
var _heights := PackedFloat32Array()
var _first: Array[PackedInt32Array] = []
var _count: Array[PackedInt32Array] = []
## The walk (`StaticSelection`, `native/godot-binding`): the intact rows by
## coarse chunk, and the fallen population (-1 until any fall).
var _selection
var _coarse := -1
var _ruins := -1
## Building -> the state the side knows it fell in.
var _fallen := {}
## Each building part's prop id -> its building, and the prop kinds a gutted
## building's parts become (`manifest.props`); empty until known.
var _part_building := {}
var _shells := {}
## Per module and tier (`module · 4 + tier`): what the last frame drew.
var counts := PackedInt32Array()
## The most pool records a frame held.
var most_resident := 0

## `tables` are the manifest's building tables read (`{name: {words,
## layout}}`), `buildings` its section; `sun` and `max_far_m` the light's.
func _init(tables: Dictionary, buildings: Dictionary, sun: Vector3, max_far_m: float) -> void:
	name = "Buildings"
	_batches = ModelBatches.new("BuildingBatches", sun, max_far_m)
	add_child(_batches)
	_tables = tables
	var tiers: Dictionary = buildings.tiers
	_lod_px_per_m = tiers.lodPxPerM
	_chunk_m = float(tiers.chunkM)
	_pool_records = int(tiers.poolRecords)
	_module_names = buildings.modules
	counts.resize(_module_names.size() * TIERS)
	var placed := WordTables.records(tables.buildings)
	for state in STATES:
		_first.append(PackedInt32Array())
		_count.append(PackedInt32Array())
	var at: Dictionary = placed.at
	for b in placed.count:
		var o: int = b * placed.stride
		for k in 4:
			_frames.append(placed.f[o + at.frame + k])
		var lo := Vector3(placed.f[o + at.min], placed.f[o + at.min + 1], placed.f[o + at.min + 2])
		var hi := Vector3(placed.f[o + at.max], placed.f[o + at.max + 1], placed.f[o + at.max + 2])
		_boxes.append(DetailView.sim_box(lo, hi))
		_heights.append(hi.z - lo.z)
		for s in STATES.size():
			var rows: int = placed.i[o + at.rows + s]
			var before := _first[s][b - 1] + maxi(_count[s][b - 1], 0) if b > 0 else 0
			_first[s].append(before)
			_count[s].append(rows)
	_selection = ClassDB.instantiate("StaticSelection")
	_selection.set_kinds(_module_names.size())
	_coarse = _selection.add_buildings(tables.buildings.words, int(tables.buildings.layout.stride), _at(tables.buildings), tables.intact.words, int(tables.intact.layout.stride), _at(tables.intact), PackedFloat64Array(_lod_px_per_m), _pool_records)

## A table's fields' first words by name.
static func _at(table: Dictionary) -> Dictionary:
	var at := {}
	for field in table.layout.fields:
		at[String(field.name)] = int(field.word)
	return at

## The kit sources the modules are drawn from (`catalog`'s).
func required_assets(catalog) -> Array:
	var sources := {}
	for module in _module_names:
		sources[catalog.source(module.kit, module.module)] = true
	sources.erase("")
	return sources.keys()

## Registers every module's tiers from `catalog`; a module it cannot draw is
## a blocker in `report`.
func build(catalog, report) -> void:
	for module in _module_names:
		var node: Node = catalog.node(module.kit, module.module)
		if node == null:
			report.add_blocker("buildings", "kit %s has no module %s staged" % [module.kit, module.module])
			_modules.append(-1)
			continue
		_modules.append(_batches.add_appearance(node))
	var rows := 0
	for state in STATES:
		rows += int(_tables[state].layout.count)
	report.add_budget(OWNER, "the browser's module instances, every building in every state its template has", "buildings", _frames.size() / 4, _frames.size() / 4, 0)
	report.add_budget(OWNER, "module instances over every state", "buildings.rows", rows, rows, 0)

## What the building parts are and what a gutted part becomes
## (`manifest.props.buildings`): each building's part props, by building.
func set_parts(parts: Array, shells: Array) -> void:
	for b in parts.size():
		for prop in parts[b]:
			_part_building[int(prop)] = b
	for kind in shells:
		_shells[String(kind)] = true

## Follows what the side knows (`knownProps` of the frame's publication), as
## the browser's `fallenBuildings` does: a part's place taken by a shell's prop
## type stands its building `gutted`; taken by anything else, or by nothing,
## a `ruin`. A state the template has no rows for is a blocker in `report`.
func set_known(known: Array, report) -> void:
	var states := {}
	for prop in known:
		if prop.authoredProp == null or not _part_building.has(int(prop.authoredProp)):
			continue
		var b: int = _part_building[int(prop.authoredProp)]
		var stands: bool = not prop.destroyed and _shells.has(prop.kind)
		states[b] = "gutted" if stands and states.get(b) != "ruin" else "ruin"
	if states == _fallen:
		return
	for b in states:
		if _count[STATES.find(states[b])][b] < 0:
			report.add_blocker("buildings", "building %d is known %s, and its template has no %s rows" % [b, states[b], states[b]])
	_fallen = states
	_selection.hide_keys(_coarse, PackedInt64Array(states.keys()))
	# The fallen population, chunked by the cell each stands in: per cell its
	# buildings' rows in the state each is known in, its box and its tallest.
	var cells := {}
	var sorted := states.keys()
	sorted.sort()
	for b in sorted:
		var cell := Vector2i(floori(_frames[b * 4] / _chunk_m), floori(_frames[b * 4 + 1] / _chunk_m))
		if not cells.has(cell):
			cells[cell] = {"buildings": [], "box": _boxes[b], "height": 0.0}
		var ruin: Dictionary = cells[cell]
		ruin.buildings.append(b)
		ruin.box = ruin.box.merge(_boxes[b])
		ruin.height = maxf(ruin.height, _heights[b])
	var stride: int = int(_tables.ruin.layout.stride)
	var rows := PackedByteArray()
	var boxes := PackedFloat32Array()
	var heights := PackedFloat32Array()
	var firsts := PackedInt32Array()
	var sizes := PackedInt32Array()
	for ruin in cells.values():
		firsts.append(rows.size() / 4 / stride)
		for b in ruin.buildings:
			var s := STATES.find(states[b])
			var words: PackedByteArray = _tables[STATES[s]].words
			rows.append_array(words.slice(_first[s][b] * stride * 4, (_first[s][b] + maxi(_count[s][b], 0)) * stride * 4))
		sizes.append(rows.size() / 4 / stride - firsts[-1])
		boxes.append_array([ruin.box.position.x, ruin.box.position.y, ruin.box.position.z, ruin.box.end.x, ruin.box.end.y, ruin.box.end.z])
		heights.append(ruin.height)
	_ruins = _selection.set_ruins(_ruins, rows, stride, _at(_tables.ruin), boxes, heights, firsts, sizes, PackedFloat64Array(_lod_px_per_m))

## Chooses what `view` (`detail_view.gd`) draws and casts this frame, counts
## it, and writes what changed. `report` takes a blocker should the pool not
## hold the residents.
func prepare(view, report) -> void:
	for key in _selection.select(view.eye, view.sides_packed(), view.pixels_per_metre, view.reach, view.fall):
		var module: int = key / TIERS
		if module < _modules.size() and _modules[module] >= 0:
			_batches.choose(_modules[module], key % TIERS, _selection.view(key), _selection.cast(key))
	_batches.write()
	counts = _selection.counts(_coarse)
	if _ruins >= 0:
		var fallen: PackedInt32Array = _selection.counts(_ruins)
		for k in counts.size():
			counts[k] += fallen[k]
	var used: int = _selection.most_resident(_coarse)
	if used > most_resident and used > POOL_SHARE * _pool_records:
		report.add_blocker("buildings", "a view holds %d resident module instances; the browser's pool (%d records) refuses past about %d%% of them, which native does not port" % [used, _pool_records, roundi(POOL_SHARE * 100.0)])
	most_resident = used
