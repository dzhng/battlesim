// The filmed cards are the player's cards, and only those (`cards.bin`): a
// battle's cards rendered as the battle view renders them,
// `<ReadoutLayer {...battleReadouts(session)} />`, placed by that layer, read back as the capture reads them
// (`readoutSnapshot`), recorded and decoded. The side's own unit and an
// ally's, an identified enemy, a contact known only by ear, and a lost
// contact remembered as it fades, still saying what the side last knew,
// then gone. An enemy the side cannot see has no card, and its name is
// nowhere in the file. Every recorded card says what the player's card
// says. jsdom has no layout and no canvas: a run of words' box and a face's
// ascent are stood in for, which place the words and never choose them.
import { cleanup, render } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { shippedRules } from "@web/shippedRules";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import { gameContactStyle } from "@apps/battle-lab/src/gameFog";
import { battleReadouts } from "@apps/battle-lab/src/BattleReadouts";
import type { BattleSession } from "@apps/battle-lab/src/useBattleSession";
import {
  CardRecording,
  decodeCardCapture,
  type CardFile,
  type SpriteDrawer,
} from "@web/battle/benchmark/cardCapture";
import { readoutSnapshot } from "@web/battle/present/readoutSnapshot";
import { ContactPresentation } from "@web/battle/present/contactPresentation";
import { contactPanel, enemyPanel, type PanelRules } from "@web/battle/present/panelRows";
import {
  CARD_FADE_START_M,
  ReadoutLayer,
  type ReadoutLayerHandle,
} from "@web/battle/present/readouts";
import type { ContactView, IdentifiedView, OwnUnitView } from "@web/battle/sim/observationView";
import { TEST_CATALOG, UNITS, WithTestCatalog } from "./catalog";

const RULES = shippedRules as unknown as PanelRules;
const name = (kind: string) => UNITS.type(kind).name.toUpperCase();
const OWNER = "BLUE 2 · AI";

const own: OwnUnitView[] = [
  specimenUnit(TEST_CATALOG, "test_jeep", { id: 1, position: [200, 200, 0] }),
  // An ally's squad, on the side's panel naming its owner.
  specimenUnit(TEST_CATALOG, "test_rifle", { id: 2, position: [400, 200, 0] }),
];
const identified = [{ id: 21, kind: "test_tank", position: [600, 200, 0] } as IdentifiedView];
const contact = (id: number, patch: Partial<ContactView>) =>
  ({
    id,
    source: "heard",
    center: [0, 0, 0],
    layer: "ground",
    aloft: false,
    radius: 20,
    evidenceTick: 90,
    expiresTick: 10_000,
    kind: null,
    heard: [],
    ...patch,
  }) as ContactView;
/** A squad the side saw, now out of sight: its last sighting. */
const remembered = contact(5, { source: "last_seen", kind: "test_rifle", center: [300, 450, 0] });
/** An enemy the side cannot see at all. It is in no report but this one,
 *  a sound with no type. */
const HIDDEN = { id: 99, kind: "test_apc" };
const heard = contact(6, { center: [600, 450, 0] });

/** The ticks filmed: both contacts reported; contact 5 lost; half its fade
 *  later; past its fade. */
const LIVE = 100;
const FADE_TICKS = gameContactStyle.fade_s * RULES.tick_hz;
const HALF = Math.round(FADE_TICKS / 2);
const FRAMES = [
  { tick: LIVE, contacts: [remembered, heard] },
  { tick: LIVE + 1, contacts: [heard] },
  { tick: LIVE + 1 + HALF, contacts: [heard] },
  { tick: LIVE + 2 + Math.ceil(FADE_TICKS), contacts: [heard] },
];

/** Each icon and glyph as a 1×1 sprite: the page's canvas, stood in for. */
const drawer: SpriteDrawer = async (svgs, fonts) => {
  let next = svgs.length;
  const glyphs = fonts.reduce((n, f) => n + 2 * f.chars.size, 0);
  return {
    sprites: Array.from({ length: svgs.length + glyphs }, () => ({
      at: [0, 0] as [number, number],
      size: [1, 1] as [number, number],
      offset: [0, 0] as [number, number],
    })),
    fonts: fonts.map(({ spec, chars }) => ({
      letterSpacing: spec.letterSpacing,
      glyphs: Object.fromEntries(
        [...chars].map((ch) => [ch, { advance: 6, ink: next++, shadow: next++ }]),
      ),
    })),
    atlas: { width: 1, height: 1, pixels: new Uint8Array(4) },
  };
};

const rangeBox = Object.getOwnPropertyDescriptor(Range.prototype, "getBoundingClientRect");
beforeEach(() => {
  // What layout and a canvas would answer: a run of words has a box, a face an ascent.
  Object.defineProperty(Range.prototype, "getBoundingClientRect", {
    configurable: true,
    value: () => ({ left: 0, top: 0, right: 40, bottom: 10, width: 40, height: 10, x: 0, y: 0 }),
  });
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    font: "",
    measureText: () => ({ fontBoundingBoxAscent: 8 }),
  } as never);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  if (rangeBox) Object.defineProperty(Range.prototype, "getBoundingClientRect", rangeBox);
  else delete (Range.prototype as { getBoundingClientRect?: unknown }).getBoundingClientRect;
});

const squeezed = (s: string) => s.replace(/\s+/g, "");
/** The words a card shows the player: its text, not its icons' markup
 *  (an icon is filmed as a picture). */
