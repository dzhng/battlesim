extends "res://tests/check.gd"

## Text as the browser makes and reads it (`core/js_text.gd`) is
## JavaScript's: a float32's `String(x)` (the fewest digits that read back,
## a tie to the even digit, small numbers in fixed notation), a decimal read
## to the nearest double (where Godot's own reader lands an ulp or more off),
## and `hashString`. The expected values were printed by Node.

const JsText = preload("res://core/js_text.gd")

## A float32's bits and JavaScript's text for it.
const NUMBERS := [
	[1122431700, "115.47427368164062"],
	[1116138036, "67.45742797851562"],
	[1019029558, "0.023090463131666183"],
	[3123387892, "-0.0006527595687657595"],
	[1120416563, "100.0999984741211"],
	[1068708659, "1.399999976158142"],
	[1159735296, "2562.5"],
	[910775196, "0.000003000000106112566"],
	[1056964608, "0.5"],
	[1152288768, "1396.25"],
]
## A decimal and the double JavaScript reads, as its two little-endian words.
const DECIMALS := [
	["18.066666666666666", [286331153, 1077022993]],
	["0.00010107040405273438", [1374389535, 1058700984]],
	["0.0009814891815185546", [4226247819, 1062212777]],
	["123.45678901234567", [133909656, 1079958844]],
]

func run() -> bool:
	var bytes := PackedByteArray()
	bytes.resize(8)
	for pair in NUMBERS:
		bytes.encode_u32(0, pair[0])
		var text := JsText.number_text(bytes.decode_float(0))
		check(text == pair[1], "a float32 writes as %s, not %s" % [pair[1], text])
	check(JsText.number_text(null) == "null" and JsText.number_text(-0.0) == "0" and JsText.number_text(12.0) == "12", "null, -0 and whole numbers write as JavaScript writes them")
	for pair in DECIMALS:
		bytes.encode_u32(0, pair[1][0])
		bytes.encode_u32(4, pair[1][1])
		var want := bytes.decode_double(0)
		check(JsText.parse_number(pair[0]) == want, "%s reads as JavaScript reads it" % pair[0])
	check(JsText.hash_string("heavy_wreck:2562.5,1396.25") == 1041707767, "hashString is the browser's")
	return true
