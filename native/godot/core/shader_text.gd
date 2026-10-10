extends RefCounted

## Numbers written into shader text. The browser splices its tuning into WGSL
## as constants; native writes the manifest's numbers into GLSL the same way,
## as literals or as `#define`s, each reading back as the same f32.

## A float as a shader literal.
static func number(value: float) -> String:
	var text := var_to_str(value)
	return text if text.contains(".") or text.contains("e") else text + ".0"

static func rgb(value: Vector3) -> String:
	return "vec3(%s, %s, %s)" % [number(value.x), number(value.y), number(value.z)]

## Appends a `#define <path>_<key>... <number>` line to `lines` for every
## number in `value`, a nested table of objects, arrays and numbers
## (`TUNING_grain_weights_2`).
static func defines(value: Variant, path: String, lines: Array[String]) -> void:
	if value is Dictionary:
		for key in value:
			defines(value[key], "%s_%s" % [path, key], lines)
	elif value is Array:
		for at in value.size():
			defines(value[at], "%s_%d" % [path, at], lines)
	else:
		lines.append("#define %s %s" % [path, number(float(value))])
