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
## The walk is the Rust extension's (`StaticLayer`, `native/godot-binding`).
## The browser expands residents over a few frames and packs its pool by
## kind; native, stepping records far apart, settles each frame at once and
## counts the pool by records. A frame whose residents would pass the share of
## the pool packing always leaves free is a blocker, not a guess.

const ModelBatches = preload("res://world/model_batches.gd")

const OWNER := "native/building_layer"
const TIERS := 4
const STATES := ["intact", "ruin", "gutted"]
## The browser's pool refuses a record once packing cannot give every kind a
## quarter to spare (`placementPool.ts`): residents past this share of its
## records are never all drawn there.
const POOL_SHARE := 0.75

var _batches: ModelBatches
var _pool_records: int
## Per exported module: its appearance in `_batches`.
var _modules: Array = []
var _module_names: Array = []
var _tables: Dictionary
var _layer
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
	_pool_records = int(buildings.tiers.poolRecords)
	_module_names = buildings.modules
	_layer = ClassDB.instantiate("StaticLayer")
	_layer.set_kinds(_module_names.size(), _module_names.size())
	_layer.add_buildings(tables, buildings.tiers)

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
	var placed := int(_tables.buildings.layout.count)
	report.add_budget(OWNER, "the browser's module instances, every building in every state its template has", "buildings", placed, placed, 0)
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
	_fallen = states
	var known_states := PackedInt32Array(states.values().map(func(state): return STATES.find(state)))
	for b in _layer.set_fallen(PackedInt32Array(states.keys()), known_states):
		report.add_blocker("buildings", "building %d is known %s, and its template has no %s rows" % [b, states[b], states[b]])

## Chooses what `view` (`detail_view.gd`) draws and casts this frame, counts
## it, and writes what changed. `report` takes a blocker should the pool not
## hold the residents.
func prepare(view, report) -> void:
	for key in _layer.select(view.packed()):
		if key / TIERS < _modules.size() and _modules[key / TIERS] >= 0:
			_batches.choose(_modules[key / TIERS], key % TIERS, _layer.view(key), _layer.cast(key))
	_batches.write()
	counts = _layer.counts(-1)
	var used: int = _layer.most_resident()
	if used > most_resident and used > POOL_SHARE * _pool_records:
		report.add_blocker("buildings", "a view holds %d resident module instances; the browser's pool (%d records) refuses past about %d%% of them, which native does not port" % [used, _pool_records, roundi(POOL_SHARE * 100.0)])
	most_resident = used
