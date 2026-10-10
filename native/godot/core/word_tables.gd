extends RefCounted

## The one owner of the browser's word tables on native: the exact u32 words
## the browser's GPU read, as a texture or as uniform vec4s. Godot spatial
## shaders have no storage buffers, and Godot 4.7 cannot wrap an `R32_UINT`
## device texture (`Texture2DRD` needs a format `Image` knows). So each word's
## four little-endian bytes are one RGBA8 texel, unfiltered and linear, and the
## shader puts the word back together exactly (`word_tables.gdshaderinc`).
## Word `i` is texel `(i % width, i / width)`; the shader reads the width from
## `textureSize`.

## Rows are this wide, so a table of a few million words stays well inside
## the device's texture size limit.
const WIDTH := 4096

## `words` (little-endian u32 bytes) as a texture. An empty table becomes one
## zero word: a texture cannot be empty.
static func texture(words: PackedByteArray) -> ImageTexture:
	var count := words.size() / 4
	var width := clampi(count, 1, WIDTH)
	var height := maxi(1, ceili(float(count) / width))
	var texels := words.duplicate()
	texels.resize(width * height * 4)
	return ImageTexture.create_from_image(Image.create_from_data(width, height, false, Image.FORMAT_RGBA8, texels))

## A one-record table (`{words, layout}`, a `params` record of whole vec4s) as
## the vec4s a shader reads, each field decoded by its type.
static func vec4s(table: Dictionary) -> PackedVector4Array:
	var words: PackedByteArray = table.words
	var vectors := PackedVector4Array()
	vectors.resize(int(table.layout.stride) / 4)
	for field in table.layout.fields:
		var at := int(field.word)
		var values := []
		for k in 4:
			match String(field.type):
				"f32":
					values.append(words.decode_float((at + k) * 4))
				"i32":
					values.append(float(words.decode_s32((at + k) * 4)))
				_:
					values.append(float(words.decode_u32((at + k) * 4)))
		vectors[at / 4] = Vector4(values[0], values[1], values[2], values[3])
	return vectors
