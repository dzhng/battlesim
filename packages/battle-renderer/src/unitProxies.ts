import type { ProxyKind } from "./proxies";

/** Which primitive stands in for a unit kind. Presentation only. */
export function proxyForUnit(kind: string): ProxyKind {
  switch (kind) {
    case "tank":
      return "tank";
    case "supply":
      return "supply";
    default:
      return "infantry";
  }
}

export const SIDE_COLORS = {
  blue: [0.55, 0.7, 1.0],
  red: [1.0, 0.55, 0.5],
} as const;

/** A deployment part's placement, in the same terms as a scene instance. */
export interface PartPlacement {
  kind: "outrigger" | "mast";
  x: number;
  y: number;
  z: number;
  yaw: number;
}

// Supply truck parts in its local frame (x forward, y left): stabiliser legs
// slide out from under the cargo bed and the mast rises out of it. Packed,
// every part sits inside the hull; the pose is a pure function of progress.
const LEG_STATIONS_X = [-2.4, 0.8];
const LEG_PACKED_Y = 1.0;
const LEG_DEPLOYED_Y = 2.5;
const LEG_PACKED_LIFT = 0.35;
const MAST_X = -0.8;
const MAST_PACKED_Z = 0.17;
const MAST_DEPLOYED_Z = 2.3;

/** The folded/unfolded pose of a deploying unit at `progress` in [0, 1],
 *  from its ground contact centre and heading. Presentation only. */
export function deploymentParts(
  base: readonly [number, number, number],
  yaw: number,
  progress: number,
): PartPlacement[] {
  const p = Math.min(1, Math.max(0, progress));
  const c = Math.cos(yaw),
    s = Math.sin(yaw);
  const at = (lx: number, ly: number) => [base[0] + lx * c - ly * s, base[1] + lx * s + ly * c];
  const reach = LEG_PACKED_Y + (LEG_DEPLOYED_Y - LEG_PACKED_Y) * p;
  const parts: PartPlacement[] = [];
  for (const lx of LEG_STATIONS_X) {
    for (const side of [1, -1]) {
      const [x, y] = at(lx, side * reach);
      const z = base[2] + LEG_PACKED_LIFT * (1 - p);
      parts.push({ kind: "outrigger", x, y, z, yaw: yaw + (side * Math.PI) / 2 });
    }
  }
  const [mx, my] = at(MAST_X, 0);
  parts.push({
    kind: "mast",
    x: mx,
    y: my,
    z: base[2] + MAST_PACKED_Z + (MAST_DEPLOYED_Z - MAST_PACKED_Z) * p,
    yaw,
  });
  return parts;
}
