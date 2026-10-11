extends Node3D

## Draws placed models as instanced batches, the way the browser's models
## layer does (`packages/battle-renderer/src/models/modelLayer.ts`): one
## MultiMesh per appearance, whose mesh holds one surface per material with
## every part's own transform baked in. Draw calls follow appearances ×
## materials × passes, not placed objects or their parts.
##
## An appearance is any node tree of meshes (a catalog scene, a kit's shell, a
## stand-in). Statics are placed once; movers are replaced every frame.

## One appearance: its merged mesh and the instances drawn with it.
class Appearance:
	var mesh: ArrayMesh
	## 12 floats an instance (`MultiMesh.TRANSFORM_3D`'s layout).
	var statics := PackedFloat32Array()
	var movers := PackedFloat32Array()
	var batch: MultiMeshInstance3D

var _appearances: Array[Appearance] = []
var _sealed := false

func _init(batches_name: String) -> void:
	name = batches_name

## Registers the meshes below `root` (not `root`'s own transform) as an
## appearance; returns its id.
func add_appearance(root: Node) -> int:
	var parts := []
	for child in root.get_children():
		_collect(child, Transform3D.IDENTITY, parts)
	var appearance := Appearance.new()
	appearance.mesh = merged(parts)
	_appearances.append(appearance)
	return _appearances.size() - 1

## Places one instance of `appearance` for the whole run.
func add_static(appearance: int, transform: Transform3D) -> void:
	_pack(_appearances[appearance].statics, transform)

## Replaces every mover with `placements`, `[appearance id, Transform3D]` each.
func show_movers(placements: Array) -> void:
	var moved := {}
	for appearance in _appearances:
		if not appearance.movers.is_empty():
			moved[appearance] = true
			appearance.movers.clear()
	for placement in placements:
		moved[_appearances[placement[0]]] = true
		_pack(_appearances[placement[0]].movers, placement[1])
	for appearance in moved:
		_write(appearance)

## Writes the statics' batches once; movers are written by `show_movers`.
func seal() -> void:
	if not _sealed:
		_sealed = true
		for appearance in _appearances:
			_write(appearance)

func _write(appearance: Appearance) -> void:
	if appearance.mesh == null:
		return
	var buffer := appearance.statics + appearance.movers
	if appearance.batch == null:
		appearance.batch = MultiMeshInstance3D.new()
		appearance.batch.multimesh = MultiMesh.new()
		appearance.batch.multimesh.transform_format = MultiMesh.TRANSFORM_3D
		appearance.batch.multimesh.mesh = appearance.mesh
		add_child(appearance.batch)
	var multimesh := appearance.batch.multimesh
	multimesh.instance_count = buffer.size() / 12
	if not buffer.is_empty():
		multimesh.buffer = buffer
	appearance.batch.visible = not buffer.is_empty()

## One mesh of `parts` (`{mesh, transform}`), a surface per material, each
## part's transform baked into its vertices; null when there are no parts.
## Instances draw no skeleton, so a skinned part is drawn as its vertices lie.
static func merged(parts: Array) -> ArrayMesh:
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
		out.surface_set_material(out.get_surface_count() - 1, material)
	return out

## Every mesh below a node, with its transform from the node `parent` maps
## from; a skinned mesh is posed where its skeleton stands.
static func _collect(node: Node, parent: Transform3D, parts: Array) -> void:
	var transform: Transform3D = parent * node.transform if node is Node3D else parent
	if node is MeshInstance3D and node.mesh != null:
		var skeleton := node.get_node_or_null(node.skeleton) as Skeleton3D if node.skin != null else null
		if skeleton != null:
			# Godot skins in the skeleton's space, whatever the mesh node's own
			# transform; its glTF import parents a skinned mesh to its skeleton.
			if skeleton != node.get_parent():
				push_error("%s is skinned to %s, which is not its parent" % [node.name, skeleton.name])
			parts.append({"mesh": _posed(node.mesh, node.skin, skeleton), "transform": parent})
		else:
			parts.append({"mesh": node.mesh, "transform": transform})
	for child in node.get_children():
		_collect(child, transform, parts)

## `mesh` skinned on the CPU to `skeleton`'s current pose, as the GPU would
## draw it, in the skeleton's space.
static func _posed(mesh: Mesh, skin: Skin, skeleton: Skeleton3D) -> ArrayMesh:
	var binds: Array[Transform3D] = []
	for i in skin.get_bind_count():
		var bone := skin.get_bind_bone(i)
		if bone < 0:
			bone = skeleton.find_bone(skin.get_bind_name(i))
		binds.append(skeleton.get_bone_global_pose(bone) * skin.get_bind_pose(i))
	var out := ArrayMesh.new()
	for surface in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(surface)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL] if arrays[Mesh.ARRAY_NORMAL] != null else PackedVector3Array()
		var tangents: PackedFloat32Array = arrays[Mesh.ARRAY_TANGENT] if arrays[Mesh.ARRAY_TANGENT] != null else PackedFloat32Array()
		var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES]
		var weights: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS]
		var per := bones.size() / vertices.size()
		for v in vertices.size():
			var blend := Transform3D(Basis(Vector3.ZERO, Vector3.ZERO, Vector3.ZERO), Vector3.ZERO)
			for k in per:
				var w := weights[v * per + k]
				if w > 0.0:
					var m := binds[bones[v * per + k]]
					blend = Transform3D(Basis(blend.basis.x + m.basis.x * w, blend.basis.y + m.basis.y * w, blend.basis.z + m.basis.z * w), blend.origin + m.origin * w)
			vertices[v] = blend * vertices[v]
			if not normals.is_empty():
				normals[v] = (blend.basis * normals[v]).normalized()
			if not tangents.is_empty():
				var t := (blend.basis * Vector3(tangents[v * 4], tangents[v * 4 + 1], tangents[v * 4 + 2])).normalized()
				tangents[v * 4] = t.x
				tangents[v * 4 + 1] = t.y
				tangents[v * 4 + 2] = t.z
		arrays[Mesh.ARRAY_VERTEX] = vertices
		if not normals.is_empty():
			arrays[Mesh.ARRAY_NORMAL] = normals
		if not tangents.is_empty():
			arrays[Mesh.ARRAY_TANGENT] = tangents
		arrays[Mesh.ARRAY_BONES] = null
		arrays[Mesh.ARRAY_WEIGHTS] = null
		out.add_surface_from_arrays(mesh.surface_get_primitive_type(surface), arrays)
		out.surface_set_material(surface, mesh.surface_get_material(surface))
	return out

## Appends `t` in `MultiMesh.TRANSFORM_3D`'s row-major 3×4 layout.
static func _pack(out: PackedFloat32Array, t: Transform3D) -> void:
	out.append_array([t.basis.x.x, t.basis.y.x, t.basis.z.x, t.origin.x, t.basis.x.y, t.basis.y.y, t.basis.z.y, t.origin.y, t.basis.x.z, t.basis.y.z, t.basis.z.z, t.origin.z])
