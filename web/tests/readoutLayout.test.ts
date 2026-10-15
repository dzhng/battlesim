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
const grounded = (cards: LayoutCard[], placed: Map<string, CardPlacement>) =>
  plane(cards, placed, false);
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
