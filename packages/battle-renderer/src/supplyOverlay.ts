// Supply for one side: each supply vehicle's service radius (a solid ring
// once it is fully deployed and can serve, faint while it cannot), and under
// each unit in reach a ring whose shape says its service state: a full ring
// while served, a broken ring while it waits (moving, firing, no stock).
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

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

/** A flat ring on the surface, broken into dashes when `dashed`. */
function ring(
  mesh: MeshBuilder,
  c: readonly [number, number],
  inner: number,
  outer: number,
  color: Rgba,
  z: SurfaceHeight,
  dashed = false,
) {
  groundAnnulus(mesh, c, inner, outer, { z, lift: 0.35, segments: 64, colorIn: color, dashed });
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
