extends Node3D

## The browser's models layer for units (`packages/battle-renderer/src/models/
## modelLayer.ts`): every soldier, vehicle and corpse the browser poses, posed
## as it poses them (`unit_poses.gd`) and drawn through `model_batches.gd`.
##
## - **Soldiers** are skinned. Their palettes are posed on the GPU by the
##   browser's pose kernel (`pose_kernel.compute`) from the manifest's clip
##   tables, and the model surface skins each vertex by them
##   (`model_surface.gdshaderinc`, `POSED_SKINNED`).
## - **Corpses** are a body in one of the ways it lies: the same skinned mesh,
##   at its corpse pose.
## - **Vehicles** are rigid parts. Their articulated nodes are posed on the
##   CPU (`articulation.gd`), each vertex follows its node (`POSED_RIGID`), and
##   a track's texture scrolls with its links.
## - Each model takes the browser's detail rule (`modelDetail`): culled off
##   screen, else the tier its projected size calls for, its shadow cast one
##   tier coarser.
##
## Native's meshes are the catalog's glTF sources. A soldier's vertices stay
## in the glTF's mesh space, and each bind of its skin moves them with the
## bundle joint of the same name. A vehicle's parts move with the bundle node
## they hang from. The bundle's joints and nodes must stand where the glTF's
## do, or the build stops at a blocker. Not yet ported: impostor cards for far
## soldiers, which draw their coarsest tier (reported).

const ModelBatches = preload("res://world/model_batches.gd")
const ModelSurface = preload("res://world/model_surface.gd")
const PoseKernel = preload("res://units/pose_kernel.gd")
const Coords = preload("res://core/coords.gd")
const Staging = preload("res://assets/staging.gd")
const Articulation = preload("res://units/articulation.gd")
const UnitPoses = preload("res://units/unit_poses.gd")

const OWNER := "native/units"
const TIERS := ModelBatches.TIERS
const COARSEST := TIERS - 1
## A posed model's palette holds at most this many matrices: a skin's binds,
## or a vehicle's data slot, hull and articulated nodes.
const MAX_PALETTE := 128
## The palette images: texels per row, and the most matrices each holds.
const PALETTE_WIDTH := PoseKernel.PALETTE_WIDTH
const SOLDIER_MATRICES := 512 * MAX_PALETTE
const VEHICLE_MATRICES := 128 * MAX_PALETTE
## A model's sun shadow reaches this far past it (`SHADOW_MARGIN_M`).
const SHADOW_MARGIN_M := 3.0
## Shadow casters draw this many tiers coarser (`CASTER_COARSER`).
const CASTER_COARSER := 1
## How far a glTF joint or node may stand from the bundle's, metres.
const REST_TOLERANCE_M := 0.02
## A vehicle palette's first slot carries its track scroll, the second is
## its hull's identity; articulated nodes follow. A rigid vertex's `CUSTOM0`
## is its slot plus `TRACK_STRIDE` times its track side (0 none, 1 left, 2 right).
const DATA_SLOT := 0
const HULL_SLOT := 1
const TRACK_STRIDE := 256

## One skinned body: its batch, its bundle record, where its joints, binds
## and clips sit in the kernel's tables, and its detail size standing and
## lying.
class Body:
	var batch: int
	var record: Dictionary
	var joint_base: int
	var binds: int
	var bind_base: int
	## Per bind: its matrix and its joint, as the kernel reads them.
	var bind_matrices: Array[Transform3D] = []
	var bind_joints := PackedInt32Array()
	## Float offset of its clip table in the kernel's samples.
	var sample_base: int
	var size: float
	var radius: float
	## Per way of lying: `{size, radius}`.
	var lying: Array = []

## One vehicle: its batch, bundle record, which palette slot each node moves
## with, and each moving slot's node and rest world inverted (Godot space).
class Vehicle:
	var batch: int
	var record: Dictionary
	var slot_of: PackedInt32Array
	var slot_nodes: PackedInt32Array
	var rest_inverse: Array[Transform3D] = []
	var size: float
	var radius: float

