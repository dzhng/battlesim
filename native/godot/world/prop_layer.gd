extends Node3D

## The side's props as the browser draws them (`models/propAppearance.ts`,
## `BattleFrame.setStructures`): the map's props, fitted once by the browser
## (`manifest.props.map`), less those the side knows replaced, then every prop
## it has learned (wrecks, shoved bodies, rubble), fitted here by the port of
## `PropAppearances.fit` from the tables the browser chose among
## (`manifest.props.fit`). A known prop that is a building's part is the
## building's to draw (`building_layer.gd`).
##
## A known prop takes the tier its projected size calls for, one by one, as
## the browser's models layer gives it (`modelDetail.ts`); the map's props are
## static, chunked and tiered by `model_batches.gd`'s own rule.

const ModelBatches = preload("res://world/model_batches.gd")
const WordTables = preload("res://core/word_tables.gd")
const Coords = preload("res://core/coords.gd")
const Mulberry32 = preload("res://core/mulberry32.gd")

const OWNER := "native/prop_layer"
const TIERS := 4
## A model's sun shadow reaches this far past it (`modelDetail.ts` `SHADOW_MARGIN_M`).
const SHADOW_MARGIN_M := 3.0
const CASTER_COARSER := 1
## A map model no knowledge replaces (`propManifest.ts` `NO_PROP`).
const NO_PROP := 0xffffffff

var _props: Dictionary
var _sun: Vector3
var _max_far_m: float
var _detail: Dictionary
## Per look: its appearance in the current map batches and in the known
## batches (-1 until registered), and its size and radius (`modelLayer.ts`
## `boundsSize`, from its finest tier).
var _map_looks := PackedInt32Array()
var _known_looks := PackedInt32Array()
var _sizes := PackedFloat32Array()
var _radii := PackedFloat32Array()
## The map's fitted models (`WordTables.records`), and the props whose models
## are hidden because the side knows what replaced them.
var _map: Dictionary
var _hidden := {}
var _map_batches: ModelBatches
var _known_batches: ModelBatches
var _catalog
## Each building part's prop id (`manifest.props.buildings.parts`).
var _parts := {}
## This frame's known models: `[look, Transform3D, tint, size, radius]`.
var _known: Array = []
## Per known look's appearance name, what the last frame drew: tiers 0–3,
## then as a card (props have none).
var counts := {}

func _init(props: Dictionary, map_table: Dictionary, sun: Vector3, max_far_m: float) -> void:
	name = "Props"
	_props = props
	_sun = sun
	_max_far_m = max_far_m
	_detail = {"lod_px": props.detail.lod_px, "sized": true}
	_map = WordTables.records(map_table)
	for parts in props.buildings.parts:
		for prop in parts:
			_parts[int(prop)] = true
	for look in props.looks.size():
		_map_looks.append(-1)
		_known_looks.append(-1)
		_sizes.append(0.0)
		_radii.append(0.0)
	_known_batches = ModelBatches.new("KnownProps", sun, max_far_m)
	add_child(_known_batches)

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
	_catalog = catalog
	for look in _props.looks.size():
		var node: Node = catalog.node(_props.looks[look].appearance, _props.looks[look].state)
		if node == null:
			report.add_blocker("props", "look %s/%s is not staged" % [_props.looks[look].appearance, _props.looks[look].state])
			continue
		_known_looks[look] = _known_batches.add_appearance(node, _detail)
		# The size the browser tiers a model by: its finest tier's largest
		# extent, and its reach from its foot.
		var box := _known_batches.bounds(_known_looks[look])
		_sizes[look] = maxf(box.size.y, maxf(box.size.x, box.size.z))
		_radii[look] = Vector3(maxf(absf(box.position.x), absf(box.end.x)), maxf(absf(box.position.z), absf(box.end.z)), box.size.y).length()
	_place_map()
	report.add_budget(OWNER, "the browser's fitted map props, tiered by 64 m chunk where the browser tiers each", "props.map", int(_map.count), int(_map.count), 0)

