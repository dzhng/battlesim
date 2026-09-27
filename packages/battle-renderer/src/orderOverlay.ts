// Order presentation (D2, D2+): remaining routes draped on the walkable
// surface, the final marker with its facing (Total War style), queued
// waypoints, blocked/waiting marks, and each soldier's resolved spot. Holding
// Space (`all`) adds, for every own unit, a marker under each soldier's and
// vehicle's current position and a cover icon under each soldier's marker:
// the cover he has now at his current marker, the cover his spot gives at
// the final one. Built only from the observing side's own-unit view.
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { WorldMeshes } from "./scene";

export type CoverTierName = "light" | "medium" | "heavy";

export interface MemberOrderMark {
  spot: readonly [number, number];
  coverNow: CoverTierName | null;
  coverThere: CoverTierName | null;
}

export interface OrderView {
  position: readonly [number, number, number];
  goal: readonly [number, number] | null;
  policy: string | null;
  state: string;
  route: readonly (readonly [number, number])[];
  queue: readonly (readonly [number, number])[];
  /** Living soldiers' positions (empty for a vehicle). */
  members: readonly (readonly [number, number, number])[];
  /** Each living soldier's resolved spot and cover, in `members` order. */
  memberOrders: readonly MemberOrderMark[];
  /** The bearing the unit ends its move at. */
  finalFacing: number;
  /** "reverse" on a reverse move (Q31). */
  direction: string | null;
}

export interface OrderOverlayOptions {
  /** Space held (D2+): current markers and cover icons, for every unit given. */
  all?: boolean;
}

/** Height of the walkable surface (bridge deck where one spans). */
export type SurfaceHeight = (x: number, y: number) => number;

const LIFT_M = 0.3;
/** Marks stacked on one spot rise in these steps, so none z-fights. */
const STACK_M = 0.04;
const DRAPE_STEP_M = 3;
const ROUTE_WIDTH_M = 1.6;
/** A soldier's final-leg line, from the route's end to his spot. */
const LEG_WIDTH_M = 0.25;
const UNIT_RING_M = 4;
const SOLDIER_MARK_M = 0.55;
const COVER_ICON_M = 1.05;
/** A pale rim round each cover icon, so dark green reads on grass. */
const COVER_RIM_M = 0.14;
const COLORS: Record<string, Rgba> = {
  shortest: [1.0, 0.42, 0.85, 1],
  fastest: [0.3, 0.85, 1.0, 1],
  queued: [0.85, 0.85, 0.8, 0.55],
  blocked: [0.95, 0.25, 0.2, 1],
  waiting: [1.0, 0.72, 0.2, 1],
};
/** Soldier and vehicle marks: near-white on a dark backing reads over sunlit
 *  ground and the dusk fog alike. */
const MARK: Rgba = [0.97, 0.97, 0.92, 1];
const CURRENT: Rgba = [0.97, 0.97, 0.92, 0.7];
const BACKING: Rgba = [0.05, 0.06, 0.07, 0.75];
const RIM: Rgba = [0.97, 0.97, 0.92, 1];
const LEG: Rgba = [0.97, 0.97, 0.92, 0.45];
const REVERSE: Rgba = [1.0, 0.72, 0.2, 1];
/** D2+: yellow light, light green medium, dark green heavy. */
export const COVER_COLORS: Record<CoverTierName, Rgba> = {
  light: [1.0, 0.86, 0.2, 1],
  medium: [0.55, 0.92, 0.4, 1],
  heavy: [0.08, 0.5, 0.16, 1],
};

type P2 = readonly [number, number];

function ribbon(
  mesh: MeshBuilder,
  a: P2,
  b: P2,
  z: SurfaceHeight,
  color: Rgba,
  dashed: boolean,
  width = ROUTE_WIDTH_M,
  lift = LIFT_M,
) {
  const dx = b[0] - a[0],
    dy = b[1] - a[1];
  const length = Math.hypot(dx, dy);
  if (length < 1e-6) return;
  const nx = (-dy / length) * (width / 2),
    ny = (dx / length) * (width / 2);
  const steps = Math.max(1, Math.ceil(length / DRAPE_STEP_M));
  for (let k = 0; k < steps; k++) {
    if (dashed && k % 2 === 1) continue;
    const [t0, t1] = [k / steps, (k + 1) / steps];
    const p0: P2 = [a[0] + dx * t0, a[1] + dy * t0];
    const p1: P2 = [a[0] + dx * t1, a[1] + dy * t1];
    const at = (p: P2, sx: number, sy: number) =>
      [p[0] + sx, p[1] + sy, z(p[0] + sx, p[1] + sy) + lift] as const;
    mesh.quad(at(p0, -nx, -ny), at(p1, -nx, -ny), at(p1, nx, ny), at(p0, nx, ny), color);
  }
}