var _poses: UnitPoses
var _units: Dictionary
## Clip table name -> its floats.
var _tables: Dictionary
var _tints: Dictionary
var _batches: ModelBatches
var _catalog: Dictionary
var _bodies := {}
var _vehicles := {}
## The batches the last frame chose instances for, so one no longer drawn is cleared.
var _chosen := {}
## Soldiers' and corpses' palettes (on the GPU), and vehicles' (on the CPU).
var _kernel: PoseKernel
var _vehicle_image: Image
var _vehicle_texture: ImageTexture
## Instances drawn and dropped past the palettes' capacity, for the report.
var _posed := {"soldiers": 0, "vehicles": 0, "corpses": 0}
var _dropped := {"soldiers": 0, "vehicles": 0, "corpses": 0}
var _far_soldiers := 0

## `units` is the manifest's section, `tables` its clip tables (`{name:
## {words}}`), `samples` the capture's decoded observations, `light` the
## manifest's (its sun casts the units' shadows).
func _init(units: Dictionary, tables: Dictionary, samples: Array, side: String, tick_hz: float, light: Dictionary) -> void:
	name = "Units"
	_units = units
	_poses = UnitPoses.new(units, samples, side, tick_hz)
	for table in tables:
		_tables[table] = tables[table].words.to_float32_array()
	_tints = units.tints
	_batches = ModelBatches.new("UnitBatches", Coords.sim_array_to_godot(light.sunDirection), float(light.cascades.max_far_m))
	add_child(_batches)
	var catalog = JSON.parse_string(FileAccess.get_file_as_string(Staging.repo_path("assets/catalog.json")))
	_catalog = catalog.appearances if typeof(catalog) == TYPE_DICTIONARY else {}

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE and _kernel != null:
		_kernel.free_device()

## The repository sources of every appearance the units can draw.
func required_assets() -> Array:
	var sources := {}
	for appearance in _units.appearances:
		var source := _source(appearance)
		if not source.is_empty():
			sources[source] = true
	return sources.keys()

## Builds every appearance's meshes and the kernel from the staged sources.
## A missing source, a body or rig that does not stand where the bundle's
## does, or one too big for a palette is a blocker.
func build(staged: Dictionary, report) -> void:
	var rd := RenderingServer.get_rendering_device()
	if rd == null:
		report.add_blocker("units", "posing soldiers needs a rendering device, which only a display-backed run has")
		return
	_kernel = PoseKernel.new(rd, SOLDIER_MATRICES)
	_vehicle_image = Image.create_empty(PALETTE_WIDTH, VEHICLE_MATRICES * 3 / PALETTE_WIDTH, false, Image.FORMAT_RGBAF)
	_vehicle_texture = ImageTexture.create_from_image(_vehicle_image)
	var samples := PackedFloat32Array()
	var table_base := {}
	for table in _tables:
		table_base[table] = samples.size()
		samples.append_array(_tables[table])
	var parents := PackedInt32Array()
	var binds := PackedFloat32Array()
	for appearance in _units.appearances:
		var record: Dictionary = _units.appearances[appearance]
		var source := _source(appearance)
		if source.is_empty() or not staged.has(source):
			report.add_blocker("units", "appearance %s has no staged source (%s)" % [appearance, source])
			continue
		var root: Node = load(staged[source]).instantiate()
		var facing := Basis(Vector3.UP, deg_to_rad(float(_catalog[appearance].get("basis_yaw_deg", 0.0))))
		var problem: String
		if record.kind == "skinned":
			problem = _build_body(appearance, record, root, facing, parents, binds, int(table_base[record.table]))
		else:
			problem = _build_vehicle(appearance, record, root, facing)
		root.free()
		if not problem.is_empty():
			report.add_blocker("units", "appearance %s: %s" % [appearance, problem])
	if report.ok() and not _kernel.load_tables(samples, parents, binds):
		report.add_blocker("units", "the pose kernel does not compile")

