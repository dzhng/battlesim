extends RefCounted

## The forest trees the side saw fall, as the browser's scenery draws them
## (`scenery/felled.ts`): each falls from the top of its stump on the
## presentation clock, from `(tick − 1) / tickHz`, kicks off the stump as it
## lands and lies beside it, its crown pressed under `rest_height_m` and
## narrowed to `rest_spread`; its stump stands where it grew.
##
## The browser poses a felled tree in its vertex stage; native bakes the same
## pose (the Rust extension's port of `felledPoint`) into one mesh per tree and
## tier, again only when the pose or tier changes. Normals turn with the fall
## but are not pressed.

const ModelBatches = preload("res://world/model_batches.gd")
const ModelSurface = preload("res://world/model_surface.gd")
const Coords = preload("res://core/coords.gd")

const TIERS := 4
const COARSEST := TIERS - 1
## The material whose triangles are foliage (`scenery/appearance.ts`).
const FOLIAGE := "leaves"
## Facets round a stump, and how close to its base a trunk's foot ring lies.
const STUMP_SIDES := 10
const FOOT_M := 0.05

var _rules: Dictionary
var _tick_hz: float
var _batches: ModelBatches
## Per kind: its appearance, its trunk's radius at the hinge and its crown's
## base (unscaled; zero for a kind the forest never stands), and its stump's
## appearance (-1 for none).
var _appearances: PackedInt32Array
var _trunks := PackedFloat32Array()
var _crowns := PackedFloat32Array()
var stumps := PackedInt32Array()
## The felled trees found in the forest: each its kind, height, reach, fall,
## packed standing instance, and its view and cast nodes with the tier each
## draws.
var _trees: Array[Dictionary] = []
var _root: Node3D
## `[kind, tier]` -> its tier's mesh drawn felled.
var _posed := {}

func _init(rules: Dictionary, tick_hz: float) -> void:
	_rules = rules
	_tick_hz = tick_hz

## Readies each forest kind (`forest_kinds`, kind -> true) from its finest
## tier in `batches` (`appearances` by kind); felled trees draw below `root`.
## `barks` gives each kind's own bark material, as imported.
func build(batches: ModelBatches, appearances: PackedInt32Array, root: Node3D, forest_kinds: Dictionary, barks: Array) -> void:
	_batches = batches
	_appearances = appearances
	_root = root
	for kind in appearances.size():
		_trunks.append(0.0)
		_crowns.append(0.0)
		stumps.append(-1)
		if not forest_kinds.has(kind) or appearances[kind] < 0:
			continue
		var mesh := batches.mesh(appearances[kind], 0)
		var trunk := trunk_at(mesh, float(_rules.stump_height_m))
		_trunks[kind] = trunk.radius
		_crowns[kind] = crown_base(mesh)
		var holder := Node3D.new()
		var part := MeshInstance3D.new()
		part.mesh = stump_mesh(trunk, float(_rules.stump_height_m), barks[kind], _rules.cut)
		holder.add_child(part)
		stumps[kind] = batches.add_appearance(holder, true)
		holder.free()

## The felled trees of `fallen_bodies` (`fallenBodies`) the forest stands:
## `tree_of` gives a trunk id's tree (`{kind, height, reach, pack}`), or
## nothing for a body the forest does not stand. A tree already shown keeps
## its nodes.
func show(fallen_bodies: Array, tree_of: Callable) -> void:
	var before := {}
	for tree in _trees:
		before[tree.prop] = tree
	var trees: Array[Dictionary] = []
	for body in fallen_bodies:
		var prop := int(body.prop)
		var tree: Dictionary = before.get(prop, {})
		if tree.is_empty():
			var found: Dictionary = tree_of.call(prop)
			if found.is_empty():
				continue
			tree = found.merged({"prop": prop, "nodes": [null, null], "tiers": [-1, -1]})
		before.erase(prop)
		tree.merge({"toward": Vector2(float(body.toward[0]), float(body.toward[1])), "fell_at": (float(body.tick) - 1.0) / _tick_hz}, true)
		trees.append(tree)
	for tree in before.values():
		for node in tree.nodes:
			if node != null:
				node.queue_free()
	_trees = trees

