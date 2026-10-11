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
const ChunkGrid = preload("res://world/chunk_grid.gd")

const OWNER := "native/scenery_layer"
const TIERS := 4
const COARSEST := TIERS - 1
const POPULATIONS := ["forest", "understorey", "backdrop"]
## How a chunk draws in a frame's plan: whole at the coarsest tier, tree by
## tree, or only into the cascades.
const FAR := 0
const NEAR := 1
const CAST := 2
## A crown's heart, as a share of the tree's height (`lod.ts` `HEART`).
const FLOATS := ModelBatches.INSTANCE_FLOATS

## One population of trees or shrubs: every instance packed once, bucketed
## in chunks; what stands of it is rebuilt chunk by chunk.
class Population:
	var casts: bool
	## Per instance: its packed instance, its centre (half its height up, Godot
	## space), its height, its kind, its chunk and its id.
	var packs := PackedFloat32Array()
	var centres := PackedVector3Array()
	var heights := PackedFloat32Array()
	var reaches := PackedFloat32Array()
	var kinds := PackedInt32Array()
	var chunk_of := PackedInt32Array()
	var ids := PackedInt64Array()
	## Per instance: all it can draw (its reach about its foot, its height above).
	var bounds: Array[AABB] = []
	## Per instance: whether it stands (not felled, not on cleared ground).
	var standing := PackedByteArray()
	## Per chunk: every instance in it, and of those standing its box, its
	## tallest height, and its instances by kind, packed whole.
	var members: Array[PackedInt32Array] = []
	var boxes: Array[AABB] = []
	var sizes := PackedFloat32Array()
	var least := PackedFloat32Array()
	var whole: Array[Dictionary] = []
	var stood: Array[PackedInt32Array] = []
	## Per chunk while near: each standing instance's tier, its instances by
	## kind and tier (`kind · 4 + tier`), and a count of their rebuilds.
	var near_tiers: Array[PackedByteArray] = []
	var near_packs: Array[Dictionary] = []
	var near_version := PackedInt32Array()
	## The chunks gathered for culling (`chunk_grid.gd`).
	var grid: ChunkGrid

var _batches: ModelBatches
var _kinds: Array
var _appearances := PackedInt32Array()
var _lod_px: Array
var _chunk_m: float
var _caster_coarser: int
var _dressing_rules: Dictionary
var _populations := {}
## The dressing: its records, and per cell its box, first piece and count.
var _dressing: Dictionary
var _cells := {}
var _cell_list: Array[Dictionary] = []
## The felled trees and their stumps (`felled.gd`); null with no forest.
var _felled: Felled
## The trunk ids of the trees the forest stands without, and the learned
## ground's `cleared_version` the shrubs and dressing stand by.
var _fallen_ids := {}
var _learned
var _cleared_version := -1
## What the last frame drew, by chunk and cell (`_plan`): unchanged, the
## batches keep what they hold.
var _last_plan: Array = []
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
	_chunk_m = float(tiers.chunkM)
	_caster_coarser = int(tiers.casterCoarser)
	_dressing_rules = tiers.dressing
	for population in POPULATIONS:
		_populations[population] = _population(WordTables.records(tables[population]), population != "backdrop")
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
	for kind in _populations.forest.kinds:
		forest_kinds[kind] = true
	var barks := _kinds.map(func(kind): return Felled.bark_of(catalog.node(kind, "summer")) if catalog.node(kind, "summer") != null else null)
	_felled.build(_batches, _appearances, self, forest_kinds, barks)
	for population in POPULATIONS:
		var pop: Population = _populations[population]
		report.add_budget(OWNER, "the browser's %s instances" % population, "scenery." + population, pop.heights.size(), pop.heights.size(), 0)
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
		_restand(_populations.forest, func(i): return not felled.has(_populations.forest.ids[i]))
	var version: int = learned.cleared_version if learned != null else 0
	if learned != _learned or version != _cleared_version:
		_learned = learned
		_cleared_version = version
		var shrubs: Population = _populations.understorey
		_restand(shrubs, func(i): return not _cleared(learned, shrubs, i))
	_felled.show(fallen_bodies, _populations.forest)

## Chooses what `view` (`detail_view.gd`) draws and casts at presentation
## second `clock`, and counts it. The batches are written only when what the
## frame draws differs from the last frame's.
func prepare(view, clock: float) -> void:
	var plan := []
	for population in POPULATIONS:
		plan.append(_plan(_populations[population], view, counts[population]))
	plan.append(_plan_dressing(view, counts.dressing))
	if plan != _last_plan:
		_last_plan = plan
		_choose(plan)
	_felled.prepare(view, clock, _lod_px, counts.felled, counts.stumps)
	_batches.write()