# Building ------------------------------------------------------------------------

## A skinned body: its glTF tiers with each vertex's binds and weights in
## `CUSTOM0` and `CUSTOM1`, its joints' parents and its binds for the kernel,
## and its detail size standing (its far pose) and lying (each corpse pose).
func _build_body(appearance: String, record: Dictionary, root: Node, facing: Basis, parents: PackedInt32Array, binds: PackedFloat32Array, sample_base: int) -> String:
	if record.joints.size() > PoseKernel.MAX_JOINTS:
		return "%d joints, more than the kernel's %d" % [record.joints.size(), PoseKernel.MAX_JOINTS]
	var index := {}
	for j in record.joints.size():
		index[record.joints[j].name] = j
	var rest := Articulation.worlds(record.joints, Articulation.binds(record.joints))
	var parts := []
	_mesh_parts(root, Transform3D.IDENTITY, parts)
	if parts.is_empty() or not parts.all(func(p): return p.skin == parts[0].skin and p.skeleton == parts[0].skeleton):
		return "its source must hold skinned meshes on one skin"
	var skin: Skin = parts[0].skin
	var skeleton: Skeleton3D = parts[0].skeleton
	if skin.get_bind_count() > MAX_PALETTE:
		return "%d skin binds, more than a palette's %d" % [skin.get_bind_count(), MAX_PALETTE]
	# Each glTF bone moves with its bundle joint, else its nearest ancestor's.
	var joint_of := PackedInt32Array()
	for bone in skeleton.get_bone_count():
		var at := bone
		while at >= 0 and not index.has(skeleton.get_bone_name(at)):
			at = skeleton.get_bone_parent(at)
		joint_of.append(index[skeleton.get_bone_name(at)] if at >= 0 else 0)
		if at == bone:
			var godot: Vector3 = facing * (parts[0].skeleton_transform * skeleton.get_bone_global_rest(bone)).origin
			var bundle := Coords.sim_to_godot(rest[joint_of[bone]].origin)
			if godot.distance_to(bundle) > REST_TOLERANCE_M:
				return "joint %s stands at %s in the glTF and %s in the bundle" % [skeleton.get_bone_name(bone), godot, bundle]
	# A bind takes a mesh vertex to the body at rest in Godot's model space,
	# then into its joint's rest frame in the bundle's.
	var body := Body.new()
	body.record = record
	body.sample_base = sample_base
	body.joint_base = parents.size()
	body.bind_base = binds.size() / PoseKernel.BIND_FLOATS
	body.binds = skin.get_bind_count()
	for joint in record.joints:
		parents.append(int(joint.parent))
	var bind_matrices := body.bind_matrices
	var bind_joints := body.bind_joints
	var to_sim := Transform3D(Coords.TO_GODOT.inverse(), Vector3.ZERO)
	for i in skin.get_bind_count():
		var bone := skin.get_bind_bone(i)
		if bone < 0:
			bone = skeleton.find_bone(skin.get_bind_name(i))
		var to_rest: Transform3D = Transform3D(facing, Vector3.ZERO) * parts[0].skeleton_transform * skeleton.get_bone_global_rest(bone) * skin.get_bind_pose(i)
		var joint := joint_of[bone]
		var m: Transform3D = rest[joint].affine_inverse() * to_sim * to_rest
		bind_matrices.append(m)
		bind_joints.append(joint)
		PoseKernel.bind(binds, m, joint)
	var tiers: Array[ArrayMesh] = []
	for tier in TIERS:
		var chosen := parts.filter(func(p): return p.tier < 0 or p.tier == tier)
		if chosen.size() > 1:
			return "tier %d has %d skinned meshes; native draws one a tier" % [tier, chosen.size()]
		var mesh = _skinned_mesh(chosen[0].mesh) if not chosen.is_empty() else null
		if mesh is String:
			return mesh
		tiers.append(mesh)
	# Detail sizes from the coarsest tier posed on the CPU; the browser's
	# from its finest, whose box is the same to a few centimetres.
	var table: PackedFloat32Array = _tables[record.table]
	var coarsest: ArrayMesh = tiers.filter(func(t): return t != null).back()
	var standing := _posed_box(coarsest, palette(record, table, record.farPose, bind_matrices, bind_joints))
	body.size = _size_of(standing)
	body.radius = _radius_of(standing)
	for pose in record.corpsePoses:
		var box := _posed_box(coarsest, palette(record, table, pose, bind_matrices, bind_joints))
		body.lying.append({"size": _size_of(box), "radius": _radius_of(box)})
	body.batch = _batches.add_meshes(tiers, _grown(standing))
	_bodies[appearance] = body
	return ""

