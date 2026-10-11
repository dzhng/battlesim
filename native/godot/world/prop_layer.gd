extends Node3D

## The side's props as the browser draws them (`models/propAppearance.ts`,
## `BattleFrame.setStructures`): the map's props, fitted once by the browser
## (`manifest.props.map`), less those the side knows replaced, then every prop
## it has learned (wrecks, shoved bodies, rubble), fitted here by the port of
## `PropAppearances.fit` from the tables the browser chose among
## (`manifest.props.fit`). A known prop that is a building's part is the
## building's to draw (`building_layer.gd`).
##
## Every prop takes the tier its projected size calls for, one by one, as the
## browser's models layer gives it (`modelDetail.ts`): the map's props are
## visited by 64 m chunk in view (`staticsInView`), each culled by its own
## sphere, and cast a tier coarser; nothing out of view casts. The walk is the
## Rust extension's (`StaticSelection`, `native/godot-binding`).

const ModelBatches = preload("res://world/model_batches.gd")
const WordTables = preload("res://core/word_tables.gd")
const Coords = preload("res://core/coords.gd")
const Mulberry32 = preload("res://core/mulberry32.gd")

const OWNER := "native/prop_layer"
const TIERS := 4
## Square chunks the map's props are visited by (`STATIC_CHUNK_M`).
const CHUNK_M := 64.0

var _props: Dictionary
var _batches: ModelBatches
## Per look: its appearance in the batches (-1 until registered), its size
## and its reach.
var _looks := PackedInt32Array()
var _sizes := PackedFloat32Array()
var _radii := PackedFloat32Array()
## The map's fitted models (`manifest.props.map`).
var _map_table: Dictionary
## The selection (`StaticSelection`): the map's props, and the known ones.
var _selection
var _map_pop := -1
var _known_pop := -1
## The map props hidden by what the side knows replaced them, and the known
## props' models as last placed.
var _hidden := {}
var _known_models: Array = []
## Each building part's prop id (`manifest.props.buildings.parts`).
var _parts := {}
## Per known look's appearance name, what the last frame drew: tiers 0–3,
## then as a card (props have none).
var counts := {}

func _init(props: Dictionary, map_table: Dictionary, sun: Vector3, max_far_m: float) -> void:
	name = "Props"
	_props = props
	_map_table = map_table
	for parts in props.buildings.parts:
		for prop in parts:
			_parts[int(prop)] = true
	for look in props.looks.size():
		_looks.append(-1)
		_sizes.append(0.0)
		_radii.append(0.0)
	_batches = ModelBatches.new("PropBatches", sun, max_far_m)
	add_child(_batches)

## Every look's GLB (`catalog`'s).
func required_assets(catalog) -> Array:
	var sources := {}
	for look in _props.looks:
		sources[catalog.source(look.appearance, look.state)] = true
	sources.erase("")
	return sources.keys()

## Registers every look the props draw, and places the map's props; a look
## the catalog cannot draw is a blocker in `report`.
func build(catalog, report) -> void:
	for look in _props.looks.size():
		var node: Node = catalog.node(_props.looks[look].appearance, _props.looks[look].state)
		if node == null:
			report.add_blocker("props", "look %s/%s is not staged" % [_props.looks[look].appearance, _props.looks[look].state])
			continue
		_looks[look] = _batches.add_appearance(node)
		# The size the browser tiers a model by: its finest tier's largest
		# extent, and its reach from its foot.
		var box := _batches.bounds(_looks[look])
		_sizes[look] = maxf(box.size.y, maxf(box.size.x, box.size.z))
		_radii[look] = Vector3(maxf(absf(box.position.x), absf(box.end.x)), maxf(absf(box.position.z), absf(box.end.z)), box.size.y).length()
	_selection = ClassDB.instantiate("StaticSelection")
	_selection.set_kinds(_looks.size())
	var lod := PackedFloat64Array(_props.detail.lod_px)
	var at := {}
	for field in _map_table.layout.fields:
		at[String(field.name)] = int(field.word)
	_map_pop = _selection.add_props(_map_table.words, int(_map_table.layout.stride), at, _sizes, _radii, CHUNK_M, lod)
	_known_pop = _selection.add_models(lod)
	report.add_budget(OWNER, "the browser's fitted map props", "props.map", int(_map_table.layout.count), int(_map_table.layout.count), 0)

## Follows what the side knows (`knownProps` of the frame's publication).
func set_known(known: Array) -> void:
	var hidden := {}
	var models := []
	for prop in known:
		if prop.authoredProp != null:
			if _parts.has(int(prop.authoredProp)):
				continue
			hidden[int(prop.authoredProp)] = true
		if not prop.destroyed:
			models.append_array(fit(_props.fit, prop).filter(func(model): return _looks[model[0]] >= 0))
	if hidden != _hidden:
		_hidden = hidden
		_selection.hide_keys(_map_pop, PackedInt64Array(hidden.keys()))
	if models != _known_models:
		_known_models = models
		var packs := PackedFloat32Array()
		var kinds := PackedInt32Array()
		var sizes := PackedFloat32Array()
		var radii := PackedFloat32Array()
		for model in models:
			ModelBatches.pack(packs, model[1], model[2])
			kinds.append(model[0])
			sizes.append(_sizes[model[0]] * model[3])
			radii.append(_radii[model[0]] * model[3])
		_selection.set_models(_known_pop, packs, kinds, sizes, radii)

