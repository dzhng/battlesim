// @vitest-environment node
// The info cards' screen layout, in two planes: high-air cards make room for
// each other above; ground and low-air cards make room for each other below,
// stepping off the high-air cards when they can. In each plane leaders are
// short and never cross. A card that would sit near the screen's edge hides.
import { expect, test } from "vitest";
import { segment2 } from "math/shapes";
import {
  layoutReadouts,
  leaderOf,
  overlaps,
  readoutPaintOrder,
  type CardPlacement,
  type LayoutCard,
  type ReadoutRect,
} from "../src/battle/present/readoutLayout";
import { layoutReadoutDetails, type DetailCard } from "../src/battle/present/readoutDetails";
import {
  cardZoomOpacity,
  CARD_FADE_END_M,
  CARD_FADE_START_M,
} from "../src/battle/present/readouts";

const GAP = 14;
/** The edge margin no card sits within. */
const EDGE = 36;
const SPACING = { gap: GAP, edge: EDGE };
/** The farthest a card may sit from its unit: its leader's length. */
const REACH_PX = 160;

type Row = [x: number, y: number, w: number, h: number, highAir?: boolean];
interface Crowd {
  name: string;
  viewport: { width: number; height: number };
  panels: ReadoutRect[];
  rows: Row[];
}
const rect = ([x0, y0, x1, y1]: number[]): ReadoutRect => ({ x0, y0, x1, y1 });

/** Anchors and card sizes read off the air-roster line-up (the user's
 *  report framing), the readouts lab pulled back, and two synthetic crowds. */
const CROWDS: Crowd[] = [
  {
    name: "air-roster line-up",
    viewport: { width: 1646, height: 1030 },
    panels: [
      [605, 10, 1041, 47],
      [1590, 10, 1634, 54],
      [0, 797, 1646, 1030],
      [1289, 660, 1634, 1018],
    ].map(rect),
    rows: [
      [51, 949, 117, 12],
      [258, 949, 154, 12],
      [464, 949, 131, 12],
      [671, 949, 94, 12],
      [878, 949, 117, 12],
      [1085, 949, 131, 12],
      [116, 769, 124, 12],
      [305, 769, 146, 12],
      [495, 769, 109, 12],
      [684, 769, 161, 12],
      [874, 769, 146, 12],
      [1063, 769, 131, 12],
      [1252, 769, 94, 12],
      [147, 566, 117, 12, true],
      [932, 607, 161, 12, true],
      [346, 616, 154, 12],
      [520, 616, 80, 12],
      [695, 616, 169, 12],
      [870, 616, 109, 12],
      [1044, 616, 154, 12],
      [1219, 616, 87, 12],
      [1393, 616, 72, 12],
      [382, 326, 72, 12, true],
      [711, 326, 72, 12, true],
    ],
  },
  {
    name: "readouts lab, pulled back",
    viewport: { width: 1280, height: 800 },
    panels: [
      [12, 12, 392, 172],
      [482, 659, 798, 800],
    ].map(rect),
    rows: [
      [624, 408, 65, 12],
      [640, 365, 87, 12],
      [606, 453, 131, 12],
      [559, 399, 124, 12],
    ],
  },
  {
    name: "a company in a village",
    viewport: { width: 1280, height: 800 },
    panels: [],
    rows: [
      [600, 420, 131, 12],
      [640, 400, 87, 12],
      [580, 450, 65, 12],
      [690, 430, 154, 12],
      [620, 470, 109, 12],
      [710, 395, 72, 12],
      [560, 410, 124, 12],
      [670, 460, 94, 12],
      [735, 450, 146, 12],
      [650, 430, 80, 12, true],
    ],
  },
  {
    name: "a column across the screen",
    viewport: { width: 1280, height: 800 },
    panels: [],
    rows: [100, 170, 240, 310, 380, 450, 520, 590, 660, 730].map(
      (x, i): Row => [x, 520 + (i % 3) * 6, [131, 154, 94, 117, 161][i % 5], 12],
    ),
  },
];

const cardsOf = (crowd: Crowd): LayoutCard[] =>
  crowd.rows.map(([x, y, w, h, highAir = false], i) => ({ id: `u${i}`, x, y, w, h, highAir }));
const lay = (crowd: Crowd, cards = cardsOf(crowd)) =>
  layoutReadouts(cards, crowd.panels, crowd.viewport, SPACING, false);
/** One plane's placed cards (high air, or ground and low air), with their
 *  leaders. */
