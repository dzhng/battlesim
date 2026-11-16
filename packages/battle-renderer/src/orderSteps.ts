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
  /** The building a garrison step enters or an exit step leaves, as the
   *  side knows it: its footprint's centre and half extents. */
  building?: { center: P2; half: P2 };
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

/** The current order lets passengers out where it ends: an unload follows it. */
export const currentUnloads = (steps: readonly OrderStep[]) => steps[1]?.kind === "unload";

/** A queued waypoint's ring, and a blocked route's warning ring. */
const QUEUED_R = 2.8;
const BLOCKED_R = 4.5;

/** A blocked route's warning ring round its goal. */
export const warningRing = (goal: P2): UnitCircle => ({ c: goal, r: BLOCKED_R, facing: null });

/** What a unit's queued chain draws in, at queued strength: its routes'
 *  colour and its attacks'. */
export interface ChainColors {
  route: Rgba;
  attack: Rgba;
}

/** The ring round a building a step enters or leaves: its corners plus
 *  the style's margin, as a held building's. */
const buildingRing = (pen: Pen, b: NonNullable<OrderStep["building"]>): UnitCircle => ({
  c: b.center,
  r: Math.hypot(b.half[0], b.half[1]) + pen.style.building_marker_margin_m,
  facing: null,
});

/** An attack's marks: a line from `from` along `route` to a reticle on
 *  `aim` (a centre cross on a ground point), dashed while queued. The
 *  reticle's circle, where the next leg leaves from. One drawer for the
 *  current attack and every queued one. */
function aimAt(
  paint: MeshBuilder,
  pen: Pen,
  from: UnitCircle | null,
  route: readonly P2[],
  aim: P2,
  step: OrderStep,
  { line, mark, dashed = false }: { line: Rgba; mark: Rgba; dashed?: boolean },
): UnitCircle {
  const reticle: UnitCircle = { c: aim, r: pen.style.reticle_m, facing: null };
  routeBetween(paint, pen, from, [...route, aim], reticle, line, { dashed });
  reticleMark(paint, pen, aim, reticle.r, mark, { cross: step.target?.kind === "ground" });
  return reticle;
}

/** One queued step, from `prev` (the mark the step before ends in), drawn
 *  as its order draws when current, its leg dashed: a move or a return to
 *  base, a leg to a ring; an attack, a line to its reticle; a garrison, a
 *  leg to the building's ring; an exit, a leg out to the building's ring
 *  it leaves; a board or a collect, a leg to the carrier or the squad; an
 *  unload, the unload mark on `prev`, facing the way the leg came in from
 *  `behind`. The mark it ends in, or `prev` where it draws none. */
function queuedStep(
  paint: MeshBuilder,
  pen: Pen,
  step: OrderStep,
  prev: UnitCircle | null,
  behind: P2 | null,
  colors: ChainColors,
): UnitCircle | null {
  const q = step.point;
  if (step.kind === "unload") {
    if (prev && behind)
      unloadMark(
        paint,
        pen,
        prev,
        Math.atan2(prev.c[1] - behind[1], prev.c[0] - behind[0]),
        colors.route,
      );
    return prev;
  }
  if (!q) return prev;
  if (step.kind === "attack")
    return aimAt(paint, pen, prev, [], q, step, {
      line: colors.attack,
      mark: colors.attack,
      dashed: true,
    });
  const house = step.building && (step.kind === "garrison" || step.kind === "exit");
  const to: UnitCircle = house
    ? buildingRing(pen, step.building!)
    : { c: q, r: QUEUED_R, facing: null };
  routeBetween(paint, pen, prev, [to.c], to, colors.route, { dashed: true });
  const ringed = step.kind === "move" || step.kind === "return_to_base" || step.kind === "garrison";
  if (ringed) ring(paint, pen, to.c, to.r, colors.route);
  return to;
}

/** A unit's queued steps after its current order, one continuous chain:
 *  the first leaves the circle where the current order ends (its
 *  destination's, `dest`, else a blocked route's warning ring), each next
 *  one the mark the step before ends in. An unload straight after the
 *  current order is that order's own mark. A winged aircraft, circling its
 *  goal, keeps only its queued moves and no ground leg to the first. */
export function queuedLegs(
  paint: MeshBuilder,
  pen: Pen,
  u: Pick<OrderView, "steps" | "goal" | "state" | "fixedWing">,
  dest: UnitCircle | null,
  colors: ChainColors,
) {
  const blocked = u.state === "route_blocked" && !u.fixedWing;
  let prev = dest ?? (blocked && u.goal ? warningRing(u.goal) : null);
  let behind: P2 | null = u.goal;
  u.steps.forEach((step, k) => {
    if (k === 0 || (k === 1 && step.kind === "unload")) return;
    if (u.fixedWing && step.kind !== "move") return;
    const next = queuedStep(paint, pen, step, prev, behind, colors);
    if (next !== prev) behind = prev?.c ?? behind;
    prev = next;
  });
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
  const line = u.goal ? routeColor : color;
  return aimAt(paint, pen, circle, u.goal ? u.route : [], aim, u.steps[0], { line, mark: color });
}
