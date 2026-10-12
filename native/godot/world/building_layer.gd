extends Node3D

## The map's buildings, resolved and walked by the browser's own Rust
## (`crates/presentation`'s `static_world`, through the extension's `BuildingWalk`): native
## places and chooses nothing; it follows what the side knows and draws the
## walk's records.

const ModelBatches = preload("res://world/model_batches.gd")
const Staging = preload("res://assets/staging.gd")

const OWNER := "native/building_layer"
const TIERS := 4
const STATES := ["intact", "ruin", "gutted"]

var _batches: ModelBatches
## Per resolved module: its appearance in `_batches`.
var _modules: Array = []
## Per resolved module, its kit and name (`{kit, module}`).
var modules: Array = []
## The module instances over every state.
var _rows := 0
var _layer
## Building -> the state the side knows it fell in.
var _fallen := {}
## Each building part's prop id -> its building, and the prop kinds a gutted
## building's parts become (`manifest.props`); empty until known.
var _part_building := {}
var _shells := {}
## Per module and tier (`module · 4 + tier`): what the last frame drew.
var counts := PackedInt32Array()

## The buildings of saved map `map` (the capture's), resolved from the
## repository's files; `sun` and `max_far_m` the light's. A map that does not
## resolve is a blocker in `report`.
func _init(map: String, sun: Vector3, max_far_m: float, report) -> void:
	name = "Buildings"
	_batches = ModelBatches.new("BuildingBatches", sun, max_far_m)
	add_child(_batches)
	_layer = ClassDB.instantiate("BuildingWalk")
	var resolved: Dictionary = _layer.load(Staging.repo_root(), map)
	if resolved.has("error"):
		report.add_blocker("buildings", resolved.error)
		return
	modules = resolved.modules
	_rows = int(resolved.rows)

## The kit sources the modules are drawn from (`catalog`'s).
func required_assets(catalog) -> Array:
	var sources := {}
	for module in modules:
		sources[catalog.source(module.kit, module.module)] = true
	sources.erase("")
	return sources.keys()

## Registers every module's tiers from `catalog`; a module it cannot draw is
## a blocker in `report`.
func build(catalog, report) -> void:
	for module in modules:
		var node: Node = catalog.node(module.kit, module.module)
		if node == null:
			report.add_blocker("buildings", "kit %s has no module %s staged" % [module.kit, module.module])
			_modules.append(-1)
			continue
		_modules.append(_batches.add_appearance(node))
	report.add_budget(OWNER, "module instances over every state, resolved in Rust", "buildings.rows", _rows, _rows, 0)

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

## Walks `view` (`detail_view.gd`), counts what it draws, and writes what
## changed.
func prepare(view, _report) -> void:
	for key in _layer.select(view.packed()):
		if key / TIERS < _modules.size() and _modules[key / TIERS] >= 0:
			_batches.choose(_modules[key / TIERS], key % TIERS, _layer.view(key), _layer.cast(key))
	_batches.write()
	counts = _layer.counts()
