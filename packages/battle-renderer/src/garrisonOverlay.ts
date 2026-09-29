// Garrison presentation for one side, from its own units' published state: a
// pad under each occupant at its perimeter slot (the small exposed region a
// round can hit; anything wide of it meets the wall): where each soldier of
// a squad holding a building stands. Entering, leaving and being pinned are
// the squad's info panel's.
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";
import type { Vec3 } from "math";

export interface GarrisonMark {
  /** Occupant positions: their perimeter slots while inside. */
  members: readonly Readonly<Vec3>[];
  phase: string;
}

export const OCCUPANT_PAD: Rgba = [0.55, 0.78, 1.0, 0.55];
const PAD_EDGE: Rgba = [0.08, 0.12, 0.2, 1];
const SEGMENTS = 32;
const LIFT_M = 0.35;
// The pad stays inside the slot's standoff from the wall, so the wall never
// hides half of it.
const PAD_M = 0.3;
const PAD_EDGE_M = 0.14;

/** A flat disc or ring round (`cx`, `cy`). */
function disc(
  mesh: MeshBuilder,
  cx: number,
  cy: number,
  inner: number,
  outer: number,
  color: Rgba,
  z: SurfaceHeight,
) {
  groundAnnulus(mesh, [cx, cy], inner, outer, {
    z,
    lift: LIFT_M,
    segments: SEGMENTS,
    colorIn: color,
  });
}

export function buildGarrisonOverlay(
  squads: readonly GarrisonMark[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of squads) {
    if (s.phase !== "inside" && s.phase !== "exiting") continue;
    for (const [x, y] of s.members) {
      disc(translucent, x, y, 0, PAD_M, OCCUPANT_PAD, z);
      disc(opaque, x, y, PAD_M, PAD_M + PAD_EDGE_M, PAD_EDGE, z);
    }
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
