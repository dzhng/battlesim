// Deployment progress on the ground: a thin light track around the unit and a
// bright arc, clockwise from its nose, whose length is how deployed the unit
// is (the one published progress value). A white arrowhead at the arc's end
// points the way progress is moving, so direction never rests on colour alone;
// full deployment adds a solid disc. Built only from the observing side's
// own-unit view; it reads state, never sets it. Drawn above route ribbons.
import { MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";

export interface DeploymentIndicator {
  position: readonly [number, number, number];
  yaw: number;
  /** In [0, 1]: 0 packed, 1 fully deployed. */
  progress: number;
  /** The end state progress heads to ("deployed" or "packed"). */
  target: string;
}

const RADIUS_M = 6;
const WIDTH_M = 1.1;
const TRACK_WIDTH_M = 0.3;
/** Above the order overlay's route ribbons (0.3 m), so a path never hides the arc. */
const LIFT_M = 0.7;
const DISC_LIFT_M = 0.15;
const ARROW_LENGTH_M = 1.8;
const ARROW_OVERHANG_M = 0.45;
const SEGMENTS = 48;

export const DEPLOYMENT_COLORS: Record<
  "track" | "deploying" | "deployed" | "packing" | "arrow" | "disc",
  Rgba
> = {
  track: [0.62, 0.65, 0.68, 1],
  deploying: [0.35, 0.9, 0.45, 1],
  deployed: [0.2, 1.0, 0.6, 1],
  packing: [1.0, 0.62, 0.15, 1],
  arrow: [0.97, 0.97, 0.97, 1],
  disc: [0.1, 0.42, 0.24, 1],
};

type P3 = readonly [number, number, number];

/** Ground point at clockwise angle `a` from the nose and radius `r`. */
function at(u: DeploymentIndicator, a: number, r: number, lift: number, z: SurfaceHeight): P3 {
  const angle = u.yaw - a;
  const x = u.position[0] + Math.cos(angle) * r,
    y = u.position[1] + Math.sin(angle) * r;
  return [x, y, z(x, y) + lift];
}

function arc(
  mesh: MeshBuilder,
  u: DeploymentIndicator,
  from: number,
  to: number,
  inner: number,
  outer: number,
  lift: number,
  z: SurfaceHeight,
  color: Rgba,
) {
  const steps = Math.max(1, Math.ceil(((to - from) / (Math.PI * 2)) * SEGMENTS));
  for (let k = 0; k < steps; k++) {
    const a0 = from + ((to - from) * k) / steps;
    const a1 = from + ((to - from) * (k + 1)) / steps;
    mesh.quad(
      at(u, a0, outer, lift, z),
      at(u, a1, outer, lift, z),
      at(u, a1, inner, lift, z),
      at(u, a0, inner, lift, z),
      color,
    );
  }
}

export function buildDeploymentOverlay(units: readonly DeploymentIndicator[], z: SurfaceHeight) {
  const mesh = new MeshBuilder();
  const inner = RADIUS_M - WIDTH_M;
  const mid = RADIUS_M - WIDTH_M / 2;
  const full = Math.PI * 2;
  for (const u of units) {
    const p = Math.min(1, Math.max(0, u.progress));
    arc(
      mesh,
      u,
      0,
      full,
      mid - TRACK_WIDTH_M / 2,
      mid + TRACK_WIDTH_M / 2,
      LIFT_M,
      z,
      DEPLOYMENT_COLORS.track,
    );
    if (p >= 1 && u.target === "deployed") {
      // Finished: a solid disc under the unit, plus the full arc.
      const c = at(u, 0, 0, DISC_LIFT_M, z);
      for (let k = 0; k < SEGMENTS; k++) {
        mesh.triangle(
          c,
          at(u, (full * (k + 1)) / SEGMENTS, inner, DISC_LIFT_M, z),
          at(u, (full * k) / SEGMENTS, inner, DISC_LIFT_M, z),
          DEPLOYMENT_COLORS.disc,
        );
      }
    }
    if (p > 0) {
      const color =
        u.target === "packed"
          ? DEPLOYMENT_COLORS.packing
          : p >= 1
            ? DEPLOYMENT_COLORS.deployed
            : DEPLOYMENT_COLORS.deploying;
      arc(mesh, u, 0, full * p, inner, RADIUS_M, LIFT_M + 0.05, z, color);
    }
    // Arrowhead at the arc's moving end, pointing the way progress runs:
    // clockwise while deploying, back along the arc while packing.
    const moving = u.target === "deployed" ? p < 1 : p > 0;
    if (!moving) continue;
    const end = full * p;
    const sweep = (ARROW_LENGTH_M / mid) * (u.target === "deployed" ? 1 : -1);
    mesh.triangle(
      at(u, end, inner - ARROW_OVERHANG_M, LIFT_M + 0.1, z),
      at(u, end, RADIUS_M + ARROW_OVERHANG_M, LIFT_M + 0.1, z),
      at(u, end + sweep, mid, LIFT_M + 0.1, z),
      DEPLOYMENT_COLORS.arrow,
    );
  }
  return mesh.build();
}
