/** Where each unit's compact info card sits on screen, from its unit's
 *  anchor and the card's size: pure, run every frame. */
import { segment2 } from "math/shapes";

/** Page-pixel box. */
export interface ReadoutRect {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}
export const overlaps = (a: ReadoutRect, b: ReadoutRect) =>
  a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
/** Whether `a` overlaps `b` grown by `m` on every side. */
const overlapsBy = (a: ReadoutRect, b: ReadoutRect, m: number) =>
  a.x0 < b.x1 + m && b.x0 - m < a.x1 && a.y0 < b.y1 + m && b.y0 - m < a.y1;

/** One card to place: its unit's anchor in page pixels and its size. */
export interface LayoutCard {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Its unit flies in the high-air layer. */
  highAir: boolean;
}
export interface CardPlacement {
  /** Where the card would sit with nothing in its way. */
  natural: ReadoutRect;
  box: ReadoutRect;
}

/** Where a card sits with nothing in its way: its near bottom corner this
 *  far to the side of its unit's anchor and up, so the leader rises off the
 *  unit. */
const CALLOUT_SIDE_PX = 30;
const CALLOUT_RISE_PX = 34;
/** Kept clear of the viewport's edge; a card that would cross the right
 *  edge hangs to the unit's left instead. */
export const EDGE_PX = 8;
/** The farthest a card's near corner moves from its unit to make room: a
 *  card with no free spot within reach hides rather than travel. */
export const READOUT_REACH_PX = 150;
/** The spots a card tries: its near corner stepped this far sideways and
 *  up (or down) from the natural spot. */
const STEP_X_PX = 24;
const STEP_Y_PX = 12;
/** A move up costs this many times a move sideways: rows spread out before
 *  they stack. */
const RISE_COST = 1.5;
/** Extra cost, in pixels of travel, of hanging to the unit's other side and
 *  of hanging below the unit. */
const SIDE_SWAP_COST_PX = 60;
const BELOW_COST_PX = 40;
/** A unit's anchor kept clear of other cards and leaders: this half-size
 *  square. */
const ANCHOR_PX = 6;
/** A leader passes no closer than this to another card, so it never reads
 *  as that card's. */
const LEADER_CLEAR_PX = 6;

/** One spot a card may take: its near corner's offset from the anchor,
 *  `dx` away from the unit on its hanging side, `dy` down the screen. */
interface Spot {
  dx: number;
  dy: number;
  swap: boolean;
  cost: number;
}
/** Every spot within reach, cheapest first: the same for every card. The
 *  cost is the travel from the natural spot. */
const SPOTS: readonly Spot[] = (() => {
  const spots: Spot[] = [];
  for (let dx = CALLOUT_SIDE_PX - STEP_X_PX; dx <= READOUT_REACH_PX; dx += STEP_X_PX)
    for (let k = -2; CALLOUT_RISE_PX + k * STEP_Y_PX <= READOUT_REACH_PX; k++)
      for (const swap of [false, true])
        for (const below of [false, true]) {
          const rise = CALLOUT_RISE_PX + k * STEP_Y_PX;
          const dy = below ? rise : -rise;
          if (Math.hypot(dx, dy) > READOUT_REACH_PX) continue;
          // Down the screen from the natural spot; a move up costs more.
          const v = dy + CALLOUT_RISE_PX;
          const travel = Math.hypot(dx - CALLOUT_SIDE_PX, v < 0 ? -v * RISE_COST : v);
          spots.push({
            dx,
            dy,
            swap,
            cost: travel + (swap ? SIDE_SWAP_COST_PX : 0) + (below ? BELOW_COST_PX : 0),
          });
        }
  return spots.sort((a, b) => a.cost - b.cost);
})();

type Segment = [[number, number], [number, number]];
/** Whether segment p-q passes through the inside of `r` grown by `m`
 *  (Liang-Barsky clipping; touching an edge is not crossing). */
