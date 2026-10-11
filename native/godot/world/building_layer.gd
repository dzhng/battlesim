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
const Coords = preload("res://core/coords.gd")

const OWNER := "native/building_layer"
const TIERS := 4
const COARSEST := TIERS - 1
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
var _caster_coarser: int
## Per exported module: its appearance in `_batches`.
var _modules: Array = []
var _module_names: Array = []
## Per building: its frame (x, y, z, yaw), its bounds in Godot space and its
## height, its coarse chunk (-1 for none), and per state its first row and
## row count (-1: its template has none).
var _frames := PackedFloat32Array()
var _boxes: Array[AABB] = []
var _heights := PackedFloat32Array()
var _chunk_of := PackedInt32Array()
var _first: Array[PackedInt32Array] = []
var _count: Array[PackedInt32Array] = []
## Per state: the module instances (`WordTables.records`).
var _rows: Array[Dictionary] = []
## Per coarse chunk: its buildings, its box (the union of theirs) and its
## tallest building.
var _chunks: Array[PackedInt32Array] = []
var _chunk_boxes: Array[AABB] = []
var _chunk_heights := PackedFloat32Array()
## Chunk and tier -> module -> instances packed for the batches; dropped when
## a building of the chunk falls or rises.
var _packs := {}
## Building -> the state the side knows it fell in.
var _fallen := {}
## Each building part's prop id -> its building, and the prop kinds a gutted
## building's parts become (`manifest.props`); empty until known.
var _part_building := {}
var _shells := {}
## The fallen population: per cell, its buildings, box and tallest building.
var _ruins: Array[Dictionary] = []
## What the last frame chose, so an unchanged view writes nothing.
var _last_choice: Array = []
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
	var tiers: Dictionary = buildings.tiers
	_lod_px_per_m = tiers.lodPxPerM
	_chunk_m = float(tiers.chunkM)
	_pool_records = int(tiers.poolRecords)
	_caster_coarser = int(tiers.casterCoarser)
	_module_names = buildings.modules
	counts.resize(_module_names.size() * TIERS)
	var placed := WordTables.records(tables.buildings)
	for state in STATES:
		_rows.append(WordTables.records(tables[state]))
		_first.append(PackedInt32Array())
		_count.append(PackedInt32Array())
	var at: Dictionary = placed.at
	var chunk_count := 0
	for b in placed.count:
		var o: int = b * placed.stride
		for k in 4:
			_frames.append(placed.f[o + at.frame + k])
		var lo := Vector3(placed.f[o + at.min], placed.f[o + at.min + 1], placed.f[o + at.min + 2])
		var hi := Vector3(placed.f[o + at.max], placed.f[o + at.max + 1], placed.f[o + at.max + 2])
		_boxes.append(DetailView.sim_box(lo, hi))
		_heights.append(hi.z - lo.z)
		_chunk_of.append(placed.i[o + at.chunk])
		chunk_count = maxi(chunk_count, placed.i[o + at.chunk] + 1)
		for s in STATES.size():
			var rows: int = placed.i[o + at.rows + s]
			var before := _first[s][b - 1] + maxi(_count[s][b - 1], 0) if b > 0 else 0
			_first[s].append(before)
			_count[s].append(rows)
	for c in chunk_count:
		_chunks.append(PackedInt32Array())
		_chunk_boxes.append(AABB())
		_chunk_heights.append(0.0)
	for b in _chunk_of.size():
		var c := _chunk_of[b]
		if c < 0:
			continue
		_chunk_boxes[c] = _boxes[b] if _chunks[c].is_empty() else _chunk_boxes[c].merge(_boxes[b])
		_chunk_heights[c] = maxf(_chunk_heights[c], _heights[b])
		_chunks[c].append(b)

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
		_modules.append(_batches.add_appearance(node, {"lod_px": _lod_px_per_m, "sized": false}))
	var rows := 0
	for state in _rows:
		rows += int(state.count)
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
	var changed := {}
	for b in states.keys() + _fallen.keys():
		if states.get(b) != _fallen.get(b):
			changed[_chunk_of[b]] = true
	for key in _packs.keys():
		if changed.has(key[0]):
			_packs.erase(key)
	_fallen = states
	_ruins.clear()
	var cells := {}
	var sorted := states.keys()
	sorted.sort()
	for b in sorted:
		var cell := Vector2i(floori(_frames[b * 4] / _chunk_m), floori(_frames[b * 4 + 1] / _chunk_m))
		if not cells.has(cell):
			cells[cell] = _ruins.size()
			_ruins.append({"buildings": PackedInt32Array(), "box": _boxes[b], "height": 0.0})
		var ruin: Dictionary = _ruins[cells[cell]]
		ruin.buildings.append(b)
		ruin.box = ruin.box.merge(_boxes[b])
		ruin.height = maxf(ruin.height, _heights[b])

