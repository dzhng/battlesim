import { clamp, vec2 } from "math";
import { box2, type Box2 } from "math/shapes";

export interface DetailCard {
  id: string;
  distanceSq: number;
  compact: Box2;
  full: Box2;
}
export interface DetailPlacement {
  box: Box2;
  full: boolean;
}
const _details_padded = box2.create();
const _details_from = vec2.create(),
  _details_to = vec2.create();
const originTravelSq = (a: Box2, b: Box2) =>
  vec2.squaredDistance(vec2.set(_details_from, a[0], a[1]), vec2.set(_details_to, b[0], b[1]));

/** Closest 30% first, then opportunistic detail. Hover always takes priority.
 * Try no movement, compare one-card moves, and only then displace blockers. */
export function layoutReadoutDetails(
  cards: readonly DetailCard[],
  obstacles: readonly Box2[],
  viewport: Box2,
  gap: number,
  expanded: boolean,
  hovered: string | null,
): Map<string, DetailPlacement> {
  let plan = new Map(cards.map((c) => [c.id, { box: c.compact, full: false }]));
  const collides = (a: Box2, b: Box2) => {
    // DOM offset sizes round to whole pixels; allow the same half-pixel
    // tolerance as the compact layout's spacing check.
    box2.expandByMargin(_details_padded, b, Math.max(0, gap - 0.5));
    return box2.intersectsBox2(a, _details_padded);
  };
  const fits = (a: Box2, others: readonly Box2[]) =>
    box2.containsBox2(viewport, a) &&
    !obstacles.some((o) => collides(a, o)) &&
    !others.some((o) => collides(a, o));
  const cost = (p: Map<string, DetailPlacement>) => {
    let moved = 0,
      travel = 0;
    for (const c of cards) {
      const d = originTravelSq(p.get(c.id)!.box, c.compact);
      if (d > 0.25) moved++;
      travel += d;
    }
    return [moved, travel] as const;
  };
  const better = (a: Map<string, DetailPlacement>, b: Map<string, DetailPlacement>) => {
    const ac = cost(a),
      bc = cost(b);
    return ac[0] < bc[0] || (ac[0] === bc[0] && ac[1] < bc[1]);
  };
  const at = (shape: Box2, x: number, y: number): Box2 => {
    const w = shape[2] - shape[0],
      h = shape[3] - shape[1];
    x = clamp(x, viewport[0], viewport[2] - w);
    y = clamp(y, viewport[1], viewport[3] - h);
    return [x, y, x + w, y + h];
  };
  const free = (shape: Box2, origin: Box2, others: readonly Box2[]) => {
    let best: Box2 | null = null;
    const consider = (x: number, y: number) => {
      const candidate = at(shape, x, y);
      if (
        fits(candidate, others) &&
        (!best || originTravelSq(candidate, origin) < originTravelSq(best, origin))
      )
        best = candidate;
    };
    consider(origin[0], origin[1]);
    const w = shape[2] - shape[0],
      h = shape[3] - shape[1];
    for (const o of [...obstacles, ...others]) {
      const left = o[0] - gap - w,
        right = o[2] + gap;
      const above = o[1] - gap - h,
        below = o[3] + gap;
      consider(left, origin[1]);
      consider(right, origin[1]);
      consider(origin[0], above);
      consider(origin[0], below);
      consider(left, above);
      consider(right, above);
      consider(left, below);
      consider(right, below);
    }
    return best;
  };
  const expand = (c: DetailCard, allowMoves: boolean, forced: boolean) => {
    const current = plan.get(c.id)!.box;
    const w = c.full[2] - c.full[0],
      h = c.full[3] - c.full[1];
    const wanted: Box2 = [current[0], current[1], current[0] + w, current[1] + h];
    const others = [...plan].filter(([id]) => id !== c.id);
    if (
      fits(
        wanted,
        others.map(([, p]) => p.box),
      )
    ) {
      plan.set(c.id, { box: wanted, full: true });
      return;
    }
    if (!allowMoves) return;
    let best: Map<string, DetailPlacement> | null = null;
    const self = free(
      wanted,
      c.compact,
      others.map(([, p]) => p.box),
    );
    if (self) {
      best = new Map(plan);
      best.set(c.id, { box: self, full: true });
    }
    const blockers = others.filter(([, p]) => collides(wanted, p.box));
    const movable =
      box2.containsBox2(viewport, wanted) && !obstacles.some((o) => collides(wanted, o));
    // A free one-card move beats shifting multiple untouched neighbors.
    const alreadyMoved = blockers.some(
      ([id, p]) => originTravelSq(p.box, cards.find((c) => c.id === id)!.compact) > 0.25,
    );
    if (movable && (!best || blockers.length === 1 || alreadyMoved)) {
      const trial = new Map(plan);
      trial.set(c.id, { box: wanted, full: true });
      let okay = true;
      for (const [id, p] of blockers) {
        const original = cards.find((c) => c.id === id)!.compact;
        const clear = free(
          p.box,
          original,
          [...trial].filter(([k]) => k !== id).map(([, p]) => p.box),
        );
        if (!clear) {
          okay = false;
          break;
        }
        trial.set(id, { ...p, box: clear });
      }
      if (okay && (!best || better(trial, best))) best = trial;
    }
    if (best) plan = best;
    else if (forced) plan.set(c.id, { box: at(wanted, current[0], current[1]), full: true });
  };
  const near = [...cards].sort((a, b) => a.distanceSq - b.distanceSq);
  const hover = hovered && cards.find((c) => c.id === hovered);
  if (hover) expand(hover, true, true);
  if (expanded) {
    const quota = Math.ceil(near.length * 0.3);
    near.forEach((c, i) => {
      if (c.id !== hovered) expand(c, i < quota, false);
    });
  }
  return plan;
}
