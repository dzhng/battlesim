// The pixels of a capture's card words and icons (`cardCapture.ts`), drawn
// by the browser that laid the cards out, with the faces, colours and
// shadows the page drew them in: each glyph's ink and its shadows as two
// sprites (so a run's shadows all paint under its ink, as CSS paints
// `text-shadow`), each icon with its drop shadows. Packed into one
// premultiplied RGBA atlas, rows top first.
import type { FontSpec, ShadowSpec, SvgSpec } from "../present/readoutSnapshot";

/** A sprite's atlas rectangle (atlas pixels) and where its top-left sits
 *  from the point it is drawn at (a glyph's pen on its baseline, an icon's
 *  box), in CSS pixels. */
export interface SpriteRect {
  at: [number, number];
  size: [number, number];
  offset: [number, number];
}

/** One face's glyphs: each character's advance (CSS pixels, before the
 *  face's letter spacing) and its ink and shadow sprites (null where it
 *  has none: a space, a face without shadows). */
export interface FontGlyphs {
  letterSpacing: number;
  glyphs: Record<string, { advance: number; ink: number | null; shadow: number | null }>;
}

export interface Atlas {
  width: number;
  height: number;
  /** Premultiplied RGBA8. */
  pixels: Uint8Array;
}

interface Drawn {
  width: number;
  height: number;
  data: Uint8ClampedArray;
  offset: [number, number];
}

const ATLAS_WIDTH = 2048;
const ATLAS_MAX = 8192;
const GAP = 1;
/** How far a shadow's blur reaches, in its CSS blur lengths. */
const BLUR_REACH = 1.5;

const css = ([r, g, b, a]: readonly number[]) =>
  `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`;

function scratch(width: number, height: number) {
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, width);
  canvas.height = Math.max(1, height);
  return canvas.getContext("2d", { willReadFrequently: true })!;
}

const faceAt = (spec: FontSpec, scale: number) =>
  `${spec.style} ${spec.weight} ${spec.size * scale}px ${spec.family}`;

/** A glyph's ink and its shadows, apart, at `scale` atlas pixels a CSS pixel. */
function glyph(spec: FontSpec, ch: string, scale: number) {
  const probe = scratch(1, 1);
  probe.font = faceAt(spec, scale);
  const m = probe.measureText(ch);
  const advance = m.width / scale;
  const left = Math.ceil(m.actualBoundingBoxLeft);
  const right = Math.ceil(m.actualBoundingBoxRight);
  const up = Math.ceil(m.actualBoundingBoxAscent);
  const down = Math.ceil(m.actualBoundingBoxDescent);
  if (left + right <= 0 || up + down <= 0) return { advance, ink: null, shadow: null };
  const reach = Math.max(0, ...spec.shadows.map((s) => s.blur)) * BLUR_REACH * scale;
  const pad = Math.ceil(reach) + 2;
  const width = left + right + 2 * pad;
  const height = up + down + 2 * pad;
  const pen: [number, number] = [pad + left, pad + up];
  const offset: [number, number] = [-pen[0] / scale, -pen[1] / scale];
  const draw = (shadows: readonly ShadowSpec[] | null): Drawn => {
    const ctx = scratch(width, height);
    ctx.font = faceAt(spec, scale);
    if (shadows) {
      // Drawn off the canvas so only its shadow lands; the first shadow last,
      // on top, as CSS paints them.
      const away = width + 2 * reach + 16;
      ctx.fillStyle = "#000";
      for (const s of [...shadows].reverse()) {
        ctx.shadowColor = css(s.color);
        ctx.shadowBlur = s.blur * scale;
        ctx.shadowOffsetX = away;
        ctx.fillText(ch, pen[0] - away, pen[1]);
      }
    } else {
      ctx.fillStyle = css(spec.color);
      ctx.fillText(ch, pen[0], pen[1]);
    }
    return { width, height, data: ctx.getImageData(0, 0, width, height).data, offset };
  };
  return { advance, ink: draw(null), shadow: spec.shadows.length ? draw(spec.shadows) : null };
}

