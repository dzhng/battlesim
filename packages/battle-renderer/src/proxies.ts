import { MeshBuilder, type Mesh, type Rgba } from "./mesh";

// Generated primitive silhouettes. Canonical local frame: forward +X, up +Z,
// ground contact at z = 0. Each record declares its visual bounds and selection
// anchor; future meshes replace the triangles without changing either the
// simulation colliders or these records' meaning. A soldier has no mesh here:
// the models layer draws him, and `infantry` is only his pick box.

export type ProxyKind = "tank" | "infantry" | "supply" | "outrigger" | "mast" | "box" | "marker";

export interface ProxyAsset {
  /** Visual half extents in the local frame (x forward, y left, z up), centred at `center`. */
  halfExtents: readonly [number, number, number];
  center: readonly [number, number, number];
  /** Local point that selection rings and world-anchored readouts attach to. */
  anchor: readonly [number, number, number];
}

const HULL: Rgba = [0.86, 0.86, 0.84, 1];
const DARK: Rgba = [0.5, 0.52, 0.5, 1];
const CUE: Rgba = [0.95, 0.85, 0.35, 1];

// Asymmetric tank: long hull, turret set back, barrel forward, glacis cue.
const tank = new MeshBuilder()
  .box(0, 0, 0.7, 3.5, 1.8, 0.7, HULL)
  .box(-0.6, 0, 1.8, 1.5, 1.1, 0.4, DARK)
  .box(1.9 + 0.9, 0, 1.85, 2.4, 0.12, 0.12, DARK)
  .wedge(3.5, 4.3, 1.2, 0.1, 1.1, CUE)
  .build();

// Truck: cab forward, cargo box behind.
const supply = new MeshBuilder()
  .box(-0.8, 0, 1.3, 2.2, 1.4, 1.0, HULL)
  .box(2.2, 0, 1.0, 0.8, 1.3, 0.8, DARK)
  .wedge(3.0, 3.5, 0.9, 0.3, 1.2, CUE)
  .build();

// Deployment parts, placed by progress (see deploymentParts). A stabiliser leg
// points along local +X from the hull: a beam ending in a foot pad at x = 0.
const outrigger = new MeshBuilder()
  .box(-0.7, 0, 0.55, 0.7, 0.14, 0.12, DARK)
  .box(0, 0, 0.3, 0.28, 0.28, 0.3, CUE)
  .build();

// Telescoping mast with a lamp head, standing on its foot.
const mast = new MeshBuilder()
  .box(0, 0, 0.95, 0.1, 0.1, 0.95, DARK)
  .box(0, 0, 2.0, 0.25, 0.25, 0.12, CUE)
  .build();

const box = new MeshBuilder().box(0, 0, 0.6, 0.6, 0.6, 0.6, [0.72, 0.62, 0.5, 1]).build();

// Probe pin: a thin post with a head, anchored at its foot.
const marker = new MeshBuilder()
  .box(0, 0, 3, 0.15, 0.15, 3, [0.95, 0.3, 0.25, 1])
  .box(0, 0, 6.4, 0.7, 0.7, 0.4, [0.95, 0.3, 0.25, 1])
  .build();

/** The kinds drawn as primitives; `infantry` is a pick box only. */
export type DrawnProxyKind = Exclude<ProxyKind, "infantry">;

export const PROXY_MESHES: Record<DrawnProxyKind, Mesh> = {
  tank,
  supply,
  outrigger,
  mast,
  box,
  marker,
};

export const PROXY_ASSETS: Record<ProxyKind, ProxyAsset> = {
  tank: { halfExtents: [3.9, 1.8, 1.1], center: [0.4, 0, 1.1], anchor: [0, 0, 2.6] },
  infantry: { halfExtents: [0.4, 0.22, 0.9], center: [0.2, 0, 0.9], anchor: [0, 0, 2.0] },
  supply: { halfExtents: [3.25, 1.4, 1.15], center: [0.25, 0, 1.15], anchor: [0, 0, 2.6] },
  outrigger: { halfExtents: [0.7, 0.28, 0.35], center: [-0.5, 0, 0.35], anchor: [0, 0, 1.0] },
  mast: { halfExtents: [0.25, 0.25, 1.06], center: [0, 0, 1.06], anchor: [0, 0, 2.3] },
  box: { halfExtents: [0.6, 0.6, 0.6], center: [0, 0, 0.6], anchor: [0, 0, 1.4] },
  marker: { halfExtents: [0.7, 0.7, 3.4], center: [0, 0, 3.4], anchor: [0, 0, 7.2] },
};
