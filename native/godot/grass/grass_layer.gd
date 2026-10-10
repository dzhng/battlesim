extends Node3D

## The browser's grass field (`grass`), grown on the GPU per frame record as the
## browser grows it (`grassPass.ts`): a compute build (`grass_build.compute`)
## walks the world tiles the view's reach covers, keeps the clumps the ground
## grows, and appends each to the near or far tier. A tier is a MultiMesh
## whose buffer the build writes and whose indirect draw command it counts,
## as the browser's build writes its clump buffer and indirect args. Each
## tier draws every clump with its kind's blade strips (`grass.gdshader`).
##
## The tables are the browser's own (`manifest.grass`); the frame's view
## fields of `params` (side planes, eye, window, grid) are recomputed from each
## frame record (`frame_fields`).

const DataTexture = preload("res://core/data_texture.gd")
const Coords = preload("res://core/coords.gd")
const TerrainLayer = preload("res://terrain/terrain_layer.gd")

const OWNER := "native/grass"
const TABLES := ["params", "props", "shapes", "indicesNear", "indicesFar"]
const BUILD_PATH := "res://grass/grass_build.compute"
const DRAW_PATH := "res://grass/grass.gdshader"
const GROUND_PATH := "res://terrain/ground.gdshaderinc"
const DRAW_MARKER := "// MANIFEST_DEFINES"
## Tiles a side the window may hold (`grassField.ts` GRASS_MAX_TILES).
const MAX_TILES := 256
## Floats per clump in a tier's MultiMesh buffer: its 3×4 transform rows.
const CLUMP_FLOATS := 12
## The screen grid clumps are counted over: columns, rows (`GRASS_CELLS`).
const CELLS := Vector2i(4, 3)

var _tables: Dictionary
var _terrain: Dictionary
var _terrain_tuning: Dictionary
var _tuning: Dictionary
var _constants: Dictionary
var _lens: Dictionary
var _lowest: float
var _vec4s: Dictionary
var _params_words: PackedByteArray
var _rd: RenderingDevice
## RD resources freed with the layer.
var _owned: Array[RID] = []
var _build: RID
var _build_shader: RID
var _uniforms: Array[RDUniform] = []
var _found: RID
var _params: RID
## Per tier (near, far): the RenderingServer MultiMesh, its instance, its mesh.
var _multimeshes: Array[RID] = []
var _instances: Array[RID] = []
var _meshes: Array[ArrayMesh] = []
var _material: ShaderMaterial
## The ground tables the build samples: their device textures live as long
## as these do.
var _ground_textures: Array[ImageTexture] = []
## The tiles the last frame's window held.
var _tiles := Vector2i.ZERO
## Each counted frame's tier counts (`counts`).
var _counted: Array[Dictionary] = []

## `ground` is `parity_run.gd` `read_ground`'s: the terrain's `{tables,
## heights}` and the grass tables (`grass`, each `{words, layout}`); `grass` is
## the manifest's (`constants`).
func _init(ground: Dictionary, grass: Dictionary, tuning: Dictionary, lens: Dictionary) -> void:
	name = "Grass"
	_tables = ground.grass
	_terrain = ground
	_terrain_tuning = tuning.terrain
	_tuning = tuning.grass
	_constants = grass.constants
	_lens = lens
	_lowest = ground.heights.lowest()
	if _tables.has("params"):
		_vec4s = field_vec4s(_tables.params.layout)
		_params_words = _tables.params.words.duplicate()

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE:
		for instance in _instances:
			RenderingServer.free_rid(instance)
		for multimesh in _multimeshes:
			RenderingServer.free_rid(multimesh)
		if _rd != null:
			# Dependents first: a pipeline before its shader.
			for at in range(_owned.size() - 1, -1, -1):
				_rd.free_rid(_owned[at])

## Builds the build pipelines and the two tiers. A table or shader the field
## cannot use is a blocker.
func build(report) -> void:
	var source := build_source()
	if not source.ok:
		report.add_blocker("grass", source.detail)
		return
	_rd = RenderingServer.get_rendering_device()
	if _rd == null:
		report.add_blocker("grass", "the grass build needs a rendering device, which only a display-backed run has")
		return
	_build_shader = _own(compile(_rd, source.source, "grass build"))
	if not _build_shader.is_valid():
		report.add_blocker("grass", "the grass build does not compile")
		return
	_build = _own(_rd.compute_pipeline_create(_build_shader))
	_build_tiers()
	report.add_budget(OWNER, "omitted: the side's learned scars, which leave clumps out and lay them over (phase 2)", "grass.scars", 1, 0, 1)