/** A flat ring `width` wide inside `radius`, lying on the surface. */
function ring(
  mesh: MeshBuilder,
  c: P2,
  radius: number,
  width: number,
  z: SurfaceHeight,
  color: Rgba,
  lift = LIFT_M,
) {
  const segments = Math.max(20, Math.ceil(radius * 10));
  groundAnnulus(mesh, c, radius - width, radius, { z, lift, segments, colorIn: color });
}

function disc(
  mesh: MeshBuilder,
  c: P2,
  radius: number,
  z: SurfaceHeight,
  color: Rgba,
  lift: number,
) {
  groundAnnulus(mesh, c, 0, radius, { z, lift, segments: 14, colorIn: color });
}

/** A notched arrowhead on the surface: tip `length` ahead of `c` along
 *  `bearing`, barbs `width` apart at `c`. */
function arrow(
  mesh: MeshBuilder,
  c: P2,
  bearing: number,
  length: number,
  width: number,
  z: SurfaceHeight,
  color: Rgba,
  lift: number,
) {
  const fx = Math.cos(bearing),
    fy = Math.sin(bearing);
  const at = (along: number, side: number) => {
    const x = c[0] + fx * along - fy * side,
      y = c[1] + fy * along + fx * side;
    return [x, y, z(x, y) + lift] as const;
  };
  const [tip, notch] = [at(length, 0), at(length * 0.3, 0)];
  mesh.triangle(tip, at(0, width / 2), notch, color);
  mesh.triangle(tip, notch, at(0, -width / 2), color);
}

function crossMark(mesh: MeshBuilder, c: P2, size: number, z: SurfaceHeight, color: Rgba) {
  const h = size / 2;
  ribbon(mesh, [c[0] - h, c[1] - h], [c[0] + h, c[1] + h], z, color, false);
  ribbon(mesh, [c[0] - h, c[1] + h], [c[0] + h, c[1] - h], z, color, false);
}

/** A soldier's marker: a cover icon (the tier's disc, when he has cover),
 *  a dark backing, and a facing arrowhead. */
function soldierMark(
  mesh: MeshBuilder,
  c: P2,
  facing: number,
  cover: CoverTierName | null,
  z: SurfaceHeight,
  color: Rgba,
) {
  if (cover) {
    ring(mesh, c, COVER_ICON_M + COVER_RIM_M, COVER_RIM_M, z, RIM, LIFT_M);
    disc(mesh, c, COVER_ICON_M, z, COVER_COLORS[cover], LIFT_M);
  }
  // A pale hairline keeps the dark disc's edge on dark fog and on a route.
  ring(mesh, c, SOLDIER_MARK_M + 0.08, 0.08, z, RIM, LIFT_M + STACK_M);
  disc(mesh, c, SOLDIER_MARK_M, z, BACKING, LIFT_M + STACK_M);
  const back: P2 = [
    c[0] - Math.cos(facing) * SOLDIER_MARK_M,
    c[1] - Math.sin(facing) * SOLDIER_MARK_M,
  ];
  arrow(mesh, back, facing, SOLDIER_MARK_M * 2, SOLDIER_MARK_M, z, color, LIFT_M + 2 * STACK_M);
}

/** A facing arrow: pale on a dark backing, above the route it may lie on. */
function facingArrow(mesh: MeshBuilder, at: P2, f: number, z: SurfaceHeight) {
  const inset: P2 = [at[0] - Math.cos(f) * 0.4, at[1] - Math.sin(f) * 0.4];
  arrow(mesh, inset, f, 4.2, 3.2, z, BACKING, LIFT_M + STACK_M);
  arrow(mesh, at, f, 3.4, 2.4, z, MARK, LIFT_M + 2 * STACK_M);
}

/** The unit's final marker: a ring in its route colour with the facing
 *  arrow outside it; a reverse move adds amber chevrons on the side it
 *  backs toward (Q31). */
function finalMark(mesh: MeshBuilder, u: OrderView, c: P2, z: SurfaceHeight, color: Rgba) {
  ring(mesh, c, UNIT_RING_M + 0.3, 1.6, z, BACKING, LIFT_M - STACK_M);
  ring(mesh, c, UNIT_RING_M, 1, z, color);
  const f = u.finalFacing;
  const edge: P2 = [c[0] + Math.cos(f) * UNIT_RING_M, c[1] + Math.sin(f) * UNIT_RING_M];
  facingArrow(mesh, edge, f, z);
  if (u.direction === "reverse") {
    const b = f + Math.PI;
    for (const d of [UNIT_RING_M + 0.4, UNIT_RING_M + 1.8]) {
      const at: P2 = [c[0] + Math.cos(b) * d, c[1] + Math.sin(b) * d];
      arrow(mesh, at, b, 1.6, 3, z, BACKING, LIFT_M + STACK_M);
      arrow(mesh, at, b, 1.2, 2.2, z, REVERSE, LIFT_M + 2 * STACK_M);
    }
  }
}