## Chooses what `view` (`detail_view.gd`) draws of the props, each at its own
## tier, counts the known ones by appearance, and writes what changed.
func prepare(view) -> void:
	for key in _selection.select(view.eye, view.sides_packed(), view.pixels_per_metre, view.reach, view.fall):
		var look: int = key / TIERS
		if _looks[look] >= 0:
			_batches.choose(_looks[look], key % TIERS, _selection.view(key), _selection.cast(key))
	_batches.write()
	counts.clear()
	var known: PackedInt32Array = _selection.counts(_known_pop)
	for k in known.size():
		if known[k] > 0:
			var name: String = _props.looks[k / TIERS].appearance
			if not counts.has(name):
				counts[name] = [0, 0, 0, 0, 0]
			counts[name][k % TIERS] += known[k]

## The known props, by appearance in name order, each per tier then as a card
## (`BattleFrame.drawnInstances` `knownProps`).
func drawn() -> Array:
	var names := counts.keys()
	names.sort()
	return names.map(func(appearance): return {"appearance": appearance, "levels": counts[appearance]})

## A model at sim `position`, turned `yaw` about +Z, scaled per axis by sim `scale`.
static func placed(position: Vector3, yaw: float, scale: Vector3) -> Transform3D:
	return Transform3D(Coords.yaw_basis(yaw).scaled_local(Vector3(scale.x, scale.z, scale.y)), Coords.sim_to_godot(position))

## The models drawing known prop `box` (`fit` of `PropAppearances`):
## `[look, Transform3D, tint, grow]` each, `grow` its largest scale. A wreck
## is its unit's own wreck; anything else the candidate whose footprint is
## nearest its box; a module repeats along its long side; a kind with no art
## draws the stand-in box, or nothing.
static func fit(table: Dictionary, box: Dictionary) -> Array:
	var kind = table.kinds.get(box.kind)
	if kind == null:
		return []
	var half := Vector3(float(box.half[0]), float(box.half[1]), float(box.half[2]))
	var chosen = null
	if kind.wreck:
		chosen = table.wrecks.get(box.wreckOf if box.wreckOf != null else "")
	else:
		var least := INF
		for candidate in kind.candidates:
			var misfit := 0.0
			for a in 3:
				misfit += absf(log(half[a] / float(candidate.footprint[a])))
			if misfit < least:
				least = misfit
				chosen = candidate
	var base := Vector3(float(box.center[0]), float(box.center[1]), float(box.baseZ))
	var yaw := float(box.yaw)
	if chosen == null:
		if kind.standIn == null:
			return []
		var tint := Color(float(kind.standIn[0]), float(kind.standIn[1]), float(kind.standIn[2]))
		return [_model(int(table.standIn), base, yaw, 0.0, 0.0, half * 2.0, tint)]
	var foot := Vector3(float(chosen.footprint[0]), float(chosen.footprint[1]), float(chosen.footprint[2]))
	if not kind.modular:
		var tint := paint_of(chosen.paints, box) if chosen.paints != null else Color.WHITE
		return [_model(int(chosen.look), base, yaw, 0.0, 0.0, half / foot, tint)]
	# A module runs along its own long axis; turn it to the box's long side.
	var turn := 0.0 if (half.x >= half.y) == (foot.x >= foot.y) else PI / 2.0
	var long := half.x if turn == 0.0 else half.y
	var across := half.y if turn == 0.0 else half.x
	var count := maxi(1, roundi(long / foot.x))
	var step := 2.0 * long / count
	var scale := Vector3(long / (count * foot.x), across / foot.y, half.z / foot.z)
	var out := []
	for k in count:
		out.append(_model(int(chosen.look), base, yaw, -long + step * (k + 0.5), turn, scale, Color.WHITE))
	return out

static func _model(look: int, base: Vector3, yaw: float, along: float, turn: float, scale: Vector3, tint: Color) -> Array:
	var turned := yaw + turn
	var at := base + Vector3(along * cos(turned), along * sin(turned), 0.0)
	return [look, placed(at, turned, scale), tint, maxf(scale.x, maxf(scale.y, scale.z))]

## The paint `box` wears of `paints` (`paintOf`): drawn from the authored prop
## it stands for, else from where it stands, so the same car is the same
## colour in every battle.
static func paint_of(paints: Array, box: Dictionary) -> Color:
	var seed: int
	if box.authoredProp != null:
		seed = int(box.authoredProp)
	else:
		# JavaScript's `Math.round`: halves round up.
		seed = floori(float(box.center[0]) * 10.0 + 0.5) * 73856093 + floori(float(box.center[1]) * 10.0 + 0.5)
	var paint: Array = paints[floori(Mulberry32.sample(Mulberry32.create(seed)) * paints.size())]
	return Color(float(paint[0]), float(paint[1]), float(paint[2]))