const plane = (cards: LayoutCard[], placed: Map<string, CardPlacement>, high: boolean) =>
  cards
    .filter((c) => c.highAir === high && placed.has(c.id))
    .map((c) => ({ c, box: placed.get(c.id)!.box, leader: leaderOf(c, placed.get(c.id)!.box) }));
/** Every crowd's two planes, each named. */
const planes = function* () {
  for (const crowd of CROWDS) {
    const cards = cardsOf(crowd);
    const placed = lay(crowd, cards);
    for (const high of [false, true])
      yield {
        name: `${crowd.name}, ${high ? "high air" : "ground and low air"}`,
        shown: plane(cards, placed, high),
      };
  }
};

test("leader lines never cross within a plane", () => {
  for (const { name, shown } of planes()) {
    const crossings: string[] = [];
    for (const [i, a] of shown.entries())
      for (const b of shown.slice(i + 1))
        for (const [p, q] of [
          [a.leader[0], a.leader[1]],
          [a.leader[1], a.leader[2]],
        ])
          for (const [r, s] of [
            [b.leader[0], b.leader[1]],
            [b.leader[1], b.leader[2]],
          ])
            if (segment2.intersects(p, q, r, s)) crossings.push(`${a.c.id}×${b.c.id}`);
    expect([...new Set(crossings)], name).toEqual([]);
  }
});

test("cards never overlap within a plane", () => {
  for (const { name, shown } of planes()) {
    const hits: string[] = [];
    for (const [i, a] of shown.entries())
      for (const b of shown.slice(i + 1))
        if (overlaps(a.box, b.box)) hits.push(`${a.c.id}×${b.c.id}`);
    expect(hits, name).toEqual([]);
  }
});

test("a card stays near its unit: its leader is never long", () => {
  for (const { name, shown } of planes()) {
    const far = shown
      .map(({ c, leader: [from, corner] }) => ({
        id: c.id,
        length: Math.round(Math.hypot(corner[0] - from[0], corner[1] - from[1])),
      }))
      .filter((l) => l.length > REACH_PX);
    expect(far, name).toEqual([]);
  }
});

test("jets whose cards would sit on one spot both show, apart", () => {
  // Two jets a few pixels apart: their cards' natural spots coincide, and
  // the open sky has room for both.
  const viewport = { width: 1280, height: 800 };
  const cards: LayoutCard[] = [
    { id: "typhoon", x: 600, y: 400, w: 161, h: 12, highAir: true },
    { id: "rafale", x: 606, y: 403, w: 117, h: 12, highAir: true },
  ];
  const placed = layoutReadouts(cards, [], viewport, SPACING, false);
  const [a, b] = cards.map((c) => placed.get(c.id)?.box);
  expect(a && b, "both are shown").toBeTruthy();
  expect(overlaps(a!, b!), JSON.stringify({ a, b })).toBe(false);
});

test("every ground or low-air card whose unit stands in the open view is shown", () => {
  // A unit hidden under a HUD panel may lose its card; one in the open view
  // of these crowds always has room within reach.
  for (const crowd of CROWDS.slice(0, 2)) {
    const cards = cardsOf(crowd);
    const placed = lay(crowd, cards);
    const open = (c: LayoutCard) =>
      !crowd.panels.some((p) => c.x >= p.x0 && c.x <= p.x1 && c.y >= p.y0 && c.y <= p.y1);
    const hidden = cards.filter((c) => open(c) && !placed.has(c.id)).map((c) => c.id);
    expect(hidden, crowd.name).toEqual([]);
  }
});

test("a high-air card's spot ignores the ground and low-air cards", () => {
  for (const crowd of CROWDS) {
    const cards = cardsOf(crowd);
    const placed = lay(crowd, cards);
    const aloft = cards.filter((c) => c.highAir);
    const alone = layoutReadouts(aloft, crowd.panels, crowd.viewport, SPACING, false);
    for (const c of aloft)
      expect(placed.get(c.id)?.box, `${crowd.name} ${c.id}`).toEqual(alone.get(c.id)?.box);
  }
});

