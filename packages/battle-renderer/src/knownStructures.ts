// The props a side remembers: wrecks, ruins and other dynamic obstacles it has
// learned. They are world geometry, drawn in the frame's structures layer
// (lit, fogged, shadow-casting), not display-space overlay.
import { sinHash } from "@packages/renderer-core/src/math";
import { MeshBuilder, type Mesh, type Rgba } from "./mesh";

export interface KnownPropShape {
  kind: string;
  center: readonly [number, number];
  yaw: number;
  half: readonly [number, number, number];
  baseZ: number;
}

/** A wreck reads as burnt-out and a ruin as broken brick, apart from ordinary masonry. */
const PROP: Record<string, Rgba> = {
  wreck: [0.16, 0.15, 0.14, 1],
  ruin: [0.5, 0.36, 0.28, 1],
  default: [0.6, 0.58, 0.55, 1],
};

const RUBBLE_CELLS = 6;
/** Broken brick and concrete, darker for the low slab the heaps lie on. */
const RUBBLE_TONES: readonly Rgba[] = [
  [0.52, 0.36, 0.27, 1],
  [0.45, 0.42, 0.38, 1],
  [0.6, 0.45, 0.33, 1],
  [0.38, 0.3, 0.25, 1],
];
const RUBBLE_SLAB: Rgba = [0.3, 0.26, 0.23, 1];

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
      const lx = -hx + (i + 0.2 + 0.6 * sinHash(k, 1)) * cx,
        ly = -hy + (j + 0.2 + 0.6 * sinHash(k, 2)) * cy;
      const size = 0.35 + 0.4 * sinHash(k, 3);
      const height = hz * (0.3 + 0.7 * sinHash(k, 4));
      const [x, y] = at(lx, ly);
      mesh.orientedBox(
        x,
        y,
        p.yaw + (sinHash(k, 5) - 0.5) * 1.6,
        [cx * size, cy * size * (0.6 + 0.6 * sinHash(k, 6)), height],
        p.baseZ,
        RUBBLE_TONES[Math.floor(sinHash(k, 7) * RUBBLE_TONES.length)],
      );
    }
  }
}

export function buildKnownStructures(knownProps: readonly KnownPropShape[]): Mesh {
  const mesh = new MeshBuilder();
  for (const p of knownProps) {
    if (p.kind === "ruin") rubble(mesh, p);
    else
      mesh.orientedBox(
        p.center[0],
        p.center[1],
        p.yaw,
        p.half,
        p.baseZ,
        PROP[p.kind] ?? PROP.default,
      );
  }
  return mesh.build();
}