function crossesBox(p: [number, number], q: [number, number], r: ReadoutRect, m: number): boolean {
  const x0 = r.x0 - m,
    x1 = r.x1 + m,
    y0 = r.y0 - m,
    y1 = r.y1 + m;
  // Most segments are nowhere near: reject on their bounds first.
  if (Math.max(p[0], q[0]) <= x0 || Math.min(p[0], q[0]) >= x1) return false;
  if (Math.max(p[1], q[1]) <= y0 || Math.min(p[1], q[1]) >= y1) return false;
  const dx = q[0] - p[0],
    dy = q[1] - p[1];
  let t0 = 0,
    t1 = 1;
  const clip = (d: number, g: number) => {
    if (d === 0) return g > 0;
    const t = g / d;
    if (d < 0) t0 = Math.max(t0, t);
    else t1 = Math.min(t1, t);
    return t0 < t1;
  };
  return clip(-dx, p[0] - x0) && clip(dx, x1 - p[0]) && clip(-dy, p[1] - y0) && clip(dy, y1 - p[1]);
}

/** Each card's box; a card missing from the result is hidden.
 *  High-air cards sit at their natural spot, kept on screen and off the HUD
 *  panels, and make no room. Ground and low-air cards are placed one by one
 *  in their units' screen order from the right (the side cards hang to), each
 *  at the cheapest spot within reach whose box clears the panels, the other
 *  units and the cards placed before it, and whose leader crosses no placed
 *  leader or card; with none, it hides. A spot over a high-air card or its
 *  unit's ring is taken only when no clear one is within reach. `overlap` (a far view) puts every card
 *  at its natural spot and lets them overprint. */