test("a card that would sit near the screen's edge hides rather than moves", () => {
  // The user's report: jets and a SAM just under the top edge, beside the
  // page's title plate. Their cards would hang above them, into the margin.
  const viewport = { width: 1646, height: 1030 };
  const panels = [[605, 10, 1041, 47]].map(rect);
  const cards: LayoutCard[] = [
    { id: "rafale", x: 40, y: 47, w: 117, h: 12, highAir: true },
    { id: "tornado", x: 190, y: 47, w: 124, h: 12, highAir: true },
    { id: "s350", x: 480, y: 70, w: 72, h: 12, highAir: false },
    { id: "edge-right", x: 1630, y: 500, w: 94, h: 12, highAir: false },
    { id: "open", x: 800, y: 500, w: 94, h: 12, highAir: false },
  ];
  const placed = layoutReadouts(cards, panels, viewport, SPACING, false);
  expect([...placed.keys()]).toEqual(["open"]);
});

test("no card sits within the edge margin", () => {
  for (const crowd of CROWDS)
    for (const [id, { box }] of lay(crowd))
      expect(
        box.x0 >= EDGE &&
          box.y0 >= EDGE &&
          box.x1 <= crowd.viewport.width - EDGE &&
          box.y1 <= crowd.viewport.height - EDGE,
        `${crowd.name} ${id} ${JSON.stringify(box)}`,
      ).toBe(true);
});

test("a ground card steps clear of a high-air card when there is room beside it", () => {
  // A jet passes just over a parked vehicle: their cards' natural spots
  // coincide, and the open field around has room to spare.
  const viewport = { width: 1280, height: 800 };
  const cards: LayoutCard[] = [
    { id: "jet", x: 600, y: 400, w: 140, h: 12, highAir: true },
    { id: "sam", x: 610, y: 404, w: 110, h: 12, highAir: false },
  ];
  const placed = layoutReadouts(cards, [], viewport, SPACING, false);
  const jet = placed.get("jet")!.box;
  const sam = placed.get("sam")?.box;
  expect(sam, "the ground card is shown").toBeDefined();
  expect(overlaps(sam!, jet), JSON.stringify({ jet, sam })).toBe(false);
  // Stepping aside still keeps it near its unit.
  const [from, corner] = leaderOf(cards[1]!, sam!);
  expect(Math.hypot(corner[0] - from[0], corner[1] - from[1])).toBeLessThanOrEqual(REACH_PX);
});

test("a ground card lies under a high-air card only when nothing clear is in reach", () => {
  // Hemmed in by the screen's edges, the ground card's only spots are under
  // the jet's card: it still shows, and the jet's card stays put.
  const viewport = { width: 300, height: 180 };
  const cards: LayoutCard[] = [
    { id: "jet", x: 60, y: 140, w: 150, h: 40, highAir: true },
    { id: "sam", x: 70, y: 142, w: 150, h: 40, highAir: false },
  ];
  const placed = layoutReadouts(cards, [], viewport, SPACING, false);
  expect(placed.get("sam"), "the ground card is shown").toBeDefined();
  expect(overlaps(placed.get("sam")!.box, placed.get("jet")!.box), "it lies under").toBe(true);
  expect(placed.get("jet")?.box).toEqual(
    layoutReadouts([cards[0]!], [], viewport, SPACING, false).get("jet")?.box,
  );
});

test("high-air cards draw above every ground and low-air card, the selected one too", () => {
  const cards = [
    { id: "ground", distanceSq: 1, selected: true, highAir: false },
    { id: "lowair", distanceSq: 2, selected: false, highAir: false },
    { id: "jet", distanceSq: 9, selected: false, highAir: true },
    { id: "blimp", distanceSq: 4, selected: false, highAir: true },
  ];
  for (const far of [false, true]) {
    const order = readoutPaintOrder(cards, null, far);
    for (const high of ["jet", "blimp"])
      for (const low of ["ground", "lowair"])
        expect(order.get(high)!, `${high} over ${low}, far ${far}`).toBeGreaterThan(
          order.get(low)!,
        );
    // The card under the pointer comes to the top of its own plane: a
    // high-air card still draws over it.
    const hovered = readoutPaintOrder(cards, "lowair", far);
    expect(hovered.get("lowair")!).toBeGreaterThan(hovered.get("ground")!);
    for (const high of ["jet", "blimp"])
      expect(hovered.get(high)!).toBeGreaterThan(hovered.get("lowair")!);
    const aloft = readoutPaintOrder(cards, "jet", far);
    expect(Math.max(...aloft.values())).toBe(aloft.get("jet"));
  }
});

