extends Node3D

## Draws placed models the way the browser's models layer does
## (`packages/battle-renderer/src/models/modelLayer.ts`, `modelDetail.ts` and
## `frame/staticChunks.ts`): instanced batches, one MultiMesh per appearance and
## detail tier, whose mesh holds one surface per material with every part's own
## transform baked in. Draw calls follow appearances × tiers × materials ×
## passes, not placed objects or their parts.
##
## - **Appearances.** Any node tree of meshes: a catalog scene, a kit's shell, a
##   stand-in. A mesh names its tier with a `_LOD<n>` suffix, finest first, and
##   an unsuffixed mesh is in every tier (`packages/scene-assets`' rule).
## - **Tiers.** Each frame a model takes the tier its projected size calls
##   for (`Detail`): by its size in pixels for a model, by a metre's pixels for a
##   building. Off screen, nothing.
## - **Statics** are placed once and bucketed in square chunks. A frame culls a
##   chunk against the view's side planes and gives each of its appearances the
##   tier the chunk's nearest point calls for; a batch is rewritten only when
##   one of its chunks changes. **Movers** are replaced every frame.
## - **Chosen** instances are a layer's own: a layer that ports a browser
##   owner's tier rule (buildings, scenery, known props) hands each appearance
##   and tier the instances it draws and casts this frame.
## - **Surfaces.** Every material draws as the browser's model surface
##   (`model_surface.gd`), and each instance carries its tint.
## - **Shadows.** A model drawn as a mesh casts through a shadow-only batch one
##   tier coarser (the tier never casts for itself), within the cascades'
##   reach. A chunk out of view whose shadow can land in it casts at the
##   coarsest tier and draws nothing else.

const DetailView = preload("res://world/detail_view.gd")
const ModelSurface = preload("res://world/model_surface.gd")

const TIERS := 4
const COARSEST := TIERS - 1
## Floats an instance takes in a batch: `MultiMesh.TRANSFORM_3D`'s 3×4 rows,
## then its custom data, the tint (`pack`).
const INSTANCE_FLOATS := 16
## Square chunks the statics are bucketed in, metres (`STATIC_CHUNK_M`).
const CHUNK_M := 64.0
## A model's sun shadow reaches this far past it, so a model just out of view
## still draws and casts (`SHADOW_MARGIN_M`).
const SHADOW_MARGIN_M := 3.0
## Shadow casters are drawn this many tiers coarser than the view (`CASTER_COARSER`).
const CASTER_COARSER := 1

## A chunk's state: unset, culled, casting only; in view it is its band of
## pixels per metre times two, plus one when it is within the shadows' reach.
const UNSET := -3
const CULLED := -1
const CAST_ONLY := -2

## One appearance: its tiers' merged meshes, its bounds, its detail rule and
## its batches.
class Appearance:
	## Per tier, or null where the tier has no geometry.
	var meshes: Array[ArrayMesh] = []
	## The detail size (its largest extent, metres) and the sphere it is drawn
	## within, about `center`, in its own space.
	var size: float
	var radius: float
	var center: Vector3
	var bounds: AABB
	## `[t0, t1, t2]`: tier 0 above t0, 1 above t1, 2 above t2, else 3.
	var lod_px: Array
	## Whether `lod_px` counts the model's projected size or a metre's.
	var sized: bool
	## Per tier: what the view draws and what the cascades draw.
	var view: Array[MultiMeshInstance3D] = []
	var cast: Array[MultiMeshInstance3D] = []
	## Its statics: chunk id and the appearance's entry in that chunk, in pairs.
	var chunks := PackedInt32Array()
	## Per tier: this frame's static, moving and chosen instances
	## (`INSTANCE_FLOATS` each), for the view and the cascades.
	var static_view: Array[PackedFloat32Array] = []
	var static_cast: Array[PackedFloat32Array] = []
	var moving_view: Array[PackedFloat32Array] = []
	var moving_cast: Array[PackedFloat32Array] = []
	var chosen_view: Array[PackedFloat32Array] = []
	var chosen_cast: Array[PackedFloat32Array] = []
	var statics_changed := false
	var chosen_changed := false
	## Whether it had movers in the last frame written.
	var moved := false

	## The tier for `pixels_per_metre` at the model.
	func tier(pixels_per_metre: float) -> int:
		var px := pixels_per_metre * (size if sized else 1.0)
		return 0 if px > lod_px[0] else 1 if px > lod_px[1] else 2 if px > lod_px[2] else COARSEST

	## The pixels per metre at which its tier changes.
	func breaks() -> Array:
		return lod_px.map(func(px): return px / size if sized else px)

