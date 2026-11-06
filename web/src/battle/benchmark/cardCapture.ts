// The unit info cards a capture drew over its battle, frame by frame, for a
// second client to draw again (`native/client/src/cards.rs` reads it): the
// live readout layer's cards as it laid them out (`readoutSnapshot.ts`),
// each one's contents, place, opacity and leader, with the pixels of their
// words and icons (`cardSprites.ts`). Written beside `capture.json` as
// `cards.bin`; the capture's frame records are unchanged, and a capture
// without the manifest's `cards` layer has none. Little-endian:
//
//   "BCRD", u32 version (1), u32 n, n bytes of UTF-8 JSON (`CardFile`),
//   u32 atlas width, u32 atlas height, width × height premultiplied RGBA8
//
// Each distinct card contents (`parts`) and leader stroke (`leaders`) is
// kept once. A frame's cards are back to front, each
//   [card, part, leader (-1 for none), opacity, x, y,
//    ring x, ring y, ring r, a x, a y, b x, b y, c x, c y]
// in CSS pixels of a `viewport`-sized page: the card's top-left, and its
// leader's ring and the three points of its line.
//
// The format changes in place: a stale file is refused, never migrated.
import type {
  CardPlacement,
  FontSpec,
  LeaderStyle,
  PartCache,
  Primitive,
  SpriteRequests,
  SvgSpec,
} from "../present/readoutSnapshot";
// Plain Node decodes the file (the scene harness), so this imports nothing
// with a runtime of its own beyond the sprite drawer, which runs only in a page.
import { drawSprites, type Atlas, type FontGlyphs, type SpriteRect } from "./cardSprites.ts";

/** The file a scene's recorded cards are written to, beside its capture. */
export const CARD_CAPTURE_FILE = "cards.bin";

const MAGIC = [0x42, 0x43, 0x52, 0x44]; // "BCRD"
const VERSION = 1;
/** Numbers in one frame's card. */
export const PLACEMENT_LENGTH = 15;

/** The JSON half of `cards.bin`. */
export interface CardFile {
  viewport: [number, number];
  /** Atlas pixels a CSS pixel. */
  scale: number;
  /** Each card's name (`CardPlacement.card`): whose, and which. */
  cards: string[];
  fonts: FontGlyphs[];
  sprites: SpriteRect[];
  leaders: LeaderStyle[];
  parts: Primitive[][];
  frames: number[][][];
}

/** How the recording draws its sprites; the page's canvas unless a test
 *  stands in for it. */
export type SpriteDrawer = typeof drawSprites;

const round = (v: number) => Math.round(v * 100) / 100;

/** A scene's cards as its frames drew them. */
export class CardRecording implements SpriteRequests {
  /** Each card's contents last read, while its markup holds. */
  readonly cache: PartCache = new WeakMap();
  private readonly fonts: { spec: FontSpec; chars: Set<string> }[] = [];
  private readonly fontIndex = new Map<string, number>();
  private readonly svgs: SvgSpec[] = [];
  private readonly svgIndex = new Map<string, number>();
  private readonly parts: Primitive[][] = [];
  private readonly partBySignature = new Map<string, number>();
  private readonly partByJson = new Map<string, number>();
  private readonly leaders: LeaderStyle[] = [];
  private readonly leaderIndex = new Map<string, number>();
  private readonly cards: string[] = [];
  private readonly cardIndex = new Map<string, number>();
  readonly frames: number[][][] = [];
  private viewport: [number, number] | null = null;
  private scale = 1;
  private encoded: Promise<Uint8Array> | null = null;

  font(spec: FontSpec, text: string): number {
    const key = JSON.stringify(spec);
    let index = this.fontIndex.get(key);
    if (index === undefined) {
      index = this.fonts.push({ spec, chars: new Set() }) - 1;
      this.fontIndex.set(key, index);
    }
    for (const ch of text) this.fonts[index].chars.add(ch);
    return index;
  }

  svg(spec: SvgSpec): number {
    const key = JSON.stringify(spec);
    let index = this.svgIndex.get(key);
    if (index === undefined) {
      index = this.svgs.push(spec) - 1;
      this.svgIndex.set(key, index);
    }
    return index;
  }

  private indexed<T>(list: T[], index: Map<string, number>, key: string, value: T): number {
    let at = index.get(key);
    if (at === undefined) {
      at = list.push(value) - 1;
      index.set(key, at);
    }
    return at;
  }

  private partOf(card: CardPlacement): number {
    const known = this.partBySignature.get(card.signature);
    if (known !== undefined) return known;
    const at = this.indexed(this.parts, this.partByJson, JSON.stringify(card.part), card.part);
    this.partBySignature.set(card.signature, at);
    return at;
  }

  /** Note one drawn frame: the cards the page showed on a `viewport`-sized
   *  page at `scale` device pixels a CSS pixel, or none before it shows any. */
  frame(cards: readonly CardPlacement[] | undefined, viewport: [number, number], scale: number) {
    if (this.viewport && (this.viewport[0] !== viewport[0] || this.viewport[1] !== viewport[1]))
      throw new Error(
        `the page was ${this.viewport.join("×")} and is now ${viewport.join("×")}: cards are laid out for one size`,
      );
    this.viewport ??= viewport;
    this.scale = scale;
    this.frames.push(
      (cards ?? []).map((c) => {
        const card = this.indexed(this.cards, this.cardIndex, c.card, c.card);
        const leader = c.leader
          ? this.indexed(
              this.leaders,
              this.leaderIndex,
              JSON.stringify(c.leader.style),
              c.leader.style,
            )
          : -1;
        const geometry = c.leader ? [...c.leader.ring, ...c.leader.path.flat()] : Array(9).fill(0);
        return [
          card,
          this.partOf(c),
          leader,
          round(c.opacity),
          ...[...c.at, ...geometry].map(round),
        ];
      }),
    );
    this.encoded = null;
  }

