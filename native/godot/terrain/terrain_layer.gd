extends Node3D

## The ground the browser draws (`terrain`): the simulation's triangles seated
## on the height field, and the flat backdrop past the map's edge out to the
## light's backdrop reach, both painted by the browser's ground material
## (`terrain.gdshader`) from the browser's own tables. In the class view the
## ground draws its classes instead and the backdrop is black.

const DataTexture = preload("res://core/data_texture.gd")
const Coords = preload("res://core/coords.gd")

const OWNER := "native/terrain"
const TABLES := ["heights", "nodes", "plots", "surfaces", "surfaceIndex", "params"]
## The texture each record table is read from in `ground.gdshaderinc`.
const SAMPLERS := {"nodes": "ground_nodes", "plots": "ground_plots", "surfaces": "ground_surfaces", "surfaceIndex": "ground_index"}
const SHADER_PATH := "res://terrain/terrain.gdshader"
const MARKER := "// MANIFEST_DEFINES"

var _tables: Dictionary
var _heights
## `shader_header`'s result for the tables and tuning.
var _header: Dictionary
var _reach_m: float
var _class_view: bool

## `tables` maps each terrain table's name to `{words, layout}`; `heights` is
## its decoded `height_field.gd`; `tuning` is the manifest's `tuning.terrain`.
func _init(tables: Dictionary, heights, tuning: Dictionary, reach_m: float, class_view: bool) -> void:
	name = "Terrain"
	_tables = tables
	_heights = heights
	_reach_m = reach_m
	_class_view = class_view
	_header = shader_header(tables, tuning)

## Builds the ground and the backdrop. A table the material cannot read is a
## blocker; what native leaves out of the browser's ground is a budget.
func build(report) -> void:
	if not _header.ok:
		report.add_blocker("terrain", _header.detail)
		return
	# One upload of each table, shared by the ground and the backdrop.
	var uniforms := {"ground_params": params_vectors(_tables.params)}
	for table in SAMPLERS:
		uniforms[SAMPLERS[table]] = DataTexture.words_texture(_tables[table].words)
	var mesh := ground_mesh(_heights)
	var ground := MeshInstance3D.new()
	ground.name = "Ground"
	ground.mesh = mesh.mesh
	ground.material_override = _material(false, uniforms)
	add_child(ground)
	var backdrop := MeshInstance3D.new()
	backdrop.name = "Backdrop"
	backdrop.mesh = backdrop_mesh(_heights, mesh.lowest, _reach_m)
	backdrop.material_override = _material(true, uniforms)
	backdrop.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(backdrop)
	report.add_budget(OWNER, "the simulation's ground triangles, all-flat rectangles coalesced, painted by the browser's ground material", "terrain.heights", mesh.triangles, mesh.triangles, 0)
	if not _class_view:
		report.add_budget(OWNER, "omitted: sun flecks under the forest's crowns (groundDapple); native draws no crowns yet", "terrain.dapple", 1, 0, 1)

## The defines `ground.gdshaderinc` reads, from the tables' layouts and the
## tuning table: `{ok: true, defines}` or `{ok: false, detail}`.
static func shader_header(tables: Dictionary, tuning: Dictionary) -> Dictionary:
	var lines: Array[String] = []
	for table in TABLES:
		if not tables.has(table):
			return {"ok": false, "detail": "the manifest has no terrain table %s" % table}
	for table in ["nodes", "plots", "surfaces"]:
		var layout: Dictionary = tables[table].layout
		if layout.get("kind") != "records":
			return {"ok": false, "detail": "terrain table %s is not records" % table}
		lines.append("#define %s_stride %d" % [table.to_upper(), int(layout.stride)])
		for field in layout.fields:
			lines.append("#define %s_%s %d" % [table.to_upper(), field.name, int(field.word)])
	var index: Dictionary = tables.surfaceIndex.layout
	if index.get("kind") != "words":
		return {"ok": false, "detail": "terrain table surfaceIndex is not words"}
	for constant in index.constants:
		lines.append("#define SURFACEINDEX_%s %du" % [constant, int(index.constants[constant])])
	var params: Dictionary = tables.params.layout
	if params.get("kind") != "records" or int(params.get("count", 0)) != 1 or int(params.stride) % 4 != 0:
		return {"ok": false, "detail": "terrain table params is not one record of whole vec4s"}
	lines.append("#define PARAMS_vec4s %d" % (int(params.stride) / 4))
	var roads := {}
	for field in params.fields:
		if int(field.word) % 4 != 0 or int(field.count) != 4:
			return {"ok": false, "detail": "params field %s is not a vec4" % field.name}
		var parts: PackedStringArray = String(field.name).split("[")
		if parts.size() == 1:
			lines.append("#define PARAMS_%s %d" % [field.name, int(field.word) / 4])
			continue
		# An array of structs: its fields for element 0, its stride and count.
		var element := int(parts[1].split("]")[0])
		var sub: String = parts[1].split("].")[1]
		var array: String = parts[0]
		roads[array] = maxi(roads.get(array, 0), element + 1)
		if element == 0:
			lines.append("#define PARAMS_%s_%s %d" % [array, sub, int(field.word) / 4])
		elif element == 1 and sub == "core":
			lines.append("#define PARAMS_%s_stride %d" % [array, int(field.word) / 4 - _field_word(params, "%s[0].core" % array) / 4])
	for array in roads:
		lines.append("#define PARAMS_%s_count %d" % [array, roads[array]])
	_tuning_defines(tuning, "TUNING", lines)
	return {"ok": true, "defines": "\n".join(lines)}