## One skinned glTF mesh with its binds and weights as custom data and its
## surfaces posed (`model_surface.gd`), or a problem.
func _skinned_mesh(mesh: Mesh) -> Variant:
	var out := ArrayMesh.new()
	var flags := (Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT) | (Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM1_SHIFT)
	for s in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(s)
		var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES]
		if bones.size() != arrays[Mesh.ARRAY_VERTEX].size() * 4:
			return "a surface weighs more than four bones a vertex"
		arrays[Mesh.ARRAY_CUSTOM0] = PackedFloat32Array(Array(bones))
		arrays[Mesh.ARRAY_CUSTOM1] = arrays[Mesh.ARRAY_WEIGHTS]
		arrays[Mesh.ARRAY_BONES] = null
		arrays[Mesh.ARRAY_WEIGHTS] = null
		out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, flags)
		out.surface_set_material(s, _posed_surface(mesh.surface_get_material(s), "skinned"))
	return out

## A vehicle: its glTF's tiers with each vertex's palette slot and track side
## in `CUSTOM0`, the slot each bundle node moves with, and its detail size.
func _build_vehicle(appearance: String, record: Dictionary, root: Node, facing: Basis) -> String:
	var rig: Dictionary = record.rig
	var nodes: Array = record.nodes
	var index := {}
	for n in nodes.size():
		index[nodes[n].name] = n
	var moving := {}
	for node in [rig.turret, rig.gun, rig.hmg, rig.hmgGun]:
		if int(node) >= 0:
			moving[int(node)] = true
	for list in [rig.wheels, rig.rotors, rig.deploy]:
		for part in list:
			moving[int(part.node)] = true
	var vehicle := Vehicle.new()
	vehicle.record = record
	vehicle.slot_of.resize(nodes.size())
	var rest := Articulation.worlds(nodes, Articulation.binds(nodes))
	for n in nodes.size():
		var parent := int(nodes[n].parent)
		if moving.has(n):
			vehicle.slot_of[n] = HULL_SLOT + 1 + vehicle.slot_nodes.size()
			vehicle.slot_nodes.append(n)
			vehicle.rest_inverse.append(Coords.sim_transform(rest[n]).affine_inverse())
		else:
			vehicle.slot_of[n] = HULL_SLOT if parent < 0 else vehicle.slot_of[parent]
	if HULL_SLOT + 1 + vehicle.slot_nodes.size() > MAX_PALETTE:
		return "%d articulated nodes, more than a palette holds" % vehicle.slot_nodes.size()
	var tracks := {}
	for track in rig.tracks:
		tracks[int(track.node)] = 1 if track.left else 2
	var parts := []
	var problem := _rigid_parts(root, Transform3D(facing, Vector3.ZERO), -1, index, rest, parts)
	if not problem.is_empty():
		return problem
	var tiers: Array[ArrayMesh] = []
	for tier in TIERS:
		tiers.append(_rigid_mesh(parts.filter(func(p): return p.tier < 0 or p.tier == tier), vehicle.slot_of, tracks))
	var box: AABB = tiers[0].get_aabb() if tiers[0] != null else AABB()
	vehicle.size = _size_of(box)
	vehicle.radius = _radius_of(box)
	vehicle.batch = _batches.add_meshes(tiers, _grown(box))
	_vehicles[appearance] = vehicle
	return ""

