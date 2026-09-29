// One side's own guided missiles: the missile, the line to the point it is
// steering at, and the path it has flown. Cyan while its launcher guides it
// (the point follows the target as the launcher sees it); amber once released
// (the point is fixed for good, on the ground, marked with a cross).
import { EMPTY_MESH, groundAnnulus, MeshBuilder, rgbA, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";
import type { Vec3 } from "math";

export interface GuidedShape {
  position: Readonly<Vec3>;
  point: Readonly<Vec3>;
  supported: boolean;
  /** Positions flown so far, oldest first. */
  trail: readonly Readonly<Vec3>[];
}

const GUIDED: Rgba = [0.3, 0.9, 1.0, 1];
const RELEASED: Rgba = [1.0, 0.7, 0.15, 1];
const TRAIL: Rgba = [0.85, 0.85, 0.85, 0.5];
const SEGMENTS = 32;

export function buildGuidanceOverlay(
  missiles: readonly GuidedShape[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const m of missiles) {
    const color = m.supported ? GUIDED : RELEASED;
    for (let k = 1; k < m.trail.length; k++)
      translucent.segment(m.trail[k - 1], m.trail[k], 0.15, TRAIL);
    opaque.box(m.position[0], m.position[1], m.position[2], 1.5, 1.5, 1.5, color);
    translucent.segment(m.position, m.point, 0.2, rgbA(color, 0.7));
    if (!m.supported) {
      // The fixed last point: a ground cross, apart from any unit's ring.
      const [cx, cy] = m.point;
      const zc = z(cx, cy) + 0.35;
      for (const [dx, dy] of [
        [1, 1],
        [1, -1],
      ]) {
        opaque.segment([cx - dx * 5, cy - dy * 5, zc], [cx + dx * 5, cy + dy * 5, zc], 0.6, color);
      }
    } else {
      opaque.box(m.point[0], m.point[1], m.point[2], 0.4, 0.4, 0.4, color);
    }
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}

/** A ground ring naming an own unit on the map (launchers, the scout). */
export function buildUnitMarks(
  marks: readonly { at: readonly [number, number]; color: Rgba }[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  for (const { at, color } of marks)
    groundAnnulus(opaque, at, 8, 10, { z, lift: 0.3, segments: SEGMENTS, colorIn: color });
  return { opaque: opaque.build(), translucent: EMPTY_MESH };
}
