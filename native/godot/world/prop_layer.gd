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
## sphere, and cast a tier coarser; nothing out of view casts. A chunk's
## instances are packed again only when one of their tiers changes.

const ModelBatches = preload("res://world/model_batches.gd")
const WordTables = preload("res://core/word_tables.gd")
const Coords = preload("res://core/coords.gd")
const Mulberry32 = preload("res://core/mulberry32.gd")
const ChunkGrid = preload("res://world/chunk_grid.gd")

const OWNER := "native/prop_layer"
const TIERS := 4
const FLOATS := ModelBatches.INSTANCE_FLOATS
## Square chunks the map's props are visited by (`STATIC_CHUNK_M`).
const CHUNK_M := 64.0
## A model's sun shadow reaches this far past it (`modelDetail.ts` `SHADOW_MARGIN_M`).
const SHADOW_MARGIN_M := 3.0
const CASTER_COARSER := 1
## A model's tier when it is out of view.
const CULLED := 255
## A map model no knowledge replaces (`propManifest.ts` `NO_PROP`).
const NO_PROP := 0xffffffff

## Placed models: per model its look, packed instance, centre (half its size
## up, Godot space), size and reach (`modelLayer.ts` `boundsSize`, grown by its
## largest scale).
class Models:
	var looks := PackedInt32Array()
	var packs := PackedFloat32Array()
	var centres := PackedVector3Array()
	var sizes := PackedFloat32Array()
	var radii := PackedFloat32Array()

	func add(look: int, transform: Transform3D, tint: Color, size: float, radius: float) -> void:
		looks.append(look)
		ModelBatches.pack(packs, transform, tint)
		centres.append(transform.origin + Vector3(0.0, size / 2.0, 0.0))
		sizes.append(size)
		radii.append(radius)

	## Each model's tier for `view` (`detail_px` the tier rule), or `CULLED`.
	func tiers(members: PackedInt32Array, view, lod: Array) -> PackedByteArray:
		var out := PackedByteArray()
		out.resize(members.size())
		var eye: Vector3 = view.eye
		var ppm: float = view.pixels_per_metre
		var l0 := float(lod[0])
		var l1 := float(lod[1])
		var l2 := float(lod[2])
		for n in members.size():
			var i := members[n]
			if not view.sphere_meets(centres[i], radii[i] + SHADOW_MARGIN_M):
				out[n] = CULLED
				continue
			var px := sizes[i] * ppm / maxf(eye.distance_to(centres[i]), 1e-3)
			out[n] = 0 if px > l0 else 1 if px > l1 else 2 if px > l2 else 3
		return out

	## `members` at `tiers`, packed by look and tier (`look · 4 + tier`).
	func packed(members: PackedInt32Array, tiers: PackedByteArray) -> Dictionary:
		var out := {}
		for n in members.size():
			if tiers[n] == CULLED:
				continue
			var i := members[n]
			var key := looks[i] * TIERS + tiers[n]
			if not out.has(key):
				out[key] = PackedFloat32Array()
			out[key].append_array(packs.slice(i * FLOATS, (i + 1) * FLOATS))
		return out

var _props: Dictionary
## `presentation.models.lod_px`: the tiers by projected height.
var _lod_px: Array
var _batches: ModelBatches
## Per look: its appearance in the batches (-1 until registered), its size
## and its reach.
var _looks := PackedInt32Array()
var _sizes := PackedFloat32Array()
var _radii := PackedFloat32Array()
## The map's fitted models (`manifest.props.map`), each one's map prop (-1:
## none), and per chunk its box, its models standing (not hidden by what the
## side knows replaced), their tiers, packs and a count of their packings.
var _map_table: Dictionary
var _map := Models.new()
var _map_props := PackedInt64Array()
var _chunk_members: Array[PackedInt32Array] = []
var _chunk_boxes: Array[AABB] = []
var _standing: Array[PackedInt32Array] = []
var _tiers: Array[PackedByteArray] = []
var _packs: Array[Dictionary] = []
var _versions := PackedInt32Array()
## The chunks gathered for culling (`chunk_grid.gd`).
var _grid: ChunkGrid
## Per chunk: its standing models' smallest and largest size.
var _least := PackedFloat32Array()
var _most := PackedFloat32Array()
## The map props hidden by what the side knows replaced them.
var _hidden := {}
## Each building part's prop id (`manifest.props.buildings.parts`).
var _parts := {}
## The known props' models, their tiers, and the last frame's plan.
var _known := Models.new()
var _known_tiers := PackedByteArray()
var _last_plan: Array = []
## Per known look's appearance name, what the last frame drew: tiers 0–3,
## then as a card (props have none).
var counts := {}