## Statics in one square of ground.
class Chunk:
	## Its models' bounds, grown by the shadow margin.
	var box: AABB
	var appearances := PackedInt32Array()
	## Per appearance entry: its instances.
	var transforms: Array[PackedFloat32Array] = []
	## Per appearance entry: its tier in the view, while the chunk is in it.
	var tiers := PackedInt32Array()
	## The sorted pixels per metre at which any of its appearances changes tier.
	var breaks := PackedFloat32Array()
	var state := UNSET

var _appearances: Array[Appearance] = []
var _chunks: Array[Chunk] = []
## Statics placed before `seal`: appearance id, then its transform.
var _placed: Array = []
## This frame's movers: `[appearance id, Transform3D]` each.
var _movers: Array = []
## The direction toward the sun in Godot space, and how far the cascades
## reach in view depth.
var _sun: Vector3
var _max_far_m: float

## `sun` is the unit direction toward the sun in Godot space; `max_far_m` how
## far the cascades reach in view depth.
func _init(batches_name: String, sun: Vector3, max_far_m: float) -> void:
	name = batches_name
	_sun = sun
	_max_far_m = max_far_m

## Registers the meshes below `root` (not `root`'s own transform) as an
## appearance drawn at the tiers `detail` gives it (`{lod_px, sized}`);
## returns its id. A `scenery` appearance takes its instances' tint on every
## surface (`model_surface.gd`).
func add_appearance(root: Node, detail: Dictionary, scenery := false) -> int:
	var parts := []
	for child in root.get_children():
		_collect(child, Transform3D.IDENTITY, parts)
	var named: bool = parts.any(func(part): return part.tier >= 0)
	var appearance := Appearance.new()
	for tier in TIERS:
		appearance.meshes.append(_merged(parts.filter(func(part): return not named or part.tier < 0 or part.tier == tier), scenery))
		for list in [appearance.static_view, appearance.static_cast, appearance.moving_view, appearance.moving_cast, appearance.chosen_view, appearance.chosen_cast]:
			list.append(PackedFloat32Array())
		appearance.view.append(null)
		appearance.cast.append(null)
	var finest: ArrayMesh = appearance.meshes.filter(func(mesh): return mesh != null).front() if appearance.meshes.any(func(mesh): return mesh != null) else null
	appearance.bounds = finest.get_aabb() if finest != null else AABB()
	appearance.size = maxf(appearance.bounds.size[appearance.bounds.get_longest_axis_index()], 1e-3)
	appearance.center = appearance.bounds.get_center()
	appearance.radius = appearance.bounds.size.length() / 2.0
	appearance.lod_px = detail.lod_px
	appearance.sized = detail.sized
	_appearances.append(appearance)
	return _appearances.size() - 1

## Registers prebuilt meshes, one per tier (null where a tier has none), as
## an appearance whose instances a layer chooses (`choose`); every pose of
## it stays within `box`, which culls its batches. Returns its id.
func add_meshes(meshes: Array[ArrayMesh], box: AABB) -> int:
	var appearance := Appearance.new()
	for tier in TIERS:
		if meshes[tier] != null:
			meshes[tier].custom_aabb = box
		appearance.meshes.append(meshes[tier])
		for list in [appearance.static_view, appearance.static_cast, appearance.moving_view, appearance.moving_cast, appearance.chosen_view, appearance.chosen_cast]:
			list.append(PackedFloat32Array())
		appearance.view.append(null)
		appearance.cast.append(null)
	appearance.bounds = box
	appearance.size = maxf(box.size[box.get_longest_axis_index()], 1e-3)
	appearance.center = box.get_center()
	appearance.radius = box.size.length() / 2.0
	appearance.lod_px = [INF, INF, INF]
	appearance.sized = false
	_appearances.append(appearance)
	return _appearances.size() - 1

## The finest tier's bounds of `appearance`, in its own space.
func bounds(appearance: int) -> AABB:
	return _appearances[appearance].bounds

## Tier `tier`'s merged mesh of `appearance`, or null where it has none.
func mesh(appearance: int, tier: int) -> ArrayMesh:
	return _appearances[appearance].meshes[tier]

## Places one instance of `appearance` for the whole run, tinted `tint`.
func add_static(appearance: int, transform: Transform3D, tint := Color.WHITE) -> void:
	_placed.append(appearance)
	_placed.append(transform)
	_placed.append(tint)

