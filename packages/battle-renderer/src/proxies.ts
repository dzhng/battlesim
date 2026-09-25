// Generated primitive silhouettes. Canonical local frame: forward +X, up +Z,
// ground contact at z = 0. Each record declares its visual bounds and selection
// anchor; future meshes replace the triangles without changing either the
// simulation colliders or these records' meaning.

export type ProxyKind = "tank" | "infantry" | "supply" | "box";

export interface ProxyAsset {
  /** Visual half extents in the local frame (x forward, y left, z up), centred at `center`. */
  halfExtents: readonly [number, number, number];
  center: readonly [number, number, number];
  /** Local point that selection rings and world-anchored readouts attach to. */
  anchor: readonly [number, number, number];
}

/** Non-indexed triangle list: xyz position, xyz normal, rgb colour per vertex. */
export interface MeshData {
  positions: Float32Array;
  normals: Float32Array;
  colors: Float32Array;
}

export const VERTEX_FLOATS = 9;

type Rgb = readonly [number, number, number];

class MeshBuilder {
  private readonly out: number[] = [];

  /** Axis-aligned box in the local frame. */
  box(cx: number, cy: number, cz: number, hx: number, hy: number, hz: number, color: Rgb) {
    const c = [
      [cx - hx, cy - hy, cz - hz],
      [cx + hx, cy - hy, cz - hz],
      [cx + hx, cy + hy, cz - hz],
      [cx - hx, cy + hy, cz - hz],
      [cx - hx, cy - hy, cz + hz],
      [cx + hx, cy - hy, cz + hz],
      [cx + hx, cy + hy, cz + hz],
      [cx - hx, cy + hy, cz + hz],
    ];
    const faces: [number[], Rgb][] = [
      [
        [0, 3, 2, 1],
        [0, 0, -1],
      ],
      [
        [4, 5, 6, 7],
        [0, 0, 1],
      ],
      [
        [0, 1, 5, 4],
        [0, -1, 0],
      ],
      [
        [2, 3, 7, 6],
        [0, 1, 0],
      ],
      [
        [1, 2, 6, 5],
        [1, 0, 0],
      ],
      [
        [3, 0, 4, 7],
        [-1, 0, 0],
      ],
    ];
    for (const [q, n] of faces) {
      for (const i of [q[0], q[1], q[2], q[0], q[2], q[3]]) this.vertex(c[i], n, color);
    }
    return this;
  }

  /** Upward wedge on the +X face: the facing cue. Apex at `tipX`. */
  wedge(baseX: number, tipX: number, hy: number, z0: number, z1: number, color: Rgb) {
    const a = [baseX, -hy, z0],
      b = [baseX, hy, z0],
      c = [baseX, hy, z1],
      d = [baseX, -hy, z1],
      t0 = [tipX, 0, z0],
      t1 = [tipX, 0, z1];
    const tri = (p: number[], q: number[], r: number[]) => {
      const u = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
      const v = [r[0] - p[0], r[1] - p[1], r[2] - p[2]];
      const n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
      const l = Math.hypot(n[0], n[1], n[2]) || 1;
      for (const p0 of [p, q, r]) this.vertex(p0, [n[0] / l, n[1] / l, n[2] / l], color);
    };
    tri(a, t0, t1);
    tri(a, t1, d);
    tri(t0, b, c);
    tri(t0, c, t1);
    tri(d, t1, c);
    tri(a, b, t0);
    return this;
  }

  private vertex(p: readonly number[], n: readonly number[], c: Rgb) {
    this.out.push(p[0], p[1], p[2], n[0], n[1], n[2], c[0], c[1], c[2]);
  }

  /** Interleaved vertex floats (VERTEX_FLOATS per vertex). */
  build(): Float32Array<ArrayBuffer> {
    return Float32Array.from(this.out);
  }
}

const HULL: Rgb = [0.62, 0.66, 0.6];
const DARK: Rgb = [0.42, 0.44, 0.42];
const CUE: Rgb = [0.95, 0.85, 0.35];

// Asymmetric tank: long hull, turret set back, barrel forward, glacis cue.
const tank = new MeshBuilder()
  .box(0, 0, 0.7, 3.5, 1.8, 0.7, HULL)
  .box(-0.6, 0, 1.8, 1.5, 1.1, 0.4, DARK)
  .box(1.9 + 0.9, 0, 1.85, 2.4, 0.12, 0.12, DARK)
  .wedge(3.5, 4.3, 1.2, 0.1, 1.1, CUE)
  .build();

// Upright soldier proxy with a chest-height nose so facing reads from above.
const infantry = new MeshBuilder()
  .box(0, 0, 0.8, 0.17, 0.22, 0.8, HULL)
  .box(0, 0, 1.7, 0.12, 0.12, 0.11, DARK)
  .wedge(0.17, 0.75, 0.22, 1.0, 1.45, CUE)
  .build();

// Truck: cab forward, cargo box behind.
const supply = new MeshBuilder()
  .box(-0.8, 0, 1.3, 2.2, 1.4, 1.0, HULL)
  .box(2.2, 0, 1.0, 0.8, 1.3, 0.8, DARK)
  .wedge(3.0, 3.5, 0.9, 0.3, 1.2, CUE)
  .build();

const box = new MeshBuilder().box(0, 0, 0.6, 0.6, 0.6, 0.6, [0.72, 0.62, 0.5]).build();

export const PROXY_MESHES: Record<ProxyKind, Float32Array<ArrayBuffer>> = {
  tank,
  infantry,
  supply,
  box,
};

export const PROXY_ASSETS: Record<ProxyKind, ProxyAsset> = {
  tank: { halfExtents: [3.9, 1.8, 1.1], center: [0.4, 0, 1.1], anchor: [0, 0, 2.6] },
  infantry: { halfExtents: [0.4, 0.22, 0.9], center: [0.2, 0, 0.9], anchor: [0, 0, 2.0] },
  supply: { halfExtents: [3.25, 1.4, 1.15], center: [0.25, 0, 1.15], anchor: [0, 0, 2.6] },
  box: { halfExtents: [0.6, 0.6, 0.6], center: [0, 0, 0.6], anchor: [0, 0, 1.4] },
};

/** Interleave a world-space triangle list into the renderer's vertex format. */
export function interleaveMesh(mesh: MeshData): Float32Array<ArrayBuffer> {
  const n = mesh.positions.length / 3;
  const out = new Float32Array(n * VERTEX_FLOATS);
  for (let i = 0; i < n; i++) {
    out.set(mesh.positions.subarray(i * 3, i * 3 + 3), i * VERTEX_FLOATS);
    out.set(mesh.normals.subarray(i * 3, i * 3 + 3), i * VERTEX_FLOATS + 3);
    out.set(mesh.colors.subarray(i * 3, i * 3 + 3), i * VERTEX_FLOATS + 6);
  }
  return out;
}