## Every skinned mesh below `node`: `{mesh, skin, skeleton,
## skeleton_transform, tier}`, the skeleton's transform from the scene root.
static func _mesh_parts(node: Node, parent: Transform3D, parts: Array) -> void:
	var transform: Transform3D = parent * node.transform if node is Node3D else parent
	if node is MeshInstance3D and node.mesh != null:
		var skeleton := node.get_node_or_null(node.skeleton) as Skeleton3D if node.skin != null else null
		parts.append({"mesh": node.mesh, "skin": node.skin, "skeleton": skeleton, "skeleton_transform": parent, "tier": ModelBatches.tier_of(node.name)})
	for child in node.get_children():
		_mesh_parts(child, transform, parts)

## Every mesh below `node` with its transform from the scene root (in the
## bundle's facing) and the bundle node it belongs to: its nearest ancestor
## the bundle names. A bundle node standing elsewhere in the glTF is a problem.
static func _rigid_parts(node: Node, parent: Transform3D, owner: int, index: Dictionary, rest: Array[Transform3D], parts: Array) -> String:
	var transform: Transform3D = parent * node.transform if node is Node3D else parent
	if index.has(String(node.name)):
		owner = index[String(node.name)]
		var bundle := Coords.sim_to_godot(rest[owner].origin)
		if transform.origin.distance_to(bundle) > REST_TOLERANCE_M:
			return "node %s stands at %s in the glTF and %s in the bundle" % [node.name, transform.origin, bundle]
	if node is MeshInstance3D and node.mesh != null:
		parts.append({"mesh": node.mesh, "transform": transform, "node": owner, "tier": ModelBatches.tier_of(node.name)})
	for child in node.get_children():
		var problem := _rigid_parts(child, transform, owner, index, rest, parts)
		if not problem.is_empty():
			return problem
	return ""

## One mesh of rigid `parts`, a surface per material, every part's transform
## baked in and its palette slot and track side in `CUSTOM0`; null for none.
func _rigid_mesh(parts: Array, slot_of: PackedInt32Array, tracks: Dictionary) -> ArrayMesh:
	var tools := {}
	var custom := Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT
	for part in parts:
		var code: float = (slot_of[part.node] if part.node >= 0 else HULL_SLOT) + TRACK_STRIDE * tracks.get(part.node, 0)
		var mesh: Mesh = part.mesh
		for s in mesh.get_surface_count():
			var arrays := mesh.surface_get_arrays(s)
			var data := PackedFloat32Array()
			data.resize(arrays[Mesh.ARRAY_VERTEX].size() * 4)
			data.fill(code)
			arrays[Mesh.ARRAY_CUSTOM0] = data
			var coded := ArrayMesh.new()
			coded.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, custom)
			var material := mesh.surface_get_material(s)
			if not tools.has(material):
				tools[material] = SurfaceTool.new()
				tools[material].begin(Mesh.PRIMITIVE_TRIANGLES)
				tools[material].set_custom_format(0, SurfaceTool.CUSTOM_RGBA_FLOAT)
			tools[material].append_from(coded, 0, part.transform)
	if tools.is_empty():
		return null
	var out := ArrayMesh.new()
	for material in tools:
		out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, tools[material].commit_to_arrays(), [], {}, custom)
		out.surface_set_material(out.get_surface_count() - 1, _posed_surface(material, "rigid"))
	return out