## Buckets the statics placed so far into chunks; call once, after the last.
func seal() -> void:
	var cells := {}
	for i in range(0, _placed.size(), 3):
		var appearance: Appearance = _appearances[_placed[i]]
		var transform: Transform3D = _placed[i + 1]
		var cell := Vector2i(floori(transform.origin.x / CHUNK_M), floori(transform.origin.z / CHUNK_M))
		if not cells.has(cell):
			cells[cell] = {"chunk": Chunk.new(), "entries": {}}
		var entry: Dictionary = cells[cell]
		var chunk: Chunk = entry.chunk
		var box: AABB = transform * appearance.bounds
		chunk.box = box if chunk.appearances.is_empty() else chunk.box.merge(box)
		if not entry.entries.has(_placed[i]):
			entry.entries[_placed[i]] = chunk.appearances.size()
			chunk.appearances.append(_placed[i])
			chunk.transforms.append(PackedFloat32Array())
			chunk.tiers.append(CULLED)
		pack(chunk.transforms[entry.entries[_placed[i]]], transform, _placed[i + 2])
	_placed.clear()
	for entry in cells.values():
		var chunk: Chunk = entry.chunk
		chunk.box = chunk.box.grow(SHADOW_MARGIN_M)
		var breaks := {}
		for k in chunk.appearances.size():
			var appearance: Appearance = _appearances[chunk.appearances[k]]
			appearance.chunks.append(_chunks.size())
			appearance.chunks.append(k)
			for at in appearance.breaks():
				breaks[at] = true
		var sorted := breaks.keys()
		sorted.sort()
		chunk.breaks = PackedFloat32Array(sorted)
		_chunks.append(chunk)

## Replaces every mover with `placements`, `[appearance id, Transform3D]` each;
## they are drawn from the next `prepare`.
func show_movers(placements: Array) -> void:
	_movers = placements

## Replaces what this batch set draws of `appearance` at `tier` by a layer's
## own choice: `view` into the view and `cast` into the cascades, `pack`ed.
## Kept until chosen again; written by the next `prepare`.
func choose(appearance: int, tier: int, view: PackedFloat32Array, cast: PackedFloat32Array) -> void:
	var chosen := _appearances[appearance]
	chosen.chosen_view[tier] = view
	chosen.chosen_cast[tier] = cast
	chosen.chosen_changed = true

## The view of `camera` drawing into a viewport `size` pixels big, under
## these batches' sun (`detail_view.gd`).
func view_of(camera: Camera3D, size: Vector2):
	return DetailView.of(camera, size, _sun, _max_far_m)

## Chooses what `camera`, drawing into a viewport `size` pixels big, draws
## this frame, and writes every batch whose instances changed.
func prepare(camera: Camera3D, size: Vector2) -> void:
	var view = view_of(camera, size)
	var eye: Vector3 = view.eye
	for chunk in _chunks:
		var state := CULLED
		var per_metre := 0.0
		if view.box_meets(chunk.box):
			var distance: float = view.distance_to(chunk.box)
			per_metre = view.pixels_per_metre / maxf(distance, 1e-3)
			state = chunk.breaks.bsearch(per_metre) * 2 + (1 if distance <= view.reach else 0)
		elif view.casts_into_view(chunk.box, chunk.box.size.y):
			state = CAST_ONLY
		if state == chunk.state:
			continue
		chunk.state = state
		for k in chunk.appearances.size():
			var appearance := _appearances[chunk.appearances[k]]
			chunk.tiers[k] = appearance.tier(per_metre) if state >= 0 else CULLED
			appearance.statics_changed = true
	for appearance in _appearances:
		if appearance.statics_changed:
			_gather_statics(appearance)
	# A mover's batches are rewritten in the frame it leaves them, too.
	for appearance in _appearances:
		if appearance.moved:
			for tier in TIERS:
				appearance.moving_view[tier].clear()
				appearance.moving_cast[tier].clear()
	var moving := {}
	for placement in _movers:
		moving[placement[0]] = true
		var appearance := _appearances[placement[0]]
		var transform: Transform3D = placement[1]
		var center := transform * appearance.center
		if not view.sphere_meets(center, appearance.radius + SHADOW_MARGIN_M):
			continue
		var tier := appearance.tier(view.pixels_per_metre / maxf(eye.distance_to(center), 1e-3))
		pack(appearance.moving_view[tier], transform)
		pack(appearance.moving_cast[mini(tier + CASTER_COARSER, COARSEST)], transform)
	for id in _appearances.size():
		var appearance := _appearances[id]
		if appearance.statics_changed or appearance.moved or appearance.chosen_changed or moving.has(id):
			_write(appearance)
			appearance.statics_changed = false
			appearance.chosen_changed = false
			appearance.moved = moving.has(id)