export function buildOrderOverlay(
  units: readonly OrderView[],
  z: SurfaceHeight,
  { all = false }: OrderOverlayOptions = {},
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const u of units) {
    const here: P2 = [u.position[0], u.position[1]];
    if (all) {
      // Under each soldier (with his cover now) or the vehicle, where it is.
      if (u.members.length === 0) ring(translucent, here, 2.6, 0.6, z, CURRENT);
      u.members.forEach((m, k) =>
        soldierMark(
          translucent,
          [m[0], m[1]],
          u.finalFacing,
          u.memberOrders[k]?.coverNow ?? null,
          z,
          CURRENT,
        ),
      );
    }
    if (!u.goal) {
      // A holding squad's soldiers walking to their posts (cover, a step out).
      if (all)
        u.memberOrders.forEach((m, k) => {
          const p = u.members[k];
          if (p && Math.hypot(m.spot[0] - p[0], m.spot[1] - p[1]) > SOLDIER_MARK_M) {
            ribbon(translucent, [p[0], p[1]], m.spot, z, LEG, false, LEG_WIDTH_M);
            soldierMark(opaque, m.spot, u.finalFacing, m.coverThere, z, MARK);
          }
        });
      continue;
    }
    const color = COLORS[u.policy ?? "shortest"] ?? COLORS.shortest;
    // A vehicle's route stops at its final ring, and short of the facing
    // arrow when that points back along the route (a reverse move).
    const last = u.route.length > 1 ? u.route[u.route.length - 2] : here;
    const inbound = Math.atan2(last[1] - u.goal[1], last[0] - u.goal[0]);
    const backAlong = Math.cos(inbound - u.finalFacing) > Math.cos(Math.PI / 4);
    const clip =
      u.members.length === 0 && u.state !== "route_blocked"
        ? UNIT_RING_M + (backAlong ? 4.6 : 0)
        : 0;
    let from: P2 = here;
    u.route.forEach((p, k) => {
      let to: P2 = p;
      const d = Math.hypot(p[0] - from[0], p[1] - from[1]);
      if (k === u.route.length - 1 && clip > 0 && d > 1e-6) {
        const keep = Math.max(0, d - clip) / d;
        to = [from[0] + (p[0] - from[0]) * keep, from[1] + (p[1] - from[1]) * keep];
      }
      ribbon(opaque, from, to, z, color, false);
      from = p;
    });
    if (u.state === "route_blocked") {
      ribbon(opaque, here, u.goal, z, COLORS.blocked, true);
      crossMark(opaque, u.goal, 6, z, COLORS.blocked);
      ring(opaque, u.goal, 4.5, 1, z, COLORS.blocked);
    } else if (u.members.length === 0) {
      finalMark(opaque, u, u.goal, z, color);
    } else {
      // A squad's final marker is its soldiers' spots, each with the facing.
      // Its facing arrow stands beyond the farthest spot ahead, on no marker.
      const end = u.route.length ? u.route[u.route.length - 1] : u.goal;
      const [fx, fy] = [Math.cos(u.finalFacing), Math.sin(u.finalFacing)];
      const ahead = u.memberOrders.reduce(
        (a, m) => Math.max(a, (m.spot[0] - end[0]) * fx + (m.spot[1] - end[1]) * fy),
        0,
      );
      const facingAt: P2 = [end[0] + fx * (ahead + 2), end[1] + fy * (ahead + 2)];
      facingArrow(opaque, facingAt, u.finalFacing, z);
      for (const m of u.memberOrders) {
        if (all) ribbon(translucent, end, m.spot, z, LEG, false, LEG_WIDTH_M);
        soldierMark(opaque, m.spot, u.finalFacing, all ? m.coverThere : null, z, MARK);
      }
    }
    if (u.state === "waiting") ring(opaque, here, 5, 0.6, z, COLORS.waiting);
    let prev: P2 = u.goal;
    for (const q of u.queue) {
      ribbon(translucent, prev, q, z, COLORS.queued, true);
      ring(translucent, q, 2.8, 0.8, z, COLORS.queued);
      prev = q;
    }
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
