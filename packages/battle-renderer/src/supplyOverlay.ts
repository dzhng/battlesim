// Supply for one side: each supply vehicle's service radius (a solid ring
// once it is fully deployed and can serve, faint while it cannot), and under
// each unit in reach a ring whose shape says its service state: a full ring
// while served, four long dashes while it waits (moving, firing, no stock,
// in a building, truck not set up), each on a dark band so it reads at any
// zoom.
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
export const SERVING: Rgba = [0.35, 0.92, 1.0, 1];
export const WAITING: Rgba = [0.35, 0.92, 1.0, 1];
/** A dark band under each recipient ring, so it reads on any ground. */
const BACKING: Rgba = [0.03, 0.07, 0.1, 0.75];
/** Recipient rings sit well outside any unit's footprint marker. */
const RECIPIENT_INNER_M = 10.5;
const RECIPIENT_OUTER_M = 12.5;
const BACKING_M = 0.6;

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
    groundAnnulus(
      translucent,
      r.center,
      RECIPIENT_INNER_M - BACKING_M,
      RECIPIENT_OUTER_M + BACKING_M,
      {
        z,
        lift: 0.3,
        segments: 48,
        colorIn: BACKING,
      },
    );
    // Waiting: four long dashes, apart from the fine dashes of a truck's reach.
    groundAnnulus(opaque, r.center, RECIPIENT_INNER_M, RECIPIENT_OUTER_M, {
      z,
      lift: 0.4,
      segments: serving ? 48 : 16,
      colorIn: serving ? SERVING : WAITING,
      dashed: !serving,
      start: Math.PI / 8,
    });
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
