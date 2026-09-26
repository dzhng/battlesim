// Uncertain evidence for one side: red contact areas that fade toward expiry,
// drawn over fog, since firing is disclosed whatever the line of sight. The
// obstacles the side has learned are world geometry (`knownStructures.ts`).
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

export interface ContactShape {
  center: readonly [number, number];
  radius: number;
  /** Remaining life in [0, 1]; the area fades as it ages. */
  freshness: number;
  source: string;
}

const LIFT_M = 0.4;
const SEGMENTS = 48;
const FIRING: Rgba = [1.0, 0.18, 0.12, 1];
const LAST_SEEN: Rgba = [1.0, 0.78, 0.2, 1];

/** Annulus from `inner` to `outer`, alpha ramping from `a0` to `a1` outward. */
function band(
  mesh: MeshBuilder,
  c: ContactShape,
  inner: number,
  outer: number,
  base: Rgba,
  a0: number,
  a1: number,
  z: SurfaceHeight,
) {
  groundAnnulus(mesh, c.center, inner, outer, {
    z,
    lift: LIFT_M,
    segments: SEGMENTS,
    colorIn: [base[0], base[1], base[2], a0],
    colorOut: [base[0], base[1], base[2], a1],
  });
}

/** An area, not a point: firing is an even fill whose rim fades out, with no
 *  centre to aim at; a last sighting is a hollow ring. Both fade with age. */
function area(mesh: MeshBuilder, c: ContactShape, z: SurfaceHeight) {
  const life = 0.35 + 0.65 * c.freshness;
  if (c.source === "last_seen") {
    band(mesh, c, c.radius * 0.88, c.radius, LAST_SEEN, 0.55 * life, 0.55 * life, z);
    return;
  }
  const fill = 0.3 * life;
  band(mesh, c, 0, c.radius * 0.85, FIRING, fill, fill, z);
  band(mesh, c, c.radius * 0.85, c.radius, FIRING, fill, 0, z);
}

export function buildEvidenceOverlay(
  contacts: readonly ContactShape[],
  z: SurfaceHeight,
): WorldMeshes {
  const translucent = new MeshBuilder();
  for (const c of contacts) area(translucent, c, z);
  return { opaque: new Float32Array(0), translucent: translucent.build() };
}