## Grows the field for one frame record: its view fields, then the build,
## on the rendering device before the frame draws.
func grow(record: Dictionary) -> void:
	if _build.is_valid():
		var fields := frame_fields(record, float(_lens.fovY), _lowest, _density().w, float(_constants.tileM))
		_tiles = fields.grid
		_write_fields(fields)
		RenderingServer.call_on_render_thread(_dispatch)

## The clumps each tier drew and found (before its capacity), and the
## clumps per cell of the screen grid whose root `camera` projects on screen,
## occlusion ignored, as the browser counts them (`reelRun.ts` `grassCells`).
## Call after the frame has drawn.
func counts(camera: Camera3D, canvas: Vector2) -> Dictionary:
	var found := _rd.buffer_get_data(_found).to_int32_array()
	var drawn := []
	var cells := []
	cells.resize(CELLS.x * CELLS.y)
	cells.fill(0)
	for tier in 2:
		var count := _rd.buffer_get_data(RenderingServer.multimesh_get_command_buffer_rd_rid(_multimeshes[tier])).to_int32_array()[1]
		drawn.append(count)
		if count == 0:
			continue
		var clumps := _rd.buffer_get_data(RenderingServer.multimesh_get_buffer_rd_rid(_multimeshes[tier]), 0, count * CLUMP_FLOATS * 4).to_float32_array()
		for i in count:
			var o := i * CLUMP_FLOATS
			var root := Coords.sim_to_godot(Vector3(clumps[o], clumps[o + 1], clumps[o + 2]))
			if camera.is_position_behind(root):
				continue
			var at := camera.unproject_position(root) / canvas
			if at.x < 0.0 or at.x >= 1.0 or at.y < 0.0 or at.y >= 1.0:
				continue
			cells[floori(at.y * CELLS.y) * CELLS.x + floori(at.x * CELLS.x)] += 1
	var result := {"near": drawn[0], "far": drawn[1], "nearFound": found[0], "farFound": found[1], "cells": cells}
	_counted.append(result)
	return result

## The tiers' capacities against the most clumps a counted frame found, and
## every clump dropped past them, as a budget row (call after the last frame).
func report_budget(report) -> void:
	var near_found := 0
	var far_found := 0
	var dropped := 0
	for frame in _counted:
		near_found = maxi(near_found, frame.nearFound)
		far_found = maxi(far_found, frame.farFound)
		dropped += maxi(0, frame.nearFound - frame.near) + maxi(0, frame.farFound - frame.far)
	var capacity: Array = _constants.capacity
	report.add_budget(OWNER, "the browser's tier capacities (%d near, %d far); most found in one frame: %d near, %d far" % [int(capacity[0]), int(capacity[1]), near_found, far_found], "grass.capacity", int(capacity[0]) + int(capacity[1]), near_found + far_found, dropped)

# The frame's view fields ---------------------------------------------------------