func _init(props: Dictionary, map_table: Dictionary, sun: Vector3, max_far_m: float) -> void:
	name = "Props"
	_props = props
	_lod_px = props.detail.lod_px
	_map_table = WordTables.records(map_table)
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
	_place_map()
	report.add_budget(OWNER, "the browser's fitted map props", "props.map", int(_map_table.count), _map.looks.size(), int(_map_table.count) - _map.looks.size())

## Follows what the side knows (`knownProps` of the frame's publication).
func set_known(known: Array) -> void:
	var hidden := {}
	_known = Models.new()
	for prop in known:
		if prop.authoredProp != null:
			if _parts.has(int(prop.authoredProp)):
				continue
			hidden[int(prop.authoredProp)] = true
		if prop.destroyed:
			continue
		for model in fit(_props.fit, prop):
			if _looks[model[0]] >= 0:
				_known.add(model[0], model[1], model[2], _sizes[model[0]] * model[3], _radii[model[0]] * model[3])
	if hidden != _hidden:
		var changed := {}
		for c in _chunk_members.size():
			for i in _chunk_members[c]:
				if hidden.has(_map_props[i]) != _hidden.has(_map_props[i]):
					changed[c] = true
		_hidden = hidden
		for c in changed:
			_stand(c)

## Chooses what `view` (`detail_view.gd`) draws of the props, each at its own
## tier, and counts the known ones by appearance. The batches are written
## only when what the frame draws differs from the last frame's.
func prepare(view) -> void:
	var lod := _lod_px
	var plan := []
	for c in _grid.visit(view, _chunk_boxes, _least, false)[0]:
		if _standing[c].is_empty():
			continue
		var tiers := _chunk_tiers(c, view, lod)
		if tiers != _tiers[c]:
			_tiers[c] = tiers
			_packs[c] = _map.packed(_standing[c], tiers)
			_versions[c] += 1
		plan.append(Vector2i(c, _versions[c]))
	var every := PackedInt32Array(range(_known.looks.size()))
	_known_tiers = _known.tiers(every, view, lod)
	counts.clear()
	for n in every.size():
		if _known_tiers[n] == CULLED:
			continue
		var name: String = _props.looks[_known.looks[n]].appearance
		if not counts.has(name):
			counts[name] = [0, 0, 0, 0, 0]
		counts[name][_known_tiers[n]] += 1
	plan.append(_known_tiers)
	plan.append(_known.packs)
	if plan == _last_plan:
		return
	_last_plan = plan
	var lists := {}
	for entry in plan.slice(0, plan.size() - 2):
		_add(lists, _packs[entry.x])
	_add(lists, _known.packed(every, _known_tiers))
	# What the view draws at a tier, the cascades draw a tier coarser.
	var casts := {}
	for key in lists:
		var coarser: int = key - key % TIERS + mini(key % TIERS + CASTER_COARSER, TIERS - 1)
		if not casts.has(coarser):
			casts[coarser] = PackedFloat32Array()
		casts[coarser].append_array(lists[key])
	for look in _looks.size():
		if _looks[look] < 0:
			continue
		for tier in TIERS:
			_batches.choose(_looks[look], tier, lists.get(look * TIERS + tier, PackedFloat32Array()), casts.get(look * TIERS + tier, PackedFloat32Array()))
	_batches.write()