## `material` as a posed surface reading its mode's palette.
func _posed_surface(material: Material, mode: String) -> ShaderMaterial:
	var surface := ModelSurface.of(material, false, mode)
	surface.set_shader_parameter("palette", _kernel.texture if mode == "skinned" else _vehicle_texture)
	surface.set_shader_parameter("palette_width", PALETTE_WIDTH)
	return surface

## A body's palette at `pose` (`{clip, phase}`), on the CPU, as the kernel
## writes it: per bind, its joint's world now times its bind (Godot space).
static func palette(record: Dictionary, table: PackedFloat32Array, pose: Dictionary, bind_matrices: Array[Transform3D], bind_joints: PackedInt32Array) -> Array[Transform3D]:
	var worlds := Articulation.pose_worlds(record, table, pose.clip, float(pose.phase), null) if record.clips.has(pose.clip) else Articulation.worlds(record.joints, Articulation.binds(record.joints))
	var to_godot := Transform3D(Coords.TO_GODOT, Vector3.ZERO)
	var out: Array[Transform3D] = []
	for i in bind_matrices.size():
		out.append(to_godot * worlds[bind_joints[i]] * bind_matrices[i])
	return out

## The box of a skinned mesh's vertices under `palette`.
static func _posed_box(mesh: ArrayMesh, palette: Array[Transform3D]) -> AABB:
	var box := AABB()
	var first := true
	for s in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(s)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var joints: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
		var weights: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM1]
		for v in vertices.size():
			var p := Vector3.ZERO
			for k in 4:
				var w := weights[v * 4 + k]
				if w > 0.0:
					p += palette[int(joints[v * 4 + k])] * vertices[v] * w
			if first:
				box = AABB(p, Vector3.ZERO)
				first = false
			else:
				box = box.expand(p)
	return box

## A detail size from a box in model space (`boundsSize`): its largest extent.
static func _size_of(box: AABB) -> float:
	return maxf(box.size.x, maxf(box.size.y, box.size.z))

## The radius it is drawn within (`boundsSize`): across its footprint from its
## foot, and its height (Godot's y is the sim's up).
static func _radius_of(box: AABB) -> float:
	var x := maxf(absf(box.position.x), absf(box.end.x))
	var z := maxf(absf(box.position.z), absf(box.end.z))
	return Vector3(x, box.size.y, z).length()

## A posed batch's culling box: every pose stays within it.
static func _grown(box: AABB) -> AABB:
	var reach := _radius_of(box)
	return AABB(Vector3(-reach, box.position.y - reach, -reach), Vector3(2.0 * reach, box.size.y + 2.0 * reach, 2.0 * reach))

func _source(appearance: String) -> String:
	var entry = _catalog.get(appearance)
	return String(entry.get("source", "")) if typeof(entry) == TYPE_DICTIONARY else ""

# Each frame -------------------------------------------------------------------------

## Poses the units for a record the run does not draw: poses carry from
## frame to frame, so every record is posed, in order.
func advance(record: Dictionary) -> void:
	_poses.step(record)

