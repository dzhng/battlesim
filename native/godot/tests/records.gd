extends RefCounted

## A `records` word table (`{words, layout}`, as the manifest's artifacts read)
## built from rows, for tests: `fields` are `[name, type, count]` in word
## order, each row a dictionary of a value (or an array of `count`) per field.

static func table(fields: Array, rows: Array) -> Dictionary:
	var layout := {"kind": "records", "stride": 0, "count": rows.size(), "fields": []}
	var word := 0
	for field in fields:
		layout.fields.append({"name": field[0], "word": word, "type": field[1], "count": field[2]})
		word += int(field[2])
	layout.stride = word
	var words := PackedByteArray()
	words.resize(rows.size() * word * 4)
	for r in rows.size():
		for field in layout.fields:
			var values = rows[r][field.name]
			if typeof(values) != TYPE_ARRAY:
				values = [values]
			for k in field.count:
				var at: int = (r * word + field.word + k) * 4
				match String(field.type):
					"f32":
						words.encode_float(at, float(values[k]))
					"i32":
						words.encode_s32(at, int(values[k]))
					_:
						words.encode_u32(at, int(values[k]))
	return {"words": words, "layout": layout}
