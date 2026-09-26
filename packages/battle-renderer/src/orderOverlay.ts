// Order presentation: remaining routes draped on the walkable surface,
// destination and queued-waypoint markers, and blocked/waiting marks. Built
// only from the observing side's own-unit view.
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { WorldMeshes } from "./scene";

export interface OrderView {
  position: readonly [number, number, number];
  goal: readonly [number, number] | null;
  policy: string | null;
  state: string;
  route: readonly (readonly [number, number])[];
  queue: readonly (readonly [number, number])[];
}

/** Height of the walkable surface (bridge deck where one spans). */
export type SurfaceHeight = (x: number, y: number) => number;

const LIFT_M = 0.3;
const DRAPE_STEP_M = 3;
const ROUTE_WIDTH_M = 1.6;
const COLORS: Record<string, Rgba> = {
  shortest: [1.0, 0.42, 0.85, 1],
  fastest: [0.3, 0.85, 1.0, 1],
  queued: [0.85, 0.85, 0.8, 0.55],
  blocked: [0.95, 0.25, 0.2, 1],
  waiting: [1.0, 0.72, 0.2, 1],
};

type P2 = readonly [number, number];

function ribbon(mesh: MeshBuilder, a: P2, b: P2, z: SurfaceHeight, color: Rgba, dashed: boolean) {
  const dx = b[0] - a[0],
    dy = b[1] - a[1];
  const length = Math.hypot(dx, dy);
  if (length < 1e-6) return;
  const nx = (-dy / length) * (ROUTE_WIDTH_M / 2),
    ny = (dx / length) * (ROUTE_WIDTH_M / 2);
  const steps = Math.max(1, Math.ceil(length / DRAPE_STEP_M));
  for (let k = 0; k < steps; k++) {
    if (dashed && k % 2 === 1) continue;
    const [t0, t1] = [k / steps, (k + 1) / steps];
    const p0: P2 = [a[0] + dx * t0, a[1] + dy * t0];
    const p1: P2 = [a[0] + dx * t1, a[1] + dy * t1];
    const at = (p: P2, sx: number, sy: number) =>
      [p[0] + sx, p[1] + sy, z(p[0] + sx, p[1] + sy) + LIFT_M] as const;
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
) {
  groundAnnulus(mesh, c, radius - width, radius, { z, lift: LIFT_M, segments: 20, colorIn: color });
}

function crossMark(mesh: MeshBuilder, c: P2, size: number, z: SurfaceHeight, color: Rgba) {
  const h = size / 2;
  ribbon(mesh, [c[0] - h, c[1] - h], [c[0] + h, c[1] + h], z, color, false);
  ribbon(mesh, [c[0] - h, c[1] + h], [c[0] + h, c[1] - h], z, color, false);
}

export function buildOrderOverlay(units: readonly OrderView[], z: SurfaceHeight): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const u of units) {
    if (!u.goal) continue;
    const color = COLORS[u.policy ?? "shortest"] ?? COLORS.shortest;
    let from: P2 = [u.position[0], u.position[1]];
    for (const p of u.route) {
      ribbon(opaque, from, p, z, color, false);
      from = p;
    }
    if (u.state === "route_blocked") {
      ribbon(opaque, [u.position[0], u.position[1]], u.goal, z, COLORS.blocked, true);
      crossMark(opaque, u.goal, 6, z, COLORS.blocked);
      ring(opaque, u.goal, 4.5, 1, z, COLORS.blocked);
    } else {
      ring(opaque, u.goal, 4, 1, z, color);
    }
    if (u.state === "waiting")
      ring(opaque, [u.position[0], u.position[1]], 5, 0.6, z, COLORS.waiting);
    let prev: P2 = u.goal;
    for (const q of u.queue) {
      ribbon(translucent, prev, q, z, COLORS.queued, true);
      ring(translucent, q, 2.8, 0.8, z, COLORS.queued);
      prev = q;
    }
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
