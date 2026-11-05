// @vitest-environment node
// A capture's recorded unit info cards: each frame reads back exactly the
// cards the page showed, in paint order, with their place, opacity and
// leader; a card's contents read once are kept once; the page is laid out
// for one size; a stale or malformed file is refused. The sprite drawer is
// the page's canvas, stood in for here: it is the edge, not the subject.
import { expect, test } from "vitest";
import {
  CardRecording,
  PLACEMENT_LENGTH,
  decodeCardCapture,
  encodeCardFile,
  type SpriteDrawer,
} from "@web/battle/benchmark/cardCapture";
import type { CardPlacement, FontSpec, Primitive } from "@web/battle/present/readoutSnapshot";

const face: FontSpec = {
  style: "normal",
  weight: "700",
  size: 10,
  family: "monospace",
  letterSpacing: 1.4,
  color: [0.45, 0.86, 1, 1],
  shadows: [{ color: [0, 0, 0, 0.95], blur: 2 }],
};

/** Draws each icon and glyph as a 1×1 sprite, in the order asked. */
const drawer: SpriteDrawer = async (svgs, fonts) => {
  let next = svgs.length;
  return {
    sprites: Array.from(
      { length: svgs.length + fonts.reduce((n, f) => n + 2 * f.chars.size, 0) },
      () => ({
        at: [0, 0] as [number, number],
        size: [1, 1] as [number, number],
        offset: [0, 0] as [number, number],
      }),
    ),
    fonts: fonts.map(({ spec, chars }) => ({
      letterSpacing: spec.letterSpacing,
      glyphs: Object.fromEntries(
        [...chars].map((ch) => [ch, { advance: 6, ink: next++, shadow: next++ }]),
      ),
    })),
    atlas: { width: 1, height: 1, pixels: new Uint8Array(4) },
  };
};

/** A card named `card` saying `name`, shown at `at` and `opacity`. */
function card(
  recording: CardRecording,
  card: string,
  name: string,
  at: [number, number],
  opacity: number,
): CardPlacement {
  const part: Primitive[] = [
    {
      rect: {
        box: [-4, -12, 60, 14],
        fill: [0.02, 0.06, 0.09, 0.6],
        border: [0, 0, 0, 0],
        borderPx: 0,
        blurPx: 10,
      },
    },
    { text: { text: name, font: recording.font(face, name), at: [3, 9], opacity: 0.82 } },
    {
      sprite: {
        sprite: recording.svg({ markup: "<svg/>", width: 12, height: 12, filter: "" }),
        at: [50, 0],
        opacity: 1,
      },
    },
  ];
  return {
    card,
    part,
    signature: `${card}|${name}`,
    at,
    opacity,
    leader: {
      ring: [at[0] - 30, at[1] + 40, 2],
      path: [
        [at[0] - 28, at[1] + 40],
        [at[0], at[1] + 14],
        [at[0] + 60, at[1] + 14],
      ],
      style: { width: 1.25, ink: [0.45, 0.86, 1, 0.95], opacity: 0.82, dash: [], shadows: [] },
    },
  };
}

test("each frame reads back the cards the page showed, as it laid them out", async () => {
  const recording = new CardRecording();
  recording.frame(undefined, [1280, 720], 1);
  recording.frame(
    [
      card(recording, "own-1", "TANK", [100, 200], 1),
      card(recording, "contact-2 retiring", "UNKNOWN", [300, 150.123], 0.25),
    ],
    [1280, 720],
    1,
  );
  const { file } = decodeCardCapture(await recording.encode(drawer));
  expect(file.viewport).toEqual([1280, 720]);
  expect(file.frames[0]).toEqual([]);
  const [own, contact] = file.frames[1];
  expect(own).toHaveLength(PLACEMENT_LENGTH);
  expect(file.cards[own[0]]).toBe("own-1");
  expect(file.cards[contact[0]]).toBe("contact-2 retiring");
  expect(contact.slice(3, 6)).toEqual([0.25, 300, 150.12]);
  expect(own.slice(6)).toEqual([70, 240, 2, 72, 240, 100, 214, 160, 214]);
  const text = (c: number[]) => file.parts[c[1]].flatMap((p) => ("text" in p ? [p.text.text] : []));
  expect(text(own)).toEqual(["TANK"]);
  expect(text(contact)).toEqual(["UNKNOWN"]);
  // Every glyph a word needs is in its font.
  const glyphs = file.fonts[0].glyphs;
  for (const ch of "TANKUNOW") expect(glyphs[ch], ch).toBeDefined();
  expect(file.leaders).toEqual([
    { width: 1.25, ink: [0.45, 0.86, 1, 0.95], opacity: 0.82, dash: [], shadows: [] },
  ]);
});

test("a card's contents, read again or written the same, are kept once", async () => {
  const recording = new CardRecording();
  for (let tick = 0; tick < 30; tick++)
    recording.frame(
      [
        card(recording, "own-1", "TANK", [100 + tick, 200], 1),
        card(recording, "enemy-21", "BMP", [400, 200 - tick], 1),
      ],
      [1280, 720],
      1,
    );
  // A new signature that lays out the same contents adds none either.
  const again = card(recording, "own-1", "TANK", [0, 0], 1);
  recording.frame([{ ...again, signature: "rebuilt" }], [1280, 720], 1);
  const { file } = decodeCardCapture(await recording.encode(drawer));
  expect(file.parts).toHaveLength(2);
  expect(file.parts.map((p) => p.flatMap((q) => ("text" in q ? [q.text.text] : [])))).toEqual([
    ["TANK"],
    ["BMP"],
  ]);
  expect(file.frames.at(-1)![0][1]).toBe(file.frames[0][0][1]);
});

test("cards are laid out for one page size", () => {
  const recording = new CardRecording();
  recording.frame([], [1280, 720], 1);
  expect(() => recording.frame([], [1920, 1080], 1)).toThrow(/1280×720 and is now 1920×1080/);
});

test("a stale or malformed file is refused", async () => {
  const recording = new CardRecording();
  recording.frame([card(recording, "own-1", "TANK", [10, 10], 1)], [1280, 720], 1);
  const good = await recording.encode(drawer);
  const { file, atlas } = decodeCardCapture(good);
  const refused = (bytes: Uint8Array, says: RegExp) =>
    expect(() => decodeCardCapture(bytes)).toThrow(says);
  const edited = (edit: (f: typeof file) => void) => {
    const copy = structuredClone(file);
    edit(copy);
    return encodeCardFile(copy, atlas);
  };
  refused(Uint8Array.from([0x58, ...good.slice(1)]), /not a recorded card file/);
  const version = good.slice();
  version[4] = 2;
  refused(version, /version 2/);
  refused(good.slice(0, good.length - 2), /ends early/);
  refused(Uint8Array.from([...good, 0]), /past the atlas/);
  refused(
    edited((f) => {
      f.frames[0][0][1] = 7;
    }),
    /frame 0 names part 7 of 1/,
  );
  refused(
    edited((f) => {
      const text = f.parts[0].find((p) => "text" in p);
      if (text && "text" in text) text.text.text = "TANKZ";
    }),
    /writes Z, which its font lacks/,
  );
  refused(
    edited((f) => {
      f.sprites[0].at = [5, 5];
    }),
    /sprite 0 lies past the 1×1 atlas/,
  );
  refused(
    edited((f) => {
      f.frames[0][0] = f.frames[0][0].slice(0, 3);
    }),
    /a card of 3 numbers/,
  );
});
