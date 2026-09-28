// Interleaved triangle-list meshes: xyz position, xyz normal, rgba colour per
// vertex, the renderer's one vertex format.
import { vec3, type Vec3 } from "math";
import { triangle3 } from "math/shapes";

export const VERTEX_FLOATS = 10;

export type Mesh = Float32Array<ArrayBuffer>;
export type Rgba = readonly [number, number, number, number];

/** A fixture colour: four channels in [0, 1]. */
export function isRgba(c: unknown): c is Rgba {
  return (
    Array.isArray(c) && c.length === 4 && c.every((v) => typeof v === "number" && v >= 0 && v <= 1)
  );
}
type P3 = readonly [number, number, number];

const AXIS_X: Vec3 = [1, 0, 0];
const AXIS_Z: Vec3 = [0, 0, 1];
const _triangle_normal = vec3.create();
const _segment_forward = vec3.create();
const _segment_u = vec3.create();
const _segment_v = vec3.create();

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
    const n = triangle3.normal(_triangle_normal, a as Vec3, b as Vec3, c as Vec3);
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
    const f = vec3.subtract(_segment_forward, b as Vec3, a as Vec3);
    if (vec3.squaredLength(f) === 0) return this;
    vec3.normalize(f, f);
    // Any axis not parallel to the segment gives the tube's cross-section frame.
    const ref = Math.abs(f[2]) < 0.9 ? AXIS_Z : AXIS_X;
    const u = vec3.normalize(_segment_u, vec3.cross(_segment_u, f, ref));
    const v = vec3.cross(_segment_v, f, u);
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

/** One mesh holding every input's triangles, in order. */
export function concatMeshes(parts: readonly Mesh[]): Mesh {
  const out = new Float32Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

export interface AnnulusOptions {
  /** Height of the surface the band lies on. */
  z: (x: number, y: number) => number;
  /** Metres above the surface. */
  lift: number;
  /** Quads per full turn. */
  segments: number;
  /** Colour at the first radius. */
  colorIn: Rgba;
  /** Colour at the second radius (default `colorIn`): alpha ramps across the band. */
  colorOut?: Rgba;
  /** Start angle in radians (default 0, +X). */
  start?: number;
  /** Fraction of a full circle swept from `start` (default 1); negative runs clockwise. */
  turn?: number;
  /** Draw every other pair of segments only: a broken ring. */
  dashed?: boolean;
}

/** A flat band on the surface between radius `inner` and `outer` around `c`,
 *  over a whole circle or an arc. Each segment is two triangles spanning from
 *  the first radius to the second, which may be the smaller one; an `inner`
 *  of 0 fills a disc. */
export function groundAnnulus(
  mesh: MeshBuilder,
  c: readonly [number, number],
  inner: number,
  outer: number,
  {
    z,
    lift,
    segments,
    colorIn,
    colorOut = colorIn,
    start = 0,
    turn = 1,
    dashed = false,
  }: AnnulusOptions,
) {
  const at = (a: number, r: number): P3 => {
    const x = c[0] + Math.cos(a) * r,
      y = c[1] + Math.sin(a) * r;
    return [x, y, z(x, y) + lift];
  };
  const n = Math.max(1, Math.ceil(segments * Math.abs(turn)));
  for (let k = 0; k < n; k++) {
    if (dashed && k % 4 >= 2) continue;
    const a0 = start + (k / n) * turn * Math.PI * 2;
    const a1 = start + ((k + 1) / n) * turn * Math.PI * 2;
    const [i0, i1, o0, o1] = [at(a0, inner), at(a1, inner), at(a0, outer), at(a1, outer)];
    mesh.shadedTriangle(i0, i1, o1, colorIn, colorIn, colorOut);
    mesh.shadedTriangle(i0, o1, o0, colorIn, colorOut, colorOut);
  }
}