export function layoutReadouts(
  cards: readonly LayoutCard[],
  panels: readonly ReadoutRect[],
  viewport: { width: number; height: number },
  gap: number,
  overlap: boolean,
): Map<string, CardPlacement> {
  const out = new Map<string, CardPlacement>();
  const right = viewport.width - EDGE_PX,
    bottom = viewport.height - EDGE_PX;
  const shift = (box: ReadoutRect, dy: number): ReadoutRect => ({
    ...box,
    y0: box.y0 + dy,
    y1: box.y1 + dy,
  });
  const naturalOf = (b: LayoutCard): ReadoutRect => {
    const left = b.x + CALLOUT_SIDE_PX + b.w > right;
    const x0 = left ? b.x - CALLOUT_SIDE_PX - b.w : b.x + CALLOUT_SIDE_PX;
    const y1 = b.y - CALLOUT_RISE_PX;
    return { x0, x1: x0 + b.w, y0: y1 - b.h, y1 };
  };
  const onScreen = (r: ReadoutRect) =>
    r.x0 >= EDGE_PX && r.x1 <= right && r.y0 >= EDGE_PX && r.y1 <= bottom;
  // On screen, and never under a panel: below the top edge, then the
  // shortest way out of a panel, right of a side panel, below a top
  // plate, above a bottom bar; never up off the top of the screen.
  const clear = (box: ReadoutRect): ReadoutRect => {
    if (box.y0 < EDGE_PX) box = shift(box, EDGE_PX - box.y0);
    for (const r of panels) {
      if (overlaps(r, box)) {
        const right = r.x1 + gap - box.x0;
        const down = r.y1 + gap - box.y0;
        const up = r.y0 - gap - box.y1;
        const dy = Math.abs(down) < Math.abs(up) || box.y0 + up < 0 ? down : up;
        box =
          right <= Math.abs(dy)
            ? { ...box, x0: box.x0 + right, x1: box.x1 + right }
            : shift(box, dy);
      }
    }
    return box;
  };

  const grounded = cards.filter((b) => !b.highAir);
  for (const b of cards) {
    if (!b.highAir && !overlap) continue;
    const natural = naturalOf(b);
    const box = clear(natural);
    if (onScreen(box)) out.set(b.id, { natural, box });
  }
  if (overlap) return out;

  // Kept clear by a card's box, padded by the gap: the panels and the cards
  // placed so far; by its box alone: the other units' anchors.
  const taken: ReadoutRect[] = [...panels];
  const anchors = grounded.map((b) => ({
    id: b.id,
    r: { x0: b.x - ANCHOR_PX, x1: b.x + ANCHOR_PX, y0: b.y - ANCHOR_PX, y1: b.y + ANCHOR_PX },
  }));
  const boxes: ReadoutRect[] = [];
  const leaders: Segment[] = [];
  const free = (b: LayoutCard, box: ReadoutRect, rise: Segment, foot: Segment) =>
    onScreen(box) &&
    !taken.some((o) => overlapsBy(box, o, gap)) &&
    !anchors.some(
      (a) => a.id !== b.id && (overlaps(a.r, box) || crossesBox(rise[0], rise[1], a.r, 0)),
    ) &&
    !boxes.some((o) => crossesBox(rise[0], rise[1], o, LEADER_CLEAR_PX)) &&
    !leaders.some(
      ([p, q]) =>
        crossesBox(p, q, box, LEADER_CLEAR_PX) ||
        segment2.intersects(p, q, rise[0], rise[1]) ||
        segment2.intersects(p, q, foot[0], foot[1]),
    );
  // High-air cards and their units' rings are a soft obstacle: a spot over
  // one is taken only when no clear spot is within reach.
  const aloft = cards.filter((b) => b.highAir && out.has(b.id));
  const clearOfAloft = (box: ReadoutRect) =>
    !aloft.some(
      (b) =>
        overlapsBy(box, out.get(b.id)!.box, gap) ||
        overlapsBy(box, { x0: b.x, x1: b.x, y0: b.y, y1: b.y }, ANCHOR_PX),
    );
  for (const b of [...grounded].sort((m, n) => n.x - m.x)) {
    const natural = naturalOf(b);
    let under: { box: ReadoutRect; rise: Segment; foot: Segment } | null = null;
    for (const s of SPOTS) {
      const hangLeft = s.swap !== natural.x0 < b.x;
      const x0 = hangLeft ? b.x - s.dx - b.w : b.x + s.dx;
      const below = s.dy > 0;
      const y0 = below ? b.y + s.dy : b.y + s.dy - b.h;
      const box = { x0, x1: x0 + b.w, y0, y1: y0 + b.h };
      const [from, corner, end] = leaderOf(b, box);
      const rise: Segment = [from, corner];
      const foot: Segment = [corner, end];
      if (!free(b, box, rise, foot)) continue;
      if (!clearOfAloft(box)) {
        under ??= { box, rise, foot };
        continue;
      }
      under = { box, rise, foot };
      break;
    }
    if (!under) continue;
    taken.push(under.box);
    boxes.push(under.box);
    leaders.push(under.rise, under.foot);
    out.set(b.id, { natural, box: under.box });
  }
  return out;
}

/** A leader line: from the unit's anchor to the card's near corner, then
 *  along the card's edge nearest the unit. */
export function leaderOf(
  anchor: { x: number; y: number },
  box: ReadoutRect,
): [[number, number], [number, number], [number, number]] {
  const [near, far] = box.x0 >= anchor.x ? [box.x0, box.x1] : [box.x1, box.x0];
  const edge = box.y0 >= anchor.y ? box.y0 : box.y1;
  return [
    [anchor.x, anchor.y],
    [near, edge],
    [far, edge],
  ];
}

/** Each card's paint order, higher on top: the hovered card above all;
 *  then every high-air card above every ground and low-air one; within each,
 *  in a far view nearer the eye above, otherwise a selected card above the
 *  rest. */
export function readoutPaintOrder(
  cards: readonly { id: string; distanceSq: number; selected: boolean; highAir: boolean }[],
  hovered: string | null,
  overlap: boolean,
): Map<string, number> {
  const order = new Map<string, number>();
  const n = cards.length;
  const near = [...cards].sort((a, b) => a.distanceSq - b.distanceSq);
  for (const [i, b] of near.entries())
    order.set(
      b.id,
      b.id === hovered
        ? 2 * n + 2
        : (overlap ? n - i : Number(b.selected)) + (b.highAir ? n + 1 : 0),
    );
  return order;
}
