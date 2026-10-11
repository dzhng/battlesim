extends "res://tests/check.gd"

## The model batches' contract (`world/model_batches.gd`): an appearance's
## tiers are its `_LOD<n>` meshes over its unsuffixed ones, and its batches
## draw exactly what its layer chose for each tier, into the view and the
## cascades apart.

const ModelBatches = preload("res://world/model_batches.gd")

func run() -> bool:
	var world := Node3D.new()
	get_root().add_child(world)
	var batches := ModelBatches.new("Batches", Vector3.UP, 2000.0)
	world.add_child(batches)
	var appearance := batches.add_appearance(_model())

	# Each tier's mesh is that tier's body on the unsuffixed base; its bounds
	# are the finest tier's.
	for tier in 4:
		var height := 1.25 + (2.0 - 0.25 * tier) / 2.0
		var mesh := batches.mesh(appearance, tier)
		check(mesh != null and is_equal_approx(mesh.get_aabb().size.y, height) and is_equal_approx(mesh.get_aabb().size.x, 3.0), "tier %d is its own body and the unsuffixed base: %s" % [tier, mesh.get_aabb() if mesh != null else null])
	check(is_equal_approx(batches.bounds(appearance).size.y, 2.25), "an appearance's bounds are its finest tier's")

	# A layer's choice: drawn at the tier it names, cast where it says, and
	# replaced by the next.
	var view := PackedFloat32Array()
	var cast := PackedFloat32Array()
	for x in 3:
		ModelBatches.pack(view, Transform3D(Basis(), Vector3(x, 0, -300)), Color(0.5, 0.25, 1.0))
	ModelBatches.pack(cast, Transform3D(Basis(), Vector3(0, 0, -300)))
	batches.choose(appearance, 1, view, cast)
	batches.write()
	check(_drawn(batches, 1, false).count == 3 and _drawn(batches, 1, true).count == 1, "a choice draws its instances at its tier, into the view and the cascades")
	check(_all(batches).all(func(batch): return batch.cast_shadow == (GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY if batch.get_meta("cast") else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF)), "a view batch casts nothing; a caster batch draws only into the cascades")
	batches.choose(appearance, 1, PackedFloat32Array(), PackedFloat32Array())
	batches.write()
	check(_total(batches) == 0, "a choice replaces the last: %d drawn" % _total(batches))
	world.free()
	return true

## A model whose body has one mesh per tier, each 0.25 m shorter than the
## last, standing on an unsuffixed base in every tier.
static func _model() -> Node3D:
	var root := Node3D.new()
	for tier in 4:
		root.add_child(_part("body_LOD%d" % tier, Vector3(2, 2.0 - 0.25 * tier, 2), Vector3.ZERO))
	root.add_child(_part("base", Vector3(3, 0.1, 3), Vector3(0, -1.2, 0)))
	return root

static func _part(part_name: String, size: Vector3, at: Vector3) -> MeshInstance3D:
	var part := MeshInstance3D.new()
	part.name = part_name
	part.mesh = BoxMesh.new()
	part.mesh.size = size
	part.position = at
	return part

## Every batch below `batches`, each tagged with whether it casts.
static func _all(batches: Node) -> Array:
	var all := []
	for batch in batches.get_children():
		batch.set_meta("cast", batch.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY)
		all.append(batch)
	return all

## The instances the view (or the cascades) draw at `tier`, and that tier's mesh.
static func _drawn(batches: Node, tier: int, cast: bool) -> Dictionary:
	var batch := _batch(batches, tier, cast)
	return {"count": batch.multimesh.instance_count, "mesh": batch.multimesh.mesh} if batch != null else {"count": 0, "mesh": null}

## The visible batch the view (or the cascades) draw `tier` through, or null.
static func _batch(batches: Node, tier: int, cast: bool) -> MultiMeshInstance3D:
	for batch in _all(batches):
		if batch.get_meta("cast") == cast and batch.visible and _tier_of(batch) == tier:
			return batch
	return null

## A batch's tier, by its mesh's height: tier t's is 2.25 − 0.125 t (`_model`).
static func _tier_of(batch: MultiMeshInstance3D) -> int:
	return roundi((2.25 - batch.multimesh.mesh.get_aabb().size.y) / 0.125)

static func _cast_total(batches: Node) -> int:
	var total := 0
	for batch in _all(batches):
		if batch.visible and batch.get_meta("cast"):
			total += batch.multimesh.instance_count
	return total

static func _total(batches: Node) -> int:
	var total := 0
	for batch in _all(batches):
		if batch.visible:
			total += batch.multimesh.instance_count
	return total