## The view fields of `params` for a frame record, as the browser's
## `prepare` writes them: the four side planes (inward unit normal, offset),
## the eye and metres per pixel per metre, the window's corner and its tiles.
## `lowest` is the ground's lowest height, `fade_end` the field's last
## metres per pixel (`density.w`).
static func frame_fields(record: Dictionary, fov_y: float, lowest: float, fade_end: float, tile_m: float) -> Dictionary:
	var eye := Vector3(float(record.eye[0]), float(record.eye[1]), float(record.eye[2]))
	var target := Vector3(float(record.target[0]), float(record.target[1]), float(record.target[2]))
	var up := Vector3(float(record.up[0]), float(record.up[1]), float(record.up[2]))
	var canvas := Vector2(float(record.canvas[0]), float(record.canvas[1]))
	var scale := 2.0 * tan(fov_y / 2.0) / canvas.y
	# Clip x is (f / aspect) times the eye's right, clip y f times its up, clip
	# w the depth: each side plane is w ± x or w ± y, normalised.
	var forward := (target - eye).normalized()
	var right := forward.cross(up).normalized()
	var screen_up := right.cross(forward)
	var f := 1.0 / tan(fov_y / 2.0)
	var aspect := canvas.x / canvas.y
	var planes: Array[Vector4] = []
	for side in [right * (f / aspect), -right * (f / aspect), screen_up * f, -screen_up * f]:
		var normal: Vector3 = forward + side
		var length := normal.length()
		planes.append(Vector4(normal.x, normal.y, normal.z, -normal.dot(eye)) / length)
	# The window: tiles around the eye within the grass's reach, down to the
	# lowest ground (`grassWindow`).
	var reach := fade_end / scale
	var drop := maxf(0.0, eye.z - lowest)
	var corner := Vector2.ZERO
	var tiles := Vector2i.ZERO
	if drop < reach:
		var flat := sqrt(reach * reach - drop * drop)
		corner = Vector2(floorf((eye.x - flat) / tile_m) * tile_m, floorf((eye.y - flat) / tile_m) * tile_m)
		tiles = Vector2i(mini(MAX_TILES, ceili((eye.x + flat - corner.x) / tile_m)), mini(MAX_TILES, ceili((eye.y + flat - corner.y) / tile_m)))
	return {"planes": planes, "eye": Vector4(eye.x, eye.y, eye.z, scale), "window": corner, "grid": tiles}

## Each `params` field's first vec4, by name (`growth[3]` is `growth` + 3).
static func field_vec4s(layout: Dictionary) -> Dictionary:
	var at := {}
	for field in layout.fields:
		var name: String = String(field.name).split("[")[0]
		if not at.has(name):
			at[name] = int(field.word) / 4
	return at

func _density() -> Vector4:
	return TerrainLayer.params_vectors(_tables.params)[_vec4s.density]

## The params fields' vec4s (`GRASS_<field>`) and `TUNING_<path>` for the
## grass tuning, as shader defines.
func _field_defines() -> Array[String]:
	var lines: Array[String] = ["#define GRASS_vec4s %d" % (int(_tables.params.layout.stride) / 4)]
	for name in _vec4s:
		lines.append("#define GRASS_%s %d" % [name, _vec4s[name]])
	TerrainLayer._tuning_defines(_tuning, "TUNING", lines)
	return lines

func _write_fields(fields: Dictionary) -> void:
	for i in 4:
		_put_vec4(_vec4s.planes + i, fields.planes[i])
	_put_vec4(_vec4s.eye, fields.eye)
	_params_words.encode_float(_vec4s.window * 16, fields.window.x)
	_params_words.encode_float(_vec4s.window * 16 + 4, fields.window.y)
	_params_words.encode_u32(_vec4s.grid * 16, fields.grid.x)
	_params_words.encode_u32(_vec4s.grid * 16 + 4, fields.grid.y)

func _put_vec4(at: int, value: Vector4) -> void:
	for k in 4:
		_params_words.encode_float(at * 16 + k * 4, value[k])

# The build ---------------------------------------------------------------------------

## The build's GLSL: the bindings and defines, then the ground formulas with
## their uniforms rebound, then the build. `{ok: true, source}` or
## `{ok: false, detail}`.
func build_source() -> Dictionary:
	for table in TABLES:
		if not _tables.has(table):
			return {"ok": false, "detail": "the manifest has no grass table %s" % table}
	var params: Dictionary = _tables.params.layout
	if params.get("kind") != "records" or int(params.get("count", 0)) != 1:
		return {"ok": false, "detail": "grass table params is not one record"}
	var ground := TerrainLayer.shader_header(_terrain.tables, _terrain_tuning)
	if not ground.ok:
		return ground
	var lines: Array[String] = ["#version 450"]
	lines.append(ground.defines)
	lines.append_array(_field_defines())
	var growth_rows := _field_count(_tables.params.layout, "growth")
	lines.append("#define GROWTH_ROWS %d" % growth_rows)
	lines.append("#define MIX_MAX %d" % (_field_count(_tables.params.layout, "mixes") / growth_rows))
	lines.append("""
layout(set = 0, binding = 0, std430) restrict readonly buffer GrassParams { uvec4 grass_params[]; };
layout(set = 0, binding = 1, std430) restrict readonly buffer Heights { uint heights[]; };
layout(set = 0, binding = 2, std430) restrict readonly buffer Props { uvec4 props[]; };
layout(set = 0, binding = 3, std430) restrict buffer NearClumps { float near_clumps[]; };
layout(set = 0, binding = 4, std430) restrict buffer FarClumps { float far_clumps[]; };
layout(set = 0, binding = 5, std430) restrict buffer NearCommand { uint near_command[]; };
layout(set = 0, binding = 6, std430) restrict buffer FarCommand { uint far_command[]; };
layout(set = 0, binding = 7) uniform sampler2D ground_nodes;
layout(set = 0, binding = 8) uniform sampler2D ground_plots;
layout(set = 0, binding = 9) uniform sampler2D ground_surfaces;
layout(set = 0, binding = 10) uniform sampler2D ground_index;
layout(set = 0, binding = 11, std430) restrict readonly buffer GroundParams { vec4 ground_params[]; };
layout(set = 0, binding = 12, std430) restrict buffer Found { uint found[]; };
""")
	# The ground formulas, as written for the ground's spatial shader: their
	# uniforms are the bindings above.
	var formulas := FileAccess.get_file_as_string(GROUND_PATH)
	var uniform := RegEx.create_from_string("(?m)^uniform [^;]*;\\n")
	if uniform.search_all(formulas).size() != 5:
		return {"ok": false, "detail": "%s no longer declares the four ground tables and the ground params as uniforms" % GROUND_PATH}
	formulas = uniform.sub(formulas, "", true)
	return {"ok": true, "source": "\n".join(lines) + "\n" + formulas + "\n" + FileAccess.get_file_as_string(BUILD_PATH)}

static func _field_count(layout: Dictionary, name: String) -> int:
	return layout.fields.filter(func(field): return String(field.name).begins_with(name + "[")).size()

## Compiles a compute shader on `rd`; a compile error is logged (a blocker).
static func compile(rd: RenderingDevice, source: String, label: String) -> RID:
	var shader_source := RDShaderSource.new()
	shader_source.source_compute = source
	var spirv := rd.shader_compile_spirv_from_source(shader_source)
	if not spirv.compile_error_compute.is_empty():
		push_error("%s does not compile: %s" % [label, spirv.compile_error_compute])
		return RID()
	return rd.shader_create_from_spirv(spirv, label)

func _own(rid: RID) -> RID:
	_owned.append(rid)
	return rid

func _storage(words: PackedByteArray) -> RID:
	var bytes := words if words.size() > 0 else PackedByteArray([0, 0, 0, 0])
	return _own(_rd.storage_buffer_create(bytes.size(), bytes))

## The two tiers: a MultiMesh each, its buffer and indirect command written by
## the build, drawing the tier's index strips with the grass material.
func _build_tiers() -> void:
	var shapes := DataTexture.words_texture(_tables.shapes.words)
	_material = ShaderMaterial.new()
	_material.shader = _draw_shader()
	var vectors := TerrainLayer.params_vectors(_tables.params)
	_material.set_shader_parameter("grass_params", vectors)
	_material.set_shader_parameter("grass_shapes", shapes)
	var heights = _terrain.heights
	var size := Vector2((heights.nx - 1) * heights.spacing, (heights.ny - 1) * heights.spacing)
	# The map's box up to the tallest clump, widened by that height either way,
	# so far a blade can lean: the CPU never sees the clumps to fit it.
	var tallest: float = vectors[_vec4s.window].w
	var box := AABB(Vector3(-tallest, _lowest - 1.0, -size.y - tallest), Vector3(size.x + 2.0 * tallest, tallest + 2.0, size.y + 2.0 * tallest))
	for tier in 2:
		var mesh := _tier_mesh(tier)
		var multimesh := RenderingServer.multimesh_create()
		RenderingServer.multimesh_allocate_data(multimesh, int(_constants.capacity[tier]), RenderingServer.MULTIMESH_TRANSFORM_3D, false, false, true)
		RenderingServer.multimesh_set_mesh(multimesh, mesh.get_rid())
		RenderingServer.multimesh_set_custom_aabb(multimesh, box)
		var instance := RenderingServer.instance_create2(multimesh, get_world_3d().scenario)
		RenderingServer.instance_geometry_set_cast_shadows_setting(instance, RenderingServer.SHADOW_CASTING_SETTING_OFF)
		_meshes.append(mesh)
		_multimeshes.append(multimesh)
		_instances.append(instance)
	_params = _storage(_params_words)
	_found = _storage(PackedByteArray([0, 0, 0, 0, 0, 0, 0, 0]))
	var sampler := _own(_sampler())
	var buffers: Array[RID] = [
		_params,
		_storage(_terrain.tables.heights.words),
		_storage(_tables.props.words),
		RenderingServer.multimesh_get_buffer_rd_rid(_multimeshes[0]),
		RenderingServer.multimesh_get_buffer_rd_rid(_multimeshes[1]),
		RenderingServer.multimesh_get_command_buffer_rd_rid(_multimeshes[0]),
		RenderingServer.multimesh_get_command_buffer_rd_rid(_multimeshes[1]),
	]
	for binding in buffers.size():
		_uniforms.append(_buffer_uniform(binding, buffers[binding]))
	var binding := buffers.size()
	for table in ["nodes", "plots", "surfaces", "surfaceIndex"]:
		var texture := DataTexture.words_texture(_terrain.tables[table].words)
		_ground_textures.append(texture)
		var uniform := RDUniform.new()
		uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_SAMPLER_WITH_TEXTURE
		uniform.binding = binding
		uniform.add_id(sampler)
		uniform.add_id(RenderingServer.texture_get_rd_texture(texture.get_rid()))
		_uniforms.append(uniform)
		binding += 1
	var ground_params := TerrainLayer.params_vectors(_terrain.tables.params)
	_uniforms.append(_buffer_uniform(11, _storage(ground_params.to_byte_array())))
	_uniforms.append(_buffer_uniform(12, _found))

static func _buffer_uniform(binding: int, buffer: RID) -> RDUniform:
	var uniform := RDUniform.new()
	uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	uniform.binding = binding
	uniform.add_id(buffer)
	return uniform

func _sampler() -> RID:
	var state := RDSamplerState.new()
	state.min_filter = RenderingDevice.SAMPLER_FILTER_NEAREST
	state.mag_filter = RenderingDevice.SAMPLER_FILTER_NEAREST
	return _rd.sampler_create(state)

## A tier's mesh: `maxBlades` blade strips of the tier's vertices, indexed by
## the browser's strip indices. The vertex stage places every vertex from its
## index; the positions here only size the buffer.
func _tier_mesh(tier: int) -> ArrayMesh:
	var name: String = ["indicesNear", "indicesFar"][tier]
	var table: Dictionary = _tables[name]
	var indices := PackedInt32Array()
	var words: PackedByteArray = table.words
	for i in int(table.layout.count):
		indices.append(words.decode_u16(i * 2))
	var vertices := PackedVector3Array()
	vertices.resize(int(_constants.maxBlades) * int(_constants.bladeVertices[tier]))
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var material := _material.duplicate()
	material.set_shader_parameter("tier", tier)
	material.set_shader_parameter("blade_vertices", int(_constants.bladeVertices[tier]))
	mesh.surface_set_material(0, material)
	return mesh

func _draw_shader() -> Shader:
	var shader := Shader.new()
	shader.code = FileAccess.get_file_as_string(DRAW_PATH).replace(DRAW_MARKER, "\n".join(_field_defines()))
	return shader

## On the rendering device: zero both tiers' counts, write the frame's
## params, and build.
func _dispatch() -> void:
	for tier in 2:
		_rd.buffer_update(RenderingServer.multimesh_get_command_buffer_rd_rid(_multimeshes[tier]), 4, 4, PackedByteArray([0, 0, 0, 0]))
	_rd.buffer_update(_found, 0, 8, PackedByteArray([0, 0, 0, 0, 0, 0, 0, 0]))
	_rd.buffer_update(_params, 0, _params_words.size(), _params_words)
	if _tiles.x == 0 or _tiles.y == 0:
		return
	var list := _rd.compute_list_begin()
	_rd.compute_list_bind_compute_pipeline(list, _build)
	_rd.compute_list_bind_uniform_set(list, UniformSetCacheRD.get_cache(_build_shader, 0, _uniforms), 0)
	_rd.compute_list_dispatch(list, _tiles.x, _tiles.y, 1)
	_rd.compute_list_end()
