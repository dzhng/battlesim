extends Node3D

## The ground the browser draws (`terrain`): the simulation's triangles seated
## on the height field, and the flat backdrop past the map's edge out to the
## light's backdrop reach, both painted by the browser's ground material
## (`terrain.gdshader`) from the browser's own tables, with the side's learned
## scars (`scars`) on the ground. With the `water` layer, the water surface
## over the browser's water cells (`water.gdshader`).

const WordTables = preload("res://core/word_tables.gd")
const Coords = preload("res://core/coords.gd")
const ShaderText = preload("res://core/shader_text.gd")

const OWNER := "native/terrain"
const TABLES := ["heights", "nodes", "plots", "surfaces", "surfaceIndex", "params", "scars", "water"]
## The texture each record table is read from in `ground.gdshaderinc`.
const SAMPLERS := {"nodes": "ground_nodes", "plots": "ground_plots", "surfaces": "ground_surfaces", "surfaceIndex": "ground_index"}
const SHADER_PATH := "res://terrain/terrain.gdshader"
const WATER_PATH := "res://terrain/water.gdshader"
const MARKER := "// MANIFEST_DEFINES"

var _tables: Dictionary
var _heights
## `shader_header`'s result for the tables and tuning.
var _header: Dictionary
var _reach_m: float
var _scars: Texture2D
var _water: bool

## `tables` maps each terrain table's name to `{words, layout}`; `heights` is
## its decoded `height_field.gd`; `tuning` is the manifest's `tuning.terrain`;
## `scars` the learned scars' words (`scar_texture.gd`); `water` whether the
## water layer is drawn.
func _init(tables: Dictionary, heights, tuning: Dictionary, reach_m: float, scars: Texture2D, water: bool) -> void:
	name = "Terrain"
	_tables = tables
	_heights = heights
	_reach_m = reach_m
	_scars = scars
	_water = water
	_header = shader_header(tables, tuning)

## Builds the ground and the backdrop. A table the material cannot read is a
## blocker; what native leaves out of the browser's ground is a budget.
func build(report) -> void:
	if not _header.ok:
		report.add_blocker("terrain", _header.detail)
		return
	# One upload of each table, shared by the ground and the backdrop.
	var uniforms := {"ground_params": WordTables.vec4s(_tables.params), "scar_params": WordTables.vec4s(_tables.scars), "scar_words": _scars}
	for table in SAMPLERS:
		uniforms[SAMPLERS[table]] = WordTables.texture(_tables[table].words)
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
	if _water:
		var cells := water_cells(_tables.water)
		if not cells.ok:
			report.add_blocker("water", cells.detail)
			return
		var water := MeshInstance3D.new()
		water.name = "Water"
		water.mesh = water_mesh(cells.cells)
		water.material_override = _material(false, uniforms, WATER_PATH)
		water.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(water)
		report.add_budget(OWNER, "approximation: the water keeps Godot's own light in shadow, not the browser's share of it (terrainTuning.water.shadow)", "water.shadow", cells.cells.size(), cells.cells.size(), 0)
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
	for table in ["params", "scars"]:
		var detail := _vec4_defines(tables[table].layout, table, lines)
		if not detail.is_empty():
			return {"ok": false, "detail": detail}
	ShaderText.defines(tuning, "TUNING", lines)
	return {"ok": true, "defines": "\n".join(lines)}

## `<TABLE>_vec4s` and `<TABLE>_<field>` (each field's vec4) for a one-record
## table of whole vec4s; an array of structs gets its element 0's fields, its
## stride and its count (`PARAMS_roads_core`, `PARAMS_roads_stride`,
## `PARAMS_roads_count`). Returns why it can't, or "".
static func _vec4_defines(layout: Dictionary, table: String, lines: Array[String]) -> String:
	var prefix := table.to_upper()
	if layout.get("kind") != "records" or int(layout.get("count", 0)) != 1 or int(layout.stride) % 4 != 0:
		return "terrain table %s is not one record of whole vec4s" % table
	lines.append("#define %s_vec4s %d" % [prefix, int(layout.stride) / 4])
	var counts := {}
	for field in layout.fields:
		if int(field.word) % 4 != 0 or int(field.count) != 4:
			return "%s field %s is not a vec4" % [table, field.name]
		var parts: PackedStringArray = String(field.name).split("[")
		if parts.size() == 1:
			lines.append("#define %s_%s %d" % [prefix, field.name, int(field.word) / 4])
			continue
		# An array of structs: its fields for element 0, its stride and count.
		var element := int(parts[1].split("]")[0])
		var sub: String = parts[1].split("].")[1]
		var array: String = parts[0]
		counts[array] = maxi(counts.get(array, 0), element + 1)
		if element == 0:
			lines.append("#define %s_%s_%s %d" % [prefix, array, sub, int(field.word) / 4])
		elif element == 1 and sub == "core":
			lines.append("#define %s_%s_stride %d" % [prefix, array, int(field.word) / 4 - _field_word(layout, "%s[0].core" % array) / 4])
	for array in counts:
		lines.append("#define %s_%s_count %d" % [prefix, array, counts[array]])
	return ""

static func _field_word(layout: Dictionary, field_name: String) -> int:
	for field in layout.fields:
		if field.name == field_name:
			return int(field.word)
	return -1

func _material(backdrop: bool, uniforms: Dictionary, path := SHADER_PATH) -> ShaderMaterial:
	var header: String = _header.defines
	if backdrop:
		header = "render_mode shadows_disabled;\n#define BACKDROP\n" + header
	var code := FileAccess.get_file_as_string(path)
	var shader := Shader.new()
	shader.code = code.replace(MARKER, header)
	var material := ShaderMaterial.new()
	material.shader = shader
	for uniform in uniforms:
		material.set_shader_parameter(uniform, uniforms[uniform])
	return material

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

## The water cells (`terrainTables.ts` `water`): `{ok, cells}`, each cell
## `Vector4(corner x, corner y, surface height, side)` in sim metres, or
## `{ok: false, detail}`.
static func water_cells(table: Dictionary) -> Dictionary:
	var layout: Dictionary = table.layout
	var fields := {}
	for field in layout.get("fields", []):
		fields[field.name] = field
	if layout.get("kind") != "records" or not fields.has("corner") or not fields.has("z") or not fields.has("size"):
		return {"ok": false, "detail": "terrain table water is not records of corner, z and size"}
	var words: PackedByteArray = table.words
	var stride := int(layout.stride) * 4
	var cells := PackedVector4Array()
	for at in range(0, int(layout.count) * stride, stride):
		var corner := at + int(fields.corner.word) * 4
		cells.append(Vector4(words.decode_float(corner), words.decode_float(corner + 4), words.decode_float(at + int(fields.z.word) * 4), words.decode_float(at + int(fields.size.word) * 4)))
	return {"ok": true, "cells": cells}

## The water surface (`waterSurfaceMesh`): each cell a flat square at its
## surface height.
static func water_mesh(cells: PackedVector4Array) -> ArrayMesh:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	for cell in cells:
		var at := func(x: float, y: float) -> Vector3: return Coords.sim_to_godot(Vector3(x, y, cell.z))
		var x1 := cell.x + cell.w
		var y1 := cell.y + cell.w
		_triangle(at.call(cell.x, cell.y), at.call(x1, cell.y), at.call(x1, y1), vertices, normals)
		_triangle(at.call(cell.x, cell.y), at.call(x1, y1), at.call(cell.x, y1), vertices, normals)
	var mesh := ArrayMesh.new()
	if vertices.is_empty():
		return mesh
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh

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
