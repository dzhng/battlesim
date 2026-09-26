// Material textures on the GPU: every installed bundle's textures, one layer
// each in two texture arrays the models layer binds once — albedo (sRGB) and
// surface (normal and ORM, linear). A texture is one layer however many
// materials or bundles sample it: layers are keyed by the texture's content
// address. Arrays have one edge length, the largest installed; a smaller
// texture keeps its own levels at the matching array levels and fills the
// finer ones by nearest-neighbour upsampling (every texture we author is one
// size, so nothing is upsampled in practice).

import type { Texture, TextureFormat } from "@packages/scene-assets/src/schema";

/** The layers of one generation's textures, by format. */
export class TextureLayers {
  readonly albedo: Texture[] = [];
  readonly surface: Texture[] = [];
  private readonly at = new Map<string, number>();

  /** The layer `texture` takes in its array (−1 for none). */
  layer(texture: Texture | undefined): number {
    if (!texture) return -1;
    let layer = this.at.get(texture.id);
    if (layer === undefined) {
      const list = texture.format === "rgba8unorm-srgb" ? this.albedo : this.surface;
      layer = list.length;
      list.push(texture);
      this.at.set(texture.id, layer);
    }
    return layer;
  }
}

/** Bytes a texture array of `layers` layers at `size` takes, every level. */
export function arrayBytes(size: number, layers: number): number {
  let bytes = 0;
  for (let s = size; s >= 1; s >>= 1) bytes += s * s * 4 * layers;
  return bytes;
}

function upsample(level: Uint8Array, from: number, to: number): Uint8Array {
  const out = new Uint8Array(to * to * 4);
  const k = from / to;
  for (let y = 0; y < to; y++)
    for (let x = 0; x < to; x++) {
      const s = (Math.floor(y * k) * from + Math.floor(x * k)) * 4;
      out.set(level.subarray(s, s + 4), (y * to + x) * 4);
    }
  return out;
}

/** One array texture holding `textures`, every mip level uploaded. */
export function uploadTextureArray(
  device: GPUDevice,
  label: string,
  format: TextureFormat,
  textures: readonly Texture[],
): GPUTexture {
  const size = Math.max(1, ...textures.map((t) => t.width));
  const levels = Math.log2(size) + 1;
  const texture = device.createTexture({
    label,
    size: [size, size, Math.max(1, textures.length)],
    format,
    mipLevelCount: levels,
    usage: 0x04 | 0x02, // TEXTURE_BINDING | COPY_DST
  });
  textures.forEach((t, layer) => {
    const shift = Math.log2(size / t.width);
    for (let level = 0; level < levels; level++) {
      const edge = size >> level;
      const data = level >= shift ? t.levels[level - shift] : upsample(t.levels[0], t.width, edge);
      device.queue.writeTexture(
        { texture, mipLevel: level, origin: [0, 0, layer] },
        data as Uint8Array<ArrayBuffer>,
        { bytesPerRow: edge * 4, rowsPerImage: edge },
        [edge, edge, 1],
      );
    }
  });
  return texture;
}
