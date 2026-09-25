// Uncertain evidence and remembered obstacles for one side: red contact areas
// that fade toward expiry (drawn over fog, since firing is disclosed whatever
// the line of sight) and the dynamic obstacles the side has learned.
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
/** A wreck reads as burnt-out and a ruin as broken brick, apart from ordinary masonry. */
const PROP: Record<string, Rgba> = {
  wreck: [0.16, 0.15, 0.14, 1],
  ruin: [0.5, 0.36, 0.28, 1],
  default: [0.6, 0.58, 0.55, 1],
};

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

const RUBBLE_CELLS = 6;
/** Broken brick and concrete, darker for the low slab the heaps lie on. */
const RUBBLE_TONES: readonly Rgba[] = [
  [0.52, 0.36, 0.27, 1],
  [0.45, 0.42, 0.38, 1],
  [0.6, 0.45, 0.33, 1],
  [0.38, 0.3, 0.25, 1],
];
const RUBBLE_SLAB: Rgba = [0.3, 0.26, 0.23, 1];

/** A fixed pseudo-random value in [0, 1) per heap and purpose: the same ruin
 *  always draws the same heaps. */
function hash(k: number, salt: number): number {
  const x = Math.sin(k * 12.9898 + salt * 78.233) * 43758.5453;
  return x - Math.floor(x);
}

/** A ruin as a low slab strewn with uneven, turned heaps of rubble: visibly
 *  broken and lower than a building, and never drawn above the collider that
 *  sight and rounds meet. */
function rubble(mesh: MeshBuilder, p: KnownPropShape) {
  const [hx, hy, hz] = p.half;
  const c = Math.cos(p.yaw),
    s = Math.sin(p.yaw);
  const at = (lx: number, ly: number) =>
    [p.center[0] + lx * c - ly * s, p.center[1] + lx * s + ly * c] as const;
  mesh.orientedBox(p.center[0], p.center[1], p.yaw, [hx, hy, hz * 0.2], p.baseZ, RUBBLE_SLAB);
  const [cx, cy] = [(2 * hx) / RUBBLE_CELLS, (2 * hy) / RUBBLE_CELLS];
  for (let i = 0; i < RUBBLE_CELLS; i++) {
    for (let j = 0; j < RUBBLE_CELLS; j++) {
      const k = i * RUBBLE_CELLS + j;
      // Jittered inside its cell, turned, sized and heaped unevenly.
      const lx = -hx + (i + 0.2 + 0.6 * hash(k, 1)) * cx,
        ly = -hy + (j + 0.2 + 0.6 * hash(k, 2)) * cy;
      const size = 0.35 + 0.4 * hash(k, 3);
      const height = hz * (0.3 + 0.7 * hash(k, 4));
      const [x, y] = at(lx, ly);
      mesh.orientedBox(
        x,
        y,
        p.yaw + (hash(k, 5) - 0.5) * 1.6,
        [cx * size, cy * size * (0.6 + 0.6 * hash(k, 6)), height],
        p.baseZ,
        RUBBLE_TONES[Math.floor(hash(k, 7) * RUBBLE_TONES.length)],
      );
    }
  }
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
    if (p.kind === "ruin") rubble(opaque, p);
    else
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
