extends Node3D

## Draws placed models the way the browser's models layer does
## (`packages/battle-renderer/src/models/modelLayer.ts`): instanced batches,
## one MultiMesh per appearance and detail tier, whose mesh holds one surface
## per material with every part's own transform baked in. Draw calls follow
## appearances × tiers × materials × passes, not placed objects or their parts.
##
## - **Appearances.** Any node tree of meshes: a catalog scene, a kit's
##   module, a stump. A mesh names its tier with a `_LOD<n>` suffix, finest
##   first, and an unsuffixed mesh is in every tier (`packages/scene-assets`'
##   rule). Or meshes a layer built per tier (`add_meshes`).
## - **Chosen.** Each layer ports its browser owner's rule for what is drawn
##   at which tier (buildings, scenery, props, units), and hands each
##   appearance and tier the instances it draws and those it casts.
## - **Shadows.** A view batch casts nothing; a caster batch draws only into
##   the cascades, at whatever tier the layer chose for them.
## - **Surfaces.** Every material draws as the browser's model surface
##   (`model_surface.gd`), and each instance carries its tint.
## - **Writes.** A batch's buffer is written only when its layer chose anew.

const DetailView = preload("res://world/detail_view.gd")
const ModelSurface = preload("res://world/model_surface.gd")

const TIERS := 4
## Floats an instance takes in a batch: `MultiMesh.TRANSFORM_3D`'s 3×4 rows,
## then its custom data, the tint (`pack`).
const INSTANCE_FLOATS := 16

## One appearance: its tiers' merged meshes, its bounds and its batches.
class Appearance:
	## Per tier, or null where the tier has no geometry.
	var meshes: Array[ArrayMesh] = []
	## Its finest tier's bounds, in its own space.
	var bounds: AABB
	## Per tier: what the view draws, and per caster region what the
	## cascades draw.
	var view: Array[MultiMeshInstance3D] = []
	var cast: Array[Dictionary] = []
	## Per tier: the instances chosen (`INSTANCE_FLOATS` each), for the view
	## and, by region, for the cascades; and whether they changed since the
	## last write.
	var chosen_view: Array[PackedFloat32Array] = []
	var chosen_cast: Array[Dictionary] = []
	var changed := false

	func _init(tier_meshes: Array) -> void:
		for tier in TIERS:
			meshes.append(tier_meshes[tier])
			chosen_view.append(PackedFloat32Array())
			chosen_cast.append({})
			view.append(null)
			cast.append({})

var _appearances: Array[Appearance] = []
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
## appearance; returns its id. A `scenery` appearance takes its instances'
## tint on every surface (`model_surface.gd`).
func add_appearance(root: Node, scenery := false) -> int:
	var parts := []
	for child in root.get_children():
		_collect(child, Transform3D.IDENTITY, parts)
	var named: bool = parts.any(func(part): return part.tier >= 0)
	var meshes := []
	for tier in TIERS:
		meshes.append(_merged(parts.filter(func(part): return not named or part.tier < 0 or part.tier == tier), scenery))
	var appearance := Appearance.new(meshes)
	var finest = meshes.filter(func(mesh): return mesh != null).front() if meshes.any(func(mesh): return mesh != null) else null
	appearance.bounds = finest.get_aabb() if finest != null else AABB()
	_appearances.append(appearance)
	return _appearances.size() - 1

## Registers prebuilt meshes, one per tier (null where a tier has none), as
## an appearance; every pose of it stays within `box`, which culls its
## batches. Returns its id.
func add_meshes(meshes: Array[ArrayMesh], box: AABB) -> int:
	for mesh in meshes:
		if mesh != null:
			mesh.custom_aabb = box
	var appearance := Appearance.new(meshes)
	appearance.bounds = box
	_appearances.append(appearance)
	return _appearances.size() - 1

## The finest tier's bounds of `appearance`, in its own space.
func bounds(appearance: int) -> AABB:
	return _appearances[appearance].bounds

## Tier `tier`'s merged mesh of `appearance`, or null where it has none.
func mesh(appearance: int, tier: int) -> ArrayMesh:
	return _appearances[appearance].meshes[tier]

## Replaces what `appearance` draws at `tier`: `view` into the view and
## `cast` into the cascades, `pack`ed. Kept until chosen again; written by
## the next `write`. A `cast` given by region (region -> instances) is a
## batch per region, which each cascade culls by its bounds: a cascade draws
## only the regions it covers.
func choose(appearance: int, tier: int, view: PackedFloat32Array, cast: Variant) -> void:
	var chosen := _appearances[appearance]
	chosen.chosen_view[tier] = view
	chosen.chosen_cast[tier] = cast if cast is Dictionary else {0: cast}
	chosen.changed = true

## The view of `camera` drawing into a viewport `size` pixels big, under
## these batches' sun (`detail_view.gd`).
func view_of(camera: Camera3D, size: Vector2):
	return DetailView.of(camera, size, _sun, _max_far_m)

## Writes every batch whose instances were chosen anew.
func write() -> void:
	for appearance in _appearances:
		if not appearance.changed:
			continue
		appearance.changed = false
		for tier in TIERS:
			if appearance.meshes[tier] == null:
				continue
			appearance.view[tier] = _fill(appearance.view[tier], appearance.meshes[tier], GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, appearance.chosen_view[tier])
			var casts: Dictionary = appearance.cast[tier]
			var chosen: Dictionary = appearance.chosen_cast[tier]
			for region in casts.keys() + chosen.keys():
				casts[region] = _fill(casts.get(region), appearance.meshes[tier], GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY, chosen.get(region, PackedFloat32Array()))

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
