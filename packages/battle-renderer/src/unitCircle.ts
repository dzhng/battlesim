// A unit's circle on the ground: where the orders draw it round a unit or at
// its order's end, and where a line leaving or reaching it meets it. Pure
// geometry; the order overlay draws it (`circleMarker`), the picking hits it
// and the range ruler leaves from it.
import { vec2, type Vec2 } from "math";
import type { OrderStyle, OrderView } from "./orderOverlay";

type P2 = readonly [number, number];

/** A soldier's marker circle radius (a vehicle's is the style's
 *  its hull's half-length plus `vehicle_marker_margin_m`). */
export const SOLDIER_R = 0.45;
/** A unit marker's arrowhead: its length past the rim, over the radius. */
const MARKER_HEAD = 0.6;
/** …never longer than this: a vehicle's reads as a pointer, not a wedge. */
const MARKER_HEAD_MAX_M = 1.5;
export const markerHead = (r: number) => Math.min(r * MARKER_HEAD, MARKER_HEAD_MAX_M);

/** How far a unit marker of radius `r` reaches along its facing (its
 *  arrowhead's tip): where a route arriving from ahead stops. */
const markerReach = (r: number) => r + markerHead(r);

/** How far from its centre a line leaving (or reaching) circle `c` along
 *  `bearing` meets it: its rim, or its arrowhead's tip near its facing (a
 *  circle with no facing has no arrowhead). */
export function circleReach(c: Pick<UnitCircle, "r" | "facing">, bearing: number): number {
  return c.facing !== null && Math.cos(bearing - c.facing) > Math.cos(Math.PI / 3)
    ? markerReach(c.r)
    : c.r;
}

/** How far along the ground from `from` toward `to` a line leaves `circle`
 *  (clearing its arrowhead where it leaves along the facing): 0 with no
 *  circle, or from outside one it doesn't cross. From the circle's centre it
 *  is `circleReach`; with `to` inside the circle it is past `to`. */
export function circleExit(
  from: readonly number[],
  to: readonly number[],
  circle: UnitCircle | null,
): number {
  if (!circle) return 0;
  const [dx, dy] = [to[0] - from[0], to[1] - from[1]];
  const length = Math.hypot(dx, dy);
  if (length < 1e-9) return 0;
  const [ux, uy] = [dx / length, dy / length];
  const r = circleReach(circle, Math.atan2(uy, ux));
  // |from + t·u − c|² = r², the larger root.
  const [fx, fy] = [from[0] - circle.c[0], from[1] - circle.c[1]];
  const b = fx * ux + fy * uy;
  const disc = b * b - (fx * fx + fy * fy - r * r);
  return disc > 0 ? Math.max(0, -b + Math.sqrt(disc)) : 0;
}

/** The circle a squad stands in now: round its soldiers, from its position. */
function squadNow(u: Pick<OrderView, "members">, here: P2): number {
  const radius = u.members.reduce(
    (r, m) => Math.max(r, Math.hypot(m[0] - here[0], m[1] - here[1])),
    0,
  );
  return radius + SOLDIER_R + 0.4;
}

/** A unit's circle on the ground, with its arrowhead's facing (null: none). */
export interface UnitCircle {
  c: P2;
  r: number;
  facing: number | null;
}

/** The circular interior, excluding the facing arrowhead. */
export function circleContains(circle: UnitCircle, point: readonly [number, number]): boolean {
  return vec2.squaredDistance(circle.c as Vec2, point as Vec2) <= circle.r * circle.r;
}

/** The circle the orders draw round a unit where it stands, or null where
 *  they draw none, one marker for every ground unit: a vehicle's under its
 *  hull (moving, selected or revealed; an aircraft's always, on the ground
 *  under it), a moving squad's round its soldiers,
 *  and, selected or revealed, a holding squad's area ring round its anchor,
 *  each with its arrowhead at the unit's facing (a moving squad's heading, a
 *  holding one's facing). A garrisoned squad's circle, moving (leaving on an
 *  order), selected or revealed, encloses its building, with no arrowhead. Lines to or from the unit (routes, the range ruler) meet this
 *  circle at its border (`circleReach`). */
export function unitCircle(
  u: Pick<
    OrderView,
    | "position"
    | "goal"
    | "state"
    | "members"
    | "area"
    | "hullHalfLength"
    | "building"
    | "yaw"
    | "selected"
    | "reveal"
    | "fixedWing"
    | "airborne"
  >,
  style: Pick<
    OrderStyle,
    "area_draw_scale" | "vehicle_marker_margin_m" | "building_marker_margin_m"
  >,
): UnitCircle | null {
  const shown = (u.reveal ?? 0) > 0;
  const here: P2 = [u.position[0], u.position[1]];
  const moving = !!u.goal && u.state !== "route_blocked";
  // A winged aircraft never stops over a point: its card carries where it
  // is, and it has no ground marker.
  if (u.fixedWing) return null;
  if (u.building) {
    const { center, half } = u.building;
    const r = Math.hypot(half[0], half[1]) + style.building_marker_margin_m;
    return moving || shown || u.selected ? { c: center, r, facing: null } : null;
  }
  // A squad's rings are drawn at `area_draw_scale` of their radius (the
  // movement area itself is the simulation's).
  const drawn = (radius: number) => radius * style.area_draw_scale;
  if (u.members.length === 0)
    return moving || shown || u.selected || u.airborne
      ? { c: here, r: u.hullHalfLength + style.vehicle_marker_margin_m, facing: u.yaw }
      : null;
  if (moving) return { c: here, r: drawn(squadNow(u, here)), facing: u.yaw };
  return (shown || u.selected) && !u.goal && u.area
    ? { c: u.area.anchor, r: drawn(u.area.radius), facing: u.yaw }
    : null;
}

/** The circle a unit's order ends in: a squad's area ring round its new
 *  anchor, a vehicle's marker at its goal, pointing the final facing. */
export function destinationCircle(
  u: Pick<OrderView, "goal" | "area" | "members" | "hullHalfLength" | "finalFacing">,
  goal: P2,
  style: Pick<OrderStyle, "area_draw_scale" | "vehicle_marker_margin_m">,
): UnitCircle {
  return u.members.length > 0 && u.area
    ? { c: u.area.anchor, r: u.area.radius * style.area_draw_scale, facing: u.finalFacing }
    : { c: goal, r: u.hullHalfLength + style.vehicle_marker_margin_m, facing: u.finalFacing };
}