## The tier an instance `height` metres tall draws at from `distance` (`tierFor`).
static func tier_for(height: float, distance: float, pixels_per_metre: float, lod_px: Array) -> int:
	var px := height * pixels_per_metre / maxf(distance, 1e-3)
	return 0 if px > float(lod_px[0]) else 1 if px > float(lod_px[1]) else 2 if px > float(lod_px[2]) else COARSEST

## What `view` draws of `pop`, counted into `drawn` by kind and tier: per
## chunk, `[chunk, FAR or NEAR or CAST, casts, its near packs' version]`.
func _plan(pop: Population, view, drawn: PackedInt32Array) -> Array:
	drawn.fill(0)
	var plan := []
	var visit: Array = pop.grid.visit(view, pop.boxes, pop.sizes, pop.casts)
	for c in visit[1]:
		plan.append([c, CAST, true, 0])
	for c in visit[0]:
		var box := pop.boxes[c]
		var distance: float = view.distance_to(box)
		if tier_for(pop.sizes[c], distance, view.pixels_per_metre, _lod_px) == COARSEST:
			for kind in pop.whole[c]:
				drawn[kind * TIERS + COARSEST] += pop.whole[c][kind].size() / FLOATS
			plan.append([c, FAR, pop.casts and distance <= view.reach, 0])
			continue
		_tier_near(pop, c, view)
		for key in pop.near_packs[c]:
			drawn[key] += pop.near_packs[c][key].size() / FLOATS
		plan.append([c, NEAR, pop.casts, pop.near_version[c]])
	return plan

## Near chunk `c`'s instances each at their own tier for `view`; their packs
## are rebuilt only when a tier changed.
func _tier_near(pop: Population, c: int, view) -> void:
	var stood := pop.stood[c]
	var tiers := PackedByteArray()
	tiers.resize(stood.size())
	var eye: Vector3 = view.eye
	var ppm: float = view.pixels_per_metre
	# Its tallest tree at the box's nearest point and its shortest at the
	# farthest taking one tier, every tree takes it.
	var near := tier_for(pop.sizes[c], view.distance_to(pop.boxes[c]), ppm, _lod_px)
	if near == tier_for(pop.least[c], view.farthest(pop.boxes[c]), ppm, _lod_px):
		tiers.fill(near)
	else:
		var l0 := float(_lod_px[0])
		var l1 := float(_lod_px[1])
		var l2 := float(_lod_px[2])
		for n in stood.size():
			var i := stood[n]
			var px := pop.heights[i] * ppm / maxf(eye.distance_to(pop.centres[i]), 1e-3)
			tiers[n] = 0 if px > l0 else 1 if px > l1 else 2 if px > l2 else COARSEST
	if tiers == pop.near_tiers[c]:
		return
	pop.near_tiers[c] = tiers
	var packs := {}
	for n in stood.size():
		var i := stood[n]
		var key := pop.kinds[i] * TIERS + tiers[n]
		if not packs.has(key):
			packs[key] = PackedFloat32Array()
		packs[key].append_array(pop.packs.slice(i * FLOATS, i * FLOATS + FLOATS))
	pop.near_packs[c] = packs
	pop.near_version[c] += 1

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

## Hands the batches every appearance and tier `plan` draws and casts.
func _choose(plan: Array) -> void:
	var view_lists := []
	var cast_lists := []
	for k in _kinds.size() * TIERS:
		view_lists.append(PackedFloat32Array())
		cast_lists.append(PackedFloat32Array())
	for p in POPULATIONS.size():
		var pop: Population = _populations[POPULATIONS[p]]
		for entry in plan[p]:
			var c: int = entry[0]
			if entry[1] == NEAR:
				for key in pop.near_packs[c]:
					view_lists[key].append_array(pop.near_packs[c][key])
					if entry[2]:
						cast_lists[key - key % TIERS + mini(key % TIERS + _caster_coarser, COARSEST)].append_array(pop.near_packs[c][key])
				continue
			for kind in pop.whole[c]:
				if entry[1] == FAR:
					view_lists[kind * TIERS + COARSEST].append_array(pop.whole[c][kind])
				if entry[2]:
					cast_lists[kind * TIERS + COARSEST].append_array(pop.whole[c][kind])
	for entry in plan[POPULATIONS.size()]:
		var pack: Dictionary = _cell_list[entry[0]].pack
		for kind in pack:
			view_lists[kind * TIERS + entry[1]].append_array(pack[kind])
	for k in _appearances.size():
		if _appearances[k] < 0:
			continue
		for tier in TIERS:
			_batches.choose(_appearances[k], tier, view_lists[k * TIERS + tier], cast_lists[k * TIERS + tier])

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

