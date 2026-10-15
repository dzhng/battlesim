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

/** How cards keep apart: the least space between a card and anything it
 *  makes room for, and the margin along the viewport's edge that no card
 *  sits within. */
export interface ReadoutSpacing {
  gap: number;
  edge: number;
}

/** Each card's box; a card missing from the result is hidden.
 *  A card whose unit, or whose natural spot, lies within `edge` of the
 *  viewport's edge hides rather than moves. The rest are laid out in two
 *  planes, high-air cards first, each plane alike: its cards are placed one
 *  by one in their units' screen order from the right (the side cards hang
 *  to), each at the cheapest spot within reach whose box keeps off the edge
 *  margin and clears the HUD panels, its plane's other units and the cards
 *  placed before it, and whose leader crosses no placed leader or card of
 *  its plane; with none, it hides. A spot whose box lies nearer another of
 *  the plane's units than its own reads as that unit's card, so it is taken
 *  only when no other spot is within reach. The high-air plane draws above,
 *  so a ground or low-air card takes a spot over a high-air card or its
 *  unit's ring only when no clear one is within reach. `overlap` (a far
 *  view) puts every card at its natural spot, clear of the panels, and lets
 *  them overprint. */
export function layoutReadouts(
  cards: readonly LayoutCard[],
  panels: readonly ReadoutRect[],
  viewport: { width: number; height: number },
  { gap, edge }: ReadoutSpacing,
  overlap: boolean,
): Map<string, CardPlacement> {
  const out = new Map<string, CardPlacement>();
  const right = viewport.width - edge,
    bottom = viewport.height - edge;
  const naturalOf = (b: LayoutCard): ReadoutRect => {
    const left = b.x + CALLOUT_SIDE_PX + b.w > right;
    const x0 = left ? b.x - CALLOUT_SIDE_PX - b.w : b.x + CALLOUT_SIDE_PX;
    const y1 = b.y - CALLOUT_RISE_PX;
    return { x0, x1: x0 + b.w, y0: y1 - b.h, y1 };
  };
  const inside = (r: ReadoutRect) =>
    r.x0 >= edge && r.x1 <= right && r.y0 >= edge && r.y1 <= bottom;
  const shown = cards.filter(
    (b) => b.x >= edge && b.x <= right && b.y >= edge && b.y <= bottom && inside(naturalOf(b)),
  );
  if (overlap) {
    for (const b of shown) {
      const natural = naturalOf(b);
      if (!panels.some((r) => overlaps(r, natural))) out.set(b.id, { natural, box: natural });
    }
    return out;
  }

  /** Lay out one plane's cards; `soft` is what they keep off when they can. */
  const place = (plane: readonly LayoutCard[], soft: (box: ReadoutRect) => boolean) => {
    /** From a unit's anchor to a box's nearest point. */
    const reach = (b: LayoutCard, box: ReadoutRect) =>
      Math.hypot(Math.max(box.x0 - b.x, 0, b.x - box.x1), Math.max(box.y0 - b.y, 0, b.y - box.y1));
    /** Whether a box lies nearer another of the plane's units than its own,
     *  so it reads as that unit's card. */
    const astray = (b: LayoutCard, box: ReadoutRect) => {
      const own = reach(b, box);
      return plane.some((o) => o.id !== b.id && reach(o, box) < own);
    };
    // Kept clear by a card's box, padded by the gap: the panels and the
    // plane's cards placed so far; by its box alone: the plane's other units.
    const taken: ReadoutRect[] = [...panels];
    const anchors = plane.map((b) => ({
      id: b.id,
      r: { x0: b.x - ANCHOR_PX, x1: b.x + ANCHOR_PX, y0: b.y - ANCHOR_PX, y1: b.y + ANCHOR_PX },
    }));
    const boxes: ReadoutRect[] = [];
    const leaders: Segment[] = [];
    const free = (b: LayoutCard, box: ReadoutRect, rise: Segment, foot: Segment) =>
      inside(box) &&
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
    for (const b of [...plane].sort((m, n) => n.x - m.x)) {
      const natural = naturalOf(b);
      // Each free spot's penalty: lying nearer another unit than its own
      // outweighs lying over a soft obstacle. The cheapest spot of the
      // least penalty within reach wins.
      let found: { box: ReadoutRect; rise: Segment; foot: Segment; penalty: number } | null = null;
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
        const penalty = (astray(b, box) ? 2 : 0) + (soft(box) ? 1 : 0);
        if (!found || penalty < found.penalty) found = { box, rise, foot, penalty };
        if (penalty === 0) break;
      }
      if (!found) continue;
      taken.push(found.box);
      boxes.push(found.box);
      leaders.push(found.rise, found.foot);
      out.set(b.id, { natural, box: found.box });
    }
  };

  place(
    shown.filter((b) => b.highAir),
    () => false,
  );
  // High-air cards and their units' rings are a soft obstacle below.
  const aloft = shown.filter((b) => b.highAir && out.has(b.id));
  place(
    shown.filter((b) => !b.highAir),
    (box) =>
      aloft.some(
        (b) =>
          overlapsBy(box, out.get(b.id)!.box, gap) ||
          overlapsBy(box, { x0: b.x, x1: b.x, y0: b.y, y1: b.y }, ANCHOR_PX),
      ),
  );
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

/** Each card's paint order, higher on top: every high-air card above every
 *  ground and low-air one; within each plane, the hovered card on top, then
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
      (b.id === hovered ? n + 1 : overlap ? n - i : Number(b.selected)) + (b.highAir ? n + 2 : 0),
    );
  return order;
}