## Follows what the side knows (`knownProps` of the frame's publication).
func set_known(known: Array) -> void:
	var hidden := {}
	_known.clear()
	for prop in known:
		if prop.authoredProp != null:
			if _parts.has(int(prop.authoredProp)):
				continue
			hidden[int(prop.authoredProp)] = true
		if not prop.destroyed:
			_known.append_array(fit(_props.fit, prop))
	if hidden != _hidden:
		_hidden = hidden
		_place_map()

## Chooses what `view` (`detail_view.gd`) draws of the known props, each at
## its own tier (`modelDetail`), and counts it by appearance.
func prepare(view) -> void:
	counts.clear()
	var view_lists := {}
	var cast_lists := {}
	for model in _known:
		var look: int = model[0]
		if _known_looks[look] < 0:
			continue
		var transform: Transform3D = model[1]
		var grow: float = model[3]
		var size := _sizes[look] * grow
		var centre := transform.origin + Vector3(0.0, size / 2.0, 0.0)
		if not view.sphere_meets(centre, _radii[look] * grow + SHADOW_MARGIN_M):
			continue
		var px: float = size * view.pixels_per_metre / maxf(view.eye.distance_to(centre), 1e-3)
		var lod: Array = _detail.lod_px
		var tier := 0 if px > float(lod[0]) else 1 if px > float(lod[1]) else 2 if px > float(lod[2]) else 3
		var name: String = _props.looks[look].appearance
		if not counts.has(name):
			counts[name] = [0, 0, 0, 0, 0]
		counts[name][tier] += 1
		for list in [[view_lists, tier], [cast_lists, mini(tier + CASTER_COARSER, TIERS - 1)]]:
			var key := Vector2i(look, list[1])
			if not list[0].has(key):
				list[0][key] = PackedFloat32Array()
			ModelBatches.pack(list[0][key], transform, model[2])
	for look in _known_looks.size():
		if _known_looks[look] < 0:
			continue
		for tier in TIERS:
			var key := Vector2i(look, tier)
			_known_batches.choose(_known_looks[look], tier, view_lists.get(key, PackedFloat32Array()), cast_lists.get(key, PackedFloat32Array()))

func prepare_batches(camera: Camera3D, size: Vector2) -> void:
	_map_batches.prepare(camera, size)
	_known_batches.prepare(camera, size)

## The known props, by appearance in name order, each per tier then as a card
## (`BattleFrame.drawnInstances` `knownProps`).
func drawn() -> Array:
	var names := counts.keys()
	names.sort()
	return names.map(func(appearance): return {"appearance": appearance, "levels": counts[appearance]})

## The map's models less those the side knows replaced, as statics.
func _place_map() -> void:
	if _map_batches != null:
		_map_batches.free()
	_map_batches = ModelBatches.new("MapProps", _sun, _max_far_m)
	add_child(_map_batches)
	_map_looks.fill(-1)
	var at: Dictionary = _map.at
	var f: PackedFloat32Array = _map.f
	var i: PackedInt32Array = _map.i
	for m in _map.count:
		var o: int = m * _map.stride
		var prop: int = i[o + at.prop] & 0xffffffff
		if prop != NO_PROP and _hidden.has(prop):
			continue
		var look: int = i[o + at.look]
		if _map_looks[look] < 0:
			var node: Node = _catalog.node(_props.looks[look].appearance, _props.looks[look].state)
			if node == null:
				continue
			_map_looks[look] = _map_batches.add_appearance(node, _detail)
		var transform := placed(Vector3(f[o + at.position], f[o + at.position + 1], f[o + at.position + 2]), f[o + at.yaw], Vector3(f[o + at.scale], f[o + at.scale + 1], f[o + at.scale + 2]))
		_map_batches.add_static(_map_looks[look], transform, Color(f[o + at.tint], f[o + at.tint + 1], f[o + at.tint + 2]))
	_map_batches.seal()

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
