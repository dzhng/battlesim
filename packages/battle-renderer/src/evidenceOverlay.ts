// Uncertain evidence and remembered obstacles for one side: red contact areas
// that fade toward expiry (drawn over fog, since firing is disclosed whatever
// the line of sight) and the dynamic obstacles the side has learned.
import { MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

export interface ContactShape {
  center: readonly [number, number];
  radius: number;
  /** Remaining life in [0, 1]; the area fades as it ages. */
  freshness: number;
  source: string;
}

export interface KnownPropShape {
  kind: string;
  center: readonly [number, number];
  yaw: number;
  half: readonly [number, number, number];
  baseZ: number;
}

const LIFT_M = 0.4;
const SEGMENTS = 48;
const FIRING: Rgba = [1.0, 0.18, 0.12, 1];
const LAST_SEEN: Rgba = [1.0, 0.78, 0.2, 1];
/** A wreck reads as burnt-out, apart from ordinary masonry. */
const PROP: Record<string, Rgba> = { wreck: [0.16, 0.15, 0.14, 1], default: [0.6, 0.58, 0.55, 1] };

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
  const at = (a: number, r: number) => {
    const x = c.center[0] + Math.cos(a) * r,
      y = c.center[1] + Math.sin(a) * r;
    return [x, y, z(x, y) + LIFT_M] as const;
  };
  const tone = (alpha: number): Rgba => [base[0], base[1], base[2], alpha];
  for (let k = 0; k < SEGMENTS; k++) {
    const [t0, t1] = [(k / SEGMENTS) * Math.PI * 2, ((k + 1) / SEGMENTS) * Math.PI * 2];
    mesh.shadedTriangle(at(t0, inner), at(t1, inner), at(t1, outer), tone(a0), tone(a0), tone(a1));
    mesh.shadedTriangle(at(t0, inner), at(t1, outer), at(t0, outer), tone(a0), tone(a1), tone(a1));
  }
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
  knownProps: readonly KnownPropShape[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const c of contacts) area(translucent, c, z);
  for (const p of knownProps) {
    opaque.orientedBox(
      p.center[0],
      p.center[1],
      p.yaw,
      p.half,
      p.baseZ,
      PROP[p.kind] ?? PROP.default,
    );
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
