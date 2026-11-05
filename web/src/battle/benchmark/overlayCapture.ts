// The indicators a capture drew over its battle, frame by frame, for a second
// client to draw again (`native/client/src/overlays.rs` reads it): the live
// battle view's overlay layers (`battleOverlayParts`), as the page combined
// and uploaded them. Written beside `capture.json` as `overlays.bin`; the
// capture's frame records are unchanged, and a capture without the
// manifest's `overlays` layer has none.
//
// The page rebuilds its overlay on every publication and the map's border
// alone is over a megabyte, so each distinct part is kept once and each
// frame names the parts it concatenates. Little-endian:
//
//   "BOVL", u32 version (1), u32 frames, u32 parts
//   parts × { u32 floats per channel (opaque, translucent, painted,
//             paintedMarching), each a multiple of ten; those f32s in order }
//   frames × { u32 count; count × u32 part index }
//
// The format changes in place: a stale file is refused, never migrated.
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";

/** The file a scene's recorded overlays are written to, beside its capture. */
export const OVERLAY_CAPTURE_FILE = "overlays.bin";

const MAGIC = [0x42, 0x4f, 0x56, 0x4c]; // "BOVL"
const VERSION = 1;
/** Floats per vertex: position, normal and colour. */
const VERTEX_FLOATS = 10;
const CHANNELS = ["opaque", "translucent", "painted", "paintedMarching"] as const;
const NO_FLOATS = new Float32Array(0);

/** One overlay layer: what it adds to each channel. */
export type OverlayPart = Partial<WorldMeshes>;
type Channels = readonly Float32Array[];

const channelsOf = (part: OverlayPart): Channels => CHANNELS.map((c) => part[c] ?? NO_FLOATS);

/** FNV-1a over a part's channel lengths and float bits. */
function hashOf(channels: Channels): number {
  let h = 0x811c9dc5;
  const mix = (word: number) => {
    h = Math.imul(h ^ word, 0x01000193) >>> 0;
  };
  for (const floats of channels) {
    mix(floats.length);
    for (const word of new Uint32Array(floats.buffer, floats.byteOffset, floats.length)) mix(word);
  }
  return h;
}

const sameFloats = (a: Float32Array, b: Float32Array) => {
  if (a.length !== b.length) return false;
  const [x, y] = [a, b].map((f) => new Uint32Array(f.buffer, f.byteOffset, f.length));
  return x.every((word, i) => word === y[i]);
};
const sameChannels = (a: Channels, b: Channels) => a.every((floats, i) => sameFloats(floats, b[i]));

/** A scene's overlays as its frames draw them. A part seen before (the same
 *  object, or the same floats) is kept once. */
export class OverlayRecording {
  private readonly parts: Channels[] = [];
  private readonly byObject = new Map<OverlayPart, number>();
  private readonly byHash = new Map<number, number[]>();
  readonly frames: number[][] = [];
  private encoded: Uint8Array | null = null;

  /** Note one drawn frame: the parts its overlay combined, or none before
   *  the page has built one. */
  frame(parts: readonly OverlayPart[] | undefined): void {
    this.frames.push((parts ?? []).map((part) => this.partIndex(part)));
    this.encoded = null;
  }

  private partIndex(part: OverlayPart): number {
    const known = this.byObject.get(part);
    if (known !== undefined) return known;
    const channels = channelsOf(part);
    for (const floats of channels)
      if (floats.length % VERTEX_FLOATS !== 0)
        throw new Error(`an overlay channel of ${floats.length} floats is not whole vertices`);
    const hash = hashOf(channels);
    const same = this.byHash.get(hash) ?? [];
    let index = same.find((i) => sameChannels(this.parts[i], channels));
    if (index === undefined) {
      index = this.parts.push(channels) - 1;
      this.byHash.set(hash, [...same, index]);
    }
    this.byObject.set(part, index);
    return index;
  }

  /** The `overlays.bin` bytes, written once per recorded frame count. */
  encode(): Uint8Array {
    this.encoded ??= this.write();
    return this.encoded;
  }

  private write(): Uint8Array {
    const words =
      4 +
      this.parts.reduce((n, p) => n + 4 + p.reduce((m, f) => m + f.length, 0), 0) +
      this.frames.reduce((n, f) => n + 1 + f.length, 0);
    const bytes = new Uint8Array(words * 4);
    const view = new DataView(bytes.buffer);
    let at = 0;
    const u32 = (value: number) => {
      view.setUint32(at, value, true);
      at += 4;
    };
    bytes.set(MAGIC, 0);
    at = 4;
    u32(VERSION);
    u32(this.frames.length);
    u32(this.parts.length);
    for (const channels of this.parts) {
      for (const floats of channels) u32(floats.length);
      for (const floats of channels)
        for (const value of floats) {
          view.setFloat32(at, value, true);
          at += 4;
        }
    }
    for (const frame of this.frames) {
      u32(frame.length);
      for (const index of frame) u32(index);
    }
    return bytes;
  }
}

/** A recorded scene's overlays: its parts' channels and each frame's part
 *  indices. Refuses a file that is not exactly the format. */
export function decodeOverlayCapture(bytes: Uint8Array): {
  parts: Float32Array[][];
  frames: number[][];
} {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let at = 0;
  const u32 = () => {
    if (at + 4 > bytes.byteLength) throw new Error("the overlay file ends early");
    const value = view.getUint32(at, true);
    at += 4;
    return value;
  };
  if (bytes.byteLength < 4 || MAGIC.some((b, i) => bytes[i] !== b))
    throw new Error("not a recorded overlay file");
  at = 4;
  const version = u32();
  if (version !== VERSION) throw new Error(`overlay file version ${version}, not ${VERSION}`);
  const frameCount = u32();
  const partCount = u32();
  const parts: Float32Array[][] = [];
  for (let p = 0; p < partCount; p++) {
    const counts = CHANNELS.map(() => u32());
    if (counts.some((n) => n % VERTEX_FLOATS !== 0))
      throw new Error(`part ${p}: a channel is not whole ${VERTEX_FLOATS}-float vertices`);
    parts.push(
      counts.map((n) => {
        if (at + n * 4 > bytes.byteLength) throw new Error("the overlay file ends early");
        const floats = new Float32Array(n);
        for (let i = 0; i < n; i++) floats[i] = view.getFloat32(at + i * 4, true);
        at += n * 4;
        return floats;
      }),
    );
  }
  const frames: number[][] = [];
  for (let f = 0; f < frameCount; f++) {
    const count = u32();
    if (at + count * 4 > bytes.byteLength) throw new Error("the overlay file ends early");
    const named = Array.from({ length: count }, () => u32());
    if (named.some((i) => i >= partCount))
      throw new Error(`frame ${f} names a part past the ${partCount} recorded`);
    frames.push(named);
  }
  if (at !== bytes.byteLength) throw new Error("bytes past the last frame");
  return { parts, frames };
}
