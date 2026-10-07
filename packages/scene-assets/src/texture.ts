// Baked textures: PNG decoding (the only image format a source may embed),
// the mip chain, the content address, and the validator's texture findings.
// Isomorphic: PNG inflate uses the platform's DecompressionStream (browsers,
// Node, Bun), so the CLI, the bake, tests and the workbench share it.

import { sha256Hex } from "./glb.ts";
import {
  TEXTURE_MAX_PX,
  TEXTURE_MIN_PX,
  type Bundle,
  type Finding,
  type Material,
  type MeshData,
  type SkeletonClips,
  type Texture,
  type TextureChannel,
  type TextureFormat,
} from "./schema.ts";

/** Decoded RGBA8 pixels, rows top to bottom. */
export interface Rgba8 {
  width: number;
  height: number;
  pixels: Uint8Array;
}

const PNG_SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];
const CHANNELS: Record<number, number> = { 0: 1, 2: 3, 4: 2, 6: 4 };

async function inflate(data: Uint8Array): Promise<Uint8Array> {
  const stream = new Blob([data as Uint8Array<ArrayBuffer>])
    .stream()
    .pipeThrough(new DecompressionStream("deflate"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Decode an 8-bit, non-interlaced PNG (grey, grey-alpha, RGB or RGBA) to RGBA8. */
export async function decodePng(bytes: Uint8Array): Promise<Rgba8> {
  if (bytes.byteLength < 8 || PNG_SIGNATURE.some((b, i) => bytes[i] !== b))
    throw new Error("not a PNG");
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let offset = 8;
  let width = 0;
  let height = 0;
  let channels = 0;
  const idat: Uint8Array[] = [];
  while (offset + 8 <= bytes.byteLength) {
    const length = dv.getUint32(offset);
    const type = String.fromCharCode(...bytes.subarray(offset + 4, offset + 8));
    const data = bytes.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = dv.getUint32(offset + 8);
      height = dv.getUint32(offset + 12);
      const depth = data[8];
      const colour = data[9];
      if (depth !== 8 || !(colour in CHANNELS) || data[12] !== 0)
        throw new Error(`PNG is ${depth}-bit, colour type ${colour}, interlace ${data[12]}`);
      channels = CHANNELS[colour];
    } else if (type === "IDAT") idat.push(data);
    else if (type === "IEND") break;
    offset += 12 + length;
  }
  if (!channels || !width || !height) throw new Error("PNG has no header");
  const raw = await inflate(concat(idat));
  const stride = width * channels;
  if (raw.byteLength < height * (stride + 1)) throw new Error("PNG data is truncated");
  const rows = new Uint8Array(height * stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const out = rows.subarray(y * stride, (y + 1) * stride);
    const up = y > 0 ? rows.subarray((y - 1) * stride, y * stride) : null;
    for (let i = 0; i < stride; i++) {
      const a = i >= channels ? out[i - channels] : 0;
      const b = up ? up[i] : 0;
      const c = up && i >= channels ? up[i - channels] : 0;
      let v = line[i];
      if (filter === 1) v += a;
      else if (filter === 2) v += b;
      else if (filter === 3) v += (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else if (filter !== 0) throw new Error(`PNG row filter ${filter}`);
      out[i] = v & 255;
    }
  }
  const pixels = new Uint8Array(width * height * 4);
  for (let p = 0; p < width * height; p++) {
    const s = p * channels;
    const grey = channels <= 2;
    pixels[p * 4] = rows[s];
    pixels[p * 4 + 1] = grey ? rows[s] : rows[s + 1];
    pixels[p * 4 + 2] = grey ? rows[s] : rows[s + 2];
    pixels[p * 4 + 3] = channels === 2 ? rows[s + 1] : channels === 4 ? rows[s + 3] : 255;
  }
  return { width, height, pixels };
}

function concat(parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.byteLength, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.byteLength;
  }
  return out;
}

const SRGB_TO_LINEAR = Float64Array.from({ length: 256 }, (_, i) => {
  const c = i / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
});
const linearToSrgbByte = (x: number) => {
  const c = x <= 0.0031308 ? x * 12.92 : 1.055 * x ** (1 / 2.4) - 0.055;
  return Math.round(Math.min(1, Math.max(0, c)) * 255);
};

/** Is `n` a texture edge the validator accepts. */
export const validTextureSize = (width: number, height: number) =>
  width === height &&
  width >= TEXTURE_MIN_PX &&
  width <= TEXTURE_MAX_PX &&
  (width & (width - 1)) === 0;

/** Level count of a full chain down to 1×1. */
export const mipCount = (size: number) => Math.log2(size) + 1;

/**
 * Every mip level of a square power-of-two image, by 2×2 box filter: sRGB
 * colour averages in linear light, normals renormalise, the rest (and every
 * alpha) average as stored. Deterministic.
 */
export function mipChain(image: Rgba8, channel: TextureChannel): Uint8Array[] {
  const levels = [image.pixels.slice()];
  let size = image.width;
  while (size > 1) {
    const src = levels[levels.length - 1];
    const half = size >> 1;
    const dst = new Uint8Array(half * half * 4);
    for (let y = 0; y < half; y++)
      for (let x = 0; x < half; x++) {
        const taps = [
          (2 * y * size + 2 * x) * 4,
          (2 * y * size + 2 * x + 1) * 4,
          ((2 * y + 1) * size + 2 * x) * 4,
          ((2 * y + 1) * size + 2 * x + 1) * 4,
        ];
        const o = (y * half + x) * 4;
        if (channel === "albedo")
          for (let c = 0; c < 3; c++)
            dst[o + c] = linearToSrgbByte(
              taps.reduce((s, t) => s + SRGB_TO_LINEAR[src[t + c]], 0) / 4,
            );
        else if (channel === "normal") {
          const n = [0, 1, 2].map((c) => taps.reduce((s, t) => s + (src[t + c] / 255) * 2 - 1, 0));
          const length = Math.hypot(n[0], n[1], n[2]) || 1;
          for (let c = 0; c < 3; c++) dst[o + c] = Math.round(((n[c] / length) * 0.5 + 0.5) * 255);
        } else
          for (let c = 0; c < 3; c++)
            dst[o + c] = Math.round(taps.reduce((s, t) => s + src[t + c], 0) / 4);
        dst[o + 3] = Math.round(taps.reduce((s, t) => s + src[t + 3], 0) / 4);
      }
    levels.push(dst);
    size = half;
  }
  return levels;
}

/** A texture's runtime file: a header line naming its format and size, then
 *  every level, finest first. Its sha256 is the texture's content address. */
function textureFile(format: TextureFormat, size: number, levels: Uint8Array[]): Uint8Array {
  return concat([new TextEncoder().encode(`${format} ${size}x${size}\n`), ...levels]);
}

/** The content address: sha256 of the texture's runtime file. */
export async function textureId(
  format: TextureFormat,
  size: number,
  levels: Uint8Array[],
): Promise<string> {
  return sha256Hex(textureFile(format, size, levels));
}

/** The runtime file of a baked texture (`textureFile`). */
export const encodeTexture = (t: Texture) => textureFile(t.format, t.width, t.levels);

/** A texture from its runtime file; `id` is the address it was verified at. */
export function decodeTexture(id: string, bytes: Uint8Array): Texture {
  const end = bytes.subarray(0, 64).indexOf(10);
  const header = /^(rgba8unorm-srgb|rgba8unorm) (\d+)x(\d+)$/.exec(
    new TextDecoder().decode(bytes.subarray(0, Math.max(0, end))),
  );
  const size = Number(header?.[2]);
  if (!header || header[3] !== header[2] || !validTextureSize(size, size))
    throw new Error(`texture ${id}: malformed header`);
  const levels: Uint8Array[] = [];
  let at = end + 1;
  for (let edge = size; edge >= 1; edge >>= 1) {
    levels.push(bytes.slice(at, at + edge * edge * 4));
    at += edge * edge * 4;
  }
  if (at !== bytes.byteLength) throw new Error(`texture ${id}: length does not match its header`);
  return { id, format: header[1] as TextureFormat, width: size, height: size, levels };
}

/** Distinct texture layers per GPU array: the renderer holds albedo (sRGB)
 *  and surface (normal and ORM) textures in one array each, a layer per
 *  content address (`battle-renderer` `models/modelTextures.ts`). */
export interface TextureLayerCounts {
  albedo: number;
  surface: number;
}

/** Distinct layers of `textures` per array, a shared texture once. */
export function textureLayerCounts(textures: Iterable<Texture>): TextureLayerCounts {
  const ids = { albedo: new Set<string>(), surface: new Set<string>() };
  for (const t of textures) ids[t.format === "rgba8unorm-srgb" ? "albedo" : "surface"].add(t.id);
  return { albedo: ids.albedo.size, surface: ids.surface.size };
}

/** An array holding more distinct layers than `max` cannot be created. */
export function textureLayerFindings(layers: TextureLayerCounts, max: number): Finding[] {
  return (Object.keys(layers) as (keyof TextureLayerCounts)[])
    .filter((array) => layers[array] > max)
    .map((array) =>
      finding(
        "budget.texture_layers",
        `the ${array} texture array would hold ${layers[array]} distinct layers, over its limit of ${max}`,
        "share a recipe's textures between appearances, or raise the limit after checking the target machine's adapter",
      ),
    );
}

/** A channel's texture from a decoded image: its format, mips and address. */
export async function bakeTexture(
  image: Rgba8,
  channel: TextureChannel,
  format: TextureFormat,
): Promise<Texture> {
  const levels = mipChain(image, channel);
  return {
    id: await textureId(format, image.width, levels),
    format,
    width: image.width,
    height: image.height,
    levels,
  };
}

/** Bytes a texture takes on the GPU (every level). */
export const textureBytes = (t: Texture) => t.levels.reduce((n, l) => n + l.byteLength, 0);

const finding = (code: Finding["code"], message: string, fix: string): Finding => ({
  code,
  severity: "error",
  message,
  fix,
});

/** Every mesh of a (non-clips) bundle, labelled. */
function bundleMeshes(bundle: Exclude<Bundle, SkeletonClips>): [string, MeshData][] {
  if (bundle.kind === "skinned") return bundle.tiers.map((m, t) => [`tier ${t}`, m]);
  if (bundle.kind === "articulated")
    return bundle.nodes.flatMap((n) =>
      n.tiers.map((m, t): [string, MeshData] => [`node ${n.name} tier ${t}`, m]),
    );
  return bundle.states.flatMap((s) =>
    s.tiers.map((m, t): [string, MeshData] => [`${s.name} tier ${t}`, m]),
  );
}

/**
 * The texture findings of a built bundle:
 * - `texture.size`: square, a power of two, within [TEXTURE_MIN_PX, TEXTURE_MAX_PX];
 * - `texture.mips`: every level down to 1×1, each the right size;
 * - `texture.tangents`: every vertex drawn with a normal-mapped material
 *   carries a tangent.
 */
export function textureFindings(label: string, bundle: Exclude<Bundle, SkeletonClips>): Finding[] {
  const out: Finding[] = [];
  bundle.textures.forEach((t, i) => {
    const name = `${label}: texture ${i} (${t.width}×${t.height} ${t.format})`;
    if (!validTextureSize(t.width, t.height)) {
      out.push(
        finding(
          "texture.size",
          `${name} is not a square power of two from ${TEXTURE_MIN_PX} to ${TEXTURE_MAX_PX} px`,
          `bake it square at a power of two, at most ${TEXTURE_MAX_PX} px`,
        ),
      );
      return;
    }
    const expected = mipCount(t.width);
    const bad = t.levels.findIndex(
      (level, l) => level.byteLength !== Math.max(1, t.width >> l) ** 2 * 4,
    );
    if (t.levels.length !== expected || bad >= 0)
      out.push(
        finding(
          "texture.mips",
          `${name} has ${t.levels.length} of its ${expected} mip levels${bad >= 0 ? `; level ${bad} is the wrong size` : ""}`,
          "re-bake: the bake writes every level down to 1×1",
        ),
      );
  });
  const normalMapped = (m: Material | undefined) => m?.textures?.normal !== undefined;
  for (const [where, mesh] of bundleMeshes(bundle))
    for (const draw of mesh.draws) {
      const material = bundle.materials[draw.material];
      if (!normalMapped(material)) continue;
      let missing = 0;
      for (let i = draw.first; i < draw.first + draw.count; i++) {
        const v = mesh.indices[i] * 4;
        const t = mesh.tangents;
        if (!t || (t[v] === 0 && t[v + 1] === 0 && t[v + 2] === 0)) missing++;
      }
      if (missing)
        out.push(
          finding(
            "texture.tangents",
            `${label} ${where}: material "${material.name}" has a normal map, but ${missing} of its ${draw.count} indexed vertices have no tangent`,
            "give the mesh UVs and export tangents (glTF TANGENT)",
          ),
        );
    }
  return out;
}
