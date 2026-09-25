// Interleaved triangle-list meshes: xyz position, xyz normal, rgba colour per
// vertex, the renderer's one vertex format.

export const VERTEX_FLOATS = 10;

export type Mesh = Float32Array<ArrayBuffer>;
export type Rgba = readonly [number, number, number, number];
type P3 = readonly [number, number, number];

export class MeshBuilder {
  private readonly out: number[] = [];

  vertex(p: P3, n: P3, c: Rgba) {
    this.out.push(p[0], p[1], p[2], n[0], n[1], n[2], c[0], c[1], c[2], c[3]);
    return this;
  }

  /** One flat-shaded triangle; the normal follows the winding. */
  triangle(a: P3, b: P3, c: P3, color: Rgba) {
    return this.shadedTriangle(a, b, c, color, color, color);
  }

  /** Flat-shaded triangle with a colour per corner. */
  shadedTriangle(a: P3, b: P3, c: P3, ca: Rgba, cb: Rgba, cc: Rgba) {
    const n = faceNormal(a, b, c);
    return this.vertex(a, n, ca).vertex(b, n, cb).vertex(c, n, cc);
  }

  quad(a: P3, b: P3, c: P3, d: P3, color: Rgba) {
    return this.triangle(a, b, c, color).triangle(a, c, d, color);
  }

  /** Box in the local frame. */
  box(cx: number, cy: number, cz: number, hx: number, hy: number, hz: number, color: Rgba) {
    return this.orientedBox(cx, cy, 0, [hx, hy, hz], cz - hz, color);
  }

  /** Box of half extents `half`, rotated by `yaw` about +Z, standing on `baseZ`. */
  orientedBox(cx: number, cy: number, yaw: number, half: P3, baseZ: number, color: Rgba) {
    const c = Math.cos(yaw),
      s = Math.sin(yaw);
    const corner = (sx: number, sy: number, sz: number): P3 => {
      const lx = sx * half[0],
        ly = sy * half[1];
      return [cx + lx * c - ly * s, cy + lx * s + ly * c, baseZ + (sz > 0 ? 2 * half[2] : 0)];
    };
    const p = [
      corner(-1, -1, -1),
      corner(1, -1, -1),
      corner(1, 1, -1),
      corner(-1, 1, -1),
      corner(-1, -1, 1),
      corner(1, -1, 1),
      corner(1, 1, 1),
      corner(-1, 1, 1),
    ];
    for (const [a, b, cc, d] of [
      [0, 3, 2, 1],
      [4, 5, 6, 7],
      [0, 1, 5, 4],
      [2, 3, 7, 6],
      [1, 2, 6, 5],
      [3, 0, 4, 7],
    ]) {
      this.quad(p[a], p[b], p[cc], p[d], color);
    }
    return this;
  }

  /** Square tube of half width `half` from `a` to `b` (a line with thickness). */
  segment(a: P3, b: P3, half: number, color: Rgba) {
    const d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    const len = Math.hypot(d[0], d[1], d[2]);
    if (len === 0) return this;
    const f = [d[0] / len, d[1] / len, d[2] / len];
    // Any axis not parallel to the segment gives the tube's cross-section frame.
    const ref = Math.abs(f[2]) < 0.9 ? [0, 0, 1] : [1, 0, 0];
    const u = normalize(cross(f, ref));
    const v = cross(f, u);
    const ring = (p: P3): P3[] =>
      [
        [1, 1],
        [-1, 1],
        [-1, -1],
        [1, -1],
      ].map(([s, t]) => [
        p[0] + (s * u[0] + t * v[0]) * half,
        p[1] + (s * u[1] + t * v[1]) * half,
        p[2] + (s * u[2] + t * v[2]) * half,
      ]);
    const ra = ring(a),
      rb = ring(b);
    for (let i = 0; i < 4; i++) {
      const j = (i + 1) % 4;
      this.quad(ra[i], ra[j], rb[j], rb[i], color);
    }
    return this.quad(ra[3], ra[2], ra[1], ra[0], color).quad(rb[0], rb[1], rb[2], rb[3], color);
  }

  /** Upright `sides`-gon prism of circumradius `radius` standing on `baseZ`. */
  prism(
    cx: number,
    cy: number,
    baseZ: number,
    radius: number,
    height: number,
    sides: number,
    color: Rgba,
  ) {
    const at = (i: number, z: number): P3 => {
      const a = (i / sides) * Math.PI * 2;
      return [cx + Math.cos(a) * radius, cy + Math.sin(a) * radius, z];
    };
    const top = baseZ + height;
    for (let i = 0; i < sides; i++) {
      this.quad(at(i, baseZ), at(i + 1, baseZ), at(i + 1, top), at(i, top), color);
      this.triangle([cx, cy, top], at(i, top), at(i + 1, top), color);
      this.triangle([cx, cy, baseZ], at(i + 1, baseZ), at(i, baseZ), color);
    }
    return this;
  }

  /** Upward wedge on a +X face: a facing cue whose apex is at `tipX`. */
  wedge(baseX: number, tipX: number, hy: number, z0: number, z1: number, color: Rgba) {
    const a: P3 = [baseX, -hy, z0],
      b: P3 = [baseX, hy, z0],
      c: P3 = [baseX, hy, z1],
      d: P3 = [baseX, -hy, z1],
      t0: P3 = [tipX, 0, z0],
      t1: P3 = [tipX, 0, z1];
    return this.triangle(a, t0, t1, color)
      .triangle(a, t1, d, color)
      .triangle(t0, b, c, color)
      .triangle(t0, c, t1, color)
      .triangle(d, t1, c, color)
      .triangle(a, b, t0, color);
  }

  build(): Mesh {
    return Float32Array.from(this.out);
  }
}

function cross(a: readonly number[], b: readonly number[]): number[] {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

function normalize(v: number[]): number[] {
  const l = Math.hypot(v[0], v[1], v[2]) || 1;
  return [v[0] / l, v[1] / l, v[2] / l];
}

function faceNormal(a: P3, b: P3, c: P3): P3 {
  const u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  const v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
  const [x, y, z] = normalize(cross(u, v));
  return [x, y, z];
}