## Poses the felled trees at presentation second `clock` for `view`, each at
## the tier its standing height takes from the eye under `lod_px` (cast a
## tier coarser), and counts them (`felled`, by kind and tier) and their
## stumps (`standing`, by kind). The browser culls neither. Returns each
## kind's stumps (`kind -> instances`), for every kind that has one.
func prepare(view, clock: float, lod_px: Array, felled: PackedInt32Array, standing: PackedInt32Array) -> Dictionary:
	felled.fill(0)
	standing.fill(0)
	var stump_view := {}
	for kind in stumps.size():
		if stumps[kind] >= 0:
			stump_view[kind] = PackedFloat32Array()
	for tree in _trees:
		var kind: int = tree.kind
		var pack: PackedFloat32Array = tree.pack
		var foot := Vector3(pack[3], pack[7], pack[11])
		var px: float = tree.height * view.pixels_per_metre / maxf(view.eye.distance_to(foot), 1e-3)
		var tier := 0 if px > float(lod_px[0]) else 1 if px > float(lod_px[1]) else 2 if px > float(lod_px[2]) else COARSEST
		felled[kind * TIERS + tier] += 1
		standing[kind] += 1
		stump_view[kind].append_array(_stump_pack(pack))
		var pose := fell_pose(_rules, tree.toward, clock - tree.fell_at, tree.height, tree.reach, _trunks[kind] * _scale_xy(pack), _crowns[kind] * _scale_z(pack))
		for cast in 2:
			if tree.nodes[cast] == null:
				tree.nodes[cast] = _node(cast == 1, pack, tree.height + tree.reach)
			var node: MultiMeshInstance3D = tree.nodes[cast]
			var at_tier := mini(tier + cast, COARSEST)
			if tree.tiers[cast] != at_tier:
				tree.tiers[cast] = at_tier
				node.multimesh.mesh = _posed_mesh(kind, at_tier)
				node.visible = node.multimesh.mesh != null
			node.set_instance_shader_parameter("fall_a", Vector4(pose[0], pose[1], pose[2], pose[3]))
			node.set_instance_shader_parameter("fall_b", Vector4(pose[4], pose[5], pose[6], pose[7]))
			node.set_instance_shader_parameter("fall_crown", pose[8])
	return stump_view

## A tree's node, standing as packed in `pack`; it lies within `reach` of its foot.
func _node(cast: bool, pack: PackedFloat32Array, reach: float) -> MultiMeshInstance3D:
	var node := MultiMeshInstance3D.new()
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY if cast else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	node.multimesh = MultiMesh.new()
	node.multimesh.transform_format = MultiMesh.TRANSFORM_3D
	node.multimesh.use_custom_data = true
	node.multimesh.instance_count = 1
	node.multimesh.buffer = pack
	var foot := Vector3(pack[3], pack[7], pack[11])
	node.custom_aabb = AABB(foot - Vector3.ONE * reach, Vector3.ONE * 2.0 * reach)
	_root.add_child(node)
	return node

## Tier `tier` of `kind` drawn felled, or null where it has none.
func _posed_mesh(kind: int, tier: int) -> ArrayMesh:
	var key := [kind, tier]
	if not _posed.has(key):
		var mesh := _batches.mesh(_appearances[kind], tier)
		_posed[key] = mesh.duplicate() if mesh != null else null
		for surface in mesh.get_surface_count() if mesh != null else 0:
			var material: ShaderMaterial = mesh.surface_get_material(surface)
			_posed[key].surface_set_material(surface, ModelSurface.felled(material, material.resource_name == FOLIAGE))
	return _posed[key]

## A stump of the tree packed in `pack`: as it stood, girth and all, a metre
## tall (its mesh is the stump's own height).
static func _stump_pack(pack: PackedFloat32Array) -> PackedFloat32Array:
	var x := Vector3(pack[0], pack[4], pack[8])
	var z := Vector3(pack[2], pack[6], pack[10])
	var basis := Basis(x, Vector3.UP, z)
	var out := PackedFloat32Array()
	ModelBatches.pack(out, Transform3D(basis, Vector3(pack[3], pack[7], pack[11])), Color(pack[12], pack[13], pack[14]))
	return out

## The tree's scale across (sim x) and up (sim z), from its packed basis.
static func _scale_xy(pack: PackedFloat32Array) -> float:
	return Vector3(pack[0], pack[4], pack[8]).length()

static func _scale_z(pack: PackedFloat32Array) -> float:
	return Vector3(pack[1], pack[5], pack[9]).length()

# The fall (`felled.ts`) ------------------------------------------------------

## The angle a tree lies at: past level by as much as lays a trunk `length`
## long, its foot a `trunk` radius up, its top on the ground.
static func rest_angle(trunk: float, length: float) -> float:
	return PI / 2.0 + asin(minf(1.0, trunk / maxf(length, 1e-3)))

## How far the tree has tipped `age` seconds after it began to fall.
static func fall_angle(rules: Dictionary, age: float, rest: float) -> float:
	if age <= 0.0:
		return 0.0
	if age < float(rules.fall_s):
		return rest * pow(age / float(rules.fall_s), 2.0)
	var u := (age - float(rules.fall_s)) / float(rules.settle_s) if float(rules.settle_s) > 0.0 else 1.0
	if u >= 1.0:
		return rest
	return rest - deg_to_rad(float(rules.settle_deg)) * sin(PI * u) * (1.0 - u)

## How far a tree tipped `angle` of its `rest` has landed.
static func lying_share(angle: float, rest: float) -> float:
	var u := clampf((angle / rest - 0.5) / 0.5, 0.0, 1.0)
	return u * u * (3.0 - 2.0 * u)

