extends RefCounted

## The checks every strict reader of a browser file is built from: parsed
## JSON values tested for exactly the shape a writer produces. JSON has one
## number type, so an integer is a number with no fraction.

## An empty string when `value` is an object with exactly `keys`, or the
## first problem, naming `what`.
static func exact_keys(value: Variant, keys: Array, what: String) -> String:
	if typeof(value) != TYPE_DICTIONARY:
		return "%s must be an object" % what
	for key in keys:
		if not value.has(key):
			return "%s is missing %s" % [what, key]
	for key in value:
		if not keys.has(key):
			return "%s has unknown field %s" % [what, key]
	return ""

static func is_name(value: Variant) -> bool:
	return typeof(value) == TYPE_STRING and not String(value).is_empty()

static func is_number(value: Variant) -> bool:
	return (typeof(value) == TYPE_INT or typeof(value) == TYPE_FLOAT) and is_finite(float(value))

static func is_integer(value: Variant) -> bool:
	return is_number(value) and float(value) == floor(float(value))

## Whether `value` is an array of `size` finite numbers.
static func is_vector(value: Variant, size: int) -> bool:
	return typeof(value) == TYPE_ARRAY and value.size() == size and value.all(is_number)

## Whether `value` is a string of `digits` hex digits, in either case.
static func is_hex(value: Variant, digits: int) -> bool:
	if typeof(value) != TYPE_STRING or String(value).length() != digits:
		return false
	for character in String(value).to_lower():
		if not "0123456789abcdef".contains(character):
			return false
	return true

## A JSON triple `[x, y, z]` as a vector, in whatever space it was written in.
static func vector3(value: Array) -> Vector3:
	return Vector3(float(value[0]), float(value[1]), float(value[2]))