## Poses the units for `record` (records must come in order) and chooses
## what `camera`, drawing into a viewport `size` pixels big, draws of them.
## Returns what the frame posed by kind (`modelCounts`).
func present(record: Dictionary, camera: Camera3D, size: Vector2) -> Dictionary:
	var frame := _poses.step(record)
	var counts := _poses.counts(frame)
	if frame.is_empty() or _kernel == null or not _kernel.ready():
		return counts
	var view = _batches.view_of(camera, size)
	var chosen := {}
	var vehicle_matrices := 0
	for s in frame.soldiers:
		var body: Body = _bodies.get(_poses.resolve(s.kind, s.side, int(s.soldier), int(s.slot), s.operatorMount, s.activeMount))
		if body == null:
			continue
		var position := _point(s.position)
		var tier := _tier(view, position, body.size, body.radius)
		if tier < 0:
			continue
		var base := _pose_body(body, s.clip, float(s.phase), s.blend, "soldiers")
		if base >= 0:
			_choose(chosen, body.batch, tier, Transform3D(Coords.yaw_basis(float(s.facing)), position), _tint(s.side, base))
	# Corpses of one body lying one way share one palette.
	var lying := {}
	for c in frame.corpses:
		_corpse(chosen, view, lying, c, 0.0)
	for f in frame.fading:
		_corpse(chosen, view, lying, f.corpse, float(f.sink))
	for v in frame.vehicles:
		var vehicle: Vehicle = _vehicles.get(_poses.resolve(v.kind, v.side, int(v.unit), 0, null, null))
		if vehicle == null:
			continue
		var position := _point(v.position)
		var tier := _tier(view, position, vehicle.size, vehicle.radius)
		if tier < 0:
			continue
		var slots := HULL_SLOT + 1 + vehicle.slot_nodes.size()
		if vehicle_matrices + slots > VEHICLE_MATRICES:
			_dropped.vehicles += 1
			continue
		_articulate(vehicle, v.articulation, vehicle_matrices)
		var basis := Coords.yaw_basis(float(v.yaw))
		if v.tilt != null:
			basis = basis * Coords.sim_transform(Transform3D(v.tilt, Vector3.ZERO)).basis
		_choose(chosen, vehicle.batch, tier, Transform3D(basis, position), _tint(v.side, vehicle_matrices))
		vehicle_matrices += slots
		_posed.vehicles += 1
	for batch in _chosen:
		if not chosen.has(batch):
			for tier in TIERS:
				_batches.choose(batch, tier, PackedFloat32Array(), PackedFloat32Array())
	for batch in chosen:
		for tier in TIERS:
			_batches.choose(batch, tier, chosen[batch].view[tier], chosen[batch].cast[tier])
	_chosen = chosen
	if vehicle_matrices > 0:
		_vehicle_texture.update(_vehicle_image)
	_kernel.dispatch()
	_batches.prepare(camera, size)
	return counts

func _corpse(chosen: Dictionary, view, lying: Dictionary, corpse: Dictionary, sink: float) -> void:
	var appearance := _poses.resolve(corpse.kind, corpse.side, int(corpse.soldier), int(corpse.slot), corpse.operatorMount, null)
	var body: Body = _bodies.get(appearance)
	var lies := int(corpse.lies)
	if body == null or lies >= body.lying.size():
		return
	var at: Array = corpse.position
	var position := _point([at[0], at[1], at[2] - sink])
	var tier := _tier(view, position, body.lying[lies].size, body.lying[lies].radius)
	if tier < 0:
		return
	var key := "%s:%d" % [appearance, lies]
	if not lying.has(key):
		lying[key] = _pose_body(body, body.record.corpsePoses[lies].clip, float(body.record.corpsePoses[lies].phase), null, "corpses")
	if lying[key] >= 0:
		_choose(chosen, body.batch, tier, Transform3D(Coords.yaw_basis(float(corpse.yaw)), position), _tint(corpse.side, lying[key]))

## The tier a model `size` metres big within `radius` of its foot at
## `position` (Godot space) takes in `view`, or -1 off screen (`modelDetail`).
## A model below the impostor size draws its coarsest tier: cards are not yet
## ported.
func _tier(view, position: Vector3, size: float, radius: float) -> int:
	var center := position + Vector3(0.0, size / 2.0, 0.0)
	if not view.sphere_meets(center, radius + SHADOW_MARGIN_M):
		return -1
	var px: float = size * view.pixels_per_metre / maxf(view.eye.distance_to(center), 1e-3)
	var lod: Array = _units.detail.lod_px
	if px > float(lod[0]):
		return 0
	if px > float(lod[1]):
		return 1
	if px > float(lod[2]):
		return 2
	if px < float(_units.detail.impostor_px):
		_far_soldiers += 1
	return COARSEST

