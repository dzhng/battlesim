// Supply for one side: each supply vehicle's service radius (a solid ring
// once it is fully deployed and can serve, faint while it cannot), and under
// each unit in reach a ring whose shape says its service state: a full ring
// while served, four long dashes while it waits (moving, firing, no stock,
// in a building, truck not set up). Every ring is a thin line with the
// overlay's glow, so it reads at any zoom.
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
/** Recipient rings sit well outside any unit's footprint marker. */
const RECIPIENT_M = 11.5;

/** A thin ring on the surface, `width` wide, broken into dashes when `dashed`. */
function ring(
  mesh: MeshBuilder,
  c: readonly [number, number],
  radius: number,
  width: number,
  color: Rgba,
  z: SurfaceHeight,
  { dashed = false, segments = 64, start = 0 } = {},
) {
  groundAnnulus(mesh, c, radius - width / 2, radius + width / 2, {
    z,
    lift: 0.35,
    segments,
    colorIn: color,
    dashed,
    start,
  });
}

/** Rings `line` metres wide (the orders' line weight, slice 27e), glowing
 *  by the overlay's own halo rather than on a dark band. */
export function buildSupplyOverlay(
  sources: readonly SupplySource[],
  recipients: readonly Recipient[],
  z: SurfaceHeight,
  line: number,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of sources) {
    if (s.ready) ring(opaque, s.center, s.radius, line, SUPPLY_READY, z);
    else ring(translucent, s.center, s.radius, line, SUPPLY_IDLE, z, { dashed: true });
  }
  for (const r of recipients) {
    const serving = r.state === "serving";
    // Waiting: four long dashes, apart from the fine dashes of a truck's reach.
    ring(opaque, r.center, RECIPIENT_M, line, serving ? SERVING : WAITING, z, {
      dashed: !serving,
      segments: serving ? 48 : 16,
      start: Math.PI / 8,
    });
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
