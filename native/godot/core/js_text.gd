extends RefCounted

## Text as the browser makes and reads it, ported exactly:
## - its `hashString` (`packages/renderer-core/src/math.ts`, FNV-1a over the
##   key's UTF-16 code units) and the text JavaScript writes for a number in
##   a template string, which its keys embed (`${x}`). A dust puff, a smoke
##   source and a tracer's chance are seeded so; native draws the same ones
##   only if it writes the same keys;
## - a decimal read as the nearest double, as JavaScript's JSON reads it.
##   Godot's own reader can land an ulp off, which moves a clock exactly on a
##   tick's boundary to the other side of it.

const Mulberry32 = preload("res://core/mulberry32.gd")

## `hashString(key)`: FNV-1a of `key`'s code units, as an unsigned word.
static func hash_string(key: String) -> int:
	var h := 2166136261
	for i in key.length():
		h = Mulberry32.imul(h ^ key.unicode_at(i), 16777619)
	return h

## `String(x)` for a number the browser decoded from a float32 (every key
## it makes from a published position): the fewest significant digits that
## read back as `x` (JavaScript's rule, ties to the even digit), in fixed
## notation as JavaScript writes it between 1e-6 and 1e21, `-0` as `0`, and
## `null` for null. Any other number is an error: no key holds one.
static func number_text(x: Variant) -> String:
	if x == null:
		return "null"
	var value := float(x)
	if value == 0.0:
		return "0"
	var size := absf(value)
	var bytes := PackedByteArray()
	bytes.resize(4)
	bytes.encode_float(0, value)
	if not is_finite(value) or size < 1e-6 or size >= 1e21 or bytes.decode_float(0) != value:
		push_error("js_text: %s is no float32 in fixed notation" % value)
		return str(value)
	var sign := "-" if value < 0.0 else ""
	if value == floorf(value):
		return sign + str(int(size))
	# The float32 exactly: mantissa · 2^-shift = mantissa · 5^shift / 10^shift.
	var bits := bytes.decode_u32(0)
	var biased := (bits >> 23) & 0xff
	var mantissa := (bits & 0x7fffff) | (0x800000 if biased > 0 else 0)
	var shift := 150 - maxi(biased, 1)
	while mantissa & 1 == 0:
		mantissa >>= 1
		shift -= 1
	var digits := _decimal(mantissa, shift)
	var point := digits.length() - shift
	# A double reads back as `value` within half its gap either side (the gap
	# below a power of two is half the one above): 2^(floor(log2 x) - 53).
	var gap := 53 - (biased - 127)
	var below_gap := gap + (1 if bits & 0x7fffff == 0 else 0)
	var above := _power_of_five(gap)
	var below := _power_of_five(below_gap)
	for count in range(1, digits.length()):
		var rounded := _round_even(digits, count)
		var rest := digits.substr(count)
		# How far the rounded digits are from the value, as 0.<d> × 10^e.
		var off: Array = [_complement(rest), point - count] if rounded[2] else [rest, point - count]
		if _compare(off, above if rounded[2] else below) <= 0:
			return sign + _fixed(rounded[0], point + rounded[1])
	return sign + _fixed(digits.rstrip("0"), point)

## `5^n · 10^-n` (a half gap) as `[digits, e]`, the value `0.<digits> × 10^e`.
static func _power_of_five(n: int) -> Array:
	var digits := _decimal(1, n)
	return [digits, digits.length() - n]

## `1 - 0.<digits>` as the digits after the point (`digits` not all zero).
static func _complement(digits: String) -> String:
	var out := digits.to_utf8_buffer()
	var borrow := true
	for k in range(out.size() - 1, -1, -1):
		var d := out[k] - 48
		if borrow:
			if d == 0:
				out[k] = 48
				continue
			out[k] = 48 + 10 - d
			borrow = false
		else:
			out[k] = 48 + 9 - d
	return out.get_string_from_utf8()

## The order of two values `[digits, e]`, each `0.<digits> × 10^e`.
static func _compare(a: Array, b: Array) -> int:
	var x := _normal(a)
	var y := _normal(b)
	if x[0].is_empty() or y[0].is_empty():
		return int(not x[0].is_empty()) - int(not y[0].is_empty())
	if x[1] != y[1]:
		return 1 if x[1] > y[1] else -1
	var width := maxi(x[0].length(), y[0].length())
	var p: String = x[0].rpad(width, "0")
	var q: String = y[0].rpad(width, "0")
	return 0 if p == q else (1 if p > q else -1)

## `[digits, e]` without leading or trailing zeros; empty digits for zero.
static func _normal(value: Array) -> Array:
	var digits: String = value[0]
	var lead := 0
	while lead < digits.length() and digits[lead] == "0":
		lead += 1
	return [digits.substr(lead).rstrip("0"), int(value[1]) - lead]