/** An icon drawn with its filter, at `scale`. */
async function icon(spec: SvgSpec, scale: number): Promise<Drawn> {
  const image = new Image();
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(spec.markup)}`;
  await image.decode();
  // Each drop shadow's last length, before its closing bracket, is its blur.
  const blurs = [...spec.filter.matchAll(/(-?\d*\.?\d+)px\s*\)/g)].map((m) => parseFloat(m[1]));
  const pad = Math.ceil(blurs.reduce((a, b) => a + b, 0) * 3 * scale) + 2;
  const w = Math.ceil(spec.width * scale);
  const h = Math.ceil(spec.height * scale);
  const width = w + 2 * pad;
  const height = h + 2 * pad;
  const ctx = scratch(width, height);
  // The filter's lengths are CSS pixels; the atlas draws `scale` to one.
  ctx.filter = spec.filter
    ? spec.filter.replace(/(-?\d*\.?\d+)px/g, (_, v: string) => `${parseFloat(v) * scale}px`)
    : "none";
  ctx.drawImage(image, pad, pad, spec.width * scale, spec.height * scale);
  return {
    width,
    height,
    data: ctx.getImageData(0, 0, width, height).data,
    offset: [-pad / scale, -pad / scale],
  };
}

/** Every icon (sprites 0…, in order) and every glyph each face drew (after
 *  them), drawn and packed at `scale`. */
export async function drawSprites(
  svgs: readonly SvgSpec[],
  fonts: readonly { spec: FontSpec; chars: ReadonlySet<string> }[],
  scale: number,
): Promise<{ sprites: SpriteRect[]; fonts: FontGlyphs[]; atlas: Atlas }> {
  const drawn: Drawn[] = [];
  for (const spec of svgs) drawn.push(await icon(spec, scale));
  const glyphs = fonts.map(({ spec, chars }) => {
    const out: FontGlyphs = { letterSpacing: spec.letterSpacing, glyphs: {} };
    for (const ch of [...chars].sort()) {
      const g = glyph(spec, ch, scale);
      const index = (d: Drawn | null) => (d ? drawn.push(d) - 1 : null);
      out.glyphs[ch] = { advance: g.advance, ink: index(g.ink), shadow: index(g.shadow) };
    }
    return out;
  });
  // Shelves, tallest first.
  const order = drawn.map((_, i) => i).sort((a, b) => drawn[b].height - drawn[a].height);
  const width = Math.max(ATLAS_WIDTH, ...drawn.map((d) => d.width + GAP));
  const at: [number, number][] = [];
  let [x, y, shelf] = [0, 0, 0];
  for (const i of order) {
    const d = drawn[i];
    if (x + d.width > width) [x, y, shelf] = [0, y + shelf + GAP, 0];
    at[i] = [x, y];
    x += d.width + GAP;
    shelf = Math.max(shelf, d.height);
  }
  const height = Math.max(1, y + shelf);
  if (height > ATLAS_MAX || width > ATLAS_MAX)
    throw new Error(`the card sprites need a ${width}×${height} atlas, past ${ATLAS_MAX}`);
  const pixels = new Uint8Array(width * height * 4);
  drawn.forEach((d, i) => {
    const [ax, ay] = at[i];
    for (let row = 0; row < d.height; row++)
      for (let col = 0; col < d.width; col++) {
        const from = (row * d.width + col) * 4;
        const to = ((ay + row) * width + ax + col) * 4;
        const a = d.data[from + 3];
        for (let c = 0; c < 3; c++) pixels[to + c] = Math.round((d.data[from + c] * a) / 255);
        pixels[to + 3] = a;
      }
  });
  return {
    sprites: drawn.map((d, i) => ({ at: at[i], size: [d.width, d.height], offset: d.offset })),
    fonts: glyphs,
    atlas: { width, height, pixels },
  };
}
