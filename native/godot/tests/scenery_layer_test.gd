extends "res://tests/check.gd"

## The scenery layer's contract (`world/scenery_layer.gd`, the port of the
## browser's `sceneryLayer.ts` selection): a chunk too far for its tallest
## tree to leave the coarsest tier draws whole at it, a nearer chunk's trees
## each take their own tier, and a dressing cell in reach draws whole at the
## tier of its tallest piece. What the side knows takes its part: a tree it
## saw fall leaves the forest to fall by its stump, and a shrub or a piece of
## dressing on ground it saw cleared is gone.

const SceneryLayer = preload("res://world/scenery_layer.gd")
const DetailView = preload("res://world/detail_view.gd")
const Records = preload("res://tests/records.gd")

const VIEWPORT := Vector2(1280, 720)
const FIELDS := [["pose", "f32", 4], ["shape", "f32", 4], ["tint", "f32", 4], ["kind", "u32", 1], ["id", "u32", 1], ["height", "f32", 1], ["reach", "f32", 1]]
const NO_BODY := 0xffffffff
const FELLED := {"fall_s": 2.6, "settle_s": 0.6, "settle_deg": 4, "rest_height_m": 1, "rest_spread": 0.6, "stump_height_m": 0.4, "cut": [0.26, 0.19, 0.11]}

## Ground the side saw cleared: everything west of x = 0.
class Cleared:
	var cleared_version := 1
	func cleared_at(x: float, _y: float) -> bool:
		return x < 0.0

## A catalog whose every appearance is a 10 m tree: a bark trunk and a crown
## of leaves, each in tier 0 only.
class TreeCatalog:
	var tree: Node3D
	func _init() -> void:
		tree = Node3D.new()
		var trunk := MeshInstance3D.new()
		trunk.name = "trunk_LOD0"
		trunk.mesh = CylinderMesh.new()
		trunk.mesh.height = 4.0
		trunk.mesh.material = StandardMaterial3D.new()
		trunk.mesh.material.resource_name = "bark"
		trunk.position.y = 2.0
		tree.add_child(trunk)
		var crown := MeshInstance3D.new()
		crown.name = "crown_LOD0"
		crown.mesh = SphereMesh.new()
		crown.mesh.radius = 3.0
		crown.mesh.height = 6.0
		crown.mesh.material = StandardMaterial3D.new()
		crown.mesh.material.resource_name = "leaves"
		crown.position.y = 7.0
		tree.add_child(crown)
	func source(_kind: String, _state: String) -> String:
		return "tree"
	func node(_kind: String, _state: String) -> Node:
		return tree

func run() -> bool:
	var root := Node3D.new()
	get_root().add_child(root)
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.fov = rad_to_deg(0.8)
	# At the origin, 2 m up, looking north (sim +y).
	camera.look_at_from_position(Vector3(0, 2, 0), Vector3(0, 2, -10))
	var view = DetailView.of(camera, VIEWPORT, Vector3(0, 1, 0), 2600.0)

	# Two 10 m trees 30 m ahead (projected 284 px: tier 0) and one 3 km
	# ahead, whose chunk draws whole at tier 3; a shrub each side of x = 0;
	# a dressing cell ahead with a piece each side of x = 0.
	var tables := {
		"forest": Records.table(FIELDS, [_tree(-2, 30, 0, 501), _tree(2, 30, 0, 502), _tree(0, 3000, 0, 503)]),
		"understorey": Records.table(FIELDS, [_tree(-3, 20, 1, NO_BODY, 1.0), _tree(3, 20, 1, NO_BODY, 1.0)]),
		"backdrop": Records.table(FIELDS, []),
		"dressing": Records.table(FIELDS, [_tree(-1, 10, 2, NO_BODY, 0.5), _tree(1, 10, 2, NO_BODY, 0.5)]),
		"dressingCells": Records.table([["cell", "i32", 2], ["first", "u32", 1], ["count", "u32", 1]], [{"cell": [-1, 0], "first": 0, "count": 2}]),
	}
	var tiers := {
		"lodPx": [260.0, 90.0, 26.0], "chunkM": 128.0, "casterCoarser": 1,
		"dressing": {"lodPx": [48.0, 22.0, 8.0], "fadePx": 6.0, "gone": 0.5, "cellM": 64.0, "topM": 0.86, "reachM": 1.0, "ground": [0.0, 0.0], "slots": 128, "cellsPerView": 2},
	}
	var layer := SceneryLayer.new(tables, {"kinds": ["oak", "shrub", "fern"], "tiers": tiers, "felled": FELLED}, Vector3(0, 1, 0), 2600.0, 30.0)
	root.add_child(layer)
	var catalog := TreeCatalog.new()
	layer.build(catalog, preload("res://report/render_report.gd").new())
	layer.set_known([], null)
	layer.prepare(view, 10.0)
	check(_drawn(layer.counts.forest) == {"oak/0": 2, "oak/3": 1}, "near trees take their own tier, a far chunk draws whole at the coarsest: %s" % _drawn(layer.counts.forest))
	check(_drawn(layer.counts.understorey) == {"shrub/2": 2}, "both shrubs stand, 1 m tall 20 m off (42 px, tier 2): %s" % _drawn(layer.counts.understorey))
	# A piece 0.86 m tall a dozen metres off: 61 px, tier 0.
	check(_drawn(layer.counts.dressing) == {"fern/0": 2}, "the dressing cell draws whole at its tallest piece's tier: %s" % _drawn(layer.counts.dressing))

	# The side saw tree 501 fall at tick 31 (a second in), and the ground
	# west of x = 0 cleared.
	layer.set_known([{"prop": 501, "toward": [1.0, 0.0], "tick": 31.0}], Cleared.new())
	layer.prepare(view, 10.0)
	check(_drawn(layer.counts.forest) == {"oak/0": 1, "oak/3": 1}, "the fallen tree leaves the forest: %s" % _drawn(layer.counts.forest))
	check(_drawn(layer.counts.felled) == {"oak/0": 1} and Array(layer.counts.stumps) == [1, 0, 0], "and lies by its stump: %s, stumps %s" % [_drawn(layer.counts.felled), layer.counts.stumps])
	check(_drawn(layer.counts.understorey) == {"shrub/2": 1}, "a shrub on cleared ground is gone: %s" % _drawn(layer.counts.understorey))
	check(_drawn(layer.counts.dressing) == {"fern/0": 1}, "and so is the dressing on it: %s" % _drawn(layer.counts.dressing))
	catalog.tree.free()
	root.free()
	return true

## A tree record at sim `(x, y)`, of `kind`, standing on body `id`, `height` tall.
static func _tree(x: float, y: float, kind: int, id: int, height := 10.0) -> Dictionary:
	return {"pose": [x, y, 0.0, 0.0], "shape": [1.0, 1.0, 1.0, 0.62 * height], "tint": [1.0, 1.0, 1.0, 0.0], "kind": kind, "id": id, "height": height, "reach": 3.0}

## Counts by kind and tier as `{"kind/tier": count}`, zeros left out.
static func _drawn(counts: PackedInt32Array) -> Dictionary:
	var kinds := ["oak", "shrub", "fern"]
	var out := {}
	for k in counts.size():
		if counts[k] > 0:
			out["%s/%d" % [kinds[k / 4], k % 4]] = counts[k]
	return out
