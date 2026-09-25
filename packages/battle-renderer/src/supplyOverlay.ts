// Supply for one side: each supply vehicle's service radius (a solid ring
// once it is fully deployed and can serve, faint while it cannot), and under
// each unit in reach a ring whose shape says its service state: a full ring
// while served, a broken ring while it waits (moving, firing, no stock).
import { MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

type P3 = readonly [number, number, number];

export interface SupplySource {
  center: readonly [number, number];
  radius: number;
  ready: boolean;
}

export interface Recipient {
  center: readonly [number, number];
  /** "serving" draws a full ring; "waiting" a broken one. */
  state: "serving" | "waiting";
}

// Hues kept apart from the deployment ring's green and orange; served and
// waiting differ by shape (full vs broken), not by colour alone.
export const SUPPLY_READY: Rgba = [0.95, 0.95, 0.98, 1];
export const SUPPLY_IDLE: Rgba = [0.75, 0.78, 0.82, 0.45];
export const SERVING: Rgba = [0.55, 0.88, 1.0, 1];
export const WAITING: Rgba = [0.55, 0.88, 1.0, 1];
const SEGMENTS = 64;

function ring(
  mesh: MeshBuilder,
  c: readonly [number, number],
  inner: number,
  outer: number,
  color: Rgba,
  z: SurfaceHeight,
  gaps = false,
) {
  const at = (a: number, r: number): P3 => {
    const x = c[0] + Math.cos(a) * r,
      y = c[1] + Math.sin(a) * r;
    return [x, y, z(x, y) + 0.35];
  };
  for (let k = 0; k < SEGMENTS; k++) {
    if (gaps && k % 4 >= 2) continue;
    const [a0, a1] = [(k / SEGMENTS) * Math.PI * 2, ((k + 1) / SEGMENTS) * Math.PI * 2];
    mesh.quad(at(a0, inner), at(a1, inner), at(a1, outer), at(a0, outer), color);
  }
}

export function buildSupplyOverlay(
  sources: readonly SupplySource[],
  recipients: readonly Recipient[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of sources) {
    if (s.ready) ring(opaque, s.center, s.radius - 1.2, s.radius, SUPPLY_READY, z);
    else ring(translucent, s.center, s.radius - 0.8, s.radius, SUPPLY_IDLE, z, true);
  }
  for (const r of recipients) {
    const serving = r.state === "serving";
    ring(opaque, r.center, 7, 8.5, serving ? SERVING : WAITING, z, !serving);
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