test("a card lies nearer its own unit than any other unit of its plane", () => {
  // The air-roster line-up at tick 40 (the critique's report): the F/A-18E
  // and A-10C cards each ended beside the other jet.
  const viewport = { width: 1280, height: 800 };
  const panels = [
    [429, 10, 851, 47],
    [1224, 10, 1268, 54],
    [0, 475, 1280, 800],
    [808, 380, 1268, 788],
  ].map(rect);
  const jet = (id: string, x: number, y: number, w: number): LayoutCard => ({
    id,
    x,
    y,
    w,
    h: 12,
    highAir: true,
  });
  const cards = [
    jet("f35a", 380, 92, 72),
    jet("a10c", 545, 89, 131),
    jet("fa18e", 660, 84, 87),
    jet("f15e", 697, 68, 146),
    jet("ea18g", 715, 125, 139),
    jet("f16c", 855, 157, 139),
  ];
  const placed = layoutReadouts(cards, panels, viewport, SPACING, false);
  expect(placed.has("fa18e") && placed.has("a10c"), "both cluster cards show").toBe(true);
  // The units whose cards were not hidden by the edge.
  const shown = cards.filter(
    (c) => c.y - 34 - c.h >= EDGE && c.x >= EDGE && c.x <= viewport.width - EDGE,
  );
  const wrong: string[] = [];
  for (const [id, { box }] of placed) {
    const own = cards.find((c) => c.id === id)!;
    // From a unit's anchor to the card's nearest point.
    const d = (c: LayoutCard) =>
      Math.hypot(Math.max(box.x0 - c.x, 0, c.x - box.x1), Math.max(box.y0 - c.y, 0, c.y - box.y1));
    const nearer = shown.filter((c) => c.id !== id && d(c) < d(own));
    if (nearer.length) wrong.push(`${id} nearer ${nearer.map((c) => c.id).join(",")}`);
  }
  expect(wrong).toEqual([]);
});

test("a card opening into its detail moves only where it stays nearer its own unit", () => {
  // The readouts lab at 900×600 with the AT team's card hovered: its detail
  // pushes the tank's card, which used to leap above the AT team's.
  const card = (
    id: string,
    anchor: [number, number],
    [x0, y0, w0]: number[],
    [w, h]: number[],
    distanceSq: number,
  ): DetailCard => ({
    id,
    anchor,
    distanceSq,
    compact: [x0, y0, x0 + w0, y0 + 12],
    full: [x0, y0, x0 + w, y0 + h],
  });
  const cards = [
    // Nearest the eye first: the rifle section, low on the screen.
    card("tank", [423, 313], [453, 291, 60], [287, 78], 2),
    card("at", [449, 245], [479, 194, 80], [132, 83], 4),
    card("rifle", [391, 393], [421, 359, 110], [145, 78], 1),
    card("supply", [316, 298], [162, 252, 110], [130, 75], 3),
  ];
  const obstacles: [number, number, number, number][] = [
    [12, 12, 392, 188],
    [292, 459, 608, 600],
    ...cards.map((c): [number, number, number, number] => [
      c.anchor[0] - 6,
      c.anchor[1] - 6,
      c.anchor[0] + 6,
      c.anchor[1] + 6,
    ]),
  ];
  const plan = layoutReadoutDetails(cards, obstacles, [36, 36, 864, 564], GAP, true, "at");
  const wrong: string[] = [];
  for (const c of cards) {
    const [x0, y0, x1, y1] = plan.get(c.id)!.box;
    const d = ([x, y]: readonly [number, number]) =>
      Math.hypot(Math.max(x0 - x, 0, x - x1), Math.max(y0 - y, 0, y - y1));
    const nearer = cards.filter((o) => o.id !== c.id && d(o.anchor) < d(c.anchor));
    if (nearer.length) wrong.push(`${c.id} nearer ${nearer.map((o) => o.id).join(",")}`);
  }
  expect(wrong).toEqual([]);
});

test("a camera close to the ground fades the cards out, and they come back as it pulls away", () => {
  expect(cardZoomOpacity(CARD_FADE_START_M * 2)).toBe(1);
  expect(cardZoomOpacity(CARD_FADE_START_M)).toBe(1);
  expect(cardZoomOpacity(CARD_FADE_END_M)).toBe(0);
  expect(cardZoomOpacity(CARD_FADE_END_M / 2)).toBe(0);
  const between = (CARD_FADE_START_M + CARD_FADE_END_M) / 2;
  expect(cardZoomOpacity(between)).toBeCloseTo(0.5, 6);
  // Zooming in only ever fades them further.
  const steps = Array.from({ length: 50 }, (_, i) => cardZoomOpacity(CARD_FADE_START_M + 10 - i));
  expect(steps.every((o, i) => i === 0 || o <= steps[i - 1]!)).toBe(true);
});
