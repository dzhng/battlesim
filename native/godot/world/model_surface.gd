extends RefCounted

## The one translation of an imported glTF material into the browser's model
## surface (`model_surface.gdshaderinc`): its base colour, textures and
## coverage as Godot imported them, and the scene assets' extras Godot keeps
## as metadata (`colour_scale`, and `tint`, the share of an instance's tint it
## takes). A scenery appearance takes its instance's tint whole, as the
## browser's scenery layer does. A posed surface (a unit's) moves each vertex
## by its instance's palette (`units/unit_layer.gd`): `skinned` by four joints,
## `rigid` by its node's. Not ported: wear chips and lit interiors.

const INCLUDE := "res://world/model_surface.gdshaderinc"

## Source material, tint mode and posing -> its ShaderMaterial.
static var _materials := {}
## Render modes and defines -> the Shader compiled with them.
static var _shaders := {}

## `material` (null for none) as a surface; `scenery` takes the whole tint;
## `posed` is "", "skinned" or "rigid".
static func of(material: Material, scenery: bool, posed := "") -> ShaderMaterial:
	var key := [material, scenery, posed]
	if _materials.has(key):
		return _materials[key]
	var out := ShaderMaterial.new()
	var base := material as BaseMaterial3D
	var extras: Dictionary = material.get_meta("extras", {}) if material != null else {}
	var modes: Array[String] = []
	var defines: Array[String] = []
	if not posed.is_empty():
		defines.append("POSED_" + posed.to_upper())
	if base != null:
		if base.cull_mode == BaseMaterial3D.CULL_DISABLED:
			modes.append("cull_disabled")
		elif base.cull_mode == BaseMaterial3D.CULL_FRONT:
			modes.append("cull_front")
		match base.transparency:
			BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR, BaseMaterial3D.TRANSPARENCY_ALPHA_HASH:
				defines.append("CUTOUT")
			BaseMaterial3D.TRANSPARENCY_ALPHA, BaseMaterial3D.TRANSPARENCY_ALPHA_DEPTH_PRE_PASS:
				defines.append("TRANSLUCENT")
		out.set_shader_parameter("base", base.albedo_color)
		out.set_shader_parameter("roughness", base.roughness)
		out.set_shader_parameter("metallic", base.metallic)
		out.set_shader_parameter("alpha_cutoff", base.alpha_scissor_threshold)
		_texture(out, "albedo", base.albedo_texture)
		# The normal map also carries a cutout's coverage in its alpha.
		_texture(out, "normal", base.normal_texture)
		out.set_shader_parameter("normal_scale", base.normal_scale)
		if _texture(out, "roughness", base.roughness_texture):
			out.set_shader_parameter("roughness_channel", _channel(base.roughness_texture_channel))
		if _texture(out, "metallic", base.metallic_texture):
			out.set_shader_parameter("metallic_channel", _channel(base.metallic_texture_channel))
	out.set_shader_parameter("colour_scale", float(extras.get("colour_scale", 1.0)))
	out.set_shader_parameter("tint_share", 1.0 if scenery else float(extras.get("tint", 0.0)))
	out.shader = _shader(modes, defines)
	out.resource_name = material.resource_name if material != null else ""
	_materials[key] = out
	return out

static func _shader(modes: Array[String], defines: Array[String]) -> Shader:
	var key := ",".join(modes) + "|" + ",".join(defines)
	if not _shaders.has(key):
		var code := "shader_type spatial;\n"
		if not modes.is_empty():
			code += "render_mode %s;\n" % ", ".join(modes)
		for define in defines:
			code += "#define %s\n" % define
		var shader := Shader.new()
		shader.code = code + "#include \"%s\"\n" % INCLUDE
		_shaders[key] = shader
	return _shaders[key]

## Binds `texture` to `<name>_texture`; whether there was one.
static func _texture(out: ShaderMaterial, name: String, texture: Texture2D) -> bool:
	out.set_shader_parameter("has_" + name, texture != null)
	if texture != null:
		out.set_shader_parameter(name + "_texture", texture)
	return texture != null

static func _channel(channel: BaseMaterial3D.TextureChannel) -> Vector4:
	var out := Vector4.ZERO
	# Grayscale reads the red channel.
	out[int(channel) if int(channel) < 4 else 0] = 1.0
	return out