  /** The `cards.bin` bytes, its sprites drawn by `draw`; once per recorded
   *  frame count. */
  encode(draw: SpriteDrawer = drawSprites): Promise<Uint8Array> {
    this.encoded ??= this.write(draw);
    return this.encoded;
  }

  private async write(draw: SpriteDrawer): Promise<Uint8Array> {
    const drawn = await draw(this.svgs, this.fonts, this.scale);
    const file: CardFile = {
      viewport: this.viewport ?? [0, 0],
      scale: this.scale,
      cards: this.cards,
      fonts: drawn.fonts,
      sprites: drawn.sprites,
      leaders: this.leaders,
      parts: this.parts,
      frames: this.frames,
    };
    return encodeCardFile(file, drawn.atlas);
  }
}

/** The bytes of `file` and its `atlas`. */
export function encodeCardFile(file: CardFile, atlas: Atlas): Uint8Array {
  const json = new TextEncoder().encode(JSON.stringify(file));
  const bytes = new Uint8Array(4 + 4 + 4 + json.length + 8 + atlas.pixels.length);
  const view = new DataView(bytes.buffer);
  bytes.set(MAGIC, 0);
  view.setUint32(4, VERSION, true);
  view.setUint32(8, json.length, true);
  bytes.set(json, 12);
  let at = 12 + json.length;
  view.setUint32(at, atlas.width, true);
  view.setUint32(at + 4, atlas.height, true);
  at += 8;
  bytes.set(atlas.pixels, at);
  return bytes;
}

/** Throws naming the first reference in `file` that names nothing. */
function check(file: CardFile, atlas: Atlas) {
  const { sprites, fonts, parts, leaders, cards } = file;
  sprites.forEach((s, i) => {
    if (s.at[0] + s.size[0] > atlas.width || s.at[1] + s.size[1] > atlas.height)
      throw new Error(`sprite ${i} lies past the ${atlas.width}×${atlas.height} atlas`);
  });
  const sprite = (i: number | null, at: string) => {
    if (i !== null && !(i >= 0 && i < sprites.length))
      throw new Error(`${at} names sprite ${i} of ${sprites.length}`);
  };
  fonts.forEach((f, i) => {
    for (const [ch, g] of Object.entries(f.glyphs)) {
      sprite(g.ink, `font ${i} glyph ${ch}`);
      sprite(g.shadow, `font ${i} glyph ${ch}`);
    }
  });
  parts.forEach((part, p) =>
    part.forEach((primitive) => {
      if ("sprite" in primitive) sprite(primitive.sprite.sprite, `part ${p}`);
      if ("text" in primitive) {
        const font = fonts[primitive.text.font];
        if (!font)
          throw new Error(`part ${p} names font ${primitive.text.font} of ${fonts.length}`);
        for (const ch of primitive.text.text)
          if (!font.glyphs[ch]) throw new Error(`part ${p} writes ${ch}, which its font lacks`);
      }
    }),
  );
  file.frames.forEach((frame, f) =>
    frame.forEach((c) => {
      if (c.length !== PLACEMENT_LENGTH)
        throw new Error(`frame ${f} has a card of ${c.length} numbers, not ${PLACEMENT_LENGTH}`);
      if (!(c[0] >= 0 && c[0] < cards.length)) throw new Error(`frame ${f} names card ${c[0]}`);
      if (!(c[1] >= 0 && c[1] < parts.length))
        throw new Error(`frame ${f} names part ${c[1]} of ${parts.length}`);
      if (!(c[2] >= -1 && c[2] < leaders.length))
        throw new Error(`frame ${f} names leader ${c[2]} of ${leaders.length}`);
    }),
  );
}

/** A recorded scene's cards. Refuses a file that is not exactly the format
 *  or names what it does not hold. */
export function decodeCardCapture(bytes: Uint8Array): { file: CardFile; atlas: Atlas } {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const need = (at: number, n: number) => {
    if (at + n > bytes.byteLength) throw new Error("the card file ends early");
  };
  if (bytes.byteLength < 4 || MAGIC.some((b, i) => bytes[i] !== b))
    throw new Error("not a recorded card file");
  need(4, 8);
  const version = view.getUint32(4, true);
  if (version !== VERSION) throw new Error(`card file version ${version}, not ${VERSION}`);
  const length = view.getUint32(8, true);
  need(12, length + 8);
  const file = JSON.parse(new TextDecoder().decode(bytes.subarray(12, 12 + length))) as CardFile;
  let at = 12 + length;
  const width = view.getUint32(at, true);
  const height = view.getUint32(at + 4, true);
  at += 8;
  need(at, width * height * 4);
  const atlas = { width, height, pixels: bytes.slice(at, at + width * height * 4) };
  at += width * height * 4;
  if (at !== bytes.byteLength) throw new Error("bytes past the atlas");
  check(file, atlas);
  return { file, atlas };
}
