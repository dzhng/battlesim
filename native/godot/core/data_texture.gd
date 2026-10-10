extends RefCounted

## The one owner of table uploads: a table of u32 words as a texture a shader
## reads word by word. Godot spatial shaders have no storage buffers, and
## Godot 4.7 cannot wrap an `R32_UINT` device texture (`Texture2DRD` needs a
## format `Image` knows). So each word's four little-endian bytes are one
## RGBA8 texel, unfiltered and linear, and the shader puts the word back
## together exactly (`ground.gdshaderinc` `table_word`). Word `i` is texel
## `(i % width, i / width)`; the shader reads the width from `textureSize`.

## Rows are this wide, so a table of a few million words stays well inside
## the device's texture size limit.
const WIDTH := 4096

## `words` (little-endian u32 bytes) as a texture. An empty table becomes one
## zero word: a texture cannot be empty.
static func words_texture(words: PackedByteArray) -> ImageTexture:
	var count := words.size() / 4
	var width := clampi(count, 1, WIDTH)
	var height := maxi(1, ceili(float(count) / width))
	var texels := words.duplicate()
	texels.resize(width * height * 4)
	return ImageTexture.create_from_image(Image.create_from_data(width, height, false, Image.FORMAT_RGBA8, texels))