## Whether the side has seen the ground under instance `i` of `pop` cleared.
static func _cleared(learned, pop: Population, i: int) -> bool:
	if learned == null:
		return false
	var foot := Coords.godot_to_sim(pop.centres[i])
	return learned.cleared_at(foot.x, foot.y)

## Appends record `r` of `records` (a `SceneryInstance` table) as an instance.
static func pack_instance(out: PackedFloat32Array, records: Dictionary, r: int) -> void:
	var o: int = r * records.stride
	var at: Dictionary = records.at
	var f: PackedFloat32Array = records.f
	var basis := Coords.yaw_basis(f[o + at.pose + 3]).scaled_local(Vector3(f[o + at.shape], f[o + at.shape + 2], f[o + at.shape + 1]))
	var foot := Coords.sim_to_godot(Vector3(f[o + at.pose], f[o + at.pose + 1], f[o + at.pose + 2]))
	ModelBatches.pack(out, Transform3D(basis, foot), Color(f[o + at.tint], f[o + at.tint + 1], f[o + at.tint + 2]))

## A population of `records`, every instance standing, chunked.
func _population(records: Dictionary, casts: bool) -> Population:
	var pop := Population.new()
	pop.casts = casts
	var at: Dictionary = records.at
	var f: PackedFloat32Array = records.f
	var cells := {}
	for r in records.count:
		var o: int = r * records.stride
		var x: float = f[o + at.pose]
		var y: float = f[o + at.pose + 1]
		var z: float = f[o + at.pose + 2]
		var height: float = f[o + at.height]
		var reach: float = f[o + at.reach]
		pack_instance(pop.packs, records, r)
		pop.centres.append(Coords.sim_to_godot(Vector3(x, y, z + height / 2.0)))
		pop.heights.append(height)
		pop.reaches.append(reach)
		pop.kinds.append(records.i[o + at.kind])
		pop.ids.append(records.i[o + at.id] & 0xffffffff)
		pop.bounds.append(DetailView.sim_box(Vector3(x - reach, y - reach, z), Vector3(x + reach, y + reach, z + height)))
		var cell := Vector2i(floori(x / _chunk_m), floori(y / _chunk_m))
		if not cells.has(cell):
			cells[cell] = pop.members.size()
			pop.members.append(PackedInt32Array())
		pop.chunk_of.append(cells[cell])
		pop.members[cells[cell]].append(r)
	pop.standing.resize(records.count)
	pop.standing.fill(1)
	for c in pop.members.size():
		pop.boxes.append(AABB())
		pop.sizes.append(0.0)
		pop.least.append(0.0)
		pop.whole.append({})
		pop.stood.append(PackedInt32Array())
		pop.near_tiers.append(PackedByteArray())
		pop.near_packs.append({})
		pop.near_version.append(0)
		_rechunk(pop, c)
	pop.grid = ChunkGrid.new(pop.boxes, pop.sizes, 4.0 * _chunk_m)
	return pop

## Rebuilds `pop`'s chunk `c` from what stands in it.
static func _rechunk(pop: Population, c: int) -> void:
	var stood := PackedInt32Array()
	var whole := {}
	var box := AABB()
	var size := 0.0
	var least := INF
	for i in pop.members[c]:
		if not pop.standing[i]:
			continue
		box = pop.bounds[i] if stood.is_empty() else box.merge(pop.bounds[i])
		size = maxf(size, pop.heights[i])
		least = minf(least, pop.heights[i])
		stood.append(i)
		var kind := pop.kinds[i]
		if not whole.has(kind):
			whole[kind] = PackedFloat32Array()
		whole[kind].append_array(pop.packs.slice(i * FLOATS, i * FLOATS + FLOATS))
	pop.stood[c] = stood
	# Tiered again when next near.
	pop.near_tiers[c] = PackedByteArray()
	pop.near_version[c] += 1
	pop.boxes[c] = box
	pop.sizes[c] = size
	pop.least[c] = least
	if pop.grid != null:
		pop.grid.update(pop.grid.cell_of(c), pop.boxes, pop.sizes)
	pop.whole[c] = whole

## Stands each instance of `pop` that `stands` keeps, rebuilding the chunks
## where that changed.
static func _restand(pop: Population, stands: Callable) -> void:
	var changed := {}
	for i in pop.heights.size():
		var now := 1 if stands.call(i) else 0
		if now != pop.standing[i]:
			pop.standing[i] = now
			changed[pop.chunk_of[i]] = true
	for c in changed:
		_rechunk(pop, c)
