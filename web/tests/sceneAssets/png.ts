// A tiny PNG encoder for the synthetic textures the scene-assets tests embed.

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (const b of bytes) c = CRC_TABLE[(c ^ b) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/**
 * An RGBA8 PNG with stored (uncompressed) deflate blocks: synchronous and
 * isomorphic, so the builders run in the browser scenes as well as vitest.
 */
export function encodePng(width: number, height: number, rgba: Uint8Array): Uint8Array {
  const row = width * 4;
  const raw = new Uint8Array(height * (row + 1));
  for (let y = 0; y < height; y++)
    raw.set(rgba.subarray(y * row, (y + 1) * row), y * (row + 1) + 1);
  const blocks = Math.max(1, Math.ceil(raw.length / 65535));
  const zlib = new Uint8Array(2 + raw.length + blocks * 5 + 4);
  const z = new DataView(zlib.buffer);
  zlib.set([0x78, 0x01]);
  let at = 2;
  for (let i = 0; i < blocks; i++) {
    const part = raw.subarray(i * 65535, (i + 1) * 65535);
    zlib[at] = i === blocks - 1 ? 1 : 0;
    z.setUint16(at + 1, part.length, true);
    z.setUint16(at + 3, ~part.length & 0xffff, true);
    zlib.set(part, at + 5);
    at += 5 + part.length;
  }
  let a = 1;
  let b = 0;
  for (const v of raw) {
    a = (a + v) % 65521;
    b = (b + a) % 65521;
  }
  z.setUint32(at, ((b << 16) | a) >>> 0);
  const chunk = (type: string, data: Uint8Array) => {
    const out = new Uint8Array(12 + data.length);
    const v = new DataView(out.buffer);
    v.setUint32(0, data.length);
    out.set(
      [...type].map((ch) => ch.charCodeAt(0)),
      4,
    );
    out.set(data, 8);
    v.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));
    return out;
  };
  const ihdr = new Uint8Array(13);
  const h = new DataView(ihdr.buffer);
  h.setUint32(0, width);
  h.setUint32(4, height);
  ihdr.set([8, 6, 0, 0, 0], 8);
  const parts = [
    new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib),
    chunk("IEND", new Uint8Array()),
  ];
  const png = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let o = 0;
  for (const p of parts) {
    png.set(p, o);
    o += p.length;
  }
  return png;
}