function readable(card: HTMLElement): string {
  const walker = document.createTreeWalker(card, NodeFilter.SHOW_TEXT);
  let out = "";
  for (let t = walker.nextNode(); t; t = walker.nextNode())
    if (!t.parentElement?.closest("svg")) out += (t as Text).data;
  return squeezed(out);
}
const wordsOf = (file: CardFile, part: number) =>
  file.parts[part].flatMap((p) => ("text" in p ? [p.text.text] : [])).join(" ");

/** The battle filmed over `FRAMES` as the graphics test films it: per frame,
 *  each recorded card's words and opacity by its name, and the words of
 *  every card the page showed. */
async function filmed() {
  const presented = new ContactPresentation(RULES.tick_hz, gameContactStyle.fade_s);
  const handle = createRef<ReadoutLayerHandle>();
  const recording = new CardRecording();
  // A commander's height: no card faded by the close camera.
  const camera = { ...gameCamera.opening(), distance: CARD_FADE_START_M + 10 };
  const allies = new Map([[2, OWNER]]);
  const shown: string[][] = [];
  const readoutsAt = ({ tick, contacts }: (typeof FRAMES)[number]) => {
    const session = {
      sim: { observation: { own, identified, tick } },
      contacts: presented.update(contacts, tick),
      rules: RULES,
      control: { selected: [] },
      readouts: handle,
      ghostReadouts: [],
      allies,
    } as unknown as BattleSession;
    return <ReadoutLayer {...battleReadouts(session)} />;
  };
  const view = render(readoutsAt(FRAMES[0]), { wrapper: WithTestCatalog });
  for (const [i, frame] of FRAMES.entries()) {
    if (i > 0) view.rerender(readoutsAt(frame));
    // Each card hangs where its unit stands on screen; nobody points at a film.
    handle.current!.place((x, y) => [x, y], camera, {}, null, false, null);
    const layer = view.container.querySelector<HTMLElement>("[data-testid=readouts]")!;
    recording.frame(
      readoutSnapshot(layer, recording, recording.cache),
      [window.innerWidth, window.innerHeight],
      window.devicePixelRatio,
    );
    shown.push([...layer.querySelectorAll<HTMLElement>(".ro-callout > .ro-unit")].map(readable));
  }
  const { file } = decodeCardCapture(await recording.encode(drawer));
  return {
    file,
    shown,
    frames: file.frames.map(
      (frame) =>
        new Map(
          frame.map((c): [string, { words: string; opacity: number }] => [
            file.cards[c[0]],
            { words: wordsOf(file, c[1]), opacity: c[3] },
          ]),
        ),
    ),
  };
}

test("the film shows the side's own and an ally's units, the enemy it identified, and its contacts", async () => {
  const [live] = (await filmed()).frames;
  expect([...live.keys()].sort()).toEqual([
    "ally-2",
    "contact-5",
    "contact-6",
    "enemy-21",
    "own-1",
  ]);
  expect(live.get("own-1")!.words).toContain(name("test_jeep"));
  expect(live.get("ally-2")!.words).toContain(name("test_rifle"));
  expect(live.get("ally-2")!.words).toContain(OWNER);
  expect(live.get("enemy-21")!.words).toContain(enemyPanel(UNITS, "test_tank", RULES).name);
  // A contact says what its report says: a remembered type, or that it is unknown.
  expect(live.get("contact-5")!.words).toContain(contactPanel(UNITS, remembered, LIVE, RULES).name);
  expect(live.get("contact-6")!.words).toContain(contactPanel(UNITS, heard, LIVE, RULES).name);
  for (const card of live.values()) expect(card.opacity).toBe(1);
});

test("an enemy the side cannot see, or only hears, is never named on film", async () => {
  const { file, frames } = await filmed();
  for (const frame of frames) {
    expect([...frame.keys()].filter((card) => card.endsWith(`-${HIDDEN.id}`))).toEqual([]);
    expect(frame.get("contact-6")!.words).not.toContain(name(HIDDEN.kind));
  }
  for (let part = 0; part < file.parts.length; part++)
    expect(wordsOf(file, part)).not.toContain(name(HIDDEN.kind));
});

test("a lost contact is filmed fading as the player sees it, saying what the side last knew, then gone", async () => {
  const [live, lost, half, gone] = (await filmed()).frames;
  const knew = live.get("contact-5")!;
  // Its card is the retiring one, never a live one beside it.
  expect(lost.has("contact-5")).toBe(false);
  expect(lost.get("contact-5 retiring")!.opacity).toBe(1);
  expect(half.get("contact-5 retiring")!.opacity).toBeCloseTo(1 - HALF / FADE_TICKS, 2);
  // What it says holds as it fades; only the age it counts moves on.
  for (const frame of [lost, half]) {
    const words = frame.get("contact-5 retiring")!.words;
    expect(words).toContain(name(remembered.kind!));
    expect(words.replace(/\d+/g, "#")).toBe(knew.words.replace(/\d+/g, "#"));
  }
  expect([...gone.keys()].filter((card) => card.startsWith("contact-5"))).toEqual([]);
  expect(gone.has("contact-6")).toBe(true);
});

test("every filmed card says exactly what the player's card says", async () => {
  const { frames, shown } = await filmed();
  frames.forEach((frame, i) => {
    const recorded = [...frame.values()].map((c) => squeezed(c.words)).sort();
    expect(recorded, `frame ${i}`).toEqual([...shown[i]].sort());
  });
});
