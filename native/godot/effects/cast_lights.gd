extends RefCounted

## The light combat throws on what is round it, ported from the browser's
## `light/castLights.ts`: of the lights the effects offer each frame
## (`effect_frame.gd`), every one that reaches into the view while they fit,
## else the `MAX` of most weight for the camera's focus. They go to two
## readers: the world's materials (`cast_lights.gdshaderinc`, through global
## shader parameters, Godot space) and the effects' own smoke
## (`effect_pass.gd`, sim space).

const Coords = preload("res://core/coords.gd")
const EffectFrame = preload("res://effects/effect_frame.gd")

## Lights the world reads at once (`CAST_LIGHTS_MAX`).
const MAX := 24
## How fast a light falls off inside its radius (`CAST_FALLOFF`); the
## materials' include writes the same number.
const FALLOFF := 6.0

var _image := Image.create_empty(2 * MAX, 1, false, Image.FORMAT_RGBAF)
var _texture: ImageTexture

func _init() -> void:
	_texture = ImageTexture.create_from_image(_image)
	RenderingServer.global_shader_parameter_set("cast_lights", _texture)

## Packs `batch`'s lights for a view (`detail_view.gd`) on `focus` (sim
## space) from `distance` metres (`packCastLights`), publishes them to the
## world's materials, and returns them for the effects' smoke: a header
## (count, wrap) and per light its position and 1/radius², then its colour.
func pack(batch: EffectFrame.Batch, view, focus: Vector3, distance: float) -> PackedFloat32Array:
	var v := batch.lights
	var order: Array[int] = []
	for i in mini(batch.light_count, EffectFrame.LIGHT_CANDIDATES):
		var o := i * EffectFrame.LIGHT_FLOATS
		if view.sphere_meets(Coords.sim_to_godot(Vector3(v[o], v[o + 1], v[o + 2])), v[o + 3]):
			order.append(i)
	if order.size() > MAX:
		# The browser's weights are float32 and its sort is stable.
		var weights := PackedFloat32Array()
		weights.resize(batch.light_count)
		for i in order:
			weights[i] = _weight(v, i * EffectFrame.LIGHT_FLOATS, focus, distance)
		order.sort_custom(func(a, b): return weights[a] > weights[b] or (weights[a] == weights[b] and a < b))
	var kept := mini(order.size(), MAX)
	var out := PackedFloat32Array()
	out.resize(4 + MAX * 8)
	out[0] = kept
	out[1] = batch.wrap
	for k in kept:
		var o := order[k] * EffectFrame.LIGHT_FLOATS
		var r := v[o + 3]
		var at := Coords.sim_to_godot(Vector3(v[o], v[o + 1], v[o + 2]))
		for j in 8:
			out[4 + k * 8 + j] = [v[o], v[o + 1], v[o + 2], 1.0 / (r * r), v[o + 4], v[o + 5], v[o + 6], 0.0][j]
		_image.set_pixel(2 * k, 0, Color(at.x, at.y, at.z, 1.0 / (r * r)))
		_image.set_pixel(2 * k + 1, 0, Color(v[o + 4], v[o + 5], v[o + 6], 0.0))
	_texture.update(_image)
	RenderingServer.global_shader_parameter_set("cast_light_count", float(kept))
	RenderingServer.global_shader_parameter_set("cast_light_wrap", batch.wrap)
	return out

## Lights nothing: the world's materials take no cast light.
static func clear() -> void:
	RenderingServer.global_shader_parameter_set("cast_light_count", 0.0)

## A light's weight for the cut: its strength over its reach, dimmed by its
## distance from the camera's focus in camera distances.
static func _weight(v: PackedFloat32Array, o: int, focus: Vector3, distance: float) -> float:
	var strength := maxf(v[o + 4], maxf(v[o + 5], v[o + 6])) * v[o + 3] * v[o + 3]
	var d := Vector3(v[o], v[o + 1], v[o + 2]) - focus
	return strength / (1.0 + d.length_squared() / maxf(distance * distance, 1.0))