func _choose(chosen: Dictionary, batch: int, tier: int, transform: Transform3D, tint: Color) -> void:
	if not chosen.has(batch):
		var view: Array[PackedFloat32Array] = []
		var cast: Array[PackedFloat32Array] = []
		for t in TIERS:
			view.append(PackedFloat32Array())
			cast.append(PackedFloat32Array())
		chosen[batch] = {"view": view, "cast": cast}
	ModelBatches.pack(chosen[batch].view[tier], transform, tint)
	ModelBatches.pack(chosen[batch].cast[mini(tier + CASTER_COARSER, COARSEST)], transform, tint)

## The side's tint, with the palette base riding in alpha.
func _tint(side: String, base: int) -> Color:
	var tint: Array = _tints[side]
	return Color(tint[0], tint[1], tint[2], base)

## Queues `body`'s pose on the kernel at `clip` and `phase`, crossfaded
## over by `blend`; returns its palette base, or -1 past the palette's
## capacity (counted under `what`).
func _pose_body(body: Body, clip: String, phase: float, blend: Variant, what: String) -> int:
	var record: Dictionary = body.record
	var a := _clip_ref(body, clip, phase)
	var b := a
	var weight := 0.0
	if blend != null and record.clips.has(blend.clip):
		b = _clip_ref(body, blend.clip, float(blend.phase))
		weight = float(blend.weight)
	var base := _kernel.queue(a, b, weight, record.joints.size(), body.joint_base, body.binds, body.bind_base)
	if base < 0:
		_dropped[what] += 1
	else:
		_posed[what] += 1
	return base

## A kernel clip reference; a clip the body lacks holds idle.
func _clip_ref(body: Body, clip: String, phase: float) -> PackedInt32Array:
	var record: Dictionary = body.record.clips.get(clip, body.record.clips.get("idle"))
	return PoseKernel.clip(body.sample_base + int(record.offset), Articulation.clip_frames(record, phase))

## A vehicle's palette at `base`: its track scroll, its hull's identity, and
## each articulated node's world now times its rest world inverted.
func _articulate(vehicle: Vehicle, articulation: Dictionary, base: int) -> void:
	var record: Dictionary = vehicle.record
	var scroll := Articulation.track_scroll(record.rig, articulation)
	_store(base + DATA_SLOT, Transform3D(Basis(Vector3(scroll.x, scroll.y, 0.0), Vector3.ZERO, Vector3.ZERO), Vector3.ZERO))
	_store(base + HULL_SLOT, Transform3D.IDENTITY)
	var worlds := Articulation.worlds(record.nodes, Articulation.articulate(record.nodes, record.rig, articulation))
	for k in vehicle.slot_nodes.size():
		_store(base + HULL_SLOT + 1 + k, Coords.sim_transform(worlds[vehicle.slot_nodes[k]]) * vehicle.rest_inverse[k])

func _store(index: int, m: Transform3D) -> void:
	for row in 3:
		var texel := index * 3 + row
		_vehicle_image.set_pixel(texel % PALETTE_WIDTH, texel / PALETTE_WIDTH, Color(m.basis.x[row], m.basis.y[row], m.basis.z[row], m.origin[row]))

static func _point(p: Array) -> Vector3:
	return Coords.sim_to_godot(Vector3(p[0], p[1], p[2]))

## Budget rows for the run: what was posed and dropped past the palettes,
## and the models that would have drawn as impostor cards.
func report_budgets(report) -> void:
	for what in ["soldiers", "corpses", "vehicles"]:
		report.add_budget(OWNER, "%s posed over the run, within the palettes' %d matrices a frame" % [what, SOLDIER_MATRICES if what != "vehicles" else VEHICLE_MATRICES], "units." + what, _posed[what] + _dropped[what], _posed[what], _dropped[what])
	report.add_budget(OWNER, "approximation: models below the impostor size draw their coarsest tier; the browser draws a card", "units.impostors", _far_soldiers, _far_soldiers, 0)
