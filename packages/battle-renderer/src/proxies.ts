import { MeshBuilder, type Mesh } from "./mesh";

// Generated primitive silhouettes. Canonical local frame: forward +X, up +Z,
// ground contact at z = 0. Each record declares its visual bounds and selection
// anchor; future meshes replace the triangles without changing either the
// simulation colliders or these records' meaning. Soldiers and vehicles have
// no mesh here: the models layer draws them, and they are picked by the
// simulation's bodies (`picking.ts`). `infantry` is only a lab's pick box.

export type ProxyKind = "infantry" | "box" | "marker";

export interface ProxyAsset {
  /** Visual half extents in the local frame (x forward, y left, z up), centred at `center`. */
  halfExtents: readonly [number, number, number];
  center: readonly [number, number, number];
  /** Local point that selection rings and world-anchored readouts attach to. */
  anchor: readonly [number, number, number];
}

const box = new MeshBuilder().box(0, 0, 0.6, 0.6, 0.6, 0.6, [0.72, 0.62, 0.5, 1]).build();

// Probe pin: a thin post with a head, anchored at its foot.
const marker = new MeshBuilder()
  .box(0, 0, 3, 0.15, 0.15, 3, [0.95, 0.3, 0.25, 1])
  .box(0, 0, 6.4, 0.7, 0.7, 0.4, [0.95, 0.3, 0.25, 1])
  .build();

/** The kinds drawn as primitives; `infantry` is a pick box only. */
export type DrawnProxyKind = Exclude<ProxyKind, "infantry">;

export const PROXY_MESHES: Record<DrawnProxyKind, Mesh> = { box, marker };

export const PROXY_ASSETS: Record<ProxyKind, ProxyAsset> = {
  infantry: { halfExtents: [0.4, 0.22, 0.9], center: [0.2, 0, 0.9], anchor: [0, 0, 2.0] },
  box: { halfExtents: [0.6, 0.6, 0.6], center: [0, 0, 0.6], anchor: [0, 0, 1.4] },
  marker: { halfExtents: [0.7, 0.7, 3.4], center: [0, 0, 3.4], anchor: [0, 0, 7.2] },
};