## Each tier's static instances in chunk order, for the view and the cascades.
func _gather_statics(appearance: Appearance) -> void:
	for tier in TIERS:
		appearance.static_view[tier].clear()
		appearance.static_cast[tier].clear()
	for i in range(0, appearance.chunks.size(), 2):
		var chunk := _chunks[appearance.chunks[i]]
		var k := appearance.chunks[i + 1]
		if chunk.state == CAST_ONLY:
			appearance.static_cast[COARSEST].append_array(chunk.transforms[k])
		elif chunk.state >= 0:
			var tier := chunk.tiers[k]
			appearance.static_view[tier].append_array(chunk.transforms[k])
			if chunk.state % 2 == 1:
				appearance.static_cast[mini(tier + CASTER_COARSER, COARSEST)].append_array(chunk.transforms[k])

func _write(appearance: Appearance) -> void:
	for tier in TIERS:
		if appearance.meshes[tier] == null:
			continue
		appearance.view[tier] = _fill(appearance.view[tier], appearance.meshes[tier], GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, appearance.static_view[tier] + appearance.moving_view[tier] + appearance.chosen_view[tier])
		appearance.cast[tier] = _fill(appearance.cast[tier], appearance.meshes[tier], GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY, appearance.static_cast[tier] + appearance.moving_cast[tier] + appearance.chosen_cast[tier])

## `batch` (made on first use) drawing `buffer`'s instances of `mesh`.
func _fill(batch: MultiMeshInstance3D, mesh: ArrayMesh, shadows: GeometryInstance3D.ShadowCastingSetting, buffer: PackedFloat32Array) -> MultiMeshInstance3D:
	if batch == null:
		if buffer.is_empty():
			return null
		batch = MultiMeshInstance3D.new()
		batch.cast_shadow = shadows
		batch.multimesh = MultiMesh.new()
		batch.multimesh.transform_format = MultiMesh.TRANSFORM_3D
		batch.multimesh.use_custom_data = true
		batch.multimesh.mesh = mesh
		add_child(batch)
	batch.multimesh.instance_count = buffer.size() / INSTANCE_FLOATS
	if not buffer.is_empty():
		batch.multimesh.buffer = buffer
	batch.visible = not buffer.is_empty()
	return batch

## One mesh of `parts` (`{mesh, transform}`), a surface per material, each
## part's transform baked into its vertices, its material the model surface
## (`model_surface.gd`); null when there are no parts.
static func _merged(parts: Array, scenery: bool) -> ArrayMesh:
	var tools := {}
	for part in parts:
		var mesh: Mesh = part.mesh
		for surface in mesh.get_surface_count():
			var material := mesh.surface_get_material(surface)
			if not tools.has(material):
				tools[material] = SurfaceTool.new()
			tools[material].append_from(mesh, surface, part.transform)
	if tools.is_empty():
		return null
	var out := ArrayMesh.new()
	for material in tools:
		var arrays: Array = tools[material].commit_to_arrays()
		arrays[Mesh.ARRAY_BONES] = null
		arrays[Mesh.ARRAY_WEIGHTS] = null
		out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		out.surface_set_material(out.get_surface_count() - 1, ModelSurface.of(material, scenery))
	return out

## The tier a mesh's name gives it (`_LOD<n>`), or -1 for every tier.
static func tier_of(mesh_name: String) -> int:
	var at := mesh_name.rfind("_LOD")
	if at < 0 or not mesh_name.substr(at + 4).is_valid_int():
		return -1
	return int(mesh_name.substr(at + 4))

## Every mesh below a node, with its transform from the node `parent` maps
## from. A skinned mesh is the units layer's to pose (`units/unit_layer.gd`):
## here it is an error.
static func _collect(node: Node, parent: Transform3D, parts: Array) -> void:
	var transform: Transform3D = parent * node.transform if node is Node3D else parent
	if node is MeshInstance3D and node.mesh != null:
		if node.skin != null:
			push_error("%s is skinned; only the units layer draws skinned meshes" % node.name)
		else:
			parts.append({"mesh": node.mesh, "transform": transform, "tier": tier_of(node.name)})
	for child in node.get_children():
		_collect(child, transform, parts)

## Appends an instance at `t` tinted `tint`: `MultiMesh.TRANSFORM_3D`'s
## row-major 3×4 layout, then the tint as custom data.
static func pack(out: PackedFloat32Array, t: Transform3D, tint := Color.WHITE) -> void:
	out.append_array([t.basis.x.x, t.basis.y.x, t.basis.z.x, t.origin.x, t.basis.x.y, t.basis.y.y, t.basis.z.y, t.origin.y, t.basis.x.z, t.basis.y.z, t.basis.z.z, t.origin.z, tint.r, tint.g, tint.b, tint.a])
