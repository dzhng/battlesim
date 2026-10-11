extends "res://tests/check.gd"

## The model batches' contract (`world/model_batches.gd`): a model draws the
## tier its projected size calls for, with the meshes of that tier only; the
## cascades draw it one tier coarser; what lies out of view draws nothing
## unless its shadow lands in view; and batches follow appearances and tiers,
## not placements.

const ModelBatches = preload("res://world/model_batches.gd")

## The browser's model tiers (`presentation.models.lod_px`).
const DETAIL := {"lod_px": [150.0, 60.0, 24.0], "sized": true}
const VIEWPORT := Vector2(1280, 720)

func run() -> bool:
	var world := Node3D.new()
	get_root().add_child(world)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.fov = rad_to_deg(0.8)
	# At 2 m, looking down -z.
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3(0, 2, -10))

	# The sun low behind the camera: a metre of height throws 5 m of shadow
	# toward -z, into the view.
	var sun := Vector3(0, 0.2, 1).normalized()
	var batches := ModelBatches.new("Batches", sun, 2000.0)
	world.add_child(batches)
	var appearance := batches.add_appearance(_model(), DETAIL)

	# Its largest extent is the 3 m base, so a metre at distance d covers
	# 851 / d px and the model 3 × that: tier 0 within 17 m, 1 to 42 m, 2 to
	# 106 m, then 3.
	var distances := [8.0, 30.0, 70.0, 300.0]
	var movers := distances.map(func(d): return [appearance, Transform3D(Basis(), Vector3(0, 0, -d))])
	movers.append([appearance, Transform3D(Basis(), Vector3(0, 0, 50))])
	batches.show_movers(movers)
	batches.prepare(camera, VIEWPORT)
	for tier in 4:
		var drawn := _drawn(batches, tier, false)
		check(drawn.count == 1, "tier %d draws the one model at %d m: %d" % [tier, distances[tier], drawn.count])
		# Each tier's body is 0.25 m shorter, on the unsuffixed base.
		var height := 1.25 + (2.0 - 0.25 * tier) / 2.0
		check(drawn.mesh != null and is_equal_approx(drawn.mesh.get_aabb().size.y, height) and is_equal_approx(drawn.mesh.get_aabb().size.x, 3.0), "tier %d draws its own body and the unsuffixed base: %s" % [tier, drawn.mesh.get_aabb() if drawn.mesh != null else null])
	var casts := range(4).map(func(tier): return _drawn(batches, tier, true).count)
	check(casts == [0, 1, 1, 2], "the cascades draw each model a tier coarser, the coarsest as itself: %s" % [casts])
	check(_all(batches).all(func(batch): return batch.cast_shadow == (GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY if batch.get_meta("cast") else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF)), "a view batch casts nothing; a caster batch draws only into the cascades")
	check(_total(batches) == 4 + 4, "the model behind the camera draws nothing: %d instances drawn" % _total(batches))

	# Statics: a thousand in one chunk ahead, one behind whose shadow falls
	# into view, one far behind whose shadow does not.
	var statics := ModelBatches.new("Statics", sun, 2000.0)
	world.add_child(statics)
	var placed := statics.add_appearance(_model(), DETAIL)
	for i in 1000:
		statics.add_static(placed, Transform3D(Basis(), Vector3(1.0 + (i % 40) * 1.5, 0, -14.0 - (i / 40) * 2.0)))
	statics.add_static(placed, Transform3D(Basis(), Vector3(0, 0, 20)))
	statics.add_static(placed, Transform3D(Basis(), Vector3(0, 0, 400)))
	statics.seal()
	statics.prepare(camera, VIEWPORT)
	check(_drawn(statics, 0, false).count == 1000, "the chunk ahead draws all its models at the tier of its nearest point: %d" % _drawn(statics, 0, false).count)
	check(_drawn(statics, 1, true).count == 1000, "and casts them a tier coarser: %d" % _drawn(statics, 1, true).count)
	check(_drawn(statics, 3, true).count == 1 and _drawn(statics, 3, false).count == 0, "a model out of view whose shadow lands in view only casts, at the coarsest tier")
	check(_total(statics) == 2001, "a model out of view whose shadow stays out draws nothing: %d drawn" % _total(statics))
	check(_all(statics).filter(func(batch): return batch.visible).size() == 3, "a thousand placements draw through three batches")

	# Turning round: the thousand leave the view, and the one behind is ahead.
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3(0, 2, 10))
	statics.prepare(camera, VIEWPORT)
	check(_drawn(statics, 0, false).count == 1, "a chunk that leaves the view stops drawing: %d drawn at tier 0" % _drawn(statics, 0, false).count)

	# A shell whose tiers finer than 2 are open (a kit's `OPEN_TIERS`): an open
	# tier draws over every coarser tier; tiers 2 and 3 draw alone.
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3(0, 2, -10))
	var shells := ModelBatches.new("Shells", sun, 2000.0)
	world.add_child(shells)
	var shell := shells.add_appearance(_model(), DETAIL.merged({"closed_tier": 2}))
	shells.show_movers(distances.map(func(d): return [shell, Transform3D(Basis(), Vector3(0, 0, -d))]))
	shells.prepare(camera, VIEWPORT)
	var drawn := _all(shells).filter(func(batch): return not batch.get_meta("cast") and batch.visible).map(func(batch): return _bodies(batch.multimesh.mesh))
	drawn.sort()
	check(drawn == [[0, 1, 2, 3], [1, 2, 3], [2], [3]], "a shell draws an open tier over every coarser tier, and tiers 2 and 3 alone: %s" % [drawn])
	world.free()
	return true

## The tiers whose bodies a mesh of `_model` holds, finest first, by their
## heights (tier t's body is 2 − 0.25 t tall, centred on the origin).
static func _bodies(mesh: ArrayMesh) -> Array:
	if mesh == null:
		return []
	var tops := {}
	for surface in mesh.get_surface_count():
		for vertex: Vector3 in mesh.surface_get_arrays(surface)[Mesh.ARRAY_VERTEX]:
			if vertex.y > 0.0:
				tops[snappedf(vertex.y, 0.001)] = true
	var tiers := []
	for tier in 4:
		if tops.has(snappedf((2.0 - 0.25 * tier) / 2.0, 0.001)):
			tiers.append(tier)
	return tiers

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
	for batch in _all(batches):
		if batch.get_meta("cast") == cast and batch.visible and _tier_of(batch) == tier:
			return {"count": batch.multimesh.instance_count, "mesh": batch.multimesh.mesh}
	return {"count": 0, "mesh": null}

## A batch's tier, by its mesh's height: tier t's is 2.25 − 0.125 t (`_model`).
static func _tier_of(batch: MultiMeshInstance3D) -> int:
	return roundi((2.25 - batch.multimesh.mesh.get_aabb().size.y) / 0.125)

static func _total(batches: Node) -> int:
	var total := 0
	for batch in _all(batches):
		if batch.visible:
			total += batch.multimesh.instance_count
	return total
