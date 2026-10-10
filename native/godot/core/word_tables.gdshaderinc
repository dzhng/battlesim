// A word table's words, read back exactly from the RGBA8 texture
// `word_tables.gd` uploads: one texel per little-endian u32.

uint table_word(sampler2D table, int i) {
	int width = textureSize(table, 0).x;
	uvec4 b = uvec4(round(texelFetch(table, ivec2(i % width, i / width), 0) * 255.0));
	return b.r | (b.g << 8u) | (b.b << 16u) | (b.a << 24u);
}

vec4 table_vec4(sampler2D table, int i) {
	return vec4(uintBitsToFloat(table_word(table, i)), uintBitsToFloat(table_word(table, i + 1)), uintBitsToFloat(table_word(table, i + 2)), uintBitsToFloat(table_word(table, i + 3)));
}
