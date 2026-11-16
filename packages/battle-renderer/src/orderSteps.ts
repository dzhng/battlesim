// A unit's orders as the overlay takes them: the published step records,
// the current order first (`OrderView.steps`). One owner says where the
// orders end (the Space ghost and its drop line) and draws the queued legs.
import type { MeshBuilder, Rgba } from "./mesh";
import type { OrderView } from "./orderOverlay";
import { ring, routeBetween, unloadMark, type Pen } from "./orderPen";
import { reticleMark } from "./reticleMark";
import type { UnitCircle } from "./unitCircle";

/** The kinds of order a step can be, by the simulation's names. */
export type OrderStepKind =
  | "move"
  | "attack"
  | "garrison"
  | "board"
  | "collect"
  | "unload"
  | "exit"
  | "return_to_base";

type P2 = readonly [number, number];

/** One order of a unit: its kind, and where the side knows it takes the
 *  unit or what it acts on (null when it knows none). */
export interface OrderStep {
  kind: OrderStepKind;
  point: P2 | null;
  /** An attack's target, by kind: a ground point's reticle carries a cross. */
  target?: { kind: string };
}

/** Where a unit's orders end: the point of its last queued step that takes
 *  it somewhere (never an attack's, which only aims), else its current
 *  goal; null while it holds. */
export function ordersEnd(steps: readonly OrderStep[], goal: P2 | null): P2 | null {
  if (!goal) return null;
  for (let k = steps.length - 1; k > 0; k--) {
    const { kind, point } = steps[k];
    if (kind !== "attack" && point) return point;
  }
  return goal;
}

/** Each queued move after the current order, its point and whether the
 *  step after it lets passengers out there: the queued legs the overlay
 *  draws (other kinds draw nothing yet). */
function queuedMoves(steps: readonly OrderStep[]): { point: P2; unloads: boolean }[] {
  return steps.flatMap((step, k) =>
    k > 0 && step.kind === "move" && step.point
      ? [{ point: step.point, unloads: steps[k + 1]?.kind === "unload" }]
      : [],
  );
}

/** The current order lets passengers out where it ends: an unload follows it. */
export const currentUnloads = (steps: readonly OrderStep[]) => steps[1]?.kind === "unload";

/** A queued waypoint's ring, and a blocked route's warning ring. */
const QUEUED_R = 2.8;
const BLOCKED_R = 4.5;

/** A blocked route's warning ring round its goal. */
export const warningRing = (goal: P2): UnitCircle => ({ c: goal, r: BLOCKED_R, facing: null });

/** A unit's queued legs after its goal: each a dashed route to a small
 *  ring, with an unload mark, pointing along the leg, where that step lets
 *  passengers out. The first leaves the circle where the current order
 *  ends: its destination's (`dest`), else a blocked route's warning ring.
 *  A winged aircraft, circling its goal, has neither: no ground leg runs to
 *  its first queued point. */
export function queuedLegs(
  paint: MeshBuilder,
  pen: Pen,
  u: Pick<OrderView, "steps" | "goal" | "state" | "fixedWing">,
  dest: UnitCircle | null,
  color: Rgba,
) {
  const blocked = u.state === "route_blocked" && !u.fixedWing;
  let prev = dest ?? (blocked && u.goal ? warningRing(u.goal) : null);
  for (const { point: q, unloads } of queuedMoves(u.steps)) {
    const next: UnitCircle = { c: q, r: QUEUED_R, facing: null };
    routeBetween(paint, pen, prev, [q], next, color, { dashed: true });
    ring(paint, pen, q, QUEUED_R, color);
    if (unloads) {
      const from = prev?.c ?? u.goal ?? q;
      unloadMark(paint, pen, next, Math.atan2(q[1] - from[1], q[0] - from[0]), color);
    }
    prev = next;
  }
}

/** Where the current order's attack aims, when the overlay draws it: a
 *  ground unit's attack on a target the side places, and not stuck behind a
 *  blocked route (that route's own mark says so). A winged aircraft's attack
 *  keeps today's marks until its 3D line (part 2). */
export function currentAim(u: Pick<OrderView, "steps" | "fixedWing" | "state">): P2 | null {
  const front = u.steps[0];
  if (front?.kind !== "attack" || u.fixedWing || u.state === "route_blocked") return null;
  return front.point;
}

/** The current attack's marks, at `aim`: the reticle on the target and, in
 *  range (no goal), a solid line in `color` from the unit's circle to it;
 *  chasing, the unit's route in `routeColor` runs on into the reticle, where
 *  a move's destination circle would be. The reticle's circle, where the
 *  queued legs leave from. */
export function currentAttack(
  paint: MeshBuilder,
  pen: Pen,
  u: Pick<OrderView, "goal" | "route" | "steps">,
  circle: UnitCircle | null,
  aim: P2,
  color: Rgba,
  routeColor: Rgba,
): UnitCircle {
  const reticle: UnitCircle = { c: aim, r: pen.style.reticle_m, facing: null };
  if (u.goal) routeBetween(paint, pen, circle, [...u.route, aim], reticle, routeColor);
  else routeBetween(paint, pen, circle, [aim], reticle, color);
  reticleMark(paint, pen, aim, reticle.r, color, { cross: u.steps[0]?.target?.kind === "ground" });
  return reticle;
}
