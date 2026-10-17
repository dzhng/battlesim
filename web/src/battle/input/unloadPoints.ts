/** Where each own carrier's orders let its passengers out: the order
 *  line's unload mark (D21). The observation publishes where a unit's
 *  orders end (its goal and queue), not what each does there, so the
 *  side's own accepted Unload orders say which of those ends unload. A
 *  point lives from its acknowledgement until, once its order has landed,
 *  the carrier's goal and queue no longer name it (arrived, dropped or
 *  replaced). */
import type { ObservationView } from "../sim/observationView";
import type { CommandAck, Order } from "../sim/protocol";

type Point = readonly [number, number];

interface UnloadPoint {
  at: Point;
  /** The tick its order lands on (`CommandAck.applied_tick`). */
  landsAt: number;
}

export type UnloadPoints = ReadonlyMap<number, readonly UnloadPoint[]>;

export const NO_UNLOAD_POINTS: UnloadPoints = new Map();

/** The same point, as published (float32) and as acknowledged. */
const samePoint = (a: Point, b: Point) =>
  Math.abs(a[0] - b[0]) < 0.05 && Math.abs(a[1] - b[1]) < 0.05;

/** `points` after the side's accepted `order`: a fresh order replaces its
 *  units' points, and an Unload at a point adds each placed carrier's. */
export function afterOrder(
  points: UnloadPoints,
  order: Order,
  ack: CommandAck,
  queued: boolean,
): UnloadPoints {
  if (ack.error || !("units" in order)) return points;
  const unloading = order.kind === "unload" && order.goal !== null;
  if (queued && !unloading) return points;
  const next = new Map(points);
  if (!queued) for (const id of order.units) next.delete(id);
  if (unloading)
    for (const d of ack.placement?.destinations ?? [])
      if (d.placed)
        next.set(d.unit, [...(next.get(d.unit) ?? []), { at: d.goal, landsAt: ack.applied_tick }]);
  return next;
}

/** `points` less those whose order has landed by `o` and that its
 *  carrier's goal and queue no longer name; `points` itself when none go. */
export function retire(points: UnloadPoints, o: ObservationView): UnloadPoints {
  let next: Map<number, readonly UnloadPoint[]> | null = null;
  for (const [id, list] of points) {
    const u = o.own.find((unit) => unit.id === id);
    const ahead = u ? [...(u.goal ? [u.goal] : []), ...u.queue] : [];
    const kept = list.filter((p) => o.tick < p.landsAt || ahead.some((q) => samePoint(p.at, q)));
    if (kept.length === list.length) continue;
    next ??= new Map(points);
    if (kept.length) next.set(id, kept);
    else next.delete(id);
  }
  return next ?? points;
}

/** Each carrier's unload points, as the order line reads them. */
export const unloadMarks = (points: UnloadPoints) =>
  new Map([...points].map(([id, list]) => [id, list.map((p) => p.at)]));