## The decimal digits of `mantissa · 5^power`, most significant first.
static func _decimal(mantissa: int, power: int) -> String:
	# Little-endian base-10 digits.
	var out := PackedByteArray()
	while mantissa > 0:
		out.append(mantissa % 10)
		mantissa /= 10
	for _i in power:
		var carry := 0
		for k in out.size():
			var d := out[k] * 5 + carry
			out[k] = d % 10
			carry = d / 10
		while carry > 0:
			out.append(carry % 10)
			carry /= 10
	var text := ""
	for k in range(out.size() - 1, -1, -1):
		text += str(out[k])
	return text

## `digits` rounded to `count` digits, a tie to the even digit:
## `[digits, carry, up]`, `carry` 1 when rounding added a leading digit and
## `up` whether it rounded up.
static func _round_even(digits: String, count: int) -> Array:
	if count >= digits.length():
		return [digits.rstrip("0"), 0, false]
	var kept := digits.substr(0, count).to_utf8_buffer()
	var next := digits.unicode_at(count) - 48
	var rest_zero := digits.substr(count + 1).rstrip("0").is_empty()
	var up := next > 5 or (next == 5 and (not rest_zero or (kept[count - 1] - 48) % 2 == 1))
	if up:
		var at := count - 1
		while at >= 0:
			if kept[at] == 57:
				kept[at] = 48
				at -= 1
			else:
				kept[at] += 1
				break
		if at < 0:
			return [("1" + kept.get_string_from_utf8()).substr(0, count).rstrip("0"), 1, true]
	return [kept.get_string_from_utf8().rstrip("0"), 0, up]

## `0.<digits> × 10^point` in fixed notation.
static func _fixed(digits: String, point: int) -> String:
	if digits.is_empty():
		return "0"
	if point <= 0:
		return "0." + "0".repeat(-point) + digits
	if point >= digits.length():
		return digits + "0".repeat(point - digits.length())
	return digits.substr(0, point) + "." + digits.substr(point)

## The double nearest the decimal `text` (ties to the even mantissa), as
## JavaScript reads it; Godot's `to_float` can be an ulp off.
static func parse_number(text: String) -> float:
	var guess := text.to_float()
	if guess == 0.0 or not is_finite(guess) or absf(guess) >= 9007199254740992.0:
		return guess
	var sign := -1.0 if guess < 0.0 else 1.0
	var wanted := _digits(text)
	var value := absf(guess)
	# Within an ulp or two: the digits as an integer over an exact power of ten.
	var digits: String = wanted[0]
	var scale: int = wanted[1] - digits.length()
	if digits.length() <= 18 and absi(scale) <= 22:
		value = float(int(digits)) * pow(10.0, scale) if scale >= 0 else float(int(digits)) / pow(10.0, -scale)
	var bytes := PackedByteArray()
	bytes.resize(8)
	for _step in 8:
		bytes.encode_double(0, value)
		var bits := bytes.decode_u64(0)
		var biased := (bits >> 52) & 0x7ff
		var mantissa := (bits & 0xfffffffffffff) | (0x10000000000000 if biased > 0 else 0)
		var exponent := maxi(biased, 1) - 1075
		# The midpoints to the doubles either side; below a power of two the
		# gap is half.
		var low := _midpoint(4 * mantissa - 1, exponent - 2) if bits & 0xfffffffffffff == 0 and biased > 1 else _midpoint(2 * mantissa - 1, exponent - 1)
		var high := _midpoint(2 * mantissa + 1, exponent - 1)
		var below := _compare(wanted, low)
		var above := _compare(wanted, high)
		if (below > 0 or (below == 0 and mantissa % 2 == 0)) and (above < 0 or (above == 0 and mantissa % 2 == 0)):
			return sign * value
		bytes.encode_u64(0, bits + (1 if above >= 0 else -1))
		value = bytes.decode_double(0)
	return sign * value

## `odd · 2^power` as `[digits, e]`, the value `0.<digits> × 10^e`, for a
## double below 2^53 (`power` negative).
static func _midpoint(odd: int, power: int) -> Array:
	var digits := _decimal(odd, -power)
	return [digits, digits.length() + power]

## The significant digits of a decimal `text` (plain or with an exponent,
## sign dropped) and where its point falls: `[digits, e]`, the value
## `0.<digits> × 10^e`.
static func _digits(text: String) -> Array:
	text = text.strip_edges().trim_prefix("-").trim_prefix("+")
	var exponent := 0
	var at := text.findn("e")
	if at >= 0:
		exponent = int(text.substr(at + 1))
		text = text.substr(0, at)
	var dot := text.find(".")
	var whole := text if dot < 0 else text.substr(0, dot)
	var digits := whole + ("" if dot < 0 else text.substr(dot + 1))
	return _normal([digits, whole.length() + exponent])