## A felled tree's pose `age` seconds after it began to fall (`fellPose`):
## `[toward x, toward y, angle, hinge, trunk, squash, spread, landed, crown]`.
static func fell_pose(rules: Dictionary, toward: Vector2, age: float, height: float, reach: float, trunk: float, crown: float) -> Array:
	var hinge := float(rules.stump_height_m)
	var length := height - hinge
	var rest := rest_angle(trunk, length)
	var angle := fall_angle(rules, age, rest)
	var landed := lying_share(angle, rest)
	var axis := trunk * (1.0 - clampf((crown - hinge) / length, 0.0, 1.0))
	var squash := clampf((float(rules.rest_height_m) - axis) / maxf(reach, 1e-3), 0.02, 1.0)
	return [toward.x, toward.y, angle, hinge, trunk, 1.0 + (squash - 1.0) * landed, 1.0 + (float(rules.rest_spread) - 1.0) * landed, landed, crown]

# A kind's trunk and crown, from its finest tier --------------------------

## Where a kind's leaves begin, unscaled (`crownBase`): its lowest leaf, or
## the top of a tree with none.
static func crown_base(mesh: ArrayMesh) -> float:
	var low := INF
	var top := 0.0
	for surface in mesh.get_surface_count():
		var material := mesh.surface_get_material(surface)
		var foliage := material != null and material.resource_name == FOLIAGE
		for vertex: Vector3 in mesh.surface_get_arrays(surface)[Mesh.ARRAY_VERTEX]:
			top = maxf(top, vertex.y)
			if foliage:
				low = minf(low, vertex.y)
	return minf(low, top)

## A kind's trunk at `height` above its foot, unscaled (`trunkAt`): the foot
## ring's and the next bark ring's mean radii, interpolated, and the bark's
## mean vertex colour.
static func trunk_at(mesh: ArrayMesh, height: float) -> Dictionary:
	var bark := []
	for surface in mesh.get_surface_count():
		var material := mesh.surface_get_material(surface)
		if material != null and material.resource_name == FOLIAGE:
			continue
		var arrays := mesh.surface_get_arrays(surface)
		var colours = arrays[Mesh.ARRAY_COLOR]
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		for v in vertices.size():
			bark.append([vertices[v], colours[v] if colours != null else Color.WHITE])
	var next := INF
	for entry in bark:
		if entry[0].y > FOOT_M:
			next = minf(next, entry[0].y)
	var rings := [[0.0, 0], [0.0, 0]]
	var colour := Color(0, 0, 0)
	for entry in bark:
		var y: float = entry[0].y
		var ring := 0 if y <= FOOT_M else 1 if absf(y - next) <= FOOT_M else -1
		if ring < 0:
			continue
		rings[ring][0] += Vector2(entry[0].x, entry[0].z).length()
		rings[ring][1] += 1
		colour += entry[1]
	if rings[0][1] == 0 or rings[1][1] == 0:
		push_error("scenery: a tree appearance has no bark ring at its foot")
		return {"radius": 0.0, "bark": Color.WHITE}
	var r0: float = rings[0][0] / rings[0][1]
	var r1: float = rings[1][0] / rings[1][1]
	var n: float = rings[0][1] + rings[1][1]
	return {"radius": r0 + (r1 - r0) * minf(1.0, height / next), "bark": Color(colour.r / n, colour.g / n, colour.b / n)}

## A kind's stump (`stumpMesh`): a short prism as wide as its trunk
## (`trunk_at`), its sides `bark_material` in the bark's colour, its top the
## cut face in `cut` (linear albedo).
static func stump_mesh(trunk: Dictionary, height: float, bark_material: Material, cut: Array) -> ArrayMesh:
	var sides := SurfaceTool.new()
	var face := SurfaceTool.new()
	sides.begin(Mesh.PRIMITIVE_TRIANGLES)
	face.begin(Mesh.PRIMITIVE_TRIANGLES)
	var radius: float = trunk.radius
	var cut_colour := Color(float(cut[0]), float(cut[1]), float(cut[2]))
	for k in STUMP_SIDES:
		var a0 := TAU * k / STUMP_SIDES
		var a1 := TAU * (k + 1) / STUMP_SIDES
		var am := (a0 + a1) / 2.0
		var normal := Coords.sim_to_godot(Vector3(cos(am), sin(am), 0.0))
		var p0 := Vector3(radius * cos(a0), radius * sin(a0), 0.0)
		var p1 := Vector3(radius * cos(a1), radius * sin(a1), 0.0)
		var up := Vector3(0, 0, height)
		# Godot's front faces wind clockwise.
		for p in [p0, p1 + up, p1, p0, p0 + up, p1 + up]:
			sides.set_color(trunk.bark)
			sides.set_normal(normal)
			sides.add_vertex(Coords.sim_to_godot(p))
		for p in [up, p1 + up, p0 + up]:
			face.set_color(cut_colour)
			face.set_normal(Vector3.UP)
			face.add_vertex(Coords.sim_to_godot(p))
	var out := sides.commit()
	out.surface_set_material(0, bark_material)
	face.commit(out)
	return out

## The bark material of a tree appearance's node tree: its first material
## that is not foliage, as imported.
static func bark_of(node: Node) -> Material:
	for part in node.find_children("*", "MeshInstance3D", true, false):
		for surface in part.mesh.get_surface_count():
			var material: Material = part.mesh.surface_get_material(surface)
			if material != null and material.resource_name != FOLIAGE:
				return material
	return null