## Chooses what `view` (`detail_view.gd`) draws and casts this frame, and
## counts it. `report` takes a blocker should the pool not hold the residents.
func prepare(view, report) -> void:
	var coarse_view := PackedInt32Array()
	var coarse_cast := PackedInt32Array()
	var near := []
	for c in _chunks.size():
		var box := _chunk_boxes[c]
		if _chunks[c].is_empty():
			continue
		if view.box_meets(box):
			var distance: float = view.distance_to(box)
			var wanted := _level(view, distance)
			if wanted < COARSEST:
				near.append([distance, c, wanted])
			else:
				coarse_view.append(c)
				if distance <= view.reach:
					coarse_cast.append(c)
		elif view.casts_into_view(box, _chunk_heights[c]):
			coarse_cast.append(c)
	near.sort_custom(func(a, b): return a[0] < b[0])
	var residents := []
	var used := 0
	var full := false
	for entry in near:
		var c: int = entry[1]
		var rows := _resident_rows(c, entry[2])
		if not full and used + rows <= _pool_records:
			used += rows
			residents.append([c, entry[2]])
			continue
		full = true
		coarse_view.append(c)
		if entry[0] <= view.reach:
			coarse_cast.append(c)
	most_resident = maxi(most_resident, used)
	if used > POOL_SHARE * _pool_records:
		report.add_blocker("buildings", "a view holds %d resident module instances; the browser's pool (%d records) refuses past about %d%% of them, which native does not port" % [used, _pool_records, roundi(POOL_SHARE * 100.0)])
	var ruins := []
	for r in _ruins.size():
		var ruin: Dictionary = _ruins[r]
		if view.box_meets(ruin.box):
			var distance: float = view.distance_to(ruin.box)
			ruins.append([r, _level(view, distance), distance <= view.reach])
		elif view.casts_into_view(ruin.box, ruin.height):
			ruins.append([r, -1, true])
	var choice := [coarse_view, coarse_cast, residents, ruins, _fallen.duplicate()]
	if choice == _last_choice:
		return
	_last_choice = choice
	_choose(coarse_view, coarse_cast, residents, ruins)

## The tier a chunk `distance` metres off draws at (`buildingLevel`).
func _level(view, distance: float) -> int:
	var px: float = view.pixels_per_metre / maxf(distance, 1e-3)
	return 0 if px > float(_lod_px_per_m[0]) else 1 if px > float(_lod_px_per_m[1]) else 2 if px > float(_lod_px_per_m[2]) else COARSEST

## Rows chunk `c`'s standing buildings draw at `tier`.
func _resident_rows(c: int, tier: int) -> int:
	var rows := 0
	for module_rows in _pack(c, tier).values():
		rows += module_rows.size() / ModelBatches.INSTANCE_FLOATS
	return rows

func _choose(coarse_view: PackedInt32Array, coarse_cast: PackedInt32Array, residents: Array, ruins: Array) -> void:
	var view := []
	var cast := []
	for k in _module_names.size() * TIERS:
		view.append(PackedFloat32Array())
		cast.append(PackedFloat32Array())
	for c in coarse_view:
		_add(view, _pack(c, COARSEST), COARSEST)
	for c in coarse_cast:
		_add(cast, _pack(c, COARSEST), COARSEST)
	for resident in residents:
		var pack := _pack(resident[0], resident[1])
		_add(view, pack, resident[1])
		_add(cast, pack, mini(resident[1] + _caster_coarser, COARSEST))
	for entry in ruins:
		var ruin: Dictionary = _ruins[entry[0]]
		for b in ruin.buildings:
			var state := STATES.find(_fallen[b])
			if entry[1] >= 0:
				_add(view, _rows_of(b, state, entry[1]), entry[1])
			if entry[2]:
				_add(cast, _rows_of(b, state, COARSEST), COARSEST)
	for m in _module_names.size():
		for tier in TIERS:
			counts[m * TIERS + tier] = view[m * TIERS + tier].size() / ModelBatches.INSTANCE_FLOATS
			if m < _modules.size() and _modules[m] >= 0:
				_batches.choose(_modules[m], tier, view[m * TIERS + tier], cast[m * TIERS + tier])

## Appends each module's instances in `pack` to `lists` at `tier`.
static func _add(lists: Array, pack: Dictionary, tier: int) -> void:
	for m in pack:
		lists[m * TIERS + tier].append_array(pack[m])

## Chunk `c`'s standing buildings' intact rows at `tier`, by module (cached).
func _pack(c: int, tier: int) -> Dictionary:
	var key := [c, tier]
	if not _packs.has(key):
		var pack := {}
		for b in _chunks[c]:
			if not _fallen.has(b):
				_merge(pack, _rows_of(b, 0, tier))
		_packs[key] = pack
	return _packs[key]

## Building `b`'s rows in state `s` whose tier mask has `tier`, by module.
func _rows_of(b: int, s: int, tier: int) -> Dictionary:
	var out := {}
	var rows: Dictionary = _rows[s]
	var at: Dictionary = rows.at
	var f: PackedFloat32Array = rows.f
	var i: PackedInt32Array = rows.i
	for r in range(_first[s][b], _first[s][b] + maxi(_count[s][b], 0)):
		var o: int = r * rows.stride
		if not (i[o + at.tiers] & (1 << tier)):
			continue
		var module: int = i[o + at.kitModule]
		if not out.has(module):
			out[module] = PackedFloat32Array()
		var basis := Coords.yaw_basis(f[o + at.yaw]).scaled_local(Vector3(f[o + at.scale], f[o + at.scale + 2], f[o + at.scale + 1]))
		var origin := Coords.sim_to_godot(Vector3(f[o + at.position], f[o + at.position + 1], f[o + at.position + 2]))
		ModelBatches.pack(out[module], Transform3D(basis, origin), Color(f[o + at.tint], f[o + at.tint + 1], f[o + at.tint + 2]))
	return out

static func _merge(into: Dictionary, more: Dictionary) -> void:
	for m in more:
		if into.has(m):
			into[m].append_array(more[m])
		else:
			into[m] = more[m].duplicate()

func prepare_batches(camera: Camera3D, size: Vector2) -> void:
	_batches.prepare(camera, size)
