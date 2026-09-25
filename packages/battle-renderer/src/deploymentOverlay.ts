// Deployment progress on the ground: a dim full-circle track around the unit
// and a bright arc, clockwise from its nose, whose length is the one published
// progress value. The arc's colour says which way progress is heading. Built
// only from the observing side's own-unit view; it reads state, never sets it.
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
const LIFT_M = 0.35;
const SEGMENTS = 48;

export const DEPLOYMENT_COLORS: Record<"track" | "deploying" | "deployed" | "packing", Rgba> = {
  track: [0.12, 0.14, 0.16, 1],
  deploying: [0.35, 0.9, 0.45, 1],
  deployed: [0.2, 1.0, 0.6, 1],
  packing: [1.0, 0.62, 0.15, 1],
};

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
  // Clockwise seen from above, starting at the unit's nose.
  const pt = (a: number, r: number) => {
    const angle = u.yaw - a;
    const x = u.position[0] + Math.cos(angle) * r,
      y = u.position[1] + Math.sin(angle) * r;
    return [x, y, z(x, y) + lift] as const;
  };
  for (let k = 0; k < steps; k++) {
    const a0 = from + ((to - from) * k) / steps;
    const a1 = from + ((to - from) * (k + 1)) / steps;
    mesh.quad(pt(a0, outer), pt(a1, outer), pt(a1, inner), pt(a0, inner), color);
  }
}

export function buildDeploymentOverlay(units: readonly DeploymentIndicator[], z: SurfaceHeight) {
  const mesh = new MeshBuilder();
  for (const u of units) {
    const p = Math.min(1, Math.max(0, u.progress));
    const full = Math.PI * 2;
    arc(mesh, u, 0, full, RADIUS_M - WIDTH_M, RADIUS_M, LIFT_M, z, DEPLOYMENT_COLORS.track);
    if (p <= 0) continue;
    const color =
      u.target === "packed"
        ? DEPLOYMENT_COLORS.packing
        : p >= 1
          ? DEPLOYMENT_COLORS.deployed
          : DEPLOYMENT_COLORS.deploying;
    // Drawn slightly narrower and above the track so the two never z-fight.
    const inset = WIDTH_M * 0.12;
    arc(
      mesh,
      u,
      0,
      full * p,
      RADIUS_M - WIDTH_M + inset,
      RADIUS_M - inset,
      LIFT_M + 0.05,
      z,
      color,
    );
  }
  return mesh.build();
}