## Chunk `c`'s models' tiers for `view`. A chunk wholly in view whose
## largest model at its nearest point and smallest at its farthest take the
## same tier has every model at it, and is not walked model by model.
func _chunk_tiers(c: int, view, lod: Array) -> PackedByteArray:
	var box := _chunk_boxes[c]
	if view.box_within(box):
		var ppm: float = view.pixels_per_metre
		var near := _tier(_most[c] * ppm / maxf(view.distance_to(box), 1e-3), lod)
		if near == _tier(_least[c] * ppm / maxf(view.farthest(box), 1e-3), lod):
			var tiers := PackedByteArray()
			tiers.resize(_standing[c].size())
			tiers.fill(near)
			return tiers
	return _map.tiers(_standing[c], view, lod)

static func _tier(px: float, lod: Array) -> int:
	return 0 if px > float(lod[0]) else 1 if px > float(lod[1]) else 2 if px > float(lod[2]) else 3

static func _add(lists: Dictionary, packs: Dictionary) -> void:
	for key in packs:
		if not lists.has(key):
			lists[key] = PackedFloat32Array()
		lists[key].append_array(packs[key])


## The known props, by appearance in name order, each per tier then as a card
## (`BattleFrame.drawnInstances` `knownProps`).
func drawn() -> Array:
	var names := counts.keys()
	names.sort()
	return names.map(func(appearance): return {"appearance": appearance, "levels": counts[appearance]})

## The map's models, chunked by where they stand.
func _place_map() -> void:
	var at: Dictionary = _map_table.at
	var f: PackedFloat32Array = _map_table.f
	var i: PackedInt32Array = _map_table.i
	var cells := {}
	for m in _map_table.count:
		var o: int = m * _map_table.stride
		var look: int = i[o + at.look]
		if _looks[look] < 0:
			continue
		var position := Vector3(f[o + at.position], f[o + at.position + 1], f[o + at.position + 2])
		var scale := Vector3(f[o + at.scale], f[o + at.scale + 1], f[o + at.scale + 2])
		var grow := maxf(scale.x, maxf(scale.y, scale.z))
		var n := _map.looks.size()
		_map.add(look, placed(position, f[o + at.yaw], scale), Color(f[o + at.tint], f[o + at.tint + 1], f[o + at.tint + 2]), _sizes[look] * grow, _radii[look] * grow)
		var prop: int = i[o + at.prop] & 0xffffffff
		_map_props.append(-1 if prop == NO_PROP else prop)
		var cell := Vector2i(floori(position.x / CHUNK_M), floori(position.y / CHUNK_M))
		if not cells.has(cell):
			cells[cell] = _chunk_members.size()
			_chunk_members.append(PackedInt32Array())
			_chunk_boxes.append(AABB())
		var c: int = cells[cell]
		var r := _map.radii[n] + SHADOW_MARGIN_M
		var reach := AABB(_map.centres[n] - Vector3(r, r, r), Vector3(r, r, r) * 2.0)
		_chunk_boxes[c] = reach if _chunk_members[c].is_empty() else _chunk_boxes[c].merge(reach)
		_chunk_members[c].append(n)
	for c in _chunk_members.size():
		_standing.append(PackedInt32Array())
		_tiers.append(PackedByteArray())
		_packs.append({})
		_versions.append(0)
		_least.append(0.0)
		_most.append(0.0)
		_stand(c)
	_grid = ChunkGrid.new(_chunk_boxes, _least, 8.0 * CHUNK_M)

## Chunk `c`'s models less those hidden; tiered again when next in view.
func _stand(c: int) -> void:
	var standing := PackedInt32Array()
	_least[c] = INF
	_most[c] = 0.0
	for n in _chunk_members[c]:
		if not _hidden.has(_map_props[n]):
			standing.append(n)
			_least[c] = minf(_least[c], _map.sizes[n])
			_most[c] = maxf(_most[c], _map.sizes[n])
	_standing[c] = standing
	_tiers[c] = PackedByteArray()
	_packs[c] = {}
	_versions[c] += 1

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