static func _field_word(layout: Dictionary, field_name: String) -> int:
	for field in layout.fields:
		if field.name == field_name:
			return int(field.word)
	return -1

static func _tuning_defines(value: Variant, path: String, lines: Array[String]) -> void:
	if value is Dictionary:
		for key in value:
			_tuning_defines(value[key], "%s_%s" % [path, key], lines)
	elif value is Array:
		for at in value.size():
			_tuning_defines(value[at], "%s_%d" % [path, at], lines)
	else:
		lines.append("#define %s %s" % [path, _float_literal(float(value))])

## A float as shader text that reads back as the same f32.
static func _float_literal(value: float) -> String:
	var text := var_to_str(value)
	return text if text.contains(".") or text.contains("e") else text + ".0"

func _material(backdrop: bool, uniforms: Dictionary) -> ShaderMaterial:
	var modes: Array[String] = []
	var defines: Array[String] = []
	if _class_view:
		modes.append("unshaded")
		defines.append("#define CLASS_VIEW")
	if backdrop:
		modes.append("shadows_disabled")
		defines.append("#define BACKDROP")
	var code := FileAccess.get_file_as_string(SHADER_PATH)
	var header: String = "\n".join(defines) + "\n" + _header.defines
	if not modes.is_empty():
		header = "render_mode %s;\n%s" % [", ".join(modes), header]
	var shader := Shader.new()
	shader.code = code.replace(MARKER, header)
	var material := ShaderMaterial.new()
	material.shader = shader
	for uniform in uniforms:
		material.set_shader_parameter(uniform, uniforms[uniform])
	return material

## The params record as the vec4s the shader reads, each field decoded by its
## type.
static func params_vectors(table: Dictionary) -> PackedVector4Array:
	var words: PackedByteArray = table.words
	var vectors := PackedVector4Array()
	vectors.resize(int(table.layout.stride) / 4)
	for field in table.layout.fields:
		var at := int(field.word)
		var values := []
		for k in 4:
			match String(field.type):
				"f32":
					values.append(words.decode_float((at + k) * 4))
				"i32":
					values.append(float(words.decode_s32((at + k) * 4)))
				_:
					values.append(float(words.decode_u32((at + k) * 4)))
		vectors[at / 4] = Vector4(values[0], values[1], values[2], values[3])
	return vectors

## The simulation's ground mesh (`World::terrain_mesh`): the vertex grid split
## in halves until a rectangle holds no relief page or is one cell, each
## rectangle two triangles on its south-west to north-east diagonal, flat
## shaded. Returns `{mesh, triangles, lowest}`.
static func ground_mesh(heights) -> Dictionary:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	_mesh_rect(heights, 0, 0, heights.nx - 1, heights.ny - 1, vertices, normals)
	var lowest := INF
	for vertex in vertices:
		lowest = minf(lowest, vertex.y)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return {"mesh": mesh, "triangles": vertices.size() / 3, "lowest": lowest}

static func _mesh_rect(heights, x0: int, y0: int, x1: int, y1: int, vertices: PackedVector3Array, normals: PackedVector3Array) -> void:
	if not heights.has_relief(x0, y0, x1, y1) or (x1 - x0 == 1 and y1 - y0 == 1):
		var corner := func(i: int, j: int) -> Vector3:
			return Coords.sim_to_godot(Vector3(i * heights.spacing, j * heights.spacing, heights.sample(i, j)))
		var sw: Vector3 = corner.call(x0, y0)
		var se: Vector3 = corner.call(x1, y0)
		var ne: Vector3 = corner.call(x1, y1)
		var nw: Vector3 = corner.call(x0, y1)
		_triangle(sw, se, ne, vertices, normals)
		_triangle(sw, ne, nw, vertices, normals)
		return
	var xm := (x0 + x1) / 2
	var ym := (y0 + y1) / 2
	var xs := [[x0, xm], [xm, x1]] if x1 - x0 > 1 else [[x0, x1]]
	var ys := [[y0, ym], [ym, y1]] if y1 - y0 > 1 else [[y0, y1]]
	for y in ys:
		for x in xs:
			_mesh_rect(heights, x[0], y[0], x[1], y[1], vertices, normals)

## One up-facing triangle, wound as Godot's front faces are (clockwise seen
## from above) whichever way its corners come.
static func _triangle(a: Vector3, b: Vector3, c: Vector3, vertices: PackedVector3Array, normals: PackedVector3Array) -> void:
	var up := (c - a).cross(b - a)
	if up.y < 0.0:
		var swap := b
		b = c
		c = swap
		up = -up
	vertices.append_array([a, b, c])
	var normal := up.normalized()
	normals.append_array([normal, normal, normal])

## The flat land past the map's edge (`backdrop.ts`): four bands round the
## grid's box at its lowest ground, out to `reach_m`.
static func backdrop_mesh(heights, lowest: float, reach_m: float) -> ArrayMesh:
	var width: float = (heights.nx - 1) * heights.spacing
	var depth: float = (heights.ny - 1) * heights.spacing
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var band := func(x0: float, y0: float, x1: float, y1: float) -> void:
		var at := func(x: float, y: float) -> Vector3: return Coords.sim_to_godot(Vector3(x, y, lowest))
		_triangle(at.call(x0, y0), at.call(x1, y0), at.call(x1, y1), vertices, normals)
		_triangle(at.call(x0, y0), at.call(x1, y1), at.call(x0, y1), vertices, normals)
	band.call(-reach_m, -reach_m, width + reach_m, 0.0)
	band.call(-reach_m, depth, width + reach_m, depth + reach_m)
	band.call(-reach_m, 0.0, 0.0, depth)
	band.call(width, 0.0, width + reach_m, depth)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh
