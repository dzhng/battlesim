# Frozen: a port of the per-frame presentation `crates/presentation` now owns; no fixes here, deleted with Godot (slice 25).
extends RefCounted

## The browser's pose kernel (`poseKernel.ts`) on Godot's rendering device: it
## owns the kernel's tables (the clip tables' samples, each body's joint
## parents and each skin's binds), the control records queued each frame, and
## the palette image the posed surfaces read (`texture`). `pose_kernel.compute`
## is its shader.

const ComputePass = preload("res://core/compute_pass.gd")

const SOURCE_PATH := "res://units/pose_kernel.compute"
## Bodies of at most this many joints are posed (the shader's `MAX_JOINTS`).
const MAX_JOINTS := 64
const GROUP := 64
## Words per control record.
const CONTROL_WORDS := 16
## Floats per bind record: its matrix, then its joint (an int) and padding.
const BIND_FLOATS := 20
## Texels per palette row; a matrix is three texels, its rows.
const PALETTE_WIDTH := 1024

## The palette image, for the posed surfaces.
var texture := Texture2DRD.new()
## The most matrices the palette holds.
var capacity: int

var _rd: RenderingDevice
var _owned: Array[RID] = []
var _palette: RID
var _shader: RID
var _pipeline: RID
var _uniform_set: RID
var _controls: RID
var _words := PackedInt32Array()
var _matrices := 0

## The kernel and its palette, holding `capacity` matrices; it poses once
## its tables are loaded (`load_tables`).
func _init(rd: RenderingDevice, palette_capacity: int) -> void:
	_rd = rd
	capacity = palette_capacity
	var format := RDTextureFormat.new()
	format.format = RenderingDevice.DATA_FORMAT_R32G32B32A32_SFLOAT
	format.width = PALETTE_WIDTH
	format.height = ceili(capacity * 3.0 / PALETTE_WIDTH)
	format.usage_bits = RenderingDevice.TEXTURE_USAGE_STORAGE_BIT | RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT | RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT
	_palette = _own(_rd.texture_create(format, RDTextureView.new(), []))
	texture.texture_rd_rid = _palette
	var source := "#version 450\nlayout(local_size_x = %d) in;\n#define MAX_JOINTS %d\n%s" % [GROUP, MAX_JOINTS, FileAccess.get_file_as_string(SOURCE_PATH)]
	_shader = _own(ComputePass.compile(_rd, source, "pose kernel"))
	if _shader.is_valid():
		_pipeline = _own(_rd.compute_pipeline_create(_shader))
	_controls = _own(_rd.storage_buffer_create(capacity * CONTROL_WORDS * 4))

## Loads the tables: `samples` (every clip table, concatenated), `parents`
## (every body's joints, concatenated) and `binds` (`bind` records,
## concatenated). Whether the kernel compiled and can pose.
func load_tables(samples: PackedFloat32Array, parents: PackedInt32Array, binds: PackedFloat32Array) -> bool:
	if not _pipeline.is_valid():
		return false
	_uniform_set = _own(_rd.uniform_set_create([
		ComputePass.storage(0, _buffer(samples.to_byte_array())),
		ComputePass.storage(1, _buffer(parents.to_byte_array())),
		ComputePass.storage(2, _controls),
		ComputePass.image(3, _palette),
		ComputePass.storage(4, _buffer(binds.to_byte_array())),
	], _shader, 0))
	return true

func ready() -> bool:
	return _uniform_set.is_valid()

## A bind record: `bind` takes a mesh vertex into `joint`'s frame at rest.
static func bind(out: PackedFloat32Array, m: Transform3D, joint: int) -> void:
	var b := m.basis
	out.append_array([b.x.x, b.x.y, b.x.z, 0.0, b.y.x, b.y.y, b.y.z, 0.0, b.z.x, b.z.y, b.z.z, 0.0, m.origin.x, m.origin.y, m.origin.z, 1.0])
	out.append_array(PackedInt32Array([joint, 0, 0, 0]).to_byte_array().to_float32_array())

## A clip reference: its first sample (a vec4 index) and its two frames
## either side of the phase with the second's weight (`clip_frames`).
static func clip(first_sample: int, frames: Array) -> PackedInt32Array:
	return PackedInt32Array([first_sample / 4, frames[0], frames[1], _bits(frames[2])])

## Queues one body's pose: clip `a` crossfaded over by clip `b` at `blend`,
## its joints at `joint_base` of the parents and its binds at `bind_base` of
## the binds. Returns its palette base, or -1 when the palette is full.
func queue(a: PackedInt32Array, b: PackedInt32Array, blend: float, joints: int, joint_base: int, binds: int, bind_base: int) -> int:
	if _matrices + binds > capacity:
		return -1
	var base := _matrices
	_words.append_array(a)
	_words.append_array(b)
	_words.append_array([_bits(blend), joints, joint_base, base, binds, bind_base, 0, 0])
	_matrices += binds
	return base

## Poses everything queued, on the rendering device before the frame draws,
## and starts the next frame's queue.
func dispatch() -> void:
	if not _words.is_empty() and ready():
		RenderingServer.call_on_render_thread(_dispatch.bind(_words.duplicate()))
	_words.resize(0)
	_matrices = 0

func _dispatch(words: PackedInt32Array) -> void:
	var count := words.size() / CONTROL_WORDS
	var bytes := words.to_byte_array()
	_rd.buffer_update(_controls, 0, bytes.size(), bytes)
	var push := PackedInt32Array([count, PALETTE_WIDTH, 0, 0]).to_byte_array()
	_rd.draw_command_begin_label("pose kernel", Color.WHITE)
	var list := _rd.compute_list_begin()
	_rd.compute_list_bind_compute_pipeline(list, _pipeline)
	_rd.compute_list_bind_uniform_set(list, _uniform_set, 0)
	_rd.compute_list_set_push_constant(list, push, push.size())
	_rd.compute_list_dispatch(list, ceili(count / float(GROUP)), 1, 1)
	_rd.compute_list_end()
	_rd.draw_command_end_label()

## The palette's first `matrices` matrices as written, three rows of four
## floats each: a readback, after the frame drew.
func read(matrices: int) -> PackedFloat32Array:
	return _rd.texture_get_data(_palette, 0).to_float32_array().slice(0, matrices * 12)

## Frees the device resources; surfaces let go of the palette first.
func free_device() -> void:
	texture.texture_rd_rid = RID()
	for at in range(_owned.size() - 1, -1, -1):
		if _owned[at].is_valid():
			_rd.free_rid(_owned[at])
	_owned.clear()

func _buffer(bytes: PackedByteArray) -> RID:
	if bytes.is_empty():
		bytes = PackedByteArray([0, 0, 0, 0])
	return _own(_rd.storage_buffer_create(bytes.size(), bytes))

func _own(rid: RID) -> RID:
	_owned.append(rid)
	return rid

static func _bits(value: float) -> int:
	return PackedFloat32Array([value]).to_byte_array().decode_s32(0)
